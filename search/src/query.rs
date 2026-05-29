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

    let display_fields = compute_query_display_fields(&args, &all_columns)?;

    let results = collect_query_results(&args, &conn, &display_fields)?;
    if !args.title.is_empty() {
        print_title_query_results(&display_fields, &results);
    } else {
        for line in results {
            println!("{line}");
        }
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
            return Err("--columns and --exclude-columns cannot be used together".to_string());
        }
        let excluded: Vec<Field> = exclude_cols.iter().map(|c| Field::parse(c)).collect();
        let mut result: Vec<Field> = all_columns
            .iter()
            .filter(|f| !excluded.contains(f))
            .cloned()
            .collect();
        if result.is_empty() {
            return Err("No columns left to display after exclusion".to_string());
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

fn compute_query_display_fields(args: &QueryArgs, all_columns: &[Field]) -> Result<Vec<Field>> {
    if !args.title.is_empty() {
        let mut result = all_columns.to_vec();
        reorder_canonical_first(&mut result);
        Ok(result)
    } else {
        compute_display_fields(args, all_columns)
    }
}

fn print_title_query_results(display_fields: &[Field], results: &[String]) {
    let bib_index = display_fields
        .iter()
        .position(|field| field.as_str() == "bib");

    for (record_idx, line) in results.iter().enumerate() {
        if record_idx > 0 {
            println!();
        }

        let values: Vec<&str> = line.split('\t').collect();
        let mut rows = Vec::new();
        for (field_idx, field) in display_fields.iter().enumerate() {
            if Some(field_idx) == bib_index {
                continue;
            }
            let value = values.get(field_idx).copied().unwrap_or("");
            rows.push((
                field.as_str().to_string(),
                format_title_table_value(field, value),
            ));
        }
        print_terminal_table(&rows);

        if let Some(idx) = bib_index {
            let bib = values.get(idx).copied().unwrap_or("").trim();
            if !bib.is_empty() {
                println!();
                println!("{bib}");
            }
        }
    }
}

fn print_terminal_table(rows: &[(String, String)]) {
    let field_width = rows
        .iter()
        .flat_map(|(field, _)| cell_lines(field))
        .chain(std::iter::once("Field"))
        .map(display_width)
        .max()
        .unwrap_or(0);

    let max_value_width = terminal_table_width()
        .saturating_sub(field_width + 7)
        .max(30);
    let wrapped_rows: Vec<(String, Vec<String>)> = rows
        .iter()
        .map(|(field, value)| (field.clone(), wrap_cell(value, max_value_width)))
        .collect();

    let value_width = wrapped_rows
        .iter()
        .flat_map(|(_, value_lines)| value_lines.iter().map(String::as_str))
        .chain(std::iter::once("Value"))
        .map(display_width)
        .max()
        .unwrap_or(0);
    let header_value = vec!["Value".to_string()];

    print_table_border(field_width, value_width);
    print_table_row(&["Field"], &header_value, field_width, value_width);
    print_table_border(field_width, value_width);
    for (field, value_lines) in &wrapped_rows {
        print_table_row(&[field.as_str()], value_lines, field_width, value_width);
    }
    print_table_border(field_width, value_width);
}

fn terminal_table_width() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(120)
        .clamp(60, 120)
}

fn print_table_border(field_width: usize, value_width: usize) {
    println!(
        "+{}+{}+",
        "-".repeat(field_width + 2),
        "-".repeat(value_width + 2)
    );
}

fn print_table_row(
    field_lines: &[&str],
    value_lines: &[String],
    field_width: usize,
    value_width: usize,
) {
    let line_count = field_lines.len().max(value_lines.len());

    for idx in 0..line_count {
        let field_line = field_lines.get(idx).copied().unwrap_or("");
        let value_line = value_lines.get(idx).map(String::as_str).unwrap_or("");
        println!(
            "| {}{} | {}{} |",
            field_line,
            " ".repeat(field_width.saturating_sub(display_width(field_line))),
            value_line,
            " ".repeat(value_width.saturating_sub(display_width(value_line)))
        );
    }
}

fn clean_table_cell(value: &str) -> String {
    value.trim().replace('\r', "")
}

fn format_title_table_value(field: &Field, value: &str) -> String {
    let value = clean_table_cell(value);
    if field.as_str() == "author" {
        abbreviate_author(&value)
    } else {
        value
    }
}

