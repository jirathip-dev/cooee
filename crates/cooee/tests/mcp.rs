//! MCP servers (#56) end to end with the fake shed (tests/support), whose CLI keeps the `--mcp` file a start hands
//! it, so each test plays the agent: it starts every server in that file as Claude's and Codex's ACP adapters do
//! (their own `initialize`, id styles and order) and talks MCP to it. A fake stdio server offers tools a, b and c.
//! The registry refuses a literal token; a profile's allowlist is all the agent sees and may call; references
//! resolve when the proxy starts and their values reach no file, log or event; a closed thread's token stops
//! working; `propose_server` only raises a card, which needs Face ID and adds the server off; the phone toggles and
//! picks tools but can't change a command or URL.

mod support;

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use p256::{
    SecretKey,
    ecdsa::{Signature, SigningKey, signature::Signer},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use support::{Cooee, FakeShed, scratch, version};

const SERVER: &str = r#"#!/bin/sh
: >"$(dirname "$0")/ran"
while IFS= read -r line; do
  id=$(printf '%s' "$line" | sed -n 's/.*"id":\([^,}]*\).*/\1/p')
  [ -n "$id" ] || continue
  case "$line" in
  *'"method":"initialize"'*) r='{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"fake","version":"1"}}' ;;
  *'"method":"tools/list"'*) r='{"tools":[{"name":"a","description":"tool a","inputSchema":{"type":"object"}},{"name":"b","description":"tool b","inputSchema":{"type":"object"}},{"name":"c","description":"tool c","inputSchema":{"type":"object"}}]}' ;;
  *'"method":"tools/call"'*)
    t=$(printf '%s' "$line" | sed -n 's/.*"name":"\([^"]*\)".*/\1/p')
    if [ -n "$TOKEN" ] && [ "$TOKEN" = "$WANT" ]; then k=matches; else k=differs; fi
    r="{\"content\":[{\"type\":\"text\",\"text\":\"$t ran; the token $k\"}]}" ;;
  *) r='{}' ;;
  esac
  printf '{"jsonrpc":"2.0","id":%s,"result":%s}\n' "$id" "$r"
done
"#;

struct Setup {
    dir: PathBuf,
    shed: FakeShed,
    cooee: Cooee,
    token: String,
}

fn setup(name: &str) -> Setup {
    let dir = scratch(name);
    let token = format!("tk-{}", "q7".repeat(16));
    let data = dir.join("data");
    fs::create_dir_all(data.join("secrets")).unwrap();
    fs::write(data.join("secrets/TOKEN"), format!("{token}\n")).unwrap();
    let server = dir.join("bin/fake-mcp");
    fs::create_dir_all(server.parent().unwrap()).unwrap();
    fs::write(&server, SERVER).unwrap();
    fs::set_permissions(&server, fs::Permissions::from_mode(0o755)).unwrap();
    fs::create_dir_all(dir.join("profiles")).unwrap();
    fs::write(
        dir.join("profiles/coder.toml"),
        "name = \"coder\"\nlabel = \"Coder\"\nagent = \"fake-acp\"\n\n[[mcp]]\nserver = \"fs\"\ntools = [\"a\"]\n",
    )
    .unwrap();
    let shed = FakeShed::start(&dir.join("shed"), version());
    let extra = format!(
        "[[mcp]]\nname = \"fs\"\ncommand = {:?}\nenv = {{ TOKEN = \"secret:TOKEN\", WANT = \"env:WANT_TOKEN\" }}\n",
        server.to_string_lossy()
    );
    let cooee = Cooee::start_env(&dir, &shed.dir, &extra, &[("WANT_TOKEN", &token)]);
    Setup {
        dir,
        shed,
        cooee,
        token,
    }
}

