//! Connectors' sign-in (#497 slice D) through `cooee serve` against a fake sign-in server on localhost: the callback
//! answers without a paired cookie only for a state made by a sign-in and not yet used, and no token reaches the
//! config, the data folder (but the secret store) or the log.

mod support;

use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    path::Path,
    sync::{Arc, Mutex},
    thread,
};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, scratch, version};

#[derive(Default)]
struct Fake {
    challenge: String,
    revoked: Vec<String>,
}

fn pairs(s: &str) -> HashMap<String, String> {
    let decode = |v: &str| {
        let mut out = Vec::new();
        let mut b = v.bytes();
        while let Some(c) = b.next() {
            out.push(if c == b'%' {
                let h = [b.next().unwrap(), b.next().unwrap()];
                u8::from_str_radix(std::str::from_utf8(&h).unwrap(), 16).unwrap()
            } else {
                c
            });
        }
        String::from_utf8(out).unwrap()
    };
    s.split('&')
        .filter_map(|p| p.split_once('='))
        .map(|(k, v)| (decode(k), decode(v)))
        .collect()
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
            let form = pairs(&String::from_utf8(body).unwrap());
            let path = line.split(' ').nth(1).unwrap_or_default();
            let mut f = f.lock().unwrap();
            let v = match path {
                "/.well-known/oauth-protected-resource/mcp" => {
                    json!({ "resource": format!("{b}/mcp"), "authorization_servers": [b] })
                }
                "/.well-known/oauth-authorization-server" => json!({
                    "issuer": b, "authorization_endpoint": format!("{b}/authorize"),
                    "token_endpoint": format!("{b}/token"), "registration_endpoint": format!("{b}/register"),
                    "revocation_endpoint": format!("{b}/revoke"),
                }),
                "/register" => json!({ "client_id": "cooee-1" }),
                "/token"
                    if form["code"] == "code-1"
                        && cooee_core::connectors::challenge(&form["code_verifier"])
                            == f.challenge =>
                {
                    json!({ "access_token": "acc-q7q7q7q7", "refresh_token": "ref-q7q7q7q7", "expires_in": 3600 })
                }
                "/revoke" => {
                    f.revoked.push(form["token"].clone());
                    json!({})
                }
                _ => json!({ "error": "invalid_grant" }),
            }
            .to_string();
            write!(
                s.get_mut(),
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{v}",
                v.len()
            )
            .unwrap();
        }
    });
    (base, fake)
}

fn files(dir: &Path, skip: &Path, out: &mut Vec<(String, String)>) {
    for e in fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.starts_with(skip) {
            continue;
        }
        if p.is_dir() {
            files(&p, skip, out);
        } else if let Ok(text) = fs::read(&p) {
            out.push((
                p.display().to_string(),
                String::from_utf8_lossy(&text).into_owned(),
            ));
        }
    }
}

