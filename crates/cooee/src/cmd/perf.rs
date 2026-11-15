//! `cooee perf [--minutes <n>]`: the latency table (#53).

use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

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

/// Prints p50/p95/max per route, shed method and first token over the last `minutes` of `perf.log` (#53).
fn perf(config: &Config, minutes: u64) -> Result<(), String> {
    let data = config
        .data_dir()
        .ok_or("no data folder: set `data` or HOME")?;
    let text = match fs::read_to_string(data.join(cooee_core::perf::FILE)) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e.to_string()),
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
    let since = now.saturating_sub(minutes.saturating_mul(60_000));
    print!("{}", cooee_core::perf::table(&text, since));
    Ok(())
}
