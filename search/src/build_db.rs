use std::collections::HashMap;
use std::io::BufRead;
use std::path::{Path, PathBuf};

use crate::db::{self, debug};
use crate::models::IndexedRecord;

pub type Result<T> = std::result::Result<T, String>;

/// Resolve PAPERS directory. Priority: PAPERS_DIR env → search/target/../../PAPERS
pub fn resolve_papers_dir() -> Result<PathBuf> {
    if let Ok(dir) = std::env::var("PAPERS_DIR") {
        let path = PathBuf::from(&dir);
        debug!("使用环境变量 PAPERS_DIR: {}", path.display());
        return Ok(path);
    }

    let exe = std::env::current_exe().map_err(|e| format!("无法获取可执行文件路径: {e}"))?;
    let exe_dir = exe.parent().ok_or("无法获取可执行文件目录")?;

    // From search/target/<profile>/search → up 3 levels → PAPERS
    let default = exe_dir.join("..").join("..").join("..").join("PAPERS");
    let canonical = std::fs::canonicalize(&default).unwrap_or(default);
    debug!("使用默认 PAPERS 目录: {}", canonical.display());
    Ok(canonical)
}

/// Resolve DB path. Priority: PAPERS_DB_PATH env → search/target/../../papers.db
pub fn resolve_db_path() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("PAPERS_DB_PATH") {
        let p = PathBuf::from(&path);
        debug!("使用环境变量 PAPERS_DB_PATH: {}", p.display());
        return Ok(p);
    }

    let exe = std::env::current_exe().map_err(|e| format!("无法获取可执行文件路径: {e}"))?;
    let exe_dir = exe.parent().ok_or("无法获取可执行文件目录")?;

    let default = exe_dir.join("..").join("..").join("papers.db");
    let canonical = std::fs::canonicalize(&default).unwrap_or(default);
    debug!("使用默认数据库路径: {}", canonical.display());
    Ok(canonical)
}

/// Scan the first JSONL file found and detect field names from the first non-empty line.
fn detect_schema(papers_dir: &Path) -> Result<(Vec<String>, PathBuf)> {
    let read_dir =
        std::fs::read_dir(papers_dir).map_err(|e| format!("无法读取 PAPERS 目录: {e}"))?;

    for level_entry in read_dir {
        let level_entry = level_entry.map_err(|e| format!("读取 level 条目失败: {e}"))?;
        let level_path = level_entry.path();
        if !level_path.is_dir() {
            continue;
        }

        let conf_entries = std::fs::read_dir(&level_path)
            .map_err(|e| format!("无法读取 level 目录 {}: {e}", level_path.display()))?;

        for conf_entry in conf_entries {
            let conf_entry = conf_entry.map_err(|e| format!("读取 conference 条目失败: {e}"))?;
            let conf_path = conf_entry.path();
            if !conf_path.is_dir() {
                continue;
            }

            let file_entries = std::fs::read_dir(&conf_path)
                .map_err(|e| format!("无法读取 conference 目录: {e}"))?;

            for file_entry in file_entries {
                let file_entry = file_entry.map_err(|e| format!("读取文件条目失败: {e}"))?;
                let file_path = file_entry.path();
                if file_path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
                    continue;
                }

                let file = std::fs::File::open(&file_path)
                    .map_err(|e| format!("无法打开 JSONL: {e}"))?;
                let reader = std::io::BufReader::new(file);

                for line in reader.lines() {
                    let line = line.map_err(|e| format!("读取 JSONL 行失败: {e}"))?;
                    if line.trim().is_empty() {
                        continue;
                    }

                    let value: serde_json::Value = serde_json::from_str(&line)
                        .map_err(|e| format!("JSONL 首行格式错误: {e}"))?;

                    let obj = value
                        .as_object()
                        .ok_or("JSONL 行不是 JSON 对象，应为 {{...}} 格式")?;

                    let keys: Vec<String> = obj.keys().cloned().collect();
                    if keys.is_empty() {
                        return Err("JSONL 对象为空，无字段可检测".to_string());
                    }

                    eprintln!(
                        "检测到字段结构 (来自 {}): {:?}",
                        file_path.display(),
                        keys
                    );
                    return Ok((keys, file_path.clone()));
                }
            }
        }
    }

    Err("未找到任何 .jsonl 文件，无法检测字段结构".to_string())
}

