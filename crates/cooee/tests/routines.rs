//! Routines (#75) end to end with the fake shed (tests/support): an agent makes one in chat with cooee's own
//! `create_routine` tool (its chat gets the routine line, Settings › routines lists it; a schedule under 15 minutes
//! apart is refused). A due time that passed runs once, into that chat, marked with the routine in the chat and the
//! thread list; a restart runs nothing again, and a run whose record was lost (a crash between the run and its
//! record) adopts the same message instead of sending a second. Settings pauses and deletes it.

mod support;

use std::{
    fs,
    io::Write as _,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, scratch, version};

fn call(shed: &FakeShed, id: &str, name: &str, args: &Value) -> Value {
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
    serde_json::from_slice::<Value>(&out.stdout).unwrap()["result"].clone()
}

fn change(file: &Path, f: impl FnOnce(&mut Vec<Value>)) {
    let mut v: Value = serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap();
    f(v["routines"].as_array_mut().unwrap());
    fs::write(file, v.to_string()).unwrap();
}

fn recorded(file: &Path, id: &str) -> Value {
    for _ in 0..100 {
        let v: Value = serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap();
        let r = v["routines"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == id);
        if let Some(r) = r.filter(|r| r["last_due"].as_u64().unwrap_or(0) > 0) {
            return r.clone();
        }
        thread::sleep(Duration::from_millis(100));
    }
    panic!(
        "routine {id} never ran: {}",
        fs::read_to_string(file).unwrap()
    );
}

fn runs(cooee: &Cooee, id: &str) -> Vec<Value> {
    let items = cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
    let items = items.as_array().unwrap().iter();
    items
        .filter(|i| i["kind"] == "you" && i["routine"].is_object())
        .cloned()
        .collect()
}

fn chat(name: &str) -> (PathBuf, FakeShed, Cooee, String) {
    let dir = scratch(name);
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    let new = json!({ "agent": "fake-acp", "folder": dir.join("root/project"), "text": "hi", "random_id": "start" });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    (dir, shed, cooee, id)
}

fn ago(days: u64) -> u64 {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    u64::try_from(now.as_millis()).unwrap() - days * 86_400_000
}

fn form(cooee: &Cooee, path: &str, body: &str) -> String {
    let headers = [
        ("Content-Type", "application/x-www-form-urlencoded"),
        ("Cookie", cooee.cookie.as_str()),
    ];
    cooee.raw("POST", path, &headers, body).0[9..12].to_owned()
}

