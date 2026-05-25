mod test_utils;

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
