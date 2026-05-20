pub const USAGE: &str = r#"Usage:
  search build-db                    Build SQLite database from PAPERS directory
  search query [OPTIONS] [<keywords>] Search papers
  search bib [OPTIONS] [<keywords>]   Output BibTeX entries
  search --help                       Show this help

Query options:
  -k, --keyword <keyword>            Title include keywords (repeatable, comma-separated)
  -x, --exclude <keyword>            Title exclude keywords (repeatable, comma-separated)
      --exclude-keyword <keyword>    Alias for --exclude
  -l, --level <level>                Level filter (repeatable, comma-separated)
      --exclude-level <level>        Level exclude (repeatable, comma-separated)
  -n, --conference <name>            Conference filter (repeatable, comma-separated)
      --exclude-conference <name>    Conference exclude (repeatable, comma-separated)
  -y, --year <year>                  Year filter (repeatable, comma-separated)
      --exclude-year <year>          Year exclude (repeatable, comma-separated)
  -s, --sort <field:order>           Sort spec (repeatable) fields: level, conference, year, title
                                      order: asc, desc
  -c, --columns <list>               Columns to show (comma-separated): level, conference, year, title, bib, author, url, etc.
  -X, --exclude-columns <list>       Columns to hide (comma-separated) — show all columns except these
      --db-path <path>               Database file path (overrides PAPERS_DB_PATH env var)

Environment variables:
  PAPERS_DIR                          Paper directory path
  PAPERS_DB_PATH                      SQLite database file path
  RUST_LOG=debug                      Enable debug logging

Examples:
  search build-db
  search query diffusion model
  search query --level A --conference AAAI --year 2024
  search query --level A,B --conference AAAI,ICML --year 2024,2025 diffusion
  search query --exclude-level B --exclude-year 2024
  search query --sort year:desc --columns conference,year,title diffusion
  search bib --keyword vla
"#;

#[derive(Debug)]
pub enum Command {
    BuildDb,
    Query(QueryArgs),
    Bib(QueryArgs),
    Help,
}

#[derive(Debug, Default)]
pub struct QueryArgs {
    pub keyword: Vec<String>,
    pub positional_keywords: Vec<String>,
    pub exclude: Vec<String>,
    pub level: Vec<String>,
    pub exclude_level: Vec<String>,
    pub conference: Vec<String>,
    pub exclude_conference: Vec<String>,
    pub year: Vec<String>,
    pub exclude_year: Vec<String>,
    pub sort: Vec<String>,
    pub columns: Option<Vec<String>>,
    pub exclude_columns: Option<Vec<String>>,
    pub db_path_override: Option<String>,
}

