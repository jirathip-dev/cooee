//! Routine guards (#690) end to end with the fake shed: a schedule's prompt runs at a due time only when it is fresh,
//! the owner wrote in its persona's threads since its last run, or it has not run yet that local day; every routine
//! stops at its runs-a-day cap (the default from Settings › routines, or its own), a webhook's capped POST still
//! feeds its silence clock and an alive POST counts for nothing; a post script is never held, a wake script only at
//! its wake; an ended routine tallies nothing; and a run keeps the chat's own model.

mod support;

use std::{
    fmt::Write as _,
    fs,
    io::{Read as _, Write as _},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use support::{Cooee, FakeShed, scratch, version};

const SECRET: &str = "guard-test-secret";

fn ago(days: u64) -> u64 {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    u64::try_from(now.as_millis()).unwrap() - days * 86_400_000
}

struct Mac {
    dir: PathBuf,
    shed: FakeShed,
    extra: String,
    cooee: Option<Cooee>,
    chat: String,
}

impl Mac {
    fn new(name: &str, model: &str) -> Self {
        let dir = scratch(name);
        let shed = FakeShed::start(&dir.join("shed"), version());
        let mut mac = Self {
            dir,
            shed,
            extra: String::new(),
            cooee: None,
            chat: String::new(),
        };
        mac.start();
        let new = json!({ "agent": "fake-acp", "folder": mac.dir.join("root/project"), "text": "hi",
                          "random_id": "start", "model": model });
        let (status, body) = mac.cooee().post("/threads", &new);
        assert_eq!(status, 200, "{body}");
        let id = serde_json::from_str::<Value>(&body).unwrap()["id"].clone();
        id.as_str().unwrap().clone_into(&mut mac.chat);
        mac.done();
        mac
    }

    fn cooee(&self) -> &Cooee {
        self.cooee.as_ref().unwrap()
    }

    fn start(&mut self) {
        self.cooee = None;
        self.cooee = Some(Cooee::start_with(&self.dir, &self.shed.dir, &self.extra));
    }

    fn done(&self) {
        self.shed
            .report(&format!("cooee-{}", self.chat), "done", None);
    }

    fn routines(&mut self, defs: &[(&str, u32, &str)]) {
        self.cooee = None;
        let mut text = String::new();
        for (rid, hour, body) in defs {
            write!(
                text,
                "[[routine]]\nid = \"{rid}\"\nname = \"{rid}\"\nwhen = \"0 {hour} * * *\"\n{body}\n\
                 [routine.place]\nin = \"chat\"\nthread = \"{}\"\n\n",
                self.chat
            )
            .unwrap();
        }
        fs::write(self.dir.join("routines.toml"), text).unwrap();
        let state = json!({ "made_in": self.chat, "made_at": ago(2) });
        let runs: serde_json::Map<String, Value> = defs
            .iter()
            .map(|(rid, ..)| ((*rid).to_owned(), state.clone()))
            .collect();
        let file = json!({ "format": 1, "runs": runs });
        fs::write(self.dir.join("data/routine-runs.json"), file.to_string()).unwrap();
    }

    fn due(&mut self, rids: &[&str], minute: u32) {
        self.cooee = None;
        let settings = self.dir.join("routines.toml");
        let text = fs::read_to_string(&settings).unwrap();
        let mut out = String::new();
        for line in text.lines() {
            match line.strip_prefix("when = \"").filter(|_| {
                let before = &out[out.rfind("id = ").unwrap_or(0)..];
                rids.iter()
                    .any(|r| before.starts_with(&format!("id = \"{r}\"")))
            }) {
                Some(rest) => {
                    let hour = rest.split(' ').nth(1).unwrap();
                    writeln!(out, "when = \"{minute} {hour} * * *\"").unwrap();
                }
                None => writeln!(out, "{line}").unwrap(),
            }
        }
        fs::write(&settings, out).unwrap();
        self.change(|runs| {
            for r in rids {
                runs[*r]["last_due"] = json!(0);
            }
        });
        self.start();
        for r in rids {
            self.ticked(r);
        }
    }

    fn change(&self, f: impl FnOnce(&mut Value)) {
        let file = self.dir.join("data/routine-runs.json");
        let mut v: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
        f(&mut v["runs"]);
        fs::write(file, v.to_string()).unwrap();
    }

    fn ticked(&self, rid: &str) -> Value {
        let file = self.dir.join("data/routine-runs.json");
        for _ in 0..100 {
            let v: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
            if v["runs"][rid]["last_due"].as_u64().unwrap_or(0) > 0 {
                return v["runs"][rid].clone();
            }
            thread::sleep(Duration::from_millis(100));
        }
        panic!("{rid} never ticked");
    }

    fn sent(&self, rid: &str) -> usize {
        let items = self
            .cooee()
            .json(&format!("/threads/{}/messages", self.chat))["items"]
            .clone();
        let items = items.as_array().unwrap().iter();
        items
            .filter(|i| i["kind"] == "you" && i["routine"]["id"] == rid)
            .count()
    }

    fn write(&self, text: &str) {
        self.done();
        let m = json!({ "text": text, "random_id": text });
        let (status, body) = self
            .cooee()
            .post(&format!("/threads/{}/send", self.chat), &m);
        assert_eq!(status, 200, "{body}");
        self.done();
    }

    fn row(&self, rid: &str) -> Value {
        let screen = self.cooee().json("/screens/settings/routines");
        let rows = screen["blocks"][0]["rows"].as_array().unwrap().clone();
        rows.into_iter()
            .find(|r| r["id"] == rid)
            .unwrap_or_default()
    }

    fn today(&self, rid: &str) -> String {
        let sub = self.row(rid)["subtitle"].clone();
        let sub = sub.as_str().unwrap_or_else(|| panic!("{rid}: {sub}"));
        sub.split(" · today: ")
            .nth(1)
            .unwrap_or_default()
            .to_owned()
    }

    fn act(&self, action: &str, values: &Value) -> (u16, String) {
        let a =
            json!({ "screen": "/screens/settings/routines", "action": action, "values": values });
        self.cooee().post("/screens/action", &a)
    }

    fn day_back(&self, rid: &str) {
        let file = self.dir.join("data/watermarks.json");
        let mut all: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
        let day = all[format!("routine-{rid}")]["today"]["day"]
            .as_u64()
            .unwrap();
        all[format!("routine-{rid}")]["today"]["day"] = json!(day - 1);
        fs::write(file, all.to_string()).unwrap();
    }
}

#[test]
fn a_schedule_runs_only_when_something_changed_and_once_on_a_quiet_day() {
    let mut mac = Mac::new("guards-idle", "");
    mac.routines(&[
        ("r-other", 1, "prompt = \"other\""),
        ("r-main", 2, "prompt = \"main\""),
    ]);
    mac.start();
    mac.ticked("r-main");
    assert_eq!(mac.sent("r-main"), 1, "a fresh routine runs");
    assert_eq!(mac.today("r-main"), "ran 1 · skipped 0 idle, 0 capped");

    mac.due(&["r-main"], 5);
    assert_eq!(mac.sent("r-main"), 1, "nothing written: no run");
    assert_eq!(mac.today("r-main"), "ran 1 · skipped 1 idle, 0 capped");

    mac.change(|runs| runs["r-other"]["last_due"] = json!(0));
    let file = mac.dir.join("data/watermarks.json");
    let mut all: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
    all.as_object_mut().unwrap().remove("routine-r-other");
    fs::write(&file, all.to_string()).unwrap();
    mac.due(&["r-other", "r-main"], 10);
    assert_eq!(mac.sent("r-other"), 2, "r-other is fresh again and runs");
    assert_eq!(mac.sent("r-main"), 1, "another routine's run wakes nothing");

    mac.write("the owner says something");
    mac.due(&["r-main"], 15);
    assert_eq!(mac.sent("r-main"), 2, "the owner wrote: it runs");
    mac.due(&["r-main"], 20);
    assert_eq!(mac.sent("r-main"), 2);

    mac.day_back("r-main");
    mac.due(&["r-main"], 25);
    assert_eq!(
        mac.sent("r-main"),
        3,
        "a quiet day's first due time runs once"
    );
    mac.due(&["r-main"], 30);
    assert_eq!(mac.sent("r-main"), 3);
    assert_eq!(mac.today("r-main"), "ran 1 · skipped 1 idle, 0 capped");
}

#[test]
fn the_daily_cap_holds_with_the_default_and_with_a_routines_own() {
    let mut mac = Mac::new("guards-cap", "");
    mac.routines(&[
        ("r-own", 1, "prompt = \"own\""),
        ("r-cap", 2, "prompt = \"capped\""),
    ]);
    mac.start();
    mac.ticked("r-cap");
    let (status, body) = mac.act("default", &json!({ "per_day": "0" }));
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("BAD_CAP"), "{body}");
    let (status, body) = mac.act("default", &json!({ "per_day": "2" }));
    assert_eq!(status, 200, "{body}");
    let (status, body) = mac.act("cap", &json!({ "routine": "r-own", "per_day": "1" }));
    assert_eq!(status, 200, "{body}");
    assert_eq!(mac.row("default")["subtitle"], "2 runs a day");
    let own = mac.row("r-own")["subtitle"].as_str().unwrap().to_owned();
    assert!(own.contains("cap 1 (its own)"), "{own}");
    let cap = mac.row("r-cap")["subtitle"].as_str().unwrap().to_owned();
    assert!(cap.contains("cap 2 (default)"), "{cap}");
    assert_eq!(mac.row("r-own")["badge"]["text"], "capped today");
    assert_eq!(mac.row("r-cap")["badge"], Value::Null);

    for (n, minute) in [(2, 5), (2, 10)] {
        mac.write(&format!("news {minute}"));
        mac.due(&["r-own", "r-cap"], minute);
        assert_eq!((mac.sent("r-own"), mac.sent("r-cap")), (1, n));
    }
    assert_eq!(mac.today("r-cap"), "ran 2 · skipped 0 idle, 1 capped");
    assert_eq!(mac.today("r-own"), "ran 1 · skipped 0 idle, 2 capped");
    assert_eq!(mac.row("r-cap")["badge"]["text"], "capped today");

    mac.day_back("r-cap");
    mac.day_back("r-own");
    mac.due(&["r-own", "r-cap"], 15);
    assert_eq!(
        (mac.sent("r-own"), mac.sent("r-cap")),
        (2, 3),
        "a new day: both run"
    );

    let (status, body) = mac.act("cap", &json!({ "routine": "r-own", "per_day": "" }));
    assert_eq!(status, 200, "{body}");
    let own = mac.row("r-own")["subtitle"].as_str().unwrap().to_owned();
    assert!(own.contains("cap 2 (default)"), "{own}");
    let toml = fs::read_to_string(mac.dir.join("routines.toml")).unwrap();
    assert!(toml.starts_with("per_day = 2\n"), "{toml}");
}

