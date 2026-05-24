# Changelog

## v2.2.0-dev (Unreleased)

- **Flat install layout**: remove `bin/` subdirectory — binary, `papers.db`, and `PAPERS/` now sit side-by-side in the install root. Default DB/PAPERS resolution prioritizes `<exe>/papers.db` and `<exe>/PAPERS` (same-directory layout), with cwd and parent-directory fallbacks for legacy compat. (build_db.rs, install.ps1, install.sh)
- **Post-build auto-copy via `cargo-post`**: add `post_build.rs` — after `cargo post build --release` the binary is automatically copied to the repo root. Removes the need for manual copy or separate build scripts. (Cargo.toml, post_build.rs)

## v2.1.0-dev (Unreleased)

- **Column include/exclude modes**: add `--exclude-columns` / `-X` flag for showing all columns except specified ones. Fix `--columns` to support non-canonical fields (`bib`, `author`, `url`, etc.) — previously these were silently dropped. `search bib` now accepts `--columns` / `--exclude-columns` for custom field output.
- **Generic SQL filter templates**: consolidate 8 field-specific filter templates into 2 generic templates — `filter_set.sql` (IN / NOT IN for level, conference, year) and `filter_substring.sql` (LIKE / NOT LIKE for title). Extract `apply_set_filter` / `apply_like_filter` helpers in `db.rs`, halving `query_records` line count.
- **Install script overhaul**: idempotent upgrade flow — safe to run repeatedly. Shell RC env-var injection (`PAPERS_DIR`, `PAPERS_DB_PATH`, `PATH`) with sentinel markers for clean removal. UTF-8 output guard. Cargo detection without auto-install (warns and exits). All messages in English. (install.sh, install.ps1)
- **CI workflow**: new `.github/workflows/ci.yml` — runs `cargo test` matrix (ubuntu + windows) and verifies `install.sh` / `install.ps1` smoke tests, bib export, column selection, and idempotent re-install on every push and PR to main.
- **Tests**: 5 new integration tests for column selection features (bib field, exclude mode, conflict detection, unknown column error, bib command with custom columns). 40 tests total.

## v2.0.0-dev

- **CLI BibTeX query**: add `search bib ...` / `search b ...` for printing BibTeX entries using the same filters as `search query`.
- **Installer robustness**: install scripts now report new install vs replacement vs legacy `PaperJson` upgrade, clean legacy data, persist Windows data-path environment variables, and avoid treating native stderr output as install failure.

- **Paper data update**: refresh paper data across all venues and years (add 2026 data, remove CRYPTO 2025-2026, trim EACL).
- **Auto-generated TXT files**: txt files are now generated from jsonl (`jq -r '.title'`) — no longer maintained manually.
- **SQL placeholder distinction**: all filter SQL templates use field-distinctive placeholders; exclude filters uniformly use `NOT IN`.
- **Architecture restructure**: Rust project moved into `search/` subdirectory; code is transparent to end users.
- **Data directory rename**: `Paper/` → `PAPERS/`.
- **JSONL support**: each `<year>.txt` is paired with `<year>.jsonl` containing structured metadata (`{"title","author","bib","url"}`).
- **Read-query separation**: `search build-db` reads JSONL and builds a SQLite database; `search query` reads from the database.
- **SQL extension layer**: modular filter templates under `sql/` (title/level/conference/year include/exclude), composed via nested subqueries.
- **Dynamic field detection**: detect JSONL schema from the first line — no hardcoded column list needed.
- **Environment variable configuration**: `PAPERS_DIR` and `PAPERS_DB_PATH` replace hardcoded paths; `RUST_LOG=debug` enables debug logging.
- **User-friendly error messages**: clear error messages for missing directories, malformed files, schema mismatches, etc.
- **Test coverage**: 34 integration tests covering all filter types, case insensitivity, substring matching, and pipeline combinations.

## v1.0.0-dev

- Promote the Rust CLI to a root-level Cargo project.
- Build the `search` binary from source instead of committing prebuilt binaries.
- Add Windows and Unix build/install scripts.
- Install paper data next to the installed binary so the CLI works without `--paper-dir`.
- Add CLI regression tests and install-time smoke tests.
