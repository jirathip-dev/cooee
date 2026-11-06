//! The `cooee` binary: arguments and wiring only.

#![forbid(unsafe_code)]

use std::{
    env, fs,
    net::{Ipv4Addr, SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Command, ExitCode},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use cooee_core::{AGENTS_FIX, Config, Devices, ROOTS_FIX, Shed};
use qrcode::{QrCode, render::unicode::Dense1x2};

mod setup;

const USAGE: &str = "usage: cooee <serve | pair | devices | devices rm <id> | install --port <p> | doctor \
                     | setup [--port <p>] [--dry-run] [--yes]> [--instance <name>] [--config <file>] | cooee --version";

/// shed's install command: the rev `shed-proto` is pinned to in Cargo.toml, since the hello needs the exact version.
const SHED_CARGO: [&str; 8] = [
    "cargo",
    "install",
    "--locked",
    "--git",
    "https://github.com/jirathip-dev/shed",
    "--rev",
    "8bac04fdb8542ef496c614b47126d85c50a7892e",
    "shed",
];

fn shed_install() -> String {
    format!("{} && shed serve", SHED_CARGO.join(" "))
}

/// Removes `--flag <value>` from `args`: `Err` when the value is missing.
fn flag(args: &mut Vec<String>, name: &str) -> Result<Option<String>, ()> {
    let Some(i) = args.iter().position(|a| a == name) else {
        return Ok(None);
    };
    if i + 1 >= args.len() {
        return Err(());
    }
    let value = args.remove(i + 1);
    args.remove(i);
    Ok(Some(value))
}

fn main() -> ExitCode {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let mut switch = |name: &str| {
        let at = args.iter().position(|a| a == name);
        at.map(|i| args.remove(i)).is_some()
    };
    let (dry_run, yes) = (switch("--dry-run"), switch("--yes"));
    let (Ok(instance), Ok(config), Ok(port)) = (
        flag(&mut args, "--instance"),
        flag(&mut args, "--config"),
        flag(&mut args, "--port"),
    ) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let instance = instance.unwrap_or_else(|| "cooee".to_owned());
    let named = !instance.is_empty()
        && instance
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    if !named {
        eprintln!("cooee: an instance name is lowercase letters, digits and `-`");
        return ExitCode::from(2);
    }
    let explicit = config.is_some();
    let path = config
        .map(PathBuf::from)
        .or_else(|| Config::default_path(&instance));
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
            return with_config(path.as_deref(), &instance, |config| remove(config, &id));
        }
        ["doctor"] => doctor,
        ["install"] => {
            let Some(port) = port.and_then(|p| p.parse().ok()) else {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            };
            return finish(install(path, &instance, port, explicit));
        }
        ["setup"] => {
            let Ok(port) = port.map(|p| p.parse()).transpose() else {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            };
            let path = path.ok_or("no config path: set HOME");
            let setup = setup::Setup {
                instance,
                port,
                explicit,
                dry_run,
                yes,
            };
            return finish(path.map_err(str::to_owned).and_then(|p| setup.run(&p)));
        }
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    with_config(path.as_deref(), &instance, run)
}

/// Loads instance `instance`'s config (the defaults when there is no file).
fn load(path: Option<&Path>, instance: &str) -> Result<Config, String> {
    let config = path.map_or_else(|| Ok(Config::default()), Config::load);
    let config = config.map_err(|e| e.to_string())?;
    Ok(Config {
        instance: instance.to_owned(),
        ..config
    })
}

/// Loads the config, then runs `f`.
fn with_config(
    path: Option<&Path>,
    instance: &str,
    f: impl FnOnce(&Config) -> Result<(), String>,
) -> ExitCode {
    finish(load(path, instance).and_then(|c| f(&c)))
}

