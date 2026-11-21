use std::{
    env,
    fs::{self, OpenOptions},
    io::{IsTerminal, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::ExitCode,
};

use cooee_core::Config;

use crate::{
    devices::now,
    service::{label, service},
    setup,
};

/// The commit this binary was built at: `COOEE_REV`, set by the release workflow.
pub const REV: Option<&str> = option_env!("COOEE_REV");

const RELEASES: &str = "https://github.com/jirathip-dev/cooee/releases/download";

fn short(sha: &str) -> &str {
    sha.get(..7).unwrap_or(sha)
}

fn api(path: &str) -> Result<serde_json::Value, String> {
    let url = format!("https://api.github.com/repos/jirathip-dev/cooee/{path}");
    let out = setup::output(&["curl", "-sfL", "--max-time", "20", &url])
        .ok_or(format!("{url}: no answer"))?;
    serde_json::from_str(&out).map_err(|e| format!("{url}: {e}"))
}

fn head() -> Result<String, String> {
    Ok(api("commits/main")?
        .get("sha")
        .and_then(serde_json::Value::as_str)
        .ok_or("main has no sha")?
        .to_owned())
}

fn asset() -> String {
    let os = if cfg!(target_os = "macos") {
        "apple-darwin"
    } else {
        "unknown-linux-gnu"
    };
    format!("cooee-{}-{os}", env::consts::ARCH)
}

fn url(sha: &str, file: &str) -> String {
    format!("{RELEASES}/build-{}/{file}", short(sha))
}

/// The commit to install, or why there is none: main's `head`, unless it is installed already or failed to install
/// before.
pub fn decide(installed: Option<&str>, failed: Option<&str>, head: &str) -> Result<String, String> {
    let at = short(head);
    if installed == Some(head) {
        Err(format!("up to date at {at}"))
    } else if failed == Some(head) {
        Err(format!(
            "main {at} failed to install before: it waits for a newer one"
        ))
    } else {
        Ok(head.to_owned())
    }
}

fn sum(sha: &str) -> Result<String, String> {
    let at = short(sha);
    let sums = setup::output(&["curl", "-sfL", "--max-time", "20", &url(sha, "SHA256SUMS")])
        .ok_or(format!("main {at}: no release yet, not installed"))?;
    let name = asset();
    sums.lines()
        .find_map(|l| l.split_once("  ").filter(|(_, f)| *f == name))
        .map(|(sum, _)| sum.to_owned())
        .ok_or(format!(
            "main {at}: the release has no {name}, not installed"
        ))
}

/// `cooee --version`: the version and the commit it was built at, if known.
pub fn version() -> ExitCode {
    let rev = REV.map(|r| format!(" ({r})")).unwrap_or_default();
    cooee_core::say!("cooee {}{rev}", env!("CARGO_PKG_VERSION"));
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
        cooee_core::say!("auto-update is off (auto_update = false in the config)");
        return Ok(());
    }
    let failed = fs::read_to_string(data.join("update.failed")).ok();
    let head = head()?;
    let found = decide(REV, failed.as_deref(), &head).and_then(|sha| Ok((sum(&sha)?, sha)));
    let (sum, sha) = match found {
        Ok(found) => found,
        Err(why) if !check => return log(&data, &why),
        Err(why) => {
            cooee_core::say!("would do nothing: {why}");
            return Ok(());
        }
    };
    let job = label(&config.instance);
    if check {
        cooee_core::say!(
            "would install {sha}'s release, restart {job} and check that port {} answers",
            config.port
        );
        return Ok(());
    }
    match install(config, &bin()?, &sha, &sum) {
        Ok(()) => log(&data, &format!("installed {sha}")),
        Err(why) => {
            fs::write(data.join("update.failed"), &sha).map_err(|e| e.to_string())?;
            let note = cooee_server::Note {
                title: "cooee update".to_owned(),
                body: why.chars().take(120).collect(),
                tag: "update".to_owned(),
                url: "/".to_owned(),
                item: 0,
                say: false,
            };
            let push = cooee_server::Push::new(&data).with_apns(config.apns.clone());
            log(&data, &why)?;
            push.send_now(None, &note);
            Err(why)
        }
    }
}

fn bin() -> Result<PathBuf, String> {
    let home = cooee_core::env_var("HOME").ok_or("no HOME")?;
    Ok(Path::new(&home).join(".local/bin/cooee"))
}

fn fetch(new: &Path, sha: &str, sum: &str) -> Result<(), String> {
    let to = new.display().to_string();
    let name = asset();
    if !setup::ok(&[
        "curl",
        "-sfL",
        "--max-time",
        "300",
        "-o",
        &to,
        &url(sha, &name),
    ]) {
        return Err(format!("downloading {name} failed"));
    }
    let got = cooee_core::sha256(&fs::read(new).map_err(|e| e.to_string())?);
    if got != sum {
        return Err(format!("{name} doesn't match SHA256SUMS"));
    }
    fs::set_permissions(new, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    let says = setup::output(&[&to, "--version"]).unwrap_or_default();
    if !says.contains(sha) {
        return Err(format!(
            "its --version names another build: {}",
            says.trim()
        ));
    }
    Ok(())
}

fn install(config: &Config, bin: &Path, sha: &str, sum: &str) -> Result<(), String> {
    let new = bin.with_extension("new");
    if let Err(why) = fetch(&new, sha, sum) {
        fs::remove_file(&new).ok();
        return Err(format!("{} not installed: {why}", short(sha)));
    }
    let old = bin.with_extension("old");
    fs::copy(bin, &old).map_err(|e| format!("keeping {}: {e}", bin.display()))?;
    fs::rename(&new, bin).map_err(|e| format!("swapping {}: {e}", bin.display()))?;
    if restart(config) {
        return Ok(());
    }
    let why = format!("the service doesn't answer on port {}", config.port);
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

fn restart(config: &Config) -> bool {
    let url = format!("http://127.0.0.1:{}/pair", config.port);
    service(config, None, "serve").is_ok_and(|s| s.restart().is_ok())
        && setup::wait(|| setup::ok(&["curl", "-sf", "-o", "/dev/null", "--max-time", "2", &url]))
}

fn log(data: &Path, line: &str) -> Result<(), String> {
    if std::io::stdout().is_terminal() {
        cooee_core::say!("{line}");
    }
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(data.join("update.log"));
    file.and_then(|mut f| writeln!(f, "{} {line}", now() / 1000))
        .map_err(|e| e.to_string())
}

/// `cooee doctor`'s line: the installed commit, main's latest release, the last check and whether auto-update is on.
pub fn doctor(config: &Config) {
    let latest = api("releases/latest").map(|r| {
        r.get("tag_name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("none")
            .to_owned()
    });
    let log = config.data_dir().map(|d| d.join("update.log"));
    let ago = log.and_then(|l| fs::metadata(l).ok()?.modified().ok()?.elapsed().ok());
    cooee_core::say!(
        "info update: installed {}, main's latest release {}, last check {}, auto-update {}",
        REV.map_or("unknown", short),
        latest.unwrap_or_else(|e| e),
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
    fn only_a_new_head_that_never_failed_is_installed() {
        assert_eq!(decide(Some(OLD), None, NEW), Ok(NEW.to_owned()));
        assert_eq!(decide(None, Some(OLD), NEW), Ok(NEW.to_owned()));
        assert_eq!(
            decide(Some(NEW), None, NEW),
            Err("up to date at 2222222".to_owned())
        );
        assert!(
            decide(Some(OLD), Some(NEW), NEW)
                .is_err_and(|e| e.contains("failed to install before"))
        );
    }
}
