use std::collections::HashMap;
use std::io::BufRead;
use std::path::{Path, PathBuf};

use crate::db::{self, debug};
use crate::models::IndexedRecord;

pub type Result<T> = std::result::Result<T, String>;

fn first_existing_or_default(candidates: &[PathBuf]) -> PathBuf {
    for candidate in candidates {
        if candidate.exists() {
            return std::fs::canonicalize(candidate).unwrap_or_else(|_| candidate.clone());
        }
    }

    candidates
        .first()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Resolve PAPERS directory. Priority: PAPERS_DIR env, installed layout, then repo layout.
pub fn resolve_papers_dir() -> Result<PathBuf> {
    if let Ok(dir) = std::env::var("PAPERS_DIR") {
        let path = PathBuf::from(&dir);
        debug!("Using env PAPERS_DIR: {}", path.display());
        return Ok(path);
    }

    let exe = std::env::current_exe().map_err(|e| format!("Failed to get executable path: {e}"))?;
    let exe_dir = exe.parent().ok_or("Failed to get executable directory")?;

    // Prefer same-directory layout: exe next to PAPERS/ (dev) or installed root.
    let candidates = [
        exe_dir.join("PAPERS"),
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("PAPERS"),
        exe_dir.join("..").join("PAPERS"),
    ];
    let canonical = first_existing_or_default(&candidates);
    debug!("Using default PAPERS dir: {}", canonical.display());
    Ok(canonical)
}

/// Resolve DB path. Priority: PAPERS_DB_PATH env, installed layout, then repo layout.
pub fn resolve_db_path() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("PAPERS_DB_PATH") {
        let p = PathBuf::from(&path);
        debug!("Using env PAPERS_DB_PATH: {}", p.display());
        return Ok(p);
    }

    let exe = std::env::current_exe().map_err(|e| format!("Failed to get executable path: {e}"))?;
    let exe_dir = exe.parent().ok_or("Failed to get executable directory")?;

    let candidates = [
        exe_dir.join("papers.db"),
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("papers.db"),
        exe_dir.join("..").join("papers.db"),
    ];
    let canonical = first_existing_or_default(&candidates);
    debug!("Using default DB path: {}", canonical.display());
    Ok(canonical)
}

/// Scan the first JSONL file found and detect field names from the first non-empty line.
fn detect_schema(papers_dir: &Path) -> Result<(Vec<String>, PathBuf)> {
    let read_dir =
        std::fs::read_dir(papers_dir).map_err(|e| format!("Failed to read PAPERS directory: {e}"))?;

    for level_entry in read_dir {
        let level_entry = level_entry.map_err(|e| format!("Failed to read level entry: {e}"))?;
        let level_path = level_entry.path();
        if !level_path.is_dir() {
            continue;
        }

        let conf_entries = std::fs::read_dir(&level_path)
            .map_err(|e| format!("Failed to read level directory {}: {e}", level_path.display()))?;

        for conf_entry in conf_entries {
            let conf_entry = conf_entry.map_err(|e| format!("Failed to read conference entry: {e}"))?;
            let conf_path = conf_entry.path();
            if !conf_path.is_dir() {
                continue;
            }

            let file_entries = std::fs::read_dir(&conf_path)
                .map_err(|e| format!("Failed to read conference directory: {e}"))?;

            for file_entry in file_entries {
                let file_entry = file_entry.map_err(|e| format!("Failed to read file entry: {e}"))?;
                let file_path = file_entry.path();
                if file_path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
                    continue;
                }

                let file =
                    std::fs::File::open(&file_path).map_err(|e| format!("Failed to open JSONL: {e}"))?;
                let reader = std::io::BufReader::new(file);

                for line in reader.lines() {
                    let line = line.map_err(|e| format!("Failed to read JSONL line: {e}"))?;
                    if line.trim().is_empty() {
                        continue;
                    }

                    let value: serde_json::Value = serde_json::from_str(&line)
                        .map_err(|e| format!("JSONL first line parse error: {e}"))?;

                    let obj = value
                        .as_object()
                        .ok_or("JSONL line is not a JSON object, expected {{...}}")?;

                    let keys: Vec<String> = obj.keys().cloned().collect();
                    if keys.is_empty() {
                        return Err("JSONL object is empty, no fields detected".to_string());
                    }

                    eprintln!("Detected field schema (from {}): {:?}", file_path.display(), keys);
                    return Ok((keys, file_path.clone()));
                }
            }
        }
    }

    Err("No .jsonl files found, cannot detect field structure".to_string())
}

