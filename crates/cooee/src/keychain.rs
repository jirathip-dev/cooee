use std::{io::Read, path::Path, process::Stdio, time::Duration};

use cooee_core::{Config, secrets};
use cooee_server::Push;

const WAIT: Duration = Duration::from_secs(5);

fn account(data: &Path) -> String {
    let hash = cooee_core::sha256(data.as_os_str().as_encoded_bytes());
    format!("vapid-{}", &hash[..16])
}

fn read(data: &Path) -> Result<Option<String>, ()> {
    let a = account(data);
    let find = ["find-generic-password", "-s", "cooee", "-a", &a, "-w"];
    let Ok(mut child) = cooee_core::runner::command("security")
        .args(find)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return Ok(None);
    };
    let deadline = cooee_core::clock::instant() + WAIT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut key = String::new();
                let out = child.stdout.take().map(|mut o| o.read_to_string(&mut key));
                let found = status.success() && matches!(out, Some(Ok(_)));
                return Ok(found.then(|| key.trim().to_owned()));
            }
            Ok(None) if cooee_core::clock::instant() < deadline => {
                cooee_core::clock::sleep(Duration::from_millis(20));
            }
            _ => {
                child.kill().ok();
                child.wait().ok();
                return Err(());
            }
        }
    }
}

/// Copies the old key into `data`'s key file if there is no key file yet: what it did, if anything. A read that
/// times out makes a new key, and every subscribed device is asked to turn alerts on again.
///
/// # Errors
/// When the key or the subscriptions can't be stored.
pub fn migrate(data: &Path) -> Result<Option<String>, String> {
    let push = Push::new(data);
    if !cfg!(target_os = "macos") || push.has_key().map_err(|e| e.to_string())? {
        return Ok(None);
    }
    match read(data) {
        Ok(None) => Ok(None),
        Ok(Some(key)) if push.adopt(&key).map_err(|e| e.to_string())? => Ok(Some(
            "push key copied from the keychain to vapid.key".to_owned(),
        )),
        Ok(Some(_)) => renew(
            &push,
            "cooee made a new push key before the old one was copied: turn alerts on again",
        ),
        Err(()) => renew(
            &push,
            "the keychain didn't answer in 5 s, so cooee made a new push key: turn alerts on again",
        ),
    }
}

fn renew(push: &Push, why: &str) -> Result<Option<String>, String> {
    push.renew(why).map_err(|e| e.to_string())?;
    Ok(Some(why.to_owned()))
}

/// `cooee push migrate`: [`migrate`], then the command that deletes the old keychain item.
///
/// # Errors
/// When there is no data folder, or the key or the subscriptions can't be stored.
pub fn command(config: &Config) -> Result<(), String> {
    let data = &config
        .data_dir()
        .ok_or("no data folder: set `data` or HOME")?;
    if let Some(done) = migrate(data)? {
        cooee_core::say!("{done}");
    }
    let file = data.join("vapid.key");
    if Push::new(data).has_key().map_err(|e| e.to_string())? {
        cooee_core::say!("push key: {}", file.display());
    } else {
        cooee_core::say!("push key: none yet (made when a phone turns on notifications)");
    }
    if cfg!(target_os = "macos") {
        cooee_core::say!(
            "the old keychain item, if any, can go:\n  security delete-generic-password -s cooee -a {}",
            account(data)
        );
    }
    Ok(())
}

/// `cooee secrets`: [`secrets::migrate`], then reads each saved secret back; names only, never a value.
///
/// # Errors
/// When there is no data folder, a secret can't be moved, or one can't be read.
pub fn check(config: &Config) -> Result<(), String> {
    let data = config
        .data_dir()
        .ok_or("no data folder: set `data` or HOME")?;
    for name in secrets::migrate(&data).map_err(|e| format!("secrets: {e}"))? {
        cooee_core::say!("moved {name} into the keychain");
    }
    let mut failed = 0;
    for name in secrets::names(&data).map_err(|e| format!("secrets: {e}"))? {
        let why = match secrets::get(&data, &name) {
            Ok(Some(_)) => {
                cooee_core::say!("ok   {name}");
                continue;
            }
            Ok(None) => "gone".to_owned(),
            Err(e) => e.to_string(),
        };
        cooee_core::say!("FAIL {name}: {why}");
        failed += 1;
    }
    if failed == 0 {
        Ok(())
    } else {
        Err(format!("{failed} secret(s) can't be read"))
    }
}
