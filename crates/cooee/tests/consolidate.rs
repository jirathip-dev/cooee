//! A persona's consolidation routine (#219) end to end with the fake shed: switched on in its `agent.toml`, a due
//! time with no new owner turn starts nothing; one with them opens a routine-run thread as the persona, whose
//! `history` call reads the new lines once and moves the watermark, whose `remember` and `skill` proposals are cards
//! that change nothing until approved, and the next due time with nothing new starts nothing again.

mod support;

use std::{
    fs,
    io::Write as _,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::Duration,
};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, agent, scratch, version};

fn tool(shed: &FakeShed, id: &str, name: &str, args: &Value) -> (String, bool) {
    let file = shed.dir.join(format!("mcp-cooee-{id}.json"));
    for _ in 0..100 {
        if file.exists() {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    let servers: Value = serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap();
    let cooee = &servers[0];
    let args_of = cooee["args"].as_array().unwrap().iter();
    let mut child = Command::new(cooee["command"].as_str().unwrap())
        .args(args_of.map(|a| a.as_str().unwrap()))
        .env("COOEE_MCP", cooee["env"][0]["value"].as_str().unwrap())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let call = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                       "params": { "name": name, "arguments": args } });
    writeln!(child.stdin.take().unwrap(), "{call}").unwrap();
    let out = child.wait_with_output().unwrap();
    let result = serde_json::from_slice::<Value>(&out.stdout).unwrap()["result"].clone();
    let text = result["content"][0]["text"].as_str().unwrap().to_owned();
    (text, result["isError"] == json!(true))
}

fn marks(dir: &Path) -> PathBuf {
    dir.join("data/watermarks.json")
}

fn mark(dir: &Path) -> Value {
    let text = fs::read_to_string(marks(dir)).unwrap_or_else(|_| "{}".to_owned());
    serde_json::from_str::<Value>(&text).unwrap()["consolidate-ada"].clone()
}

fn due_again(dir: &Path) -> Value {
    let mut m = mark(dir);
    if m.is_null() {
        m = json!({ "line": 0 });
    }
    m["due"] = json!(1);
    fs::write(marks(dir), json!({ "consolidate-ada": m }).to_string()).unwrap();
    m
}

fn ticked(dir: &Path) {
    for _ in 0..100 {
        if mark(dir)["due"].as_u64().unwrap_or(0) > 1 {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
    panic!("no tick: {}", mark(dir));
}

fn runs(cooee: &Cooee) -> Vec<String> {
    let rows = cooee.json("/threads");
    let rows = rows.as_array().unwrap().iter();
    let rows = rows.filter(|r| r["routine"] == "Consolidation");
    rows.map(|r| r["id"].as_str().unwrap().to_owned()).collect()
}

fn card(cooee: &Cooee, id: &str, form: &str) -> Value {
    let items = cooee.json(&format!("/threads/{id}/messages"));
    let mut cards = items["items"].as_array().unwrap().iter();
    let found = cards.rfind(|i| i["form"] == form);
    found
        .unwrap_or_else(|| panic!("no {form} card: {items}"))
        .clone()
}

fn approve(cooee: &Cooee, id: &str, card: &Value) {
    let tap = json!({ "item": card["id"], "option": "approve" });
    let (status, body) = cooee.post(&format!("/threads/{id}/approve"), &tap);
    assert_eq!(status, 200, "{body}");
}

#[test]
fn a_run_reads_new_history_once_and_its_writes_wait_for_approval() {
    let dir = scratch("consolidate");
    let toml = "name = \"Ada\"\nagent = \"fake-acp\"\n\n[consolidate]\non = true\n";
    let home = agent(&dir, "ada", toml, "");
    let shed = FakeShed::start(&dir.join("shed"), version());

    let cooee = Cooee::start(&dir, &shed.dir);
    for _ in 0..100 {
        if !mark(&dir).is_null() {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(mark(&dir)["line"], 0, "first sight only notes the due time");
    drop(cooee);
    due_again(&dir);
    let cooee = Cooee::start(&dir, &shed.dir);
    ticked(&dir);
    assert!(runs(&cooee).is_empty(), "nothing new: no run");

    let new = json!({ "agent": "fake-acp", "profile": "ada", "text": "the kiwi order ships Fridays", "random_id": "k" });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    drop(cooee);
    due_again(&dir);
    let cooee = Cooee::start(&dir, &shed.dir);
    ticked(&dir);
    let mut ran = Vec::new();
    for _ in 0..100 {
        ran = runs(&cooee);
        if !ran.is_empty() {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(ran.len(), 1, "{ran:?}");
    let run = ran[0].clone();
    let sent = first_prompts(&shed, &run);
    assert!(sent.contains("consolidation run"), "{sent}");

    let (read, failed) = tool(&shed, &run, "history", &json!({}));
    assert!(!failed, "{read}");
    assert!(
        read.contains("owner: the kiwi order ships Fridays"),
        "{read}"
    );
    assert!(!read.contains("consolidation run"), "{read}");
    let line = mark(&dir)["line"].as_u64().unwrap();
    assert!(line > 0, "{}", mark(&dir));
    let again = tool(&shed, &run, "history", &json!({}));
    assert_eq!(again, ("nothing new since the last run".to_owned(), false));
    assert_memory_and_skill_wait_for_approval(&cooee, &shed, &run, &home);

    drop(cooee);
    let m = due_again(&dir);
    assert_eq!(m["line"], line);
    let cooee = Cooee::start(&dir, &shed.dir);
    ticked(&dir);
    thread::sleep(Duration::from_millis(500));
    assert_eq!(runs(&cooee), vec![run], "nothing new since: no second run");
    assert_eq!(mark(&dir)["line"], line, "never reread");
}

fn assert_memory_and_skill_wait_for_approval(
    cooee: &Cooee,
    shed: &FakeShed,
    run: &str,
    home: &Path,
) {
    let note = json!({ "text": "Kiwi orders ship on Fridays." });
    let (said, failed) = tool(shed, run, "remember", &note);
    assert!(!failed && said.starts_with("Proposed."), "{said}");
    let memory = home.join("memory.md");
    assert!(!memory.exists(), "nothing written before approval");
    let notes = card(cooee, run, "memory");
    assert_eq!(notes["detail"], "Kiwi orders ship on Fridays.", "{notes}");
    let skill = json!({ "propose": { "name": "ship-kiwi", "description": "How I ship a kiwi order.",
                                     "body": "1. Pack it.\n2. Ship it Friday." } });
    let (said, failed) = tool(shed, run, "skill", &skill);
    assert!(!failed && said.starts_with("Proposed."), "{said}");
    let skill_file = home.join("skills/ship-kiwi/SKILL.md");
    assert!(!skill_file.exists(), "nothing written before approval");
    approve(cooee, run, &notes);
    let kept = fs::read_to_string(&memory).unwrap();
    assert!(kept.contains("Kiwi orders ship on Fridays."), "{kept}");
    approve(cooee, run, &card(cooee, run, "skill"));
    assert!(
        skill_file.exists(),
        "{}",
        cooee.json(&format!("/threads/{run}/messages"))
    );
}

fn first_prompts(shed: &FakeShed, run: &str) -> String {
    let mut sent = String::new();
    for _ in 0..100 {
        sent = shed.sent(&format!("cooee-{run}")).join("\n");
        if !sent.is_empty() {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    sent
}
