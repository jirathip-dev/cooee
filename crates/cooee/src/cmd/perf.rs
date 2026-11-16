use std::{fs, time::UNIX_EPOCH};

use cooee_core::Config;

use crate::cli::{Failed, Flag, Invocation};

pub(super) const USAGE: &str = "perf [--minutes <n>]";
pub(super) const FLAGS: &[Flag] = &[Flag::Value("--minutes")];

pub(super) fn run(inv: &Invocation) -> Result<(), Failed> {
    if !inv.rest().is_empty() {
        return Err(Failed::Usage);
    }
    let Ok(minutes) = inv.value("--minutes").map_or(Ok(60), str::parse::<u64>) else {
        return Err(Failed::Usage);
    };
    Ok(perf(&inv.config()?, minutes)?)
}

fn perf(config: &Config, minutes: u64) -> Result<(), String> {
    let data = config
        .data_dir()
        .ok_or("no data folder: set `data` or HOME")?;
    let text = match fs::read_to_string(data.join(cooee_core::perf::FILE)) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e.to_string()),
    };
    let now = cooee_core::clock::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
    let since = now.saturating_sub(minutes.saturating_mul(60_000));
    cooee_core::say_open!("{}", cooee_core::perf::table(&text, since));
    Ok(())
}
