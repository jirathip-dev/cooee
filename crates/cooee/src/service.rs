//! The instance as a launchd service: what `cooee install`, `cooee doctor` and `cooee setup` share.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use cooee_core::{Config, SHED_CARGO};

pub(crate) fn shed_install() -> String {
    format!("{} && shed serve", SHED_CARGO.join(" "))
}

/// This machine's tailnet name (`tailscale status`), if Tailscale is up.
pub(crate) fn tailnet_name() -> Option<String> {
    let out = Command::new("tailscale")
        .args(["status", "--json"])
        .output()
        .ok()?;
    let status: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let name = status["Self"]["DNSName"].as_str()?.trim_end_matches('.');
    (!name.is_empty()).then(|| name.to_owned())
}

pub(crate) fn label(instance: &str) -> String {
    format!("dev.cooee.{instance}")
}

/// XML text: `&`, `<` and `>` escaped.
pub(crate) fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// The instance's launchd plist, `~/Library/LaunchAgents/dev.cooee.<instance>.plist`, and its text: `cooee serve`
/// (with `--config <path>` if `explicit`), kept alive, logging to the data folder.
pub(crate) fn plist(
    config: &Config,
    path: &Path,
    explicit: bool,
) -> Result<(PathBuf, String), String> {
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
pub(crate) fn write_plist(config: &Config, file: &Path, plist: &str) -> Result<(), String> {
    let data = config.data_dir().ok_or("no data folder")?;
    fs::create_dir_all(&data).map_err(|e| e.to_string())?;
    fs::create_dir_all(file.parent().unwrap_or(file)).map_err(|e| e.to_string())?;
    fs::write(file, plist).map_err(|e| e.to_string())
}
