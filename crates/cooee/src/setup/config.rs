use std::{fs, path::Path};

use cooee_core::Config;

use super::{Setup, step};
use crate::{cli::load, service::tailnet_name};

impl Setup {
    pub(super) fn config(&self, path: &Path) -> Result<(Config, bool), String> {
        let text = fs::read_to_string(path).unwrap_or_default();
        let keys: toml::Table = text
            .parse()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let bundled = keys
            .get("shed")
            .and_then(toml::Value::as_str)
            .is_some_and(|s| s.contains(".app/Contents/Helpers/"));
        let text = if bundled {
            without_top_key(&text, "shed")
        } else {
            text
        };
        let port = match (
            keys.get("port").and_then(toml::Value::as_integer),
            self.port,
        ) {
            (Some(had), Some(want)) if had != i64::from(want) => {
                return Err(format!(
                    "{} says port = {had}: change it to {want} first",
                    path.display()
                ));
            }
            (Some(had), _) => u16::try_from(had).map_err(|e| e.to_string())?,
            (None, want) => want.unwrap_or(Config::default().port),
        };
        let mut add = Vec::new();
        if !keys.contains_key("port") {
            add.push(format!("port = {port}"));
        }
        if !keys.contains_key("allowed_hosts") {
            match tailnet_name() {
                Some(name) => add.push(format!("allowed_hosts = [{name:?}]")),
                None => {
                    cooee_core::say!(
                        "  note: no tailnet name (is Tailscale up?): allowed_hosts left out"
                    );
                }
            }
        }
        if !keys.contains_key("roots") {
            let root = self.ask("folders for threads [~/Projects]:", "~/Projects");
            add.push(format!("roots = [{root:?}]"));
        }
        let plugins = keys.get("plugins").and_then(toml::Value::as_array);
        let has = plugins.is_some_and(|a| {
            a.iter()
                .any(|p| p.get("name").and_then(toml::Value::as_str) == Some(cooee_terminal::NAME))
        });
        let plugin = if has {
            String::new()
        } else {
            let at = port
                .checked_add(1000)
                .ok_or("no port + 1000 for the terminal plugin")?;
            let token = cooee_core::sha256(&cooee_core::random(32).map_err(|e| e.to_string())?);
            format!(
                "\n[[plugins]]\nname = \"terminal\"\nlabel = \"Terminal\"\nurl = \"http://127.0.0.1:{at}\"\n\
                 module = \"/tab.js\"\ntoken = \"{token}\"\n"
            )
        };
        let changed = bundled || !add.is_empty() || !plugin.is_empty();
        if !step(&format!("config {}", path.display()), !changed) {
            if !text.is_empty() {
                let what = add.iter().map(String::as_str);
                let what: Vec<&str> = what
                    .chain((!has).then_some("the terminal plugin"))
                    .chain(bundled.then_some("shed from PATH, not the old Mac app's"))
                    .collect();
                cooee_core::say!("  note: kept its keys; adding {}", what.join(", "));
            }
            let head: String = add.iter().flat_map(|k| [k.as_str(), "\n"]).collect();
            let new = format!("{head}{text}{plugin}");
            self.write(path, &new, || {
                let dir = path.parent().unwrap_or(path);
                if !dir.exists() {
                    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
                    cooee_core::gitignore::write(dir).map_err(|e| e.to_string())?;
                }
                cooee_core::write_private(path, new.as_bytes()).map_err(|e| e.to_string())
            })?;
        }
        let config = if path.exists() {
            load(Some(path), &self.instance)?
        } else {
            Config {
                port,
                instance: self.instance.clone(),
                ..Config::default()
            }
        };
        Ok((config, changed))
    }
}

fn without_top_key(text: &str, key: &str) -> String {
    let mut top = true;
    text.split_inclusive('\n')
        .filter(|line| {
            top &= !line.trim_start().starts_with('[');
            !(top && line.split_once('=').is_some_and(|(k, _)| k.trim() == key))
        })
        .collect()
}
