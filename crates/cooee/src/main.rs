//! The `cooee` binary: arguments and wiring only.

#![forbid(unsafe_code)]

use std::{
    env, fs,
    io::{IsTerminal, Write},
    net::{Ipv4Addr, SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Command, ExitCode},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use cooee_core::{Config, Devices, ROOTS_FIX, Shed, agents_fix};
use qrcode::{QrCode, render::unicode::Dense1x2};

mod accounts;
mod check;
mod keychain;
mod setup;

const USAGE: &str = "usage: cooee <serve | pair | pair asks | pair approve|decline <id> | devices | reports | devices rm <id> | install --port <p> | doctor | push migrate | secrets \
                     | perf [--minutes <n>] \
                     | setup [--port <p>] [--dry-run] [--yes] | plugin add <name | git-url> [--rev <r>] [--yes] \
                     | plugin update [<name>] [--yes] | plugin remove <name> [--purge] | plugin list | plugin check <answer.json>... \
                     | theme add <name | git-url | path> [--rev <r>] [--yes] | theme remove <name> | theme list \
                     | theme check <path>... | accounts ls [--json] \
                     | accounts env <provider> <name> [--require-signed-in] \
                     | accounts add <provider> <name> --folder <path> | accounts rm <provider> <name> | mcp ls \
                     | mcp add <name> (<command> [<arg>...] | --url <url> [--type sse]) \
                     [--env K=secret:N|env:N]... [--header K=secret:N|env:N]... | mcp rm|enable|disable <name> \
                     | mcp proxy <server> --thread <id> | mcp serve --thread <id>> [--instance <name>] [--config <file>] \
                     | cooee --version";

/// shed's install command: the rev `shed-proto` is pinned to in Cargo.toml, since the hello needs the exact version.
const SHED_CARGO: [&str; 8] = [
    "cargo",
    "install",
    "--locked",
    "--git",
    "https://github.com/jirathip-dev/shed",
    "--rev",
    "f026a4d35a2705e560a52515da8e3a5d32e4d659",
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

/// The usage line on stderr; exit 2.
fn usage() -> ExitCode {
    eprintln!("{USAGE}");
    ExitCode::from(2)
}

/// Whether `instance` is a valid instance name: lowercase letters, digits and `-`.
fn named(instance: &str) -> bool {
    !instance.is_empty()
        && instance
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn main() -> ExitCode {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let mut switch = |name: &str| {
        let at = args.iter().position(|a| a == name);
        at.map(|i| args.remove(i)).is_some()
    };
    let (dry_run, yes, purge) = (switch("--dry-run"), switch("--yes"), switch("--purge"));
    let (json, require) = (switch("--json"), switch("--require-signed-in"));
    let (Ok(instance), Ok(config), Ok(port), Ok(rev), Ok(minutes)) = (
        flag(&mut args, "--instance"),
        flag(&mut args, "--config"),
        flag(&mut args, "--port"),
        flag(&mut args, "--rev"),
        flag(&mut args, "--minutes"),
    ) else {
        return usage();
    };
    let instance = instance.unwrap_or_else(|| "cooee".to_owned());
    if !named(&instance) {
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
        ["pair", rest @ ..] => {
            return with_config(path.as_deref(), &instance, |c| pairing(c, rest));
        }
        ["devices"] => devices,
        ["reports"] => reports,
        ["perf"] => {
            let Ok(minutes) = minutes.map_or(Ok(60), |m| m.parse::<u64>()) else {
                return usage();
            };
            return with_config(path.as_deref(), &instance, |config| perf(config, minutes));
        }
        ["devices", "rm", id] => {
            return with_config(path.as_deref(), &instance, |config| remove(config, id));
        }
        ["doctor"] => doctor,
        ["push", "migrate"] => keychain::command,
        ["secrets"] => keychain::check,
        ["accounts", rest @ ..] => {
            let run = |c: Config| accounts::run(&c, rest, json, require);
            return load(path.as_deref(), &instance).map_or_else(|e| finish(Err(e)), run);
        }
        ["plugin", "check", files @ ..] => return finish(check::run(files)),
        ["plugin", rest @ ..] => {
            let rest: Vec<String> = rest.iter().map(|a| (*a).to_owned()).collect();
            let rest: Vec<&str> = rest.iter().map(String::as_str).collect();
            return with_config(path.as_deref(), &instance, |config| {
                plugin(config, &rest, rev.as_deref(), yes, purge)
            });
        }
        ["mcp", agent @ ("proxy" | "serve"), rest @ ..] => return finish(mcp_agent(agent, rest)),
        ["mcp", rest @ ..] => {
            return with_config(path.as_deref(), &instance, |config| mcp(config, rest));
        }
        ["theme", rest @ ..] => {
            return with_config(path.as_deref(), &instance, |config| {
                theme(config, rest, rev.as_deref(), yes)
            });
        }
        ["install"] => {
            let Some(port) = port.and_then(|p| p.parse().ok()) else {
                return usage();
            };
            return finish(install(path, &instance, port, explicit));
        }
        ["setup"] => {
            let Ok(port) = port.map(|p| p.parse()).transpose() else {
                return usage();
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
            return usage();
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

/// Serves the instance; `[[plugins]]` entries named `machines` and `terminal` are the first-party plugins, run here.
fn serve(config: &Config) -> Result<(), String> {
    let machines = config.plugins.iter().find(|p| p.name == "machines");
    let terminal = config
        .plugins
        .iter()
        .find(|p| p.name == cooee_terminal::NAME);
    // In the background, so a keychain that never answers holds up nothing.
    if let Some(data) = config.data_dir() {
        std::thread::spawn(move || {
            match keychain::migrate(&data) {
                Ok(Some(done)) => eprintln!("cooee: {done}"),
                Ok(None) => {}
                Err(e) => eprintln!("cooee: push key: {e}"),
            }
            match cooee_core::secrets::migrate(&data) {
                Ok(moved) if moved.is_empty() => {}
                Ok(moved) => eprintln!("cooee: moved into the keychain: {}", moved.join(", ")),
                Err(e) => eprintln!("cooee: secrets: {e}"),
            }
        });
    }
    cooee_server::serve(config, |addr| {
        println!("cooee: listening on http://{addr}");
        if let (Some(p), Some(data)) = (terminal, config.data_dir()) {
            let shed = Shed::new(config.shed.clone(), config.shed_dir.clone());
            if let Err(e) = cooee_terminal::start(p, shed, &data) {
                eprintln!("cooee: plugin terminal at {}: {e}", p.url);
            }
        }
        let Some(p) = machines else { return };
        let ip = if addr.ip().is_unspecified() {
            Ipv4Addr::LOCALHOST.into()
        } else {
            addr.ip()
        };
        let (shed, dir) = (config.shed.clone(), config.shed_dir.clone());
        if let Err(e) = cooee_machines::start(p, shed, dir, SocketAddr::new(ip, addr.port())) {
            eprintln!("cooee: plugin machines at {}: {e}", p.url);
        }
    })
    .map_err(|e| e.to_string())
}

fn devices_of(config: &Config) -> Result<Devices, String> {
    let data = config
        .data_dir()
        .ok_or("no data folder: set `data` or HOME")?;
    Ok(Devices::new(&data))
}

/// Prints a new pairing code, the link to enter it at and that link as a QR code; in a terminal, then waits for it
/// to be used (Ctrl-C leaves the code open until it runs out).
fn pair(config: &Config) -> Result<(), String> {
    let devices = devices_of(config)?;
    let had = devices.list().map_err(|e| e.to_string())?.len();
    let code = devices.pair(now()).map_err(|e| e.to_string())?;
    // Behind `tailscale serve --https=<port>` the phone uses https, the first allowed name and the port; else this
    // machine itself.
    let url = match config.allowed_hosts.first() {
        Some(host) => format!("https://{host}:{}/pair?code={code}", config.port),
        None => format!("http://127.0.0.1:{}/pair?code={code}", config.port),
    };
    println!("Scan it with the new device's Camera, or open the link and type the code.");
    if let Ok(qr) = QrCode::new(&url) {
        println!("{}", qr.render::<Dense1x2>().quiet_zone(true).build());
    }
    println!(
        "code  {} {}  (one use, 10 minutes)\nlink  {url}",
        &code[..3],
        &code[3..]
    );
    if !std::io::stdout().is_terminal() {
        return Ok(());
    }
    print!("waiting…  ");
    let _ = std::io::stdout().flush();
    let until = now() + 10 * 60 * 1000;
    while now() < until && devices.code_open(now()).unwrap_or(false) {
        std::thread::sleep(Duration::from_secs(1));
    }
    match devices.list().map_err(|e| e.to_string())?.get(had) {
        Some(d) => println!("✓ paired: {}", d.label),
        None => println!("the code ran out: run cooee pair again"),
    }
    Ok(())
}

/// `cooee pair`, `cooee pair asks` and `cooee pair approve|decline <id>`.
fn pairing(config: &Config, args: &[&str]) -> Result<(), String> {
    match args {
        [] => pair(config),
        ["asks"] => asks(config),
        [verb @ ("approve" | "decline"), id] => answer(config, id, *verb == "approve"),
        _ => Err(USAGE.to_owned()),
    }
}

/// Unix milliseconds.
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// The requests to pair still waiting, as `GET /pair/asks` lists them (JSON): the Mac app's Approve prompt reads it.
fn asks(config: &Config) -> Result<(), String> {
    let asking = devices_of(config)?
        .asking(now())
        .map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::to_string(&asking).map_err(|e| e.to_string())?
    );
    Ok(())
}

/// Approves (`yes`) or declines the waiting request `id` as "this Mac": the owner at its keyboard, so no passkey,
/// as `cooee pair` makes a code without one.
fn answer(config: &Config, id: &str, yes: bool) -> Result<(), String> {
    let devices = devices_of(config)?;
    if devices
        .answer(id, yes, "this Mac", now())
        .map_err(|e| e.to_string())?
    {
        Ok(())
    } else {
        Err(format!("no request {id} is waiting"))
    }
}

fn devices(config: &Config) -> Result<(), String> {
    for d in devices_of(config)?.list().map_err(|e| e.to_string())? {
        println!("{}  {}", d.id, d.label);
    }
    Ok(())
}

/// Lists the problems reported from the page (report mode), oldest first: when, the device, the page, the element
/// and the note.
fn reports(config: &Config) -> Result<(), String> {
    let data = config
        .data_dir()
        .ok_or("no data folder: set `data` or HOME")?;
    let text = match std::fs::read_to_string(data.join("reports.jsonl")) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e.to_string()),
    };
    for r in text
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
    {
        let s = |k: &str| r[k].as_str().unwrap_or_default().to_owned();
        let at = r["at"].as_u64().unwrap_or(0) / 1000;
        println!(
            "{at}  {}  {}  {}\n    {}",
            s("device"),
            s("path"),
            s("target"),
            s("note")
        );
    }
    if text.is_empty() {
        println!("no reports");
    }
    Ok(())
}

/// Prints p50/p95/max per route, shed method and first token over the last `minutes` of `perf.log` (#53).
fn perf(config: &Config, minutes: u64) -> Result<(), String> {
    let data = config
        .data_dir()
        .ok_or("no data folder: set `data` or HOME")?;
    let text = match fs::read_to_string(data.join(cooee_core::perf::FILE)) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e.to_string()),
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
    let since = now.saturating_sub(minutes.saturating_mul(60_000));
    print!("{}", cooee_core::perf::table(&text, since));
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

/// Shows a confirm's `text`; goes on with `--yes` or a `y` typed.
fn confirm(text: &str, yes: bool) -> Result<(), String> {
    println!("{text}");
    if yes {
        return Ok(());
    }
    print!("go ahead? [y/N] ");
    let _ = std::io::Write::flush(&mut std::io::stdout());
    let mut answer = String::new();
    let _ = std::io::stdin().read_line(&mut answer);
    match answer.trim() {
        "y" | "yes" => Ok(()),
        _ => Err("nothing changed".to_owned()),
    }
}

/// `cooee plugin add|update|remove|list` (docs/plugins.md).
fn plugin(
    config: &Config,
    args: &[&str],
    rev: Option<&str>,
    yes: bool,
    purge: bool,
) -> Result<(), String> {
    let m = cooee_server::Manager::new(config).map_err(|e| e.to_string())?;
    let e = |e: std::io::Error| e.to_string();
    match args {
        ["add", source] => {
            let plan = m.plan(source, rev).map_err(e)?;
            confirm(&plan.text, yes)?;
            m.add(&plan).map_err(e)?;
            println!("added {}: cooee serve runs it", plan.manifest.name);
        }
        ["update", names @ ..] => {
            let all: Vec<String> = match names {
                [] => m
                    .installed()
                    .map_err(e)?
                    .into_iter()
                    .map(|l| l.name)
                    .collect(),
                _ => names.iter().map(|n| (*n).to_owned()).collect(),
            };
            for name in all {
                let plan = m.plan_update(&name).map_err(e)?;
                if m.installed()
                    .map_err(e)?
                    .iter()
                    .any(|l| l.name == name && l.commit == plan.commit)
                {
                    println!("{name}: up to date");
                    continue;
                }
                confirm(&plan.text, yes)?;
                m.update(&plan)
                    .map_err(|x| format!("{name}: {x}; the old build keeps running"))?;
                println!("updated {name} to {}", &plan.commit[..7]);
            }
        }
        ["remove", name] => {
            m.remove(name, purge).map_err(e)?;
            let kept = if purge {
                ""
            } else {
                " (its checkout stays: --purge removes it)"
            };
            println!("removed {name}{kept}");
        }
        ["list"] => {
            for r in m.list().map_err(e)? {
                let l = &r.locked;
                let source = if l.listed { "index" } else { "url" };
                println!(
                    "{}  {}  {}  {source}  {}",
                    l.name,
                    l.version,
                    &l.commit[..7],
                    r.state
                );
            }
        }
        _ => return Err(USAGE.to_owned()),
    }
    Ok(())
}

/// `cooee theme add|remove|list|check` (docs/themes.md).
/// `cooee mcp proxy <server> --thread <id>` and `cooee mcp serve --thread <id>`: what a thread's agent runs (#56).
fn mcp_agent(agent: &str, args: &[&str]) -> Result<(), String> {
    match (agent, args) {
        ("proxy", [server, "--thread", thread]) => cooee_server::mcp::proxy(server, thread),
        ("serve", ["--thread", thread]) => cooee_server::mcp::serve(thread),
        _ => Err(USAGE.to_owned()),
    }
}

/// `cooee mcp ls | add | rm | enable | disable` (#56): the registry in the config file, changed on the Mac.
fn mcp(config: &Config, args: &[&str]) -> Result<(), String> {
    use cooee_core::mcp::{Server, save};
    let path = config
        .path
        .as_deref()
        .ok_or("no config file: set HOME or --config")?;
    let mut list = config.mcp.clone();
    let said = match args {
        ["ls"] => {
            for s in &list {
                let on = if s.enabled { "on" } else { "off" };
                println!("[{}] {} · {on} · {}", s.kind, s.name, s.shown());
            }
            return Ok(());
        }
        ["add", name, rest @ ..] => {
            let mut s = Server {
                name: (*name).to_owned(),
                kind: "stdio".to_owned(),
                command: String::new(),
                args: Vec::new(),
                url: String::new(),
                env: std::collections::BTreeMap::new(),
                headers: std::collections::BTreeMap::new(),
                enabled: true,
            };
            let mut words = rest.iter();
            let pair = |v: Option<&&str>| {
                v.and_then(|v| v.split_once('='))
                    .map(|(k, v)| (k.to_owned(), v.to_owned()))
            };
            while let Some(w) = words.next() {
                match *w {
                    "--url" | "--type" if s.command.is_empty() => {
                        let v = words.next().ok_or(USAGE)?;
                        if *w == "--url" {
                            (*v).clone_into(&mut s.url);
                            if s.kind == "stdio" {
                                "http".clone_into(&mut s.kind);
                            }
                        } else {
                            (*v).clone_into(&mut s.kind);
                        }
                    }
                    "--env" | "--header" => {
                        let (k, v) = pair(words.next()).ok_or(USAGE)?;
                        let to = if *w == "--env" {
                            &mut s.env
                        } else {
                            &mut s.headers
                        };
                        to.insert(k, v);
                    }
                    _ if s.command.is_empty() && s.url.is_empty() => {
                        (*w).clone_into(&mut s.command);
                    }
                    _ => s.args.push((*w).to_owned()),
                }
            }
            if list.iter().any(|o| o.name == s.name) {
                return Err(format!(
                    "there is an MCP server {name} already: `cooee mcp rm {name}` first"
                ));
            }
            let said = format!("added {name}: [{}] {}", s.kind, s.shown());
            list.push(s);
            said
        }
        ["rm", name] => {
            list.retain(|s| s.name != *name);
            format!("removed {name}")
        }
        [on @ ("enable" | "disable"), name] => {
            let s = list
                .iter_mut()
                .find(|s| s.name == *name)
                .ok_or(format!("no MCP server {name}"))?;
            s.enabled = *on == "enable";
            format!("{name} is {}", if s.enabled { "on" } else { "off" })
        }
        _ => return Err(USAGE.to_owned()),
    };
    cooee_core::mcp::check(&list)?;
    save(path, &list).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("{said}");
    Ok(())
}

fn theme(config: &Config, args: &[&str], rev: Option<&str>, yes: bool) -> Result<(), String> {
    if let ["check", paths @ ..] = args {
        let mut failed = 0;
        for p in paths {
            match cooee_web::check(Path::new(p)) {
                Ok(()) => println!("ok   {p}"),
                Err(fails) => {
                    failed += 1;
                    println!("FAIL {p}\n     {}", fails.join("\n     "));
                }
            }
        }
        return match (paths.len(), failed) {
            (0, _) => Err(USAGE.to_owned()),
            (_, 0) => Ok(()),
            _ => Err(format!("{failed} theme(s) fail the theme gate")),
        };
    }
    let t = cooee_server::Themes::new(config).map_err(|e| e.to_string())?;
    let e = |e: std::io::Error| e.to_string();
    match args {
        ["add", source] => {
            let pack = t.plan(source, rev).map_err(e)?;
            if pack.listed {
                println!("{}", pack.text);
            } else {
                confirm(&pack.text, yes)?;
            }
            t.add(&pack).map_err(e)?;
            println!("added {}: pick it in Settings › look", pack.name);
        }
        ["remove", name] => {
            t.remove(name).map_err(e)?;
            println!("removed {name}");
        }
        ["list"] => {
            let packs = t.list().map_err(e)?;
            if packs.is_empty() {
                println!(
                    "no themes installed: plain is built in; `cooee theme add <name>` adds one"
                );
            }
            for p in packs {
                let commit = p.commit.get(..7).unwrap_or("-");
                println!("{}  {}  {commit}", p.name, p.source);
            }
        }
        _ => return Err(USAGE.to_owned()),
    }
    Ok(())
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
                println!(
                    "warn no agents installed in shed\n     fix: {}",
                    agents_fix()
                );
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
    match cooee_server::Push::new(&data)
        .with_apns(config.apns.clone())
        .apns_ready()
    {
        Ok(()) => println!("ok   apns ready"),
        Err(why) => println!("warn apns: {why}: the native app gets no pushes"),
    }
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
    if let Some(h) = &config.hooks {
        let up = TcpStream::connect_timeout(&h.bind, Duration::from_secs(2)).is_ok();
        let to = format!("http://{}/hooks/github", h.bind);
        check(
            up,
            format!(
                "hooks listening on {} for {}\n     funnel: tailscale funnel --bg --https=8443 --set-path /hooks/github {to}",
                h.bind, h.url
            ),
        );
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
