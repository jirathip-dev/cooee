//! The owner's terminal plugin (#28) in `cooee serve` over the fake shed: its tab is closed without the device's
//! passkey (403) and open with it; a shell starts as the owner, its screen updates stream through cooee's proxy,
//! typed text and keys arrive; an agent's terminal is read-only; "insert text" never presses Enter.

mod support;

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    time::Duration,
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use cooee_core::shed_proto::Hello;
use p256::ecdsa::{Signature, SigningKey, signature::Signer};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use support::{Cooee, FakeShed, scratch, version};

const TOKEN: &str = "terminal-token-0123456789";

const SPKI_P256: [u8; 26] = [
    0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08, 0x2a,
    0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
];

struct Mac {
    dir: PathBuf,
    shed: FakeShed,
    cooee: Cooee,
    backend: String,
}

fn mac(name: &str) -> Mac {
    mac_on(name, version())
}

fn mac_on(name: &str, shed: Hello) -> Mac {
    let dir = scratch(name);
    let shed = FakeShed::start(&dir.join("shed"), shed);
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let entry = format!(
        "[[plugins]]\nname = \"terminal\"\nlabel = \"Terminal\"\nurl = \"http://127.0.0.1:{port}\"\nmodule = \"/tab.js\"\ntoken = \"{TOKEN}\"\n"
    );
    let home = dir.join("home");
    fs::create_dir_all(&home).unwrap();
    let env = [("SHELL", "/bin/sh"), ("HOME", home.to_str().unwrap())];
    let cooee = Cooee::start_env(&dir, &shed.dir, &entry, &env);
    Mac {
        dir,
        shed,
        cooee,
        backend: format!("127.0.0.1:{port}"),
    }
}

fn key() -> SigningKey {
    SigningKey::from_slice(&[5; 32]).unwrap()
}

impl Mac {
    fn ceremony(&self, register: bool) -> (u16, String) {
        let (status, o) = self.cooee.post("/passkey/options", &json!({}));
        assert_eq!(status, 200, "{o}");
        let o: Value = serde_json::from_str(&o).unwrap();
        let kind = if register {
            "webauthn.create"
        } else {
            "webauthn.get"
        };
        let origin = format!("http://{}", self.cooee.addr);
        let client =
            json!({ "type": kind, "challenge": o["challenge"], "origin": origin }).to_string();
        let auth = [
            Sha256::digest(b"127.0.0.1").as_slice(),
            &[0b101, 0, 0, 0, 1],
        ]
        .concat();
        let mut proof = json!({ "id": B64.encode(b"passkey-1"), "client": B64.encode(&client), "auth": B64.encode(&auth) });
        if register {
            let point = key().verifying_key().to_sec1_point(false);
            proof["key"] = B64
                .encode([SPKI_P256.as_slice(), point.as_bytes()].concat())
                .into();
            self.cooee.post("/passkey/register", &proof)
        } else {
            let sig: Signature = key().sign(&[auth.as_slice(), &Sha256::digest(&client)].concat());
            proof["sig"] = B64.encode(sig.to_der().as_bytes()).into();
            self.cooee.post("/passkey/unlock", &proof)
        }
    }

    fn api(&self, path: &str, body: &Value) -> (u16, String) {
        self.cooee.post(&format!("/p/terminal/api{path}"), body)
    }

    fn list(&self) -> Value {
        self.cooee.json("/p/terminal/api/list")
    }

