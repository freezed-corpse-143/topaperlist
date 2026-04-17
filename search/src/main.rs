use std::cmp::Ordering;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;

const USAGE: &str = r#"Usage:
  search [OPTIONS] [<title-keyword> ...]

Search accepted paper titles under the Paper directory.

Positional arguments:
  <title-keyword>                    Optional title include keywords. Every keyword must match.
                                     Positional keywords are equivalent to --keyword.

Options:
  -k, --keyword <keyword>            Title include keyword. Repeatable, supports comma-separated values.
  -x, --exclude <keyword>            Title exclude keyword. Repeatable, supports comma-separated values.
      --exclude-keyword <keyword>    Alias of --exclude.
  -l, --level <level>                Conference level include filter. Repeatable, supports comma-separated values.
      --exclude-level <level>        Conference level exclude filter. Repeatable, supports comma-separated values.
  -n, --conference <name>            Conference name include filter. Repeatable, supports comma-separated values.
      --exclude-conference <name>    Conference name exclude filter. Repeatable, supports comma-separated values.
  -y, --year <year>                  Conference year include filter. Exact match, repeatable, supports comma-separated values.
      --exclude-year <year>          Conference year exclude filter. Exact match, repeatable, supports comma-separated values.
  -s, --sort <field>:<order>         Sort rule, repeatable. Fields: level, conference, year, title.
                                     Orders: asc, desc.
  -c, --columns <list>               Comma-separated columns to display.
                                     Available: level, conference, year, title.
                                     Output order is always: level, conference, year, title.
      --paper-dir <path>             Override Paper directory path.
  -h, --help                         Show this help message.

Examples:
  search diffusion model
  search --keyword diffusion --keyword model
  search --level A --conference AAAI --year 2024
  search --level A,B --conference AAAI,ICML --year 2024,2025 diffusion
  search --exclude-level B --exclude-year 2024
  search --conference NeurIPS --exclude survey --exclude-year 2023 --sort year:desc --sort title:asc
  search --level A --columns conference,year,title
"#;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Field {
    Level,
    Conference,
    Year,
    Title,
}

