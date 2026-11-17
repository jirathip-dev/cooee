use std::{
    net::{Ipv4Addr, SocketAddr, TcpStream},
    time::Duration,
};

use cooee_core::{Config, ROOTS_FIX, Shed, ShedError, agents_fix};
use cooee_server::machine;

use crate::service::{app_items, label, shed_install, tailnet_name};

pub(crate) fn doctor(config: &Config) -> Result<(), String> {
    let mut failed = Vec::new();
    let mut check = |ok: bool, what: String| {
        cooee_core::say!("{} {what}", if ok { "ok  " } else { "FAIL" });
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
    let app = app_items();
    if loaded && !app.is_empty() {
        cooee_core::say!(
            "warn both {label} and the Mac app's login items ({}) are installed: two servers on one Mac\n     \
             fix: in the app, take over the CLI install, or remove the app's login items",
            app.join(", ")
        );
    }
    let port = config.port;
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let up = TcpStream::connect_timeout(&addr, Duration::from_secs(2)).is_ok();
    check(up, format!("listening on port {port}"));
    let shed = Shed::new(config.shed.clone(), config.shed_dir.clone());
    match shed.hello() {
        Ok(()) => {
            check(true, format!("shed at {}", shed.socket.display()));
            if shed.agents().is_ok_and(|a| a.is_empty()) {
                cooee_core::say!(
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
        cooee_core::say!("warn no roots configured\n     fix: add {ROOTS_FIX} to config.toml");
    }
    let data = config.data_dir().ok_or("no data folder")?;
    match cooee_server::Push::new(&data)
        .with_apns(config.apns.clone())
        .apns_ready()
    {
        Ok(()) => cooee_core::say!("ok   apns ready"),
        Err(why) => cooee_core::say!("warn apns: {why}: the native app gets no pushes"),
    }
    match cooee_server::Push::new(&data).has_key() {
        Ok(true) => cooee_core::say!("ok   push keys stored"),
        Ok(false) => {
            cooee_core::say!("ok   push keys: none yet (made when a phone turns on notifications)");
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
    if let Some((up, what)) = hooks(config) {
        check(up, what);
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

fn hooks(config: &Config) -> Option<(bool, String)> {
    let h = config.hooks.as_ref()?;
    let up = TcpStream::connect_timeout(&h.bind, Duration::from_secs(2)).is_ok();
    let to = format!("http://{}/hooks/github", h.bind);
    Some((
        up,
        format!(
            "hooks listening on {} for {}\n     funnel: tailscale funnel --bg --https=8443 --set-path /hooks/github {to}",
            h.bind, h.url
        ),
    ))
}
