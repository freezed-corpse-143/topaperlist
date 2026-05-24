---
name: topaperlist
description: Use when adding papers, editing the search CLI, updating README tables, running install/test/CI, or querying the paper database in the topaperlist project.
---

# Top Conferences Paper List

A curated collection of paper titles and metadata from top CS conferences, paired with a Rust CLI search tool (`search`) backed by SQLite.

## Cloning

If you haven't cloned the project yet:

```bash
git clone https://github.com/freezed-corpse-143/topaperlist.git
cd topaperlist
```

## Project Layout

```
PAPERS/<level>/<conference>/<year>.txt    ← paper titles (generated from jsonl)
PAPERS/<level>/<conference>/<year>.jsonl  ← structured metadata (title, author, bib, url)
search/                                   ← Rust CLI project (Cargo.toml, src/, tests/, sql/)
install.sh                                ← Linux/macOS installer
install.ps1                               ← Windows installer
README.md                                 ← English docs
README.zh.md                              ← Chinese docs
```

## Adding Paper Data

When adding new papers for a conference/year:

1. Create or update `PAPERS/<level>/<conference>/<year>.jsonl` with JSONL records:
   ```json
   {"title": "Paper Title", "author": "", "bib": "", "url": ""}
   ```
2. Regenerate the `.txt` file from the jsonl:
   ```bash
   jq -r '.title' PAPERS/<level>/<conference>/<year>.jsonl > PAPERS/<level>/<conference>/<year>.txt
   ```
3. If adding a new conference or year, update both README.md and README.zh.md tables.
4. Rebuild and copy the binary (`cargo post build --release` from `search/`), then rebuild the database: `./search build-db`

## Search Tool (`search/`)

Rust binary with subcommands:
- `search build-db` — build SQLite database from JSONL files
- `search query [filters]` — query papers with tab-separated output
- `search bib [filters]` — output BibTeX entries

### Build & Test

Requires [`cargo-post`](https://crates.io/crates/cargo-post) (`cargo install cargo-post`), which auto-copies the binary to the repo root after a successful build.

```bash
cd search
cargo post build --release    # compiles + copies binary to repo root
cargo test                    # 40 integration tests
```

Set env vars for development (optional; the tool auto-resolves paths from the binary location):
```bash
export PAPERS_DIR="$PWD/../PAPERS"
export PAPERS_DB_PATH="$PWD/../papers.db"
RUST_LOG=debug cargo run -- query --conference ICML diffusion
```

### Filter System

All filters are repeatable and case-insensitive. Include filters use IN/LIKE; exclude filters use NOT IN/NOT LIKE.

| Filter | Include | Exclude |
|--------|---------|---------|
| Title keyword | `-k`, `--keyword`, positional | `-x`, `--exclude`, `--exclude-keyword` |
| Level | `-l`, `--level` | `--exclude-level` |
| Conference | `-n`, `--conference` | `--exclude-conference` |
| Year | `-y`, `--year` | `--exclude-year` |

Sorting: `-s field:direction` (repeatable). Fields: level, conference, year, title.

Columns: `-c` (include mode), `-X` (exclude mode). Cannot use both simultaneously.

### SQL Layer

Generic filter templates under `search/sql/`:
- `filter_set.sql` — IN / NOT IN for set membership (level, conference, year)
- `filter_substring.sql` — LIKE / NOT LIKE for substring match (title)
- `projection.sql` — final column selection + ORDER BY

Filters compose via nested subqueries: each filter wraps the current inner query with `SELECT * FROM ({inner}) WHERE {condition}`.

### Install Scripts

- `install.sh` — Linux/macOS. Defaults: `~/.local/share/topaperlist`, `~/.local/bin/search`.
- `install.ps1` — Windows. Defaults: `%LOCALAPPDATA%\topaperlist`.

Both scripts are idempotent (safe to re-run), inject env vars into shell RC files with sentinel markers, and run a smoke test after install.

### CI

`.github/workflows/ci.yml` — runs on push/PR to main:
- `cargo test` matrix (ubuntu + windows)
- install.sh verification (smoke test, bib, columns, idempotent re-install)
- install.ps1 verification

## README Table Maintenance

Both README.md and README.zh.md contain conference × year matrices. When adding new data:
1. If a conference has no entry in the table, add a row in alphabetical order within the correct level section (A or B).
2. If a year column is new, add it to the header row and fill `-` for conferences without that year.
3. Keep both language versions in sync.
