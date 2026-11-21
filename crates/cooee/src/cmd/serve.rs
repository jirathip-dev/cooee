use std::net::{Ipv4Addr, SocketAddr};

use cooee_core::{Config, Shed};

use crate::{
    cli::{Failed, Flag, Invocation},
    keychain,
};

pub(super) const USAGE: &str = "serve";
pub(super) const FLAGS: &[Flag] = &[];

pub(super) fn run(inv: &Invocation) -> Result<(), Failed> {
    match inv.rest().as_slice() {
        [] => Ok(serve(&inv.config()?)?),
        _ => Err(Failed::Usage),
    }
}

fn serve(config: &Config) -> Result<(), String> {
    if let Err(e) = crate::cli::migrate(config, false) {
        cooee_core::note!("cooee: {e}");
    }
    let mut config = config.clone();
    let (data, shed) = (config.data_dir(), config.shed.clone());
    let shed = Shed::new(shed, config.shed_dir.clone());
    let terminal = (config.plugins.iter_mut()).find(|p| p.name == cooee_terminal::NAME);
    if let (Some(p), Some(data)) = (terminal, &data) {
        match cooee_terminal::start(p, shed, data) {
            Ok(()) => cooee_core::note!("cooee: plugin terminal on {}", p.url),
            Err(e) => cooee_core::note!("cooee: plugin terminal at {}: {e}", p.url),
        }
    }
    let config = &config;
    let machines = config.plugins.iter().find(|p| p.name == "machines");
    if let Some(data) = data {
        std::thread::spawn(move || {
            match keychain::migrate(&data) {
                Ok(Some(done)) => cooee_core::note!("cooee: {done}"),
                Ok(None) => {}
                Err(e) => cooee_core::note!("cooee: push key: {e}"),
            }
            match cooee_core::secrets::migrate(&data) {
                Ok(moved) if moved.is_empty() => {}
                Ok(moved) => {
                    cooee_core::note!("cooee: moved into the keychain: {}", moved.join(", "));
                }
                Err(e) => cooee_core::note!("cooee: secrets: {e}"),
            }
        });
    }
    cooee_server::serve(config, |addr| {
        cooee_core::say!("cooee: listening on http://{addr}");
        let Some(p) = machines else { return };
        let ip = if addr.ip().is_unspecified() {
            Ipv4Addr::LOCALHOST.into()
        } else {
            addr.ip()
        };
        let (shed, dir) = (config.shed.clone(), config.shed_dir.clone());
        let addr = SocketAddr::new(ip, addr.port());
        if let Err(e) = cooee_machines::start(p, shed, dir, config.container.clone(), addr) {
            cooee_core::note!("cooee: plugin machines at {}: {e}", p.url);
        }
    })
    .map_err(|e| e.to_string())
}
