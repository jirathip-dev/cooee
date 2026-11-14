//! `cooee update [--check]` (#332): installs main's head once its `ci` run passed, with `scripts/install.sh`'s steps,
//! restarts the instance's service and puts the old binary back if the service doesn't answer.

use std::{
    env,
    fs::{self, OpenOptions},
    io::{IsTerminal, Write},
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use cooee_core::Config;

use super::{label, now, setup};

const INSTALL: &str = include_str!("../../../scripts/install.sh");

/// The commit this binary was built at: `COOEE_REV`, set while `cargo install` builds it by `cooee update`.
pub const REV: Option<&str> = option_env!("COOEE_REV");

fn short(sha: &str) -> &str {
    sha.get(..7).unwrap_or(sha)
}

/// GitHub's API answer at `path` in cooee's repo (public: no token).
fn api(path: &str) -> Result<serde_json::Value, String> {
    let url = format!("https://api.github.com/repos/jirathip-dev/cooee/{path}");
    let out = setup::output(&["curl", "-sfL", "--max-time", "20", &url])
        .ok_or(format!("{url}: no answer"))?;
    serde_json::from_str(&out).map_err(|e| format!("{url}: {e}"))
}

/// The newest `ci` run of a push to main matching `query`.
fn ci_run(query: &str) -> Result<serde_json::Value, String> {
    let runs = api(&format!(
        "actions/workflows/ci.yml/runs?branch=main&event=push&per_page=1&{query}"
    ))?;
    Ok(runs["workflow_runs"][0].clone())
}

/// main's head and its `ci` run's conclusion (none while it runs or before it starts).
fn head() -> Result<(String, Option<String>), String> {
    let head = api("commits/main")?["sha"]
        .as_str()
        .ok_or("main has no sha")?
        .to_owned();
    let run = ci_run(&format!("head_sha={head}"))?;
    Ok((head, run["conclusion"].as_str().map(str::to_owned)))
}

/// The commit to install, or why there is none: main's `head` once its ci run passed, unless it is installed already
/// or failed to install before.
pub fn decide(
    installed: Option<&str>,
    failed: Option<&str>,
    head: &str,
    ci: Option<&str>,
) -> Result<String, String> {
    let at = short(head);
    match ci {
        _ if installed == Some(head) => Err(format!("up to date at {at}")),
        _ if failed == Some(head) => Err(format!(
            "main {at} failed to install before: it waits for a newer one"
        )),
        Some("success") => Ok(head.to_owned()),
        Some(ci) => Err(format!("main {at}: ci {ci}, not installed")),
        None => Err(format!("main {at}: ci pending, not installed")),
    }
}

/// `cooee --version`: the version and the commit it was built at, if known.
pub fn version() -> ExitCode {
    let rev = REV.map(|r| format!(" ({r})")).unwrap_or_default();
    println!("cooee {}{rev}", env!("CARGO_PKG_VERSION"));
    ExitCode::SUCCESS
}

/// `cooee update --check`: what [`run`] would do; nothing changes.
pub fn check(config: &Config) -> Result<(), String> {
    update(config, true)
}

/// `cooee update`: installs main's head if [`decide`] says so.
pub fn run(config: &Config) -> Result<(), String> {
    update(config, false)
}

fn update(config: &Config, check: bool) -> Result<(), String> {
    let data = config.data_dir().ok_or("no data folder")?;
    if !config.auto_update {
        println!("auto-update is off (auto_update = false in the config)");
        return Ok(());
    }
    let failed = fs::read_to_string(data.join("update.failed")).ok();
    let (head, ci) = head()?;
    let sha = match decide(REV, failed.as_deref(), &head, ci.as_deref()) {
        Ok(sha) => sha,
        Err(why) if !check => return log(&data, &why),
        Err(why) => {
            println!("would do nothing: {why}");
            return Ok(());
        }
    };
    let job = label(&config.instance);
    if check {
        println!(
            "would install {sha}, restart {job} and check that port {} answers",
            config.port
        );
        return Ok(());
    }
    match install(config, &bin()?, &sha) {
        Ok(()) => log(&data, &format!("installed {sha}")),
        Err(why) => {
            fs::write(data.join("update.failed"), &sha).map_err(|e| e.to_string())?;
            let note = cooee_server::Note {
                title: "cooee update".to_owned(),
                body: why.chars().take(120).collect(),
                tag: "update".to_owned(),
                url: "/".to_owned(),
                item: 0,
            };
            let push = cooee_server::Push::new(&data).with_apns(config.apns.clone());
            let sent = push
                .deliver(None, &note)
                .map_or_else(|e| format!(": {e}"), |l| l.concat());
            log(&data, &format!("{why}; push{sent}"))?;
            Err(why)
        }
    }
}

/// The installed binary, where `scripts/install.sh` puts it.
fn bin() -> Result<PathBuf, String> {
    let home = env::var_os("HOME").ok_or("no HOME")?;
    Ok(Path::new(&home).join(".cargo/bin/cooee"))
}

/// Installs `sha` over `bin`, keeping the old one; if that fails or the restarted service doesn't answer, puts the old
/// one back and restarts it.
fn install(config: &Config, bin: &Path, sha: &str) -> Result<(), String> {
    let old = bin.with_extension("old");
    fs::copy(bin, &old).map_err(|e| format!("keeping {}: {e}", bin.display()))?;
    let installed = Command::new("sh")
        .args(["-c", INSTALL, "install.sh"])
        .env("COOEE_REV", sha)
        .status()
        .map_err(|e| e.to_string())?;
    let why = if !installed.success() {
        format!("install.sh {installed}")
    } else if restart(config) {
        return Ok(());
    } else {
        format!("the service doesn't answer on port {}", config.port)
    };
    fs::rename(&old, bin).map_err(|e| format!("{why}; putting back {}: {e}", bin.display()))?;
    let back = if restart(config) {
        "it serves again"
    } else {
        "it doesn't answer either"
    };
    Err(format!(
        "{} not installed: {why}; rolled back to the old binary: {back}",
        short(sha)
    ))
}

/// Restarts the instance's service; whether it answers on its port.
fn restart(config: &Config) -> bool {
    let uid = setup::output(&["id", "-u"]).unwrap_or_default();
    let job = format!("gui/{}/{}", uid.trim(), label(&config.instance));
    let url = format!("http://127.0.0.1:{}/pair", config.port);
    setup::ok(&["launchctl", "kickstart", "-k", &job])
        && setup::wait(|| setup::ok(&["curl", "-sf", "-o", "/dev/null", "--max-time", "2", &url]))
}

/// Appends `line` to the one log, `<data>/update.log`, with the time; shows it in a terminal.
fn log(data: &Path, line: &str) -> Result<(), String> {
    if std::io::stdout().is_terminal() {
        println!("{line}");
    }
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(data.join("update.log"));
    file.and_then(|mut f| writeln!(f, "{} {line}", now() / 1000))
        .map_err(|e| e.to_string())
}

/// `cooee doctor`'s line: the installed commit, main's green head, the last check and whether auto-update is on.
pub fn doctor(config: &Config) {
    let green =
        ci_run("status=success").map(|r| r["head_sha"].as_str().map_or("none", short).to_owned());
    let log = config.data_dir().map(|d| d.join("update.log"));
    let ago = log.and_then(|l| fs::metadata(l).ok()?.modified().ok()?.elapsed().ok());
    println!(
        "info update: installed {}, main's green head {}, last check {}, auto-update {}",
        REV.map_or("unknown", short),
        green.unwrap_or_else(|e| e),
        ago.map_or("never".to_owned(), |a| format!(
            "{} min ago",
            a.as_secs() / 60
        )),
        if config.auto_update { "on" } else { "off" }
    );
}

#[cfg(test)]
mod tests {
    use super::decide;

    const OLD: &str = "1111111111111111111111111111111111111111";
    const NEW: &str = "2222222222222222222222222222222222222222";

    #[test]
    fn only_a_new_green_head_is_installed() {
        assert_eq!(
            decide(Some(OLD), None, NEW, Some("success")),
            Ok(NEW.to_owned())
        );
        assert_eq!(
            decide(None, Some(OLD), NEW, Some("success")),
            Ok(NEW.to_owned())
        );
        assert_eq!(
            decide(Some(OLD), None, NEW, Some("failure")),
            Err("main 2222222: ci failure, not installed".to_owned())
        );
        assert_eq!(
            decide(Some(OLD), None, NEW, None),
            Err("main 2222222: ci pending, not installed".to_owned())
        );
        assert_eq!(
            decide(Some(NEW), None, NEW, Some("success")),
            Err("up to date at 2222222".to_owned())
        );
        assert!(
            decide(Some(OLD), Some(NEW), NEW, Some("success"))
                .is_err_and(|e| e.contains("failed to install before"))
        );
    }
}
