use std::{fs, path::PathBuf, thread};

use cooee_core::shed_proto::Hello;
use serde_json::json;

use crate::{
    setup::setup,
    support::{Cooee, FakeShed, scratch, version},
};

#[test]
fn a_shed_of_another_protocol_is_refused_naming_both_sides() {
    for protocol in [0, 3] {
        let dir = scratch(&format!("version-{protocol}"));
        let theirs = Hello {
            build: "fffffff".to_owned(),
            protocol,
            ..version()
        };
        let shed = FakeShed::start(&dir.join("shed"), theirs);
        let cooee = Cooee::start(&dir, &shed.dir);
        let new = json!({ "agent": "fake-acp", "folder": dir.join("root/project"), "text": "hi", "random_id": "x" });
        let (status, body) = cooee.post("/threads", &new);
        assert_eq!(status, 502, "{body}");
        let (v, b) = (&version().version, &version().build);
        let theirs = if protocol == 0 {
            format!("the server runs {v} (fffffff) protocol 0, older than protocol 1")
        } else {
            format!(
                "the server refused it: client {b} / server fffffff: protocol 1 vs 3; restart shed serve"
            )
        };
        let said = format!("SHED_VERSION: cooee speaks shed {v} ({b}) protocol 1; {theirs}");
        assert!(body.contains(&said), "{body}\nwanted: {said}");
        assert!(
            !shed.cli_log().contains(" run "),
            "nothing started: {}",
            shed.cli_log()
        );
    }
}

#[test]
fn a_send_to_a_shed_on_another_protocol_says_why_in_the_chat_then_goes_through_once_fixed() {
    let s = setup("version-send");
    let id = s.create("hi");
    let old = Hello {
        build: "fffffff".to_owned(),
        protocol: 0,
        ..version()
    };
    let shed = s.shed.restart(old);
    for n in 0..2 {
        let (status, body) = s.send(&id, "again", &format!("v-{n}"));
        assert_eq!(status, 502, "{body}");
    }
    let (v, b) = (&version().version, &version().build);
    let needs = format!(
        "SHED_VERSION: cooee speaks shed {v} ({b}) protocol 1; the server runs {v} (fffffff) protocol 0, older than protocol 1; \
         fix: install or update shed to protocol 1 or newer from https://github.com/jirathip-dev/shed/releases, and start its serve"
    );
    let notes: Vec<String> = s
        .items(&id)
        .iter()
        .filter(|i| i["kind"] == "note")
        .map(|i| i["text"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(notes.len(), 2, "{notes:?}");
    assert!(notes.iter().all(|n| n.starts_with(&needs)), "{notes:?}");
    let _fixed = shed.restart(version());
    let (status, body) = s.send(&id, "now", "v-2");
    assert_eq!(status, 200, "{body}");
    let log = || fs::read_to_string(s.dir.join("cooee.err")).unwrap();
    let cleared = "cooee: shed speaks this build's protocol again";
    for _ in 0..500 {
        if log().contains(cleared) {
            break;
        }
        thread::sleep(std::time::Duration::from_millis(20));
    }
    let log = log();
    let lines = |what: &str| log.lines().filter(|l| l.contains(what)).count();
    assert_eq!((lines("SHED_VERSION"), lines(cleared)), (1, 1), "{log}");
}

#[test]
#[ignore = "a server for apple/e2e, not a test"]
fn fake_shed_server() {
    let Some(dir) = std::env::var_os("FAKE_SHED_DIR") else {
        return;
    };
    let shed = FakeShed::start(PathBuf::from(dir).as_path(), version());
    if std::env::var_os("FAKE_SHED_LANES").is_some() {
        let lane = |name: &str, item: &str, role: &str, prompt: &str| {
            let url = format!("https://github.com/o/cooee/issues/{}", &item[1..]);
            let labels = [
                ("source", "drover"),
                ("group", "cooee"),
                ("item", item),
                ("role", role),
                ("round", "1"),
                ("url", url.as_str()),
            ];
            shed.lane(name, &labels);
            shed.report(name, "working", None);
            shed.input(name, &format!("\x1e{}\n", json!({ "prompt": prompt })));
        };
        lane("lane-impl", "#33", "impl", "hang on");
        lane("lane-rev", "#31", "rev", "approve it");
        lane("lane-done", "#30", "fix", "use a tool");
    }
    loop {
        thread::park();
    }
}
