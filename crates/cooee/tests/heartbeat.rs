//! A persona's heartbeat (#498) end to end with the fake shed and a fake APNs: switched on in its `agent.toml`, a
//! due time with nothing new starts nothing; one with news opens a routine-run thread whose run pushes only when the
//! persona's `notify.md` rule is set and the run says it matches; an action in a run is a card and nothing is done;
//! the daily limit holds; the persona panel shows the last run.

mod support;

use std::{
    fs,
    io::Write as _,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::Duration,
};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, agent, scratch, version};

fn eventually(what: &str, mut f: impl FnMut() -> bool) {
    for _ in 0..200 {
        if f() {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("never: {what}");
}

fn mark(dir: &Path) -> Value {
    let text = fs::read_to_string(dir.join("data/watermarks.json")).unwrap_or_else(|_| "{}".into());
    serde_json::from_str::<Value>(&text).unwrap()["heartbeat-ada"].clone()
}

fn due_again(dir: &Path) {
    let mut m = mark(dir);
    m["due"] = json!(1);
    let all = json!({ "heartbeat-ada": m });
    fs::write(dir.join("data/watermarks.json"), all.to_string()).unwrap();
}

fn runs(cooee: &Cooee) -> Vec<String> {
    let rows = cooee.json("/threads");
    let rows = rows.as_array().unwrap().iter();
    let rows = rows.filter(|r| r["routine"] == "Heartbeat");
    rows.map(|r| r["id"].as_str().unwrap().to_owned()).collect()
}

struct Setup {
    dir: PathBuf,
    home: PathBuf,
    shed: FakeShed,
    extra: String,
    path: String,
    log: PathBuf,
}

impl Setup {
    fn new(name: &str, heartbeat: &str) -> Self {
        let dir = scratch(name);
        let toml =
            format!("name = \"Ada\"\nagent = \"fake-acp\"\n\n[heartbeat]\non = true\n{heartbeat}");
        let home = agent(&dir, "ada", &toml, "");
        let (bin, log) = (dir.join("bin"), dir.join("pushed"));
        fs::create_dir_all(&bin).unwrap();
        fs::create_dir_all(&log).unwrap();
        let curl = format!(
            "#!/bin/sh\ncat >\"{}/$$.json\"\nprintf '\\n200'\n",
            log.display()
        );
        fs::write(bin.join("curl"), curl).unwrap();
        fs::set_permissions(bin.join("curl"), fs::Permissions::from_mode(0o755)).unwrap();
        let key = "MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQg/FMQqoOmZU6bop6k\n\
                   jwzlkZBiOA5CCW7ngc03CgGpgsKhRANCAASkJsDwiTE2wL6OIdpH5tfnsdyw5UK8\n\
                   NOnlJZkFHy9kIEFLJ8uttasmC9v/MsS+Cbb3MJBa25UlVnmKe8xoXskq";
        let p = "PRIVATE KEY-----";
        fs::create_dir_all(dir.join("data")).unwrap();
        let pem = format!("-----BEGIN {p}\n{key}\n-----END {p}\n");
        fs::write(dir.join("data/apns.p8"), pem).unwrap();
        let extra = "[apns]\nkey_id = \"K\"\nteam_id = \"T\"\ntopic = \"dev.cooee.app\"\nenv = \"sandbox\"\nurl = \"https://apns.test\"\n".to_owned();
        let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
        let shed = FakeShed::start(&dir.join("shed"), version());
        Self {
            dir,
            home,
            shed,
            extra,
            path,
            log,
        }
    }

    fn start(&self) -> Cooee {
        let cooee = Cooee::start_env(
            &self.dir,
            &self.shed.dir,
            &self.extra,
            &[("PATH", self.path.as_str())],
        );
        let token = json!({ "token": "ab".repeat(32) });
        assert_eq!(cooee.post("/push/apns", &token).0, 200);
        cooee
    }

    fn first_sight(&self) {
        let cooee = self.start();
        eventually("the first sight", || !mark(&self.dir).is_null());
        assert_eq!(
            mark(&self.dir)["line"],
            0,
            "first sight only notes the due time"
        );
        drop(cooee);
    }

    fn tick(&self) -> Cooee {
        due_again(&self.dir);
        let cooee = self.start();
        eventually("a tick", || {
            mark(&self.dir)["due"].as_u64().unwrap_or(0) > 1
        });
        cooee
    }

    fn pushes(&self) -> Vec<Value> {
        let files = fs::read_dir(&self.log).unwrap();
        let read = files.map(|f| fs::read_to_string(f.unwrap().path()).unwrap());
        read.filter_map(|t| serde_json::from_str(&t).ok()).collect()
    }

    fn beats(&self) -> Vec<(String, String)> {
        let all = self.pushes();
        let alerts = all.iter().map(|p| &p["aps"]["alert"]);
        let beats = alerts.filter(|a| a["title"].as_str().unwrap().ends_with("· heartbeat"));
        beats
            .map(|a| {
                (
                    a["title"].as_str().unwrap().to_owned(),
                    a["body"].as_str().unwrap().to_owned(),
                )
            })
            .collect()
    }
}

fn say(cooee: &Cooee, text: &str, random_id: &str) {
    let new =
        json!({ "agent": "fake-acp", "profile": "ada", "text": text, "random_id": random_id });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    eventually("its reply", || {
        let items = cooee.json(&format!("/threads/{id}/messages"));
        items["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["kind"] == "agent")
    });
}

fn ended(dir: &Path) -> Value {
    eventually("the run's end", || {
        let last = &mark(dir)["last"];
        last["pushed"] == true || last["why"].as_str().is_some_and(|w| !w.is_empty())
    });
    mark(dir)["last"].clone()
}

fn panel_row(cooee: &Cooee) -> Value {
    let screen = cooee.json("/screens/persona/ada");
    let blocks = screen["blocks"].as_array().unwrap();
    let block = blocks.iter().find(|b| b["type"] == "routines").unwrap();
    let rows = block["rows"].as_array().unwrap();
    rows.iter()
        .find(|r| r["id"] == "heartbeat")
        .unwrap()
        .clone()
}

#[test]
fn a_heartbeat_runs_only_on_news_and_pushes_only_when_its_rule_matches() {
    let s = Setup::new("heartbeat-push", "");
    s.first_sight();
    let cooee = s.tick();
    thread::sleep(Duration::from_millis(500));
    assert!(runs(&cooee).is_empty(), "nothing new: no run");
    assert_eq!(mark(&s.dir)["runs"], Value::Null, "no run counted");

    say(&cooee, "fake says: The build is red. / NOTIFY", "a");
    drop(cooee);
    let cooee = s.tick();
    eventually("one run", || runs(&cooee).len() == 1);
    let last = ended(&s.dir);
    assert_eq!(last["why"], "no notify rule", "{last}");
    assert_eq!(last["pushed"], false);
    assert!(
        last["read"].as_str().unwrap().contains("new lines"),
        "{last}"
    );
    thread::sleep(Duration::from_millis(500));
    assert_eq!(
        s.beats(),
        Vec::<(String, String)>::new(),
        "an empty rule pushes nothing"
    );
    let row = panel_row(&cooee);
    assert_eq!(row["name"], "Heartbeat", "{row}");
    let next = row["next"].as_str().unwrap();
    assert!(
        next.starts_with("last run ") && next.ends_with("· silent: no notify rule"),
        "{next}"
    );
    assert!(
        next.contains("read ") && next.contains(" characters"),
        "{next}"
    );

    fs::write(s.home.join("notify.md"), "Tell me when the build is red.\n").unwrap();
    say(&cooee, "fake says: The build is red again. / NOTIFY", "b");
    drop(cooee);
    let cooee = s.tick();
    eventually("a second run", || runs(&cooee).len() == 2);
    let last = ended(&s.dir);
    assert_eq!(last["pushed"], true, "{last}");
    eventually("its push", || !s.beats().is_empty());
    thread::sleep(Duration::from_millis(300));
    let one = (
        "Ada · heartbeat".to_owned(),
        "The build is red again.".to_owned(),
    );
    assert_eq!(s.beats(), vec![one], "one push");
    let next = panel_row(&cooee)["next"].as_str().unwrap().to_owned();
    assert!(next.ends_with("· pushed"), "{next}");
    let run = runs(&cooee).into_iter().find(|r| {
        let sent = s.shed.sent(&format!("cooee-{r}")).join("\n");
        sent.contains("again")
    });
    let sent = s.shed.sent(&format!("cooee-{}", run.unwrap())).join("\n");
    assert!(
        sent.contains("heartbeat run") && sent.contains("Tell me when the build is red."),
        "{sent}"
    );
    assert!(
        !sent.contains("The build is red. "),
        "the first run's news is not read again: {sent}"
    );
}

fn tool(shed: &FakeShed, id: &str, name: &str, args: &Value) -> (String, bool) {
    let file = shed.dir.join(format!("mcp-cooee-{id}.json"));
    eventually("its MCP file", || file.exists());
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

#[test]
fn a_heartbeat_action_is_a_card_with_nothing_done_and_its_daily_limit_holds() {
    let s = Setup::new("heartbeat-card", "per_day = 1\n");
    fs::write(s.home.join("notify.md"), "Anything at all.\n").unwrap();
    s.first_sight();
    let cooee = s.start();
    say(
        &cooee,
        "fake says: The invoice is due. / SILENT: only a reminder",
        "a",
    );
    drop(cooee);
    let cooee = s.tick();
    eventually("one run", || runs(&cooee).len() == 1);
    let run = runs(&cooee).remove(0);
    let last = ended(&s.dir);
    assert_eq!(last["why"], "only a reminder", "{last}");
    let terminal = format!("cooee-{run}");
    let log = s.shed.cli_log();
    let started = log
        .lines()
        .find(|l| l.contains(&format!("--name {terminal} ")))
        .unwrap();
    assert!(
        started.contains("--policy ask"),
        "every tool call asks: {started}"
    );

    let (said, failed) = tool(
        &s.shed,
        &run,
        "remember",
        &json!({ "text": "The invoice is due." }),
    );
    assert!(!failed && said.starts_with("Proposed."), "{said}");
    assert!(
        !s.home.join("memory.md").exists(),
        "nothing written before approval"
    );
    s.shed.ask(&terminal);
    eventually("the card", || {
        let items = cooee.json(&format!("/threads/{run}/messages"));
        let items = items["items"].as_array().unwrap().clone();
        items
            .iter()
            .any(|i| i["kind"] == "approval" && i["form"].is_null())
    });
    thread::sleep(Duration::from_millis(500));
    let answered = s
        .shed
        .sent(&terminal)
        .iter()
        .any(|t| t.contains("\"approve\""));
    assert!(!answered, "nothing approved");
    let about_run = s
        .pushes()
        .iter()
        .filter(|p| p.to_string().contains(&run))
        .count();
    assert_eq!(
        about_run, 0,
        "a heartbeat's card is a background card: no push"
    );
    assert_eq!(
        s.beats(),
        Vec::<(String, String)>::new(),
        "a silent run pushes nothing"
    );

    say(&cooee, "fake says: Another invoice. / NOTIFY", "b");
    let line = mark(&s.dir)["line"].clone();
    drop(cooee);
    let cooee = s.tick();
    thread::sleep(Duration::from_millis(500));
    assert_eq!(runs(&cooee), vec![run], "one run a day: no second run");
    assert_eq!(
        mark(&s.dir)["line"],
        line,
        "the news waits for the next allowed run"
    );
    assert_eq!(mark(&s.dir)["runs"].as_array().unwrap().len(), 1);
}
