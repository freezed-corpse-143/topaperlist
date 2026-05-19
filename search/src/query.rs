use crate::cli::QueryArgs;
use crate::db::{self, debug};
use crate::models::{canonical_fields, Field, SortSpec};

pub type Result<T> = std::result::Result<T, String>;

pub fn run_query(args: QueryArgs, db_path: &std::path::Path) -> Result<()> {
    let display_fields: Vec<Field> = if let Some(ref cols) = args.columns {
        let col_str = cols.join(",");
        db::parse_columns(&col_str)?
    } else {
        canonical_fields()
    };

    let results = collect_query_results(args, db_path, &display_fields)?;
    for line in results {
        println!("{line}");
    }

    Ok(())
}

pub fn run_bib_query(args: QueryArgs, db_path: &std::path::Path) -> Result<()> {
    let display_fields = vec![Field("bib".to_string())];
    let results = collect_query_results(args, db_path, &display_fields)?;

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
    args: QueryArgs,
    db_path: &std::path::Path,
    display_fields: &[Field],
) -> Result<Vec<String>> {
    debug!("开始查询");

    // Override db path if specified in args
    let db_path = if let Some(ref override_path) = args.db_path_override {
        std::path::PathBuf::from(override_path)
    } else {
        db_path.to_path_buf()
    };

    let conn = db::open_db(&db_path)?;

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
        &conn,
        &title_include,
        &title_exclude,
        &level_include,
        &level_exclude,
        &conference_include,
        &conference_exclude,
        &year_include,
        &year_exclude,
        &sort_specs,
        &display_fields,
    )?;

    Ok(results)
}
