use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    path::Path,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};

use serde_json::{Value, json};

use crate::support::{Cooee, FakeShed, scratch, version};

const TOKEN: &str = "token-drover-0123456789";

struct Drover {
    port: u16,
    rows: mpsc::Sender<Value>,
    posted: Arc<Mutex<Vec<(String, Value)>>>,
}

fn drover() -> Drover {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (rows, next) = mpsc::channel::<Value>();
    let next = Arc::new(Mutex::new(next));
    let posted = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&posted);
    thread::spawn(move || {
        for s in listener.incoming().flatten() {
            let (next, log) = (Arc::clone(&next), Arc::clone(&log));
            thread::spawn(move || {
                let mut r = BufReader::new(s);
                let (mut head, mut line) = (String::new(), String::new());
                while r.read_line(&mut line).unwrap() > 2 {
                    head.push_str(&line.to_ascii_lowercase());
                    line.clear();
                }
                let len = head
                    .lines()
                    .find_map(|l| l.strip_prefix("content-length: "))
                    .map_or(0, |n| n.trim().parse().unwrap());
                let mut body = vec![0; len];
                r.read_exact(&mut body).unwrap();
                let path = head.split(' ').nth(1).unwrap_or_default().to_owned();
                let mut s = r.into_inner();
                let reply = |s: &mut std::net::TcpStream, v: Value| {
                    write!(
                        s,
                        "HTTP/1.0 200 OK\r\nContent-Type: application/json\r\n\r\n{v}"
                    )
                    .ok();
                };
                match path.as_str() {
                    "/stream" => {
                        write!(
                            s,
                            "HTTP/1.0 200 OK\r\nContent-Type: text/event-stream\r\n\r\n"
                        )
                        .unwrap();
                        let next = next.lock().unwrap();
                        while let Ok(row) = next.recv() {
                            let sent = format!("id: {}\ndata: {row}\n\n", row["seq"]);
                            if s.write_all(sent.as_bytes()).is_err() {
                                return;
                            }
                        }
                    }
                    "/rows?open=1" => reply(&mut s, json!({ "seq": 1, "more": false, "rows": [] })),
                    "/health" => reply(&mut s, json!({})),
                    _ => {
                        let body = serde_json::from_slice(&body).unwrap_or(Value::Null);
                        log.lock().unwrap().push((path, body));
                        reply(&mut s, json!({ "line": "q2 answered: split" }));
                    }
                }
            });
        }
    });
    Drover { port, rows, posted }
}

impl Drover {
    fn posted(&self, path: &str) -> Vec<Value> {
        let posted = self.posted.lock().unwrap();
        posted
            .iter()
            .filter(|(p, _)| p == path)
            .map(|(_, b)| b.clone())
            .collect()
    }
}

fn install(dir: &Path, port: u16) {
    let build = dir.join("data/plugins/drover/builds/c0ffee12345");
    fs::create_dir_all(&build).unwrap();
    let manifest = format!(
        "name = \"drover\"\nversion = \"0.1.0\"\nattach = \"http://127.0.0.1:{port}\"\n\n\
         [[places]]\ntype = \"chat-cards\"\n\n\
         [permissions]\nplaces = [\"chat-cards\"]\npush = true\nactions = [\"answer\"]\n\n\
         [threads]\nrows = \"/rows\"\nstream = \"/stream\"\nsaid = \"/thread/{{id}}/said\"\n"
    );
    fs::write(build.join("cooee-plugin.toml"), manifest).unwrap();
    let lock = json!([{ "name": "drover", "url": "https://example.com/drover", "commit": "c0ffee12345",
                        "version": "0.1.0", "listed": true, "tabs": ["drover"], "port": port, "enabled": true }]);
    fs::write(dir.join("data/plugins.lock"), lock.to_string()).unwrap();
    let entry = format!(
        "[[plugins]]\nname = \"drover\"\nlabel = \"drover\"\nurl = \"http://127.0.0.1:{port}\"\nmodule = \"\"\ntoken = \"{TOKEN}\"\n"
    );
    fs::write(dir.join("data/plugins/plugins.toml"), entry).unwrap();
}

