use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};

use serde_json::{Value, json};

use cooee_core::mcp::{AuthKind, Server};

use super::{Connectors, Vault};

fn at(data: &Path, name: &str) -> PathBuf {
    data.join("vault").join(name)
}

const FILES: Vault = Vault {
    get: |data, name| match fs::read_to_string(at(data, name)) {
        Ok(v) => Ok(Some(v)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    },
    set: |data, name, value| {
        fs::create_dir_all(data.join("vault"))?;
        fs::write(at(data, name), value)
    },
    remove: |data, name| match fs::remove_file(at(data, name)) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    },
};

#[derive(Default)]
struct Fake {
    codes: HashMap<String, (String, String)>,
    refresh_refused: bool,
    tokens: Vec<HashMap<String, String>>,
    revoked: Vec<String>,
    issued: usize,
}

fn decode(s: &str) -> String {
    let mut out = Vec::new();
    let mut b = s.bytes();
    while let Some(c) = b.next() {
        match c {
            b'%' => {
                let h: String = [b.next().unwrap(), b.next().unwrap()]
                    .iter()
                    .map(|&c| char::from(c))
                    .collect();
                out.push(u8::from_str_radix(&h, 16).unwrap());
            }
            b'+' => out.push(b' '),
            c => out.push(c),
        }
    }
    String::from_utf8(out).unwrap()
}

fn pairs(s: &str) -> HashMap<String, String> {
    s.split('&')
        .filter_map(|p| p.split_once('='))
        .map(|(k, v)| (decode(k), decode(v)))
        .collect()
}

fn answer(fake: &Mutex<Fake>, base: &str, line: &str, body: &str) -> (u16, Value) {
    let mut f = fake.lock().unwrap();
    let issue = |f: &mut Fake| {
        f.issued += 1;
        let n = f.issued;
        json!({ "access_token": format!("acc-{n}"), "refresh_token": format!("ref-{n}"), "expires_in": 3600 })
    };
    let form = pairs(body);
    match line.split(' ').take(2).collect::<Vec<_>>()[..] {
        ["GET", "/.well-known/oauth-protected-resource/mcp"] => (
            200,
            json!({ "resource": format!("{base}/mcp"), "authorization_servers": [base] }),
        ),
        ["GET", "/.well-known/oauth-authorization-server"] => (
            200,
            json!({ "issuer": base, "authorization_endpoint": format!("{base}/authorize"),
                    "token_endpoint": format!("{base}/token"), "registration_endpoint": format!("{base}/register"),
                    "revocation_endpoint": format!("{base}/revoke") }),
        ),
        ["POST", "/register"] => (201, json!({ "client_id": "cooee-1" })),
        ["POST", "/token"] => {
            f.tokens.push(form.clone());
            let ok = match form["grant_type"].as_str() {
                "authorization_code" => {
                    f.codes.remove(&form["code"]).is_some_and(|(ch, redirect)| {
                        cooee_core::connectors::challenge(&form["code_verifier"]) == ch
                            && form["redirect_uri"] == redirect
                            && form["client_id"] == "cooee-1"
                            && form["resource"] == format!("{base}/mcp")
                    })
                }
                _ => !f.refresh_refused && form["refresh_token"] == format!("ref-{}", f.issued),
            };
            if ok {
                (200, issue(&mut f))
            } else {
                (400, json!({ "error": "invalid_grant" }))
            }
        }
        ["POST", "/revoke"] => {
            f.revoked.push(form["token"].clone());
            (200, json!({}))
        }
        _ => (404, json!({})),
    }
}

fn sign_in_server() -> (String, Arc<Mutex<Fake>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let fake = Arc::new(Mutex::new(Fake::default()));
    let (b, f) = (base.clone(), Arc::clone(&fake));
    thread::spawn(move || {
        for s in listener.incoming() {
            let mut s = BufReader::new(s.unwrap());
            let (mut line, mut len) = (String::new(), 0);
            s.read_line(&mut line).unwrap();
            loop {
                let mut h = String::new();
                s.read_line(&mut h).unwrap();
                if let Some(n) = h.to_ascii_lowercase().strip_prefix("content-length:") {
                    len = n.trim().parse().unwrap();
                }
                if h.trim().is_empty() {
                    break;
                }
            }
            let mut body = vec![0; len];
            s.read_exact(&mut body).unwrap();
            let (status, v) = answer(&f, &b, &line, &String::from_utf8(body).unwrap());
            let v = v.to_string();
            write!(
                s.get_mut(),
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{v}",
                v.len()
            )
            .unwrap();
        }
    });
    (base, fake)
}

