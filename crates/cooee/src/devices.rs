use std::{
    io::{IsTerminal, Write},
    time::{Duration, SystemTime, UNIX_EPOCH},
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
    println!("Type the link into the cooee app on the new device.");
    if let Ok(qr) = QrCode::new(&url) {
        println!("{}", qr.render::<Dense1x2>().quiet_zone(true).build());
    }
    println!(
        "code  {} {}  (one use, 10 minutes)\nlink  {url}",
        &code[..3],
        &code[3..]
    );
    if !std::io::stdout().is_terminal() {
        return Ok(());
    }
    print!("waiting…  ");
    let _ = std::io::stdout().flush();
    let until = now() + 10 * 60 * 1000;
    while now() < until && devices.code_open(now()).unwrap_or(false) {
        std::thread::sleep(Duration::from_secs(1));
    }
    match devices.list().map_err(|e| e.to_string())?.get(had) {
        Some(d) => println!("✓ paired: {}", d.label),
        None => println!("the code ran out: run cooee pair again"),
    }
    Ok(())
}

pub(crate) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}
