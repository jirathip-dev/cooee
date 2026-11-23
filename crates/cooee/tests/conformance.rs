//! The provider contract's conformance suite (#503, #531; docs/architecture/provider-contract/conformance.md): one
//! scenario, written as data, run through `cooee serve`'s HTTP API as the app uses it. On every PR it runs on the fake
//! shed and its scripted agent; the nightly (`.github/workflows/conformance.yml`) runs the ignored test against a
//! real shed and harness, within its ceiling, and writes the report one row per step.

mod support;

use std::{
    env,
    fmt::Write as _,
    fs,
    path::Path,
    process::Command,
    thread,
    time::{Duration, Instant},
};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, agent, scratch, version};

const BRIEF: &str = "The code word is PLOVER-7.";
const MAX_PROMPTS: usize = 10;
const MAX_RUN: Duration = Duration::from_mins(10);
const MAX_TURN: Duration = Duration::from_mins(3);
const STOP_WITHIN: Duration = Duration::from_secs(10);

struct Step {
    n: u8,
    name: &'static str,
    says: &'static str,
    stop: bool,
    tap: &'static str,
    before: fn(&Run),
    check: fn(&Run, &Value) -> Result<(), String>,
}

const ALLOW: &str = "allow_once";
const PROBE: &str = "printf conformance > probe.txt";

const SCENARIO: [Step; 8] = [
    Step {
        n: 1,
        name: "The brief is honoured",
        says: "What is the code word?",
        stop: false,
        tap: ALLOW,
        before: |_| {},
        check: |_, reply| holds(reply, "PLOVER-7"),
    },
    Step {
        n: 2,
        name: "A cooee tool is called",
        says: "Keep a note that the check ran, with your remember tool.",
        stop: false,
        tap: ALLOW,
        before: |_| {},
        check: |run, _| {
            let memory = fs::read_to_string(run.home.join("memory.md")).unwrap_or_default();
            let noted = memory
                .lines()
                .any(|l| l.contains("check ran") && l.contains("by:persona"));
            noted
                .then_some(())
                .ok_or(format!("memory.md has no note: {memory:?}"))
        },
    },
    Step {
        n: 3,
        name: "A skill is read",
        says: "Use your conformance skill.",
        stop: false,
        tap: ALLOW,
        before: |_| {},
        check: |_, reply| holds(reply, "HERON"),
    },
    Step {
        n: 4,
        name: "An approval is raised",
        says: "Run `printf conformance > probe.txt`.",
        stop: false,
        tap: "reject_once",
        before: |_| {},
        check: |run, _| {
            let cards = run.cards();
            match cards.as_slice() {
                [c] if c["action"] == "run" && c["tool"] == PROBE && c["chosen"] == "deny" => {}
                _ => return Err(format!("not one denied run card: {cards:?}")),
            }
            let probe = run.folder.join("probe.txt");
            (!probe.exists())
                .then_some(())
                .ok_or("probe.txt exists".to_owned())
        },
    },
    Step {
        n: 5,
        name: "A rule answers one",
        says: "Run `printf conformance > probe.txt`.",
        stop: false,
        tap: ALLOW,
        before: |run| {
            let rule =
                json!({ "effect": "allow", "kind": "run", "pattern": "printf conformance*" });
            let (status, body) = run.cooee.post("/personas/scout/rules", &rule);
            assert_eq!(status, 200, "{body}");
        },
        check: |run, _| {
            let cards = run.cards().len();
            let rows = run.items();
            let row = rows.iter().rfind(|i| i["kind"] == "tool");
            let ruled = row.map(|r| &r["ruled"]);
            if cards != 1
                || ruled.and_then(Value::as_str) != Some("Allow running printf conformance*")
            {
                return Err(format!("{cards} cards, the row is ruled {ruled:?}"));
            }
            let probe = fs::read_to_string(run.folder.join("probe.txt"));
            (probe.ok().as_deref() == Some("conformance"))
                .then_some(())
                .ok_or("probe.txt was not written".to_owned())
        },
    },
    Step {
        n: 6,
        name: "Stop works",
        says: "Run `sleep 120`.",
        stop: true,
        tap: ALLOW,
        before: |_| {},
        check: |run, reply| {
            if reply["stop_reason"] != "cancelled" {
                return Err(format!("the turn ended {}", reply["stop_reason"]));
            }
            let rows = run.items();
            let row = rows.iter().rfind(|i| i["kind"] == "tool");
            match row.map(|r| &r["status"]) {
                Some(s) if s == "cancelled" => {}
                s => return Err(format!("the tool row is {s:?}")),
            }
            let left = under(run.tree).into_iter().find(|(_, c)| c == "sleep 120");
            left.map_or(Ok(()), |(pid, _)| {
                Err(format!("sleep 120 is left: pid {pid}"))
            })
        },
    },
    Step {
        n: 7,
        name: "A limit is reported",
        says: "",
        stop: false,
        tap: ALLOW,
        before: |_| {},
        check: |run, _| {
            if let Some(shed) = &run.fake {
                shed.limit(&format!("cooee-{}", run.thread.as_deref().unwrap()));
                let limited = |turn: &Value| turn["state"] == "limited" && turn["note"] == "3pm";
                run.until(MAX_TURN, "limit state", |_| {
                    limited(&run.turn_state()).then_some(())
                })?;
            }
            let turn = run.turn_state();
            let used = turn["context"]["used"].as_u64().unwrap_or_default();
            (used > 0 && turn["context"]["size"].as_u64() > Some(0))
                .then_some(())
                .ok_or(format!("no context use: {turn}"))
        },
    },
    Step {
        n: 8,
        name: "Built-ins are off",
        says: "List the names of every tool you have, one per line.",
        stop: false,
        tap: ALLOW,
        before: |_| {},
        check: |run, reply| {
            let text = reply["text"].as_str().unwrap_or_default();
            let names: Vec<&str> = text
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .collect();
            let off = OFF.iter().find(|(p, _)| *p == run.provider);
            let off = off.ok_or(format!("no names turned off for {}", run.provider))?;
            let on: Vec<_> = off.1.iter().filter(|t| names.contains(t)).collect();
            on.is_empty()
                .then_some(())
                .ok_or(format!("still on: {on:?} in {text:?}"))
        },
    },
];

