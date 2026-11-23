use std::{fs, process::Command};

use serde_json::json;

use crate::{
    setup::{Case, Setup, id_of, listed, setup, start},
    support::{Cooee, FakeShed, scratch, version},
};

#[test]
fn r3_a_store_error_is_a_503_not_unpaired_and_a_401_does_not_latch() {
    let s = setup("r3-unpaired");
    let wrong = [("Cookie", "cooee=not-a-token")];
    let (head, _) = s.cooee.raw("GET", "/threads/x/messages", &wrong, "");
    assert!(head.starts_with("HTTP/1.0 401"), "{head}");
    let (status, body) = s.cooee.get("/threads");
    assert_eq!((status, body.as_str()), (200, "[]"));

    let devices = s.dir.join("data/devices.json");
    let kept = fs::read(&devices).unwrap();
    fs::write(&devices, "{ torn").unwrap();
    let (status, body) = s.cooee.get("/threads/x/messages");
    assert_eq!(status, 503, "{body}");
    assert!(body.contains("can't read its paired devices"), "{body}");
    assert_eq!(s.cooee.get("/threads").0, 503);
    fs::write(&devices, kept).unwrap();
    assert_eq!(s.cooee.get("/threads").0, 200);
}

#[test]
fn r7_requests_run_no_subprocess() {
    let dir = scratch("r7-no-io");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let provider = "[[providers]]\nname = \"fake\"\nagent = \"fake-acp\"\nlogin = [\"true\"]\nstatus = [\"true\"]\n";
    let cooee = Cooee::start_with(&dir, &shed.dir, provider);
    let root = dir.join("root/project");
    let new = json!({ "agent": "fake-acp", "folder": root, "text": "hi", "random_id": "r7" });
    let (code, body) = cooee.post("/threads", &new);
    assert_eq!(code, 200, "{body}");
    let id = id_of(&body);

    let statuses = || shed.calls_of("account.status");
    let before = shed.cli_log();
    for _ in 0..5 {
        assert_eq!(listed(&cooee), std::slice::from_ref(&id));
        cooee.json(&format!("/threads/{id}/messages"));
        assert_eq!(cooee.json("/logins/fake")["state"], "none");
        for path in ["/models", "/personas", "/folders"] {
            cooee.json(path);
        }
    }
    assert_eq!(shed.cli_log(), before);
    assert_eq!(statuses(), 0, "no status command runs for a request");
}

#[test]
fn agents_are_listed_from_the_hub_and_a_failed_refresh_keeps_the_list() {
    let dir = scratch("agents-cache");
    let shed = FakeShed::start(&dir.join("shed"), version());
    shed.gate("agents-fails", true);
    let s = Setup {
        cooee: Cooee::start(&dir, &shed.dir),
        dir,
        shed,
    };
    let lists = || {
        let log = s.shed.cli_log();
        log.lines().filter(|l| l.ends_with(" agents")).count()
    };
    assert_ne!(s.cooee.get("/agents").0, 200);
    s.shed.gate("agents-fails", false);
    for _ in 0..10 {
        assert_eq!(s.cooee.get("/threads").0, 200);
    }
    assert_eq!(lists(), 1, "{}", s.shed.cli_log());

    let (code, body) = s.cooee.post("/threads", &start(&s, "hi", "c1"));
    assert_eq!(code, 200, "{body}");
    assert_eq!(lists(), 2);
    assert_eq!(s.cooee.post("/threads", &start(&s, "again", "c2")).0, 200);
    assert_eq!(lists(), 2, "{}", s.shed.cli_log());

    s.shed.gate("agents-fails", true);
    let ghost = json!({ "agent": "ghost-acp", "folder": s.dir.join("root/project"), "text": "hi", "random_id": "c3" });
    s.cooee.post("/threads", &ghost);
    assert_eq!(lists(), 3);
    let err = fs::read_to_string(s.dir.join("cooee.err")).unwrap();
    assert!(
        err.contains("cooee: kept the agent list: ") && err.contains("agents down"),
        "{err}"
    );
    let (code, body) = s.cooee.post("/threads", &start(&s, "still", "c4"));
    assert_eq!(code, 200, "{body}");
    assert_eq!(lists(), 3, "{}", s.shed.cli_log());
}

#[test]
fn r13_malformed_input_or_config_is_denied_with_a_reason() {
    let s = setup("r13-closed");
    let cookie = s.cooee.cookie.clone();
    let cases: [Case; 6] = [
        ("/", vec![("Host", ""), ("Cookie", &cookie)], "403"),
        (
            "/",
            vec![("Host", "evil.example"), ("Cookie", &cookie)],
            "403",
        ),
        ("/", vec![("Origin", "null"), ("Cookie", &cookie)], "403"),
        ("/threads/x/messages", vec![("Cookie", "cooee=;;=;")], "401"),
        ("/threads/x/messages", vec![("Cookie", "cooee")], "401"),
        ("/plugin/push", vec![("Authorization", "Bearer ")], "401"),
    ];
    for (path, headers, want) in cases {
        let (head, body) = s.cooee.raw("GET", path, &headers, "");
        assert!(
            head.starts_with(&format!("HTTP/1.0 {want}")),
            "{headers:?}: {head}"
        );
        assert!(body.len() > 10, "{headers:?}: no reason: {body:?}");
    }
    let (status, _) = s.cooee.post("/threads", &json!({ "agent": 1 }));
    assert!((400..500).contains(&status), "{status}");

    let bad = s.dir.join("bad.toml");
    fs::write(&bad, "port = \"x\"\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_cooee"))
        .args(["serve", "--config"])
        .arg(&bad)
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success() && err.contains("port"), "{err}");
}

#[test]
fn r17_paths_in_text_are_never_files_served() {
    let s = setup("r17-paths");
    let png = s.dir.join("a.png");
    fs::write(&png, "not for the page").unwrap();
    let text = format!("see {} and ../x.png", png.display());
    let (status, body) = s.cooee.post("/threads", &start(&s, &text, "r17"));
    assert_eq!(status, 200, "{body}");
    let id = id_of(&body);
    let items = s.cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
    assert_eq!(items[0]["text"], text.as_str());
    assert_eq!(
        items[0].as_object().unwrap().len(),
        6,
        "only text: {}",
        items[0]
    );
    let traversals = [
        png.to_string_lossy().into_owned(),
        "/../x.png".to_owned(),
        "/%2e%2e/a.png".to_owned(),
    ];
    for path in traversals {
        let (status, body) = s.cooee.get(&path);
        assert!(status == 404 || status == 400, "{path}: {status}");
        assert!(!body.contains("not for the page"), "{path}");
    }
}