#[test]
fn a_post_script_is_never_held_and_a_wake_script_only_at_its_wake() {
    let mut mac = Mac::new("guards-scripts", "");
    let script =
        |run: &str, mode: &str| format!("[routine.script]\nrun = \"{run}\"\nmode = \"{mode}\"");
    let (post, wake, quiet) = (
        script("echo fine", "post"),
        script("echo red; exit 1", "wake"),
        script("echo green", "wake"),
    );
    mac.routines(&[
        ("r-post", 1, &post),
        ("r-wake", 2, &wake),
        ("r-quiet", 3, &quiet),
    ]);
    fs::write(mac.dir.join("routines.toml"), {
        let t = fs::read_to_string(mac.dir.join("routines.toml")).unwrap();
        format!("per_day = 1\n\n{t}")
    })
    .unwrap();
    mac.start();
    for r in ["r-post", "r-wake", "r-quiet"] {
        mac.ticked(r);
    }
    mac.due(&["r-post", "r-wake", "r-quiet"], 5);
    let outcome = |r: &str| {
        let run = mac.ticked(r)["last_run"].clone();
        (run["outcome"].clone(), run["why"].clone())
    };
    assert_eq!(outcome("r-post"), (json!("ran"), Value::Null));
    assert_eq!(outcome("r-wake"), (json!("skipped"), json!("capped")));
    assert_eq!(outcome("r-quiet"), (json!("skipped"), json!("quiet")));
    assert_eq!(mac.sent("r-wake"), 1, "one wake under a cap of 1");
    let items = mac.cooee().json(&format!("/threads/{}/messages", mac.chat))["items"].clone();
    let posted = items
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["text"] == "⟳ r-post: fine");
    assert_eq!(posted.count(), 2, "a post script posts at both due times");
    let post = mac.row("r-post")["subtitle"].as_str().unwrap().to_owned();
    assert!(
        post.ends_with("post script · no cap: it runs no agent"),
        "{post}"
    );
    assert_eq!(mac.today("r-wake"), "ran 1 · skipped 0 idle, 1 capped");
    assert_eq!(mac.today("r-quiet"), "ran 0 · skipped 0 idle, 0 capped");
    let (status, body) = mac.act("cap", &json!({ "routine": "r-post", "per_day": "3" }));
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("BAD_CAP: a script that only posts"), "{body}");
}

