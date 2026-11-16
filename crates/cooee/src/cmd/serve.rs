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
    let machines = config.plugins.iter().find(|p| p.name == "machines");
    let terminal = config
        .plugins
        .iter()
        .find(|p| p.name == cooee_terminal::NAME);
    if let Some(data) = config.data_dir() {
        std::thread::spawn(move || {
            match keychain::migrate(&data) {
                Ok(Some(done)) => eprintln!("cooee: {done}"),
                Ok(None) => {}
                Err(e) => eprintln!("cooee: push key: {e}"),
            }
            match cooee_core::secrets::migrate(&data) {
                Ok(moved) if moved.is_empty() => {}
                Ok(moved) => eprintln!("cooee: moved into the keychain: {}", moved.join(", ")),
                Err(e) => eprintln!("cooee: secrets: {e}"),
            }
        });
    }
    cooee_server::serve(config, |addr| {
        println!("cooee: listening on http://{addr}");
        if let (Some(p), Some(data)) = (terminal, config.data_dir()) {
            let shed = Shed::new(config.shed.clone(), config.shed_dir.clone());
            if let Err(e) = cooee_terminal::start(p, shed, &data) {
                eprintln!("cooee: plugin terminal at {}: {e}", p.url);
            }
        }
        let Some(p) = machines else { return };
        let ip = if addr.ip().is_unspecified() {
            Ipv4Addr::LOCALHOST.into()
        } else {
            addr.ip()
        };
        let (shed, dir) = (config.shed.clone(), config.shed_dir.clone());
        if let Err(e) = cooee_machines::start(p, shed, dir, SocketAddr::new(ip, addr.port())) {
            eprintln!("cooee: plugin machines at {}: {e}", p.url);
        }
    })
    .map_err(|e| e.to_string())
}
