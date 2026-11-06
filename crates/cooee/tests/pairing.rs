//! Pairing and the gate, end to end: unpaired reads and writes refused, pair → use → remove, only the token's hash
//! on disk, a foreign `Host` or `Origin` refused even with a paired cookie, and push subscriptions tied to a device.

mod support;

use std::{fs, path::Path};

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
    // The shell (#21 row 7): the app's frame with the composer off and a pair hint, no thread data; the look loads.
    let (head, page) = c.raw("GET", "/pair", &[], "");
    assert_eq!(status(&head), 200);
    assert!(
        page.contains("isn't paired")
            && page.contains(r#"placeholder="pair this device to send" disabled"#),
        "{page}"
    );
    assert_eq!(status(&c.raw("GET", "/look.js", &[], "").0), 200);
    assert_eq!(
        status(&c.raw("GET", "/look/terminal/theme.json", &[], "").0),
        200
    );
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

/// `POST path` with `body` and no cookie: the status and headers, and the body.
fn open_post(c: &Cooee, path: &str, body: &Value) -> (String, String) {
    let json = ("Content-Type", "application/json");
    c.raw("POST", path, &[json], &body.to_string())
}

#[test]
fn a_phone_asks_for_a_code_and_the_mac_approves_it() {
    let (_dir, _shed, c) = setup("approve", "");
    let (head, body) = open_post(&c, "/pair/request", &json!({}));
    assert_eq!(status(&head), 200, "{head}");
    let r: Value = serde_json::from_str(&body).unwrap();
    let (code, claim) = (r["code"].as_str().unwrap(), json!({ "claim": r["claim"] }));
    assert_eq!(status(&open_post(&c, "/pair/claim", &claim).0), 202);

    // A code no phone shows is refused; the phone's own pairs it, once.
    let wrong = c.command().args(["pair", "000000x"]).output().unwrap();
    assert!(!wrong.status.success());
    assert!(c.cli(&["pair", code]).contains("approved"));
    let (head, _) = open_post(&c, "/pair/claim", &claim);
    assert_eq!(status(&head), 200, "{head}");
    let token = head
        .lines()
        .find_map(|l| l.strip_prefix("set-cookie: cooee="))
        .map(|c| c.split(';').next().unwrap().to_owned())
        .unwrap();
    let (head, _) = c.raw("GET", "/", &[("Cookie", &format!("cooee={token}"))], "");
    assert_eq!(status(&head), 200, "{head}");
    assert_eq!(status(&open_post(&c, "/pair/claim", &claim).0), 410);

    // Cancelled: gone.
    let (_, body) = open_post(&c, "/pair/request", &json!({}));
    let claim = json!({ "claim": serde_json::from_str::<Value>(&body).unwrap()["claim"] });
    assert_eq!(status(&open_post(&c, "/pair/cancel", &claim).0), 200);
    assert_eq!(status(&open_post(&c, "/pair/claim", &claim).0), 410);
}

#[test]
fn devices_are_listed_and_removed_from_the_phone() {
    let (_dir, _shed, c) = setup("devices-web", "");
    let other = c.pair().unwrap();
    let list = c.json("/devices");
    let list = list.as_array().unwrap();
    assert_eq!(list.len(), 2);
    let me: Vec<bool> = list.iter().map(|d| d["me"].as_bool().unwrap()).collect();
    assert_eq!(me, [true, false]);
    let (code, body) = c.post("/devices/remove", &json!({ "id": list[1]["id"] }));
    assert_eq!(code, 200, "{body}");
    let (head, _) = c.raw(
        "GET",
        "/devices",
        &[("Cookie", &format!("cooee={other}"))],
        "",
    );
    assert_eq!(status(&head), 401, "the removed device is unpaired");
    assert_eq!(c.json("/devices").as_array().unwrap().len(), 1);
    assert_eq!(c.post("/devices/remove", &json!({ "id": "nope" })).0, 404);
}

#[test]
fn alerts_status_test_and_off_are_this_devices() {
    let (_dir, _shed, c) = setup("alerts", "");
    let (code, body) = c.post("/push/test", &json!({}));
    assert_eq!(code, 400, "no subscription yet: {body}");
    let endpoint = "https://push.example.com/mine";
    let sub = json!({ "endpoint": endpoint, "p256dh": "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4", "auth": "BTBZMqHH6r4Tts7J_aSIgg" });
    assert_eq!(c.post("/push/subscribe", &sub).0, 200);
    assert_eq!(c.json("/push/endpoints"), json!([endpoint]));
    // Another device sees none of it and can't turn it off.
    let other = format!("cooee={}", c.pair().unwrap());
    let (_, body) = c.raw("GET", "/push/endpoints", &[("Cookie", &other)], "");
    assert_eq!(body, "[]");
    let off = json!({ "endpoint": endpoint }).to_string();
    let json_type = ("Content-Type", "application/json");
    c.raw(
        "POST",
        "/push/unsubscribe",
        &[json_type, ("Cookie", &other)],
        &off,
    );
    assert_eq!(c.json("/push/endpoints"), json!([endpoint]));
    assert_eq!(
        c.post("/push/unsubscribe", &json!({ "endpoint": endpoint }))
            .0,
        200
    );
    assert_eq!(c.json("/push/endpoints"), json!([]));
}

#[test]
fn a_report_is_kept_and_listed() {
    let (_dir, _shed, c) = setup("report", "");
    assert_eq!(c.post("/report", &json!({ "note": " " })).0, 400);
    assert!(c.cli(&["reports"]).contains("no reports"));
    let r = json!({ "note": "the toast runs off", "path": "/t/x", "target": "main > p#toast", "html": "<p>" });
    let (code, body) = c.post("/report", &r);
    assert_eq!(code, 200, "{body}");
    let out = c.cli(&["reports"]);
    assert!(
        out.contains("the toast runs off")
            && out.contains("main > p#toast")
            && out.contains("/t/x"),
        "{out}"
    );
}

#[test]
fn the_new_calls_need_a_paired_device_but_pairing_requests_do_not() {
    let (_dir, _shed, c) = setup("gate21", "");
    for path in ["/devices", "/push/endpoints"] {
        assert_eq!(status(&c.raw("GET", path, &[], "").0), 401, "{path}");
    }
    for path in [
        "/devices/remove",
        "/push/test",
        "/push/unsubscribe",
        "/report",
    ] {
        assert_eq!(
            status(
                &open_post(
                    &c,
                    path,
                    &json!({ "id": "x", "endpoint": "x", "note": "x" })
                )
                .0
            ),
            401,
            "{path}"
        );
    }
    assert_eq!(status(&open_post(&c, "/pair/request", &json!({})).0), 200);
    let foreign = [
        ("Host", "evil.example"),
        ("Content-Type", "application/json"),
    ];
    assert_eq!(
        status(&c.raw("POST", "/pair/request", &foreign, "{}").0),
        403
    );
}

#[test]
fn the_look_is_drawn_from_its_cookie_and_a_form_post_sets_it() {
    // #49: the look sheet with no script posts the key tapped; the cookie it sets draws the next page from its first byte.
    // kitchen, installed beside the config (#38: only plain is built in).
    let dir = scratch("look");
    let kitchen = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/themes/kitchen");
    fs::create_dir_all(dir.join("themes/kitchen")).unwrap();
    for f in ["theme.json", "sprites.svg", "icon.png"] {
        fs::copy(kitchen.join(f), dir.join("themes/kitchen").join(f)).unwrap();
    }
    let shed = FakeShed::start(&dir.join("shed"), version());
    let c = Cooee::start(&dir, &shed.dir);
    let form = "to=%2Fpair&style=terminal";
    let form_type = ("Content-Type", "application/x-www-form-urlencoded");
    let (head, _) = c.raw("POST", "/look", &[form_type], form);
    assert_eq!(status(&head), 303, "{head}");
    assert!(head.contains("location: /pair"), "{head}");
    let set = head
        .lines()
        .find_map(|l| l.strip_prefix("set-cookie: "))
        .unwrap();
    assert!(set.starts_with("cooee-look=style:terminal;"), "{set}");
    let pick = "cooee-look=theme:kitchen/style:terminal/sprites:off/mode:dark";
    let (_, page) = c.raw("GET", "/pair", &[("Cookie", pick)], "");
    assert!(
        page.contains(r#"<html lang="en" data-theme="kitchen" data-mode="dark" data-style="terminal" data-sprites="off""#),
        "{page}"
    );
    assert!(
        page.contains("<style id=\"scheme\">:root{") && page.contains("--bg:#1a1816;"),
        "{page}"
    );
    // The sheet is server HTML, its keys checked for the pick.
    assert!(
        page.contains(r#"data-k="style" data-v="terminal" aria-checked="true""#),
        "{page}"
    );
    // Back to this server only.
    let (head, _) = c.raw(
        "POST",
        "/look",
        &[form_type],
        "to=%2F%2Fevil.example&theme=farm",
    );
    assert!(
        head.contains("location: /\n") || head.ends_with("location: /"),
        "{head}"
    );
    // A browser drops the tab, so `/\t/evil.example` would become `//evil.example`.
    let (head, _) = c.raw(
        "POST",
        "/look",
        &[form_type],
        "to=%2F%09%2Fevil.example&theme=farm",
    );
    assert!(
        head.contains("location: /\n") || head.ends_with("location: /"),
        "{head}"
    );
}