#[test]
fn an_ended_routine_tallies_nothing_and_resumed_it_runs_fresh() {
    let mut mac = Mac::new("guards-ended", "");
    mac.routines(&[("r-end", 1, "prompt = \"end\"")]);
    mac.start();
    mac.ticked("r-end");
    mac.cooee = None;
    let settings = mac.dir.join("routines.toml");
    let text = fs::read_to_string(&settings).unwrap();
    fs::write(
        &settings,
        text.replace(
            "prompt = \"end\"",
            &format!("prompt = \"end\"\nends_at = {}", ago(0)),
        ),
    )
    .unwrap();
    mac.due(&["r-end"], 5);
    assert_eq!(mac.sent("r-end"), 1);
    assert_eq!(mac.today("r-end"), "ran 1 · skipped 0 idle, 0 capped");
    let sub = mac.row("r-end")["subtitle"].as_str().unwrap().to_owned();
    assert!(sub.contains("ended "), "{sub}");
    let form = [
        ("Content-Type", "application/x-www-form-urlencoded"),
        ("Cookie", mac.cooee().cookie.as_str()),
    ];
    let (head, _) = mac
        .cooee()
        .raw("POST", "/routines/r-end", &form, "paused=false");
    assert_eq!(&head[9..12], "204", "{head}");
    mac.due(&["r-end"], 10);
    assert_eq!(
        mac.sent("r-end"),
        2,
        "resumed: fresh, it runs with nothing written"
    );
}