const OFF: [(&str, &[&str]); 4] = [
    ("fake", &["Agent", "Workflow", "Skill"]),
    ("claude", &["Agent", "Workflow", "Skill"]),
    ("codex", &["spawn_agent"]),
    ("antigravity", &["invoke_subagent", "run_workflow"]),
];

const FAKE: &str = "[[providers]]\nname = \"fake\"\nbase = \"claude\"\nagent = \"fake-acp\"\nhome = \"FAKE_HOME\"\n\
                    login = [\"true\"]\nstatus = [\"true\"]\n";

fn under(root: u32) -> Vec<(u32, String)> {
    let ps = Command::new("ps")
        .args(["-A", "-o", "pid=,ppid=,command="])
        .output()
        .unwrap();
    let ps = String::from_utf8(ps.stdout).unwrap();
    let rows: Vec<(u32, u32, String)> = ps
        .lines()
        .filter_map(|l| {
            let mut w = l.split_whitespace();
            let (pid, parent) = (w.next()?.parse().ok()?, w.next()?.parse().ok()?);
            Some((pid, parent, w.collect::<Vec<_>>().join(" ")))
        })
        .collect();
    let mut tree = vec![root];
    let mut found = Vec::new();
    while let Some(parent) = tree.pop() {
        for (pid, _, command) in rows.iter().filter(|r| r.1 == parent) {
            tree.push(*pid);
            found.push((*pid, command.clone()));
        }
    }
    found
}

fn holds(reply: &Value, word: &str) -> Result<(), String> {
    let text = reply["text"].as_str().unwrap_or_default();
    text.contains(word)
        .then_some(())
        .ok_or(format!("the reply is {text:?}"))
}

