//! `ask_persona` (#156) end to end against the fake shed, called the way an agent calls it: through cooee's own MCP
//! server with its thread's token. A persona asks one its `can_message` names and gets the reply as the tool's
//! result, in a linked exchange thread the owner reads and steers; anyone else is `NOT_ALLOWED`; an asked persona
//! may ask one more and no further (`DEPTH`, G4); no reply in time is `TIMEOUT`; a session that can't start is
//! `OFFLINE`.

mod support;

use std::{
    fs,
    io::Write as _,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, scratch, version};

struct Setup {
    shed: FakeShed,
    cooee: Cooee,
}

fn setup(name: &str) -> Setup {
    let dir = scratch(name);
    fs::create_dir_all(dir.join("personas")).unwrap();
    for (n, may) in [
        ("Ada", "bo"),
        ("Bo", "cy"),
        ("Cy", "ada\", \"ed"),
        ("Di", ""),
    ] {
        let may = if may.is_empty() {
            String::new()
        } else {
            format!("\"{may}\"")
        };
        let file = dir.join(format!("personas/{}.toml", n.to_lowercase()));
        let toml = format!("name = \"{n}\"\nagent = \"fake-acp\"\ncan_message = [{may}]\n");
        fs::write(file, toml).unwrap();
    }
    let away = dir.join("root/away");
    fs::create_dir_all(&away).unwrap();
    let ed = format!(
        "name = \"Ed\"\nagent = \"fake-acp\"\nfolder = {:?}\n",
        away.to_string_lossy()
    );
    fs::write(dir.join("personas/ed.toml"), ed).unwrap();
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    Setup { shed, cooee }
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
        let new =
            json!({ "agent": "fake-acp", "profile": persona, "text": "hi", "random_id": persona });
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

    fn ask(&self, id: &str, args: &Value) -> (String, bool) {
        let file = self.shed.dir.join(format!("mcp-cooee-{id}.json"));
        let servers: Value = serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap();
        let cooee = &servers[0];
        assert_eq!(cooee["name"], "cooee");
        let args_of = cooee["args"].as_array().unwrap().iter();
        let mut child = Command::new(cooee["command"].as_str().unwrap())
            .args(args_of.map(|a| a.as_str().unwrap()))
            .env("COOEE_MCP", cooee["env"][0]["value"].as_str().unwrap())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let call = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                           "params": { "name": "ask_persona", "arguments": args } });
        writeln!(child.stdin.take().unwrap(), "{call}").unwrap();
        let out = child.wait_with_output().unwrap();
        let result = serde_json::from_slice::<Value>(&out.stdout).unwrap()["result"].clone();
        let text = result["content"][0]["text"].as_str().unwrap().to_owned();
        (text, result["isError"] == json!(true))
    }

    fn linked(&self, id: &str) -> Vec<String> {
        let items = self.items(id);
        let links = items.iter().filter(|i| i["kind"] == "link");
        links
            .map(|i| i["thread"].as_str().unwrap().to_owned())
            .collect()
    }

    fn threads(&self) -> usize {
        self.cooee.json("/threads").as_array().unwrap().len()
    }
}

#[test]
fn an_allowed_ask_returns_the_reply_in_a_linked_thread_the_owner_reads_and_steers() {
    let s = setup("ask-allowed");
    let ada = s.start("ada");
    let (reply, failed) = s.ask(
        &ada,
        &json!({ "persona": "bo", "message": "what is for dinner?" }),
    );
    assert_eq!((reply.as_str(), failed), ("Hello", false));

    let linked = s.linked(&ada);
    assert_eq!(linked.len(), 1, "{:?}", s.items(&ada));
    let x = &linked[0].clone();
    let rows = s.cooee.json("/threads");
    assert!(
        rows.as_array().unwrap().iter().any(|r| &r["id"] == x),
        "{rows}"
    );
    let said: Vec<(Value, Value)> = s
        .items(x)
        .iter()
        .map(|i| (i["kind"].clone(), i["text"].clone()))
        .collect();
    assert_eq!(
        said,
        [
            (json!("you"), json!("[Ada]: what is for dinner?")),
            (json!("agent"), json!("Hello"))
        ]
    );

    let (reply, _) = s.ask(&ada, &json!({ "persona": "bo", "message": "and dessert?" }));
    assert_eq!(reply, "Hello");
    assert_eq!((s.linked(&ada), s.threads()), (linked, 2));
    let replies = s.items(x).iter().filter(|i| i["kind"] == "agent").count();
    assert_eq!(replies, 2);

    let steer = json!({ "text": "keep it short", "random_id": "steer" });
    assert_eq!(s.cooee.post(&format!("/threads/{x}/send"), &steer).0, 200);
    eventually(|| {
        s.shed
            .sent(&format!("cooee-{x}"))
            .iter()
            .any(|p| p.contains("keep it short"))
    });
}

#[test]
fn a_persona_its_can_message_doesnt_name_is_not_allowed() {
    let s = setup("ask-refused");
    let ada = s.start("ada");
    let (why, failed) = s.ask(&ada, &json!({ "persona": "cy", "message": "hi" }));
    assert!(failed && why.starts_with("NOT_ALLOWED: "), "{why}");
    let di = s.start("di");
    let (why, failed) = s.ask(&di, &json!({ "persona": "bo", "message": "hi" }));
    assert!(failed && why.starts_with("NOT_ALLOWED: "), "{why}");
    assert_eq!(s.threads(), 2);
    assert!(s.linked(&ada).is_empty() && s.linked(&di).is_empty());
}

#[test]
fn an_asked_persona_may_ask_one_more_and_no_further() {
    let s = setup("ask-depth");
    let ada = s.start("ada");
    s.ask(&ada, &json!({ "persona": "bo", "message": "ask Cy" }));
    let bo = s.linked(&ada).remove(0);
    let (reply, failed) = s.ask(&bo, &json!({ "persona": "cy", "message": "hi" }));
    assert_eq!((reply.as_str(), failed), ("Hello", false));
    let cy = s.linked(&bo).remove(0);
    let (why, failed) = s.ask(&cy, &json!({ "persona": "ada", "message": "hi" }));
    assert!(failed && why.starts_with("DEPTH: "), "{why}");
    assert_eq!(s.linked(&cy), Vec::<String>::new());
}

#[test]
fn no_reply_in_time_is_a_timeout_and_a_session_that_cant_start_is_offline() {
    let s = setup("ask-timeout");
    let ada = s.start("ada");
    let at = Instant::now();
    let (why, failed) = s.ask(
        &ada,
        &json!({ "persona": "bo", "message": "hang on", "timeout": 1 }),
    );
    assert!(failed && why.starts_with("TIMEOUT: "), "{why}");
    assert!(at.elapsed() >= Duration::from_secs(1));

    let cy = s.start("cy");
    s.shed.gate("fails-in-away", true);
    let (why, failed) = s.ask(&cy, &json!({ "persona": "ed", "message": "hi" }));
    assert!(failed && why.starts_with("OFFLINE: "), "{why}");
}