#[test]
fn the_callback_answers_only_its_own_state_and_no_token_leaks() {
    let dir = scratch("connectors");
    let (base, fake) = sign_in_server();
    let shed = FakeShed::start(&dir.join("shed"), version());
    let extra = format!(
        "[[mcp]]\nname = \"acme\"\ntype = \"http\"\nurl = \"{base}/mcp\"\nauth = \"oauth\"\nissuer = \"{base}\"\n"
    );
    let cooee = Cooee::start_with(&dir, &shed.dir, &extra);
    let open = |path: &str| {
        let (head, body) = cooee.raw("GET", path, &[], "");
        (head.lines().next().unwrap()[9..12].to_owned(), head, body)
    };
    assert_eq!(open("/connectors").0, "401", "only the callback is open");

    for _ in 0..2 {
        let (status, body) = cooee.post("/connectors", &json!({ "catalogue": "linear" }));
        assert_eq!(status, 200, "{body}");
        assert_eq!(
            serde_json::from_str::<Value>(&body).unwrap()["reviewed"],
            true
        );
    }
    let config = fs::read_to_string(&cooee.config).unwrap();
    assert_eq!(config.matches("catalogue = true").count(), 1, "{config}");

    let origin = format!("http://{}", cooee.addr);
    let (status, body) = cooee.post(
        "/connectors/acme/signin",
        &json!({ "account": "main", "origin": origin }),
    );
    assert_eq!(status, 200, "{body}");
    let started: Value = serde_json::from_str(&body).unwrap();
    let q = pairs(started["open"].as_str().unwrap().split_once('?').unwrap().1);
    fake.lock().unwrap().challenge = q["code_challenge"].clone();

    for bad in ["", "?code=code-1", "?state=made-up&code=code-1"] {
        let (status, _, body) = open(&format!("/connectors/callback{bad}"));
        assert_eq!(status, "400", "{bad}: {body}");
    }
    let back = format!("/connectors/callback?state={}&code=code-1", q["state"]);
    let (status, head, _) = open(&back);
    assert_eq!(status, "302", "{head}");
    let flow = started["flow"].as_str().unwrap();
    assert!(
        head.contains(&format!("location: cooee://connector?flow={flow}")),
        "{head}"
    );
    assert_eq!(open(&back).0, "400", "a used state");
    assert_the_flow_ended_and_no_token_leaks(&dir, &cooee, &fake, flow, config);
}

fn assert_the_flow_ended_and_no_token_leaks(
    dir: &Path,
    cooee: &Cooee,
    fake: &Mutex<Fake>,
    flow: &str,
    config: String,
) {
    let ended = cooee.json(&format!("/connectors/flows/{flow}"));
    let listed = cooee.json("/connectors");
    let acme = &listed["connectors"][0];
    if cfg!(target_os = "macos") {
        assert_eq!(ended["state"], "failed", "{ended}");
        assert!(
            ended["why"].as_str().unwrap().contains("isn't signed"),
            "{ended}"
        );
    } else {
        assert_eq!(ended["state"], "signed_in", "{ended}");
        assert_eq!(acme["accounts"][0]["state"], "signed_in", "{listed}");
    }

    let secrets = dir.join("data/secrets");
    let mut all = vec![("config".to_owned(), config)];
    all.push((
        "log".to_owned(),
        fs::read_to_string(dir.join("cooee.err")).unwrap(),
    ));
    files(&dir.join("data"), &secrets, &mut all);
    for (path, text) in &all {
        assert!(!text.contains("q7q7q7q7"), "a token in {path}");
    }

    if !cfg!(target_os = "macos") {
        let item = secrets.join("connector.acme.main");
        assert!(fs::read_to_string(&item).unwrap().contains("acc-q7q7q7q7"));
        let (status, body) = cooee.post("/connectors/acme/signout", &json!({ "account": "main" }));
        assert_eq!(status, 200, "{body}");
        assert!(body.contains("revoked"), "{body}");
        assert_eq!(fake.lock().unwrap().revoked, ["ref-q7q7q7q7"]);
        assert!(!item.exists());
    }
}

