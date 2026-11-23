//! Pairing and the gate, end to end: unpaired reads and writes refused, pair → use → remove, only the token's hash
//! on disk, a foreign `Host` or `Origin` refused even with a paired cookie, and push subscriptions tied to a device;
//! #47: a QR code, refused codes and the rate limit, approving with a passkey (a fake authenticator and push), declining
//! and signing back in with a passkey.

mod support;

use std::fs;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use p256::{
    SecretKey,
    ecdsa::{Signature, SigningKey, signature::Signer},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
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

fn statuses(c: &Cooee, cookie: &str) -> Vec<(&'static str, u16)> {
    let json = ("Content-Type", "application/json");
    let calls: [(&str, &str, &str); 8] = [
        ("GET", "/threads", ""),
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
            assert_eq!(code, 401, "{path} with {cookie:?}");
        }
    }
}

#[test]
fn pair_use_remove_and_only_the_hash_on_disk() {
    let (dir, _shed, c) = setup("pair", "");
    let cookie = c.cookie.clone();
    let (head, _) = c.raw("GET", "/threads", &[("Cookie", &cookie)], "");
    assert_eq!(status(&head), 200, "{head}");

    let token = cookie.strip_prefix("cooee=").unwrap();
    let disk = fs::read_to_string(dir.join("data/devices.json")).unwrap();
    assert!(!disk.contains(token), "{disk}");
    let devices: Value = serde_json::from_str(&disk).unwrap();
    let id = devices["devices"][0]["id"].as_str().unwrap().to_owned();
    assert!(c.cli(&["devices"]).contains(&id));

    let second = c.pair().unwrap();
    assert_ne!(second, token);
    assert_eq!(c.cli(&["devices"]).lines().count(), 2);

    assert!(c.cli(&["devices", "rm", &id]).contains("removed"));
    for (path, code) in statuses(&c, &cookie) {
        assert_eq!(code, 401, "{path}");
    }
    let (head, _) = c.raw(
        "GET",
        "/threads",
        &[("Cookie", &format!("cooee={second}"))],
        "",
    );
    assert_eq!(status(&head), 200, "the other device stays paired: {head}");
}

const FORM: (&str, &str) = ("Content-Type", "application/x-www-form-urlencoded");

fn new_code(c: &Cooee) -> String {
    let out = c.cli(&["pair"]);
    let line = out.lines().find_map(|l| l.strip_prefix("code  ")).unwrap();
    line[..7].replace(' ', "")
}

fn enter(c: &Cooee, addr: &str, body: &str) -> (String, Option<String>) {
    let (head, _) = c.raw("POST", "/pair", &[FORM, ("X-Forwarded-For", addr)], body);
    assert_eq!(status(&head), 303, "{head}");
    let at = |k: &str| {
        head.lines()
            .find_map(|l| l.strip_prefix(k))
            .map(str::to_owned)
    };
    let cookie = at("set-cookie: cooee=").map(|c| c.split(';').next().unwrap().to_owned());
    (at("location: ").unwrap(), cookie)
}

#[test]
fn a_qr_codes_link_carries_a_code_that_pairs_at_once() {
    let (_dir, _shed, c) = setup("qr", "");
    let (code, body) = c.post("/devices/pair", &json!({}));
    assert_eq!(code, 200, "{body}");
    let qr: Value = serde_json::from_str(&body).unwrap();
    let pin = qr["code"].as_str().unwrap();
    let link = qr["link"].as_str().unwrap();
    assert_eq!(link, format!("https://{}/pair?code={pin}", c.addr));
    assert!(qr["svg"].as_str().unwrap().contains("<svg"), "{qr}");
    let six: Vec<String> = (2..=6)
        .zip(pin.chars().skip(1))
        .map(|(i, d)| format!("&d{i}={d}"))
        .collect();
    let six = six.concat();
    let (to, cookie) = enter(&c, "203.0.113.1", &format!("code={}{six}", &pin[..1]));
    assert_eq!(to, "/?paired");
    let cookie = format!("cooee={}", cookie.unwrap());
    assert_eq!(
        status(&c.raw("GET", "/threads", &[("Cookie", &cookie)], "").0),
        200
    );
}

