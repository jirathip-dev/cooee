use std::{
    fs,
    io::Write as _,
    path::PathBuf,
    process::{Command, Stdio},
};

use serde_json::{Value, json};

use crate::support::{Cooee, FakeShed, agent, scratch, version};

pub struct Setup {
    pub dir: PathBuf,
    pub shed: FakeShed,
    pub cooee: Cooee,
}

pub fn setup(name: &str) -> Setup {
    let dir = scratch(name);
    let toml = "name = \"Coder\"\nagent = \"fake-acp\"\npolicy = \"allow\"\navatar = \"idle\"\n\
                [env]\nMODE = \"review\"\n";
    agent(&dir, "coder", toml, "Follow the house rules.\n");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    Setup { dir, shed, cooee }
}

impl Setup {
    pub fn call(&self, method: &str, path: &str, body: Option<&Value>) -> (u16, Value) {
        let (status, text) = self.cooee.request(method, path, body);
        (
            status,
            serde_json::from_str(&text).unwrap_or(Value::String(text)),
        )
    }

    pub fn start(&self, persona: &str) -> (String, String, String) {
        let random = format!("start-{}", self.shed.cli_log().len());
        let new =
            json!({ "agent": "other-acp", "profile": persona, "text": "hi", "random_id": random });
        let (status, body) = self.call("POST", "/threads", Some(&new));
        assert_eq!(status, 200, "{body}");
        let id = body["id"].as_str().unwrap().to_owned();
        let log = self.shed.cli_log();
        let line = log
            .lines()
            .rfind(|l| l.contains(" run "))
            .unwrap()
            .to_owned();
        assert!(line.contains(&format!("cooee-{id}")), "{line}");
        let on = line.split("--on ").nth(1).and_then(|r| r.split(' ').next());
        let terminal = on.map_or(format!("cooee-{id}"), |m| format!("{m}/cooee-{id}"));
        let sent = self.shed.sent(&terminal);
        let prompt: Value = serde_json::from_str(sent[0].trim_start_matches('\x1e')).unwrap();
        let prompt = prompt["prompt"].as_str().unwrap();
        let topic = prompt.strip_prefix("This topic: hi (folder ");
        let brief = topic.and_then(|t| t.split_once(").\n\n"));
        let brief = brief.map_or(prompt, |b| b.1);
        assert_eq!(persona.is_empty(), brief == prompt, "{prompt}");
        (id, line, brief.to_owned())
    }

    pub fn row(&self, id: &str) -> Value {
        let rows = self.cooee.json("/threads");
        let row = rows.as_array().unwrap().iter().find(|r| r["id"] == id);
        row.unwrap_or_else(|| panic!("no row {id}: {rows}")).clone()
    }

    pub fn row_persona(&self, id: &str) -> Value {
        self.row(id)["persona"].clone()
    }

    pub fn remember(&self, id: &str, args: &Value) -> Value {
        self.tool((id, id), "remember", args)
    }

    pub fn tool(&self, (id, thread): (&str, &str), name: &str, args: &Value) -> Value {
        let file = self.shed.dir.join(format!("mcp-cooee-{id}.json"));
        let servers: Value = serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap();
        let cooee = &servers[0];
        assert_eq!(cooee["name"], "cooee");
        let args_of = cooee["args"].as_array().unwrap().iter();
        let mut child = Command::new(cooee["command"].as_str().unwrap())
            .args(args_of.map(|a| a.as_str().unwrap().replace(id, thread)))
            .env("COOEE_MCP", cooee["env"][0]["value"].as_str().unwrap())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let call = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                           "params": { "name": name, "arguments": args } });
        writeln!(child.stdin.take().unwrap(), "{call}").unwrap();
        let out = child.wait_with_output().unwrap();
        serde_json::from_slice::<Value>(&out.stdout).unwrap()["result"].clone()
    }
}

pub fn install_drover(s: &Setup) {
    let lock = json!([{ "name": "drover", "url": "https://example.com/drover", "commit": "c0",
                        "version": "0.1.0", "tabs": ["queue"], "port": 1, "enabled": false }]);
    fs::write(s.dir.join("data/plugins.lock"), lock.to_string()).unwrap();
}

pub fn old_install(name: &str, drover: bool, old: &[(&str, &str)]) -> Setup {
    let dir = scratch(name);
    for (file, text) in old {
        fs::create_dir_all(dir.join(file).parent().unwrap()).unwrap();
        fs::write(dir.join(file), text).unwrap();
    }
    if drover {
        let lock = json!([{ "name": "drover", "url": "https://example.com/drover", "commit": "c0",
                            "version": "0.1.0", "tabs": ["queue"], "port": 1, "enabled": false }]);
        fs::create_dir_all(dir.join("data")).unwrap();
        fs::write(dir.join("data/plugins.lock"), lock.to_string()).unwrap();
    }
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    Setup { dir, shed, cooee }
}

pub fn cook(s: &Setup) -> String {
    let new = json!({ "name": "Cook", "agent": "fake-acp", "folder": "scratch" });
    assert_eq!(s.call("POST", "/personas", Some(&new)).0, 200);
    s.start("cook").0
}

pub fn memory(s: &Setup) -> Vec<Value> {
    let notes = s.cooee.json("/personas/cook/memory");
    notes.as_array().unwrap().clone()
}

pub fn items(s: &Setup, id: &str) -> Vec<Value> {
    let got = s.cooee.json(&format!("/threads/{id}/messages"));
    got["items"].as_array().unwrap().clone()
}

pub fn cook_text(s: &Setup) -> (Value, Value) {
    let p = s.cooee.json("/personas/cook");
    (p["character"].clone(), p["instructions"].clone())
}

pub fn say(s: &Setup, id: &str, text: &str, via: Option<&str>) -> (Value, String) {
    let mut m = json!({ "text": text, "random_id": format!("say-{text}") });
    if let Some(v) = via {
        m["via"] = json!(v);
    }
    let (status, item) = s.call("POST", &format!("/threads/{id}/send"), Some(&m));
    assert_eq!(status, 200, "{item}");
    let sent = s.shed.sent(&format!("cooee-{id}"));
    let prompt: Value =
        serde_json::from_str(sent.last().unwrap().trim_start_matches('\x1e')).unwrap();
    (item, prompt["prompt"].as_str().unwrap().to_owned())
}