struct Run {
    cooee: Cooee,
    home: std::path::PathBuf,
    folder: std::path::PathBuf,
    agent: String,
    provider: String,
    model: Option<String>,
    thread: Option<String>,
    prompts: usize,
    tree: u32,
    fake: Option<FakeShed>,
    tap: &'static str,
}

impl Run {
    fn start(
        dir: &Path,
        (shed, extra): (&Path, &str),
        (agent_id, provider, model): (&str, &str, Option<&str>),
        tree: u32,
    ) -> Self {
        let toml = format!("name = \"Scout\"\nagent = \"{agent_id}\"\n");
        let home = agent(dir, "scout", &toml, "");
        fs::write(home.join("CHARACTER.md"), BRIEF).unwrap();
        let cooee = Cooee::start_with(dir, shed, extra);
        let skill = json!({ "name": "conformance", "description": "The suite's check.",
                            "body": "Reply with the single word HERON." });
        let (status, body) = cooee.post("/personas/scout/skills", &skill);
        assert_eq!(status, 200, "{body}");
        Self {
            cooee,
            home,
            folder: dir.join("root/project"),
            agent: agent_id.to_owned(),
            provider: provider.to_owned(),
            model: model.map(str::to_owned),
            thread: None,
            prompts: 0,
            tree,
            fake: None,
            tap: ALLOW,
        }
    }

    fn items(&self) -> Vec<Value> {
        let id = self.thread.as_deref().unwrap();
        let items = self.cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
        items.as_array().unwrap().clone()
    }

    fn turn_state(&self) -> Value {
        let id = self.thread.as_deref().unwrap();
        self.cooee.json(&format!("/threads/{id}/messages"))["turn"].clone()
    }

    fn agent_items(&self) -> Vec<Value> {
        let items = self
            .thread
            .as_ref()
            .map(|_| self.items())
            .unwrap_or_default();
        items.into_iter().filter(|i| i["kind"] == "agent").collect()
    }

    fn say(&mut self, text: &str) -> Result<(), String> {
        self.prompts += 1;
        let random_id = format!("conformance-{}", self.prompts);
        let (status, body) = if let Some(id) = &self.thread {
            let send = json!({ "text": text, "random_id": random_id });
            self.cooee.post(&format!("/threads/{id}/send"), &send)
        } else {
            let mut new = json!({ "agent": self.agent, "profile": "scout", "folder": self.folder,
                                  "text": text, "random_id": random_id });
            if let Some(m) = &self.model {
                new["model"] = json!(m);
            }
            let (status, body) = self.cooee.post("/threads", &new);
            let id = serde_json::from_str::<Value>(&body).ok();
            self.thread = id.and_then(|v| Some(v["id"].as_str()?.to_owned()));
            (status, body)
        };
        (status == 200)
            .then_some(())
            .ok_or(format!("{status}: {body}"))
    }

    fn until<T>(
        &self,
        within: Duration,
        what: &str,
        mut f: impl FnMut(&[Value]) -> Option<T>,
    ) -> Result<T, String> {
        let at = Instant::now();
        loop {
            let items = self.items();
            if let Some(t) = f(&items) {
                return Ok(t);
            }
            self.answer(&items);
            if at.elapsed() > within {
                return Err(format!("no {what} within {within:?}"));
            }
            thread::sleep(Duration::from_millis(50));
        }
    }

    fn cards(&self) -> Vec<Value> {
        let items = self.items().into_iter();
        items.filter(|i| i["kind"] == "approval").collect()
    }

    fn answer(&self, items: &[Value]) {
        let open = items
            .iter()
            .filter(|i| i["kind"] == "approval" && i["chosen"].is_null());
        for card in open {
            let options = card["options"].as_array().unwrap();
            let allow = options
                .iter()
                .find(|o| o["kind"] == self.tap)
                .unwrap_or(&options[0]);
            let tap = json!({ "item": card["id"], "option": allow["id"] });
            let id = self.thread.as_deref().unwrap();
            self.cooee.post(&format!("/threads/{id}/approve"), &tap);
        }
    }

