//! MCP servers (#56) end to end with the fake shed (tests/support), whose CLI keeps the `--mcp` file a start hands
//! it, so each test plays the agent: it starts every server in that file as Claude's and Codex's ACP adapters do
//! (their own `initialize`, id styles and order) and talks MCP to it. A fake stdio server offers tools a, b and c.
//! The registry refuses a literal token; a profile's allowlist is all the agent sees and may call; references
//! resolve when the proxy starts and their values reach no file, log or event; a closed thread's token stops
//! working; `propose_server` only raises a card, which needs Face ID and adds the server off.

mod support;

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::{Arc, Mutex},
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
            "skill"
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

const CHROMIUM: &str = r#"#!/bin/sh
for a in "$@"; do
  case "$a" in --user-data-dir=*) d=${a#--user-data-dir=} ;; esac
done
printf '%s\n' "$d" >>"$(dirname "$0")/chromium.log"
printf '1\n/devtools/browser/x\n' >"$d/DevToolsActivePort"
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

fn fake_cdp(pages: Arc<Mutex<Vec<String>>>) -> u16 {
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
    let chromium = CHROMIUM.replace("printf '1\\n", &format!("printf '{port}\\n"));
    for (name, text) in [("chromium", chromium.as_str()), ("playwright", PLAYWRIGHT)] {
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
    let s = frames_setup(&dir, fake_cdp(Arc::clone(&pages)));
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
    let want = [
        r#"navigate "https://www.example.com/a""#,
        r#"click "Sign in""#,
        r#"type "••••••••" into "Password""#,
    ];
    assert_eq!(captions, want, "{items}");
    let stored = dir.join("data/frames").join(&id);
    assert_eq!(fs::read_dir(&stored).unwrap().count(), 3);
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
    assert_eq!(shots.lines().count(), 5, "{shots}");
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
