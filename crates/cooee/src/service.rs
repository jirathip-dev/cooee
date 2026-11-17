use std::{
    env, fs,
    path::{Path, PathBuf},
};

use cooee_core::{Config, SHED_CARGO};

pub(crate) fn shed_install() -> String {
    format!("{} && shed serve", SHED_CARGO.join(" "))
}

pub(crate) fn tailnet_name() -> Option<String> {
    let out = cooee_core::runner::command("tailscale")
        .args(["status", "--json"])
        .output()
        .ok()?;
    let status: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let name = status
        .pointer("/Self/DNSName")?
        .as_str()?
        .trim_end_matches('.');
    (!name.is_empty()).then(|| name.to_owned())
}

pub(crate) fn label(instance: &str) -> String {
    format!("dev.cooee.{instance}")
}

pub(crate) fn app_items() -> Vec<&'static str> {
    ["com.jirathip.cooee.serve", "com.jirathip.cooee.shed"]
        .into_iter()
        .filter(|l| {
            cooee_core::runner::command("launchctl")
                .args(["list", l])
                .output()
                .is_ok_and(|o| o.status.success())
        })
        .collect()
}

pub(crate) fn refuse_app() -> Result<(), String> {
    let items = app_items();
    if items.is_empty() {
        return Ok(());
    }
    Err(format!(
        "the cooee Mac app already runs this Mac's server (login items {}): use the app, or remove its login \
         items in System Settings > General > Login Items first",
        items.join(", ")
    ))
}

pub(crate) fn job(instance: &str, verb: &str) -> String {
    match verb {
        "serve" => label(instance),
        _ => format!("{}.{verb}", label(instance)),
    }
}

pub(crate) fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub(crate) fn plist(
    config: &Config,
    path: &Path,
    explicit: bool,
    verb: &str,
) -> Result<(PathBuf, String), String> {
    let home = cooee_core::env_var("HOME").ok_or("no HOME")?;
    let data = config.data_dir().ok_or("no data folder")?;
    let exe = env::current_exe().map_err(|e| e.to_string())?;
    let mut argv = vec![
        exe.display().to_string(),
        verb.into(),
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
    let (log, run) = if verb == "serve" {
        ("cooee.log", "KeepAlive</key>\n  <true/>")
    } else {
        (
            "update.log",
            "StartInterval</key>\n  <integer>900</integer>",
        )
    };
    let log = xml(&data.join(log).display().to_string());
    let env_path = xml(&cooee_core::env_var("PATH")
        .and_then(|v| v.into_string().ok())
        .unwrap_or_default());
    let label = job(&config.instance, verb);
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
  <key>{run}
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

pub(crate) fn write_plist(config: &Config, file: &Path, plist: &str) -> Result<(), String> {
    let data = config.data_dir().ok_or("no data folder")?;
    fs::create_dir_all(&data).map_err(|e| e.to_string())?;
    fs::create_dir_all(file.parent().unwrap_or(file)).map_err(|e| e.to_string())?;
    fs::write(file, plist).map_err(|e| e.to_string())
}
