mod test_utils;

use test_utils::*;

// ── build-db tests ──

#[test]
fn build_db_creates_database() {
    let dir = temp_test_dir();
    let paper_dir = dir.join("PAPERS");
    let db_path = dir.join("test.db");

    create_test_papers(
        &paper_dir, "A", "ICML", "2024",
        &["First Paper", "Second Paper"],
    );
    create_test_papers(
        &paper_dir, "B", "EMNLP", "2025",
        &["Third Paper"],
    );

    let output = run_search(&paper_dir, &db_path, &["build-db"]);
    assert_success(&output);

    assert!(db_path.exists(), "数据库文件应存在");
    let meta = std::fs::metadata(&db_path).unwrap();
    assert!(meta.len() > 100, "数据库文件应有内容");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_db_reports_missing_papers_dir() {
    let dir = temp_test_dir();
    let nonexistent = dir.join("NONEXISTENT");
    let db_path = dir.join("test.db");

    let output = run_search(&nonexistent, &db_path, &["build-db"]);
    assert!(!output.status.success(), "不存在目录时应失败");
    let stderr = stderr_str(&output);
    assert!(stderr.contains("不存在"), "应提示目录不存在: {stderr}");

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
    assert!(!output.status.success(), "无 JSONL 文件时应失败");
    let stderr = stderr_str(&output);
    assert!(
        stderr.to_lowercase().contains("jsonl"),
        "应提示无 JSONL: {stderr}"
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
    assert!(!output.status.success(), "JSONL 格式错误时应失败");
    let stderr = stderr_str(&output);
    assert!(
        stderr.contains("格式错误") || stderr.contains("JSONL"),
        "应提示格式错误: {stderr}"
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
    assert!(stdout.contains("title"), "应检测到 title 字段");
    assert!(stdout.contains("author"), "应检测到 author 字段");

    let _ = std::fs::remove_dir_all(&dir);
}

// ── query tests ──

fn setup_query_test() -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
    let dir = temp_test_dir();
    let paper_dir = dir.join("PAPERS");
    let db_path = dir.join("test.db");

    create_test_papers(
        &paper_dir, "A", "AAAI", "2024",
        &[
            "Diffusion Models for Image Generation",
            "Graph Neural Networks for Drug Discovery",
        ],
    );
    create_test_papers(
        &paper_dir, "A", "ICML", "2024",
        &["A Survey of Graph Diffusion Models"],
    );
    create_test_papers(
        &paper_dir, "A", "ICML", "2023",
        &["Attention Mechanisms in Deep Learning"],
    );
    create_test_papers(
        &paper_dir, "B", "EMNLP", "2024",
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
    assert!(!lines.is_empty(), "应有 diffusion 结果");

    for line in &lines {
        assert!(
            line.to_lowercase().contains("diffusion"),
            "每条结果应包含 diffusion: {line}"
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
        assert_eq!(parts[0], "A", "level 应为 A: {line}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_with_conference_filter() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(&paper_dir, &db_path, &["query", "--conference", "ICML"]);
    assert_success(&output);
    let lines = stdout_lines(&output);
    assert!(!lines.is_empty(), "应有 ICML 结果");

    for line in &lines {
        let parts: Vec<&str> = line.split('\t').collect();
        assert_eq!(parts[1], "ICML", "conference 应为 ICML: {line}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_with_year_filter() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(&paper_dir, &db_path, &["query", "--year", "2023"]);
    assert_success(&output);
    let lines = stdout_lines(&output);
    assert!(!lines.is_empty(), "应有 2023 结果");

    for line in &lines {
        let parts: Vec<&str> = line.split('\t').collect();
        assert_eq!(parts[2], "2023", "year 应为 2023: {line}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_with_exclude_filters() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(&paper_dir, &db_path, &[
        "query",
        "--exclude-level", "B",
        "--exclude-keyword", "survey",
        "diffusion",
    ]);
    assert_success(&output);
    let lines = stdout_lines(&output);
    assert!(!lines.is_empty(), "应有结果");

    for line in &lines {
        let parts: Vec<&str> = line.split('\t').collect();
        assert_ne!(parts[0], "B", "不应包含 B level");
        let title_lower = parts[3].to_lowercase();
        assert!(!title_lower.contains("survey"), "不应包含 survey: {line}");
        assert!(title_lower.contains("diffusion"), "应包含 diffusion: {line}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_with_combined_filters() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(&paper_dir, &db_path, &[
        "query",
        "--level", "A",
        "--conference", "ICML,AAAI",
        "--year", "2024",
        "--keyword", "graph",
    ]);
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

    let output = run_search(&paper_dir, &db_path, &[
        "query",
        "--conference", "ICML",
        "--sort", "year:desc",
        "--sort", "title:asc",
    ]);
    assert_success(&output);
    let lines = stdout_lines(&output);

    let years: Vec<&str> = lines
        .iter()
        .map(|l| l.split('\t').nth(2).unwrap())
        .collect();
    for w in years.windows(2) {
        assert!(w[0] >= w[1], "year 应降序: {w:?}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_with_custom_columns() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(&paper_dir, &db_path, &[
        "query",
        "--conference", "AAAI",
        "--columns", "conference,year,title",
    ]);
    assert_success(&output);
    let lines = stdout_lines(&output);

    for line in &lines {
        let parts: Vec<&str> = line.split('\t').collect();
        assert_eq!(parts.len(), 3, "列数应为 3: {line}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_requires_at_least_one_filter() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(&paper_dir, &db_path, &["query"]);
    assert!(!output.status.success(), "无筛选条件时应失败");
    let stderr = stderr_str(&output);
    assert!(stderr.contains("筛选条件"), "应提示需要筛选条件: {stderr}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_positional_keywords_work_like_explicit() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let positional = run_search(&paper_dir, &db_path, &["query", "graph", "diffusion"]);
    let explicit = run_search(&paper_dir, &db_path, &["query", "--keyword", "graph,diffusion"]);

    assert_success(&positional);
    assert_success(&explicit);
    assert_eq!(
        stdout_lines(&positional),
        stdout_lines(&explicit),
        "positional 和 --keyword 结果应相同"
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

    let output = run_search(&paper_dir, &db_path, &[
        "query",
        "--conference", "ICML",
        "--exclude-year", "2023",
    ]);
    assert_success(&output);
    let lines = stdout_lines(&output);

    for line in &lines {
        let parts: Vec<&str> = line.split('\t').collect();
        assert_ne!(parts[2], "2023", "不应包含 2023: {line}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_exclude_conference() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(&paper_dir, &db_path, &[
        "query",
        "--level", "A",
        "--exclude-conference", "AAAI",
    ]);
    assert_success(&output);
    let lines = stdout_lines(&output);

    for line in &lines {
        let parts: Vec<&str> = line.split('\t').collect();
        assert_ne!(parts[1], "AAAI", "不应包含 AAAI: {line}");
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
    assert!(stdout.contains("Usage"), "应显示使用说明");

    let _ = std::fs::remove_dir_all(&dir);
}

// ── 独立排除过滤测试 ──

#[test]
fn query_title_exclude_standalone() {
    let (dir, paper_dir, db_path) = setup_query_test();

    // Only exclude keywords, no include filter
    let output = run_search(&paper_dir, &db_path, &["query", "--exclude-keyword", "survey"]);
    assert_success(&output);
    let lines = stdout_lines(&output);
    assert!(!lines.is_empty(), "应有结果");

    for line in &lines {
        let parts: Vec<&str> = line.split('\t').collect();
        let title_lower = parts[3].to_lowercase();
        assert!(!title_lower.contains("survey"), "不应包含 survey: {line}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_level_exclude_standalone() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(&paper_dir, &db_path, &["query", "--exclude-level", "B"]);
    assert_success(&output);
    let lines = stdout_lines(&output);
    assert!(!lines.is_empty(), "应有结果");

    for line in &lines {
        let parts: Vec<&str> = line.split('\t').collect();
        assert_ne!(parts[0], "B", "不应包含 B level: {line}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_year_exclude_standalone() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(&paper_dir, &db_path, &["query", "--exclude-year", "2023"]);
    assert_success(&output);
    let lines = stdout_lines(&output);
    assert!(!lines.is_empty(), "应有结果");

    for line in &lines {
        let parts: Vec<&str> = line.split('\t').collect();
        assert_ne!(parts[2], "2023", "不应包含 2023: {line}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

// ── 标题关键词 AND / 排除 AND 逻辑 ──

#[test]
fn query_title_multiple_keywords_and_logic() {
    let (dir, paper_dir, db_path) = setup_query_test();

    // Both "graph" AND "diffusion" must appear in the title
    let output = run_search(&paper_dir, &db_path, &["query", "--keyword", "graph", "--keyword", "diffusion"]);
    assert_success(&output);
    let lines = stdout_lines(&output);
    assert!(!lines.is_empty(), "应有同时包含 graph 和 diffusion 的结果");

    for line in &lines {
        let title_lower = line.to_lowercase();
        assert!(title_lower.contains("graph"), "标题应包含 graph: {line}");
        assert!(title_lower.contains("diffusion"), "标题应包含 diffusion: {line}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_title_multiple_exclude_keywords() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(
        &paper_dir, &db_path,
        &["query", "--conference", "ICML", "--exclude-keyword", "survey", "--exclude-keyword", "attention"],
    );
    assert_success(&output);
    let lines = stdout_lines(&output);

    for line in &lines {
        let title_lower = line.to_lowercase();
        assert!(!title_lower.contains("survey"), "不应包含 survey: {line}");
        assert!(!title_lower.contains("attention"), "不应包含 attention: {line}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

// ── 大小写不敏感 ──

#[test]
fn query_title_keyword_case_insensitive() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let lower = run_search(&paper_dir, &db_path, &["query", "--keyword", "diffusion"]);
    let upper = run_search(&paper_dir, &db_path, &["query", "--keyword", "DIFFUSION"]);
    let mixed = run_search(&paper_dir, &db_path, &["query", "--keyword", "Diffusion"]);

    assert_success(&lower);
    assert_success(&upper);
    assert_success(&mixed);

    assert_eq!(stdout_lines(&lower), stdout_lines(&upper), "大小写应不敏感");
    assert_eq!(stdout_lines(&lower), stdout_lines(&mixed), "大小写应不敏感");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_level_filter_case_insensitive() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let lower = run_search(&paper_dir, &db_path, &["query", "--level", "a"]);
    let upper = run_search(&paper_dir, &db_path, &["query", "--level", "A"]);

    assert_success(&lower);
    assert_success(&upper);
    assert_eq!(stdout_lines(&lower), stdout_lines(&upper), "level 大小写应不敏感");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_conference_filter_case_insensitive() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let lower = run_search(&paper_dir, &db_path, &["query", "--conference", "icml"]);
    let upper = run_search(&paper_dir, &db_path, &["query", "--conference", "ICML"]);

    assert_success(&lower);
    assert_success(&upper);
    assert_eq!(stdout_lines(&lower), stdout_lines(&upper), "conference 大小写应不敏感");

    let _ = std::fs::remove_dir_all(&dir);
}

// ── 子串匹配 ──

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
    assert!(substr_lines.len() >= full_lines.len(),
        "子串 diffus 应匹配不少于 diffusion 的结果: diffus={} vs diffusion={}",
        substr_lines.len(), full_lines.len());

    let _ = std::fs::remove_dir_all(&dir);
}

// ── 同维度 include + exclude 组合 ──

#[test]
fn query_include_exclude_same_dimension() {
    let (dir, paper_dir, db_path) = setup_query_test();

    // Include ICML and AAAI, but exclude AAAI → only ICML remains
    let output = run_search(&paper_dir, &db_path, &[
        "query",
        "--conference", "ICML,AAAI",
        "--exclude-conference", "AAAI",
    ]);
    assert_success(&output);
    let lines = stdout_lines(&output);
    assert!(!lines.is_empty(), "应有 ICML 结果");

    for line in &lines {
        let parts: Vec<&str> = line.split('\t').collect();
        assert_eq!(parts[1], "ICML", "只应有 ICML: {line}");
        assert_ne!(parts[1], "AAAI", "不应有 AAAI: {line}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

// ── 空结果 ──

#[test]
fn query_empty_result_when_nothing_matches() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(&paper_dir, &db_path, &["query", "--keyword", "NONEXISTENTKEYWORDXYZ"]);
    assert_success(&output);
    let lines = stdout_lines(&output);
    assert!(lines.is_empty(), "不存在的关键词应返回空结果");

    let _ = std::fs::remove_dir_all(&dir);
}

// ── 逗号分隔值 ──

#[test]
fn query_comma_separated_keywords() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let comma = run_search(&paper_dir, &db_path, &["query", "--keyword", "graph,diffusion"]);
    let separate = run_search(&paper_dir, &db_path, &["query", "--keyword", "graph", "--keyword", "diffusion"]);

    assert_success(&comma);
    assert_success(&separate);
    assert_eq!(stdout_lines(&comma), stdout_lines(&separate), "逗号分隔和多次 --keyword 应等价");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_comma_separated_levels() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let comma = run_search(&paper_dir, &db_path, &["query", "--level", "A,B"]);
    let separate = run_search(&paper_dir, &db_path, &["query", "--level", "A", "--level", "B"]);

    assert_success(&comma);
    assert_success(&separate);
    assert_eq!(stdout_lines(&comma), stdout_lines(&separate), "逗号分隔和多次 --level 应等价");

    let _ = std::fs::remove_dir_all(&dir);
}

// ── 排序方向验证 ──

#[test]
fn query_sort_ascending() {
    let (dir, paper_dir, db_path) = setup_query_test();

    let output = run_search(&paper_dir, &db_path, &[
        "query",
        "--conference", "ICML",
        "--sort", "year:asc",
    ]);
    assert_success(&output);
    let lines = stdout_lines(&output);

    let years: Vec<&str> = lines
        .iter()
        .map(|l| l.split('\t').nth(2).unwrap())
        .collect();
    for w in years.windows(2) {
        assert!(w[0] <= w[1], "year 应升序: {w:?}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

// ── 多 filter 管道顺序独立验证 ──

#[test]
fn query_pipeline_order_independent() {
    let dir = temp_test_dir();
    let paper_dir = dir.join("PAPERS");
    let db_path = dir.join("test.db");

    // Create test data where order of filtering could matter
    create_test_papers(
        &paper_dir, "A", "ICML", "2024",
        &["Graph Diffusion Methods"],
    );

    let output = run_search(&paper_dir, &db_path, &["build-db"]);
    assert_success(&output);

    // Apply filters in different effective orders — result should be the same
    let result1 = run_search(&paper_dir, &db_path, &[
        "query", "--keyword", "graph", "--keyword", "diffusion", "--level", "A", "--year", "2024",
    ]);
    assert_success(&result1);

    let lines = stdout_lines(&result1);
    assert_eq!(lines.len(), 1, "应精确返回 1 条结果");
    assert!(lines[0].contains("Graph Diffusion Methods"));

    let _ = std::fs::remove_dir_all(&dir);
}