/// Build the SQLite database from JSONL files.
pub fn build_db(papers_dir: &Path, db_path: &Path) -> Result<()> {
    if !papers_dir.is_dir() {
        return Err(format!(
            "PAPERS 目录不存在: {}\n请设置 PAPERS_DIR 环境变量或确保目录存在。",
            papers_dir.display()
        ));
    }

    eprintln!("开始构建数据库");
    eprintln!("PAPERS 目录: {}", papers_dir.display());
    eprintln!("数据库路径: {}", db_path.display());

    // Validate directory structure
    let has_level_dirs = std::fs::read_dir(papers_dir)
        .map_err(|e| format!("无法读取 PAPERS 目录: {e}"))?
        .filter_map(|e| e.ok())
        .any(|e| e.path().is_dir());

    if !has_level_dirs {
        return Err(format!(
            "PAPERS 目录结构不符合预期。\n期望结构: PAPERS/<level>/<conference>/<year>.jsonl\n请检查目录: {}",
            papers_dir.display()
        ));
    }

    // Step 1: Detect schema from first JSONL line
    eprintln!("正在检测 JSONL 字段结构...");
    let (data_columns, _schema_source) = detect_schema(papers_dir)?;

    // Step 2: Read all JSONL files
    let mut records = Vec::new();
    let mut jsonl_file_count: u64 = 0;
    let mut schema_warned = false;

    let read_dir =
        std::fs::read_dir(papers_dir).map_err(|e| format!("无法读取 PAPERS 目录: {e}"))?;

    for level_entry in read_dir {
        let level_entry = level_entry.map_err(|e| format!("读取 level 条目失败: {e}"))?;
        let level_path = level_entry.path();
        if !level_path.is_dir() {
            continue;
        }
        let level = level_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| format!("无效的 level 目录名: {}", level_path.display()))?
            .to_string();
        debug!("扫描 level: {level}");

        let conf_entries = std::fs::read_dir(&level_path)
            .map_err(|e| format!("无法读取 level 目录: {e}"))?;

        for conf_entry in conf_entries {
            let conf_entry = conf_entry.map_err(|e| format!("读取 conference 条目失败: {e}"))?;
            let conf_path = conf_entry.path();
            if !conf_path.is_dir() {
                continue;
            }
            let conference = conf_path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| {
                    format!("无效的 conference 目录名: {}", conf_path.display())
                })?
                .to_string();
            debug!("  扫描 conference: {conference}");

            let file_entries = std::fs::read_dir(&conf_path)
                .map_err(|e| format!("无法读取 conference 目录: {e}"))?;

            for file_entry in file_entries {
                let file_entry = file_entry.map_err(|e| format!("读取文件条目失败: {e}"))?;
                let file_path = file_entry.path();

                if file_path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
                    continue;
                }

                let year = file_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| format!("无效的文件名（应为 <year>.jsonl）: {}", file_path.display()))?
                    .to_string();

                jsonl_file_count += 1;
                debug!("    读取文件: {}", file_path.display());

                let file = std::fs::File::open(&file_path)
                    .map_err(|e| format!("无法打开文件 {}: {e}", file_path.display()))?;
                let reader = std::io::BufReader::new(file);

                for (line_num, line) in reader.lines().enumerate() {
                    let line = line.map_err(|e| {
                        format!("读取文件失败 {}:{}: {e}", file_path.display(), line_num + 1)
                    })?;
                    if line.trim().is_empty() {
                        continue;
                    }

                    let value: serde_json::Value = serde_json::from_str(&line).map_err(|e| {
                        format!(
                            "JSONL 格式错误: {} 第 {} 行: {e}\n内容: {}",
                            file_path.display(),
                            line_num + 1,
                            &line[..line.len().min(80)]
                        )
                    })?;

                    let obj = value.as_object().ok_or_else(|| {
                        format!(
                            "JSONL 行不是 JSON 对象: {} 第 {} 行",
                            file_path.display(),
                            line_num + 1
                        )
                    })?;

                    // Warn about extra fields
                    if !schema_warned {
                        for key in obj.keys() {
                            if !data_columns.contains(key) {
                                eprintln!(
                                    "警告: 文件 {} 第 {} 行包含未在 schema 中的字段: {}. 将忽略。",
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
            "未找到任何 .jsonl 文件。\n期望: PAPERS/<level>/<conference>/<year>.jsonl\n请先用转换工具生成 JSONL 文件。"
                .to_string(),
        );
    }

    eprintln!(
        "扫描到 {} 个 JSONL 文件，共 {} 条论文记录",
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
        eprintln!("覆盖旧数据库 (旧记录: {old_count} 条)");
    }

    db::clear_db(&conn)?;
    db::create_table(&conn, &data_columns)?;
    let count = db::insert_records(&conn, &records, &data_columns)?;

    eprintln!(
        "数据库构建完成: {} 条记录写入 {}",
        count,
        db_path.display()
    );
    println!(
        "数据库构建完成: {} 条论文记录\n  固定字段: {:?}\n  数据字段: {:?}",
        count,
        db::FIXED_COLUMNS,
        data_columns
    );
    Ok(())
}