fn model(dir: &Path, id: &str) -> Value {
    let file = fs::read_to_string(dir.join(format!("data/threads/{id}.json"))).unwrap();
    serde_json::from_str::<Value>(&file).unwrap()["thread"]["model"].clone()
}

#[test]
fn a_run_keeps_the_chats_own_model() {
    let mut mac = Mac::new("guards-model", "m2");
    assert_eq!(model(&mac.dir, &mac.chat), "m2");
    let digest = "prompt = \"digest\"\nplace = { in = \"new_thread\", who = { agent = \"fake-acp\", model = \"m2\" } }";
    mac.routines(&[("r-chat", 1, "prompt = \"chat\""), ("r-new", 2, digest)]);
    let settings = mac.dir.join("routines.toml");
    let text = fs::read_to_string(&settings).unwrap();
    let place = format!(
        "\n[routine.place]\nin = \"chat\"\nthread = \"{}\"\n\n",
        mac.chat
    );
    let (chat, new) = text.split_once(&place).unwrap();
    let new = new.replace(&place, "\n");
    fs::write(&settings, format!("{chat}{place}{new}")).unwrap();
    mac.start();
    let run = mac.ticked("r-new")["last_run"].clone();
    assert_eq!(run["outcome"], "ran", "{run}");
    assert_eq!(
        model(&mac.dir, run["thread"].as_str().unwrap()),
        "m2",
        "a new thread starts on the chat's model"
    );
    mac.ticked("r-chat");
    assert_eq!(mac.sent("r-chat"), 1);
    assert_eq!(
        model(&mac.dir, &mac.chat),
        "m2",
        "a chat run is a message on the chat's model"
    );
}

