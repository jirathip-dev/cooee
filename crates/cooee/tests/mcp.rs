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
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use serde_json::{Value, json};
use support::{Authenticator, Cooee, FakeShed, agent, is_item, scratch, version};

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
    fn spawn(entry: &Value, agent: Agent) -> Self {
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
        Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            agent,
            next: 0,
        }
    }

    fn start(entry: &Value, agent: Agent) -> Self {
        let mut m = Self::spawn(entry, agent);
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
        let answer = loop {
            let mut line = String::new();
            self.stdout.read_line(&mut line).unwrap();
            let v: Value =
                serde_json::from_str(&line).unwrap_or_else(|_| panic!("{method}: {line:?}"));
            if v.get("method").is_none() {
                break v;
            }
        };
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
            "handoff",
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
    assert_eq!(card["block"]["kind"], "proposal");
    assert_eq!(
        card["block"]["blocks"][0]["markdown"],
        format!("```\n{exact}\n```")
    );
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

    let (coder, _) = d.s.thread("coder");
    assert!(
        d.s.shed
            .adapter_env_of(&format!("cooee-{coder}"))
            .contains(&format!("SITE_KEY={DROPPED}"))
    );
    let writer = "name = \"Writer\"\nagent = \"fake-acp\"\nenv = { MODE = \"write\" }\n";
    agent(&d.s.dir, "writer", writer, "");
    let (writer, _) = d.s.thread("writer");
    let env = d.s.shed.adapter_env_of(&format!("cooee-{writer}"));
    assert!(env.lines().any(|l| l == "MODE=write"), "{env}");
    assert!(!env.contains(DROPPED), "only coder's");
}

const CHROMIUM: &str = r#"#!/bin/sh
for a in "$@"; do
  case "$a" in --user-data-dir=*) d=${a#--user-data-dir=} ;; esac
done
printf '%s\n' "$d" >>"$(dirname "$0")/chromium.log"
p=$(cat "$(dirname "$0")/port" 2>/dev/null || echo 1)
printf '%s\n/devtools/browser/x\n' "$p" >"$d/DevToolsActivePort"
"#;

const PLAYWRIGHT: &str = r#"#!/bin/sh
printf '%s\n' "$*" >>"$(dirname "$0")/playwright.log"
while IFS= read -r line; do
  id=$(printf '%s' "$line" | sed -n 's/.*"id":\([^,}]*\).*/\1/p')
  [ -n "$id" ] || continue
  case "$line" in
  *'"method":"initialize"'*) r='{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"pw","version":"1"}}' ;;
  *'"name":"browser_take_screenshot"'*) printf '%s\n' "$line" >>"$(dirname "$0")/shots.log"; r='{"content":[{"type":"text","text":"shot"},{"type":"image","data":"ZnJhbWUtcGl4ZWxz","mimeType":"image/jpeg"}]}' ;;
  *'"method":"tools/call"'*) printf '%s\n' "$line" >>"$(dirname "$0")/calls.log"; r='{"content":[{"type":"text","text":"done"}]}' ;;
  *) r='{}' ;;
  esac
  printf '{"jsonrpc":"2.0","id":%s,"result":%s}\n' "$id" "$r"
done
"#;

