use cooee_core::Config;

use crate::{
    cli::{Failed, Flag, Invocation},
    devices::devices_of,
};

pub(super) const USAGE: &str = "devices | devices rm <id>";
pub(super) const FLAGS: &[Flag] = &[];

pub(super) fn run(inv: &Invocation) -> Result<(), Failed> {
    match inv.rest().as_slice() {
        [] => Ok(devices(&inv.config()?)?),
        ["rm", id] => Ok(remove(&inv.config()?, id)?),
        _ => Err(Failed::Usage),
    }
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
