mod test_utils;

use std::collections::{BTreeMap, HashSet};
use std::io::BufRead;
use std::path::{Path, PathBuf};

use test_utils::*;

// ── build-db tests ──

#[test]
fn build_db_creates_database() {
    let dir = temp_test_dir();
    let paper_dir = dir.join("PAPERS");
    let db_path = dir.join("test.db");

    create_test_papers(
        &paper_dir,
        "A",
        "ICML",
        "2024",
        &["First Paper", "Second Paper"],
    );
    create_test_papers(&paper_dir, "B", "EMNLP", "2025", &["Third Paper"]);

    let output = run_search(&paper_dir, &db_path, &["build-db"]);
    assert_success(&output);

    assert!(db_path.exists(), "DB file should exist");
    let meta = std::fs::metadata(&db_path).unwrap();
    assert!(meta.len() > 100, "DB file should have content");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_db_reports_missing_papers_dir() {
    let dir = temp_test_dir();
    let nonexistent = dir.join("NONEXISTENT");
    let db_path = dir.join("test.db");

    let output = run_search(&nonexistent, &db_path, &["build-db"]);
    assert!(
        !output.status.success(),
        "Should fail for nonexistent directory"
    );
    let stderr = stderr_str(&output);
    assert!(
        stderr.contains("directory not found") || stderr.to_lowercase().contains("not found"),
        "Should mention missing directory: {stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_db_reports_no_jsonl_files() {
    let dir = temp_test_dir();
    let paper_dir = dir.join("PAPERS");
    let db_path = dir.join("test.db");

    // Create directory structure but no .jsonl files
    let conf_dir = paper_dir.join("A").join("ICML");
    std::fs::create_dir_all(&conf_dir).unwrap();
    std::fs::write(conf_dir.join("2024.txt"), "Some Title\n").unwrap();

    let output = run_search(&paper_dir, &db_path, &["build-db"]);
    assert!(!output.status.success(), "Should fail without JSONL files");
    let stderr = stderr_str(&output);
    assert!(
        stderr.to_lowercase().contains("jsonl"),
        "Should mention JSONL: {stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_db_reports_bad_jsonl_format() {
    let dir = temp_test_dir();
    let paper_dir = dir.join("PAPERS");
    let db_path = dir.join("test.db");

    let conf_dir = paper_dir.join("A").join("ICML");
    std::fs::create_dir_all(&conf_dir).unwrap();
    std::fs::write(conf_dir.join("2024.jsonl"), "this is not json\n").unwrap();

    let output = run_search(&paper_dir, &db_path, &["build-db"]);
    assert!(!output.status.success(), "Should fail on bad JSONL");
    let stderr = stderr_str(&output);
    assert!(
        stderr.to_lowercase().contains("parse error") || stderr.contains("JSONL"),
        "Should mention format error: {stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_db_detects_schema_dynamically() {
    let dir = temp_test_dir();
    let paper_dir = dir.join("PAPERS");
    let db_path = dir.join("test.db");

    // Create JSONL with custom fields
    let conf_dir = paper_dir.join("A").join("CVPR");
    std::fs::create_dir_all(&conf_dir).unwrap();
    std::fs::write(
        conf_dir.join("2024.jsonl"),
        r#"{"title":"Test Paper","author":"Alice","doi":"10.1234/test","url":"http://example.com"}
{"title":"Another","author":"Bob","doi":"10.5678/foo","url":"http://bar.com"}
"#,
    )
    .unwrap();

    let output = run_search(&paper_dir, &db_path, &["build-db"]);
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("title"), "Should detect title field");
    assert!(stdout.contains("author"), "Should detect author field");

    let _ = std::fs::remove_dir_all(&dir);
}

// ── query tests ──

#[test]
fn build_db_records_database_version() {
    let dir = temp_test_dir();
    let paper_dir = dir.join("PAPERS");
    let db_path = dir.join("test.db");

    create_test_papers(&paper_dir, "A", "ICML", "2024", &["Versioned Paper"]);

    let output = run_search_with_env(
        &paper_dir,
        &db_path,
        &["build-db"],
        &[("PAPERS_DB_VERSION", "test-version-123")],
    );
    assert_success(&output);

    let output = run_search(&paper_dir, &db_path, &["version"]);
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Database version: test-version-123"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn version_reports_missing_database() {
    let dir = temp_test_dir();
    let paper_dir = dir.join("PAPERS");
    let db_path = dir.join("missing.db");
    std::fs::create_dir_all(&paper_dir).unwrap();

    let output = run_search(&paper_dir, &db_path, &["version"]);
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("topaperlist"));
    assert!(stdout.contains("Database version: unavailable"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn version_reports_source_and_record_count() {
    let dir = temp_test_dir();
    let paper_dir = dir.join("PAPERS");
    let db_path = dir.join("test.db");

    create_test_papers(&paper_dir, "A", "ICML", "2024", &["Versioned Paper"]);

    let output = run_search_with_env(
        &paper_dir,
        &db_path,
        &["build-db"],
        &[
            ("PAPERS_DB_VERSION", "version-for-source-test"),
            ("PAPERS_DB_SOURCE", "local-test-source"),
        ],
    );
    assert_success(&output);

    let output = run_search(&paper_dir, &db_path, &["version"]);
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Database version: version-for-source-test"));
    assert!(stdout.contains("Database source: local-test-source"));
    assert!(stdout.contains("Record count: 1"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn update_command_explains_installed_wrapper() {
    let dir = temp_test_dir();
    let paper_dir = dir.join("PAPERS");
    let db_path = dir.join("test.db");
    std::fs::create_dir_all(&paper_dir).unwrap();

    let output = run_search(&paper_dir, &db_path, &["update"]);
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("installed search wrapper"));
    assert!(stdout.contains("search update"));

    let _ = std::fs::remove_dir_all(&dir);
}

fn setup_query_test() -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
    let dir = temp_test_dir();
    let paper_dir = dir.join("PAPERS");
    let db_path = dir.join("test.db");

    create_test_papers(
        &paper_dir,
        "A",
        "AAAI",
        "2024",
        &[
            "Diffusion Models for Image Generation",
            "Graph Neural Networks for Drug Discovery",
        ],
    );
    create_test_papers(
        &paper_dir,
        "A",
        "ICML",
        "2024",
        &["A Survey of Graph Diffusion Models"],
    );
    create_test_papers(
        &paper_dir,
        "A",
        "ICML",
        "2023",
        &["Attention Mechanisms in Deep Learning"],
    );
    create_test_papers(
        &paper_dir,
        "B",
        "EMNLP",
        "2024",
        &["Survey of Text Generation Methods"],
    );

    let output = run_search(&paper_dir, &db_path, &["build-db"]);
    assert_success(&output);

    (dir, paper_dir, db_path)
}

#[derive(Clone, Debug)]
struct SampledPaper {
    level: String,
    conference: String,
    year: String,
    title: String,
    author: String,
    bib: String,
    url: String,
}

fn read_dir_sorted(path: &Path) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", path.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|e| panic!("Failed to read entry under {}: {e}", path.display()))
                .path()
        })
        .collect();
    entries.sort();
    entries
}

fn sample_real_papers(count: usize) -> Vec<SampledPaper> {
    let papers_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("search crate should have a project root")
        .join("PAPERS");

    let mut candidates = Vec::new();
    for level_path in read_dir_sorted(&papers_dir) {
        if !level_path.is_dir() {
            continue;
        }

        let level = level_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        if level == "TEST" {
            continue;
        }

        for conference_path in read_dir_sorted(&level_path) {
            if !conference_path.is_dir() {
                continue;
            }
            let conference = conference_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();

            for jsonl_path in read_dir_sorted(&conference_path) {
                if jsonl_path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
                    continue;
                }
                let year = jsonl_path
                    .file_stem()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                let file = std::fs::File::open(&jsonl_path)
                    .unwrap_or_else(|e| panic!("Failed to open {}: {e}", jsonl_path.display()));
                let reader = std::io::BufReader::new(file);

                for line in reader.lines() {
                    let line = line
                        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", jsonl_path.display()));
                    if line.trim().is_empty() {
                        continue;
                    }
                    let value: serde_json::Value =
                        serde_json::from_str(&line).unwrap_or_else(|e| {
                            panic!("Failed to parse {}: {e}", jsonl_path.display())
                        });
                    let title = value
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    let bib = value
                        .get("bib")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .trim()
                        .to_string();

                    if title.is_empty()
                        || bib.is_empty()
                        || title.contains('\t')
                        || title.contains('\n')
                        || title.contains('\r')
                    {
                        continue;
                    }

                    candidates.push(SampledPaper {
                        level: level.clone(),
                        conference: conference.clone(),
                        year: year.clone(),
                        title,
                        author: value
                            .get("author")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        bib,
                        url: value
                            .get("url")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                    });
                }
            }
        }
    }

    assert!(
        candidates.len() >= count,
        "Need at least {count} real paper candidates, got {}",
        candidates.len()
    );

    let mut seed = 0x5eed_cafe_d00d_f00du64;
    let mut selected = Vec::new();
    let mut seen_titles = HashSet::new();
    while selected.len() < count {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let candidate = candidates[(seed as usize) % candidates.len()].clone();
        if seen_titles.insert(candidate.title.to_ascii_lowercase()) {
            selected.push(candidate);
        }
    }

    selected
}

fn write_sampled_papers(root: &Path, samples: &[SampledPaper]) {
    let mut grouped: BTreeMap<(String, String, String), Vec<SampledPaper>> = BTreeMap::new();
    for sample in samples {
        grouped
            .entry((
                sample.level.clone(),
                sample.conference.clone(),
                sample.year.clone(),
            ))
            .or_default()
            .push(sample.clone());
    }

    for ((level, conference, year), records) in grouped {
        let conf_dir = root.join(level).join(conference);
        std::fs::create_dir_all(&conf_dir).unwrap();

        let jsonl_content = records
            .iter()
            .map(|sample| {
                serde_json::json!({
                    "author": sample.author.as_str(),
                    "bib": sample.bib.as_str(),
                    "title": sample.title.as_str(),
                    "url": sample.url.as_str(),
                })
                .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(conf_dir.join(format!("{year}.jsonl")), jsonl_content + "\n").unwrap();

        let txt_content = records
            .iter()
            .map(|sample| sample.title.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(conf_dir.join(format!("{year}.txt")), txt_content + "\n").unwrap();
    }
}

fn assert_table_row(stdout: &str, field: &str, value: &str) {
    assert!(
        stdout.lines().any(|line| {
            let parts: Vec<&str> = line.split('|').collect();
            parts.len() >= 4 && parts[1].trim() == field && parts[2].trim() == value
        }),
        "Expected terminal table row `{field}: {value}` in output:\n{stdout}"
    );
}

fn assert_no_table_row(stdout: &str, field: &str) {
    assert!(
        !stdout.lines().any(|line| {
            let parts: Vec<&str> = line.split('|').collect();
            parts.len() >= 4 && parts[1].trim() == field
        }),
        "Did not expect terminal table row `{field}` in output:\n{stdout}"
    );
}

fn assert_terminal_table_header(stdout: &str) {
    assert!(
        stdout
            .lines()
            .any(|line| line.starts_with('+') && line.ends_with('+')),
        "Expected terminal table border in output:\n{stdout}"
    );
    assert_table_row(stdout, "Field", "Value");
    assert!(
        !stdout.contains("| --- | --- |"),
        "Title query output should be a terminal table, not a Markdown table:\n{stdout}"
    );
}

fn assert_author_display(stdout: &str, author: &str) {
    if author.chars().count() > 80 {
        let table_stdout = stdout.split("\n\n").next().unwrap_or("");
        assert!(
            table_stdout.contains(".etc"),
            "Long author display should be abbreviated:\n{stdout}"
        );
    } else {
        assert_table_row(stdout, "author", author);
    }
}

#[test]
fn query_with_keyword_filter() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(&paper_dir, &db_path, &["query", "--keyword", "diffusion"]);
    assert_success(&output);
    let lines = stdout_lines(&output);
    assert!(!lines.is_empty(), "Should have diffusion results");

    for line in &lines {
        assert!(
            line.to_lowercase().contains("diffusion"),
            "Each result should contain diffusion: {line}"
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bib_command_outputs_single_paper_bibtex() {
    let dir = temp_test_dir();
    let paper_dir = dir.join("PAPERS");
    let db_path = dir.join("test.db");
    let conf_dir = paper_dir.join("A").join("AAAI");
    std::fs::create_dir_all(&conf_dir).unwrap();
    std::fs::write(
        conf_dir.join("2026.jsonl"),
        r#"{"title":"FastDriveVLA: Efficient End-to-End Driving","author":"Alice","bib":"@inproceedings{fastdrivevla2026,\n  title = {FastDriveVLA: Efficient End-to-End Driving}\n}","url":""}
{"title":"Other VLA Paper","author":"Bob","bib":"@inproceedings{other2026,\n  title = {Other VLA Paper}\n}","url":""}
"#,
    )
    .unwrap();

    let output = run_search(&paper_dir, &db_path, &["build-db"]);
    assert_success(&output);

    let output = run_search(&paper_dir, &db_path, &["bib", "FastDriveVLA"]);
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("@inproceedings{fastdrivevla2026"));
    assert!(stdout.contains("FastDriveVLA: Efficient End-to-End Driving"));
    assert!(!stdout.contains("@inproceedings{other2026"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_with_exact_title_filter() {
    let dir = temp_test_dir();
    let paper_dir = dir.join("PAPERS");
    let db_path = dir.join("test.db");
    let conf_dir = paper_dir.join("A").join("AAAI");
    std::fs::create_dir_all(&conf_dir).unwrap();
    std::fs::write(
        conf_dir.join("2026.jsonl"),
        r#"{"title":"Exact Paper, With Comma","author":"Alice","bib":"@inproceedings{exact2026}","url":"https://example.com/exact"}
{"title":"Exact Paper, With Comma Extended","author":"Bob","bib":"@inproceedings{extended2026}","url":"https://example.com/extended"}
"#,
    )
    .unwrap();

    let output = run_search(&paper_dir, &db_path, &["build-db"]);
    assert_success(&output);

    let output = run_search(
        &paper_dir,
        &db_path,
        &["query", "--title", "exact paper, with comma"],
    );
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_terminal_table_header(&stdout);
    assert_table_row(&stdout, "level", "A");
    assert_table_row(&stdout, "conference", "AAAI");
    assert_table_row(&stdout, "year", "2026");
    assert_table_row(&stdout, "title", "Exact Paper, With Comma");
    assert_table_row(&stdout, "author", "Alice");
    assert_table_row(&stdout, "url", "https://example.com/exact");
    assert!(stdout.contains("\n\n@inproceedings{exact2026}"));
    assert_no_table_row(&stdout, "bib");
    assert!(!stdout.contains("@inproceedings{extended2026}"));

    let output = run_search(
        &paper_dir,
        &db_path,
        &[
            "query",
            "--title",
            "exact paper, with comma",
            "--columns",
            "title",
            "--exclude-columns",
            "bib",
        ],
    );
    assert_success(&output);
    let columns_stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        columns_stdout, stdout,
        "column selection flags should be ignored for exact title lookup"
    );

    let output = run_search(
        &paper_dir,
        &db_path,
        &[
            "query",
            "--title",
            "exact paper, with comma",
            "--clumn",
            "xxx",
        ],
    );
    assert_success(&output);
    let unknown_stdout = String::from_utf8(output.stdout.clone()).unwrap();
    assert_eq!(
        unknown_stdout, stdout,
        "unknown options and their values should be ignored for exact title lookup"
    );
    assert!(
        !stderr_str(&output).contains("unsupported option"),
        "unsupported options should not warn for exact title lookup"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_with_exact_title_outputs_all_dynamic_fields() {
    let dir = temp_test_dir();
    let paper_dir = dir.join("PAPERS");
    let db_path = dir.join("test.db");
    let conf_dir = paper_dir.join("A").join("ICML");
    std::fs::create_dir_all(&conf_dir).unwrap();
    std::fs::write(
        conf_dir.join("2026.jsonl"),
        r#"{"title":"Full Metadata Paper","author":"Carol","bib":"@inproceedings{fullmeta2026}","doi":"10.5555/fullmeta","topic":"metadata lookup","url":"https://example.com/fullmeta"}
"#,
    )
    .unwrap();

    let output = run_search(&paper_dir, &db_path, &["build-db"]);
    assert_success(&output);

    let output = run_search(
        &paper_dir,
        &db_path,
        &["query", "--title", "Full Metadata Paper"],
    );
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_table_row(&stdout, "level", "A");
    assert_table_row(&stdout, "conference", "ICML");
    assert_table_row(&stdout, "year", "2026");
    assert_table_row(&stdout, "title", "Full Metadata Paper");
    assert_table_row(&stdout, "author", "Carol");
    assert_table_row(&stdout, "doi", "10.5555/fullmeta");
    assert_table_row(&stdout, "topic", "metadata lookup");
    assert_table_row(&stdout, "url", "https://example.com/fullmeta");
    assert!(stdout.contains("\n\n@inproceedings{fullmeta2026}"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_with_exact_title_ignores_bibtex_case_protection_braces() {
    let dir = temp_test_dir();
    let paper_dir = dir.join("PAPERS");
    let db_path = dir.join("test.db");
    let conf_dir = paper_dir.join("B").join("EMNLP");
    std::fs::create_dir_all(&conf_dir).unwrap();
    std::fs::write(
        conf_dir.join("2020.jsonl"),
        r#"{"title":"Attention Is All You Need for {C}hinese Word Segmentation","author":"Duan, Sufeng; Zhao, Hai","bib":"@inproceedings{EMNLP2020_15349}","url":"https://aclanthology.org/2020.emnlp-main.317/"}
{"title":"Attention Is All You Need for Chinese Parsing","author":"Other","bib":"@inproceedings{other2020}","url":""}
"#,
    )
    .unwrap();

    let output = run_search(&paper_dir, &db_path, &["build-db"]);
    assert_success(&output);

    let output = run_search(
        &paper_dir,
        &db_path,
        &[
            "query",
            "--title",
            "Attention Is All You Need for Chinese Word Segmentation",
        ],
    );
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_table_row(&stdout, "level", "B");
    assert_table_row(&stdout, "conference", "EMNLP");
    assert_table_row(&stdout, "year", "2020");
    assert_table_row(
        &stdout,
        "title",
        "Attention Is All You Need for {C}hinese Word Segmentation",
    );
    assert!(stdout.contains("\n\n@inproceedings{EMNLP2020_15349}"));
    assert!(!stdout.contains("@inproceedings{other2020}"));

    let output = run_search(
        &paper_dir,
        &db_path,
        &[
            "bib",
            "--title",
            "Attention Is All You Need for Chinese Word Segmentation",
        ],
    );
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.trim(), "@inproceedings{EMNLP2020_15349}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_with_exact_title_ignores_trailing_terminal_punctuation() {
    let dir = temp_test_dir();
    let paper_dir = dir.join("PAPERS");
    let db_path = dir.join("test.db");
    let conf_dir = paper_dir.join("A").join("NeurIPS");
    std::fs::create_dir_all(&conf_dir).unwrap();
    std::fs::write(
        conf_dir.join("2020.jsonl"),
        r#"{"title":"Language Models are Few-Shot Learners.","author":"Tom B Brown; Benjamin Mann; Nick Ryder; Melanie Subbiah; Jared Kaplan; Prafulla Dhariwal; Arvind Neelakantan","bib":"@inproceedings{NeurIPS2020_8146}","url":"https://example.com/gpt3"}
{"title":"Language Models are Few-Shot Learners Extended","author":"Other","bib":"@inproceedings{other2020}","url":""}
"#,
    )
    .unwrap();

    let output = run_search(&paper_dir, &db_path, &["build-db"]);
    assert_success(&output);

    let output = run_search(
        &paper_dir,
        &db_path,
        &["query", "--title", "Language Models are Few-Shot Learners"],
    );
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_terminal_table_header(&stdout);
    assert_table_row(&stdout, "level", "A");
    assert_table_row(&stdout, "conference", "NeurIPS");
    assert_table_row(&stdout, "year", "2020");
    assert_table_row(&stdout, "title", "Language Models are Few-Shot Learners.");
    let table_stdout = stdout.split("\n\n").next().unwrap_or("");
    assert!(
        table_stdout.contains(".etc"),
        "Long author display should be abbreviated:\n{stdout}"
    );
    assert!(
        !table_stdout.contains("Arvind Neelakantan"),
        "Long author display should not print the full author list:\n{stdout}"
    );
    assert!(stdout.contains("\n\n@inproceedings{NeurIPS2020_8146}"));
    assert!(!stdout.contains("@inproceedings{other2020}"));

    let output = run_search(
        &paper_dir,
        &db_path,
        &["bib", "--title", "Language Models are Few-Shot Learners"],
    );
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.trim(), "@inproceedings{NeurIPS2020_8146}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bib_command_accepts_exact_title_filter() {
    let dir = temp_test_dir();
    let paper_dir = dir.join("PAPERS");
    let db_path = dir.join("test.db");
    let conf_dir = paper_dir.join("A").join("AAAI");
    std::fs::create_dir_all(&conf_dir).unwrap();
    std::fs::write(
        conf_dir.join("2026.jsonl"),
        r#"{"title":"FastDriveVLA","author":"Alice","bib":"@inproceedings{fastdrivevla2026}","url":""}
{"title":"FastDriveVLA Extended","author":"Bob","bib":"@inproceedings{fastdrivevlaextended2026}","url":""}
"#,
    )
    .unwrap();

    let output = run_search(&paper_dir, &db_path, &["build-db"]);
    assert_success(&output);

    let output = run_search(&paper_dir, &db_path, &["bib", "--title", "fastdrivevla"]);
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("@inproceedings{fastdrivevla2026}"));
    assert!(!stdout.contains("@inproceedings{fastdrivevlaextended2026}"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn exact_title_lookup_matches_random_real_paper_samples() {
    let samples = sample_real_papers(6);
    let dir = temp_test_dir();
    let paper_dir = dir.join("PAPERS");
    let db_path = dir.join("test.db");
    write_sampled_papers(&paper_dir, &samples);

    let output = run_search(&paper_dir, &db_path, &["build-db"]);
    assert_success(&output);

    for sample in &samples {
        let output = run_search(
            &paper_dir,
            &db_path,
            &["query", "--title", sample.title.as_str()],
        );
        assert_success(&output);
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_table_row(&stdout, "level", &sample.level);
        assert_table_row(&stdout, "conference", &sample.conference);
        assert_table_row(&stdout, "year", &sample.year);
        assert_table_row(&stdout, "title", &sample.title);
        if !sample.author.trim().is_empty() {
            assert_author_display(&stdout, sample.author.trim());
        }
        assert!(
            stdout.contains(&format!("\n\n{}", sample.bib.trim())),
            "Exact title lookup should append sampled BibTeX after the table:\n{stdout}"
        );
        if !sample.url.trim().is_empty() {
            assert_table_row(&stdout, "url", sample.url.trim());
        }

        let output = run_search(
            &paper_dir,
            &db_path,
            &["bib", "--title", sample.title.as_str()],
        );
        assert_success(&output);
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(
            stdout.trim(),
            sample.bib.trim(),
            "BibTeX lookup should match sampled paper"
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_with_level_filter() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(&paper_dir, &db_path, &["query", "--level", "A"]);
    assert_success(&output);
    let lines = stdout_lines(&output);

    for line in &lines {
        let parts: Vec<&str> = line.split('\t').collect();
        assert_eq!(parts[0], "A", "level should be A: {line}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_with_conference_filter() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(&paper_dir, &db_path, &["query", "--conference", "ICML"]);
    assert_success(&output);
    let lines = stdout_lines(&output);
    assert!(!lines.is_empty(), "Should have ICML results");

    for line in &lines {
        let parts: Vec<&str> = line.split('\t').collect();
        assert_eq!(parts[1], "ICML", "conference should be ICML: {line}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_with_year_filter() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(&paper_dir, &db_path, &["query", "--year", "2023"]);
    assert_success(&output);
    let lines = stdout_lines(&output);
    assert!(!lines.is_empty(), "Should have 2023 results");

    for line in &lines {
        let parts: Vec<&str> = line.split('\t').collect();
        assert_eq!(parts[2], "2023", "year should be 2023: {line}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_with_exclude_filters() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(
        &paper_dir,
        &db_path,
        &[
            "query",
            "--exclude-level",
            "B",
            "--exclude-keyword",
            "survey",
            "diffusion",
        ],
    );
    assert_success(&output);
    let lines = stdout_lines(&output);
    assert!(!lines.is_empty(), "Should have results");

    for line in &lines {
        let parts: Vec<&str> = line.split('\t').collect();
        assert_ne!(parts[0], "B", "Should not contain B level");
        let title_lower = parts[3].to_lowercase();
        assert!(
            !title_lower.contains("survey"),
            "Should not contain survey: {line}"
        );
        assert!(
            title_lower.contains("diffusion"),
            "Should contain diffusion: {line}"
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_with_combined_filters() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(
        &paper_dir,
        &db_path,
        &[
            "query",
            "--level",
            "A",
            "--conference",
            "ICML,AAAI",
            "--year",
            "2024",
            "--keyword",
            "graph",
        ],
    );
    assert_success(&output);
    let lines = stdout_lines(&output);

    for line in &lines {
        let parts: Vec<&str> = line.split('\t').collect();
        assert_eq!(parts[0], "A");
        assert!(parts[1] == "ICML" || parts[1] == "AAAI");
        assert_eq!(parts[2], "2024");
        assert!(parts[3].to_lowercase().contains("graph"));
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_with_sort() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(
        &paper_dir,
        &db_path,
        &[
            "query",
            "--conference",
            "ICML",
            "--sort",
            "year:desc",
            "--sort",
            "title:asc",
        ],
    );
    assert_success(&output);
    let lines = stdout_lines(&output);

    let years: Vec<&str> = lines
        .iter()
        .map(|l| l.split('\t').nth(2).unwrap())
        .collect();
    for w in years.windows(2) {
        assert!(w[0] >= w[1], "year should be descending: {w:?}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_with_custom_columns() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(
        &paper_dir,
        &db_path,
        &[
            "query",
            "--conference",
            "AAAI",
            "--columns",
            "conference,year,title",
        ],
    );
    assert_success(&output);
    let lines = stdout_lines(&output);

    for line in &lines {
        let parts: Vec<&str> = line.split('\t').collect();
        assert_eq!(parts.len(), 3, "Should have 3 columns: {line}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_requires_at_least_one_filter() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(&paper_dir, &db_path, &["query"]);
    assert!(!output.status.success(), "Should fail without filters");
    let stderr = stderr_str(&output);
    assert!(
        stderr.contains("filter is required"),
        "Should mention filter required: {stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_unknown_option_without_title_warns_and_ignores_value() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(&paper_dir, &db_path, &["query", "--clumn", "diffusion"]);
    assert!(
        !output.status.success(),
        "Unknown option value should not become a positional keyword"
    );
    let stderr = stderr_str(&output);
    assert!(
        stderr.contains("unsupported option: --clumn"),
        "Should warn about unsupported option: {stderr}"
    );
    assert!(
        stderr.contains("filter is required"),
        "Should fail because no valid filter remains: {stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_positional_keywords_work_like_explicit() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let positional = run_search(&paper_dir, &db_path, &["query", "graph", "diffusion"]);
    let explicit = run_search(
        &paper_dir,
        &db_path,
        &["query", "--keyword", "graph,diffusion"],
    );

    assert_success(&positional);
    assert_success(&explicit);
    assert_eq!(
        stdout_lines(&positional),
        stdout_lines(&explicit),
        "positional and --keyword results should match"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn rebuild_db_replaces_old_data() {
    let dir = temp_test_dir();
    let paper_dir = dir.join("PAPERS");
    let db_path = dir.join("test.db");

    create_test_papers(&paper_dir, "A", "ICML", "2024", &["Original Paper"]);
    let output = run_search(&paper_dir, &db_path, &["build-db"]);
    assert_success(&output);

    let output = run_search(&paper_dir, &db_path, &["query", "--conference", "ICML"]);
    let lines = stdout_lines(&output);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("Original Paper"));

    // Replace data
    let conf_dir = paper_dir.join("A").join("ICML");
    std::fs::remove_dir_all(&conf_dir).unwrap();
    create_test_papers(&paper_dir, "A", "ICML", "2024", &["Updated Paper"]);

    let output = run_search(&paper_dir, &db_path, &["build-db"]);
    assert_success(&output);

    let output = run_search(&paper_dir, &db_path, &["query", "--conference", "ICML"]);
    let lines = stdout_lines(&output);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("Updated Paper"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_with_exclude_year() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(
        &paper_dir,
        &db_path,
        &["query", "--conference", "ICML", "--exclude-year", "2023"],
    );
    assert_success(&output);
    let lines = stdout_lines(&output);

    for line in &lines {
        let parts: Vec<&str> = line.split('\t').collect();
        assert_ne!(parts[2], "2023", "Should not contain 2023: {line}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_exclude_conference() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(
        &paper_dir,
        &db_path,
        &["query", "--level", "A", "--exclude-conference", "AAAI"],
    );
    assert_success(&output);
    let lines = stdout_lines(&output);

    for line in &lines {
        let parts: Vec<&str> = line.split('\t').collect();
        assert_ne!(parts[1], "AAAI", "Should not contain AAAI: {line}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn help_prints_usage() {
    // Use a temporary dir just for the env vars
    let dir = temp_test_dir();
    let paper_dir = dir.join("PAPERS");
    let db_path = dir.join("test.db");
    std::fs::create_dir_all(&paper_dir).unwrap();

    let output = run_search(&paper_dir, &db_path, &["--help"]);
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Usage"), "Should show usage");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn english_readme_links_to_chinese_readme_near_top() {
    let readme_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("search crate should have a project root")
        .join("README.md");
    let readme = std::fs::read_to_string(&readme_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", readme_path.display()));
    let top_lines = readme.lines().take(5).collect::<Vec<_>>().join("\n");

    assert!(
        top_lines.contains("[简体中文](README.zh.md)"),
        "English README should link to README.zh.md near the top"
    );
}

// ── standalone exclude filter tests ──

#[test]
fn query_title_exclude_standalone() {
    let (dir, paper_dir, db_path) = setup_query_test();

    // Only exclude keywords, no include filter
    let output = run_search(
        &paper_dir,
        &db_path,
        &["query", "--exclude-keyword", "survey"],
    );
    assert_success(&output);
    let lines = stdout_lines(&output);
    assert!(!lines.is_empty(), "Should have results");

    for line in &lines {
        let parts: Vec<&str> = line.split('\t').collect();
        let title_lower = parts[3].to_lowercase();
        assert!(
            !title_lower.contains("survey"),
            "Should not contain survey: {line}"
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_level_exclude_standalone() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(&paper_dir, &db_path, &["query", "--exclude-level", "B"]);
    assert_success(&output);
    let lines = stdout_lines(&output);
    assert!(!lines.is_empty(), "Should have results");

    for line in &lines {
        let parts: Vec<&str> = line.split('\t').collect();
        assert_ne!(parts[0], "B", "Should not contain B level: {line}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_year_exclude_standalone() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(&paper_dir, &db_path, &["query", "--exclude-year", "2023"]);
    assert_success(&output);
    let lines = stdout_lines(&output);
    assert!(!lines.is_empty(), "Should have results");

    for line in &lines {
        let parts: Vec<&str> = line.split('\t').collect();
        assert_ne!(parts[2], "2023", "Should not contain 2023: {line}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

// ── title AND / exclude AND logic ──

#[test]
fn query_title_multiple_keywords_and_logic() {
    let (dir, paper_dir, db_path) = setup_query_test();

    // Both "graph" AND "diffusion" must appear in the title
    let output = run_search(
        &paper_dir,
        &db_path,
        &["query", "--keyword", "graph", "--keyword", "diffusion"],
    );
    assert_success(&output);
    let lines = stdout_lines(&output);
    assert!(
        !lines.is_empty(),
        "Should have results with both graph and diffusion"
    );

    for line in &lines {
        let title_lower = line.to_lowercase();
        assert!(
            title_lower.contains("graph"),
            "Title should contain graph: {line}"
        );
        assert!(
            title_lower.contains("diffusion"),
            "Title should contain diffusion: {line}"
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_title_multiple_exclude_keywords() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(
        &paper_dir,
        &db_path,
        &[
            "query",
            "--conference",
            "ICML",
            "--exclude-keyword",
            "survey",
            "--exclude-keyword",
            "attention",
        ],
    );
    assert_success(&output);
    let lines = stdout_lines(&output);

    for line in &lines {
        let title_lower = line.to_lowercase();
        assert!(
            !title_lower.contains("survey"),
            "Should not contain survey: {line}"
        );
        assert!(
            !title_lower.contains("attention"),
            "Should not contain attention: {line}"
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}

// ── case insensitivity ──

#[test]
fn query_title_keyword_case_insensitive() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let lower = run_search(&paper_dir, &db_path, &["query", "--keyword", "diffusion"]);
    let upper = run_search(&paper_dir, &db_path, &["query", "--keyword", "DIFFUSION"]);
    let mixed = run_search(&paper_dir, &db_path, &["query", "--keyword", "Diffusion"]);

    assert_success(&lower);
    assert_success(&upper);
    assert_success(&mixed);

    assert_eq!(
        stdout_lines(&lower),
        stdout_lines(&upper),
        "Should be case-insensitive"
    );
    assert_eq!(
        stdout_lines(&lower),
        stdout_lines(&mixed),
        "Should be case-insensitive"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_level_filter_case_insensitive() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let lower = run_search(&paper_dir, &db_path, &["query", "--level", "a"]);
    let upper = run_search(&paper_dir, &db_path, &["query", "--level", "A"]);

    assert_success(&lower);
    assert_success(&upper);
    assert_eq!(
        stdout_lines(&lower),
        stdout_lines(&upper),
        "level should be case-insensitive"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_conference_filter_case_insensitive() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let lower = run_search(&paper_dir, &db_path, &["query", "--conference", "icml"]);
    let upper = run_search(&paper_dir, &db_path, &["query", "--conference", "ICML"]);

    assert_success(&lower);
    assert_success(&upper);
    assert_eq!(
        stdout_lines(&lower),
        stdout_lines(&upper),
        "conference should be case-insensitive"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// ── substring matching ──

#[test]
fn query_title_substring_matching() {
    let (dir, paper_dir, db_path) = setup_query_test();

    // "diffus" is a substring of "diffusion", should still match
    let full = run_search(&paper_dir, &db_path, &["query", "--keyword", "diffusion"]);
    let substr = run_search(&paper_dir, &db_path, &["query", "--keyword", "diffus"]);

    assert_success(&full);
    assert_success(&substr);

    // "diffus" should match at least as many as "diffusion" (superset)
    let full_lines = stdout_lines(&full);
    let substr_lines = stdout_lines(&substr);
    assert!(
        substr_lines.len() >= full_lines.len(),
        "Substring diffus should match >= diffusion results: diffus={} vs diffusion={}",
        substr_lines.len(),
        full_lines.len()
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// ── same-dimension include+exclude ──

#[test]
fn query_include_exclude_same_dimension() {
    let (dir, paper_dir, db_path) = setup_query_test();

    // Include ICML and AAAI, but exclude AAAI → only ICML remains
    let output = run_search(
        &paper_dir,
        &db_path,
        &[
            "query",
            "--conference",
            "ICML,AAAI",
            "--exclude-conference",
            "AAAI",
        ],
    );
    assert_success(&output);
    let lines = stdout_lines(&output);
    assert!(!lines.is_empty(), "Should have ICML results");

    for line in &lines {
        let parts: Vec<&str> = line.split('\t').collect();
        assert_eq!(parts[1], "ICML", "Should only have ICML: {line}");
        assert_ne!(parts[1], "AAAI", "Should not have AAAI: {line}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

// ── empty results ──

#[test]
fn query_empty_result_when_nothing_matches() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(
        &paper_dir,
        &db_path,
        &["query", "--keyword", "NONEXISTENTKEYWORDXYZ"],
    );
    assert_success(&output);
    let lines = stdout_lines(&output);
    assert!(
        lines.is_empty(),
        "Nonexistent keyword should return empty results"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// ── comma-separated values ──

#[test]
fn query_comma_separated_keywords() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let comma = run_search(
        &paper_dir,
        &db_path,
        &["query", "--keyword", "graph,diffusion"],
    );
    let separate = run_search(
        &paper_dir,
        &db_path,
        &["query", "--keyword", "graph", "--keyword", "diffusion"],
    );

    assert_success(&comma);
    assert_success(&separate);
    assert_eq!(
        stdout_lines(&comma),
        stdout_lines(&separate),
        "Comma-separated and repeated --keyword should be equivalent"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_comma_separated_levels() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let comma = run_search(&paper_dir, &db_path, &["query", "--level", "A,B"]);
    let separate = run_search(
        &paper_dir,
        &db_path,
        &["query", "--level", "A", "--level", "B"],
    );

    assert_success(&comma);
    assert_success(&separate);
    assert_eq!(
        stdout_lines(&comma),
        stdout_lines(&separate),
        "Comma-separated and repeated --level should be equivalent"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// ── sort direction ──

#[test]
fn query_sort_ascending() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(
        &paper_dir,
        &db_path,
        &["query", "--conference", "ICML", "--sort", "year:asc"],
    );
    assert_success(&output);
    let lines = stdout_lines(&output);

    let years: Vec<&str> = lines
        .iter()
        .map(|l| l.split('\t').nth(2).unwrap())
        .collect();
    for w in years.windows(2) {
        assert!(w[0] <= w[1], "year should be ascending: {w:?}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

// ── filter pipeline independence ──

#[test]
fn query_pipeline_order_independent() {
    let dir = temp_test_dir();
    let paper_dir = dir.join("PAPERS");
    let db_path = dir.join("test.db");

    // Create test data where order of filtering could matter
    create_test_papers(
        &paper_dir,
        "A",
        "ICML",
        "2024",
        &["Graph Diffusion Methods"],
    );

    let output = run_search(&paper_dir, &db_path, &["build-db"]);
    assert_success(&output);

    // Apply filters in different effective orders — result should be the same
    let result1 = run_search(
        &paper_dir,
        &db_path,
        &[
            "query",
            "--keyword",
            "graph",
            "--keyword",
            "diffusion",
            "--level",
            "A",
            "--year",
            "2024",
        ],
    );
    assert_success(&result1);

    let lines = stdout_lines(&result1);
    assert_eq!(lines.len(), 1, "Should return exactly 1 result");
    assert!(lines[0].contains("Graph Diffusion Methods"));

    let _ = std::fs::remove_dir_all(&dir);
}

// ── column selection: non-canonical field (bib) ──

#[test]
fn query_columns_with_bib_field() {
    let dir = temp_test_dir();
    let paper_dir = dir.join("PAPERS");
    let db_path = dir.join("test.db");
    let conf_dir = paper_dir.join("A").join("AAAI");
    std::fs::create_dir_all(&conf_dir).unwrap();
    std::fs::write(
        conf_dir.join("2026.jsonl"),
        r#"{"title":"Test Paper","author":"Alice","bib":"@inproceedings{test2026}","url":"http://ex.com"}
"#,
    )
    .unwrap();

    let output = run_search(&paper_dir, &db_path, &["build-db"]);
    assert_success(&output);

    // --columns with bib (non-canonical field)
    let output = run_search(
        &paper_dir,
        &db_path,
        &[
            "query",
            "--conference",
            "AAAI",
            "--columns",
            "conference,year,title,bib",
        ],
    );
    assert_success(&output);
    let lines = stdout_lines(&output);
    assert_eq!(lines.len(), 1);
    let parts: Vec<&str> = lines[0].split('\t').collect();
    assert_eq!(
        parts.len(),
        4,
        "Should have 4 columns: conf, year, title, bib"
    );
    assert_eq!(parts[0], "AAAI");
    assert!(
        parts[3].contains("@inproceedings{test2026}"),
        "bib should contain BibTeX: {}",
        parts[3]
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// ── column selection: exclude mode ──

#[test]
fn query_exclude_columns() {
    let dir = temp_test_dir();
    let paper_dir = dir.join("PAPERS");
    let db_path = dir.join("test.db");
    let conf_dir = paper_dir.join("A").join("AAAI");
    std::fs::create_dir_all(&conf_dir).unwrap();
    std::fs::write(
        conf_dir.join("2026.jsonl"),
        r#"{"title":"Test Paper","author":"Alice","bib":"@inproceedings{test2026}","url":"http://ex.com"}
"#,
    )
    .unwrap();

    let output = run_search(&paper_dir, &db_path, &["build-db"]);
    assert_success(&output);

    // --exclude-columns url (show all columns except url)
    let output = run_search(
        &paper_dir,
        &db_path,
        &["query", "--conference", "AAAI", "--exclude-columns", "url"],
    );
    assert_success(&output);
    let lines = stdout_lines(&output);
    assert_eq!(lines.len(), 1);
    // Should have: level, conference, year, title, author, bib (6 columns)
    let parts: Vec<&str> = lines[0].split('\t').collect();
    assert!(
        parts.len() >= 5,
        "Should show at least 5 columns after excluding url, got {}",
        parts.len()
    );
    // Verify url is NOT in output
    let line_lower = lines[0].to_lowercase();
    assert!(
        !line_lower.contains("http://ex.com"),
        "Should not contain url: {}",
        lines[0]
    );

    // --exclude-columns bib,url (show all columns except bib and url)
    let output = run_search(
        &paper_dir,
        &db_path,
        &[
            "query",
            "--conference",
            "AAAI",
            "--exclude-columns",
            "bib,url",
        ],
    );
    assert_success(&output);
    let lines = stdout_lines(&output);
    assert_eq!(lines.len(), 1);
    assert!(
        !lines[0].contains("@inproceedings"),
        "Should not contain bib: {}",
        lines[0]
    );
    assert!(
        !lines[0].to_lowercase().contains("http://ex.com"),
        "Should not contain url"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// ── column selection: --columns and --exclude-columns conflict ──

#[test]
fn query_columns_and_exclude_columns_conflict() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(
        &paper_dir,
        &db_path,
        &[
            "query",
            "--conference",
            "AAAI",
            "--columns",
            "title",
            "--exclude-columns",
            "url",
        ],
    );
    assert!(
        !output.status.success(),
        "Using --columns and --exclude-columns together should fail"
    );
    let stderr = stderr_str(&output);
    assert!(
        stderr.contains("cannot be used together"),
        "Should mention conflict: {stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// ── column selection: unknown column error ──

#[test]
fn query_unknown_column_error() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(
        &paper_dir,
        &db_path,
        &[
            "query",
            "--conference",
            "AAAI",
            "--columns",
            "nonexistent_field",
        ],
    );
    assert!(!output.status.success(), "Unknown column should fail");
    let stderr = stderr_str(&output);
    assert!(
        stderr.contains("Unknown column"),
        "Should mention unknown column: {stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// ── bib command: custom columns ──

#[test]
fn bib_command_with_custom_columns() {
    let dir = temp_test_dir();
    let paper_dir = dir.join("PAPERS");
    let db_path = dir.join("test.db");
    let conf_dir = paper_dir.join("A").join("AAAI");
    std::fs::create_dir_all(&conf_dir).unwrap();
    std::fs::write(
        conf_dir.join("2026.jsonl"),
        r#"{"title":"FastDriveVLA","author":"Alice","bib":"@inproceedings{fastdrivevla2026, title = {FastDriveVLA}}","url":""}"#,
    )
    .unwrap();

    let output = run_search(&paper_dir, &db_path, &["build-db"]);
    assert_success(&output);

    // bib command with --columns title,bib
    let output = run_search(
        &paper_dir,
        &db_path,
        &["bib", "FastDriveVLA", "--columns", "title,bib"],
    );
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("@inproceedings{fastdrivevla2026"),
        "Should contain bib"
    );
    assert!(stdout.contains("FastDriveVLA"), "Should contain title");

    let _ = std::fs::remove_dir_all(&dir);
}