impl Field {
    fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "level" => Ok(Self::Level),
            "conference" | "conf" | "name" => Ok(Self::Conference),
            "year" => Ok(Self::Year),
            "title" | "paper" => Ok(Self::Title),
            other => Err(format!("unsupported field: {other}")),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Direction {
    Asc,
    Desc,
}

impl Direction {
    fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "asc" => Ok(Self::Asc),
            "desc" => Ok(Self::Desc),
            other => Err(format!("unsupported order: {other}")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SortSpec {
    field: Field,
    direction: Direction,
}

#[derive(Debug, Eq, PartialEq)]
struct Config {
    title_include_keywords: Vec<String>,
    title_exclude_keywords: Vec<String>,
    level_include_filters: Vec<String>,
    level_exclude_filters: Vec<String>,
    conference_include_filters: Vec<String>,
    conference_exclude_filters: Vec<String>,
    year_include_filters: Vec<String>,
    year_exclude_filters: Vec<String>,
    sort_specs: Vec<SortSpec>,
    display_fields: Vec<Field>,
    paper_dir: Option<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Record {
    level: String,
    conference: String,
    year: String,
    title: String,
}

fn main() {
    match run() {
        Ok(()) => {}
        Err(AppError::Help) => {
            print!("{USAGE}");
        }
        Err(AppError::Message(message)) => {
            eprintln!("Error: {message}\n");
            eprintln!("{USAGE}");
            process::exit(1);
        }
        Err(AppError::Io(error)) => {
            eprintln!("Error: {error}");
            process::exit(1);
        }
    }
}

fn run() -> Result<(), AppError> {
    let config = parse_args(env::args().skip(1))?;
    let paper_dir = resolve_paper_dir(config.paper_dir.as_deref())?;
    let mut records = load_records(&paper_dir)?;

    records.retain(|record| record_matches(record, &config));

    if !config.sort_specs.is_empty() {
        records.sort_by(|left, right| compare_records(left, right, &config.sort_specs));
    }

    for record in records {
        println!("{}", format_record(&record, &config.display_fields));
    }

    Ok(())
}

#[derive(Debug)]
enum AppError {
    Help,
    Message(String),
    Io(io::Error),
}

impl From<io::Error> for AppError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

fn parse_args<I>(args: I) -> Result<Config, AppError>
where
    I: IntoIterator<Item = String>,
{
    let mut title_include_keywords = Vec::new();
    let mut title_exclude_keywords = Vec::new();
    let mut level_include_filters = Vec::new();
    let mut level_exclude_filters = Vec::new();
    let mut conference_include_filters = Vec::new();
    let mut conference_exclude_filters = Vec::new();
    let mut year_include_filters = Vec::new();
    let mut year_exclude_filters = Vec::new();
    let mut sort_specs = Vec::new();
    let mut display_fields = canonical_fields();
    let mut paper_dir = None;

    let args: Vec<String> = args.into_iter().collect();
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "-h" | "--help" => return Err(AppError::Help),
            "-k" | "--keyword" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| AppError::Message("missing value for --keyword".to_string()))?;
                append_normalized_values(&mut title_include_keywords, value)?;
            }
            "-x" | "--exclude" | "--exclude-keyword" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| AppError::Message("missing value for --exclude".to_string()))?;
                append_normalized_values(&mut title_exclude_keywords, value)?;
            }
            "-l" | "--level" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| AppError::Message("missing value for --level".to_string()))?;
                append_normalized_values(&mut level_include_filters, value)?;
            }
            "--exclude-level" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    AppError::Message("missing value for --exclude-level".to_string())
                })?;
                append_normalized_values(&mut level_exclude_filters, value)?;
            }
            "-n" | "--conference" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    AppError::Message("missing value for --conference".to_string())
                })?;
                append_normalized_values(&mut conference_include_filters, value)?;
            }
            "--exclude-conference" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    AppError::Message("missing value for --exclude-conference".to_string())
                })?;
                append_normalized_values(&mut conference_exclude_filters, value)?;
            }
            "-y" | "--year" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| AppError::Message("missing value for --year".to_string()))?;
                append_normalized_values(&mut year_include_filters, value)?;
            }
            "--exclude-year" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    AppError::Message("missing value for --exclude-year".to_string())
                })?;
                append_normalized_values(&mut year_exclude_filters, value)?;
            }
            "-s" | "--sort" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| AppError::Message("missing value for --sort".to_string()))?;
                sort_specs.push(parse_sort_spec(value)?);
            }
            "-c" | "--columns" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| AppError::Message("missing value for --columns".to_string()))?;
                display_fields = parse_columns(value)?;
            }
            "--paper-dir" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    AppError::Message("missing value for --paper-dir".to_string())
                })?;
                paper_dir = Some(PathBuf::from(value));
            }
            _ if arg.starts_with("--keyword=") => {
                let value = arg.trim_start_matches("--keyword=");
                append_normalized_values(&mut title_include_keywords, value)?;
            }
            _ if arg.starts_with("--exclude=") || arg.starts_with("--exclude-keyword=") => {
                let value = arg
                    .split_once('=')
                    .map(|(_, value)| value)
                    .unwrap_or_default();
                append_normalized_values(&mut title_exclude_keywords, value)?;
            }
            _ if arg.starts_with("--level=") => {
                let value = arg.trim_start_matches("--level=");
                append_normalized_values(&mut level_include_filters, value)?;
            }
            _ if arg.starts_with("--exclude-level=") => {
                let value = arg.trim_start_matches("--exclude-level=");
                append_normalized_values(&mut level_exclude_filters, value)?;
            }
            _ if arg.starts_with("--conference=") => {
                let value = arg.trim_start_matches("--conference=");
                append_normalized_values(&mut conference_include_filters, value)?;
            }
            _ if arg.starts_with("--exclude-conference=") => {
                let value = arg.trim_start_matches("--exclude-conference=");
                append_normalized_values(&mut conference_exclude_filters, value)?;
            }
            _ if arg.starts_with("--year=") => {
                let value = arg.trim_start_matches("--year=");
                append_normalized_values(&mut year_include_filters, value)?;
            }
            _ if arg.starts_with("--exclude-year=") => {
                let value = arg.trim_start_matches("--exclude-year=");
                append_normalized_values(&mut year_exclude_filters, value)?;
            }
            _ if arg.starts_with("--sort=") => {
                let value = arg.trim_start_matches("--sort=");
                sort_specs.push(parse_sort_spec(value)?);
            }
            _ if arg.starts_with("--columns=") => {
                let value = arg.trim_start_matches("--columns=");
                display_fields = parse_columns(value)?;
            }
            _ if arg.starts_with("--paper-dir=") => {
                let value = arg.trim_start_matches("--paper-dir=");
                paper_dir = Some(PathBuf::from(value));
            }
            _ if arg.starts_with('-') => {
                return Err(AppError::Message(format!("unsupported option: {arg}")));
            }
            _ => title_include_keywords.push(normalize_value(arg)?),
        }

        index += 1;
    }

    if !has_any_filter(&[
        &title_include_keywords,
        &title_exclude_keywords,
        &level_include_filters,
        &level_exclude_filters,
        &conference_include_filters,
        &conference_exclude_filters,
        &year_include_filters,
        &year_exclude_filters,
    ]) {
        return Err(AppError::Message(
            "at least one filter is required: keyword/level/conference/year include or exclude"
                .to_string(),
        ));
    }

    Ok(Config {
        title_include_keywords,
        title_exclude_keywords,
        level_include_filters,
        level_exclude_filters,
        conference_include_filters,
        conference_exclude_filters,
        year_include_filters,
        year_exclude_filters,
        sort_specs,
        display_fields,
        paper_dir,
    })
}

