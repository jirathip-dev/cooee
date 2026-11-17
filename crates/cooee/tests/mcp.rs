//! MCP servers (#56) end to end with the fake shed (tests/support), whose CLI keeps the `--mcp` file a start hands
//! it, so each test plays the agent: it starts every server in that file as Claude's and Codex's ACP adapters do
//! (their own `initialize`, id styles and order) and talks MCP to it. A fake stdio server offers tools a, b and c.
//! The registry refuses a literal token; a profile's allowlist is all the agent sees and may call; references
//! resolve when the proxy starts and their values reach no file, log or event; a closed thread's token stops
//! working; `propose_server` only raises a card, which needs Face ID and adds the server off. A `secret` (#163) the owner
//! drops with Face ID is typed into its terminal once, or saved for the persona, and reaches no transcript, event,
//! file, log or model input; the agent only reads `secret NAME entered`.

mod support;

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use serde_json::{Value, json};
use support::{Authenticator, Cooee, FakeShed, agent, scratch, version};

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
    let coder =
        "name = \"Coder\"\nagent = \"fake-acp\"\n\n[[mcp]]\nserver = \"fs\"\ntools = [\"a\"]\n";
    agent(&dir, "coder", coder, "");
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
        self.child.kill().ok();
        self.child.wait().ok();
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