fn setup(
    name: &str,
    auth: Option<AuthKind>,
) -> (PathBuf, Connectors, Server, String, Arc<Mutex<Fake>>) {
    let data = cooee_core::scratch::owned(
        std::env::temp_dir().join(format!("cooee-connectors-{name}-{}", std::process::id())),
    );
    fs::remove_dir_all(&data).ok();
    fs::create_dir_all(&data).unwrap();
    let (base, fake) = sign_in_server();
    let issuer = if auth == Some(AuthKind::OAuth) {
        base.clone()
    } else {
        String::new()
    };
    let s = Server {
        name: "acme".to_owned(),
        kind: "http".to_owned(),
        url: format!("{base}/mcp"),
        auth,
        issuer,
        enabled: true,
        ..Server::default()
    };
    (data.clone(), Connectors::new(&data, FILES), s, base, fake)
}

const ORIGIN: (&str, &str) = ("http://127.0.0.1:4747", "127.0.0.1:4747");
const T0: u64 = 1_800_000_000_000;
const HOUR: u64 = 3_600_000;

fn begin(c: &Connectors, s: &Server, fake: &Mutex<Fake>, code: &str) -> String {
    let started = c.signin(s, "main", ORIGIN, T0).unwrap();
    let open = started["open"].as_str().unwrap();
    let q = pairs(open.split_once('?').unwrap().1);
    assert_eq!(
        (
            q["code_challenge_method"].as_str(),
            q["redirect_uri"].as_str(),
            started["until"].as_str()
        ),
        (
            "S256",
            "http://127.0.0.1:4747/connectors/callback",
            Some("cooee")
        )
    );
    let redirect = q["redirect_uri"].clone();
    fake.lock()
        .unwrap()
        .codes
        .insert(code.to_owned(), (q["code_challenge"].clone(), redirect));
    q["state"].clone()
}

#[test]
fn signs_in_with_pkce_refreshes_before_expiry_and_signs_out_with_a_revoke() {
    let (data, c, s, _, fake) = setup("signin", Some(AuthKind::OAuth));
    let state = begin(&c, &s, &fake, "code-1");
    let flow = c.callback(&state, Ok("code-1"), T0 + 1000).unwrap();
    assert_eq!(c.flow(&flow, T0 + 2000).unwrap()["state"], "signed_in");
    assert_eq!(
        c.callback(&state, Ok("code-1"), T0 + 3000),
        None,
        "a used state"
    );
    let file = fs::read_to_string(data.join("connectors.json")).unwrap();
    assert!(
        file.contains("\"signed_in\"") && !file.contains("acc-1") && !file.contains("ref-1"),
        "{file}"
    );
    assert!(
        fs::read_to_string(at(&data, "connector.acme.main"))
            .unwrap()
            .contains("acc-1")
    );

    let bearer = |n: &str| ("Authorization".to_owned(), format!("Bearer acc-{n}"));
    assert_eq!(
        c.credential(&s, "main", T0 + HOUR / 2).unwrap(),
        bearer("1")
    );
    assert_eq!(
        fake.lock().unwrap().tokens.len(),
        1,
        "no refresh while fresh"
    );
    assert_eq!(
        c.credential(&s, "main", T0 + HOUR - 30_000).unwrap(),
        bearer("2")
    );
    let last = fake.lock().unwrap().tokens.last().cloned().unwrap();
    assert_eq!(
        (last["grant_type"].as_str(), last["refresh_token"].as_str()),
        ("refresh_token", "ref-1")
    );
    assert_eq!(
        c.credential(&s, "main", T0 + HOUR).unwrap(),
        bearer("2"),
        "the new token holds"
    );

    let said = c.signout("acme", "main").unwrap();
    assert!(said.contains("revoked"), "{said}");
    assert_eq!(fake.lock().unwrap().revoked, ["ref-2"]);
    assert!(!at(&data, "connector.acme.main").exists());
    assert!(
        !fs::read_to_string(data.join("connectors.json"))
            .unwrap()
            .contains("acme")
    );
    assert!(
        c.credential(&s, "main", T0)
            .unwrap_err()
            .contains("needs sign-in")
    );
    assert!(
        c.signout("acme", "main")
            .unwrap()
            .contains("isn't signed in")
    );
}

