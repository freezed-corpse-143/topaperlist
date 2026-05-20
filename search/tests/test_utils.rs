use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn binary_path() -> &'static str {
    env!("CARGO_BIN_EXE_search")
}

pub fn temp_test_dir() -> PathBuf {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    let path = std::env::temp_dir().join(format!("topaperlist-test-{pid}-{id}"));
    fs::create_dir_all(&path).unwrap();
    path
}

pub fn create_test_papers(
    root: &PathBuf,
    level: &str,
    conference: &str,
    year: &str,
    titles: &[&str],
) {
    let conf_dir = root.join(level).join(conference);
    fs::create_dir_all(&conf_dir).unwrap();

    // Write .txt
    let txt_path = conf_dir.join(format!("{year}.txt"));
    fs::write(&txt_path, titles.join("\n") + "\n").unwrap();

    // Write .jsonl
    let jsonl_path = conf_dir.join(format!("{year}.jsonl"));
    let jsonl_content: String = titles
        .iter()
        .map(|t| {
            format!(
                r#"{{"title":"{}","author":"","bib":"","url":""}}"#,
                t.replace('\\', "\\\\").replace('"', "\\\"")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&jsonl_path, jsonl_content + "\n").unwrap();
}

pub fn run_search(paper_dir: &PathBuf, db_path: &PathBuf, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(binary_path());
    cmd.env("PAPERS_DIR", paper_dir);
    cmd.env("PAPERS_DB_PATH", db_path);
    cmd.env("RUST_LOG", "debug");
    cmd.args(args);
    cmd.output().expect("Failed to run search")
}

pub fn stdout_lines(output: &std::process::Output) -> Vec<String> {
    String::from_utf8(output.stdout.clone())
        .expect("stdout is not utf-8")
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn stderr_str(output: &std::process::Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap_or_default()
}

pub fn assert_success(output: &std::process::Output) {
    if !output.status.success() {
        eprintln!("STDERR: {}", stderr_str(output));
    }
    assert!(output.status.success(), "Command execution failed");
}
