use cooee_core::{Config, Store};

use crate::cli::{Failed, Flag, Invocation};

pub(super) const USAGE: &str = "export <thread>";
pub(super) const FLAGS: &[Flag] = &[];

pub(super) fn run(inv: &Invocation) -> Result<(), Failed> {
    match inv.rest().as_slice() {
        [id] => Ok(export(&inv.config()?, id)?),
        _ => Err(Failed::Usage),
    }
}

fn export(config: &Config, id: &str) -> Result<(), String> {
    let data = config
        .data_dir()
        .ok_or("no data folder: set `data` or HOME")?;
    let items = Store::open(&data)
        .and_then(|s| s.items(id))
        .map_err(|e| e.to_string())?;
    for item in items {
        cooee_core::say!(
            "{}",
            serde_json::to_string(&item).map_err(|e| e.to_string())?
        );
    }
    Ok(())
}