fn finish(result: Result<(), String>) -> ExitCode {
    match result {
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
    // Behind `tailscale serve --https=<port>` the phone uses https, the first allowed name and the port; else this
    // machine itself.
    let url = match config.allowed_hosts.first() {
        Some(host) => format!("https://{host}:{}/pair?code={code}", config.port),
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

/// This machine's tailnet name (`tailscale status`), if Tailscale is up.
fn tailnet_name() -> Option<String> {
    let out = Command::new("tailscale")
        .args(["status", "--json"])
        .output()
        .ok()?;
    let status: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let name = status["Self"]["DNSName"].as_str()?.trim_end_matches('.');
    (!name.is_empty()).then(|| name.to_owned())
}

fn label(instance: &str) -> String {
    format!("dev.cooee.{instance}")
}

/// XML text: `&`, `<` and `>` escaped.
fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Writes the instance's launchd plist (and its config with `port`, if it has none), then prints the commands that
/// start it, serve it on the tailnet and, if shed isn't running, install shed.
fn install(path: Option<PathBuf>, instance: &str, port: u16, explicit: bool) -> Result<(), String> {
    let path = path.ok_or("no config path: set HOME")?;
    if !path.exists() {
        let hosts = tailnet_name().map(|n| format!("allowed_hosts = [{n:?}]\n"));
        let toml = format!("port = {port}\n{}", hosts.unwrap_or_default());
        fs::create_dir_all(path.parent().unwrap_or(&path)).map_err(|e| e.to_string())?;
        fs::write(&path, toml).map_err(|e| e.to_string())?;
        println!("wrote {}", path.display());
    }
    let config = load(Some(&path), instance)?;
    if config.port != port {
        return Err(format!(
            "{} says port = {}: change it to {port} first",
            path.display(),
            config.port
        ));
    }
    let (file, plist) = plist(&config, &path, explicit)?;
    write_plist(&config, &file, &plist)?;
    println!("wrote {}\n\nrun these yourself:", file.display());
    println!("  launchctl bootstrap gui/$(id -u) {}", file.display());
    println!("  tailscale serve --bg --https={port} http://127.0.0.1:{port}");
    if Shed::new(config.shed.clone(), config.shed_dir.clone())
        .hello()
        .is_err()
    {
        println!("  {}   # shed isn't running", shed_install());
    }
    Ok(())
}

/// The instance's launchd plist, `~/Library/LaunchAgents/dev.cooee.<instance>.plist`, and its text: `cooee serve`
/// (with `--config <path>` if `explicit`), kept alive, logging to the data folder.
fn plist(config: &Config, path: &Path, explicit: bool) -> Result<(PathBuf, String), String> {
    let home = env::var_os("HOME").ok_or("no HOME")?;
    let data = config.data_dir().ok_or("no data folder")?;
    let exe = env::current_exe().map_err(|e| e.to_string())?;
    let mut argv = vec![
        exe.display().to_string(),
        "serve".into(),
        "--instance".into(),
        config.instance.clone(),
    ];
    if explicit {
        argv.extend(["--config".into(), path.display().to_string()]);
    }
    let argv: String = argv
        .iter()
        .map(|a| ["\n    <string>", &xml(a), "</string>"].concat())
        .collect();
    let log = xml(&data.join("cooee.log").display().to_string());
    let env_path = xml(&env::var("PATH").unwrap_or_default());
    let label = label(&config.instance);
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{label}</string>
  <key>ProgramArguments</key>
  <array>{argv}
  </array>
  <key>EnvironmentVariables</key>
  <dict>
    <key>PATH</key>
    <string>{env_path}</string>
  </dict>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>{log}</string>
  <key>StandardErrorPath</key>
  <string>{log}</string>
</dict>
</plist>
"#
    );
    let file = Path::new(&home).join(format!("Library/LaunchAgents/{label}.plist"));
    Ok((file, plist))
}

/// Writes the plist `file`, and the data folder its log goes to.
fn write_plist(config: &Config, file: &Path, plist: &str) -> Result<(), String> {
    let data = config.data_dir().ok_or("no data folder")?;
    fs::create_dir_all(&data).map_err(|e| e.to_string())?;
    fs::create_dir_all(file.parent().unwrap_or(file)).map_err(|e| e.to_string())?;
    fs::write(file, plist).map_err(|e| e.to_string())
}

/// Checks the instance: its service, port, shed, push keys and tailnet URL; fails if any check fails.
fn doctor(config: &Config) -> Result<(), String> {
    let mut failed = Vec::new();
    let mut check = |ok: bool, what: String| {
        println!("{} {what}", if ok { "ok  " } else { "FAIL" });
        if !ok {
            failed.push(format!("FAIL {what}"));
        }
    };
    let label = label(&config.instance);
    let loaded = Command::new("launchctl")
        .args(["list", &label])
        .output()
        .is_ok_and(|o| o.status.success());
    check(
        loaded,
        format!("service {label} loaded (else: cooee install)"),
    );
    let port = config.port;
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let up = TcpStream::connect_timeout(&addr, Duration::from_secs(2)).is_ok();
    check(up, format!("listening on port {port}"));
    let shed = Shed::new(config.shed.clone(), config.shed_dir.clone());
    match shed.hello() {
        Ok(()) => {
            check(true, format!("shed at {}", shed.socket.display()));
            if shed.agents().is_ok_and(|a| a.is_empty()) {
                println!("warn no agents installed in shed\n     fix: {AGENTS_FIX}");
            }
        }
        Err(e) => check(
            false,
            format!("shed: {e}\n     install: {}", shed_install()),
        ),
    }
    if config.roots.is_empty() {
        println!("warn no roots configured\n     fix: add {ROOTS_FIX} to config.toml");
    }
    let data = config.data_dir().ok_or("no data folder")?;
    match cooee_server::Push::new(&data).has_key() {
        Ok(true) => println!("ok   push keys stored"),
        Ok(false) => {
            println!("ok   push keys: none yet (made when a phone turns on notifications)");
        }
        Err(e) => check(false, format!("push keys: {e}")),
    }
    match tailnet_name() {
        None => check(false, "tailnet: no name (is Tailscale up?)".to_owned()),
        Some(name) => {
            let url = format!("https://{name}:{port}/");
            if !config.allowed_hosts.contains(&name) {
                check(false, format!("{url}: add {name:?} to allowed_hosts"));
            }
            let reached = Command::new("curl")
                .args(["-sf", "-o", "/dev/null", "--max-time", "5"])
                .arg(format!("{url}pair"))
                .status()
                .is_ok_and(|s| s.success());
            check(
                reached,
                format!(
                    "{url} (else: tailscale serve --bg --https={port} http://127.0.0.1:{port})"
                ),
            );
        }
    }
    if failed.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{} check(s) failed:\n{}",
            failed.len(),
            failed.join("\n")
        ))
    }
}
