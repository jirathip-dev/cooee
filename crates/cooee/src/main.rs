//! The `cooee` binary: arguments and wiring only.

#![forbid(unsafe_code)]

use std::{env, path::PathBuf, process::ExitCode};

use cooee_core::Config;

const USAGE: &str = "usage: cooee serve [--config <file>] | cooee --version";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["--version" | "-V"] => {
            println!("cooee {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        ["--help" | "-h"] => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        ["serve"] => serve(Config::default_path()),
        ["serve", "--config", path] => serve(Some(PathBuf::from(path))),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn serve(path: Option<PathBuf>) -> ExitCode {
    let config = match path.map_or_else(|| Ok(Config::default()), |p| Config::load(&p)) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("cooee: {e}");
            return ExitCode::FAILURE;
        }
    };
    match cooee_server::serve(&config, |addr| {
        println!("cooee: listening on http://{addr}");
    }) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("cooee: {e}");
            ExitCode::FAILURE
        }
    }
}