#[test]
fn a_schedule_made_in_chat_runs_once_per_due_time_even_across_restarts() {
    let (dir, shed, cooee, id) = chat("routines");
    let terminal = format!("cooee-{id}");
    shed.report(&terminal, "done", None);

    let often = json!({ "name": "Ping", "when": "*/5 * * * *", "prompt": "ping" });
    let refused = call(&shed, &id, "create_routine", &often);
    assert_eq!(refused["isError"], true, "{refused}");
    let said = refused["content"][0]["text"].as_str().unwrap();
    assert!(said.starts_with("BAD_SCHEDULE"), "{said}");
    let ask = json!({ "name": "Monday renewals", "when": "0 9 * * 1", "prompt": "Renewals due {{due}}?" });
    let made = call(&shed, &id, "create_routine", &ask);
    assert_eq!(made["isError"], false, "{made}");
    let items = cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
    let line = items
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["kind"] == "routine");
    let line = line.unwrap_or_else(|| panic!("no routine line: {items}"));
    assert_eq!(
        (&line["name"], &line["when"], &line["place"]),
        (
            &json!("Monday renewals"),
            &json!("Mondays 09:00"),
            &json!("in this chat")
        )
    );
    let list = cooee.json("/routines.json");
    assert_eq!(list.as_array().unwrap().len(), 1, "{list}");
    let rid = list[0]["id"].as_str().unwrap().to_owned();
    assert_eq!(
        (&list[0]["made_in"], &list[0]["last_due"]),
        (&json!(id), &json!(0))
    );
    let (status, page) = cooee.get("/routines");
    assert_eq!(status, 200);
    assert!(
        page.contains("Monday renewals") && page.contains("Mondays 09:00"),
        "{page}"
    );
    drop(cooee);

    let eight_days_ago = ago(8);
    let file = dir.join("data/routines.json");
    change(&file, |list| list[0]["made_at"] = json!(eight_days_ago));
    let cooee = Cooee::start(&dir, &shed.dir);
    let ran = recorded(&file, &rid);
    let due = ran["last_due"].as_u64().unwrap();
    assert!(due > eight_days_ago, "{ran}");
    let run = &ran["last_run"];
    assert_eq!(
        (&run["outcome"], &run["thread"], &run["key"]),
        (
            &json!("ran"),
            &json!(id),
            &json!(format!("routine-{rid}-{due}"))
        )
    );
    let sent = runs(&cooee, &id);
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert_eq!(sent[0]["routine"]["name"], "Monday renewals");
    let text = sent[0]["text"].as_str().unwrap();
    assert!(
        text.starts_with("Renewals due 20") && !text.contains("{{"),
        "{text}"
    );
    let rows = cooee.json("/threads");
    let row = rows.as_array().unwrap().iter().find(|r| r["id"] == id);
    assert_eq!(row.unwrap()["routine"], "Monday renewals", "{rows}");
    let prompts = || {
        let sent = shed.sent(&terminal);
        sent.iter().filter(|t| t.contains("Renewals due")).count()
    };
    assert_eq!(prompts(), 1);
    drop(cooee);

    change(&file, |list| {
        let mut other = list[0].clone();
        other["id"] = json!("r-other");
        other["prompt"] = json!("Other due {{due}}");
        other["last_due"] = json!(0);
        other.as_object_mut().unwrap().remove("last_run");
        list.push(other);
    });
    let cooee = Cooee::start(&dir, &shed.dir);
    recorded(&file, "r-other");
    assert_eq!(recorded(&file, &rid), ran);
    assert_eq!((runs(&cooee, &id).len(), prompts()), (2, 1));
    drop(cooee);

    change(&file, |list| {
        list[0]["last_due"] = json!(0);
        list[0].as_object_mut().unwrap().remove("last_run");
    });
    let cooee = Cooee::start(&dir, &shed.dir);
    assert_eq!(recorded(&file, &rid)["last_due"], due);
    let mine = runs(&cooee, &id);
    let mine = mine.iter().filter(|i| i["routine"]["id"] == rid.as_str());
    assert_eq!((mine.count(), prompts()), (1, 1));
}

#[test]
fn a_new_thread_routine_starts_a_thread_marked_with_it_in_the_list() {
    let (dir, shed, cooee, id) = chat("routines-new");
    let ask = json!({ "name": "Daily digest", "when": "30 18 * * *", "prompt": "Digest for {{due}}", "place": "new_thread" });
    let made = call(&shed, &id, "create_routine", &ask);
    assert_eq!(made["isError"], false, "{made}");
    let rid = cooee.json("/routines.json")[0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    drop(cooee);
    let file = dir.join("data/routines.json");
    change(&file, |list| list[0]["made_at"] = json!(ago(2)));
    let cooee = Cooee::start(&dir, &shed.dir);
    let ran = recorded(&file, &rid);
    let started = ran["last_run"]["thread"].as_str().unwrap().to_owned();
    assert_ne!(started, id, "{ran}");
    let rows = cooee.json("/threads");
    let row = |t: &str| {
        rows.as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == t)
            .cloned()
    };
    assert_eq!(row(&started).unwrap()["routine"], "Daily digest", "{rows}");
    assert_eq!(row(&id).unwrap()["routine"], "", "{rows}");
    let again = call(&shed, &started, "create_routine", &ask);
    let said = again["content"][0]["text"].as_str().unwrap();
    assert!(said.starts_with("NOT_HERE"), "{said}");

    assert_eq!(
        form(&cooee, &format!("/routines/{rid}"), "paused=true"),
        "303"
    );
    assert_eq!(cooee.json("/routines.json")[0]["paused"]["why"], "owner");
    let bad = form(&cooee, &format!("/routines/{rid}"), "when=*%2F5+*+*+*+*");
    assert_eq!(bad, "400");
    assert_eq!(form(&cooee, &format!("/routines/{rid}/delete"), ""), "303");
    let left = cooee.json("/routines.json");
    assert_eq!(left, json!([]));
    assert!(
        cooee
            .json("/threads")
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["id"] == started)
    );
}
