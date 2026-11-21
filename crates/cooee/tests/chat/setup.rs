use std::path::PathBuf;

use serde_json::{Value, json};

use crate::support::{Cooee, Events, FakeShed, Sse, scratch, version};

pub struct Setup {
    pub dir: PathBuf,
    pub shed: FakeShed,
    pub cooee: Cooee,
}

pub fn setup(name: &str) -> Setup {
    let dir = scratch(name);
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    Setup { dir, shed, cooee }
}

impl Setup {
    pub fn folder(&self) -> PathBuf {
        self.dir.join("root/project")
    }

    pub fn create(&self, text: &str) -> String {
        static STARTS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = STARTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let new = json!({ "agent": "fake-acp", "folder": self.folder(), "text": text, "random_id": format!("start-{n}") });
        let (status, body) = self.cooee.post("/threads", &new);
        assert_eq!(status, 200, "{body}");
        serde_json::from_str::<Value>(&body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    pub fn events(&self, id: &str) -> Events {
        let cursor = self.cooee.json(&format!("/threads/{id}/messages"))["cursor"]
            .as_str()
            .unwrap()
            .to_owned();
        let boot = cursor.split('.').next().unwrap();
        self.cooee.events(id, &format!("{boot}.0"), false)
    }

    pub fn send(&self, id: &str, text: &str, random_id: &str) -> (u16, String) {
        self.cooee.post(
            &format!("/threads/{id}/send"),
            &json!({ "text": text, "random_id": random_id }),
        )
    }

    pub fn items(&self, id: &str) -> Vec<Value> {
        self.cooee.json(&format!("/threads/{id}/messages"))["items"]
            .as_array()
            .unwrap()
            .clone()
    }
}

pub fn text_of(events: &[Sse]) -> String {
    events
        .iter()
        .filter(|e| e.event == "text")
        .map(|e| e.data["text"].as_str().unwrap())
        .collect()
}

pub fn terminal(id: &str) -> String {
    format!("cooee-{id}")
}

pub fn restarts(s: &Setup) -> Vec<String> {
    let log = s.shed.cli_log();
    let runs = log.lines().filter(|l| l.contains(" run --acp "));
    runs.skip(1).map(str::to_owned).collect()
}

pub fn notes(s: &Setup, id: &str) -> usize {
    s.items(id).iter().filter(|i| i["kind"] == "note").count()
}
