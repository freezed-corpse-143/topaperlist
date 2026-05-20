# Changelog

## v2.0.0-dev (Unreleased)

- **CLI BibTeX query**: add `search bib ...` / `search b ...` for printing BibTeX entries using the same filters as `search query`.
- **Installer robustness**: install scripts now report new install vs replacement vs legacy `PaperJson` upgrade, clean legacy data, persist Windows data-path environment variables, and avoid treating native stderr output as install failure.

- **论文数据更新**: 更新所有会议和年份的论文数据（新增 2026 年数据、CRYPTO 2025-2026 移除、EACL 精简）。
- **TXT 自动生成**: txt 文件由 jsonl 自动生成（`jq -r '.title'`），不再手动维护。
- **SQL 占位符区分**: 所有过滤器 SQL 模板使用有区分度的占位符（`{values}`, `{like_clauses}`, `{not_like_clauses}`），排除过滤器统一使用 `NOT IN` 模式。
- **架构重构**: Rust 项目封装到 `search/` 子目录，代码对用户透明。
- **数据目录重命名**: `Paper/` → `PAPERS/`。
- **JSONL 支持**: 每个 `<year>.txt` 对应 `<year>.jsonl`，元数据结构为 `{"title","author","bib","url"}`。
- **读查分离**: `search build-db` 读取 JSONL 构建 SQLite 数据库；`search query` 从数据库查询。
- **SQL 扩展层**: `sql/` 目录包含模块化过滤器（标题/等级/会议/年份的包含/排除），通过嵌套子查询管道组合。
- **动态字段检测**: 从 JSONL 首行自动检测字段结构，无需硬编码。
- **环境变量配置**: `PAPERS_DIR` 和 `PAPERS_DB_PATH` 取代硬编码路径；`RUST_LOG=debug` 启用调试日志。
- **用户友好错误提示**: 目录缺失、结构不符、格式错误等提供明确中文错误信息。
- **测试覆盖**: 34 个集成测试，覆盖所有过滤器、大小写、子串匹配、管道组合等场景。

## v1.0.0-dev

- Promote the Rust CLI to a root-level Cargo project.
- Build the `search` binary from source instead of committing prebuilt binaries.
- Add Windows and Unix build/install scripts.
- Install paper data next to the installed binary so the CLI works without `--paper-dir`.
- Add CLI regression tests and install-time smoke tests.