fn has_any_filter(filter_groups: &[&[String]]) -> bool {
    filter_groups.iter().any(|group| !group.is_empty())
}

fn append_normalized_values(target: &mut Vec<String>, raw: &str) -> Result<(), AppError> {
    let mut appended = false;
    for item in raw.split(',') {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        let normalized = normalize_value(trimmed)?;
        if !target.contains(&normalized) {
            target.push(normalized);
        }
        appended = true;
    }

    if appended {
        Ok(())
    } else {
        Err(AppError::Message(
            "filter value cannot be empty".to_string(),
        ))
    }
}

fn normalize_value(value: &str) -> Result<String, AppError> {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        Err(AppError::Message(
            "filter value cannot be empty".to_string(),
        ))
    } else {
        Ok(normalized)
    }
}

fn parse_sort_spec(value: &str) -> Result<SortSpec, AppError> {
    let (field_raw, order_raw) = value
        .split_once(':')
        .ok_or_else(|| AppError::Message(format!("invalid sort spec: {value}")))?;

    let field = Field::parse(field_raw).map_err(AppError::Message)?;
    let direction = Direction::parse(order_raw).map_err(AppError::Message)?;

    Ok(SortSpec { field, direction })
}

fn parse_columns(value: &str) -> Result<Vec<Field>, AppError> {
    let mut selected = Vec::new();
    for item in value.split(',') {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        let field = Field::parse(trimmed).map_err(AppError::Message)?;
        if !selected.contains(&field) {
            selected.push(field);
        }
    }

    if selected.is_empty() {
        return Err(AppError::Message(
            "at least one column must be selected".to_string(),
        ));
    }

    Ok(canonical_fields()
        .into_iter()
        .filter(|field| selected.contains(field))
        .collect())
}

fn canonical_fields() -> Vec<Field> {
    vec![Field::Level, Field::Conference, Field::Year, Field::Title]
}

