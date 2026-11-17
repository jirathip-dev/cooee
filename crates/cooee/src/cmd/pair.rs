use cooee_core::Config;

use crate::{
    cli::{Failed, Flag, Invocation},
    devices::{devices_of, now, pair},
};

pub(super) const USAGE: &str = "pair | pair asks | pair approve|decline <id>";
pub(super) const FLAGS: &[Flag] = &[];

pub(super) fn run(inv: &Invocation) -> Result<(), Failed> {
    Ok(pairing(&inv.config()?, &inv.rest(), inv.usage())?)
}

fn pairing(config: &Config, args: &[&str], usage: &str) -> Result<(), String> {
    match args {
        [] => pair(config),
        ["asks"] => asks(config),
        [verb @ ("approve" | "decline"), id] => answer(config, id, *verb == "approve"),
        _ => Err(usage.to_owned()),
    }
}

fn asks(config: &Config) -> Result<(), String> {
    let asking = devices_of(config)?
        .asking(now())
        .map_err(|e| e.to_string())?;
    cooee_core::say!(
        "{}",
        serde_json::to_string(&asking).map_err(|e| e.to_string())?
    );
    Ok(())
}

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
