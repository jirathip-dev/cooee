use cooee_core::Config;

use crate::cli::{Failed, Flag, Invocation};

pub(super) const USAGE: &str = "reports";
pub(super) const FLAGS: &[Flag] = &[];

pub(super) fn run(inv: &Invocation) -> Result<(), Failed> {
    match inv.rest().as_slice() {
        [] => Ok(reports(&inv.config()?)?),
        _ => Err(Failed::Usage),
    }
}

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
        let s = |k: &str| {
            r.get(k)
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        let at = r.get("at").and_then(serde_json::Value::as_u64).unwrap_or(0) / 1000;
        cooee_core::say!(
            "{at}  {}  {}  {}\n    {}",
            s("device"),
            s("path"),
            s("target"),
            s("note")
        );
    }
    if text.is_empty() {
        cooee_core::say!("no reports");
    }
    Ok(())
}