    fn turn(&mut self, step: &Step) -> Result<Value, String> {
        self.tap = step.tap;
        (step.before)(self);
        if step.says.is_empty() {
            return Ok(Value::Null);
        }
        let replies = self.agent_items().len();
        self.say(step.says)?;
        if step.stop {
            let running = |items: &[Value]| {
                let row = items.iter().rfind(|i| i["kind"] == "tool")?;
                (row["status"] == "in_progress").then_some(())
            };
            self.until(MAX_TURN, "running tool row", running)?;
            let id = self.thread.clone().unwrap();
            let (status, body) = self
                .cooee
                .post(&format!("/threads/{id}/cancel"), &json!({}));
            if status != 200 {
                return Err(format!("cancel: {status}: {body}"));
            }
        }
        let within = if step.stop { STOP_WITHIN } else { MAX_TURN };
        self.until(within, "reply", |items| {
            items
                .iter()
                .filter(|i| i["kind"] == "agent")
                .nth(replies)
                .cloned()
        })
    }
}

fn scenario(run: &mut Run) -> Vec<(&'static Step, String)> {
    let began = Instant::now();
    let mut results = Vec::new();
    for step in &SCENARIO {
        let result = if run.prompts >= MAX_PROMPTS || began.elapsed() > MAX_RUN {
            "over ceiling".to_owned()
        } else {
            match run.turn(step).and_then(|reply| (step.check)(run, &reply)) {
                Ok(()) => "pass".to_owned(),
                Err(why) => format!("fail: {why}"),
            }
        };
        results.push((step, result));
    }
    results
}

fn report(provider: &str, results: &[(&Step, String)]) -> String {
    let mut out = format!("### conformance: {provider}\n\n| # | Step | Result |\n|---|---|---|\n");
    for (step, result) in results {
        let result = result.replace('|', "\\|").replace('\n', " ");
        writeln!(out, "| {} | {} | {result} |", step.n, step.name).unwrap();
    }
    if let Ok(file) = env::var("CONFORMANCE_REPORT") {
        fs::write(file, &out).unwrap();
    }
    out
}

#[test]
fn the_scenario_passes_on_the_fake_shed() {
    let dir = scratch("conformance");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let fake = ("fake-acp", "fake", None);
    let mut run = Run::start(&dir, (&shed.dir, FAKE), fake, std::process::id());
    run.fake = Some(shed.clone());
    let results = scenario(&mut run);
    let table = report("fake", &results);
    assert!(results.iter().all(|(_, r)| r == "pass"), "{table}");
    let terminal = format!("cooee-{}", run.thread.as_deref().unwrap());
    let sent = shed.sent(&terminal);
    let system = shed.meta(&terminal).unwrap()["systemPrompt"]["append"].clone();
    let system = system.as_str().unwrap().to_owned();
    assert!(
        system.contains(BRIEF),
        "the brief is in the system channel: {system}"
    );
    assert!(
        sent.iter()
            .all(|s| !s.contains(BRIEF) && !s.contains("HERON")),
        "the brief is not sent again, and the skill's body never: {sent:?}"
    );
    assert!(!system.contains("HERON"), "{system}");
    drop(run);
    fs::remove_dir_all(&dir).ok();
}

#[test]
#[ignore = "the nightly: a real shed and harness, on the owner's own login"]
fn the_scenario_on_a_real_harness() {
    let var = |k: &str| env::var(k).unwrap_or_else(|_| panic!("{k} is not set"));
    let dir = scratch("conformance-real");
    let shed = Path::new(&var("CONFORMANCE_SHED")).to_owned();
    let model = var("CONFORMANCE_MODEL");
    let serve = var("CONFORMANCE_SHED_PID").parse().unwrap();
    let provider = var("CONFORMANCE_PROVIDER");
    let mut run = Run::start(
        &dir,
        (&shed, ""),
        (&var("CONFORMANCE_AGENT"), &provider, Some(&model)),
        serve,
    );
    let results = scenario(&mut run);
    let table = report(&provider, &results);
    drop(run);
    fs::remove_dir_all(&dir).ok();
    assert!(results.iter().all(|(_, r)| r == "pass"), "{table}");
}