#[test]
fn a_wrong_expired_or_reused_code_says_so() {
    let (dir, _shed, c) = setup("bad-code", "");
    let pin = new_code(&c);
    let wrong = if pin == "000000" { "111111" } else { "000000" };
    let (to, cookie) = enter(&c, "203.0.113.2", &format!("code={wrong}"));
    assert_eq!((to.as_str(), cookie), ("/pair?bad=wrong&left=4", None));
    assert!(enter(&c, "203.0.113.2", &format!("code={pin}")).1.is_some());
    let (to, cookie) = enter(&c, "203.0.113.2", &format!("code={pin}"));
    assert_eq!(
        (to.as_str(), cookie),
        (format!("/pair?bad=expired&code={pin}").as_str(), None)
    );
    let pin = new_code(&c);
    let file = dir.join("data/devices.json");
    let mut devices: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
    devices["code"][1] = json!(0);
    fs::write(&file, devices.to_string()).unwrap();
    assert_eq!(
        enter(&c, "203.0.113.2", &format!("code={pin}")).0,
        format!("/pair?bad=expired&code={pin}")
    );
}

#[test]
fn five_wrong_codes_a_minute_refuse_that_address() {
    let (_dir, _shed, c) = setup("rate", "");
    let pin = new_code(&c);
    let wrong = if pin == "000000" { "111111" } else { "000000" };
    for left in (1..5).rev() {
        let (to, _) = enter(&c, "203.0.113.3", &format!("code={wrong}"));
        assert_eq!(to, format!("/pair?bad=wrong&left={left}"));
    }
    assert_eq!(
        enter(&c, "203.0.113.3", &format!("code={wrong}")).0,
        "/pair?bad=limit&wait=60"
    );
    let pin = new_code(&c);
    let (to, cookie) = enter(&c, "203.0.113.3", &format!("code={pin}"));
    assert!(
        to.starts_with("/pair?bad=limit&wait=") && cookie.is_none(),
        "{to}"
    );
    assert!(enter(&c, "203.0.113.4", &format!("code={pin}")).1.is_some());
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
    assert_eq!(get(&[cookie]), 404);
    assert_eq!(get(&[cookie, ("Host", "localhost:4747")]), 404);
    assert_eq!(get(&[cookie, ("Host", "PHONE.tailnet.example")]), 404);
    assert_eq!(
        get(&[cookie, ("Origin", "https://phone.tailnet.example")]),
        404
    );
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

fn open_post(c: &Cooee, path: &str, body: &Value) -> (String, String) {
    let json = ("Content-Type", "application/json");
    c.raw("POST", path, &[json], &body.to_string())
}

struct Authenticator {
    key: SigningKey,
    id: String,
    host: String,
}

impl Authenticator {
    fn new(c: &Cooee, seed: u8) -> Self {
        Self {
            key: SigningKey::from(SecretKey::from_slice(&[seed; 32]).unwrap()),
            id: B64.encode([seed; 16]),
            host: c.addr.clone(),
        }
    }

    fn auth(&self) -> Vec<u8> {
        let mut a = Sha256::digest(self.host.split(':').next().unwrap()).to_vec();
        a.extend([0x05, 0, 0, 0, 1]);
        a
    }

    fn client(&self, kind: &str, challenge: &str) -> Vec<u8> {
        json!({ "type": kind, "challenge": challenge, "origin": format!("http://{}", self.host) })
            .to_string()
            .into_bytes()
    }

    fn sig(&self, auth: &[u8], client: &[u8]) -> String {
        let mut msg = auth.to_vec();
        msg.extend(Sha256::digest(client));
        let sig: Signature = self.key.sign(&msg);
        B64.encode(sig.to_der().as_bytes())
    }

    fn create(&self, challenge: &str) -> Value {
        let mut spki = vec![
            0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01,
        ];
        spki.extend([
            0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
        ]);
        spki.extend(self.key.verifying_key().to_sec1_point(false).as_bytes());
        let client = self.client("webauthn.create", challenge);
        json!({ "id": self.id, "key": B64.encode(spki), "client": B64.encode(client), "auth": B64.encode(self.auth()) })
    }

    fn get(&self, challenge: &str) -> Value {
        let (auth, client) = (self.auth(), self.client("webauthn.get", challenge));
        json!({ "id": self.id, "client": B64.encode(&client), "auth": B64.encode(&auth), "sig": self.sig(&auth, &client) })
    }
}

fn challenge(c: &Cooee) -> String {
    let (_, body) = open_post(c, "/passkey/challenge", &json!({}));
    serde_json::from_str::<Value>(&body).unwrap()["challenge"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn save_passkey(c: &Cooee, a: &Authenticator) {
    let (code, body) = c.post("/passkeys", &a.create(&challenge(c)));
    assert_eq!(code, 200, "{body}");
    assert_eq!(c.json("/passkeys")[0]["id"], a.id.as_str());
}

const IPAD: [(&str, &str); 2] = [
    ("Content-Type", "application/json"),
    ("User-Agent", "Mozilla/5.0 (iPad) Safari/604.1"),
];

fn ask(c: &Cooee) -> (String, Value) {
    let (head, body) = c.raw("POST", "/pair/request", &IPAD, "{}");
    assert_eq!(status(&head), 200, "{head}");
    let r: Value = serde_json::from_str(&body).unwrap();
    (
        r["word"].as_str().unwrap().to_owned(),
        json!({ "claim": r["claim"] }),
    )
}

fn cooee_pushing_through_a_fake_curl(
    dir: &std::path::Path,
) -> (FakeShed, Cooee, std::path::PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    let shed = FakeShed::start(&dir.join("shed"), version());
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let log = dir.join("pushed.log");
    let fakes = [(
        "curl",
        format!(
            "#!/bin/sh\nsed -n 's/^url = \"\\(.*\\)\"$/\\1/p' >>\"{}\"\nprintf 201\n",
            log.display()
        ),
    )];
    for (name, script) in fakes {
        fs::write(bin.join(name), script).unwrap();
        fs::set_permissions(bin.join(name), fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
    let c = Cooee::start_env(dir, &shed.dir, "", &[("PATH", path.as_str())]);
    (shed, c, log)
}

fn wait_for_push(log: &std::path::Path, endpoint: &str) {
    let at = std::time::Instant::now();
    while !fs::read_to_string(log)
        .unwrap_or_default()
        .contains(endpoint)
    {
        assert!(at.elapsed().as_secs() < 10, "no push");
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

fn only_the_paired_passkey_approves(c: &Cooee, phone: &Authenticator, id: &Value, claim: &Value) {
    let answer = |passkey: Value| {
        c.post(
            "/pair/answer",
            &json!({ "id": id, "yes": true, "passkey": passkey }),
        )
    };
    assert_eq!(answer(Value::Null).0, 400);
    let mut forged = phone.get(&challenge(c));
    forged["sig"] = json!(phone.sig(b"other", b"data"));
    assert_eq!(answer(forged).0, 403);
    assert_eq!(answer(phone.get("not-a-challenge-this-server-gave")).0, 403);
    let stranger = Authenticator::new(c, 9);
    let (code, body) = answer(stranger.get(&challenge(c)));
    assert_eq!(code, 403, "{body}");
    assert!(body.contains("No passkey for this cooee"), "{body}");
    assert_eq!(
        status(&open_post(c, "/pair/claim", claim).0),
        202,
        "still waiting"
    );
    let (code, body) = answer(phone.get(&challenge(c)));
    assert_eq!(code, 200, "{body}");
}

fn assert_the_ipad_is_paired(c: &Cooee, claim: &Value) {
    let (head, _) = c.raw("POST", "/pair/claim", &IPAD, &claim.to_string());
    let token = head
        .lines()
        .find_map(|l| l.strip_prefix("set-cookie: cooee="))
        .unwrap();
    let cookie = format!("cooee={}", token.split(';').next().unwrap());
    assert_eq!(
        status(&c.raw("GET", "/threads", &[("Cookie", &cookie)], "").0),
        200
    );
    assert_eq!(c.json("/pair/asks"), json!([]));
    assert!(c.cli(&["devices"]).contains("iPad"));
    let ipad = |path: &str| -> Value {
        serde_json::from_str(&c.raw("GET", path, &[("Cookie", &cookie)], "").1).unwrap()
    };
    let me = ipad("/devices");
    let me = me
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["me"] == true)
        .unwrap();
    assert_eq!(
        (me["label"].as_str(), me["by"].as_str()),
        (Some("iPad"), Some("browser"))
    );
    assert_eq!(ipad("/passkeys")[0]["mine"], false);
    assert_eq!(c.json("/passkeys")[0]["mine"], true);
}

#[test]
fn a_new_device_asks_and_a_paired_one_approves_it_with_its_passkey() {
    let dir = scratch("approve");
    let (_shed, c, log) = cooee_pushing_through_a_fake_curl(&dir);
    let endpoint = "https://push.example.com/paired-phone";
    let sub = json!({ "endpoint": endpoint, "p256dh": "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4", "auth": "BTBZMqHH6r4Tts7J_aSIgg" });
    assert_eq!(c.post("/push/subscribe", &sub).0, 200);
    let phone = Authenticator::new(&c, 7);
    save_passkey(&c, &phone);

    let (word, claim) = ask(&c);
    assert_eq!(word.split(' ').count(), 2, "{word}");
    assert_eq!(status(&open_post(&c, "/pair/claim", &claim).0), 202);
    wait_for_push(&log, endpoint);
    let asks = c.json("/pair/asks");
    assert_eq!(
        (asks[0]["word"].as_str(), asks[0]["label"].as_str()),
        (Some(word.as_str()), Some("iPad"))
    );
    let id = asks[0]["id"].clone();

    only_the_paired_passkey_approves(&c, &phone, &id, &claim);
    assert_the_ipad_is_paired(&c, &claim);
}

#[test]
fn a_declined_request_tells_the_new_device_who_said_no() {
    let (_dir, _shed, c) = setup("decline", "");
    let (_, claim) = ask(&c);
    let id = c.json("/pair/asks")[0]["id"].clone();
    let (code, body) = c.post("/pair/answer", &json!({ "id": id, "yes": false }));
    assert_eq!(code, 200, "{body}");
    let (head, body) = open_post(&c, "/pair/claim", &claim);
    assert_eq!(status(&head), 403, "{head}");
    assert_eq!(body, "browser said no.");
    assert!(!head.contains("set-cookie"));
    assert_eq!(c.cli(&["devices"]).lines().count(), 1, "not paired");
}

#[test]
fn the_mac_lists_and_approves_a_request_from_the_cli() {
    let (_dir, _shed, c) = setup("cli-approve", "");
    let (word, claim) = ask(&c);
    let asks: Value = serde_json::from_str(&c.cli(&["pair", "asks"])).unwrap();
    assert_eq!(asks, c.json("/pair/asks"));
    assert_eq!(asks[0]["word"].as_str(), Some(word.as_str()));
    let id = asks[0]["id"].as_str().unwrap();
    let out = c
        .command()
        .args(["pair", "approve", "ffffffff"])
        .output()
        .unwrap();
    assert!(!out.status.success(), "an unknown id is refused");
    assert!(String::from_utf8_lossy(&out.stderr).contains("no request ffffffff is waiting"));
    c.cli(&["pair", "approve", id]);
    let (head, _) = open_post(&c, "/pair/claim", &claim);
    assert_eq!(status(&head), 200, "{head}");
    assert!(head.contains("set-cookie: cooee="), "{head}");
    assert_eq!(c.cli(&["pair", "asks"]).trim(), "[]");
    let devices = c.json("/devices");
    let ipad = devices
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["label"] == "iPad")
        .unwrap();
    assert_eq!(ipad["by"], "this Mac");
}

#[test]
fn the_mac_declines_a_request_from_the_cli() {
    let (_dir, _shed, c) = setup("cli-decline", "");
    let (_, claim) = ask(&c);
    let asks: Value = serde_json::from_str(&c.cli(&["pair", "asks"])).unwrap();
    c.cli(&["pair", "decline", asks[0]["id"].as_str().unwrap()]);
    let (head, body) = open_post(&c, "/pair/claim", &claim);
    assert_eq!(status(&head), 403, "{head}");
    assert_eq!(body, "this Mac said no.");
}

#[test]
fn a_passkey_signs_a_device_back_in_and_can_be_removed() {
    let (_dir, _shed, c) = setup("passkey", "");
    let phone = Authenticator::new(&c, 7);
    save_passkey(&c, &phone);

    let signed = phone.get(&challenge(&c));
    let (head, _) = open_post(&c, "/passkey/signin", &signed);
    assert_eq!(status(&head), 200, "{head}");
    let token = head
        .lines()
        .find_map(|l| l.strip_prefix("set-cookie: cooee="))
        .unwrap();
    let cookie = format!("cooee={}", token.split(';').next().unwrap());
    assert_eq!(
        status(&c.raw("GET", "/threads", &[("Cookie", &cookie)], "").0),
        200
    );
    assert_ne!(c.json("/passkeys")[0]["used"], 0);
    let mine = |cookie: &str| {
        let (_, body) = c.raw("GET", "/passkeys", &[("Cookie", cookie)], "");
        serde_json::from_str::<Value>(&body).unwrap()[0]["mine"].clone()
    };
    assert_eq!(mine(&cookie), true);
    assert_eq!(c.json("/passkeys")[0]["mine"], false);
    assert_eq!(status(&open_post(&c, "/passkey/signin", &signed).0), 403);
    let (head, body) = open_post(
        &c,
        "/passkey/signin",
        &Authenticator::new(&c, 9).get(&challenge(&c)),
    );
    assert_eq!(status(&head), 403);
    assert!(
        body.contains("No passkey for this cooee on this device"),
        "{body}"
    );

    assert_eq!(
        c.post("/passkeys/remove", &json!({ "id": phone.id })).0,
        200
    );
    assert_eq!(c.json("/passkeys"), json!([]));
    assert_eq!(
        status(&open_post(&c, "/passkey/signin", &phone.get(&challenge(&c))).0),
        403
    );
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
    for path in ["/devices", "/push/endpoints", "/pair/asks", "/passkeys"] {
        assert_eq!(status(&c.raw("GET", path, &[], "").0), 401, "{path}");
    }
    for path in [
        "/devices/remove",
        "/push/test",
        "/push/unsubscribe",
        "/report",
        "/pair/answer",
        "/devices/pair",
        "/passkeys",
        "/passkeys/remove",
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

fn act(c: &Cooee, cookie: &str, addr: &str, action: &str, values: &Value) -> (String, Value) {
    let body = json!({ "screen": "/screens/pair", "action": action, "values": values });
    let headers = [
        ("Content-Type", "application/json"),
        ("Cookie", cookie),
        ("X-Forwarded-For", addr),
    ];
    let (head, body) = c.raw("POST", "/screens/pair/action", &headers, &body.to_string());
    assert_eq!(status(&head), 200, "{head}");
    (head, serde_json::from_str(&body).unwrap())
}

fn set(head: &str, name: &str) -> Option<String> {
    head.lines().find_map(|l| {
        let value = l.strip_prefix("set-cookie: ")?.strip_prefix(name)?;
        Some(value.strip_prefix('=')?.split(';').next()?.to_owned())
    })
}

fn pair_screen(c: &Cooee, cookie: &str, path: &str) -> Value {
    let (head, body) = c.raw("GET", path, &[("Cookie", cookie)], "");
    assert_eq!(status(&head), 200, "{head}");
    serde_json::from_str(&body).unwrap()
}

fn works(c: &Cooee, token: &str) -> bool {
    let cookie = format!("cooee={token}");
    status(&c.raw("GET", "/threads", &[("Cookie", &cookie)], "").0) == 200
}

fn assert_the_unpaired_screen_offers_a_code_an_ask_and_a_key(c: &Cooee) -> String {
    let screen = pair_screen(c, "", "/screens/pair?code=123456");
    let host = c.addr.split(':').next().unwrap();
    assert_eq!(
        (&screen["id"], &screen["title"]),
        (&json!("pair"), &json!(format!("Pair with {host}")))
    );
    let blocks = &screen["blocks"];
    assert_eq!(blocks[0]["fields"][0]["value"], "123456");
    assert_eq!(blocks[1]["action"], "ask");
    assert_eq!(
        (&blocks[2]["name"], &blocks[2]["props"]["mode"]),
        (&json!("device-key"), &json!("signin"))
    );
    assert!(blocks[2]["fallback"]["title"].is_string(), "{screen}");
    host.to_owned()
}

#[test]
fn a_typed_code_pairs_and_five_wrong_codes_lock_the_form_and_post_pair_together() {
    let (_dir, _shed, c) = setup("pair-screen-code", "");
    let host = assert_the_unpaired_screen_offers_a_code_an_ask_and_a_key(&c);

    let pin = new_code(&c);
    let wrong = json!({ "code": if pin == "000000" { "111111" } else { "000000" } });
    for left in (2..5).rev() {
        let (head, a) = act(&c, "", "203.0.113.9", "code", &wrong);
        assert_eq!(
            a["toast"]["title"],
            format!("That code is wrong: {left} tries left.")
        );
        assert_eq!(set(&head, "cooee"), None);
    }
    let (to, _) = enter(
        &c,
        "203.0.113.9",
        &format!("code={}", wrong["code"].as_str().unwrap()),
    );
    assert_eq!(
        to, "/pair?bad=wrong&left=1",
        "POST /pair counts the same misses"
    );
    let (_, a) = act(&c, "", "203.0.113.9", "code", &wrong);
    assert_eq!(a["toast"]["title"], "Too many tries: wait 60 s.");
    let (head, a) = act(&c, "", "203.0.113.9", "code", &json!({ "code": pin }));
    assert!(
        a["toast"]["title"]
            .as_str()
            .unwrap()
            .starts_with("Too many tries"),
        "{a}"
    );
    assert_eq!(
        set(&head, "cooee"),
        None,
        "a locked address can't pair even with the right code"
    );
    assert_eq!(
        enter(&c, "203.0.113.9", &format!("code={pin}"))
            .0
            .split('&')
            .next(),
        Some("/pair?bad=limit")
    );

    let pin = new_code(&c);
    let (head, a) = act(&c, "", "203.0.113.10", "code", &json!({ "code": pin }));
    assert_eq!(a, json!({ "open": "/screens/sessions" }));
    let token = set(&head, "cooee").unwrap();
    assert!(works(&c, &token));
    let paired = pair_screen(&c, &format!("cooee={token}"), "/screens/pair");
    assert_eq!(paired["blocks"][0]["text"], format!("Paired with {host}"));
    assert_eq!(paired["blocks"][1]["props"]["mode"], "enroll");
    let (head, a) = act(&c, "", "203.0.113.10", "code", &json!({ "code": pin }));
    assert_eq!(set(&head, "cooee"), None, "a code pairs once: {a}");
}

#[test]
fn an_asking_device_sees_its_word_then_approved_and_gets_its_cookie_only_when_it_collects() {
    let (_dir, _shed, c) = setup("pair-screen-word", "");
    let (head, a) = act(&c, "", "local", "ask", &json!({}));
    assert_eq!(a, json!({ "open": "/screens/pair" }));
    assert_eq!(set(&head, "cooee"), None);
    let claim = format!("cooee_claim={}", set(&head, "cooee_claim").unwrap());
    assert!(
        head.contains("Max-Age=300; HttpOnly; SameSite=Strict"),
        "{head}"
    );
    let screen = pair_screen(&c, &claim, "/screens/pair");
    let word = c.json("/pair/asks")[0]["word"].as_str().unwrap().to_owned();
    assert_eq!(screen["blocks"][0]["text"], format!("Check word: {word}"));
    let left = screen["blocks"][1]["text"].as_str().unwrap();
    assert!(
        left == "Runs out in 5:00" || left.starts_with("Runs out in 4:"),
        "{left}"
    );
    for path in ["/threads", "/screens/sessions", "/pair/asks"] {
        assert_eq!(
            status(&c.raw("GET", path, &[("Cookie", &claim)], "").0),
            401,
            "{path}"
        );
    }
    let (head, a) = act(&c, &claim, "local", "collect", &json!({}));
    assert_eq!(
        (a["toast"]["title"].as_str(), set(&head, "cooee")),
        (Some("Not approved yet."), None)
    );

    c.cli(&[
        "pair",
        "approve",
        c.json("/pair/asks")[0]["id"].as_str().unwrap(),
    ]);
    let screen = pair_screen(&c, &claim, "/screens/pair");
    assert_eq!(
        screen["blocks"][0]["text"],
        "this Mac approved this device."
    );
    assert_eq!(screen["blocks"][1]["action"], "collect");
    let (head, a) = act(&c, &claim, "local", "collect", &json!({}));
    assert_eq!(a, json!({ "open": "/screens/sessions" }));
    assert!(works(&c, &set(&head, "cooee").unwrap()));
    assert_eq!(
        set(&head, "cooee_claim").as_deref(),
        Some(""),
        "the claim cookie is cleared"
    );
    let (head, a) = act(&c, &claim, "local", "collect", &json!({}));
    assert_eq!(
        (a["toast"]["title"].as_str(), set(&head, "cooee")),
        (Some("The request ran out: ask again."), None)
    );
}

#[test]
fn a_declined_or_expired_request_never_gets_a_cookie() {
    let (dir, _shed, c) = setup("pair-screen-no", "");
    let (head, _) = act(&c, "", "local", "ask", &json!({}));
    let claim = format!("cooee_claim={}", set(&head, "cooee_claim").unwrap());
    c.cli(&[
        "pair",
        "decline",
        c.json("/pair/asks")[0]["id"].as_str().unwrap(),
    ]);
    let screen = pair_screen(&c, &claim, "/screens/pair");
    assert_eq!(screen["blocks"][0]["text"], "this Mac said no.");
    assert_eq!(screen["blocks"][1]["action"], "ask");
    let (head, a) = act(&c, &claim, "local", "collect", &json!({}));
    assert_eq!(
        (a["toast"]["title"].as_str(), set(&head, "cooee")),
        (Some("this Mac said no."), None)
    );

    let (head, _) = act(&c, "", "local", "ask", &json!({}));
    let claim = format!("cooee_claim={}", set(&head, "cooee_claim").unwrap());
    c.cli(&[
        "pair",
        "approve",
        c.json("/pair/asks")[0]["id"].as_str().unwrap(),
    ]);
    let file = dir.join("data/devices.json");
    let mut devices: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
    devices["asks"][0]["at"] = json!(0);
    fs::write(&file, devices.to_string()).unwrap();
    let screen = pair_screen(&c, &claim, "/screens/pair");
    assert_eq!(screen["blocks"][0]["text"], "The request ran out.");
    let (head, a) = act(&c, &claim, "local", "collect", &json!({}));
    assert_eq!(
        (a["toast"]["title"].as_str(), set(&head, "cooee")),
        (Some("The request ran out: ask again."), None)
    );
    let (head, _) = c.raw("GET", "/stream?v=2", &[("Cookie", &claim)], "");
    assert_eq!(status(&head), 401, "a run-out claim opens no stream");
}

#[test]
fn a_device_key_signs_in_and_a_wrong_signature_does_not() {
    let (_dir, _shed, c) = setup("pair-screen-key", "");
    let laptop = Authenticator::new(&c, 11);
    save_passkey(&c, &laptop);
    let mut forged = laptop.get(&challenge(&c));
    forged["sig"] = json!(laptop.sig(b"other", b"data"));
    let (head, a) = act(&c, "", "local", "signin", &forged);
    assert_eq!(
        (a["toast"]["title"].as_str(), set(&head, "cooee")),
        (Some("That passkey's signature doesn't check out."), None)
    );
    let (head, a) = act(
        &c,
        "",
        "local",
        "signin",
        &Authenticator::new(&c, 12).get(&challenge(&c)),
    );
    assert_eq!(set(&head, "cooee"), None, "an unknown key: {a}");
    let (head, a) = act(&c, "", "local", "signin", &json!({ "id": 1 }));
    assert_eq!(
        (a["toast"]["title"].as_str(), set(&head, "cooee")),
        (Some("That sign-in isn't readable."), None)
    );

    let (head, a) = act(&c, "", "local", "signin", &laptop.get(&challenge(&c)));
    assert_eq!(a, json!({ "open": "/screens/sessions" }));
    let token = set(&head, "cooee").unwrap();
    assert!(works(&c, &token));
    let (_, mine) = c.raw(
        "GET",
        "/passkeys",
        &[("Cookie", &format!("cooee={token}"))],
        "",
    );
    assert_eq!(
        serde_json::from_str::<Value>(&mine).unwrap()[0]["mine"],
        true,
        "the key is the new device's"
    );
}
