//! The `cooee` binary: arguments and wiring only.

#![forbid(unsafe_code)]

use std::{
    env,
    path::PathBuf,
    process::ExitCode,
    time::{SystemTime, UNIX_EPOCH},
};

use cooee_core::{Config, Devices};
use qrcode::{QrCode, render::unicode::Dense1x2};

const USAGE: &str =
    "usage: cooee <serve | pair | devices | devices rm <id>> [--config <file>] | cooee --version";

fn main() -> ExitCode {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let mut path = Config::default_path();
    if let Some(i) = args.iter().position(|a| a == "--config") {
        if i + 1 >= args.len() {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
        path = Some(PathBuf::from(args.remove(i + 1)));
        args.remove(i);
    }
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let run = match args.as_slice() {
        ["--version" | "-V"] => {
            println!("cooee {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        ["--help" | "-h"] => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        ["serve"] => serve,
        ["pair"] => pair,
        ["devices"] => devices,
        ["devices", "rm", id] => {
            let id = (*id).to_owned();
            return with_config(path, |config| remove(config, &id));
        }
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    with_config(path, run)
}

/// Loads the config (the defaults when there is no file), then runs `f`.
fn with_config(path: Option<PathBuf>, f: impl FnOnce(&Config) -> Result<(), String>) -> ExitCode {
    let config = path.map_or_else(|| Ok(Config::default()), |p| Config::load(&p));
    match config.map_err(|e| e.to_string()).and_then(|c| f(&c)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("cooee: {e}");
            ExitCode::FAILURE
        }
    }
}

fn serve(config: &Config) -> Result<(), String> {
    cooee_server::serve(config, |addr| {
        println!("cooee: listening on http://{addr}");
    })
    .map_err(|e| e.to_string())
}

fn devices_of(config: &Config) -> Result<Devices, String> {
    let data = config
        .data_dir()
        .ok_or("no data folder: set `data` or HOME")?;
    Ok(Devices::new(&data))
}

/// Prints a new pairing code, the link to enter it at and that link as a QR code.
fn pair(config: &Config) -> Result<(), String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
    let code = devices_of(config)?.pair(now).map_err(|e| e.to_string())?;
    // Behind `tailscale serve` the phone uses https and the first allowed name; else this machine itself.
    let url = match config.allowed_hosts.first() {
        Some(host) => format!("https://{host}/pair?code={code}"),
        None => format!("http://127.0.0.1:{}/pair?code={code}", config.port),
    };
    println!("code: {code} (for 10 minutes)\nopen {url}");
    if let Ok(qr) = QrCode::new(&url) {
        println!("{}", qr.render::<Dense1x2>().quiet_zone(true).build());
    }
    Ok(())
}

fn devices(config: &Config) -> Result<(), String> {
    for d in devices_of(config)?.list().map_err(|e| e.to_string())? {
        println!("{}  {}", d.id, d.label);
    }
    Ok(())
}

fn remove(config: &Config, id: &str) -> Result<(), String> {
    if devices_of(config)?.remove(id).map_err(|e| e.to_string())? {
        println!("removed {id}");
        Ok(())
    } else {
        Err(format!("no device {id}"))
    }
}
