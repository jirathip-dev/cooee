//! Chief (#664) end to end with the fake shed, its tools called the way an agent calls them: through cooee's own MCP
//! server with its thread's token. Only Chief's own chat with the owner lists and runs them; persona and routine
//! changes save at once with no card; MCP, secret, access, machine and config changes are each a card that needs Face
//! ID and applies exactly what it showed, and a decline changes nothing. Chief asks any persona, reads any persona's
//! history and opens rooms; everyone else keeps `can_message`. The owner can rename Chief but never delete it.

mod support;

use std::{
    fs,
    io::Write as _,
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::Duration,
};

use serde_json::{Value, json};
use support::{Authenticator, Cooee, FakeShed, agent, scratch, version};

struct Setup {
    dir: PathBuf,
    shed: FakeShed,
    cooee: Cooee,
}

const OWN: [&str; 12] = [
    "propose_server",
    "remember",
    "ask_persona",
    "handoff",
    "history",
    "update_persona",
    "create_routine",
    "stop_routine",
    "skill",
    "secret",
    "mail",
    "desk",
];

const CHIEFS: [&str; 9] = [
    "access",
    "config",
    "look",
    "machine",
    "mcp_server",
    "persona",
    "room",
    "routine",
    "stored_secret",
];

fn setup(name: &str) -> Setup {
    let dir = scratch(name);
    agent(
        &dir,
        "chief",
        "name = \"Chief\"\nrole = \"chief\"\nagent = \"fake-acp\"\n",
        "",
    );
    agent(
        &dir,
        "coder",
        "name = \"Coder\"\nagent = \"fake-acp\"\ncan_message = [\"chief\"]\n",
        "",
    );
    agent(
        &dir,
        "finance",
        "name = \"Finance\"\nagent = \"fake-acp\"\n",
        "",
    );
    let shed = FakeShed::start(&dir.join("shed"), version());
    let extra = "[[mcp]]\nname = \"flights\"\ntype = \"http\"\nurl = \"https://mcp.example/flights\"\nenabled = false\n";
    let cooee = Cooee::start_with(&dir, &shed.dir, extra);
    Setup { dir, shed, cooee }
}

