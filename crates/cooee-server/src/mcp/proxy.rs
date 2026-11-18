use std::{
    collections::{HashMap, HashSet},
    io::{self, BufRead},
    sync::{
        Arc, Mutex, PoisonError,
        mpsc::{self, Receiver, Sender},
    },
    time::Duration,
};

use serde_json::{Value, json};

use super::{
    Grant,
    grant::{ask, grant},
    rpc::{error, put},
    upstream::{Upstream, open},
};

fn only(mut msg: Value, tools: Option<&[String]>) -> Value {
    if let (Some(allow), Some(list)) = (
        tools,
        msg.pointer_mut("/result/tools")
            .and_then(Value::as_array_mut),
    ) {
        list.retain(|t| {
            t["name"]
                .as_str()
                .is_some_and(|n| allow.iter().any(|a| a == n))
        });
    }
    msg
}

#[derive(Default)]
struct Frames {
    acts: HashMap<String, String>,
    shots: HashMap<String, (String, Value)>,
    asked: HashSet<String>,
    next: u64,
}

fn held<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Frames {
    fn asked_for(&mut self, tool: &str, id: String) {
        if tool == "browser_take_screenshot" {
            self.asked.insert(id);
        }
    }
}

fn keep(thread: &str, caption: &str, shot: &Value, asked: bool) {
    let list = shot.pointer("/result/content").and_then(Value::as_array);
    let image = list
        .into_iter()
        .flatten()
        .find(|c| c.get("type").and_then(Value::as_str) == Some("image"))
        .and_then(|c| c.get("data"))
        .cloned();
    if let Some(image) = image {
        let frame = json!({ "thread": thread, "server": { "caption": caption, "image": image, "asked": asked } });
        if let Err(e) = ask("/mcp/agent/frame", &frame) {
            cooee_core::note!("cooee: a replay frame: {e}");
        }
    }
}

fn shoot(
    pending: &Mutex<Frames>,
    up: &Mutex<Upstream>,
    out: &Mutex<io::Stdout>,
    caption: String,
    reply: Value,
) {
    let mut fr = held(pending);
    fr.next += 1;
    let shot = json!(format!("cooee-frame-{}", fr.next));
    let call = json!({ "jsonrpc": "2.0", "id": shot, "method": "tools/call",
                       "params": { "name": "browser_take_screenshot", "arguments": { "type": "jpeg" } } });
    fr.shots.insert(shot.to_string(), (caption, reply));
    drop(fr);
    if held(up).send(&call).is_err()
        && let Some((_, reply)) = held(pending).shots.remove(&shot.to_string())
    {
        put(out, &reply);
    }
}

