use std::path::PathBuf;
use std::process::Command;

fn binary_path() -> &'static str {
    env!("CARGO_BIN_EXE_search")
}

fn paper_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("Paper")
}

fn run_search(args: &[&str]) -> std::process::Output {
    let mut command = Command::new(binary_path());
    command.arg("--paper-dir").arg(paper_dir());
    command.args(args);
    command.output().expect("failed to run search binary")
}

fn stdout_lines(output: &std::process::Output) -> Vec<String> {
    let stdout = String::from_utf8(output.stdout.clone()).expect("stdout is not utf-8");
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

fn assert_success(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "command failed: status={:?}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn title_matches_keywords(title: &str, include: &[&str], exclude: &[&str]) -> bool {
    let words: Vec<String> = title
        .split_whitespace()
        .map(|word| word.to_ascii_lowercase())
        .collect();

    include.iter().all(|keyword| {
        words
            .iter()
            .any(|word| word.contains(&keyword.to_ascii_lowercase()))
    }) && !exclude.iter().any(|keyword| {
        words
            .iter()
            .any(|word| word.contains(&keyword.to_ascii_lowercase()))
    })
}

#[test]
fn help_command_prints_usage() {
    let output = run_search(&["--help"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("--exclude-year"));
}

#[test]
fn positional_keywords_match_explicit_keyword_option() {
    let positional = run_search(&["graph", "diffusion"]);
    let explicit = run_search(&["--keyword", "graph,diffusion"]);

    assert_success(&positional);
    assert_success(&explicit);
    assert_eq!(positional.stdout, explicit.stdout);
}

#[test]
fn level_conference_year_filters_are_exact_and_case_insensitive() {
    let output = run_search(&["--level", "a", "--conference", "aaai", "--year", "2024"]);
    assert_success(&output);

    let lines = stdout_lines(&output);
    assert!(!lines.is_empty(), "expected results for AAAI 2024");

    for line in lines {
        let parts: Vec<&str> = line.split('\t').collect();
        assert_eq!(parts.len(), 4, "unexpected column count in line: {line}");
        assert_eq!(parts[0], "A");
        assert_eq!(parts[1], "AAAI");
        assert_eq!(parts[2], "2024");
    }
}

#[test]
fn keyword_include_and_exclude_arrays_filter_titles() {
    let output = run_search(&[
        "--keyword",
        "graph,diffusion",
        "--exclude-keyword",
        "survey,tutorial",
    ]);
    assert_success(&output);

    let lines = stdout_lines(&output);
    assert!(!lines.is_empty(), "expected keyword results");

    for line in lines {
        let parts: Vec<&str> = line.split('\t').collect();
        assert_eq!(parts.len(), 4, "unexpected column count in line: {line}");
        assert!(title_matches_keywords(
            parts[3],
            &["graph", "diffusion"],
            &["survey", "tutorial"]
        ));
    }
}

#[test]
fn scalar_exclude_filters_work_together() {
    let output = run_search(&[
        "--exclude-level",
        "B",
        "--exclude-conference",
        "AAAI,ICML",
        "--exclude-year",
        "2024,2025",
        "diffusion",
    ]);
    assert_success(&output);

    let lines = stdout_lines(&output);
    assert!(
        !lines.is_empty(),
        "expected results after scalar exclusions"
    );

    for line in lines {
        let parts: Vec<&str> = line.split('\t').collect();
        assert_eq!(parts.len(), 4, "unexpected column count in line: {line}");
        assert_ne!(parts[0], "B");
        assert_ne!(parts[1], "AAAI");
        assert_ne!(parts[1], "ICML");
        assert_ne!(parts[2], "2024");
        assert_ne!(parts[2], "2025");
        assert!(title_matches_keywords(parts[3], &["diffusion"], &[]));
    }
}

#[test]
fn mixed_filters_sorting_and_columns_are_honored() {
    let output = run_search(&[
        "--conference",
        "ICML,NeurIPS",
        "--exclude-year",
        "2025",
        "--sort",
        "conference:asc",
        "--sort",
        "year:desc",
        "--columns",
        "conference,year,title",
        "diffusion",
    ]);
    assert_success(&output);

    let lines = stdout_lines(&output);
    assert!(!lines.is_empty(), "expected sorted filtered results");

    let mut previous: Option<(String, u32, String)> = None;
    for line in lines {
        let parts: Vec<&str> = line.split('\t').collect();
        assert_eq!(
            parts.len(),
            3,
            "unexpected filtered column count in line: {line}"
        );
        assert!(matches!(parts[0], "ICML" | "NeurIPS"));
        assert_ne!(parts[1], "2025");
        assert!(title_matches_keywords(parts[2], &["diffusion"], &[]));

        let current = (
            parts[0].to_string(),
            parts[1].parse::<u32>().expect("year should be numeric"),
            parts[2].to_string(),
        );

        if let Some(previous) = &previous {
            assert!(
                previous.0 < current.0 || (previous.0 == current.0 && previous.1 >= current.1),
                "results are not sorted by conference asc, year desc: prev={previous:?}, current={current:?}"
            );
        }

        previous = Some(current);
    }
}
