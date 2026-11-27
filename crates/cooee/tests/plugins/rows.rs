use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
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
    posted: Posted,
    served: Served,
}

type Posted = Arc<Mutex<Vec<(String, Value)>>>;

type Served = Arc<Mutex<Vec<String>>>;

fn request(s: TcpStream) -> (String, Vec<u8>, TcpStream) {
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
    (path, body, r.into_inner())
}

fn serve(s: TcpStream, next: &Mutex<mpsc::Receiver<Value>>, log: &Posted, served: &Served) {
    let (path, body, mut s) = request(s);
    let reply = |s: &mut TcpStream, v: Value| {
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
        "/cooee/profiles" => reply(
            &mut s,
            json!({ "profiles": [{ "name": "orchestrator", "label": "Orchestrator", "agent": "fake-acp" }] }),
        ),
        "/attach" => {
            let body: Value = serde_json::from_slice(&body).unwrap();
            let mut a: Value = serde_json::from_str(include_str!(
                "../fixtures/drover-contract/orchestrator-attach.json"
            ))
            .unwrap();
            a["env"]["DROVER_THREAD"] = body["thread"].clone();
            a["settings"] = json!("");
            a["brief"] = json!("You are drover's orchestrator.");
            log.lock().unwrap().push((path, body));
            reply(&mut s, a);
        }
        "/health" => reply(&mut s, json!({})),
        "/manifest" => {
            let actions = served.lock().unwrap().clone();
            reply(
                &mut s,
                json!({ "name": "drover", "version": "0.1.0", "screens": [{ "type": "chat-cards" }],
                        "actions": actions, "destructive": ["config-card"] }),
            );
        }
        _ => {
            let body = serde_json::from_slice(&body).unwrap_or(Value::Null);
            log.lock().unwrap().push((path, body));
            reply(&mut s, json!({ "line": "q2 answered: split" }));
        }
    }
}