#[test]
fn propose_server_shows_the_exact_command_runs_nothing_and_needs_face_id_to_add_it_off() {
    let s = setup("mcp-propose");
    let (id, servers) = s.thread("");
    let ran = s.dir.join("bin/ran");
    let web = s.dir.join("bin/fake-mcp");
    let mut cooee = Mcp::start(named(&servers, "cooee"), Agent::Codex);
    assert_eq!(
        cooee.tools(),
        [
            "propose_server",
            "remember",
            "ask_persona",
            "history",
            "update_persona",
            "create_routine",
            "skill",
            "secret"
        ]
    );
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
        (403, "Approving this needs Face ID.")
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

const DROPPED: &str = "drop-v4lue-k9q2-unseen";

struct SecretDrop {
    s: Setup,
    id: String,
    agent: Mcp,
    phone: Authenticator,
}

fn drop_setup(name: &str) -> SecretDrop {
    let s = setup(name);
    s.shed.plain("login");
    let (id, servers) = s.thread("coder");
    let agent = Mcp::start(named(&servers, "cooee"), Agent::Claude);
    let phone = Authenticator::new(&s.cooee);
    phone.save(&s.cooee);
    SecretDrop {
        s,
        id,
        agent,
        phone,
    }
}

impl SecretDrop {
    fn card(&self) -> Value {
        let items = self.s.cooee.json(&format!("/threads/{}/messages", self.id));
        let cards = items["items"].as_array().unwrap().iter();
        let mut cards = cards.filter(|i| i["form"] == "secret");
        cards.next_back().expect("the secret card").clone()
    }

    fn drop_it(&self, card: &Value, passkey: &Value) -> (u16, String) {
        let answer = card["block"]["blocks"][0]["values"]["choice"].clone();
        self.s.cooee.post(
            &format!("/threads/{}/approve", self.id),
            &json!({ "item": card["id"], "option": answer, "value": DROPPED, "passkey": passkey }),
        )
    }

    fn events(&self) -> support::Events {
        let messages = self.s.cooee.json(&format!("/threads/{}/messages", self.id));
        let boot = messages["cursor"]
            .as_str()
            .unwrap()
            .split('.')
            .next()
            .unwrap();
        self.s.cooee.events(&self.id, &format!("{boot}.0"), false)
    }

    fn told(&self) -> Vec<String> {
        self.s.shed.sent(&format!("cooee-{}", self.id))
    }

    fn leaks(&self, events: &[support::Sse]) -> String {
        let mut seen = self
            .s
            .cooee
            .json(&format!("/threads/{}/messages", self.id))
            .to_string();
        seen.extend(events.iter().map(|e| e.data.to_string()));
        seen.extend(self.told());
        files(&self.s.dir, &mut seen);
        seen
    }
}

#[test]
fn a_secret_dropped_once_is_typed_into_its_terminal_and_reaches_no_transcript_event_file_log_or_model()
 {
    let mut d = drop_setup("mcp-secret-once");
    let own = d.agent.run(
        "secret",
        &json!({ "name": "SITE_PASSWORD", "terminal": format!("cooee-{}", d.id) }),
    );
    assert!(own.unwrap().contains("an agent's session"));
    let bad = d.agent.run(
        "secret",
        &json!({ "name": "not a var", "terminal": "login" }),
    );
    assert!(bad.unwrap().contains("like a variable"));
    let mut events = d.events();
    let ask = json!({ "name": "SITE_PASSWORD", "why": "sign in to the vendor portal", "terminal": "login" });
    let said = d.agent.run("secret", &ask).unwrap();
    assert!(
        said.starts_with("Asked the owner for secret SITE_PASSWORD"),
        "{said}"
    );
    let card = d.card();
    assert!(
        card["tool"]
            .as_str()
            .unwrap()
            .ends_with("typed once, never stored"),
        "{card}"
    );
    assert_eq!(card["block"]["kind"], "secret", "{card}");

    assert_eq!(d.drop_it(&card, &Value::Null).0, 403, "no Face ID");
    assert_eq!(d.s.shed.sent("login"), Vec::<String>::new());
    let (status, body) = d.drop_it(&card, &d.phone.get(&d.s.cooee));
    assert_eq!(status, 200, "{body}");
    let line = serde_json::from_str::<Value>(&body).unwrap()["line"].clone();
    assert_eq!(line, "secret SITE_PASSWORD entered");
    assert_eq!(d.s.shed.sent("login"), [DROPPED]);
    assert_eq!(d.s.shed.calls("keys", "login").len(), 1, "then enter");
    let seen = events.until(|e| e.data.to_string().contains("SITE_PASSWORD entered"));
    let told = d.told();
    assert!(
        told.iter()
            .any(|t| t.contains("secret SITE_PASSWORD entered")),
        "{told:?}"
    );
    assert!(!d.leaks(&seen).contains(DROPPED), "the value leaked");
    assert!(
        d.s.dir
            .join("data/secrets")
            .read_dir()
            .unwrap()
            .all(|e| e.unwrap().file_name() == "TOKEN"),
        "never stored"
    );

    let again = d.agent.run("secret", &ask).unwrap();
    assert!(
        again.starts_with("Asked the owner"),
        "a once secret isn't kept: {again}"
    );
}

#[cfg(not(target_os = "macos"))]
#[test]
fn a_saved_secret_is_typed_again_without_asking_and_reaches_the_personas_environment_only() {
    let mut d = drop_setup("mcp-secret-saved");
    let mut events = d.events();
    let ask =
        json!({ "name": "SITE_KEY", "why": "the vendor API", "terminal": "login", "save": true });
    d.agent.run("secret", &ask).unwrap();
    let card = d.card();
    assert!(
        card["tool"]
            .as_str()
            .unwrap()
            .ends_with("saved in the keychain for coder"),
        "{card}"
    );
    let (status, body) = d.drop_it(&card, &d.phone.get(&d.s.cooee));
    assert_eq!(status, 200, "{body}");
    let line = serde_json::from_str::<Value>(&body).unwrap()["line"].clone();
    assert_eq!(line, "secret SITE_KEY entered");
    let seen = events.until(|e| e.data.to_string().contains("SITE_KEY entered"));
    assert!(!d.leaks(&seen).contains(DROPPED), "the value leaked");
    let file = d.s.dir.join("data/secrets/coder.SITE_KEY");
    assert_eq!(fs::read_to_string(&file).unwrap(), DROPPED);
    assert_eq!(
        fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o600
    );

    let said = d.agent.run("secret", &ask).unwrap();
    assert_eq!(said, "secret SITE_KEY entered");
    assert_eq!(d.s.shed.sent("login"), [DROPPED, DROPPED]);
    assert_eq!(d.card()["id"], card["id"], "no second card");

    d.s.thread("coder");
    assert!(
        d.s.shed
            .adapter_env()
            .contains(&format!("SITE_KEY={DROPPED}"))
    );
    d.s.thread("");
    assert!(!d.s.shed.adapter_env().contains(DROPPED), "only coder's");
}
