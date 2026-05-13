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
                eprintln!("错误: {e}");
                std::process::exit(1);
            });
            let db_path = build_db::resolve_db_path().unwrap_or_else(|e| {
                eprintln!("错误: {e}");
                std::process::exit(1);
            });
            build_db::build_db(&papers_dir, &db_path)
        }
        cli::Command::Query(args) => {
            let db_path = build_db::resolve_db_path().unwrap_or_else(|e| {
                eprintln!("错误: {e}");
                std::process::exit(1);
            });
            query::run_query(args, &db_path)
        }
        cli::Command::Help => {
            print!("{}", cli::USAGE);
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("错误: {e}");
        std::process::exit(1);
    }
}