fn resolve_paper_dir(override_path: Option<&Path>) -> Result<PathBuf, AppError> {
    if let Some(path) = override_path {
        if path.is_dir() {
            return Ok(path.to_path_buf());
        }
        return Err(AppError::Message(format!(
            "paper directory does not exist: {}",
            path.display()
        )));
    }

    let exe_dir = env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf));
    let current_dir = env::current_dir().ok();

    let mut candidates = Vec::new();

    if let Some(dir) = &exe_dir {
        candidates.push(dir.join("Paper"));
    }
    if let Some(dir) = &current_dir {
        candidates.push(dir.join("Paper"));
    }
    if let Some(dir) = &exe_dir {
        candidates.push(dir.join("..").join("Paper"));
    }
    if let Some(dir) = &current_dir {
        candidates.push(dir.join("..").join("Paper"));
    }

    for candidate in candidates {
        if candidate.is_dir() {
            return Ok(candidate);
        }
    }

    Err(AppError::Message(
        "unable to locate Paper directory; use --paper-dir to specify it".to_string(),
    ))
}

fn load_records(root: &Path) -> Result<Vec<Record>, AppError> {
    let mut records = Vec::new();

    for level_dir in sorted_dirs(root)? {
        let level = file_name(&level_dir)?;
        for conference_dir in sorted_dirs(&level_dir)? {
            let conference = file_name(&conference_dir)?;
            for file_path in sorted_txt_files(&conference_dir)? {
                let year = file_path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| {
                        AppError::Message(format!("invalid file name: {}", file_path.display()))
                    })?
                    .to_string();

                let content = fs::read_to_string(&file_path)?;
                for line in content.lines() {
                    let title = line.trim();
                    if title.is_empty() {
                        continue;
                    }
                    records.push(Record {
                        level: level.clone(),
                        conference: conference.clone(),
                        year: year.clone(),
                        title: title.to_string(),
                    });
                }
            }
        }
    }

    Ok(records)
}

fn sorted_dirs(path: &Path) -> Result<Vec<PathBuf>, AppError> {
    let mut items = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            items.push(entry.path());
        }
    }
    items.sort();
    Ok(items)
}

fn sorted_txt_files(path: &Path) -> Result<Vec<PathBuf>, AppError> {
    let mut items = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if !file_type.is_file() {
            continue;
        }
        let item_path = entry.path();
        if item_path.extension().and_then(|value| value.to_str()) == Some("txt") {
            items.push(item_path);
        }
    }
    items.sort();
    Ok(items)
}

fn file_name(path: &Path) -> Result<String, AppError> {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(str::to_string)
        .ok_or_else(|| AppError::Message(format!("invalid directory name: {}", path.display())))
}

fn record_matches(record: &Record, config: &Config) -> bool {
    matches_scalar_filter(
        &record.level,
        &config.level_include_filters,
        &config.level_exclude_filters,
    ) && matches_scalar_filter(
        &record.conference,
        &config.conference_include_filters,
        &config.conference_exclude_filters,
    ) && matches_scalar_filter(
        &record.year,
        &config.year_include_filters,
        &config.year_exclude_filters,
    ) && matches_title_keywords(
        &record.title,
        &config.title_include_keywords,
        &config.title_exclude_keywords,
    )
}

fn matches_scalar_filter(
    value: &str,
    include_filters: &[String],
    exclude_filters: &[String],
) -> bool {
    let normalized = value.to_ascii_lowercase();
    (include_filters.is_empty() || include_filters.iter().any(|filter| filter == &normalized))
        && !exclude_filters.iter().any(|filter| filter == &normalized)
}

fn matches_title_keywords(
    title: &str,
    include_keywords: &[String],
    exclude_keywords: &[String],
) -> bool {
    let words: Vec<String> = title
        .split_whitespace()
        .map(|word| word.to_ascii_lowercase())
        .collect();

    include_keywords
        .iter()
        .all(|keyword| words.iter().any(|word| word.contains(keyword)))
        && !exclude_keywords
            .iter()
            .any(|keyword| words.iter().any(|word| word.contains(keyword)))
}

