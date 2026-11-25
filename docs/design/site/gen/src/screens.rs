use blocks::Block;
use serde_json::{Value, json};

use crate::{b, prose};

fn screen(id: &str, title: &str, back: Option<&str>, blocks: Vec<Value>) -> Block {
    let mut v = json!({"type": "screen", "source": "core", "id": id, "title": title, "blocks": blocks});
    if let Some(back) = back {
        v["back"] = json!(back);
    }
    b(v)
}

fn said(id: &str, md: &str) -> Value {
    json!({"type": "message", "id": id, "role": "owner", "name": "You", "blocks": [prose(&format!("{id}-t"), md)]})
}

fn agent(id: &str, name: &str, blocks: Vec<Value>) -> Value {
    json!({"type": "message", "id": id, "role": "agent", "name": name, "blocks": blocks})
}

fn tool(id: &str, name: &str, summary: &str) -> Value {
    json!({"type": "tool", "id": id, "name": name, "state": "done", "summary": summary})
}

fn composer(id: &str) -> Value {
    json!({"type": "composer", "id": id, "action": "send",
        "acts": {"queue": "queue", "steer": "steer", "unqueue": "unqueue", "stop": "stop"}, "queued": []})
}

fn row(id: &str, title: &str, sub: Value) -> Value {
    json!({"type": "row", "id": id, "title": title, "subtitle": sub})
}

fn list(id: &str, title: &str, rows: Vec<Value>) -> Block {
    screen(id, title, None, vec![json!({"type": "list", "id": format!("{id}-rows"), "rows": rows})])
}

/// chat.rs `screen`: a thread's log, then its composer.
pub fn chat() -> Block {
    let log = json!({"type": "section", "id": "log", "blocks": [
        said("m1", "Find us a quiet beach for Saturday, under two hours away."),
        agent("m2", "Planner", vec![
            tool("m2-1", "web search", "beaches within two hours, tide times"),
            serde_json::to_value(prose("m2-2", "**Shell Cove** fits: 1 h 40 by car, low tide at 11:20 and few people before noon.\n\n- Park at the north end\n- Café opens at 8")).unwrap_or_default(),
        ]),
        said("m3", "Book the café for 9."),
    ]});
    screen("chat", "Weekend away", Some("/screens/sessions"), vec![log, composer("say")])
}

/// sessions.rs `screen`: one section per persona, its chats and New chat.
pub fn sessions() -> Block {
    let badge = |n: &str| json!({"text": n, "tone": "accent"});
    let chat = |id: &str, t: &str, last: &str, unseen: Option<&str>| {
        let mut r = json!({"type": "row", "id": id, "title": t, "subtitle": {"text": last}, "tap": {"open": "#"}});
        if let Some(n) = unseen {
            r["badge"] = badge(n);
        }
        r
    };
    let sec = |id: &str, h: &str, rows: Vec<Value>| json!({"type": "section", "id": id, "header": h, "blocks": rows});
    screen("sessions", "Chats", None, vec![
        sec("p-planner", "Planner", vec![
            chat("t-1", "Weekend away", "Booked: café at 9", Some("2")),
            chat("t-2", "Gift ideas", "Three picks under $50", None),
        ]),
        sec("p-cook", "Cook", vec![chat("t-3", "Meal plan", "Shopping list ready", None)]),
        sec("p-coder", "Coder", vec![chat("t-4", "Fix the search box", "Tests pass", Some("1"))]),
    ])
}

/// A Chief chat: Chief hands it to the persona that fits, as chat.rs's link row.
pub fn routed() -> Block {
    let log = json!({"type": "section", "id": "chief-log", "blocks": [
        said("r1", "Plan dinners for the week, nothing with nuts."),
        agent("r2", "Chief", vec![tool("r2-1", "route", "meal planning → Cook")]),
        {"type": "message", "id": "r3", "role": "system", "name": "cooee", "blocks": [
            {"type": "row", "id": "r3-t", "title": "Continued in Cook · Meal plan", "tap": {"open": "#"}}
        ]},
    ]});
    screen("chief", "Chief", None, vec![log, composer("chief-say")])
}

/// drover's queue, as the drover plugin draws it in cooee.
pub fn drover() -> Block {
    let tone = |t: &str, tone: &str| json!({"text": t, "tone": tone});
    list("queue", "Queue", vec![
        row("q-12", "#12 search box", tone("in review", "working")),
        row("q-14", "#14 dark mode", tone("needs you", "needs")),
        row("q-9", "#9 sign-in page", tone("merged", "done")),
        row("q-15", "#15 export CSV", tone("queued", "idle")),
    ])
}

/// settings.rs `machine_rows`: each machine, online or not, with its tags.
pub fn machines() -> Block {
    list("machines", "Machines", vec![
        row("mac", "This Mac", json!("online · macos")),
        row("box", "Linux box", json!("online · linux · docker")),
        row("laptop", "Laptop", json!("offline")),
    ])
}

/// settings.rs `device_rows`: the paired devices, this one marked.
pub fn home() -> Block {
    list("devices", "Devices", vec![
        row("d-phone", "iPhone", json!("this device")),
        json!({"type": "row", "id": "d-mac", "title": "Mac"}),
        json!({"type": "row", "id": "d-ipad", "title": "iPad"}),
    ])
}

/// The terminal plugin's screen: shed's cell runs, drawn by the plugin's own row code.
pub fn terminal() -> Block {
    let run = |t: &str, fg: Option<u8>, bold: bool| json!({"text": t, "fg": fg, "bold": bold});
    let rows = json!([
        [run("~/app ", Some(4), true), run("main ", Some(5), false), run("$ ", None, false), run("cargo test", None, false)],
        [run("   Compiling app v0.4.0", Some(2), true)],
        [run("    Finished test profile in 3.1s", Some(2), true)],
        [run("running 42 tests", None, false)],
        [run("test result: ", None, false), run("ok", Some(2), true), run(". 42 passed; 0 failed", None, false)],
        [run("~/app ", Some(4), true), run("main ", Some(5), false), run("$ ", None, false)],
    ]);
    screen("terminal", "Terminal", None, vec![json!({"type": "island", "id": "shell", "name": "cooee-terminal",
        "fallback": {"title": "~/app $ cargo test", "subtitle": "test result: ok. 42 passed"},
        "props": {"rows": rows}})])
}

/// A small chats screen per look, for the themes showcase.
pub fn tile(id: &str) -> Block {
    let tone = |t: &str, tone: &str| json!({"text": t, "tone": tone});
    list(&format!("tile-{id}"), "Chats", vec![
        row(&format!("{id}-a"), "Weekend away", tone("working", "working")),
        row(&format!("{id}-b"), "Meal plan", tone("needs you", "needs")),
        row(&format!("{id}-c"), "Fix the search box", tone("done", "done")),
    ])
}