#[test]
fn a_stale_unknown_or_unchecked_sign_in_is_refused_and_a_refused_refresh_needs_sign_in() {
    let (_data, c, s, _, fake) = setup("stale", Some(AuthKind::OAuth));
    let state = begin(&c, &s, &fake, "code-1");
    assert_eq!(
        c.callback(&state, Ok("code-1"), T0 + 11 * 60_000),
        None,
        "a stale state"
    );
    assert_eq!(
        c.callback(&state, Ok("code-1"), T0 + 1000),
        None,
        "ended by the stale try"
    );
    assert_eq!(c.callback("made-up", Ok("code-1"), T0), None);
    assert_eq!(c.callback("", Ok("code-1"), T0), None);
    assert!(fake.lock().unwrap().tokens.is_empty(), "nothing exchanged");

    let state = begin(&c, &s, &fake, "code-2");
    fake.lock().unwrap().codes.insert(
        "code-2".to_owned(),
        ("another-challenge".to_owned(), String::new()),
    );
    let flow = c.callback(&state, Ok("code-2"), T0).unwrap();
    let ended = c.flow(&flow, T0).unwrap();
    assert_eq!(
        (ended["state"].as_str(), ended["why"].as_str()),
        (
            Some("failed"),
            Some("the sign-in server refused (400): invalid_grant")
        )
    );

    let state = begin(&c, &s, &fake, "code-3");
    let denied = c.callback(&state, Err("access_denied"), T0).unwrap();
    assert_eq!(
        c.flow(&denied, T0).unwrap()["why"],
        "the service said: access_denied"
    );

    let state = begin(&c, &s, &fake, "code-4");
    c.callback(&state, Ok("code-4"), T0).unwrap();
    fake.lock().unwrap().refresh_refused = true;
    assert!(
        c.credential(&s, "main", T0 + HOUR)
            .unwrap_err()
            .contains("needs sign-in")
    );
    let list = c.list(&[s], &[]).unwrap();
    assert_eq!(
        list["connectors"][0]["accounts"][0]["state"],
        "needs_sign_in"
    );
}

#[test]
fn a_token_is_pasted_once_and_sent_in_its_header() {
    let (data, c, s, _, _) = setup("token", Some(AuthKind::Token));
    let flow = c
        .signin(&s, "main", ("https://evil.example", "x"), T0)
        .unwrap()["flow"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(c.paste("acme", &flow, "line\nbreak", T0).is_err());
    c.paste("acme", &flow, "tok-1", T0).unwrap();
    assert!(
        c.paste("acme", &flow, "tok-2", T0).is_err(),
        "a flow is used once"
    );
    assert_eq!(
        c.credential(&s, "main", T0).unwrap(),
        ("Authorization".to_owned(), "Bearer tok-1".to_owned())
    );
    assert!(
        !fs::read_to_string(data.join("connectors.json"))
            .unwrap()
            .contains("tok-1")
    );
    let oauth = Server {
        auth: Some(AuthKind::OAuth),
        issuer: "https://a.example".to_owned(),
        ..s
    };
    let e = c
        .signin(
            &oauth,
            "main",
            ("http://mac.tailnet.example", "mac.tailnet.example"),
            T0,
        )
        .unwrap_err();
    assert!(e.contains("tailscale serve"), "{e}");
    assert!(c.signin(&oauth, "../x", ORIGIN, T0).is_err());
}
