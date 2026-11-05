//! Pairing and the gate, end to end: unpaired reads and writes refused, pair → use → remove, only the token's hash
//! on disk, a foreign `Host` or `Origin` refused even with a paired cookie, and push subscriptions tied to a device.

mod support;

use std::fs;

use serde_json::{Value, json};
use support::{Cooee, FakeShed, scratch, version};

fn setup(name: &str, extra: &str) -> (std::path::PathBuf, FakeShed, Cooee) {
    let dir = scratch(name);
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start_with(&dir, &shed.dir, extra);
    (dir, shed, cooee)
}

fn status(head: &str) -> u16 {
    head[9..12].parse().unwrap()
}

/// Every page, read and write, with `cookie` (maybe none): its status.
fn statuses(c: &Cooee, cookie: &str) -> Vec<(&'static str, u16)> {
    let json = ("Content-Type", "application/json");
    let calls: [(&str, &str, &str); 9] = [
        ("GET", "/", ""),
        ("GET", "/t/t1", ""),
        ("GET", "/threads/t1/messages", ""),
        ("GET", "/threads/t1/events", ""),
        (
            "POST",
            "/threads",
            r#"{"agent":"a","folder":"/","text":"x","random_id":"r"}"#,
        ),
        (
            "POST",
            "/threads/t1/send",
            r#"{"text":"x","random_id":"r"}"#,
        ),
        (
            "POST",
            "/threads/t1/approve",
            r#"{"item":1,"option":"allow"}"#,
        ),
        ("GET", "/push/key", ""),
        (
            "POST",
            "/push/subscribe",
            r#"{"endpoint":"https://push.example.com/x","p256dh":"","auth":""}"#,
        ),
    ];
    calls
        .iter()
        .map(|(m, p, b)| {
            let (head, _) = c.raw(m, p, &[json, ("Cookie", cookie)], b);
            (*p, status(&head))
        })
        .collect()
}

#[test]
fn unpaired_reads_and_writes_are_refused() {
    let (_dir, _shed, c) = setup("unpaired", "");
    for cookie in ["", "cooee=", "cooee=0123456789abcdef"] {
        for (path, code) in statuses(&c, cookie) {
            // Pages send the browser to pairing; everything else is 401.
            let want = if path == "/" || path.starts_with("/t/") {
                303
            } else {
                401
            };
            assert_eq!(code, want, "{path} with {cookie:?}");
        }
    }
    let (head, _) = c.raw("GET", "/", &[], "");
    assert!(head.contains("location: /pair"), "{head}");
}

#[test]
fn pair_use_remove_and_only_the_hash_on_disk() {
    let (dir, _shed, c) = setup("pair", "");
    let cookie = c.cookie.clone();
    let (head, page) = c.raw("GET", "/", &[("Cookie", &cookie)], "");
    assert_eq!(status(&head), 200, "{head}");
    assert!(page.contains("<h1>cooee</h1>"));

    let token = cookie.strip_prefix("cooee=").unwrap();
    let disk = fs::read_to_string(dir.join("data/devices.json")).unwrap();
    assert!(!disk.contains(token), "{disk}");
    let devices: Value = serde_json::from_str(&disk).unwrap();
    let id = devices["devices"][0]["id"].as_str().unwrap().to_owned();
    assert!(c.cli(&["devices"]).contains(&id));

    // A code is single use: entering it again (here: a second pairing) is a new device.
    let second = c.pair().unwrap();
    assert_ne!(second, token);
    assert_eq!(c.cli(&["devices"]).lines().count(), 2);

    assert!(c.cli(&["devices", "rm", &id]).contains("removed"));
    for (path, code) in statuses(&c, &cookie) {
        assert!(code == 401 || code == 303, "{path}: {code}");
    }
    let (head, _) = c.raw("GET", "/", &[("Cookie", &format!("cooee={second}"))], "");
    assert_eq!(status(&head), 200, "the other device stays paired: {head}");
}

#[test]
fn a_wrong_code_does_not_pair() {
    let (_dir, _shed, c) = setup("wrong", "");
    let form = ("Content-Type", "application/x-www-form-urlencoded");
    let (head, _) = c.raw("POST", "/pair", &[form], "code=abcdef");
    assert!(
        head.contains("location: /pair?bad=1") && !head.contains("set-cookie"),
        "{head}"
    );
}

#[test]
fn a_foreign_host_or_origin_is_refused_even_paired() {
    let (_dir, _shed, c) = setup(
        "host",
        "allowed_hosts = [\"phone.tailnet.example\", \"*\"]\n",
    );
    let cookie = ("Cookie", c.cookie.as_str());
    let get =
        |headers: &[(&str, &str)]| status(&c.raw("GET", "/threads/t1/messages", headers, "").0);
    // Ours: the bind address (the default Host), localhost and an allowed name, with or without a port.
    assert_eq!(get(&[cookie]), 404);
    assert_eq!(get(&[cookie, ("Host", "localhost:4747")]), 404);
    assert_eq!(get(&[cookie, ("Host", "PHONE.tailnet.example")]), 404);
    assert_eq!(
        get(&[cookie, ("Origin", "https://phone.tailnet.example")]),
        404
    );
    // DNS rebinding: another name, even with a paired cookie; the `*` entry widens nothing.
    assert_eq!(get(&[cookie, ("Host", "evil.example")]), 403);
    assert_eq!(get(&[cookie, ("Host", "evil.example:4747")]), 403);
    assert_eq!(get(&[cookie, ("Origin", "http://evil.example")]), 403);
    assert_eq!(get(&[cookie, ("Origin", "null")]), 403);
    let (head, _) = c.raw("POST", "/pair", &[("Host", "evil.example")], "code=1");
    assert_eq!(status(&head), 403);
}

#[test]
fn push_subscriptions_are_checked_and_tied_to_the_device() {
    let (dir, _shed, c) = setup("push", "");
    let p256dh =
        "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4";
    let sub = |endpoint: &str| json!({ "endpoint": endpoint, "p256dh": p256dh, "auth": "BTBZMqHH6r4Tts7J_aSIgg" });
    let (code, body) = c.post("/push/subscribe", &sub("http://push.example.com/x"));
    assert_eq!(code, 400, "{body}");
    let (code, body) = c.post("/push/subscribe", &sub("https://push.example.com/x"));
    assert_eq!(code, 200, "{body}");
    let subs: Value =
        serde_json::from_str(&fs::read_to_string(dir.join("data/push.json")).unwrap()).unwrap();
    let devices: Value =
        serde_json::from_str(&fs::read_to_string(dir.join("data/devices.json")).unwrap()).unwrap();
    assert_eq!(subs[0]["device"], devices["devices"][0]["id"]);
    assert_eq!(subs[0]["endpoint"], "https://push.example.com/x");
}