#[test]
fn each_persona_gets_its_own_browser_and_every_call_is_checked_and_logged() {
    let dir = scratch("mcp-browser");
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    for (name, text) in [("chromium", CHROMIUM), ("playwright", PLAYWRIGHT)] {
        fs::write(bin.join(name), text).unwrap();
        fs::set_permissions(bin.join(name), fs::Permissions::from_mode(0o755)).unwrap();
    }
    for (id, site) in [("ann", "example.com"), ("bob", "github.com")] {
        let toml = format!(
            "name = \"{id}\"\nagent = \"fake-acp\"\n\n[[mcp]]\nserver = \"browser\"\n\n[[sites]]\nsite = \"{site}\"\naccount = true\n"
        );
        agent(&dir, id, &toml, "");
    }
    let shed = FakeShed::start(&dir.join("shed"), version());
    let extra = format!(
        "chromium = {:?}\n\n[[mcp]]\nname = \"browser\"\ncommand = {:?}\n",
        bin.join("chromium").to_string_lossy(),
        bin.join("playwright").to_string_lossy()
    );
    let cooee = Cooee::start_env(&dir, &shed.dir, &extra, &[]);
    let s = Setup {
        dir: dir.clone(),
        shed,
        cooee,
        token: String::new(),
    };
    let navigate = |m: &mut Mcp, url: &str| m.run("browser_navigate", &json!({ "url": url }));
    let (_, servers) = s.thread("ann");
    let mut ann = Mcp::start(named(&servers, "browser"), Agent::Claude);
    let (_, servers) = s.thread("bob");
    let mut bob = Mcp::start(named(&servers, "browser"), Agent::Codex);
    assert_eq!(
        navigate(&mut ann, "https://www.example.com/a"),
        Ok("done".to_owned())
    );
    assert_eq!(
        navigate(&mut bob, "https://github.com/"),
        Ok("done".to_owned())
    );
    let e = navigate(&mut ann, "https://github.com/").unwrap_err();
    assert!(
        e.contains("https://github.com/ isn't on ann's site allowlist"),
        "{e}"
    );
    assert!(ann.run("browser_snapshot", &json!({})).is_ok());
    drop((ann, bob));

    let data = dir.join("data/browsers");
    let launched = fs::read_to_string(bin.join("chromium.log")).unwrap();
    let mut profiles: Vec<&str> = launched.lines().collect();
    profiles.sort_unstable();
    profiles.dedup();
    let want = [data.join("ann"), data.join("bob")].map(|p| p.to_string_lossy().into_owned());
    assert_eq!(profiles, want);
    let started = fs::read_to_string(bin.join("playwright.log")).unwrap();
    assert_eq!(
        started.lines().collect::<Vec<_>>(),
        ["--cdp-endpoint http://127.0.0.1:1"; 2]
    );
    let calls = fs::read_to_string(bin.join("calls.log")).unwrap();
    assert_eq!(calls.lines().count(), 3, "{calls}");
    assert_eq!(calls.matches("https://github.com/").count(), 1, "{calls}");
    let logged = |id: &str| -> Vec<(String, Value, bool)> {
        let text = fs::read_to_string(data.join(format!("{id}.log"))).unwrap();
        text.lines()
            .map(|l| serde_json::from_str::<Value>(l).unwrap())
            .map(|v| {
                (
                    v["tool"].as_str().unwrap().to_owned(),
                    v["arguments"].clone(),
                    v["refused"].as_bool().unwrap(),
                )
            })
            .collect()
    };
    let nav = |u: &str| ("browser_navigate".to_owned(), json!({ "url": u }), false);
    let (mut off, snap) = (
        nav("https://github.com/"),
        ("browser_snapshot".to_owned(), json!({}), false),
    );
    off.2 = true;
    assert_eq!(logged("ann"), [nav("https://www.example.com/a"), off, snap]);
    assert_eq!(logged("bob"), [nav("https://github.com/")]);
}

fn fake_page_list(pages: Arc<Mutex<Vec<String>>>) -> u16 {
    let cdp = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = cdp.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for mut c in cdp.incoming().map_while(Result::ok) {
            let head: Vec<String> = BufReader::new(&c)
                .lines()
                .map_while(Result::ok)
                .take_while(|l| !l.trim().is_empty())
                .collect();
            if !head
                .first()
                .is_some_and(|l| l.starts_with("GET /json/list "))
            {
                continue;
            }
            let list: Vec<Value> = pages
                .lock()
                .unwrap()
                .iter()
                .map(|u| json!({ "type": "page", "url": u }))
                .collect();
            let body = json!(list).to_string();
            write!(
                c,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .ok();
        }
    });
    port
}

fn frames_setup(dir: &Path, port: u16) -> Setup {
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    fs::write(bin.join("port"), port.to_string()).unwrap();
    for (name, text) in [("chromium", CHROMIUM), ("playwright", PLAYWRIGHT)] {
        fs::write(bin.join(name), text).unwrap();
        fs::set_permissions(bin.join(name), fs::Permissions::from_mode(0o755)).unwrap();
    }
    let toml = "name = \"ann\"\nagent = \"fake-acp\"\n\n[[mcp]]\nserver = \"browser\"\n\n[[sites]]\nsite = \"example.com\"\n\n[[sites]]\nsite = \"bank.com\"\nno_frames = true\n";
    agent(dir, "ann", toml, "");
    let secrets = dir.join("data/secrets");
    fs::create_dir_all(&secrets).unwrap();
    let key = "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=";
    fs::write(secrets.join("FRAMES_KEY"), key).unwrap();
    let shed = FakeShed::start(&dir.join("shed"), version());
    let extra = format!(
        "chromium = {:?}\n\n[[mcp]]\nname = \"browser\"\ncommand = {:?}\n",
        bin.join("chromium").to_string_lossy(),
        bin.join("playwright").to_string_lossy()
    );
    let cooee = Cooee::start_env(dir, &shed.dir, &extra, &[]);
    Setup {
        dir: dir.to_owned(),
        shed,
        cooee,
        token: String::new(),
    }
}