fn compare_records(left: &Record, right: &Record, sort_specs: &[SortSpec]) -> Ordering {
    for spec in sort_specs {
        let ordering = match spec.field {
            Field::Level => compare_case_insensitive(&left.level, &right.level),
            Field::Conference => compare_case_insensitive(&left.conference, &right.conference),
            Field::Year => compare_year(&left.year, &right.year),
            Field::Title => compare_case_insensitive(&left.title, &right.title),
        };

        let ordering = match spec.direction {
            Direction::Asc => ordering,
            Direction::Desc => ordering.reverse(),
        };

        if ordering != Ordering::Equal {
            return ordering;
        }
    }

    Ordering::Equal
}

fn compare_case_insensitive(left: &str, right: &str) -> Ordering {
    left.to_ascii_lowercase().cmp(&right.to_ascii_lowercase())
}

fn compare_year(left: &str, right: &str) -> Ordering {
    match (left.parse::<u32>(), right.parse::<u32>()) {
        (Ok(l), Ok(r)) => l.cmp(&r),
        _ => left.cmp(right),
    }
}

fn format_record(record: &Record, fields: &[Field]) -> String {
    let mut parts = Vec::with_capacity(fields.len());
    for field in fields {
        let value = match field {
            Field::Level => &record.level,
            Field::Conference => &record.conference,
            Field::Year => &record.year,
            Field::Title => &record.title,
        };
        parts.push(value.as_str());
    }
    parts.join("\t")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parse_columns_keeps_canonical_order() {
        let fields = parse_columns("title,year,conference").unwrap();
        assert_eq!(fields, vec![Field::Conference, Field::Year, Field::Title]);
    }

    #[test]
    fn parse_args_accepts_combined_structured_filters_without_keywords() {
        let config = parse_args([
            "--level".to_string(),
            "A,B".to_string(),
            "--conference".to_string(),
            "AAAI".to_string(),
            "--year".to_string(),
            "2024,2025".to_string(),
        ])
        .unwrap();

        assert_eq!(
            config.level_include_filters,
            vec!["a".to_string(), "b".to_string()]
        );
        assert_eq!(config.conference_include_filters, vec!["aaai".to_string()]);
        assert_eq!(
            config.year_include_filters,
            vec!["2024".to_string(), "2025".to_string()]
        );
        assert!(config.title_include_keywords.is_empty());
    }

    #[test]
    fn parse_args_accepts_exclude_filters_only() {
        let config = parse_args([
            "--exclude-level".to_string(),
            "B".to_string(),
            "--exclude-conference".to_string(),
            "COLING,EACL".to_string(),
            "--exclude-year".to_string(),
            "2020,2021".to_string(),
            "--exclude-keyword".to_string(),
            "survey,tutorial".to_string(),
        ])
        .unwrap();

        assert_eq!(config.level_exclude_filters, vec!["b".to_string()]);
        assert_eq!(
            config.conference_exclude_filters,
            vec!["coling".to_string(), "eacl".to_string()]
        );
        assert_eq!(
            config.year_exclude_filters,
            vec!["2020".to_string(), "2021".to_string()]
        );
        assert_eq!(
            config.title_exclude_keywords,
            vec!["survey".to_string(), "tutorial".to_string()]
        );
    }

    #[test]
    fn record_matches_supports_combined_filters() {
        let record = Record {
            level: "A".to_string(),
            conference: "ICML".to_string(),
            year: "2024".to_string(),
            title: "Graph-aware Diffusion Models for Retrieval".to_string(),
        };
        let config = Config {
            title_include_keywords: vec!["graph".to_string(), "diff".to_string()],
            title_exclude_keywords: vec!["survey".to_string()],
            level_include_filters: vec!["a".to_string()],
            level_exclude_filters: vec![],
            conference_include_filters: vec!["icml".to_string(), "neurips".to_string()],
            conference_exclude_filters: vec![],
            year_include_filters: vec!["2024".to_string()],
            year_exclude_filters: vec![],
            sort_specs: Vec::new(),
            display_fields: canonical_fields(),
            paper_dir: None,
        };

        assert!(record_matches(&record, &config));

        let mut wrong_year = config;
        wrong_year.year_include_filters = vec!["2023".to_string()];
        assert!(!record_matches(&record, &wrong_year));
    }

    #[test]
    fn record_matches_respects_scalar_exclude_filters() {
        let record = Record {
            level: "A".to_string(),
            conference: "ICML".to_string(),
            year: "2024".to_string(),
            title: "Graph-aware Diffusion Models for Retrieval".to_string(),
        };
        let config = Config {
            title_include_keywords: vec![],
            title_exclude_keywords: vec![],
            level_include_filters: vec![],
            level_exclude_filters: vec!["b".to_string()],
            conference_include_filters: vec![],
            conference_exclude_filters: vec!["neurips".to_string()],
            year_include_filters: vec![],
            year_exclude_filters: vec!["2023".to_string()],
            sort_specs: Vec::new(),
            display_fields: canonical_fields(),
            paper_dir: None,
        };

        assert!(record_matches(&record, &config));

        let mut excluded_conf = config;
        excluded_conf.conference_exclude_filters = vec!["icml".to_string()];
        assert!(!record_matches(&record, &excluded_conf));
    }

    #[test]
    fn title_keyword_matching_uses_space_split_and_substring_logic() {
        assert!(matches_title_keywords(
            "Graph-aware Diffusion Models for Retrieval",
            &["graph".to_string(), "diff".to_string()],
            &[]
        ));
        assert!(!matches_title_keywords(
            "Graph-aware Diffusion Models for Retrieval",
            &["graph".to_string(), "survey".to_string()],
            &[]
        ));
        assert!(!matches_title_keywords(
            "Graph-aware Diffusion Models for Retrieval",
            &["graph".to_string()],
            &["retriev".to_string()]
        ));
    }

    #[test]
    fn compare_records_respects_priority_order() {
        let mut records = [
            Record {
                level: "A".to_string(),
                conference: "ICML".to_string(),
                year: "2023".to_string(),
                title: "B".to_string(),
            },
            Record {
                level: "A".to_string(),
                conference: "ICML".to_string(),
                year: "2024".to_string(),
                title: "A".to_string(),
            },
            Record {
                level: "B".to_string(),
                conference: "ACL".to_string(),
                year: "2024".to_string(),
                title: "C".to_string(),
            },
        ];

        let specs = vec![
            SortSpec {
                field: Field::Year,
                direction: Direction::Desc,
            },
            SortSpec {
                field: Field::Title,
                direction: Direction::Asc,
            },
        ];

        records.sort_by(|left, right| compare_records(left, right, &specs));

        assert_eq!(records[0].year, "2024");
        assert_eq!(records[0].title, "A");
        assert_eq!(records[1].year, "2024");
        assert_eq!(records[1].title, "C");
        assert_eq!(records[2].year, "2023");
    }

    #[test]
    fn load_records_reads_level_conference_year_from_path() {
        let root = temp_test_dir();
        let paper_dir = root.join("Paper");
        fs::create_dir_all(paper_dir.join("A").join("ICML")).unwrap();
        fs::create_dir_all(paper_dir.join("B").join("EMNLP")).unwrap();
        fs::write(
            paper_dir.join("A").join("ICML").join("2024.txt"),
            "First Paper\nSecond Paper\n",
        )
        .unwrap();
        fs::write(
            paper_dir.join("B").join("EMNLP").join("2025.txt"),
            "Third Paper\n",
        )
        .unwrap();

        let records = load_records(&paper_dir).unwrap();

        assert_eq!(records.len(), 3);
        assert_eq!(records[0].level, "A");
        assert_eq!(records[0].conference, "ICML");
        assert_eq!(records[0].year, "2024");
        assert_eq!(records[0].title, "First Paper");
        assert_eq!(records[2].level, "B");
        assert_eq!(records[2].conference, "EMNLP");
        assert_eq!(records[2].year, "2025");
        assert_eq!(records[2].title, "Third Paper");

        fs::remove_dir_all(root).unwrap();
    }

    fn temp_test_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = env::temp_dir().join(format!("topaperlist-search-test-{nanos}"));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
