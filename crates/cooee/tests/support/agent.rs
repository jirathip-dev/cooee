use std::{
    env, fs,
    io::Write as _,
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

use cooee_core::shed_proto::{ApprovalOption, EventKind};
use serde_json::{Value, json};

use super::FakeShed;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Input {
    Prompt(String),
    Approve(String),
    Cancel,
}

impl FakeShed {
    /// A framed input (`RS`, JSON, newline) for terminal `name`'s agent.
    pub fn input(&self, name: &str, text: &str) {
        let v: Value = serde_json::from_str(text.strip_prefix('\x1e').unwrap().trim_end()).unwrap();
        let input = match (&v["prompt"], &v["approve"]) {
            (Value::String(p), _) => Input::Prompt(p.clone()),
            (_, Value::String(o)) => Input::Approve(o.clone()),
            _ => Input::Cancel,
        };
        let mut s = self.shared.lock().unwrap();
        let tx = s.agents.entry(name.to_owned()).or_insert_with(|| {
            let (tx, rx) = mpsc::channel();
            let shed = self.clone();
            let name = name.to_owned();
            thread::spawn(move || shed.agent(&name, &rx));
            tx
        });
        tx.send(input).unwrap();
    }

    fn agent(&self, name: &str, rx: &Receiver<Input>) {
        while let Ok(input) = rx.recv() {
            let Input::Prompt(prompt) = input else {
                continue;
            };
            let turn = EventKind::Turn {
                prompt: prompt.clone(),
            };
            self.emit(name, turn);
            let Some((events, stop)) = self.script(name, &prompt, rx) else {
                continue;
            };
            for event in events {
                self.emit(name, event);
            }
            self.report(name, "done", Some(stop));
        }
    }

    fn script(
        &self,
        name: &str,
        prompt: &str,
        rx: &Receiver<Input>,
    ) -> Option<(Vec<EventKind>, &'static str)> {
        let delay = Duration::from_millis(
            env::var("FAKE_SHED_DELAY_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
        );
        let read = "Read src/lib.rs";
        if let Some(turn) = self.conformance(name, prompt, rx) {
            return Some(turn);
        }
        Some(if let Some(said) = self.cued(name) {
            (vec![reply(&said, "end_turn")], "end_turn")
        } else if prompt.contains("approve") {
            self.approval(name, rx)
        } else if prompt.contains("markdown") {
            let mut events: Vec<EventKind> = MARKDOWN.split_inclusive('\n').map(text).collect();
            events.push(reply(MARKDOWN, "end_turn"));
            (events, "end_turn")
        } else if prompt.contains("paragraphs") {
            let said = "Questions for you:\n1. **HTML or SwiftUI?**\n\n**Call voice:** the iOS target building.";
            (vec![reply(said, "end_turn")], "end_turn")
        } else if prompt.contains("commands") {
            let mut events = Vec::new();
            for (id, title) in [
                ("k1", "$ cargo fmt"),
                ("k2", "$ cargo clippy"),
                ("k3", "$ cargo test"),
            ] {
                events.extend([tool(id, title, "in_progress"), tool(id, title, "completed")]);
            }
            events.push(reply("Ran 3 commands.", "end_turn"));
            (events, "end_turn")
        } else if prompt.contains("tool") {
            let events = [
                tool("c1", read, "pending"),
                tool("c1", read, "in_progress"),
                tool("c1", read, "completed"),
                text("Read "),
                text("it."),
                reply("Read it.", "end_turn"),
            ];
            (events.to_vec(), "end_turn")
        } else if prompt.contains("plan") {
            let entry = |c: &str, s: &str| json!({ "content": c, "priority": "high", "status": s });
            let entries = vec![
                entry("Read the code", "completed"),
                entry("Fix the bug", "in_progress"),
            ];
            let events = vec![EventKind::Plan { entries }, reply("Planned.", "end_turn")];
            (events, "end_turn")
        } else if prompt.contains("hang") {
            self.emit(name, tool("c3", "cargo test", "in_progress"));
            self.emit(name, text("w0 "));
            while rx.recv().is_ok_and(|i| i != Input::Cancel) {}
            (vec![reply("w0 ", "cancelled")], "cancelled")
        } else if let Some(n) = prompt
            .rsplit("\n\n")
            .next()
            .and_then(|m| m.strip_prefix("slow "))
        {
            let n: usize = n.trim().parse().unwrap();
            let mut stop = "end_turn";
            for i in 0..n {
                thread::sleep(delay);
                if rx.try_recv() == Ok(Input::Cancel) {
                    stop = "cancelled";
                    break;
                }
                self.emit(name, text(&format!("w{i} ")));
            }
            if stop == "end_turn" {
                while rx.recv().is_ok_and(|i| i != Input::Cancel) {}
            }
            (vec![reply(&format!("streamed {n} words"), stop)], stop)
        } else if prompt.contains("wait for a note") {
            let note = loop {
                match rx.recv() {
                    Ok(Input::Prompt(p)) => break p,
                    Ok(_) => {}
                    Err(_) => return None,
                }
            };
            (
                vec![reply(&format!("noted: {note}"), "end_turn")],
                "end_turn",
            )
        } else if let Some(said) = prompt.strip_prefix("echo ") {
            (vec![reply(said, "end_turn")], "end_turn")
        } else if prompt.contains("exit") {
            self.shared.lock().unwrap().stopped.insert(name.to_owned());
            self.emit(name, EventKind::Exited { code: Some(1) });
            return None;
        } else {
            let events = vec![text("Hel"), text("lo"), reply("Hello", "end_turn")];
            (events, "end_turn")
        })
    }

    fn conformance(
        &self,
        name: &str,
        prompt: &str,
        rx: &Receiver<Input>,
    ) -> Option<(Vec<EventKind>, &'static str)> {
        let said = if prompt.contains("What is the code word?") {
            let word = prompt
                .split("The code word is ")
                .nth(1)
                .unwrap_or("unknown.");
            word.split_whitespace()
                .next()
                .unwrap()
                .trim_end_matches('.')
                .to_owned()
        } else if prompt.contains("with your remember tool") {
            let note = json!({ "text": "The conformance check ran." });
            self.cooee_tool(name, "remember", &note)
        } else if prompt.contains("Use your conformance skill") {
            self.cooee_tool(name, "skill", &json!({ "read": "conformance" }))
        } else if prompt.contains("Run `sleep 120`") {
            return Some(self.sleep_until_cancel(name, rx));
        } else {
            return None;
        };
        Some((vec![reply(&said, "end_turn")], "end_turn"))
    }

    fn cooee_tool(&self, name: &str, tool: &str, arguments: &Value) -> String {
        let file = self.dir.join(format!("mcp-{name}.json"));
        let servers: Value = serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap();
        let cooee = servers
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["name"] == "cooee")
            .unwrap();
        let args = cooee["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a.as_str().unwrap());
        let mut command = Command::new(cooee["command"].as_str().unwrap());
        for v in cooee["env"].as_array().unwrap() {
            command.env(v["name"].as_str().unwrap(), v["value"].as_str().unwrap());
        }
        let mut child = command
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let call = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                           "params": { "name": tool, "arguments": arguments } });
        writeln!(child.stdin.take().unwrap(), "{call}").unwrap();
        let out = child.wait_with_output().unwrap();
        let answer: Value = serde_json::from_slice(&out.stdout).unwrap();
        answer["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    fn sleep_until_cancel(
        &self,
        name: &str,
        rx: &Receiver<Input>,
    ) -> (Vec<EventKind>, &'static str) {
        let mut sleep = Command::new("sleep").arg("120").spawn().unwrap();
        self.emit(name, tool("s1", "sleep 120", "in_progress"));
        while rx.recv().is_ok_and(|i| i != Input::Cancel) {}
        sleep.kill().unwrap();
        sleep.wait().unwrap();
        (vec![reply("", "cancelled")], "cancelled")
    }

    fn approval(&self, name: &str, rx: &Receiver<Input>) -> (Vec<EventKind>, &'static str) {
        let call = "Edit src/lib.rs";
        self.emit(name, tool("c2", call, "pending"));
        let option = |id: &str, name: &str, kind: &str| ApprovalOption {
            id: id.to_owned(),
            name: name.to_owned(),
            kind: kind.to_owned(),
        };
        let options = vec![
            option("allow", "Allow", "allow_once"),
            option("always", "Always allow", "allow_always"),
            option("reject", "Reject", "reject_once"),
        ];
        let tool_name = call.to_owned();
        self.emit(
            name,
            EventKind::ApprovalRequest {
                tool: tool_name,
                options,
            },
        );
        self.report(name, "blocked", Some(call));
        let Ok(Input::Approve(option)) = rx.recv() else {
            return (vec![reply("", "cancelled")], "cancelled");
        };
        let done = if option == "reject" {
            "failed"
        } else {
            "completed"
        };
        self.emit(name, EventKind::Approved { option });
        self.report(name, "working", None);
        (
            vec![tool("c2", call, done), reply("Edited.", "end_turn")],
            "end_turn",
        )
    }
}

/// The `markdown` turn's reply: a heading, inline marks, a link, a 3-column table, a fenced block, a list and raw
/// HTML (which stays text).
pub const MARKDOWN: &str = "## Build report

The **build** passed on *staging*: see [the run](https://example.com/run/1) and `cargo test`.

| crate | tests | time |
|---|---:|---|
| cooee-core | 42 | 1.2 s |
| cooee-server | 17 | 3.4 s |

```rust
fn main() {
    println!(\"hello\");
}
```

- fmt is clean
- clippy is clean

<script>alert(1)</script>
";

fn text(t: &str) -> EventKind {
    EventKind::Text { text: t.to_owned() }
}

fn tool(id: &str, title: &str, status: &str) -> EventKind {
    EventKind::ToolCall {
        id: id.to_owned(),
        title: title.to_owned(),
        status: status.to_owned(),
        kind: None,
    }
}

fn reply(t: &str, stop: &str) -> EventKind {
    EventKind::Reply {
        text: t.to_owned(),
        stop_reason: stop.to_owned(),
    }
}