fn abbreviate_author(value: &str) -> String {
    const MAX_AUTHOR_WIDTH: usize = 80;
    const SUFFIX: &str = ".etc";

    if display_width(value) <= MAX_AUTHOR_WIDTH {
        return value.to_string();
    }

    let target_width = MAX_AUTHOR_WIDTH.saturating_sub(display_width(SUFFIX));
    let prefix = abbreviate_prefix(value, target_width);
    format!("{prefix}{SUFFIX}")
}

fn abbreviate_prefix(value: &str, max_width: usize) -> String {
    let mut prefix = take_display_width(value, max_width);
    if let Some(pos) = prefix.rfind(';') {
        prefix.truncate(pos);
    } else if let Some(pos) = prefix.rfind(' ') {
        prefix.truncate(pos);
    }

    let prefix = prefix.trim_end_matches(&[' ', ';'][..]).trim_end();
    if prefix.is_empty() {
        take_display_width(value, max_width)
    } else {
        prefix.to_string()
    }
}

fn take_display_width(value: &str, max_width: usize) -> String {
    let mut result = String::new();
    let mut width = 0;

    for ch in value.chars() {
        let ch_width = if is_wide_char(ch) { 2 } else { 1 };
        if width + ch_width > max_width {
            break;
        }
        result.push(ch);
        width += ch_width;
    }

    result.trim_end().to_string()
}

fn wrap_cell(value: &str, max_width: usize) -> Vec<String> {
    let mut wrapped = Vec::new();
    for line in cell_lines(value) {
        wrapped.extend(wrap_line(line, max_width));
    }
    if wrapped.is_empty() {
        vec![String::new()]
    } else {
        wrapped
    }
}

fn wrap_line(value: &str, max_width: usize) -> Vec<String> {
    let value = value.trim();
    if value.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0;

    for word in value.split_whitespace() {
        let word_width = display_width(word);
        if word_width > max_width {
            if !current.is_empty() {
                lines.push(current);
                current = String::new();
                current_width = 0;
            }
            lines.extend(wrap_long_word(word, max_width));
        } else if current.is_empty() {
            current.push_str(word);
            current_width = word_width;
        } else if current_width + 1 + word_width <= max_width {
            current.push(' ');
            current.push_str(word);
            current_width += 1 + word_width;
        } else {
            lines.push(current);
            current = word.to_string();
            current_width = word_width;
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

fn wrap_long_word(value: &str, max_width: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_width = 0;

    for ch in value.chars() {
        let ch_width = if is_wide_char(ch) { 2 } else { 1 };
        if current_width > 0 && current_width + ch_width > max_width {
            chunks.push(current);
            current = String::new();
            current_width = 0;
        }
        current.push(ch);
        current_width += ch_width;
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}

fn cell_lines(value: &str) -> Vec<&str> {
    let lines: Vec<&str> = value.split('\n').collect();
    if lines.is_empty() {
        vec![""]
    } else {
        lines
    }
}

fn display_width(value: &str) -> usize {
    value
        .chars()
        .map(|ch| if is_wide_char(ch) { 2 } else { 1 })
        .sum()
}

fn is_wide_char(ch: char) -> bool {
    matches!(
        ch as u32,
        0x1100..=0x115f
            | 0x2329..=0x232a
            | 0x2e80..=0xa4cf
            | 0xac00..=0xd7a3
            | 0xf900..=0xfaff
            | 0xfe10..=0xfe19
            | 0xfe30..=0xfe6f
            | 0xff00..=0xff60
            | 0xffe0..=0xffe6
            | 0x1f300..=0x1f64f
            | 0x1f900..=0x1f9ff
    )
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
    debug!("Starting query");

    let mut title_exact: Vec<String> = args
        .title
        .iter()
        .map(|s| normalize_exact_title(s))
        .filter(|s| !s.is_empty())
        .collect();
    title_exact.dedup();

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
    let has_filter = !title_exact.is_empty()
        || !title_include.is_empty()
        || !title_exclude.is_empty()
        || !level_include.is_empty()
        || !level_exclude.is_empty()
        || !conference_include.is_empty()
        || !conference_exclude.is_empty()
        || !year_include.is_empty()
        || !year_exclude.is_empty();

    if !has_filter {
        return Err(
            "At least one filter is required: title/keyword/level/conference/year\nUse --help for details"
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
        &title_exact,
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

fn normalize_exact_title(value: &str) -> String {
    value
        .trim()
        .chars()
        .filter(|ch| !matches!(ch, '{' | '}'))
        .collect::<String>()
        .trim_end_matches(is_title_terminal_punctuation)
        .trim_end()
        .to_ascii_lowercase()
}

fn is_title_terminal_punctuation(ch: char) -> bool {
    matches!(ch, '.' | '!' | '?')
}