impl Setup {
    fn thread(&self, profile: &str) -> (String, Vec<Value>) {
        let random = format!("start-{}", self.shed.cli_log().len());
        let new = json!({ "agent": "fake-acp", "profile": profile, "folder": self.dir.join("root"),
                          "text": "hi", "random_id": random });
        let (status, body) = self.cooee.post("/threads", &new);
        assert_eq!(status, 200, "{body}");
        let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let file = self.shed.dir.join(format!("mcp-cooee-{id}.json"));
        let servers: Value = serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap();
        (id, servers.as_array().unwrap().clone())
    }

    fn ls(&self) -> String {
        self.cooee.cli(&["mcp", "ls"])
    }

    fn form(&self, path: &str, body: &str) -> (u16, String) {
        let headers = [
            ("Content-Type", "application/x-www-form-urlencoded"),
            ("Cookie", self.cooee.cookie.as_str()),
        ];
        let (head, body) = self.cooee.raw("POST", path, &headers, body);
        (head[9..12].parse().unwrap(), body)
    }
}

#[derive(Clone, Copy, Debug)]
enum Agent {
    Claude,
    Codex,
}

struct Mcp {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    agent: Agent,
    next: u64,
}

impl Mcp {
    fn start(entry: &Value, agent: Agent) -> Self {
        let mut c = Command::new(entry["command"].as_str().unwrap());
        c.args(
            entry["args"]
                .as_array()
                .unwrap()
                .iter()
                .map(|a| a.as_str().unwrap()),
        );
        for v in entry["env"].as_array().unwrap() {
            c.env(v["name"].as_str().unwrap(), v["value"].as_str().unwrap());
        }
        let mut child = c
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let (stdin, stdout) = (child.stdin.take().unwrap(), child.stdout.take().unwrap());
        let mut m = Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            agent,
            next: 0,
        };
        let (version, client) = match agent {
            Agent::Claude => ("2025-06-18", "claude-code"),
            Agent::Codex => ("2025-03-26", "codex-mcp-client"),
        };
        let hello = json!({ "protocolVersion": version, "capabilities": {}, "clientInfo": { "name": client, "version": "1" } });
        m.call("initialize", &hello).expect("initialize");
        writeln!(
            m.stdin,
            r#"{{"jsonrpc":"2.0","method":"notifications/initialized"}}"#
        )
        .unwrap();
        m
    }

    fn call(&mut self, method: &str, params: &Value) -> Result<Value, String> {
        let id = match self.agent {
            Agent::Claude => json!(self.next),
            Agent::Codex => json!(format!("codex-{}", self.next)),
        };
        self.next += 1;
        let msg = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        writeln!(self.stdin, "{msg}").unwrap();
        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();
        let answer: Value =
            serde_json::from_str(&line).unwrap_or_else(|_| panic!("{method}: {line:?}"));
        assert_eq!(answer["id"], id, "{answer}");
        match answer.get("error") {
            Some(e) => Err(e["message"].as_str().unwrap().to_owned()),
            None => Ok(answer["result"].clone()),
        }
    }

    fn tools(&mut self) -> Vec<String> {
        let params = match self.agent {
            Agent::Claude => json!({}),
            Agent::Codex => json!({ "cursor": null }),
        };
        let listed = self.call("tools/list", &params).unwrap();
        let names = listed["tools"].as_array().unwrap().iter();
        names
            .map(|t| t["name"].as_str().unwrap().to_owned())
            .collect()
    }

    fn run(&mut self, tool: &str, arguments: &Value) -> Result<String, String> {
        let r = self.call(
            "tools/call",
            &json!({ "name": tool, "arguments": arguments }),
        )?;
        Ok(r["content"][0]["text"].as_str().unwrap().to_owned())
    }
}

impl Drop for Mcp {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn named<'a>(servers: &'a [Value], name: &str) -> &'a Value {
    servers
        .iter()
        .find(|s| s["name"] == name)
        .unwrap_or_else(|| panic!("no {name} in {servers:?}"))
}