/// Build the SQLite database from JSONL files.
pub fn build_db(papers_dir: &Path, db_path: &Path) -> Result<()> {
    if !papers_dir.is_dir() {
        return Err(format!(
            "PAPERS directory not found: {}\nSet the PAPERS_DIR environment variable or ensure the directory exists.",
            papers_dir.display()
        ));
    }

    eprintln!("Building database...");
    eprintln!("PAPERS directory: {}", papers_dir.display());
    eprintln!("Database path: {}", db_path.display());

    // Validate directory structure
    let has_level_dirs = std::fs::read_dir(papers_dir)
        .map_err(|e| format!("Failed to read PAPERS directory: {e}"))?
        .filter_map(|e| e.ok())
        .any(|e| e.path().is_dir());

    if !has_level_dirs {
        return Err(format!(
            "PAPERS directory structure is invalid.\nExpected: PAPERS/<level>/<conference>/<year>.jsonl\nCheck directory: {}",
            papers_dir.display()
        ));
    }

    // Step 1: Detect schema from first JSONL line
    eprintln!("Detecting JSONL field schema...");
    let (data_columns, _schema_source) = detect_schema(papers_dir)?;

    // Step 2: Read all JSONL files
    let mut records = Vec::new();
    let mut jsonl_file_count: u64 = 0;
    let mut schema_warned = false;

    let read_dir =
        std::fs::read_dir(papers_dir).map_err(|e| format!("Failed to read PAPERS directory: {e}"))?;

    for level_entry in read_dir {
        let level_entry = level_entry.map_err(|e| format!("Failed to read level entry: {e}"))?;
        let level_path = level_entry.path();
        if !level_path.is_dir() {
            continue;
        }
        let level = level_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| format!("Invalid level directory name: {}", level_path.display()))?
            .to_string();
        debug!("Scanning level: {level}");

        let conf_entries =
            std::fs::read_dir(&level_path).map_err(|e| format!("Failed to read level directory: {e}"))?;

        for conf_entry in conf_entries {
            let conf_entry = conf_entry.map_err(|e| format!("Failed to read conference entry: {e}"))?;
            let conf_path = conf_entry.path();
            if !conf_path.is_dir() {
                continue;
            }
            let conference = conf_path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| format!("Invalid conference directory name: {}", conf_path.display()))?
                .to_string();
            debug!("  Scanning conference: {conference}");

            let file_entries = std::fs::read_dir(&conf_path)
                .map_err(|e| format!("Failed to read conference directory: {e}"))?;

            for file_entry in file_entries {
                let file_entry = file_entry.map_err(|e| format!("Failed to read file entry: {e}"))?;
                let file_path = file_entry.path();

                if file_path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
                    continue;
                }

                let year = file_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| {
                        format!("Invalid filename (expected <year>.jsonl): {}", file_path.display())
                    })?
                    .to_string();

                jsonl_file_count += 1;
                debug!("    Reading file: {}", file_path.display());

                let file = std::fs::File::open(&file_path)
                    .map_err(|e| format!("Failed to open file {}: {e}", file_path.display()))?;
                let reader = std::io::BufReader::new(file);

                for (line_num, line) in reader.lines().enumerate() {
                    let line = line.map_err(|e| {
                        format!("Failed to read file {}:{}: {e}", file_path.display(), line_num + 1)
                    })?;
                    if line.trim().is_empty() {
                        continue;
                    }

                    let value: serde_json::Value = serde_json::from_str(&line).map_err(|e| {
                        format!(
                            "JSONL parse error: {} line {}: {e}\nContent: {}",
                            file_path.display(),
                            line_num + 1,
                            &line[..line.len().min(80)]
                        )
                    })?;

                    let obj = value.as_object().ok_or_else(|| {
                        format!(
                            "JSONL line is not a JSON object: {} line {}",
                            file_path.display(),
                            line_num + 1
                        )
                    })?;

                    // Warn about extra fields
                    if !schema_warned {
                        for key in obj.keys() {
                            if !data_columns.contains(key) {
                                eprintln!(
                                    "Warning: file {} line {} contains fields not in schema: {}. Ignoring.",
                                    file_path.display(),
                                    line_num + 1,
                                    key
                                );
                                schema_warned = true;
                            }
                        }
                    }

                    let mut data = HashMap::new();
                    for col in &data_columns {
                        let val = obj
                            .get(col)
                            .map(|v| match v {
                                serde_json::Value::String(s) => s.clone(),
                                other => other.to_string(),
                            })
                            .unwrap_or_default();
                        data.insert(col.clone(), val);
                    }

                    records.push(IndexedRecord {
                        level: level.clone(),
                        conference: conference.clone(),
                        year: year.clone(),
                        data,
                    });
                }
            }
        }
    }

    if jsonl_file_count == 0 {
        return Err(
            "No .jsonl files found.\nExpected: PAPERS/<level>/<conference>/<year>.jsonl\nGenerate JSONL files first."
                .to_string(),
        );
    }

    eprintln!(
        "Found {} JSONL files, {} paper records total",
        jsonl_file_count,
        records.len()
    );

    // Step 3: Create table and insert
    let conn = db::open_db(db_path)?;

    // Check if DB already has data and warn
    let old_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM papers", [], |row| row.get(0))
        .unwrap_or(0);
    if old_count > 0 {
        eprintln!("Replacing old database (old records: {old_count})");
    }

    db::clear_db(&conn)?;
    db::create_table(&conn, &data_columns)?;
    let count = db::insert_records(&conn, &records, &data_columns)?;

    eprintln!("Database build complete: {} records written to {}", count, db_path.display());
    println!(
        "Database build complete: {} paper records\n  Fixed fields: {:?}\n  Data fields: {:?}",
        count,
        db::FIXED_COLUMNS,
        data_columns
    );
    Ok(())
}