    fn log(&self) -> Vec<Value> {
        let text = fs::read_to_string(self.dir.join("data/terminal.log")).unwrap_or_default();
        text.lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    fn watch(&self, name: &str) -> impl Iterator<Item = Value> {
        let mut s = TcpStream::connect(&self.cooee.addr).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        write!(
            s,
            "GET /p/terminal/api/watch?t={name} HTTP/1.0\r\nHost: {}\r\nCookie: {}\r\n\r\n",
            self.cooee.addr, self.cooee.cookie
        )
        .unwrap();
        let mut lines = BufReader::new(s).lines();
        std::iter::from_fn(move || {
            loop {
                let line = lines.next()?.ok()?;
                let Some(data) = line.strip_prefix("data: ") else {
                    continue;
                };
                let e: Value = serde_json::from_str(data).unwrap();
                if e["event"] == "screen" {
                    return Some(serde_json::from_str(e["data"].as_str().unwrap()).unwrap());
                }
            }
        })
    }
}

fn texts(u: &Value) -> Vec<String> {
    u["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| {
            l["runs"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r["text"].as_str().unwrap())
                .collect()
        })
        .collect()
}

#[test]
fn no_passkey_is_403_a_passkey_unlocks_and_the_token_alone_is_not_enough() {
    let m = mac("term-gate");
    let (status, body) = m.cooee.get("/p/terminal/api/list");
    assert_eq!((status, body.as_str()), (403, "locked"));
    assert_eq!(m.ceremony(false).0, 403, "an unlock before any passkey");

    let mut s = TcpStream::connect(&m.backend).unwrap();
    write!(
        s,
        "GET /list HTTP/1.0\r\nAuthorization: Bearer {TOKEN}\r\n\r\n"
    )
    .unwrap();
    let mut status = String::new();
    BufReader::new(s).read_line(&mut status).unwrap();
    assert!(status.starts_with("HTTP/1.0 403"), "{status}");

    let (status, body) = m.ceremony(true);
    assert_eq!((status, body.as_str()), (200, "unlocked"));
    assert_eq!(m.list(), json!([]));
    let (status, body) = m.ceremony(true);
    assert_eq!(status, 403, "a second passkey: {body}");
    assert_eq!(m.ceremony(false), (200, "unlocked".to_owned()));
    let log = m.log();
    assert_eq!(log.len(), 2, "{log:?}");
    assert!(
        log.iter()
            .all(|l| l["event"] == "unlock" && l["device"].as_str().is_some_and(|d| d.len() == 8))
    );
}

#[test]
fn a_shed_of_another_protocol_fails_the_list_naming_both_sides() {
    let theirs = Hello {
        build: "fffffff".to_owned(),
        protocol: 2,
        ..version()
    };
    let m = mac_on("term-version", theirs);
    m.ceremony(true);
    let (status, body) = m.cooee.get("/p/terminal/api/list");
    assert_eq!(status, 502, "{body}");
    let (v, b) = (&version().version, &version().build);
    let said = format!(
        "SHED_VERSION: cooee speaks shed {v} ({b}) protocol 1; the server refused it: client {b} / server fffffff: protocol 1 vs 2"
    );
    assert!(body.contains(&said), "{body}\nwanted: {said}");
}

