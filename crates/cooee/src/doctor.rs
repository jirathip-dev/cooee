use std::{
    net::{Ipv4Addr, SocketAddr, TcpStream},
    time::Duration,
};

use cooee_core::{Config, ROOTS_FIX, Shed, ShedError, agents_fix};
use cooee_server::machine;

use crate::service::{label, shed_install, tailnet_name};

pub(crate) fn doctor(config: &Config) -> Result<(), String> {
    let mut failed = Vec::new();
    let mut check = |ok: bool, what: String| {
        println!("{} {what}", if ok { "ok  " } else { "FAIL" });
        if !ok {
            failed.push(format!("FAIL {what}"));
        }
    };
    let label = label(&config.instance);
    let loaded = cooee_core::runner::command("launchctl")
        .args(["list", &label])
        .output()
        .is_ok_and(|o| o.status.success());
    check(
        loaded,
        format!("service {label} loaded (else: cooee install)"),
    );
    let port = config.port;
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let up = TcpStream::connect_timeout(&addr, Duration::from_secs(2)).is_ok();
    check(up, format!("listening on port {port}"));
    let shed = Shed::new(config.shed.clone(), config.shed_dir.clone());
    match shed.hello() {
        Ok(()) => {
            check(true, format!("shed at {}", shed.socket.display()));
            if shed.agents().is_ok_and(|a| a.is_empty()) {
                println!(
                    "warn no agents installed in shed\n     fix: {}",
                    agents_fix()
                );
            }
        }
        Err(e @ ShedError::Version { .. }) => check(false, format!("shed: {e}")),
        Err(e) => check(
            false,
            format!("shed: {e}\n     install: {}", shed_install()),
        ),
    }
    for (ok, what) in machine::rows(config) {
        check(ok, what);
    }
    if config.roots.is_empty() {
        println!("warn no roots configured\n     fix: add {ROOTS_FIX} to config.toml");
    }
    let data = config.data_dir().ok_or("no data folder")?;
    match cooee_server::Push::new(&data)
        .with_apns(config.apns.clone())
        .apns_ready()
    {
        Ok(()) => println!("ok   apns ready"),
        Err(why) => println!("warn apns: {why}: the native app gets no pushes"),
    }
    match cooee_server::Push::new(&data).has_key() {
        Ok(true) => println!("ok   push keys stored"),
        Ok(false) => {
            println!("ok   push keys: none yet (made when a phone turns on notifications)");
        }
        Err(e) => check(false, format!("push keys: {e}")),
    }
    crate::update::doctor(config);
    match tailnet_name() {
        None => check(false, "tailnet: no name (is Tailscale up?)".to_owned()),
        Some(name) => {
            let url = format!("https://{name}:{port}/");
            if !config.allowed_hosts.contains(&name) {
                check(false, format!("{url}: add {name:?} to allowed_hosts"));
            }
            let reached = cooee_core::runner::command("curl")
                .args(["-sf", "-o", "/dev/null", "--max-time", "5"])
                .arg(format!("{url}pair"))
                .status()
                .is_ok_and(|s| s.success());
            check(
                reached,
                format!(
                    "{url} (else: tailscale serve --bg --https={port} http://127.0.0.1:{port})"
                ),
            );
        }
    }
    if failed.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{} check(s) failed:\n{}",
            failed.len(),
            failed.join("\n")
        ))
    }
}