pub fn parse(args: &[String]) -> Command {
    if args.len() <= 1 {
        return Command::Help;
    }

    let subcommand = &args[1];

    match subcommand.as_str() {
        "build-db" => Command::BuildDb,

        "query" | "q" | "bib" | "b" => {
            let output_bib = matches!(subcommand.as_str(), "bib" | "b");
            let mut qargs = QueryArgs::default();
            let mut i = 2;

            while i < args.len() {
                let arg = &args[i];

                match arg.as_str() {
                    "-k" | "--keyword" => {
                        i += 1;
                        if let Some(val) = args.get(i) {
                            push_csv(&mut qargs.keyword, val);
                        }
                    }
                    "-x" | "--exclude" | "--exclude-keyword" => {
                        i += 1;
                        if let Some(val) = args.get(i) {
                            push_csv(&mut qargs.exclude, val);
                        }
                    }
                    "-l" | "--level" => {
                        i += 1;
                        if let Some(val) = args.get(i) {
                            push_csv(&mut qargs.level, val);
                        }
                    }
                    "--exclude-level" => {
                        i += 1;
                        if let Some(val) = args.get(i) {
                            push_csv(&mut qargs.exclude_level, val);
                        }
                    }
                    "-n" | "--conference" => {
                        i += 1;
                        if let Some(val) = args.get(i) {
                            push_csv(&mut qargs.conference, val);
                        }
                    }
                    "--exclude-conference" => {
                        i += 1;
                        if let Some(val) = args.get(i) {
                            push_csv(&mut qargs.exclude_conference, val);
                        }
                    }
                    "-y" | "--year" => {
                        i += 1;
                        if let Some(val) = args.get(i) {
                            push_csv(&mut qargs.year, val);
                        }
                    }
                    "--exclude-year" => {
                        i += 1;
                        if let Some(val) = args.get(i) {
                            push_csv(&mut qargs.exclude_year, val);
                        }
                    }
                    "-s" | "--sort" => {
                        i += 1;
                        if let Some(val) = args.get(i) {
                            qargs.sort.push(val.clone());
                        }
                    }
                    "-c" | "--columns" => {
                        i += 1;
                        if let Some(val) = args.get(i) {
                            qargs.columns = Some(
                                val.split(',')
                                    .map(|s| s.trim().to_string())
                                    .filter(|s| !s.is_empty())
                                    .collect(),
                            );
                        }
                    }
                    "-X" | "--exclude-columns" => {
                        i += 1;
                        if let Some(val) = args.get(i) {
                            qargs.exclude_columns = Some(
                                val.split(',')
                                    .map(|s| s.trim().to_string())
                                    .filter(|s| !s.is_empty())
                                    .collect(),
                            );
                        }
                    }
                    "--db-path" => {
                        i += 1;
                        if let Some(val) = args.get(i) {
                            qargs.db_path_override = Some(val.clone());
                        }
                    }
                    "--paper-dir" => {
                        i += 1;
                        // Legacy option, ignored — use PAPERS_DIR env var instead
                        eprintln!("Note: --paper-dir is deprecated, set the PAPERS_DIR environment variable instead");
                    }
                    "-h" | "--help" => return Command::Help,
                    _ if arg.starts_with("--keyword=") => {
                        let val = arg.trim_start_matches("--keyword=");
                        push_csv(&mut qargs.keyword, val);
                    }
                    _ if arg.starts_with("--exclude=") || arg.starts_with("--exclude-keyword=") => {
                        let val = arg.split_once('=').map(|(_, v)| v).unwrap_or("");
                        push_csv(&mut qargs.exclude, val);
                    }
                    _ if arg.starts_with("--level=") => {
                        let val = arg.trim_start_matches("--level=");
                        push_csv(&mut qargs.level, val);
                    }
                    _ if arg.starts_with("--exclude-level=") => {
                        let val = arg.trim_start_matches("--exclude-level=");
                        push_csv(&mut qargs.exclude_level, val);
                    }
                    _ if arg.starts_with("--conference=") => {
                        let val = arg.trim_start_matches("--conference=");
                        push_csv(&mut qargs.conference, val);
                    }
                    _ if arg.starts_with("--exclude-conference=") => {
                        let val = arg.trim_start_matches("--exclude-conference=");
                        push_csv(&mut qargs.exclude_conference, val);
                    }
                    _ if arg.starts_with("--year=") => {
                        let val = arg.trim_start_matches("--year=");
                        push_csv(&mut qargs.year, val);
                    }
                    _ if arg.starts_with("--exclude-year=") => {
                        let val = arg.trim_start_matches("--exclude-year=");
                        push_csv(&mut qargs.exclude_year, val);
                    }
                    _ if arg.starts_with("--sort=") => {
                        let val = arg.trim_start_matches("--sort=");
                        qargs.sort.push(val.to_string());
                    }
                    _ if arg.starts_with("--columns=") => {
                        let val = arg.trim_start_matches("--columns=");
                        qargs.columns = Some(
                            val.split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect(),
                        );
                    }
                    _ if arg.starts_with("--exclude-columns=") => {
                        let val = arg.trim_start_matches("--exclude-columns=");
                        qargs.exclude_columns = Some(
                            val.split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect(),
                        );
                    }
                    _ if arg.starts_with("--db-path=") => {
                        let val = arg.trim_start_matches("--db-path=");
                        qargs.db_path_override = Some(val.to_string());
                    }
                    _ if arg.starts_with('-') => {
                        eprintln!("Warning: unsupported option: {arg}");
                        i += 1;
                        continue;
                    }
                    _ => {
                        qargs.positional_keywords.push(arg.clone());
                    }
                }
                i += 1;
            }

            if output_bib {
                Command::Bib(qargs)
            } else {
                Command::Query(qargs)
            }
        }

        "-h" | "--help" | "help" => Command::Help,

        _ => {
            eprintln!("Unknown command: {subcommand}\n");
            Command::Help
        }
    }
}

/// Normalize and push comma-separated values, deduplicating.
fn push_csv(target: &mut Vec<String>, raw: &str) {
    for item in raw.split(',') {
        let trimmed = item.trim().to_ascii_lowercase();
        if !trimmed.is_empty() && !target.contains(&trimmed) {
            target.push(trimmed);
        }
    }
}