#[test]
fn a_shell_starts_its_screen_diffs_stream_and_typed_text_and_keys_arrive() {
    let m = mac("term-shell");
    m.ceremony(true);
    m.shed.lane("lane-7", &[("herdsman", "1")]);
    let (status, body) = m.api("/shell", &json!({}));
    assert_eq!(status, 200, "{body}");
    let name = serde_json::from_str::<Value>(&body).unwrap()["terminal"]
        .as_str()
        .unwrap()
        .to_owned();
    let run = &m.shed.calls("run", &name)[0];
    assert_eq!(run["cmd"], json!(["/bin/sh", "-l"]));
    assert_eq!(run["labels"], json!({ "cooee.owner": "1" }));
    assert_eq!(run["cwd"], m.dir.join("home").to_str().unwrap());
    let log = m.log();
    assert_eq!(
        (log[1]["event"].as_str(), log[1]["terminal"].as_str()),
        (Some("shell"), Some(name.as_str()))
    );

    let list = m.list();
    assert_eq!(list[0]["name"], name.as_str());
    assert_eq!(
        (&list[0]["owner"], &list[1]["name"], &list[1]["owner"]),
        (&json!(true), &json!("lane-7"), &json!(false))
    );

    let mut screen = m.watch(&name);
    let first = screen.next().unwrap();
    assert_eq!(
        (
            first["full"].as_bool(),
            first["lines"].as_array().unwrap().len()
        ),
        (Some(true), 24)
    );
    assert_eq!(
        m.api(
            "/input",
            &json!({ "terminal": name, "text": "echo ok สวัสดี" })
        )
        .0,
        200
    );
    assert_eq!(
        m.api("/keys", &json!({ "terminal": name, "keys": ["enter"] }))
            .0,
        200
    );
    let shown = screen
        .find(|u| texts(u).iter().any(|t| t == "ok สวัสดี"))
        .unwrap();
    assert_eq!(
        shown["full"], false,
        "a diff, not the whole screen: {shown}"
    );
    let line = shown["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["runs"][0]["text"] == "ok สวัสดี")
        .unwrap();
    assert_eq!(
        (&line["runs"][0]["fg"], &line["runs"][0]["bold"]),
        (&json!(2), &json!(true))
    );
    assert_eq!(
        m.shed.calls("terminal.input", &name)[0]["text"],
        "echo ok สวัสดี"
    );
    assert_eq!(
        m.shed.calls("terminal.keys", &name)[0]["keys"],
        json!(["enter"])
    );
    assert_eq!(
        m.api(
            "/resize",
            &json!({ "terminal": name, "cols": 44, "rows": 30 })
        )
        .0,
        200
    );
    assert_eq!(m.shed.calls("terminal.resize", &name)[0]["cols"], 44);

    assert_eq!(m.api("/close", &json!({ "terminal": name })).0, 200);
    assert_eq!(m.shed.calls("kill", &name).len(), 1);
    assert_eq!(
        m.list(),
        json!([{ "name": "lane-7", "owner": false, "cmd": "", "state": null }])
    );
}

#[test]
fn an_agents_terminal_can_be_watched_but_refuses_input() {
    let m = mac("term-agent");
    m.ceremony(true);
    m.shed.lane("lane-7", &[("herdsman", "1")]);
    let first = m.watch("lane-7").next().unwrap();
    assert_eq!(texts(&first)[0], "agent lane-7");
    for (path, body) in [
        (
            "/input",
            json!({ "terminal": "lane-7", "text": "rm -rf ~" }),
        ),
        ("/keys", json!({ "terminal": "lane-7", "keys": ["ctrl+c"] })),
        (
            "/resize",
            json!({ "terminal": "lane-7", "cols": 40, "rows": 20 }),
        ),
        ("/close", json!({ "terminal": "lane-7" })),
    ] {
        let (status, why) = m.api(path, &body);
        assert_eq!(
            (status, why.as_str()),
            (403, "Read-only: that is an agent's terminal."),
            "{path}"
        );
    }
    for method in ["terminal.input", "terminal.keys", "terminal.resize", "kill"] {
        assert_eq!(
            m.shed.calls(method, "lane-7"),
            Vec::<Value>::new(),
            "{method}"
        );
    }
}

#[test]
fn insert_text_types_into_the_shell_and_never_presses_enter() {
    let m = mac("term-insert");
    m.ceremony(true);
    let (status, body) = m.api("/insert", &json!({ "text": "cooee doctor\n" }));
    assert_eq!(status, 200, "{body}");
    let name = serde_json::from_str::<Value>(&body).unwrap()["terminal"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, body) = m.api("/insert", &json!({ "text": "ls \\\n  -la\r\ncd ~\n" }));
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap()["terminal"],
        name.as_str(),
        "the same shell"
    );
    let typed: Vec<Value> = m
        .shed
        .calls("terminal.input", &name)
        .into_iter()
        .map(|p| p["text"].clone())
        .collect();
    assert_eq!(typed, [json!("cooee doctor"), json!("ls   -la; cd ~")]);
    assert_eq!(
        m.shed.calls("terminal.keys", &name),
        Vec::<Value>::new(),
        "no Enter"
    );
}
