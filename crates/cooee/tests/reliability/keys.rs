#![cfg(target_os = "macos")]

use std::{
    fs,
    time::{Duration, Instant},
};

use serde_json::{Value, json};

use crate::{
    setup::{fake_security, until},
    support::{Cooee, FakeShed, scratch, version},
};

#[cfg(target_os = "macos")]
#[test]
fn r1_a_hung_keychain_falls_through_to_a_new_key_and_blocks_nothing() {
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch("r1-keychain");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let path = fake_security(&dir, "#!/bin/sh\nexec sleep 60\n");
    let at = Instant::now();
    let cooee = Cooee::start_env(&dir, &shed.dir, "", &[("PATH", path.as_str())]);
    let sub = json!({ "endpoint": "https://push.example.com/x", "p256dh": "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4", "auth": "BTBZMqHH6r4Tts7J_aSIgg" });
    assert_eq!(cooee.post("/push/subscribe", &sub).0, 200);
    assert_eq!(cooee.get("/devices").0, 200);
    assert_eq!(
        cooee.json("/push/endpoints"),
        json!(["https://push.example.com/x"])
    );
    assert!(at.elapsed() < Duration::from_secs(4), "{:?}", at.elapsed());
    until("the new key", || cooee.json("/push/reset").is_string());
    assert!(at.elapsed() < Duration::from_secs(8), "{:?}", at.elapsed());
    let why = cooee.json("/push/reset");
    assert!(
        why.as_str().unwrap().contains("didn't answer in 5 s"),
        "{why}"
    );
    assert_eq!(cooee.json("/push/endpoints"), json!([]));
    let key = dir.join("data/vapid.key");
    assert_eq!(
        fs::metadata(&key).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(cooee.get("/push/key").0, 200);
    let again = json!({ "endpoint": "https://push.example.com/y", "p256dh": sub["p256dh"], "auth": sub["auth"] });
    assert_eq!(cooee.post("/push/subscribe", &again).0, 200);
    assert_eq!(cooee.json("/push/reset"), Value::Null);
}

#[cfg(target_os = "macos")]
#[test]
fn an_old_keychain_key_is_copied_to_the_file_once() {
    let dir = scratch("keychain-migrate");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let calls = dir.join("security.log");
    let old = "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc";
    let path = fake_security(
        &dir,
        &format!("#!/bin/sh\necho \"$@\" >>{calls:?}\necho {old}\n"),
    );
    let key = dir.join("data/vapid.key");
    let first = Cooee::start_env(&dir, &shed.dir, "", &[("PATH", path.as_str())]);
    until("the copied key", || {
        fs::read_to_string(&key).is_ok_and(|k| k == old)
    });
    let public = first.json("/push/key");
    drop(first);
    let second = Cooee::start_env(&dir, &shed.dir, "", &[("PATH", path.as_str())]);
    assert_eq!(second.json("/push/key"), public);
    let out = second
        .command()
        .args(["push", "migrate"])
        .env("PATH", &path)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let out = String::from_utf8(out.stdout).unwrap();
    assert!(
        out.contains("security delete-generic-password -s cooee -a vapid-"),
        "{out}"
    );
    let log = fs::read_to_string(&calls).unwrap();
    assert_eq!(log.lines().count(), 1, "{log}");
    assert!(
        log.starts_with("find-generic-password -s cooee -a vapid-"),
        "{log}"
    );
    assert_eq!(fs::read_to_string(&key).unwrap(), old);
}

#[cfg(target_os = "macos")]
#[test]
fn a_key_made_during_the_keychain_read_wins_and_old_subscriptions_reset() {
    let dir = scratch("keychain-race");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let old = "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc";
    let gate = dir.join("gate");
    let script = format!("#!/bin/sh\nwhile [ ! -e {gate:?} ]; do sleep 0.02; done\necho {old}\n");
    let path = fake_security(&dir, &script);
    let cooee = Cooee::start_env(&dir, &shed.dir, "", &[("PATH", path.as_str())]);
    let sub = json!({ "endpoint": "https://push.example.com/x", "p256dh": "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4", "auth": "BTBZMqHH6r4Tts7J_aSIgg" });
    assert_eq!(cooee.post("/push/subscribe", &sub).0, 200);
    let public = cooee.json("/push/key");
    let key = dir.join("data/vapid.key");
    let made = fs::read_to_string(&key).unwrap();
    assert_ne!(made, old);
    fs::write(&gate, "").unwrap();
    until("the reset", || cooee.json("/push/reset").is_string());
    let why = cooee.json("/push/reset");
    assert!(
        why.as_str().unwrap().contains("before the old one"),
        "{why}"
    );
    assert_eq!(cooee.json("/push/key"), public);
    assert_eq!(fs::read_to_string(&key).unwrap(), made);
}