fn sign(body: &str) -> String {
    let mut key = [0u8; 64];
    key[..SECRET.len()].copy_from_slice(SECRET.as_bytes());
    let pad = |b: u8| key.iter().map(|k| k ^ b).collect::<Vec<_>>();
    let inner = Sha256::new()
        .chain_update(pad(0x36))
        .chain_update(body)
        .finalize();
    let outer = Sha256::new()
        .chain_update(pad(0x5c))
        .chain_update(inner)
        .finalize();
    outer.iter().fold("sha256=".to_owned(), |mut s, b| {
        write!(s, "{b:02x}").unwrap();
        s
    })
}

fn post_in(port: u16, id: &str, body: &str) -> (u16, String) {
    let mut c = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        c,
        "POST /hooks/in/{id} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\n\
         X-Signature-256: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        sign(body),
        body.len()
    )
    .unwrap();
    let mut out = String::new();
    c.read_to_string(&mut out).unwrap();
    let text = out.split("\r\n\r\n").nth(1).unwrap_or_default().to_owned();
    (out[9..12].parse().unwrap(), text)
}

fn watermark(dir: &Path, rid: &str) -> Value {
    let text = fs::read_to_string(dir.join("data/watermarks.json")).unwrap_or_else(|_| "{}".into());
    serde_json::from_str::<Value>(&text).unwrap()[format!("routine-{rid}")].clone()
}

#[test]
fn a_capped_signed_post_runs_nothing_but_still_feeds_the_silence_clock() {
    let mut mac = Mac::new("guards-webhook", "");
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    mac.extra = format!(
        "[hooks]\nbind = \"127.0.0.1:{port}\"\nurl = \"https://mac.example/hooks/github\"\n"
    );
    fs::create_dir_all(mac.dir.join("data/secrets")).unwrap();
    fs::write(mac.dir.join("data/secrets/BEACON"), SECRET).unwrap();
    mac.routines(&[("r-hook", 1, "prompt = \"read it\"\nper_day = 1")]);
    let settings = mac.dir.join("routines.toml");
    let text = fs::read_to_string(&settings).unwrap();
    let hook = "[routine.webhook]\nsecret = \"BEACON\"\nsilence = 60\n\n[routine.place]";
    let text = text.replace("when = \"0 1 * * *\"\n", "");
    fs::write(&settings, text.replace("[routine.place]", hook)).unwrap();
    mac.start();
    for n in 0..3 {
        let alive = format!(r#"{{"kind":"alive","n":{n}}}"#);
        assert_eq!(post_in(port, "r-hook", &alive), (200, "alive".to_owned()));
    }
    assert_eq!(
        watermark(&mac.dir, "r-hook"),
        Value::Null,
        "an alive POST counts for nothing"
    );
    assert_eq!(
        post_in(port, "r-hook", r#"{"n":1}"#),
        (202, "runs: 1".to_owned())
    );
    let heard = |mac: &Mac| {
        mac.cooee().json("/routines.json")[0]["heard"]
            .as_u64()
            .unwrap()
    };
    let first = heard(&mac);
    thread::sleep(Duration::from_millis(5));
    assert_eq!(
        post_in(port, "r-hook", r#"{"n":2}"#),
        (202, "runs: 0".to_owned())
    );
    assert!(
        heard(&mac) > first,
        "a capped POST still says the sender is alive"
    );
    let last = mac.cooee().json("/routines.json")[0]["last_run"].clone();
    assert_eq!(
        (&last["outcome"], &last["why"]),
        (&json!("skipped"), &json!("capped")),
        "{last}"
    );
    assert_eq!(mac.today("r-hook"), "ran 1 · skipped 0 idle, 1 capped");
}
