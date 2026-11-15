//! The `cooee` binary: parses the line, looks the command up in `cmd::ALL`, maps its result to an exit code.

#![forbid(unsafe_code)]

use std::{env, process::ExitCode};

use cli::{Failed, Invocation};

mod accounts;
mod check;
mod cli;
mod cmd;
mod devices;
mod doctor;
mod keychain;
mod service;
mod setup;

/// The usage line, every command's part in name order.
fn usage_line() -> String {
    let parts: Vec<&str> = cmd::ALL.iter().map(|c| c.usage).collect();
    format!(
        "usage: cooee <{}> [--instance <name>] [--config <file>] | cooee --version",
        parts.join(" | ")
    )
}

/// Whether `instance` is a valid instance name: lowercase letters, digits and `-`.
fn named(instance: &str) -> bool {
    !instance.is_empty()
        && instance
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn main() -> ExitCode {
    let usage = usage_line();
    let flags = cmd::ALL.iter().flat_map(|c| c.flags);
    let inv = match Invocation::parse(env::args().skip(1).collect(), flags, usage.clone()) {
        Ok(inv) => inv,
        Err(failed) => return exit(Err(failed), &usage),
    };
    if !named(inv.instance()) {
        eprintln!("cooee: an instance name is lowercase letters, digits and `-`");
        return ExitCode::from(2);
    }
    let result = match inv.words().as_slice() {
        ["--version" | "-V"] => {
            println!("cooee {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        ["--help" | "-h"] => {
            println!("{usage}");
            return ExitCode::SUCCESS;
        }
        [word, ..] => cmd::ALL
            .iter()
            .find(|c| c.name == *word)
            .map_or(Err(Failed::Usage), |c| (c.run)(&inv)),
        [] => Err(Failed::Usage),
    };
    exit(result, &usage)
}

fn exit(result: Result<(), Failed>, usage: &str) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(Failed::Usage) => {
            eprintln!("{usage}");
            ExitCode::from(2)
        }
        Err(Failed::Because(e)) => {
            eprintln!("cooee: {e}");
            ExitCode::FAILURE
        }
        Err(Failed::Exit(code)) => code,
    }
}