#[test]
fn each_browser_action_leaves_one_captioned_encrypted_frame_and_a_no_frames_site_none() {
    let dir = scratch("mcp-frames");
    let pages = Arc::new(Mutex::new(vec!["https://www.example.com/a".to_owned()]));
    let s = frames_setup(&dir, fake_page_list(Arc::clone(&pages)));
    let (id, servers) = s.thread("ann");
    let mut ann = Mcp::start(named(&servers, "browser"), Agent::Claude);
    let shown = [
        (
            "browser_navigate",
            json!({ "url": "https://www.example.com/a" }),
        ),
        (
            "browser_click",
            json!({ "element": "Sign in", "ref": "e1" }),
        ),
        (
            "browser_type",
            json!({ "element": "Password", "ref": "e2", "text": "hunter2" }),
        ),
        ("browser_snapshot", json!({})),
    ];
    let banked = [
        (
            "browser_navigate",
            json!({ "url": "https://bank.com/acct" }),
        ),
        (
            "browser_click",
            json!({ "element": "Transfer", "ref": "e3" }),
        ),
    ];
    for (tool, args) in &shown {
        assert_eq!(ann.run(tool, args), Ok("done".to_owned()), "{tool}");
    }
    let shot = ann.run("browser_take_screenshot", &json!({ "type": "jpeg" }));
    assert_eq!(shot, Ok("shot".to_owned()));
    *pages.lock().unwrap() = vec!["https://bank.com/acct".to_owned()];
    for (tool, args) in &banked {
        assert_eq!(ann.run(tool, args), Ok("done".to_owned()), "{tool}");
    }
    drop(ann);

    let items = s.cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
    let frames: Vec<(&str, &str)> = items
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["kind"] == "frame")
        .map(|i| (i["text"].as_str().unwrap(), i["name"].as_str().unwrap()))
        .collect();
    let captions: Vec<&str> = frames.iter().map(|f| f.0).collect();
    let asked: Vec<bool> = items
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["kind"] == "frame")
        .map(|i| i["asked"] == true)
        .collect();
    assert_eq!(asked, [false, false, false, true]);
    let want = [
        r#"navigate "https://www.example.com/a""#,
        r#"click "Sign in""#,
        r#"type "••••••••" into "Password""#,
        "https://www.example.com/a",
    ];
    assert_eq!(captions, want, "{items}");
    let stored = dir.join("data/frames").join(&id);
    assert_eq!(fs::read_dir(&stored).unwrap().count(), 4);
    for (_, name) in &frames {
        let served = s.cooee.get(&format!("/threads/{id}/frames/{name}"));
        assert_eq!(served, (200, "frame-pixels".to_owned()));
        let sealed = fs::read(stored.join(name)).unwrap();
        assert!(
            !sealed.windows(12).any(|w| w == b"frame-pixels"),
            "{name} is plain on disk"
        );
    }
    let bin = dir.join("bin");
    let shots = fs::read_to_string(bin.join("shots.log")).unwrap();
    assert_eq!(shots.lines().count(), 6, "{shots}");
    assert!(
        fs::read_to_string(bin.join("calls.log"))
            .unwrap()
            .contains("hunter2")
    );
    let log = fs::read_to_string(dir.join("data/browsers/ann.log")).unwrap();
    assert!(
        !log.contains("hunter2") && log.contains("••••••••"),
        "{log}"
    );
}

