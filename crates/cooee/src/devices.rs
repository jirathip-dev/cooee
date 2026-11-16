use std::{
    io::{IsTerminal, Write},
    time::{Duration, UNIX_EPOCH},
};

use cooee_core::{Config, Devices};
use qrcode::{QrCode, render::unicode::Dense1x2};

pub(crate) fn devices_of(config: &Config) -> Result<Devices, String> {
    let data = config
        .data_dir()
        .ok_or("no data folder: set `data` or HOME")?;
    Ok(Devices::new(&data))
}

pub(crate) fn pair(config: &Config) -> Result<(), String> {
    let devices = devices_of(config)?;
    let had = devices.list().map_err(|e| e.to_string())?.len();
    let code = devices.pair(now()).map_err(|e| e.to_string())?;
    let url = match config.allowed_hosts.first() {
        Some(host) => format!("https://{host}:{}/pair?code={code}", config.port),
        None => format!("http://127.0.0.1:{}/pair?code={code}", config.port),
    };
    cooee_core::say!("Type the link into the cooee app on the new device.");
    if let Ok(qr) = QrCode::new(&url) {
        cooee_core::say!("{}", qr.render::<Dense1x2>().quiet_zone(true).build());
    }
    cooee_core::say!(
        "code  {} {}  (one use, 10 minutes)\nlink  {url}",
        &code[..3],
        &code[3..]
    );
    if !std::io::stdout().is_terminal() {
        return Ok(());
    }
    cooee_core::say_open!("waiting…  ");
    std::io::stdout().flush().ok();
    let until = now() + 10 * 60 * 1000;
    while now() < until && devices.code_open(now()).unwrap_or(false) {
        cooee_core::clock::sleep(Duration::from_secs(1));
    }
    match devices.list().map_err(|e| e.to_string())?.get(had) {
        Some(d) => cooee_core::say!("✓ paired: {}", d.label),
        None => cooee_core::say!("the code ran out: run cooee pair again"),
    }
    Ok(())
}

pub(crate) fn now() -> u64 {
    cooee_core::clock::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}
