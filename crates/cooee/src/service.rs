use std::{env, ffi::OsString, path::Path, time::Duration};

use cooee_core::Config;
use farrier_service::{Env, Job, Log, Restart, Schedule, Service};

pub(crate) fn tailnet_name() -> Option<String> {
    let out = cooee_core::runner::command("tailscale")
        .args(["status", "--json"])
        .output()
        .ok()?;
    let status: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let name = status
        .pointer("/Self/DNSName")?
        .as_str()?
        .trim_end_matches('.');
    (!name.is_empty()).then(|| name.to_owned())
}

pub(crate) fn label(instance: &str) -> String {
    format!("dev.cooee.{instance}")
}

pub(crate) fn app_items() -> Vec<&'static str> {
    ["com.jirathip.cooee.serve", "com.jirathip.cooee.shed"]
        .into_iter()
        .filter(|l| {
            cooee_core::runner::command("launchctl")
                .args(["list", l])
                .output()
                .is_ok_and(|o| o.status.success())
        })
        .collect()
}

pub(crate) fn service(
    config: &Config,
    config_arg: Option<&Path>,
    verb: &str,
) -> Result<Service, String> {
    let data = config.data_dir().ok_or("no data folder")?;
    let exe = env::current_exe().map_err(|e| e.to_string())?;
    let mut argv: Vec<OsString> = vec![
        exe.into(),
        verb.into(),
        "--instance".into(),
        config.instance.clone().into(),
    ];
    if let Some(path) = config_arg {
        argv.extend(["--config".into(), path.into()]);
    }
    let (name, log, schedule) = if verb == "serve" {
        let keep = Schedule::KeepAlive(Restart::Always);
        (label(&config.instance), "cooee.log", keep)
    } else {
        let every = Schedule::Every(Duration::from_mins(15));
        (
            format!("{}.{verb}", label(&config.instance)),
            "update.log",
            every,
        )
    };
    let path = cooee_core::env_var("PATH")
        .and_then(|v| v.into_string().ok())
        .unwrap_or_default();
    let job = Job::new(&name, argv, schedule)
        .map_err(|e| e.to_string())?
        .path(&path)
        .log(Log::File(data.join(log)));
    let host = Env::host().map_err(|e| e.to_string())?;
    Ok(Service::new(job, &host))
}
