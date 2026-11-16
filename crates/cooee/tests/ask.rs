//! `ask_persona` (#156) end to end against the fake shed, called the way an agent calls it: through cooee's own MCP
//! server with its thread's token. A persona asks one its `can_message` names and gets the reply as the tool's
//! result, in a linked exchange thread the owner reads and steers; anyone else is `NOT_ALLOWED`; an asked persona
//! may ask one more and no further (`DEPTH`, G4); no reply in time is `TIMEOUT`; a session that can't start is
//! `OFFLINE`. Each ask is a card in the asker's chat; the owner's note mid-exchange reaches the asked persona marked as
//! the owner's and goes back to the asker with the reply (#177).

mod support;

use std::{
    fs,
    io::Write as _,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, agent, scratch, version};

struct Setup {
    shed: FakeShed,
    cooee: Cooee,
}

fn setup(name: &str) -> Setup {
    let dir = scratch(name);
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
        let toml = format!("name = \"{n}\"\nagent = \"fake-acp\"\ncan_message = [{may}]\n");
        agent(&dir, &n.to_lowercase(), &toml, "");
    }
    let away = dir.join("root/away");
    fs::create_dir_all(&away).unwrap();
    let ed = format!(
        "name = \"Ed\"\nagent = \"fake-acp\"\nfolder = {:?}\n",
        away.to_string_lossy()
    );
    agent(&dir, "ed", &ed, "");
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

    fn cards(&self, id: &str) -> Vec<Value> {
        let items = self.items(id);
        items.into_iter().filter(|i| i["kind"] == "ask").collect()
    }

    fn linked(&self, id: &str) -> Vec<String> {
        let mut linked = Vec::new();
        for card in self.cards(id) {
            let x = card["thread"].as_str().unwrap().to_owned();
            if !x.is_empty() && !linked.contains(&x) {
                linked.push(x);
            }
        }
        linked
    }

    fn outcomes(&self, id: &str) -> Vec<(Value, Value)> {
        let cards = self.cards(id);
        cards
            .iter()
            .map(|c| (c["persona"].clone(), c["outcome"].clone()))
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

    let card = &s.cards(&ada)[0];
    let asked = (
        &card["persona"],
        &card["name"],
        &card["text"],
        &card["outcome"],
    );
    assert_eq!(
        asked,
        (
            &json!("bo"),
            &json!("Bo"),
            &json!("what is for dinner?"),
            &json!("replied")
        )
    );
    assert!(card.get("note").is_none(), "{card}");

    let linked = s.linked(&ada);
    assert_eq!(linked.len(), 1, "{:?}", s.items(&ada));
    let x = &linked[0].clone();
    let rows = s.cooee.json("/threads");
    let row = rows.as_array().unwrap().iter().find(|r| &r["id"] == x);
    assert_eq!(row.map(|r| &r["asked_by"]), Some(&json!(ada)), "{rows}");
    let items = s.items(x);
    let said: Vec<(Value, Value, Value)> = items
        .iter()
        .map(|i| (i["kind"].clone(), i["speaker"].clone(), i["text"].clone()))
        .collect();
    assert_eq!(
        said[..2],
        [
            (json!("you"), json!("ada"), json!("what is for dinner?")),
            (json!("agent"), Value::Null, json!("Hello"))
        ]
    );
    let back = said[2].2.as_str().unwrap();
    assert!(
        said[2].0 == "note" && back.starts_with("returned to Ada · "),
        "{said:?}"
    );
    let prompts = s.shed.sent(&format!("cooee-{x}"));
    let framed = r#"\n\n[Ada]:\n\nwhat is for dinner?"}"#;
    assert!(prompts[0].contains(framed), "{prompts:?}");

    let (reply, _) = s.ask(&ada, &json!({ "persona": "bo", "message": "and dessert?" }));
    assert_eq!(reply, "Hello");
    assert_eq!((s.linked(&ada), s.threads()), (linked, 2));
    let replies = s.items(x).iter().filter(|i| i["kind"] == "agent").count();
    assert_eq!(replies, 2);

    let steer = json!({ "text": "keep it short", "random_id": "steer" });
    assert_eq!(s.cooee.post(&format!("/threads/{x}/send"), &steer).0, 200);
    let marked = r#""[The owner's note; the asker gets it with your reply]:\n\nkeep it short""#;
    eventually(|| {
        s.shed
            .sent(&format!("cooee-{x}"))
            .iter()
            .any(|p| p.contains(marked))
    });
}

#[test]
fn the_owners_note_mid_exchange_reaches_the_asked_persona_and_returns_to_the_asker_with_the_reply()
{
    let s = setup("ask-note");
    let ada = s.start("ada");
    let message = json!({ "persona": "bo", "message": "wait for a note", "timeout": 10 });
    let (x, waiting, (reply, failed)) = thread::scope(|scope| {
        let asking = scope.spawn(|| s.ask(&ada, &message));
        eventually(|| !s.linked(&ada).is_empty());
        let x = s.linked(&ada).remove(0);
        let waiting = s.outcomes(&ada);
        let note = json!({ "text": "Use the 10-Q", "random_id": "note" });
        assert_eq!(s.cooee.post(&format!("/threads/{x}/send"), &note).0, 200);
        (x, waiting, asking.join().unwrap())
    });
    assert_eq!(waiting, [(json!("bo"), json!("asking"))]);

    let marked = "[The owner's note; the asker gets it with your reply]:\n\nUse the 10-Q";
    let want = format!("noted: {marked}\n\n[The owner's note to Bo: Use the 10-Q]");
    assert_eq!((reply, failed), (want, false));
    let card = &s.cards(&ada)[0];
    assert_eq!(
        (&card["outcome"], &card["note"]),
        (&json!("replied"), &json!("Use the 10-Q"))
    );

    let items = s.items(&x);
    let note = items.iter().find(|i| i["text"] == "Use the 10-Q").unwrap();
    assert!(
        note["kind"] == "you" && note.get("speaker").is_none(),
        "{note}"
    );
    let back = items.last().unwrap()["text"].as_str().unwrap();
    assert!(
        back.starts_with("returned to Ada with your note · "),
        "{back}"
    );
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
    assert_eq!(s.outcomes(&ada), [(json!("cy"), json!("not_allowed"))]);
    assert_eq!(s.outcomes(&di), [(json!("bo"), json!("not_allowed"))]);
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
    assert_eq!(s.outcomes(&cy), [(json!("ada"), json!("depth"))]);
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
    assert_eq!(s.outcomes(&ada), [(json!("bo"), json!("timeout"))]);

    let cy = s.start("cy");
    s.shed.gate("fails-in-away", true);
    let (why, failed) = s.ask(&cy, &json!({ "persona": "ed", "message": "hi" }));
    assert!(failed && why.starts_with("OFFLINE: "), "{why}");
    assert_eq!(s.outcomes(&cy), [(json!("ed"), json!("offline"))]);
}