fn files(dir: &Path, out: &mut String) {
    for e in fs::read_dir(dir).into_iter().flatten().flatten() {
        if e.file_name() == "secrets" {
            continue;
        }
        if e.file_type().unwrap().is_dir() {
            files(&e.path(), out);
        } else if e.file_type().unwrap().is_file() {
            out.push_str(&String::from_utf8_lossy(&fs::read(e.path()).unwrap()));
        }
    }
}

#[test]
fn a_literal_token_in_the_registry_is_refused_at_load_naming_the_key_only() {
    let dir = scratch("mcp-literal");
    let config = dir.join("config.toml");
    let token = format!("ghp_{}", "z9".repeat(14));
    let refused = |text: &str| {
        fs::write(&config, text).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_cooee"))
            .arg("--config")
            .arg(&config)
            .args(["mcp", "ls"])
            .output()
            .unwrap();
        assert!(!out.status.success());
        String::from_utf8(out.stderr).unwrap()
    };
    let e = refused(&format!(
        "[[mcp]]\nname = \"gh\"\ncommand = \"gh-mcp\"\nenv = {{ GH_TOKEN = \"{token}\" }}\n"
    ));
    assert!(
        e.contains("mcp gh: env GH_TOKEN: only a reference") && !e.contains(&token),
        "{e}"
    );
    let e = refused(&format!(
        "[[mcp]]\nname = \"gh\"\ncommand = \"gh-mcp\"\nargs = [\"--token\", \"{token}\"]\n"
    ));
    assert!(
        e.contains("looks like a token") && !e.contains(&token),
        "{e}"
    );
    let e = refused(&format!(
        "[[mcp]]\nname = \"w\"\ntype = \"http\"\nurl = \"https://x/mcp\"\nheaders = {{ Authorization = \"Bearer {token}\" }}\n"
    ));
    assert!(
        e.contains("headers Authorization: only a reference") && !e.contains(&token),
        "{e}"
    );
}

#[test]
fn the_agent_sees_and_calls_only_the_allowed_tools_under_claude_and_codex_and_no_value_is_written()
{
    let s = setup("mcp-proxy");
    let (id, servers) = s.thread("coder");
    let names: Vec<&str> = servers
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["cooee", "fs"]);
    let fs_entry = named(&servers, "fs");
    assert_eq!(fs_entry["command"], env!("CARGO_BIN_EXE_cooee"));
    assert_eq!(
        fs_entry["args"],
        json!(["mcp", "proxy", "fs", "--thread", id])
    );
    for agent in [Agent::Claude, Agent::Codex] {
        let mut m = Mcp::start(fs_entry, agent);
        assert_eq!(m.tools(), ["a"], "{agent:?}");
        assert_eq!(
            m.run("a", &json!({})),
            Ok("a ran; the token matches".to_owned()),
            "{agent:?}"
        );
        let refused = m.run("b", &json!({})).unwrap_err();
        assert!(
            refused.contains("tool b isn't allowed"),
            "{agent:?}: {refused}"
        );
        assert!(m.run("c", &json!({})).is_err(), "{agent:?}");
    }

    let (plain, servers) = s.thread("");
    assert_eq!(servers.len(), 1, "{servers:?}");
    let proxy = |var: &Value| {
        let out = Command::new(env!("CARGO_BIN_EXE_cooee"))
            .args(["mcp", "proxy", "fs", "--thread", &plain])
            .env("COOEE_MCP", var["env"][0]["value"].as_str().unwrap())
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(!out.status.success());
        String::from_utf8(out.stderr).unwrap()
    };
    let e = proxy(&servers[0]);
    assert!(
        e.contains(&format!(
            "thread {plain}'s profile doesn't pick MCP server fs"
        )),
        "{e}"
    );
    let e = proxy(fs_entry);
    assert!(
        e.contains(&format!("this MCP token isn't thread {plain}'s")),
        "{e}"
    );

    let mut m = Mcp::start(fs_entry, Agent::Claude);
    assert!(m.run("a", &json!({})).is_ok());
    assert_eq!(
        s.cooee.post(&format!("/threads/{id}/close"), &json!({})).0,
        200
    );
    let e = m.run("a", &json!({})).unwrap_err();
    assert!(e.contains("closed"), "{e}");
    drop(m);

    let mut all = fs::read_to_string(&s.cooee.config).unwrap();
    files(&s.dir.join("data"), &mut all);
    files(&s.shed.dir, &mut all);
    all.push_str(&fs::read_to_string(s.dir.join("cooee.err")).unwrap());
    all.push_str(&serde_json::to_string(&*s.shed.requests.lock().unwrap()).unwrap());
    all.push_str(&s.cooee.get(&format!("/threads/{id}/messages")).1);
    assert!(all.contains("secret:TOKEN"), "the scan reads the config");
    assert!(!all.contains(&s.token), "a value was written");
    let left: Vec<_> = fs::read_dir(s.dir.join("data/secrets"))
        .unwrap()
        .flatten()
        .map(|e| e.file_name())
        .collect();
    assert_eq!(left, ["TOKEN"], "the --mcp file is gone");
}