fn cdp_client(mut s: TcpStream, port: u16, got: &Mutex<Vec<Value>>) {
    let mut r = BufReader::new(s.try_clone().unwrap());
    let mut first = String::new();
    if r.read_line(&mut first).unwrap_or(0) == 0 {
        return;
    }
    let mut line = String::new();
    while r.read_line(&mut line).unwrap() > 2 {
        line.clear();
    }
    if first.starts_with("GET /json/list") {
        let tabs = json!([{ "type": "page", "url": "https://www.example.com/a", "title": "A",
                            "webSocketDebuggerUrl": format!("ws://127.0.0.1:{port}/devtools/page/1") }])
        .to_string();
        let head = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", tabs.len());
        s.write_all((head + &tabs).as_bytes()).unwrap();
        return;
    }
    s.write_all(
        b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\r\n",
    )
    .unwrap();
    let mut head = [0; 2];
    while r.read_exact(&mut head).is_ok() {
        let len = match head[1] & 0x7f {
            126 => {
                let mut b = [0; 2];
                r.read_exact(&mut b).unwrap();
                usize::from(u16::from_be_bytes(b))
            }
            n => usize::from(n),
        };
        let mut mask = [0; 4];
        r.read_exact(&mut mask).unwrap();
        let mut body = vec![0; len];
        r.read_exact(&mut body).unwrap();
        for (i, b) in body.iter_mut().enumerate() {
            *b ^= mask[i % 4];
        }
        let call: Value = serde_json::from_slice(&body).unwrap();
        let event = json!({ "method": "Page.loadEventFired" }).to_string();
        let result = match call["params"]["expression"].as_str() {
            Some(e) if e.contains("elementFromPoint") => {
                json!({ "result": { "value": "Sign in" } })
            }
            Some(_) => json!({ "result": { "value": "Email" } }),
            None => json!({ "data": "anBlZw==" }),
        };
        let reply = json!({ "id": call["id"], "result": result }).to_string();
        got.lock().unwrap().push(call);
        for m in [event, reply] {
            s.write_all(&[0x81, u8::try_from(m.len()).unwrap()])
                .unwrap();
            s.write_all(m.as_bytes()).unwrap();
        }
    }
}

fn fake_cdp() -> (u16, Arc<Mutex<Vec<Value>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let got: Arc<Mutex<Vec<Value>>> = Arc::default();
    let g = Arc::clone(&got);
    thread::spawn(move || {
        for s in listener.incoming().map_while(Result::ok) {
            let g = Arc::clone(&g);
            thread::spawn(move || cdp_client(s, port, &g));
        }
    });
    (port, got)
}

fn screen_setup(name: &str) -> (Setup, PathBuf, Arc<Mutex<Vec<Value>>>) {
    let dir = scratch(name);
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    for (name, text) in [("chromium", CHROMIUM), ("playwright", PLAYWRIGHT)] {
        fs::write(bin.join(name), text).unwrap();
        fs::set_permissions(bin.join(name), fs::Permissions::from_mode(0o755)).unwrap();
    }
    let (port, cdp) = fake_cdp();
    fs::write(bin.join("port"), port.to_string()).unwrap();
    let toml = "name = \"ann\"\nagent = \"fake-acp\"\n\n[[mcp]]\nserver = \"browser\"\n\n[[sites]]\nsite = \"example.com\"\n";
    agent(&dir, "ann", toml, "");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let extra = format!(
        "chromium = {:?}\n\n[[mcp]]\nname = \"browser\"\ncommand = {:?}\n",
        bin.join("chromium").to_string_lossy(),
        bin.join("playwright").to_string_lossy()
    );
    let cooee = Cooee::start_env(&dir, &shed.dir, &extra, &[]);
    let s = Setup {
        dir: dir.clone(),
        shed,
        cooee,
        token: String::new(),
    };
    (s, bin, cdp)
}

