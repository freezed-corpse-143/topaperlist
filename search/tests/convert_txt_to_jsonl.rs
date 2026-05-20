/// One-shot script: converts PAPERS/<level>/<conf>/<year>.txt to .jsonl.
/// Run once: cargo test --test convert_txt_to_jsonl -- --ignored
///
/// Each line in .txt is a paper title. Output .jsonl has:
/// {"title": "...", "author": "", "bib": "", "url": ""}

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

fn papers_dir() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // search/ is inside the project root
    manifest.parent().unwrap().join("PAPERS")
}

#[test]
#[ignore]
fn convert_all_txt_to_jsonl() {
    let root = papers_dir();
    if !root.is_dir() {
        panic!("PAPERS directory not found: {}", root.display());
    }
    println!("Converting PAPERS directory: {}", root.display());

    let mut txt_count = 0;
    let mut jsonl_count = 0;
    let mut skip_count = 0;

    for level_entry in root.read_dir().expect("Failed to read PAPERS directory") {
        let level_entry = level_entry.unwrap();
        let level_path = level_entry.path();
        if !level_path.is_dir() {
            continue;
        }

        for conf_entry in level_path.read_dir().unwrap() {
            let conf_entry = conf_entry.unwrap();
            let conf_path = conf_entry.path();
            if !conf_path.is_dir() {
                continue;
            }

            for file_entry in conf_path.read_dir().unwrap() {
                let file_entry = file_entry.unwrap();
                let file_path = file_entry.path();
                if file_path.extension().and_then(|s| s.to_str()) != Some("txt") {
                    continue;
                }

                txt_count += 1;
                let jsonl_path = file_path.with_extension("jsonl");

                if jsonl_path.exists() {
                    println!("Skipping existing: {}", jsonl_path.display());
                    skip_count += 1;
                    continue;
                }

                let txt_file = fs::File::open(&file_path)
                    .unwrap_or_else(|e| panic!("Failed to open {}: {}", file_path.display(), e));
                let reader = BufReader::new(txt_file);

                let mut jsonl_file = fs::File::create(&jsonl_path)
                    .unwrap_or_else(|e| panic!("Failed to create {}: {}", jsonl_path.display(), e));

                for line in reader.lines() {
                    let title = line.unwrap().trim().to_string();
                    if title.is_empty() {
                        continue;
                    }
                    let record = serde_json::json!({
                        "title": title,
                        "author": "",
                        "bib": "",
                        "url": ""
                    });
                    writeln!(
                        jsonl_file,
                        "{}",
                        serde_json::to_string(&record).unwrap()
                    )
                    .unwrap();
                }

                jsonl_count += 1;
                println!("Generated: {}", jsonl_path.display());
            }
        }
    }

    println!(
        "Conversion complete: {} .txt files, {} new .jsonl, {} skipped",
        txt_count, jsonl_count, skip_count
    );
}