struct Authenticator {
    key: SigningKey,
    id: String,
    host: String,
}

impl Authenticator {
    fn new(c: &Cooee) -> Self {
        Self {
            key: SigningKey::from(SecretKey::from_slice(&[7; 32]).unwrap()),
            id: B64.encode([7; 16]),
            host: c.addr.clone(),
        }
    }

    fn signed(&self, kind: &str, c: &Cooee) -> (Vec<u8>, Vec<u8>, String) {
        let (_, body) = c.raw(
            "POST",
            "/passkey/challenge",
            &[("Content-Type", "application/json")],
            "{}",
        );
        let challenge = serde_json::from_str::<Value>(&body).unwrap()["challenge"].clone();
        let client = json!({ "type": kind, "challenge": challenge, "origin": format!("http://{}", self.host) });
        let client = client.to_string().into_bytes();
        let mut auth = Sha256::digest(self.host.split(':').next().unwrap()).to_vec();
        auth.extend([0x05, 0, 0, 0, 1]);
        let mut msg = auth.clone();
        msg.extend(Sha256::digest(&client));
        let sig: Signature = self.key.sign(&msg);
        (auth, client, B64.encode(sig.to_der().as_bytes()))
    }

    fn save(&self, c: &Cooee) {
        let (auth, client, _) = self.signed("webauthn.create", c);
        let mut spki = vec![
            0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01,
        ];
        spki.extend([
            0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
        ]);
        spki.extend(self.key.verifying_key().to_sec1_point(false).as_bytes());
        let key = json!({ "id": self.id, "key": B64.encode(spki), "client": B64.encode(client), "auth": B64.encode(auth) });
        assert_eq!(c.post("/passkeys", &key).0, 200);
    }

    fn get(&self, c: &Cooee) -> Value {
        let (auth, client, sig) = self.signed("webauthn.get", c);
        json!({ "id": self.id, "client": B64.encode(client), "auth": B64.encode(auth), "sig": sig })
    }
}

