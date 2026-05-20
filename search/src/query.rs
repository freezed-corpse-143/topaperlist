use crate::cli::QueryArgs;
use crate::db::{self, debug};
use crate::models::{canonical_fields, Field, SortSpec};
use rusqlite::Connection;

pub type Result<T> = std::result::Result<T, String>;

pub fn run_query(args: QueryArgs, db_path: &std::path::Path) -> Result<()> {
    let db_path = if let Some(ref override_path) = args.db_path_override {
        std::path::PathBuf::from(override_path)
    } else {
        db_path.to_path_buf()
    };

    let conn = db::open_db(&db_path)?;
    let all_columns = db::get_all_columns(&conn)?;

    let display_fields = compute_display_fields(&args, &all_columns)?;

    let results = collect_query_results(&args, &conn, &display_fields)?;
    for line in results {
        println!("{line}");
    }

    Ok(())
}

/// Reorder fields so canonical fields come first (in fixed order), then non-canonical fields.
fn reorder_canonical_first(fields: &mut Vec<Field>) {
    let canonicals = canonical_fields();
    let mut result: Vec<Field> = Vec::new();
    for cf in &canonicals {
        if let Some(pos) = fields.iter().position(|f| f == cf) {
            result.push(fields.remove(pos));
        }
    }
    result.extend(fields.drain(..));
    *fields = result;
}

fn compute_display_fields(args: &QueryArgs, all_columns: &[Field]) -> Result<Vec<Field>> {
    if let Some(ref exclude_cols) = args.exclude_columns {
        if args.columns.is_some() {
            return Err("--columns 和 --exclude-columns 不能同时使用".to_string());
        }
        let excluded: Vec<Field> = exclude_cols.iter().map(|c| Field::parse(c)).collect();
        let mut result: Vec<Field> = all_columns
            .iter()
            .filter(|f| !excluded.contains(f))
            .cloned()
            .collect();
        if result.is_empty() {
            return Err("排除后无剩余列可显示".to_string());
        }
        reorder_canonical_first(&mut result);
        Ok(result)
    } else if let Some(ref include_cols) = args.columns {
        let col_str = include_cols.join(",");
        db::parse_columns(&col_str, all_columns)
    } else {
        Ok(canonical_fields())
    }
}

pub fn run_bib_query(args: QueryArgs, db_path: &std::path::Path) -> Result<()> {
    let db_path = if let Some(ref override_path) = args.db_path_override {
        std::path::PathBuf::from(override_path)
    } else {
        db_path.to_path_buf()
    };

    let conn = db::open_db(&db_path)?;

    let display_fields = if args.columns.is_some() || args.exclude_columns.is_some() {
        let all_columns = db::get_all_columns(&conn)?;
        compute_display_fields(&args, &all_columns)?
    } else {
        vec![Field("bib".to_string())]
    };

    let results = collect_query_results(&args, &conn, &display_fields)?;

    let mut wrote_entry = false;
    for entry in results {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        if wrote_entry {
            println!();
        }
        println!("{entry}");
        wrote_entry = true;
    }

    Ok(())
}

fn collect_query_results(
    args: &QueryArgs,
    conn: &Connection,
    display_fields: &[Field],
) -> Result<Vec<String>> {
    debug!("开始查询");

    // Merge positional keywords with --keyword
    let mut title_include: Vec<String> = args
        .keyword
        .iter()
        .chain(args.positional_keywords.iter())
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    title_include.dedup();

    let title_exclude: Vec<String> = args
        .exclude
        .iter()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    let level_include: Vec<String> = args
        .level
        .iter()
        .map(|s| s.trim().to_ascii_lowercase())
        .collect();

    let level_exclude: Vec<String> = args
        .exclude_level
        .iter()
        .map(|s| s.trim().to_ascii_lowercase())
        .collect();

    let conference_include: Vec<String> = args
        .conference
        .iter()
        .map(|s| s.trim().to_ascii_lowercase())
        .collect();

    let conference_exclude: Vec<String> = args
        .exclude_conference
        .iter()
        .map(|s| s.trim().to_ascii_lowercase())
        .collect();

    let year_include: Vec<String> = args.year.iter().map(|s| s.trim().to_string()).collect();

    let year_exclude: Vec<String> = args
        .exclude_year
        .iter()
        .map(|s| s.trim().to_string())
        .collect();

    // Validate: at least one filter
    let has_filter = !title_include.is_empty()
        || !title_exclude.is_empty()
        || !level_include.is_empty()
        || !level_exclude.is_empty()
        || !conference_include.is_empty()
        || !conference_exclude.is_empty()
        || !year_include.is_empty()
        || !year_exclude.is_empty();

    if !has_filter {
        return Err(
            "至少需要一个筛选条件: keyword/level/conference/year\n使用 --help 查看详细用法"
                .to_string(),
        );
    }

    // Parse sort specs
    let sort_specs: Vec<SortSpec> = args
        .sort
        .iter()
        .map(|s| db::parse_sort_spec(s))
        .collect::<db::Result<Vec<_>>>()?;

    let results = db::query_records(
        conn,
        &title_include,
        &title_exclude,
        &level_include,
        &level_exclude,
        &conference_include,
        &conference_exclude,
        &year_include,
        &year_exclude,
        &sort_specs,
        display_fields,
    )?;

    Ok(results)
}