fn eventually(mut f: impl FnMut() -> bool) {
    for _ in 0..200 {
        if f() {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("never happened");
}

impl Setup {
    fn start(&self, persona: &str) -> String {
        let new = json!({ "agent": "fake-acp", "profile": persona, "text": "hi", "random_id": format!("start-{persona}") });
        let (status, body) = self.cooee.post("/threads", &new);
        assert_eq!(status, 200, "{body}");
        let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        eventually(|| self.items(&id).iter().any(|i| i["kind"] == "agent"));
        id
    }

    fn items(&self, id: &str) -> Vec<Value> {
        let all = self.cooee.json(&format!("/threads/{id}/messages"));
        all["items"].as_array().unwrap().clone()
    }

    fn entry(&self, id: &str) -> Value {
        let file = self.shed.dir.join(format!("mcp-cooee-{id}.json"));
        let servers: Value = serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap();
        assert_eq!(servers[0]["name"], "cooee");
        servers[0].clone()
    }

    fn mcp(&self, id: &str, method: &str, params: &Value) -> Value {
        let cooee = self.entry(id);
        let args_of = cooee["args"].as_array().unwrap().iter();
        let mut child = Command::new(cooee["command"].as_str().unwrap())
            .args(args_of.map(|a| a.as_str().unwrap()))
            .env("COOEE_MCP", cooee["env"][0]["value"].as_str().unwrap())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let call = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
        writeln!(child.stdin.take().unwrap(), "{call}").unwrap();
        let out = child.wait_with_output().unwrap();
        serde_json::from_slice::<Value>(&out.stdout).unwrap()
    }

    fn tools(&self, id: &str) -> Vec<String> {
        let listed = self.mcp(id, "tools/list", &json!({}));
        let tools = listed["result"]["tools"].as_array().unwrap().iter();
        tools
            .map(|t| t["name"].as_str().unwrap().to_owned())
            .collect()
    }

    fn call(&self, id: &str, tool: &str, arguments: &Value) -> Result<String, String> {
        let got = self.mcp(
            id,
            "tools/call",
            &json!({ "name": tool, "arguments": arguments }),
        );
        let result = &got["result"];
        let text = result["content"][0]["text"].as_str();
        let text = text.map_or_else(|| got["error"]["message"].to_string(), str::to_owned);
        if result["isError"] == json!(false) {
            Ok(text)
        } else {
            Err(text)
        }
    }

    fn direct(&self, id: &str, tool: &str) -> (String, String) {
        let value = self.entry(id)["env"][0]["value"]
            .as_str()
            .unwrap()
            .to_owned();
        let token = value.split_whitespace().nth(1).unwrap().to_owned();
        let body = json!({ "thread": id, "server": { "tool": tool, "arguments": { "what": "personas" } } });
        let bearer = format!("Bearer {token}");
        let headers = [
            ("Authorization", bearer.as_str()),
            ("Content-Type", "application/json"),
        ];
        self.cooee
            .raw("POST", "/mcp/agent/chief", &headers, &body.to_string())
    }

    fn cards(&self, id: &str) -> Vec<Value> {
        let items = self.items(id).into_iter();
        items
            .filter(|i| i["kind"] == "approval" && i["form"].is_string())
            .collect()
    }

    fn card(&self, id: &str) -> Value {
        let open = self.cards(id).into_iter().rev();
        open.clone()
            .find(|c| c["tapped"].is_null())
            .expect("an open card")
    }

    fn approve(&self, id: &str, card: &Value, option: &str, extra: &Value) -> (u16, Value) {
        let mut body = json!({ "item": card["id"], "option": option });
        body.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        let (status, text) = self.cooee.post(&format!("/threads/{id}/approve"), &body);
        (status, serde_json::from_str(&text).unwrap_or(json!(text)))
    }

    fn told(&self, id: &str, line: &str) {
        eventually(|| {
            let sent = self.shed.sent(&format!("cooee-{id}"));
            let queued = self.cooee.json(&format!("/threads/{id}/messages"))["queue"].to_string();
            sent.iter().any(|s| s.contains(line)) || queued.contains(line)
        });
    }

    fn config(&self) -> String {
        fs::read_to_string(self.dir.join("config.toml")).unwrap()
    }
}

#[test]
fn only_chiefs_own_chat_lists_and_runs_its_tools_and_none_answers_or_sends_as_another() {
    let s = setup("chief-gate");
    let chief = s.start("chief");
    let own = OWN
        .iter()
        .filter(|t| !["history", "ask_persona"].contains(t));
    let mut want: Vec<String> = own
        .chain(&CHIEFS)
        .chain(&["history", "ask_persona"])
        .map(|t| (*t).to_owned())
        .collect();
    assert_eq!(s.tools(&chief), want);
    let listed = s.mcp(&chief, "tools/list", &json!({}));
    let history = listed["result"]["tools"].as_array().unwrap().iter();
    let history = history.clone().find(|t| t["name"] == "history").unwrap();
    assert!(
        history["inputSchema"]["properties"]["persona"].is_object(),
        "{history}"
    );
    want.retain(|t| {
        ["approve", "answer", "send", "queue"]
            .iter()
            .any(|w| t.contains(w))
    });
    assert_eq!(want, Vec::<String>::new());

    let coder = s.start("coder");
    assert_eq!(s.tools(&coder), OWN);
    assert!(
        s.call(&coder, "persona", &json!({ "do": "create", "name": "X" }))
            .unwrap_err()
            .starts_with("NOT_CHIEF")
    );
    let (head, body) = s.direct(&coder, "look");
    assert!(
        head.contains(" 400 ") && body.starts_with("NOT_CHIEF"),
        "{head} {body}"
    );
    assert!(s.direct(&chief, "look").1.contains("Finance"));

    let asked = s.call(
        &coder,
        "ask_persona",
        &json!({ "persona": "chief", "message": "hi" }),
    );
    assert_eq!(asked.unwrap(), "Hello");
    let rows = s.cooee.json("/threads");
    let exchange = rows.as_array().unwrap().iter();
    let exchange = exchange
        .clone()
        .find(|r| r["asked_by"] == json!(coder))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(s.tools(&exchange), OWN);
    let (head, body) = s.direct(&exchange, "look");
    assert!(
        head.contains(" 400 ") && body.starts_with("NOT_HERE"),
        "{head} {body}"
    );
}

#[test]
fn a_persona_and_its_routine_are_made_at_once_with_no_card_and_show_in_the_app() {
    let s = setup("chief-free");
    let chief = s.start("chief");
    let made = s.call(
        &chief,
        "persona",
        &json!({ "do": "create", "name": "Travel",
        "character": "A careful travel agent.", "instructions": "Check flight prices." }),
    );
    assert_eq!(
        made.unwrap(),
        "Created persona travel (Travel): agent fake-acp, on this machine. It is on Home."
    );
    let personas = s.cooee.json("/personas");
    let travel = personas
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "travel");
    let travel = travel.expect("travel is listed").clone();
    assert_eq!(
        (&travel["agent"], &travel["pinned"], &travel["instructions"]),
        (
            &json!("fake-acp"),
            &json!(true),
            &json!("Check flight prices.")
        )
    );

    let routine = s.call(
        &chief,
        "routine",
        &json!({ "do": "create", "persona": "travel",
        "name": "Flight prices", "when": "0 8 * * *", "prompt": "Check today's fares" }),
    );
    let said = routine.unwrap();
    assert!(
        said.contains("for travel") && said.contains("each run its own thread"),
        "{said}"
    );
    let list = s.cooee.json("/routines.json");
    let r = &list.as_array().unwrap()[0];
    assert_eq!(
        (&r["name"], &r["place"]["who"]["profile"], &r["made_in"]),
        (&json!("Flight prices"), &json!("travel"), &json!(chief))
    );
    let id = r["id"].as_str().unwrap();
    let paused = s.call(&chief, "routine", &json!({ "do": "pause", "id": id }));
    assert!(paused.unwrap().starts_with("Paused routine Flight prices"));
    assert!(s.cooee.json("/routines.json")[0]["paused"].is_object());

    let items = s.items(&chief);
    assert!(s.cards(&chief).is_empty(), "no card: {items:?}");
    let notes: Vec<&Value> = items
        .iter()
        .map(|i| &i["text"])
        .filter(|t| t.is_string())
        .collect();
    assert!(
        notes.contains(&&json!("Created persona Travel")),
        "{notes:?}"
    );
    assert!(items.iter().any(|i| i["kind"] == "routine"), "{items:?}");

    let edit = json!({ "do": "edit", "id": "travel", "plugins": ["nope"] });
    assert!(
        s.call(&chief, "persona", &edit)
            .unwrap_err()
            .contains("no installed plugin nope")
    );
    let sneaky = json!({ "do": "edit", "id": "travel", "machine": "studio" });
    assert!(
        s.call(&chief, "persona", &sneaky)
            .unwrap_err()
            .starts_with("BAD_ARGS")
    );
    let me = json!({ "do": "archive", "id": "chief" });
    assert!(
        s.call(&chief, "persona", &me)
            .unwrap_err()
            .starts_with("IS_CHIEF")
    );
    s.call(
        &chief,
        "persona",
        &json!({ "do": "archive", "id": "travel" }),
    )
    .unwrap();
    let ids = |v: Value| {
        v.as_array()
            .unwrap()
            .iter()
            .map(|p| p["id"].clone())
            .collect::<Vec<_>>()
    };
    assert!(!ids(s.cooee.json("/personas")).contains(&json!("travel")));
    assert!(s.dir.join("agents/.archive/travel/agent.toml").is_file());
    s.call(
        &chief,
        "persona",
        &json!({ "do": "restore", "id": "travel" }),
    )
    .unwrap();
    assert!(ids(s.cooee.json("/personas")).contains(&json!("travel")));
}

#[test]
fn an_mcp_change_is_a_face_id_card_that_applies_only_when_approved() {
    let s = setup("chief-mcp");
    let chief = s.start("chief");
    let said = s.call(
        &chief,
        "mcp_server",
        &json!({ "do": "on", "name": "flights" }),
    );
    assert!(said.unwrap().starts_with("Asked."));
    let card = s.card(&chief);
    assert_eq!(
        (&card["form"], &card["tool"]),
        (&json!("change"), &json!("turn on MCP server flights"))
    );
    assert!(
        card["detail"]
            .as_str()
            .unwrap()
            .contains("-enabled = false\n+enabled = true")
    );
    let off = s.config();
    assert_eq!(s.approve(&chief, &card, "approve", &json!({})).0, 403);
    assert_eq!(s.config(), off);
    let phone = Authenticator::new(&s.cooee);
    phone.save(&s.cooee);
    let (status, done) = s.approve(
        &chief,
        &card,
        "approve",
        &json!({ "passkey": phone.get(&s.cooee) }),
    );
    assert_eq!(status, 200, "{done}");
    assert_eq!(done["line"], "MCP server flights is on");
    assert!(s.cooee.cli(&["mcp", "ls"]).contains("flights · on"));
    s.told(
        &chief,
        "[cooee] approved: turn on MCP server flights: MCP server flights is on",
    );

    s.call(
        &chief,
        "mcp_server",
        &json!({ "do": "off", "name": "flights" }),
    )
    .unwrap();
    let card = s.card(&chief);
    let on = s.config();
    assert_eq!(s.approve(&chief, &card, "decline", &json!({})).0, 200);
    assert_eq!(s.config(), on);
    s.told(&chief, "[cooee] declined: turn off MCP server flights");
}

#[test]
fn access_and_config_cards_show_the_files_diff_and_write_exactly_it_after_face_id() {
    let s = setup("chief-files");
    let chief = s.start("chief");
    let phone = Authenticator::new(&s.cooee);
    phone.save(&s.cooee);
    let face = || json!({ "passkey": phone.get(&s.cooee) });
    let file = s.dir.join("agents/finance/agent.toml");
    let before = fs::read_to_string(&file).unwrap();
    let ask = json!({ "persona": "finance", "env": { "FLIGHTS_KEY": "secret:FLIGHTS_KEY" }, "mcp": [{ "server": "flights" }] });
    assert!(
        s.call(&chief, "access", &ask)
            .unwrap()
            .starts_with("Asked.")
    );
    let card = s.card(&chief);
    let detail = card["detail"].as_str().unwrap();
    assert!(
        detail.contains("+ [env]")
            && detail.contains("+ FLIGHTS_KEY = \"secret:FLIGHTS_KEY\"")
            && detail.contains("+ server = \"flights\""),
        "{detail}"
    );
    assert_eq!(s.approve(&chief, &card, "approve", &json!({})).0, 403);
    assert_eq!(fs::read_to_string(&file).unwrap(), before);
    assert_eq!(s.approve(&chief, &card, "approve", &face()).0, 200);
    let finance = s.cooee.json("/personas/finance");
    assert_eq!(
        finance["env"],
        json!({ "FLIGHTS_KEY": "secret:FLIGHTS_KEY" })
    );
    assert_eq!(finance["mcp"], json!([{ "server": "flights" }]));
    let bad = json!({ "persona": "finance", "env": { "KEY": format!("tk-{}", "q7".repeat(16)) } });
    assert!(
        s.call(&chief, "access", &bad)
            .unwrap_err()
            .contains("never inline")
    );
    let away = json!({ "persona": "finance", "machine": "studio" });
    assert!(
        s.call(&chief, "access", &away)
            .unwrap_err()
            .starts_with("NO_MACHINE")
    );

    let set = json!({ "set": { "approval_expiry_minutes": 60 } });
    assert!(
        s.call(&chief, "config", &set)
            .unwrap()
            .starts_with("Asked.")
    );
    let card = s.card(&chief);
    assert!(
        card["detail"]
            .as_str()
            .unwrap()
            .contains("+ approval_expiry_minutes = 60")
    );
    let was = s.config();
    assert_eq!(s.approve(&chief, &card, "decline", &json!({})).0, 200);
    assert_eq!(s.config(), was);
    s.call(&chief, "config", &set).unwrap();
    let card = s.card(&chief);
    fs::write(s.dir.join("config.toml"), format!("{was}# mine\n")).unwrap();
    let (_, stale) = s.approve(&chief, &card, "approve", &face());
    assert!(
        stale["line"].as_str().unwrap().contains("changed on disk"),
        "{stale}"
    );
    assert_eq!(s.config(), format!("{was}# mine\n"));
    s.call(&chief, "config", &set).unwrap();
    let card = s.card(&chief);
    assert_eq!(s.approve(&chief, &card, "approve", &face()).0, 200);
    let set = was.replacen("[[mcp]]", "approval_expiry_minutes = 60\n[[mcp]]", 1);
    assert_eq!(s.config(), format!("{set}# mine\n"));

    let tokens = json!({ "set": { "plugins": [] } });
    assert!(
        s.call(&chief, "config", &tokens)
            .unwrap_err()
            .starts_with("BAD_KEY")
    );
    let unknown = json!({ "set": { "colour": "red" } });
    assert!(
        s.call(&chief, "config", &unknown)
            .unwrap_err()
            .starts_with("BAD_CONFIG")
    );
    assert_eq!(s.cards(&chief).len(), 4);
}

#[test]
fn a_secret_and_a_machine_change_wait_on_face_id() {
    let s = setup("chief-secret");
    let chief = s.start("chief");
    let said = s.call(
        &chief,
        "stored_secret",
        &json!({ "do": "set", "name": "FLIGHTS_KEY", "why": "fares" }),
    );
    assert!(said.unwrap().starts_with("Asked."));
    let card = s.card(&chief);
    assert_eq!(
        (&card["form"], &card["detail"]),
        (&json!("secret"), &json!("FLIGHTS_KEY"))
    );
    let typed = json!({ "value": "fl-1234" });
    assert_eq!(s.approve(&chief, &card, "approve", &typed).0, 403);
    assert!(!s.dir.join("data/secrets/FLIGHTS_KEY").exists());
    #[cfg(not(target_os = "macos"))]
    {
        let phone = Authenticator::new(&s.cooee);
        phone.save(&s.cooee);
        let typed = json!({ "value": "fl-1234", "passkey": phone.get(&s.cooee) });
        assert_eq!(s.approve(&chief, &card, "approve", &typed).0, 200);
        let saved = fs::read_to_string(s.dir.join("data/secrets/FLIGHTS_KEY")).unwrap();
        assert_eq!(saved.trim(), "fl-1234");
        let look = s
            .call(&chief, "look", &json!({ "what": "secrets" }))
            .unwrap();
        assert_eq!(look, "[\"FLIGHTS_KEY\"]");
    }

    fs::write(s.shed.dir.join("machines.json"), r#"[{"name":"studio"}]"#).unwrap();
    s.shed.link("studio", "down", 1, Some("asleep"));
    let said = s.call(
        &chief,
        "machine",
        &json!({ "do": "setup", "name": "studio" }),
    );
    assert!(said.unwrap().starts_with("Asked."));
    let card = s.card(&chief);
    assert_eq!(
        (&card["form"], &card["tool"]),
        (&json!("change"), &json!("set up machine studio"))
    );
    assert!(
        card["detail"]
            .as_str()
            .unwrap()
            .contains("shed on studio: down")
    );
    assert_eq!(s.approve(&chief, &card, "approve", &json!({})).0, 403);
    let none = s.call(
        &chief,
        "machine",
        &json!({ "do": "setup", "name": "attic" }),
    );
    assert!(none.unwrap_err().starts_with("NO_MACHINE"));
}

#[test]
fn chief_asks_any_persona_reads_any_history_and_opens_rooms_while_others_keep_can_message() {
    let s = setup("chief-reach");
    let finance = s.start("finance");
    let chief = s.start("chief");
    let asked = s.call(
        &chief,
        "ask_persona",
        &json!({ "persona": "finance", "message": "budget?" }),
    );
    assert_eq!(asked.unwrap(), "Hello");
    let coder = s.start("coder");
    let refused = s.call(
        &coder,
        "ask_persona",
        &json!({ "persona": "finance", "message": "budget?" }),
    );
    assert!(refused.unwrap_err().starts_with("NOT_ALLOWED"));

    let found = s
        .call(
            &chief,
            "history",
            &json!({ "persona": "finance", "query": "hi" }),
        )
        .unwrap();
    assert!(found.contains(&finance), "{found}");
    let read = s
        .call(&chief, "history", &json!({ "thread": finance }))
        .unwrap();
    assert!(read.contains("Hello"), "{read}");
    let theirs = s.call(
        &coder,
        "history",
        &json!({ "persona": "finance", "query": "hi" }),
    );
    assert!(theirs.unwrap_err().starts_with("NOT_CHIEF"));
    let peek = s.call(&coder, "history", &json!({ "thread": finance }));
    assert!(peek.unwrap_err().contains("isn't one of coder's"));

    let room = s.call(&chief, "room", &json!({ "members": ["finance", "coder"], "title": "Trip budget", "why": "to settle June" }));
    let said = room.unwrap();
    let id = said
        .split_whitespace()
        .nth(2)
        .unwrap()
        .trim_end_matches(';');
    let rows = s.cooee.json("/threads");
    let row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == id)
        .unwrap();
    assert_eq!(
        row["members"],
        json!(["chief", "finance", "coder"]),
        "{row}"
    );
    assert!(
        s.items(id)
            .iter()
            .any(|i| i["text"] == "Chief opened this room: to settle June")
    );
}

#[test]
fn chief_can_be_renamed_but_never_deleted_and_no_other_persona_takes_its_role() {
    let s = setup("chief-api");
    let (status, body) = s.cooee.request("DELETE", "/personas/chief", None);
    assert_eq!(
        (status, body.as_str()),
        (400, "Chief can't be deleted: rename it instead.")
    );
    assert!(s.dir.join("agents/chief/agent.toml").is_file());
    let mut chief = s.cooee.json("/personas/chief");
    chief["name"] = json!("Jeeves");
    chief["role"] = json!("");
    let (status, body) = s.cooee.request("PUT", "/personas/chief", Some(&chief));
    assert_eq!(status, 200, "{body}");
    let renamed = s.cooee.json("/personas/chief");
    assert_eq!(
        (&renamed["name"], &renamed["role"]),
        (&json!("Jeeves"), &json!("chief"))
    );

    let mut coder = s.cooee.json("/personas/coder");
    coder["role"] = json!("chief");
    let (status, body) = s.cooee.request("PUT", "/personas/coder", Some(&coder));
    assert_eq!(
        (status, body.as_str()),
        (400, "only cooee's own Chief has that role")
    );
    let new = json!({ "name": "Boss", "agent": "fake-acp", "role": "chief" });
    assert_eq!(s.cooee.post("/personas", &new).0, 400);
    assert_eq!(s.cooee.request("DELETE", "/personas/coder", None).0, 200);
}