/// `cooee mcp proxy <server> --thread <id>`: forwards the agent's stdio to the server and back, with only the
/// thread's allowed tools; cooee checks each `tools/call` again (a closed thread's token stops working) and logs a
/// browser's, which waits while the owner holds control and then carries their hand-back note in its result. After a
/// browser action cooee captions, the proxy takes a screenshot for cooee's replay frame (#161) before the agent gets
/// the answer; the agent never sees it. A screenshot the agent takes itself is kept as an `asked` frame (#166).
///
/// # Errors
/// When the grant is refused or the server can't start.
pub fn proxy(server: &str, thread: &str) -> Result<(), String> {
    let g = grant(server, thread)?;
    let (tx, rx): (Sender<Value>, Receiver<Value>) = mpsc::channel();
    let up = Arc::new(Mutex::new(open(&g, tx)?));
    let out = Arc::new(Mutex::new(io::stdout()));
    let lists: Arc<Mutex<HashSet<String>>> = Arc::default();
    let frames: Arc<Mutex<Frames>> = Arc::default();
    let notes: Arc<Mutex<HashMap<String, String>>> = Arc::default();
    let (o, l, pending, n, upstream) = (
        Arc::clone(&out),
        Arc::clone(&lists),
        Arc::clone(&frames),
        Arc::clone(&notes),
        Arc::clone(&up),
    );
    let (tools, me) = (g.tools.clone(), thread.to_owned());
    std::thread::spawn(move || {
        for mut msg in rx {
            let id = msg.get("id").map(Value::to_string).unwrap_or_default();
            let answer = msg.get("method").is_none();
            let note = held(&n).remove(&id);
            if let (true, Some(note), Some(content)) = (
                answer,
                note,
                msg.pointer_mut("/result/content")
                    .and_then(Value::as_array_mut),
            ) {
                let text = format!("The owner had the browser and handed it back: {note}");
                content.push(json!({ "type": "text", "text": text }));
            }
            if let Some((caption, reply)) =
                answer.then(|| held(&pending).shots.remove(&id)).flatten()
            {
                keep(&me, &caption, &msg, false);
                put(&o, &reply);
                continue;
            }
            if answer && held(&pending).asked.remove(&id) {
                keep(&me, "", &msg, true);
            }
            if let Some(caption) = answer.then(|| held(&pending).acts.remove(&id)).flatten() {
                shoot(&pending, &upstream, &o, caption, msg);
                continue;
            }
            let listed = answer && held(&l).remove(&id);
            put(
                &o,
                &if listed {
                    only(msg, tools.as_deref())
                } else {
                    msg
                },
            );
        }
    });
    for line in io::stdin().lock().lines().map_while(Result::ok) {
        let Ok(msg) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let id = msg.get("id").cloned().unwrap_or(Value::Null);
        match msg.get("method").and_then(Value::as_str) {
            Some("tools/list") => {
                held(&lists).insert(id.to_string());
            }
            Some("tools/call") => {
                let name = msg
                    .pointer("/params/name")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let args = msg.pointer("/params/arguments").cloned();
                let checked = json!({ "server": server, "tool": name, "arguments": args });
                match ask(
                    "/mcp/agent/call",
                    &json!({ "thread": thread, "server": checked }),
                ) {
                    Err(why) => {
                        put(&out, &error(&id, -32602, &why));
                        continue;
                    }
                    Ok(called) => {
                        let field =
                            |k: &str| called.get(k).and_then(Value::as_str).map(str::to_owned);
                        held(&frames).asked_for(name, id.to_string());
                        if let Some(caption) = field("caption") {
                            held(&frames).acts.insert(id.to_string(), caption);
                        }
                        if let Some(note) = field("note") {
                            held(&notes).insert(id.to_string(), note);
                        }
                    }
                }
            }
            _ => {}
        }
        held(&up).send(&msg)?;
    }
    if let Upstream::Stdio(child, _) = &mut *held(&up) {
        child.kill().ok();
        child.wait().ok();
    }
    Ok(())
}

/// Lists `g`'s tools (name, description) for the profile sheet: initialize, then `tools/list`, 10 s at most.
///
/// # Errors
/// When the server can't start or doesn't answer.
pub fn tools(g: &Grant) -> Result<Vec<(String, String)>, String> {
    let (tx, rx) = mpsc::channel();
    let mut up = open(g, tx)?;
    let hello = json!({ "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "cooee", "version": env!("CARGO_PKG_VERSION") } });
    let call = |up: &mut Upstream, id: u64, method: &str, params: Value| -> Result<Value, String> {
        up.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))?;
        loop {
            let msg = rx
                .recv_timeout(Duration::from_secs(10))
                .map_err(|_| format!("no answer to {method}"))?;
            if msg.get("id").is_some_and(|v| *v == id) {
                return msg.get("result").cloned().ok_or_else(|| {
                    msg.pointer("/error/message")
                        .unwrap_or(&Value::Null)
                        .to_string()
                });
            }
        }
    };
    let listed = call(&mut up, 1, "initialize", hello).and_then(|_| {
        up.send(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))?;
        call(&mut up, 2, "tools/list", json!({}))
    });
    if let Upstream::Stdio(mut child, _) = up {
        child.kill().ok();
        child.wait().ok();
    }
    let list = listed?
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let text = |t: &Value, k: &str| t[k].as_str().unwrap_or_default().to_owned();
    Ok(list
        .iter()
        .map(|t| (text(t, "name"), text(t, "description")))
        .collect())
}
