mod build_db;
mod cli;
mod db;
mod models;
mod query;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = cli::parse(&args);

    let result = match cmd {
        cli::Command::BuildDb => {
            let papers_dir = build_db::resolve_papers_dir().unwrap_or_else(|e| {
                eprintln!("Error: {e}");
                std::process::exit(1);
            });
            let db_path = build_db::resolve_db_path().unwrap_or_else(|e| {
                eprintln!("Error: {e}");
                std::process::exit(1);
            });
            build_db::build_db(&papers_dir, &db_path)
        }
        cli::Command::Query(args) => {
            let db_path = build_db::resolve_db_path().unwrap_or_else(|e| {
                eprintln!("Error: {e}");
                std::process::exit(1);
            });
            query::run_query(args, &db_path)
        }
        cli::Command::Bib(args) => {
            let db_path = build_db::resolve_db_path().unwrap_or_else(|e| {
                eprintln!("Error: {e}");
                std::process::exit(1);
            });
            query::run_bib_query(args, &db_path)
        }
        cli::Command::UpdateInfo => {
            println!(
                "Data updates are handled by the installed search wrapper. Run `search update`, or run scripts/check-update manually from the source tree."
            );
            Ok(())
        }
        cli::Command::Version => {
            let db_path = build_db::resolve_db_path().unwrap_or_else(|e| {
                eprintln!("Error: {e}");
                std::process::exit(1);
            });
            print_version(&db_path)
        }
        cli::Command::Help => {
            print!("{}", cli::USAGE);
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn print_version(db_path: &std::path::Path) -> Result<(), String> {
    println!("topaperlist {}", env!("CARGO_PKG_VERSION"));
    println!("Database path: {}", db_path.display());

    if !db_path.exists() {
        println!("Database version: unavailable (database file not found)");
        return Ok(());
    }

    let conn = db::open_db(db_path)?;
    let version =
        db::get_metadata(&conn, "database_version")?.unwrap_or_else(|| "unknown".to_string());
    let source =
        db::get_metadata(&conn, "database_source")?.unwrap_or_else(|| "unknown".to_string());
    let built_at =
        db::get_metadata(&conn, "built_at_unix")?.unwrap_or_else(|| "unknown".to_string());
    let records = db::get_metadata(&conn, "record_count")?.unwrap_or_else(|| "unknown".to_string());

    println!("Database version: {version}");
    println!("Database source: {source}");
    println!("Built at unix: {built_at}");
    println!("Record count: {records}");
    Ok(())
}
