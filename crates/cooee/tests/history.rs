//! `history` (#217) end to end against the fake shed, called the way an agent calls it: through cooee's own MCP server
//! with its thread's token. A persona finds and reads its own past threads (owner text and final replies, no tool
//! rows); another persona's thread is refused.

mod support;

use std::{
    fs,
    io::Write as _,
    process::{Command, Stdio},
    thread,
    time::Duration,
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
    for n in ["Ada", "Bo"] {
        let file = dir.join(format!("personas/{}.toml", n.to_lowercase()));
        fs::write(file, format!("name = \"{n}\"\nagent = \"fake-acp\"\n")).unwrap();
    }
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    Setup { shed, cooee }
}

impl Setup {
    /// A thread of `persona`'s started with `text`, its first reply in.
    fn start(&self, persona: &str, text: &str) -> String {
        let new =
            json!({ "agent": "fake-acp", "profile": persona, "text": text, "random_id": text });
        let (status, body) = self.cooee.post("/threads", &new);
        assert_eq!(status, 200, "{body}");
        let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        for _ in 0..200 {
            let all = self.cooee.json(&format!("/threads/{id}/messages"));
            if all["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|i| i["kind"] == "agent")
            {
                return id;
            }
            thread::sleep(Duration::from_millis(50));
        }
        panic!("no reply in {id}");
    }

    /// `history` called by thread `id`'s agent through cooee's own MCP server: the tool's text and whether it is an
    /// error.
    fn history(&self, id: &str, args: &Value) -> (String, bool) {
        let file = self.shed.dir.join(format!("mcp-cooee-{id}.json"));
        let servers: Value = serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap();
        let cooee = &servers[0];
        let mut child = Command::new(cooee["command"].as_str().unwrap())
            .args(
                cooee["args"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|a| a.as_str().unwrap()),
            )
            .env("COOEE_MCP", cooee["env"][0]["value"].as_str().unwrap())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let call = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                           "params": { "name": "history", "arguments": args } });
        writeln!(child.stdin.take().unwrap(), "{call}").unwrap();
        let out = child.wait_with_output().unwrap();
        let result = serde_json::from_slice::<Value>(&out.stdout).unwrap()["result"].clone();
        let text = result["content"][0]["text"].as_str().unwrap().to_owned();
        (text, result["isError"] == json!(true))
    }
}

#[test]
fn a_persona_finds_and_reads_its_own_threads_and_not_another_personas() {
    let s = setup("history");
    let past = s.start("ada", "use the tool on the kiwi notes");
    let bo = s.start("bo", "kiwi for Bo only");
    let now = s.start("ada", "hi");

    let (found, failed) = s.history(&now, &json!({ "query": "KIWI notes" }));
    assert!(!failed, "{found}");
    assert_eq!(found.lines().count(), 1, "{found}");
    assert!(found.starts_with(&format!("{past} ")), "{found}");
    assert!(found.contains("use the tool on the kiwi notes"), "{found}");

    let (read, failed) = s.history(&now, &json!({ "thread": past }));
    assert!(!failed, "{read}");
    assert!(
        read.contains("owner: use the tool on the kiwi notes"),
        "{read}"
    );
    assert!(read.contains("reply: Read it."), "{read}");
    assert!(!read.contains("Read src/lib.rs"), "a tool row: {read}");

    let (refused, failed) = s.history(&now, &json!({ "thread": bo }));
    assert!(failed, "{refused}");
    assert_eq!(refused, format!("thread {bo} isn't one of ada's"));
}