#[test]
fn the_phone_lists_adds_and_attaches_a_connector_and_the_menu_says_what_needs_sign_in() {
    let dir = scratch("connectors-phone");
    let shed = FakeShed::start(&dir.join("shed"), version());
    write_coder_and_linear_accounts(&dir);
    let cooee = Cooee::start(&dir, &shed.dir);
    let menu_row = || {
        let menu = cooee.json("/screens/menu");
        let rows = menu["blocks"].as_array().unwrap().iter();
        let mut rows = rows.flat_map(|g| g["rows"].as_array().cloned().unwrap_or_default());
        rows.find(|r| r["id"] == "connectors").unwrap()
    };
    assert_eq!(menu_row()["value"], "0");
    assert_notion_is_in_the_catalogue(&cooee);

    let (status, body) = cooee.post("/connectors", &json!({ "catalogue": "linear" }));
    assert_eq!(status, 200, "{body}");
    let row = menu_row();
    assert_eq!(
        (&row["value"], &row["warn"]),
        (&json!("1 needs sign-in"), &json!(true)),
        "{row}"
    );
    let rows = cooee.json("/screens/settings/connectors")["blocks"][0]["rows"].clone();
    assert_eq!(rows[0]["title"], "Linear");
    assert_eq!(rows[0]["subtitle"]["tone"], "warn", "{rows}");

    let mut coder = cooee.json("/personas/coder");
    coder["mcp"] = json!([{ "server": "linear", "account": "work" }]);
    let (status, body) = save_coder(&cooee, &coder);
    assert_eq!(status, 200, "{body}");
    let linear = cooee.json("/connectors")["connectors"][0].clone();
    assert_eq!(linear["source"], "catalogue");
    let used: Vec<_> = linear["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["personas"].clone())
        .collect();
    assert_eq!(used, [json!([]), json!(["Coder"])], "{linear}");

    let mut edited = cooee.json("/personas/coder");
    edited.as_object_mut().unwrap().remove("mcp");
    edited["character"] = json!("Terse.");
    assert_eq!(save_coder(&cooee, &edited).0, 200);
    assert_eq!(
        cooee.json("/personas/coder")["mcp"],
        json!([{ "server": "linear", "account": "work" }])
    );
    assert_a_removed_connectors_pick_stays_and_an_unknown_one_is_refused(&cooee, coder);
}

fn assert_notion_is_in_the_catalogue(cooee: &Cooee) {
    let listed = cooee.json("/connectors");
    let notion = &listed["catalogue"].as_array().unwrap();
    let notion = notion.iter().find(|e| e["name"] == "notion").unwrap();
    assert_eq!(notion["source"], "catalogue");
    assert_eq!(notion["host"], "mcp.notion.com", "{notion}");
    assert!(
        notion["checked"].as_str().unwrap().starts_with("2026-"),
        "{notion}"
    );
    assert!(notion["tools"]["read"].is_array(), "{notion}");
}

fn save_coder(cooee: &Cooee, values: &Value) -> (u16, String) {
    let act = json!({ "screen": "/screens/persona/coder", "block": "", "action": "save", "values": values });
    cooee.post("/screens/action", &act)
}

fn assert_a_removed_connectors_pick_stays_and_an_unknown_one_is_refused(
    cooee: &Cooee,
    mut coder: Value,
) {
    let (status, body) = cooee.request("DELETE", "/connectors/linear", None);
    assert_eq!(status, 200, "{body}");
    let mut kept = cooee.json("/personas/coder");
    kept["character"] = json!("Terse, kind.");
    assert_eq!(
        save_coder(cooee, &kept).0,
        200,
        "a pick of a removed connector stays and saves"
    );
    coder["version"] = json!("");
    coder["mcp"] = json!([{ "server": "nowhere", "account": "main" }]);
    let (status, body) = save_coder(cooee, &coder);
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("no connector nowhere"), "{body}");
    coder["mcp"] = json!([]);
    assert_eq!(save_coder(cooee, &coder).0, 200);
    assert!(cooee.json("/personas/coder").get("mcp").is_none());
}

fn write_coder_and_linear_accounts(dir: &Path) {
    fs::create_dir_all(dir.join("profiles")).unwrap();
    fs::write(
        dir.join("profiles/coder.toml"),
        "name = \"coder\"\nlabel = \"Coder\"\nagent = \"fake-acp\"\n",
    )
    .unwrap();
    let account = |name: &str, state: Value| {
        let mut a = json!({ "connector": "linear", "name": name, "issuer": "", "client": "", "scopes": [],
                            "since": 0 });
        a.as_object_mut()
            .unwrap()
            .extend(state.as_object().unwrap().clone());
        a
    };
    let accounts = [
        account(
            "main",
            json!({ "state": "needs_sign_in", "why": "the refresh was refused" }),
        ),
        account("work", json!({ "state": "signed_in" })),
    ];
    fs::create_dir_all(dir.join("data")).unwrap();
    let file = json!({ "format": 1, "clients": [], "accounts": accounts });
    fs::write(dir.join("data/connectors.json"), file.to_string()).unwrap();
}