fn drover(actions: &[&str]) -> Drover {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (rows, next) = mpsc::channel::<Value>();
    let next = Arc::new(Mutex::new(next));
    let posted = Arc::new(Mutex::new(Vec::new()));
    let served = Arc::new(Mutex::new(
        actions.iter().map(|a| (*a).to_owned()).collect(),
    ));
    let (log, menu) = (Arc::clone(&posted), Arc::clone(&served));
    thread::spawn(move || {
        for s in listener.incoming().flatten() {
            let (next, log, menu) = (Arc::clone(&next), Arc::clone(&log), Arc::clone(&menu));
            thread::spawn(move || serve(s, &next, &log, &menu));
        }
    });
    Drover {
        port,
        rows,
        posted,
        served,
    }
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

fn install(dir: &Path, port: u16, listed: bool) {
    let build = dir.join("data/plugins/drover/builds/c0ffee12345");
    fs::create_dir_all(&build).unwrap();
    let manifest = format!(
        "name = \"drover\"\nversion = \"0.1.0\"\nattach = \"http://127.0.0.1:{port}\"\n\n\
         [[places]]\ntype = \"chat-cards\"\n\n\
         [permissions]\nplaces = [\"chat-cards\"]\npush = true\nactions = [\"answer\"]\n\n\
         [threads]\nrows = \"/rows\"\nstream = \"/stream\"\nsaid = \"/thread/{{id}}/said\"\nattach = \"/attach\"\n"
    );
    fs::write(build.join("cooee-plugin.toml"), manifest).unwrap();
    let lock = json!([{ "name": "drover", "url": "https://example.com/drover", "commit": "c0ffee12345",
                        "version": "0.1.0", "listed": listed, "tabs": ["drover"], "port": port, "enabled": true }]);
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
    start_serving(name, &["answer"], true)
}

fn start_serving(
    name: &str,
    serves: &[&str],
    listed: bool,
) -> (std::path::PathBuf, FakeShed, Cooee, Drover) {
    let dir = scratch(name);
    let d = drover(serves);
    install(&dir, d.port, listed);
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
fn an_action_drovers_served_manifest_adds_reaches_it_once_the_owner_accepts_it_on_cooees_card() {
    let (_dir, _shed, cooee, d) = start_serving("rows-served", &["answer", "config-card"], true);
    let cards = "/p/drover/screens/cards";
    let tap = |block: &str, action: &str| {
        let act = json!({ "screen": "/screens/cards", "block": block, "action": action });
        cooee.post("/p/drover/action", &act)
    };
    let card = until("cooee's card asking for config-card at connect", || {
        let shown = cooee.json(cards);
        holds(&shown, "cooee-permissions").then_some(shown)
    });
    assert!(
        card.to_string()
            .contains("config-card (always confirmed first)"),
        "{card}"
    );
    let (code, body) = tap("m-6", "config-card");
    assert_eq!(code, 403, "{body}");
    let (code, body) = tap("cooee-permissions", "accept");
    assert_eq!(code, 200, "{body}");
    assert_eq!(cooee.json(cards)["blocks"], json!([]));
    let (code, body) = tap("m-6", "config-card");
    assert_eq!(code, 200, "{body}");

    d.served.lock().unwrap().push("secret-card".to_owned());
    let (code, body) = tap("m-6", "secret-card");
    assert_eq!(code, 403, "{body}");
    assert!(holds(&cooee.json(cards), "cooee-permissions"));
    let (code, body) = tap("cooee-permissions", "accept");
    assert_eq!(code, 200, "{body}");
    let (code, body) = tap("m-6", "secret-card");
    assert_eq!(code, 200, "{body}");
    let (code, body) = tap("m-6", "drop");
    assert_eq!(code, 403, "{body}");
    let sent: Vec<Value> = d
        .posted("/action")
        .iter()
        .map(|p| p["action"].clone())
        .collect();
    assert_eq!(sent, [json!("config-card"), json!("secret-card")]);
}

#[test]
fn a_plugin_installed_from_git_keeps_its_pinned_actions_whatever_its_backend_serves() {
    let (_dir, _shed, cooee, d) = start_serving("rows-git", &["answer", "config-card"], false);
    let act = json!({ "screen": "/screens/cards", "block": "m-6", "action": "config-card" });
    let (code, body) = cooee.post("/p/drover/action", &act);
    assert_eq!(code, 403, "{body}");
    assert_eq!(cooee.json("/p/drover/screens/cards")["blocks"], json!([]));
    assert_eq!(d.posted("/action"), Vec::<Value>::new());
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

fn attached(cooee: &Cooee, repo: &str, adopt: Option<&str>) -> String {
    let mut new = json!({ "agent": "fake-acp", "profile": "drover/orchestrator", "text": "Plan it",
                          "random_id": format!("r-{repo}"), "repo": repo });
    if let Some(old) = adopt {
        new["adopt"] = json!(old);
    }
    let body = until("drover's orchestrator profile served", || {
        let (code, body) = cooee.post("/threads", &new);
        (code == 200).then_some(body)
    });
    serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn listed(cooee: &Cooee, repo: &str) -> Vec<Value> {
    cooee
        .json(&format!("/threads?repo={repo}"))
        .as_array()
        .unwrap()
        .clone()
}

#[test]
fn a_new_chat_with_a_repo_attaches_its_agent_carries_the_thread_and_a_tap_answers_its_ask() {
    let (_dir, shed, cooee, d) = start("rows-attach");
    let id = attached(&cooee, "sendmeter", None);
    assert_eq!(
        d.posted("/attach"),
        [json!({ "fleet": "sendmeter", "thread": id })]
    );
    let terminal = format!("cooee-{id}");
    let env = shed.adapter_env_of(&terminal);
    assert!(
        env.lines().any(|l| l == format!("DROVER_THREAD={id}")),
        "{env}"
    );
    assert!(env.lines().any(|l| l == "DROVER_PORT=41712"), "{env}");
    let shown = listed(&cooee, "sendmeter");
    assert_eq!(shown.len(), 1, "{shown:?}");
    assert_eq!(
        (&shown[0]["id"], &shown[0]["repo"]),
        (&json!(id), &json!("sendmeter"))
    );
    assert_eq!(listed(&cooee, "none"), Vec::<Value>::new());

    let mut ask = the_ask();
    ask["to"] = json!({ "k": "thread", "id": id });
    d.rows.send(ask).unwrap();
    let screen = format!("/screens/chat/{id}");
    until("the ask filed from the chat shows there", || {
        holds(&cooee.json(&screen), "q2").then_some(())
    });
    let tap = json!({ "screen": screen, "block": "q2", "action": "answer", "values": { "choice": "split" } });
    let (code, body) = cooee.post("/screens/action", &tap);
    assert_eq!(code, 200, "{body}");
    assert_eq!(d.posted("/action")[0]["values"]["choice"], "split");
}

#[test]
fn a_none_chat_attaches_and_a_desk_row_stays_out_of_it() {
    let (_dir, shed, cooee, d) = start("rows-none");
    let id = attached(&cooee, "none", None);
    assert_eq!(
        d.posted("/attach"),
        [json!({ "fleet": "none", "thread": id })]
    );
    let env = shed.adapter_env_of(&format!("cooee-{id}"));
    assert!(
        env.lines().any(|l| l == format!("DROVER_THREAD={id}")),
        "{env}"
    );
    d.rows.send(the_ask()).unwrap();
    until("the desk row in the cards place", || {
        holds(&cooee.json("/p/drover/screens/cards"), "m-6").then_some(())
    });
    let items = cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
    let rows = items
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["kind"] == "row");
    assert_eq!(rows.count(), 0, "{items}");
}

#[test]
fn a_chat_adopting_an_old_thread_takes_its_rows_and_copies_lines_under_its_id() {
    let (_dir, shed, cooee, d) = start("rows-adopt");
    let id = attached(&cooee, "all", Some("chat-1"));
    assert_eq!(
        d.posted("/attach"),
        [json!({ "fleet": "all", "thread": "chat-1" })]
    );
    let env = shed.adapter_env_of(&format!("cooee-{id}"));
    assert!(env.lines().any(|l| l == "DROVER_THREAD=chat-1"), "{env}");
    let mut ask = the_ask();
    ask["to"] = json!({ "k": "thread", "id": "chat-1" });
    d.rows.send(ask).unwrap();
    until("the old thread's row in the adopting chat", || {
        holds(&cooee.json(&format!("/screens/chat/{id}")), "q2").then_some(())
    });
    until("the first line copied under the old id", || {
        let said = d.posted("/thread/chat-1/said");
        (said.first().map(|s| s["text"].clone()) == Some(json!("Plan it"))).then_some(())
    });
}

#[test]
fn a_repo_on_a_chat_whose_plugin_attaches_none_is_refused() {
    let (dir, _shed, cooee, d) = start("rows-no-attach");
    let new = json!({ "agent": "fake-acp", "folder": dir.join("root/project"), "text": "Plan it",
                      "random_id": "r1", "repo": "sendmeter" });
    let (code, body) = cooee.post("/threads", &new);
    assert_eq!(code, 400, "{body}");
    assert_eq!(d.posted("/attach"), Vec::<Value>::new());
}