#[test]
fn the_owner_watches_takes_control_and_hands_back_with_a_note_the_waiting_call_carries() {
    let (s, bin, cdp) = screen_setup("mcp-screen");
    let (_, servers) = s.thread("ann");
    let mut ann = Mcp::start(named(&servers, "browser"), Agent::Claude);
    let nav = json!({ "name": "browser_navigate", "arguments": { "url": "https://example.com/" } });
    let texts = |r: &Value| -> Vec<String> {
        let content = r["content"].as_array().unwrap();
        content
            .iter()
            .map(|c| c["text"].as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(texts(&ann.call("tools/call", &nav).unwrap()), ["done"]);
    let calls = || {
        fs::read_to_string(bin.join("calls.log"))
            .unwrap()
            .lines()
            .count()
    };
    let screen = || s.cooee.json("/personas/ann/screen");
    assert_eq!(
        screen(),
        json!({ "control": { "by": "agent" }, "waiting": 0 })
    );

    let mut live = s.cooee.sse("/personas/ann/screen/live", "");
    let frame = live.next();
    assert_eq!(frame.event, "frame");
    assert_eq!(
        frame.data,
        json!({ "url": "https://www.example.com/a", "title": "A", "jpeg": "anBlZw==" })
    );
    drop(live);

    let click = json!({ "click": { "x": 10.0, "y": 20.0 } });
    let (status, body) = s.cooee.post("/personas/ann/screen/input", &click);
    assert_eq!(
        (status, body.as_str()),
        (400, "take control of ann's browser first")
    );
    let (status, body) = s
        .cooee
        .post("/personas/ann/screen/back", &json!({ "note": "x" }));
    assert_eq!(
        (status, body.as_str()),
        (400, "you don't hold ann's browser")
    );

    let (status, body) = s.cooee.post("/personas/ann/screen/control", &json!({}));
    assert_eq!(status, 200, "{body}");
    let until = serde_json::from_str::<Value>(&body).unwrap()["control"]["until"]
        .as_u64()
        .unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let left = u128::from(until) - now;
    assert!((590_000..=600_000).contains(&left), "{left}");
    for input in [click, json!({ "text": "hunter2" })] {
        let (status, body) = s.cooee.post("/personas/ann/screen/input", &input);
        assert_eq!(status, 200, "{body}");
    }

    let waiting = thread::spawn(move || {
        let r = ann.call("tools/call", &nav).unwrap();
        (ann, r)
    });
    let mut tries = 0;
    while screen()["waiting"] != 1 {
        tries += 1;
        assert!(tries < 500, "the agent's call never waited: {}", screen());
        thread::sleep(Duration::from_millis(20));
    }
    assert!(!waiting.is_finished());
    assert_eq!(
        calls(),
        1,
        "the agent's call ran while the owner held control"
    );

    let (status, body) = s
        .cooee
        .post("/personas/ann/screen/back", &json!({ "note": "signed in" }));
    assert_eq!(status, 200, "{body}");
    let (mut ann, r) = waiting.join().unwrap();
    assert_eq!(
        texts(&r),
        [
            "done",
            "The owner had the browser and handed it back: signed in"
        ]
    );
    assert_eq!(calls(), 2);
    assert_eq!(
        screen(),
        json!({ "control": { "by": "agent" }, "waiting": 0 })
    );
    let nav =
        json!({ "name": "browser_navigate", "arguments": { "url": "https://example.com/b" } });
    assert_eq!(texts(&ann.call("tools/call", &nav).unwrap()), ["done"]);

    assert_sent(&cdp);
}

#[test]
fn a_lease_saved_as_a_skill_hands_the_persona_its_steps_and_no_typed_value_and_a_declined_card_saves_nothing()
 {
    let (s, _, _) = screen_setup("mcp-screen-skill");
    let (id, servers) = s.thread("ann");
    let mut ann = Mcp::start(named(&servers, "browser"), Agent::Claude);
    let (status, body) = s.cooee.post("/personas/ann/screen/control", &json!({}));
    assert_eq!(status, 200, "{body}");
    for input in [
        json!({ "click": { "x": 10.0, "y": 20.0 } }),
        json!({ "text": "hunter2" }),
    ] {
        let (status, body) = s.cooee.post("/personas/ann/screen/input", &input);
        assert_eq!(status, 200, "{body}");
    }
    let until = |b: &str| serde_json::from_str::<Value>(b).unwrap()["control"]["until"].as_u64();
    let (_, more) = s.cooee.post("/personas/ann/screen/control", &json!({}));
    assert_eq!(until(&more), until(&body).map(|u| u + 600_000), "{more}");
    let back = json!({ "note": "done it", "skill": true });
    let (status, body) = s.cooee.post("/personas/ann/screen/back", &back);
    assert_eq!(status, 200, "{body}");
    let nav = json!({ "name": "browser_navigate", "arguments": { "url": "https://example.com/" } });
    let r = ann.call("tools/call", &nav).unwrap();
    let note = r["content"][1]["text"].as_str().unwrap();
    assert_eq!(
        note,
        "The owner had the browser and handed it back: done it\n\nThe owner asks you to save what they just did as a \
         skill: draft it from these steps and propose it with your skill tool. What they typed is left out: name the \
         field, never a value.\n1. https://www.example.com \"A\": click \"Sign in\"\n2. https://www.example.com \"A\": \
         type into \"Email\""
    );

    let steps = note.split_once("value.\n").unwrap().1;
    let mut cooee = Mcp::start(named(&servers, "cooee"), Agent::Claude);
    let plan = json!({ "name": "sign-in", "description": "How I sign in.", "body": steps });
    let said = cooee.run("skill", &json!({ "propose": plan })).unwrap();
    assert!(said.starts_with("Proposed."), "{said}");
    let items = s.cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
    let card = items
        .as_array()
        .unwrap()
        .iter()
        .rfind(|i| i["form"] == "skill")
        .unwrap()
        .clone();
    assert!(
        card["detail"]
            .as_str()
            .unwrap()
            .contains("type into \"Email\"")
    );
    let tap = json!({ "item": card["id"], "option": "decline" });
    let (status, body) = s.cooee.post(&format!("/threads/{id}/approve"), &tap);
    assert_eq!(status, 200, "{body}");
    assert!(!s.dir.join("agents/ann/skills/sign-in").exists());
    assert_eq!(s.cooee.json("/personas/ann/skills"), json!([]));
}

fn assert_sent(cdp: &Mutex<Vec<Value>>) {
    let sent: Vec<(String, Value)> = cdp
        .lock()
        .unwrap()
        .iter()
        .map(|c| {
            let params = match c["params"]["expression"].as_str() {
                Some(e) => json!(e.rsplit_once(")(").unwrap().1),
                None => c["params"].clone(),
            };
            (c["method"].as_str().unwrap().to_owned(), params)
        })
        .collect();
    let ask = |at: &str| ("Runtime.evaluate".to_owned(), json!(at));
    let mouse = |kind: &str| {
        (
            "Input.dispatchMouseEvent".to_owned(),
            json!({ "type": kind, "x": 10.0, "y": 20.0, "button": "left", "clickCount": 1 }),
        )
    };
    assert_eq!(
        sent,
        [
            (
                "Page.captureScreenshot".to_owned(),
                json!({ "format": "jpeg", "quality": 60 })
            ),
            ask("document.elementFromPoint(10, 20))"),
            mouse("mousePressed"),
            mouse("mouseReleased"),
            ask("document.activeElement)"),
            ("Input.insertText".to_owned(), json!({ "text": "hunter2" })),
        ]
    );
}

fn heard(s: &TcpStream) -> (String, Value) {
    let mut r = BufReader::new(s.try_clone().unwrap());
    let mut head = String::new();
    while r.read_line(&mut head).unwrap() > 2 {}
    let len = head.lines().find_map(|l| {
        let l = l.to_ascii_lowercase();
        l.strip_prefix("content-length:")
            .map(|n| n.trim().parse().unwrap())
    });
    let mut body = vec![0; len.unwrap_or(0)];
    r.read_exact(&mut body).unwrap();
    (head, serde_json::from_slice(&body).unwrap_or_default())
}

fn answered(msg: &Value, head: &str, token: &str) -> Value {
    let tools = json!([{ "name": "a", "inputSchema": {} }, { "name": "b", "inputSchema": {} },
                       { "name": "c", "inputSchema": {} }]);
    let signed = head
        .to_ascii_lowercase()
        .contains(&format!("authorization: bearer {token}\r\n"));
    let k = if signed { "matches" } else { "differs" };
    let result = match msg["method"].as_str().unwrap_or_default() {
        "initialize" => json!({ "protocolVersion": "2025-06-18", "capabilities": { "tools": {} } }),
        "tools/list" => json!({ "tools": tools }),
        "tools/call" => {
            json!({ "content": [{ "type": "text", "text": format!("{} ran; the token {k}", msg["params"]["name"].as_str().unwrap()) }] })
        }
        _ => json!({}),
    };
    json!({ "jsonrpc": "2.0", "id": msg["id"], "result": result })
}

fn fake_web(token: String) -> (u16, Arc<Mutex<Vec<String>>>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let seen: Arc<Mutex<Vec<String>>> = Arc::default();
    let open: Arc<Mutex<Option<std::sync::mpsc::Sender<Value>>>> = Arc::default();
    let log = Arc::clone(&seen);
    thread::spawn(move || {
        for mut s in l.incoming().map(Result::unwrap) {
            let (log, open, token) = (Arc::clone(&log), Arc::clone(&open), token.clone());
            thread::spawn(move || {
                let (head, msg) = heard(&s);
                log.lock().unwrap().push(format!("{head}{msg}"));
                let path = head.split([' ', '?']).nth(1).unwrap().to_owned();
                let reply = |status: &str, more: &str, body: &str| {
                    format!(
                        "HTTP/1.1 {status}\r\n{more}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                };
                let answer = answered(&msg, &head, &token);
                let out = match (path.as_str(), msg["method"].as_str()) {
                    ("/sse", _) => {
                        let (tx, rx) = std::sync::mpsc::channel();
                        *open.lock().unwrap() = Some(tx);
                        write!(s, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\nevent: endpoint\ndata: /messages?s=1\n\n").unwrap();
                        for m in rx {
                            write!(s, "event: message\ndata: {m}\n\n").unwrap();
                        }
                        return;
                    }
                    ("/messages", _) => {
                        if msg.get("id").is_some() {
                            let tx = open.lock().unwrap().clone().unwrap();
                            tx.send(answer).unwrap();
                        }
                        reply("202 Accepted", "", "")
                    }
                    ("/moved", _) => reply(
                        "307 Temporary Redirect",
                        &format!("Location: http://127.0.0.1:{port}/mcp?redirected\r\n"),
                        "",
                    ),
                    (_, Some("tools/list")) => {
                        let note = json!({ "jsonrpc": "2.0", "method": "notifications/message", "params": {} });
                        let body = format!(
                            "event: message\ndata: {note}\n\nevent: message\ndata: {answer}\n\n"
                        );
                        reply("200 OK", "Content-Type: text/event-stream\r\n", &body)
                    }
                    (_, Some("initialize")) => reply(
                        "200 OK",
                        "Content-Type: application/json\r\nMcp-Session-Id: s-1\r\n",
                        &answer.to_string(),
                    ),
                    _ if msg.get("id").is_none() => reply("202 Accepted", "", ""),
                    _ => reply(
                        "200 OK",
                        "Content-Type: application/json\r\n",
                        &answer.to_string(),
                    ),
                };
                s.write_all(out.as_bytes()).unwrap();
            });
        }
    });
    (port, seen)
}

#[test]
fn a_streamable_http_and_an_sse_server_answer_through_the_pipe_with_headers_on_curls_stdin_and_no_redirect()
 {
    let dir = scratch("mcp-pipe");
    let token = format!("tk-{}", "w3".repeat(16));
    fs::create_dir_all(dir.join("data/secrets")).unwrap();
    fs::write(dir.join("data/secrets/WEB"), format!("Bearer {token}\n")).unwrap();
    let real = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|p| p.join("curl"))
        .find(|p| p.is_file())
        .unwrap();
    let wrap = dir.join("wrap/curl");
    fs::create_dir_all(wrap.parent().unwrap()).unwrap();
    let log = dir.join("curl-argv");
    fs::write(
        &wrap,
        format!("#!/bin/sh\nprintf '%s\\n' \"$*\" >>{log:?}\nexec {real:?} \"$@\"\n"),
    )
    .unwrap();
    fs::set_permissions(&wrap, fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!(
        "{}:{}",
        wrap.parent().unwrap().display(),
        std::env::var("PATH").unwrap()
    );
    let picks = "name = \"Web\"\nagent = \"fake-acp\"\n\n[[mcp]]\nserver = \"web\"\ntools = [\"a\"]\n\n[[mcp]]\nserver = \"old\"\n\n[[mcp]]\nserver = \"moved\"\n";
    agent(&dir, "web", picks, "");
    let (port, seen) = fake_web(token.clone());
    let server = |name: &str, kind: &str, at: &str| {
        format!(
            "[[mcp]]\nname = \"{name}\"\ntype = \"{kind}\"\nurl = \"http://127.0.0.1:{port}/{at}\"\nheaders = {{ Authorization = \"secret:WEB\" }}\nread = [\"a\", \"c\"]\n\n"
        )
    };
    let extra = server("web", "http", "mcp")
        + &server("old", "sse", "sse")
        + &server("moved", "http", "moved");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start_env(&dir, &shed.dir, &extra, &[("PATH", &path)]);
    let s = Setup {
        dir: dir.clone(),
        shed,
        cooee,
        token: token.clone(),
    };
    let (id, servers) = s.thread("web");

    let mut web = Mcp::start(named(&servers, "web"), Agent::Claude);
    assert_eq!(web.tools(), ["a"]);
    assert_eq!(
        web.run("a", &json!({})),
        Ok("a ran; the token matches".to_owned())
    );
    let refused = web.run("b", &json!({})).unwrap_err();
    assert!(refused.contains("tool b isn't allowed"), "{refused}");
    let last = seen.lock().unwrap().last().unwrap().to_ascii_lowercase();
    assert!(
        last.contains("mcp-session-id: s-1\r\n")
            && last.contains("mcp-protocol-version: 2025-06-18\r\n"),
        "{last}"
    );

    let mut old = Mcp::start(named(&servers, "old"), Agent::Codex);
    assert_eq!(old.tools(), ["a", "b", "c"]);
    assert_eq!(
        old.run("c", &json!({})),
        Ok("c ran; the token matches".to_owned())
    );

    let mut moved = Mcp::spawn(named(&servers, "moved"), Agent::Claude);
    let e = moved.call("initialize", &json!({})).unwrap_err();
    assert!(e.contains("HTTP 307"), "{e}");
    assert!(
        !seen
            .lock()
            .unwrap()
            .iter()
            .any(|h| h.contains("redirected"))
    );

    let var = named(&servers, "web")["env"][0]["value"]
        .as_str()
        .unwrap()
        .to_owned();
    let (url, bearer) = var.split_once(' ').unwrap();
    let mut c = TcpStream::connect(url.strip_prefix("http://").unwrap()).unwrap();
    let body = json!({ "thread": id, "server": "web" }).to_string();
    write!(c, "POST /mcp/agent/grant HTTP/1.0\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {bearer}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", body.len()).unwrap();
    let mut grant = String::new();
    c.read_to_string(&mut grant).unwrap();
    assert!(grant.ends_with("\"pipe\""), "{grant}");
    assert!(
        !grant.contains(&s.token) && !grant.contains(&port.to_string()),
        "{grant}"
    );

    let argv = fs::read_to_string(&log).unwrap();
    assert!(argv.lines().count() >= 8, "{argv}");
    assert!(argv.lines().all(|l| l == "-K -"), "{argv}");
    let mut all = fs::read_to_string(&s.cooee.config).unwrap();
    files(&dir.join("data"), &mut all);
    all.push_str(&fs::read_to_string(dir.join("cooee.err")).unwrap());
    assert!(!all.contains(&s.token), "a value was written");
}

#[test]
fn a_read_tool_runs_with_no_card_and_a_write_tool_runs_only_once_its_card_is_allowed() {
    let dir = scratch("mcp-gate");
    let token = format!("tk-{}", "g5".repeat(16));
    fs::create_dir_all(dir.join("data/secrets")).unwrap();
    fs::write(dir.join("data/secrets/WEB"), format!("Bearer {token}\n")).unwrap();
    let picks = "name = \"Web\"\nagent = \"fake-acp\"\n\n[[mcp]]\nserver = \"web\"\n";
    agent(&dir, "web", picks, "");
    let (port, seen) = fake_web(token.clone());
    let extra = format!(
        "[[mcp]]\nname = \"web\"\ntype = \"http\"\nurl = \"http://127.0.0.1:{port}/mcp\"\nheaders = {{ Authorization = \"secret:WEB\" }}\nread = [\"a\"]\n"
    );
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start_with(&dir, &shed.dir, &extra);
    let s = Setup {
        dir,
        shed,
        cooee,
        token,
    };
    let (id, servers) = s.thread("web");
    let messages = || s.cooee.json(&format!("/threads/{id}/messages"));
    let sent = |tool: &str| {
        let name = format!("\"name\":\"{tool}\"");
        seen.lock()
            .unwrap()
            .iter()
            .filter(|h| h.contains(&name))
            .count()
    };
    let mut web = Mcp::start(named(&servers, "web"), Agent::Claude);

    assert_eq!(
        web.run("a", &json!({})),
        Ok("a ran; the token matches".to_owned())
    );
    let items = messages()["items"].clone();
    assert!(
        !items
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["kind"] == "approval"),
        "{items}"
    );

    for (tool, option) in [("b", "approve"), ("c", "decline")] {
        let cursor = messages()["cursor"].as_str().unwrap().to_owned();
        let mut events = s.cooee.events(&id, &cursor, false);
        let asked = thread::spawn(move || {
            let r = web.run(tool, &json!({ "title": "Fix it" }));
            (web, r)
        });
        let card = events
            .until(|e| is_item(e, "approval") && e.data["form"] == "tool")
            .pop()
            .unwrap()
            .data;
        assert_eq!(
            card["tool"],
            format!("Web · web: {tool} {{\"title\":\"Fix it\"}}"),
            "{card}"
        );
        assert_eq!(sent(tool), 0, "nothing is sent before the owner answers");
        let tap = json!({ "item": card["id"], "option": option });
        let (status, body) = s.cooee.post(&format!("/threads/{id}/approve"), &tap);
        assert_eq!(status, 200, "{body}");
        let (back, r) = asked.join().unwrap();
        web = back;
        if option == "approve" {
            assert_eq!(r, Ok("b ran; the token matches".to_owned()));
            assert_eq!(sent(tool), 1);
        } else {
            assert_eq!(
                r,
                Err("the owner didn't allow c: its card was declined or expired".to_owned())
            );
            assert_eq!(sent(tool), 0);
        }
    }
}
