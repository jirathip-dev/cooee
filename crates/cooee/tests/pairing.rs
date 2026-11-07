//! Pairing and the gate, end to end: unpaired reads and writes refused, pair → use → remove, only the token's hash
//! on disk, a foreign `Host` or `Origin` refused even with a paired cookie, and push subscriptions tied to a device;
//! #47: a QR code, refused codes and the rate limit, approving with a passkey (a fake authenticator and push), declining
//! and signing back in with a passkey.

mod support;

use std::{fs, path::Path};

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
    // The shell only (q528): the welcome, three steps and six code boxes, no thread data; the look loads.
    let (head, page) = c.raw("GET", "/pair", &[], "");
    assert_eq!(status(&head), 200);
    assert!(
        page.contains("pair this device")
            && page.contains("Open cooee on your Mac or a paired phone")
            && page.matches("inputmode=\"numeric\"").count() == 6
            && !page.contains("sign in with passkey")
            && !page.contains("Add cooee to your Home Screen"),
        "{page}"
    );
    // Safari on an iPhone gets the Home Screen hint; iOS 26 says where Share is.
    let safari = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_6 like Mac OS X) Version/26.0 Mobile/15E148 Safari/604.1";
    let (_, page) = c.raw("GET", "/pair", &[("User-Agent", safari)], "");
    assert!(
        page.contains("Add cooee to your Home Screen first.") && page.contains("(in the ⋯ menu)"),
        "{page}"
    );
    let chrome = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_6 like Mac OS X) CriOS/140.0 Mobile/15E148 Safari/604.1";
    assert!(
        !c.raw("GET", "/pair", &[("User-Agent", chrome)], "")
            .1
            .contains("Home Screen first")
    );
    assert_eq!(status(&c.raw("GET", "/look.js", &[], "").0), 200);
    assert_eq!(
        status(&c.raw("GET", "/look/plain/theme.json", &[], "").0),
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

const FORM: (&str, &str) = ("Content-Type", "application/x-www-form-urlencoded");

/// The code `cooee pair` prints.
fn new_code(c: &Cooee) -> String {
    let out = c.cli(&["pair"]);
    let line = out.lines().find_map(|l| l.strip_prefix("code  ")).unwrap();
    line[..7].replace(' ', "")
}

/// Where a `POST /pair` of `body` from `addr` sends the browser, and the cookie it sets.
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
fn a_qr_code_is_prefilled_and_pairs_at_once() {
    let (_dir, _shed, c) = setup("qr", "");
    // A paired device shows the QR: the link with a new code, as an SVG.
    let (code, body) = c.post("/devices/pair", &json!({}));
    assert_eq!(code, 200, "{body}");
    let qr: Value = serde_json::from_str(&body).unwrap();
    let pin = qr["code"].as_str().unwrap();
    let link = qr["link"].as_str().unwrap();
    assert_eq!(link, format!("https://{}/pair?code={pin}", c.addr));
    assert!(qr["svg"].as_str().unwrap().contains("<svg"), "{qr}");
    // The Camera opens the link: the six boxes are filled in and say so; the page sends them itself.
    let (_, page) = c.raw("GET", &format!("/pair?code={pin}"), &[], "");
    assert!(
        page.contains(r#"class="cbx ok""#) && page.contains("pairing…"),
        "{page}"
    );
    for d in pin.chars() {
        assert!(page.contains(&format!(r#"value="{d}""#)), "{d}: {page}");
    }
    // The six boxes, as the form posts them: `code` then `d2`…`d6`.
    let six: Vec<String> = (2..=6)
        .zip(pin.chars().skip(1))
        .map(|(i, d)| format!("&d{i}={d}"))
        .collect();
    let six = six.concat();
    let (to, cookie) = enter(&c, "203.0.113.1", &format!("code={}{six}", &pin[..1]));
    assert_eq!(to, "/?paired");
    let cookie = format!("cooee={}", cookie.unwrap());
    assert_eq!(
        status(&c.raw("GET", "/", &[("Cookie", &cookie)], "").0),
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
    let (_, page) = c.raw("GET", &to, &[], "");
    assert!(
        page.contains("That code isn't right.") && page.contains("4 tries left this minute"),
        "{page}"
    );
    // Used once, it has run out.
    assert!(enter(&c, "203.0.113.2", &format!("code={pin}")).1.is_some());
    let (to, cookie) = enter(&c, "203.0.113.2", &format!("code={pin}"));
    assert_eq!((to.as_str(), cookie), ("/pair?bad=expired", None));
    let (_, page) = c.raw("GET", &to, &[], "");
    assert!(
        page.contains("That code has run out.") && page.contains("type a new code"),
        "{page}"
    );
    // After 10 minutes, too.
    let pin = new_code(&c);
    let file = dir.join("data/devices.json");
    let mut devices: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
    devices["code"][1] = json!(0);
    fs::write(&file, devices.to_string()).unwrap();
    assert_eq!(
        enter(&c, "203.0.113.2", &format!("code={pin}")).0,
        "/pair?bad=expired"
    );
}

#[test]
fn five_wrong_codes_a_minute_close_the_boxes_for_that_address() {
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
    // Even the right code is refused from there for the minute.
    let pin = new_code(&c);
    let (to, cookie) = enter(&c, "203.0.113.3", &format!("code={pin}"));
    assert!(
        to.starts_with("/pair?bad=limit&wait=") && cookie.is_none(),
        "{to}"
    );
    let (_, page) = c.raw("GET", &to, &[], "");
    assert!(
        page.contains("Too many tries.") && page.contains("disabled"),
        "{page}"
    );
    // Another address may still pair.
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

/// A fake platform authenticator: one P-256 passkey, signing as `WebAuthn` does for this server's origin.
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

    /// The authenticator data: the relying party's hash, then user present and verified (Face ID).
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

    /// `navigator.credentials.create` for `challenge`: the body `POST /passkeys` takes.
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

    /// `navigator.credentials.get` for `challenge`, signed.
    fn get(&self, challenge: &str) -> Value {
        let (auth, client) = (self.auth(), self.client("webauthn.get", challenge));
        json!({ "id": self.id, "client": B64.encode(&client), "auth": B64.encode(&auth), "sig": self.sig(&auth, &client) })
    }
}

/// A fresh challenge from `POST /passkey/challenge`, asked without a cookie.
fn challenge(c: &Cooee) -> String {
    let (_, body) = open_post(c, "/passkey/challenge", &json!({}));
    serde_json::from_str::<Value>(&body).unwrap()["challenge"]
        .as_str()
        .unwrap()
        .to_owned()
}

/// The paired device saves a passkey; it is listed.
fn save_passkey(c: &Cooee, a: &Authenticator) {
    let (code, body) = c.post("/passkeys", &a.create(&challenge(c)));
    assert_eq!(code, 200, "{body}");
    assert_eq!(c.json("/passkeys")[0]["id"], a.id.as_str());
}

/// An iPad's JSON request.
const IPAD: [(&str, &str); 2] = [
    ("Content-Type", "application/json"),
    ("User-Agent", "Mozilla/5.0 (iPad) Safari/604.1"),
];

/// A new device's request: its check word and claim.
fn ask(c: &Cooee) -> (String, Value) {
    let (head, body) = c.raw("POST", "/pair/request", &IPAD, "{}");
    assert_eq!(status(&head), 200, "{head}");
    let r: Value = serde_json::from_str(&body).unwrap();
    (
        r["word"].as_str().unwrap().to_owned(),
        json!({ "claim": r["claim"] }),
    )
}

#[test]
fn a_new_device_asks_and_a_paired_one_approves_it_with_its_passkey() {
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch("approve");
    let shed = FakeShed::start(&dir.join("shed"), version());
    // A fake push service: `curl` logs where it posts; a fake keychain holds the VAPID key.
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let log = dir.join("pushed.log");
    let fakes = [
        (
            "curl",
            format!(
                "#!/bin/sh\ncat >/dev/null\nfor a; do last=$a; done\necho \"$last\" >>{log:?}\nprintf 201\n"
            ),
        ),
        (
            "security",
            "#!/bin/sh\necho BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc\n".to_owned(),
        ),
    ];
    for (name, script) in fakes {
        fs::write(bin.join(name), script).unwrap();
        fs::set_permissions(bin.join(name), fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
    let c = Cooee::start_env(&dir, &shed.dir, "", &[("PATH", path.as_str())]);
    let endpoint = "https://push.example.com/paired-phone";
    let sub = json!({ "endpoint": endpoint, "p256dh": "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4", "auth": "BTBZMqHH6r4Tts7J_aSIgg" });
    assert_eq!(c.post("/push/subscribe", &sub).0, 200);
    let phone = Authenticator::new(&c, 7);
    save_passkey(&c, &phone);

    let (word, claim) = ask(&c);
    assert_eq!(word.split(' ').count(), 2, "{word}");
    assert_eq!(status(&open_post(&c, "/pair/claim", &claim).0), 202);
    // Every paired device gets the push, and the card.
    let at = std::time::Instant::now();
    while !fs::read_to_string(&log)
        .unwrap_or_default()
        .contains(endpoint)
    {
        assert!(at.elapsed().as_secs() < 10, "no push");
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let asks = c.json("/pair/asks");
    assert_eq!(
        (asks[0]["word"].as_str(), asks[0]["label"].as_str()),
        (Some(word.as_str()), Some("iPad"))
    );
    let id = asks[0]["id"].clone();

    // Approving needs a valid passkey signature over a fresh challenge.
    let answer = |passkey: Value| {
        c.post(
            "/pair/answer",
            &json!({ "id": id, "yes": true, "passkey": passkey }),
        )
    };
    assert_eq!(answer(Value::Null).0, 400);
    let mut forged = phone.get(&challenge(&c));
    forged["sig"] = json!(phone.sig(b"other", b"data"));
    assert_eq!(answer(forged).0, 403);
    assert_eq!(answer(phone.get("not-a-challenge-this-server-gave")).0, 403);
    let stranger = Authenticator::new(&c, 9);
    let (code, body) = answer(stranger.get(&challenge(&c)));
    assert_eq!(code, 403, "{body}");
    assert!(body.contains("No passkey for this cooee"), "{body}");
    assert_eq!(
        status(&open_post(&c, "/pair/claim", &claim).0),
        202,
        "still waiting"
    );
    let (code, body) = answer(phone.get(&challenge(&c)));
    assert_eq!(code, 200, "{body}");

    // The new device pairs by itself; the card clears.
    let (head, _) = c.raw("POST", "/pair/claim", &IPAD, &claim.to_string());
    let token = head
        .lines()
        .find_map(|l| l.strip_prefix("set-cookie: cooee="))
        .unwrap();
    let cookie = format!("cooee={}", token.split(';').next().unwrap());
    assert_eq!(
        status(&c.raw("GET", "/", &[("Cookie", &cookie)], "").0),
        200
    );
    assert_eq!(c.json("/pair/asks"), json!([]));
    assert!(c.cli(&["devices"]).contains("iPad"));
    // The devices list says who approved it; the passkey is the approver's, not the iPad's.
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
fn a_passkey_signs_a_device_back_in_and_can_be_removed() {
    let (_dir, _shed, c) = setup("passkey", "");
    let phone = Authenticator::new(&c, 7);
    save_passkey(&c, &phone);
    // The pairing page offers it once there is one.
    assert!(
        c.raw("GET", "/pair", &[], "")
            .1
            .contains("sign in with passkey")
    );

    // The cookie is gone (Safari's data cleared): the passkey pairs this browser again, with no code.
    let signed = phone.get(&challenge(&c));
    let (head, _) = open_post(&c, "/passkey/signin", &signed);
    assert_eq!(status(&head), 200, "{head}");
    let token = head
        .lines()
        .find_map(|l| l.strip_prefix("set-cookie: cooee="))
        .unwrap();
    let cookie = format!("cooee={}", token.split(';').next().unwrap());
    assert_eq!(
        status(&c.raw("GET", "/", &[("Cookie", &cookie)], "").0),
        200
    );
    assert_ne!(c.json("/passkeys")[0]["used"], 0);
    // The passkey is now the signed-in browser's (its approve card offers approving), no more the old cookie's.
    let mine = |cookie: &str| {
        let (_, body) = c.raw("GET", "/passkeys", &[("Cookie", cookie)], "");
        serde_json::from_str::<Value>(&body).unwrap()[0]["mine"].clone()
    };
    assert_eq!(mine(&cookie), true);
    assert_eq!(c.json("/passkeys")[0]["mine"], false);
    // A challenge is good once; another passkey is refused.
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

    // Removed in Settings › devices: it signs in no more.
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

#[test]
fn the_look_is_drawn_from_its_cookie_and_a_form_post_sets_it() {
    // #49: the look sheet with no script posts the key tapped; the cookie it sets draws the next page from its first byte.
    // kitchen, installed beside the config (#38: only plain is built in).
    let dir = scratch("look");
    let kitchen = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/themes/kitchen");
    fs::create_dir_all(dir.join("themes/kitchen")).unwrap();
    for f in ["theme.json", "sprites.svg", "scene.svg", "icon.png"] {
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
    let pick = "cooee-look=theme:kitchen/style:terminal/figures:none/mode:dark";
    let (_, page) = c.raw("GET", "/pair", &[("Cookie", pick)], "");
    assert!(
        page.contains(r#"<html lang="en" data-theme="kitchen" data-mode="dark" data-style="terminal" data-figures="none""#),
        "{page}"
    );
    // #35: Figures none hides Scene from Style (still in the page for `look.ts`, not listed); Sprite stays offered.
    assert!(
        page.contains(r#"<button hidden="" type="submit" role="radio" name="style" value="scene""#),
        "{page}"
    );
    assert!(
        page.contains(r#"<button type="submit" role="radio" name="figures" value="sprite""#),
        "{page}"
    );
    // A saved `terminal` theme is plain, which has no sprites: ASCII, and Sprite not offered.
    let (_, plain) = c.raw(
        "GET",
        "/pair",
        &[("Cookie", "cooee-look=theme:terminal")],
        "",
    );
    assert!(
        plain.contains(r#"data-theme="plain" data-style="modern" data-figures="ascii""#),
        "{plain}"
    );
    assert!(
        plain.contains(
            r#"<button hidden="" type="submit" role="radio" name="figures" value="sprite""#
        ),
        "{plain}"
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