#[test]
fn propose_server_shows_the_exact_command_runs_nothing_and_needs_face_id_to_add_it_off() {
    let s = setup("mcp-propose");
    let (id, servers) = s.thread("");
    let ran = s.dir.join("bin/ran");
    let web = s.dir.join("bin/fake-mcp");
    let mut cooee = Mcp::start(named(&servers, "cooee"), Agent::Codex);
    assert_eq!(cooee.tools(), ["propose_server", "remember", "ask_persona"]);
    let proposal = json!({ "name": "web", "type": "stdio", "command": web, "args": ["--port", "9 9"], "why": "browse the docs" });
    let said = cooee.run("propose_server", &proposal).unwrap();
    assert!(
        said.starts_with("Proposed.") && said.contains("Nothing has run"),
        "{said}"
    );
    let bad = cooee.call("tools/call", &json!({ "name": "propose_server", "arguments": { "name": "fs", "type": "stdio", "command": "x" } }));
    assert!(
        bad.unwrap()["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("already")
    );

    let items = s.cooee.json(&format!("/threads/{id}/messages"));
    let card = items["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["form"] == "mcp")
        .expect("the card")
        .clone();
    let exact = format!("{} --port '9 9'", web.display());
    assert_eq!(card["detail"], exact.as_str());
    assert!(
        card["tool"]
            .as_str()
            .unwrap()
            .contains("add MCP server web (stdio): browse the docs"),
        "{card}"
    );
    assert!(
        !ran.exists() && !s.ls().contains("web"),
        "nothing runs before approval"
    );

    let approve = |passkey: Value| {
        s.cooee.post(
            &format!("/threads/{id}/approve"),
            &json!({ "item": card["id"], "option": "approve", "passkey": passkey }),
        )
    };
    let (status, body) = approve(Value::Null);
    assert_eq!(
        (status, body.as_str()),
        (403, "Approving an MCP server needs Face ID.")
    );
    let phone = Authenticator::new(&s.cooee);
    assert_eq!(approve(phone.get(&s.cooee)).0, 403, "an unknown passkey");
    assert!(!s.ls().contains("web"));
    phone.save(&s.cooee);
    let (status, body) = approve(phone.get(&s.cooee));
    assert_eq!(status, 200, "{body}");
    let line = serde_json::from_str::<Value>(&body).unwrap()["line"].clone();
    assert_eq!(line, "added web, off: turn it on in settings › mcp servers");
    let ls = s.ls();
    assert!(ls.contains(&format!("[stdio] web · off · {exact}")), "{ls}");
    assert!(!ran.exists(), "approving adds it off; nothing ran");
}

#[test]
fn the_phone_toggles_and_picks_tools_but_a_command_or_url_change_is_refused_server_side() {
    let s = setup("mcp-phone");
    let page = s.cooee.get("/mcp").1;
    for part in [
        r#"<div class="acr" data-server="fs"><span class="ptg">stdio</span>"#,
        r#"<span class="st m">edit on Mac</span>"#,
        r#"name="t:fs:a" checked"#,
        r#"name="t:fs:b">"#,
    ] {
        assert!(page.contains(part), "{part} not in {page}");
    }
    let config = || fs::read_to_string(&s.cooee.config).unwrap();
    let before = config();
    for edit in [
        "enabled=on&command=%2Fbin%2Fevil",
        "url=https%3A%2F%2Fevil",
        "args=x&enabled=off",
        "env=T",
    ] {
        let (status, body) = s.form("/mcp/servers/fs", edit);
        assert_eq!(status, 403, "{edit}: {body}");
        assert!(body.contains("on the Mac only"), "{body}");
    }
    assert_eq!(config(), before, "a refused edit changes nothing");
    assert_eq!(s.form("/mcp/servers/fs", "enabled=off").0, 303);
    assert!(s.ls().contains("[stdio] fs · off"));
    assert_eq!(s.form("/mcp/servers/fs", "enabled=on").0, 303);
    assert!(s.ls().contains("[stdio] fs · on"));
    assert!(config().contains("secret:TOKEN"), "{}", config());

    assert_eq!(
        s.form(
            "/profiles/coder/mcp",
            "s%3Afs=on&t%3Afs%3Aa=on&t%3Afs%3Ac=on"
        )
        .0,
        303
    );
    let profile = || fs::read_to_string(s.dir.join("profiles/coder.toml")).unwrap();
    assert!(
        profile().ends_with("[[mcp]]\nserver = \"fs\"\ntools = [\"a\",\"c\"]\n"),
        "{}",
        profile()
    );
    let mut m = Mcp::start(named(&s.thread("coder").1, "fs"), Agent::Claude);
    assert_eq!(m.tools(), ["a", "c"]);
    drop(m);
    assert_eq!(
        s.form(
            "/profiles/coder/mcp",
            "s%3Afs=on&t%3Afs%3Aa=on&t%3Afs%3Ab=on&t%3Afs%3Ac=on"
        )
        .0,
        303
    );
    assert!(
        profile().ends_with("[[mcp]]\nserver = \"fs\"\n"),
        "{}",
        profile()
    );

    let add = "name=docs&type=http&target=https%3A%2F%2Fdocs.example%2Fmcp";
    assert_eq!(s.form("/mcp/servers", add).0, 403);
    let phone = Authenticator::new(&s.cooee);
    phone.save(&s.cooee);
    let passkey: String = url(&phone.get(&s.cooee).to_string());
    let (status, body) = s.form("/mcp/servers", &format!("{add}&passkey={passkey}"));
    assert_eq!(
        (status, body.as_str()),
        (200, "added docs, off: turn it on in settings › mcp servers")
    );
    assert!(
        s.ls()
            .contains("[http] docs · off · https://docs.example/mcp")
    );
    let passkey: String = url(&phone.get(&s.cooee).to_string());
    let (status, body) = s.form(
        "/mcp/servers",
        &format!("name=fs&type=stdio&target=%2Fbin%2Fevil&passkey={passkey}"),
    );
    assert!(
        status == 400 && body.contains("change it on the Mac"),
        "{body}"
    );
    assert_eq!(s.form("/mcp/servers/docs/remove", "").0, 303);
    assert!(!s.ls().contains("docs"));

    if !cfg!(target_os = "macos") {
        assert_eq!(s.form("/mcp/secret", "name=NEW&value=s3cr3t-value").0, 303);
        assert_eq!(
            fs::read_to_string(s.dir.join("data/secrets/NEW")).unwrap(),
            "s3cr3t-value"
        );
        assert!(!config().contains("s3cr3t-value"));
    }
}

fn url(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

#[test]
fn the_mac_adds_toggles_and_removes_servers_with_references_only() {
    let dir = scratch("mcp-cli");
    let config = dir.join("config.toml");
    fs::write(&config, "# mine\nport = 1\n").unwrap();
    let cooee = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_cooee"))
            .arg("--config")
            .arg(&config)
            .args(args)
            .output()
            .unwrap();
        (
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).into_owned()
                + &String::from_utf8_lossy(&out.stderr),
        )
    };
    let add = [
        "mcp",
        "add",
        "docs",
        "--type",
        "sse",
        "--url",
        "https://docs.example/sse",
        "--header",
        "Authorization=secret:DOCS",
    ];
    assert_eq!(
        cooee(&add),
        (
            true,
            "added docs: [sse] https://docs.example/sse\n".to_owned()
        )
    );
    let (ok, said) = cooee(&[
        "mcp",
        "add",
        "git",
        "uvx",
        "mcp-server-git",
        "--env",
        "GIT_TOKEN=env:GIT_TOKEN",
    ]);
    assert!(ok, "{said}");
    let (ok, said) = cooee(&["mcp", "add", "bad", "x", "--env", "T=literal"]);
    assert!(!ok && said.contains("env T: only a reference"), "{said}");
    assert!(cooee(&["mcp", "disable", "git"]).0);
    assert_eq!(
        cooee(&["mcp", "ls"]).1,
        "[sse] docs · on · https://docs.example/sse\n[stdio] git · off · uvx mcp-server-git\n"
    );
    assert!(cooee(&["mcp", "rm", "docs"]).0);
    let text = fs::read_to_string(&config).unwrap();
    assert!(
        text.starts_with("# mine\nport = 1\n\n[[mcp]]\nname = \"git\"\n") && !text.contains("docs"),
        "{text}"
    );
    assert!(
        text.contains(r#"env = { "GIT_TOKEN" = "env:GIT_TOKEN" }"#),
        "{text}"
    );
}
