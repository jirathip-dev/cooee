//! The one read of the macOS keychain: before #99 cooee kept the VAPID key there (service `cooee`); now it lives only
//! in `<data>/vapid.key`. While there is no key file, a start copies the old key over, once.

use std::{
    io::Read,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use cooee_core::Config;
use cooee_server::Push;

/// Past this a keychain read gives up: a locked keychain can hang behind a prompt no one sees.
const WAIT: Duration = Duration::from_secs(5);

/// The keychain account of data folder `data`'s old key.
fn account(data: &Path) -> String {
    let hash = cooee_core::sha256(data.as_os_str().as_encoded_bytes());
    format!("vapid-{}", &hash[..16])
}

/// The old key, if the keychain has one; `Err` when the read took over [`WAIT`].
fn read(data: &Path) -> Result<Option<String>, ()> {
    let a = account(data);
    let find = ["find-generic-password", "-s", "cooee", "-a", &a, "-w"];
    let Ok(mut child) = Command::new("security")
        .args(find)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return Ok(None);
    };
    let deadline = Instant::now() + WAIT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut key = String::new();
                let out = child.stdout.take().map(|mut o| o.read_to_string(&mut key));
                let found = status.success() && matches!(out, Some(Ok(_)));
                return Ok(found.then(|| key.trim().to_owned()));
            }
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
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
        Ok(Some(key)) => {
            let kept = push.adopt(&key).map_err(|e| e.to_string())?;
            Ok(kept.then(|| "push key copied from the keychain to vapid.key".to_owned()))
        }
        Err(()) => {
            let why = "the keychain didn't answer in 5 s, so cooee made a new push key: turn alerts on again";
            push.renew(why).map_err(|e| e.to_string())?;
            Ok(Some(why.to_owned()))
        }
    }
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
        println!("{done}");
    }
    let file = data.join("vapid.key");
    if Push::new(data).has_key().map_err(|e| e.to_string())? {
        println!("push key: {}", file.display());
    } else {
        println!("push key: none yet (made when a phone turns on notifications)");
    }
    if cfg!(target_os = "macos") {
        println!(
            "the old keychain item, if any, can go:\n  security delete-generic-password -s cooee -a {}",
            account(data)
        );
    }
    Ok(())
}
