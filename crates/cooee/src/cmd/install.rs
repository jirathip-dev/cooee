use std::{
    fs,
    path::{Path, PathBuf},
};

use cooee_core::Shed;

use crate::{
    cli::{Failed, Flag, Invocation, load},
    service::{service, tailnet_name},
};

pub(super) const USAGE: &str = "install --port <p>";
pub(super) const FLAGS: &[Flag] = &[Flag::Value("--port")];

pub(super) fn run(inv: &Invocation) -> Result<(), Failed> {
    if !inv.rest().is_empty() {
        return Err(Failed::Usage);
    }
    let Some(port) = inv.value("--port").and_then(|p| p.parse().ok()) else {
        return Err(Failed::Usage);
    };
    let path = inv.path().map(Path::to_path_buf);
    Ok(install(path, inv.instance(), port, inv.explicit())?)
}

fn install(path: Option<PathBuf>, instance: &str, port: u16, explicit: bool) -> Result<(), String> {
    let path = path.ok_or("no config path: set HOME")?;
    if !path.exists() {
        let hosts = tailnet_name().map(|n| format!("allowed_hosts = [{n:?}]\n"));
        let toml = format!("port = {port}\n{}", hosts.unwrap_or_default());
        fs::create_dir_all(path.parent().unwrap_or(&path)).map_err(|e| e.to_string())?;
        fs::write(&path, toml).map_err(|e| e.to_string())?;
        cooee_core::say!("wrote {}", path.display());
    }
    let config = load(Some(&path), instance)?;
    if config.port != port {
        return Err(format!(
            "{} says port = {}: change it to {port} first",
            path.display(),
            config.port
        ));
    }
    let service = service(&config, explicit.then_some(&path), "serve")?;
    let done = service.install().map_err(|e| e.to_string())?;
    let commands = service.commands();
    cooee_core::say!("{} {done}", service.label());
    cooee_core::say!("  restart: {}\n  log: {}", commands.restart, commands.log);
    cooee_core::say!("\nrun these yourself:");
    cooee_core::say!("  tailscale serve --bg --https={port} http://127.0.0.1:{port}");
    if Shed::new(config.shed.clone(), config.shed_dir.clone())
        .hello()
        .is_err()
    {
        cooee_core::say!("  shed isn't running: {}", cooee_core::shed_fix());
    }
    Ok(())
}