fn until<T>(what: &str, mut f: impl FnMut() -> Option<T>) -> T {
    for _ in 0..500 {
        if let Some(v) = f() {
            return v;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("never: {what}");
}

fn holds(v: &Value, id: &str) -> bool {
    match v {
        Value::Object(o) => o.get("id") == Some(&json!(id)) || o.values().any(|v| holds(v, id)),
        Value::Array(a) => a.iter().any(|v| holds(v, id)),
        _ => false,
    }
}

fn the_ask() -> Value {
    let samples = include_str!("../fixtures/drover-contract/row-ask.json");
    serde_json::from_str::<Value>(samples).unwrap()[1].clone()
}

fn start(name: &str) -> (std::path::PathBuf, FakeShed, Cooee, Drover) {
    let dir = scratch(name);
    let d = drover();
    install(&dir, d.port);
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    (dir, shed, cooee, d)
}

#[test]
fn an_ask_from_an_attached_chat_is_drovers_own_block_there_and_a_tap_answers_it() {
    let (dir, shed, cooee, d) = start("rows-chat");
    let new = json!({ "agent": "fake-acp", "folder": dir.join("root/project"), "text": "Plan it", "random_id": "r1" });
    let (code, body) = cooee.post("/threads", &new);
    assert_eq!(code, 200, "{body}");
    let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut ask = the_ask();
    ask["to"] = json!({ "k": "thread", "id": id });
    ask["prompt"] = json!("q2 · split the PR? Answer with drover answer.");
    d.rows.send(ask.clone()).unwrap();

    let screen = format!("/screens/chat/{id}");
    until("the ask's block in its chat", || {
        holds(&cooee.json(&screen), "q2").then_some(())
    });
    let terminal = format!("cooee-{id}");
    until("its prompt typed into the agent", || {
        let sent = shed.sent(&terminal);
        sent.iter()
            .any(|t| t.contains("q2 · split the PR?"))
            .then_some(())
    });

    let tap = json!({ "screen": screen, "block": "q2", "action": "answer", "values": { "thread": "", "choice": "split" } });
    let (code, body) = cooee.post("/screens/action", &tap);
    assert_eq!(code, 200, "{body}");
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap()["toast"]["title"],
        "q2 answered: split"
    );
    let posted = d.posted("/action");
    assert_eq!(posted.len(), 1, "{posted:?}");
    assert_eq!(
        (
            &posted[0]["block"],
            &posted[0]["action"],
            &posted[0]["values"]["choice"]
        ),
        (&json!("q2"), &json!("answer"), &json!("split"))
    );

    let mut settled = the_ask();
    settled["seq"] = json!(7);
    settled["open"] = json!(false);
    settled["to"] = json!({ "k": "thread", "id": id });
    settled["block"] = json!({ "type": "text", "id": "q2-settled", "markdown": "✓ split" });
    d.rows.send(settled).unwrap();
    until("the settled row replaces the ask", || {
        let shown = cooee.json(&screen);
        (holds(&shown, "q2-settled") && !holds(&shown, "q2")).then_some(())
    });
    let rows = cooee.json(&format!("/threads/{id}/messages"))["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["kind"] == "row")
        .count();
    assert_eq!(rows, 1, "one item per row key");
}

#[test]
fn a_desk_row_shows_in_the_cards_place_and_leaves_it_once_settled() {
    let (_dir, _shed, cooee, d) = start("rows-desk");
    let cards = "/p/drover/screens/cards";
    d.rows.send(the_ask()).unwrap();
    until("the desk ask in the cards place", || {
        holds(&cooee.json(cards), "m-6").then_some(())
    });
    let mut settled = the_ask();
    settled["seq"] = json!(7);
    settled["open"] = json!(false);
    d.rows.send(settled).unwrap();
    until("the settled desk row gone", || {
        (cooee.json(cards)["blocks"] == json!([])).then_some(())
    });
}
