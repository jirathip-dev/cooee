//! Reliability (#24, herdsman#553's "Reliability" rows): herdsman's fixed bug classes that apply to cooee, each
//! run against `cooee serve` and the fake shed (tests/support). Rows proven in a crate's own tests name it here.
//! R2: cooee-core `upgrade_compat`; R6, R15: cooee-server `hub`; R16: cooee-core `chat`; R18: cooee-core `devices`.

mod support;

use std::{
    fs,
    io::{Read, Write},
    net::TcpStream,
    path::PathBuf,
    process::Command,
    thread,
    time::{Duration, Instant},
};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, scratch, version};

/// A request's path, headers and the status it must get.
type Case<'a> = (&'a str, Vec<(&'a str, &'a str)>, &'a str);

struct Setup {
    dir: PathBuf,
    shed: FakeShed,
    cooee: Cooee,
}

fn setup(name: &str) -> Setup {
    let dir = scratch(name);
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    Setup { dir, shed, cooee }
}

/// Polls `f` until it holds (10 s at most).
fn until(what: &str, f: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !f() {
        assert!(Instant::now() < deadline, "never: {what}");
        thread::sleep(Duration::from_millis(20));
    }
}

/// A new-thread request with `random_id`, as the page sends it.
fn start(s: &Setup, text: &str, random_id: &str) -> Value {
    json!({ "agent": "fake-acp", "folder": s.dir.join("root/project"), "text": text, "random_id": random_id })
}

fn id_of(body: &str) -> String {
    serde_json::from_str::<Value>(body).unwrap()["id"]
        .as_str()
        .unwrap_or_else(|| panic!("{body}"))
        .to_owned()
}

/// The `shed run` lines of the fake CLI.
fn runs(shed: &FakeShed) -> usize {
    shed.cli_log()
        .lines()
        .filter(|l| l.contains(" run "))
        .count()
}

/// The prompts shed got for thread `id`.
fn prompts(shed: &FakeShed, id: &str) -> Vec<String> {
    let sent = shed.sent(&format!("cooee-{id}"));
    sent.into_iter()
        .filter(|t| t.contains("\"prompt\""))
        .collect()
}

/// The thread ids `GET /threads` lists.
fn listed(cooee: &Cooee) -> Vec<String> {
    let rows = cooee.json("/threads");
    let ids = rows.as_array().unwrap().iter();
    ids.map(|r| r["id"].as_str().unwrap().to_owned()).collect()
}

/// A POST that may never get its answer (the server is killed under it).
fn post_raw(addr: &str, cookie: &str, path: &str, body: &Value) -> Option<String> {
    let mut stream = TcpStream::connect(addr).ok()?;
    let body = body.to_string();
    let request = format!(
        "POST {path} HTTP/1.0\r\nHost: {addr}\r\nCookie: {cookie}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).ok()?;
    let mut answer = String::new();
    stream.read_to_string(&mut answer).ok()?;
    Some(answer)
}

/// R1 (herdsman#548, #556): a keychain that never answers (an unseen prompt after a rebuild) is a clear error within
/// 5 s, and other requests are served meanwhile.
#[cfg(target_os = "macos")]
#[test]
fn r1_a_hung_keychain_is_a_clear_error_within_5_s_and_blocks_nothing() {
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch("r1-keychain");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    fs::write(bin.join("security"), "#!/bin/sh\nexec sleep 60\n").unwrap();
    fs::set_permissions(bin.join("security"), fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
    let cooee = Cooee::start_env(&dir, &shed.dir, "", &[("PATH", path.as_str())]);
    thread::scope(|s| {
        let key = s.spawn(|| {
            let at = Instant::now();
            (cooee.get("/push/key"), at.elapsed())
        });
        // While the keychain hangs, the rest answers at once.
        let at = Instant::now();
        assert_eq!(cooee.get("/devices").0, 200);
        assert!(at.elapsed() < Duration::from_secs(2), "{:?}", at.elapsed());
        let ((status, body), took) = key.join().unwrap();
        assert_eq!(status, 500, "{body}");
        assert!(body.contains("keychain didn't answer in 5 s"), "{body}");
        assert!(took < Duration::from_secs(7), "{took:?}");
    });
}

/// R3 (herdsman#501): a 401 then a 200 leaves no pairing page, and a devices store that can't be read is a 503,
/// never "pair this device first".
#[test]
fn r3_a_store_error_is_a_503_not_unpaired_and_a_401_does_not_latch() {
    let s = setup("r3-unpaired");
    let wrong = [("Cookie", "cooee=not-a-token")];
    let (head, _) = s.cooee.raw("GET", "/threads/x/messages", &wrong, "");
    assert!(head.starts_with("HTTP/1.0 401"), "{head}");
    let (status, page) = s.cooee.get("/");
    assert_eq!(status, 200, "{page}");
    assert!(!page.contains("pairing code"), "{page}");

    let devices = s.dir.join("data/devices.json");
    let kept = fs::read(&devices).unwrap();
    fs::write(&devices, "{ torn").unwrap();
    let (status, body) = s.cooee.get("/threads/x/messages");
    assert_eq!(status, 503, "{body}");
    assert!(body.contains("can't read its paired devices"), "{body}");
    // A page load too: no redirect to pairing.
    assert_eq!(s.cooee.get("/").0, 503);
    fs::write(&devices, kept).unwrap();
    assert_eq!(s.cooee.get("/").0, 200);
}

/// R7 (herdsman#337): a page load runs no subprocess: the agents and the sign-ins' status are read as last checked
/// (at start, then in the background), never per request.
#[test]
fn r7_page_loads_run_no_subprocess() {
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch("r7-no-io");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let status = dir.join("status.sh");
    let log = dir.join("status.log");
    fs::write(
        &status,
        format!("#!/bin/sh\necho checked >>{log:?}\necho 'Signed in'\n"),
    )
    .unwrap();
    fs::set_permissions(&status, fs::Permissions::from_mode(0o755)).unwrap();
    let provider = format!(
        "[[providers]]\nname = \"fake\"\nagent = \"fake-acp\"\nlogin = [\"true\"]\nstatus = [{:?}]\n",
        status.to_string_lossy()
    );
    let cooee = Cooee::start_with(&dir, &shed.dir, &provider);
    let root = dir.join("root/project");
    let new = json!({ "agent": "fake-acp", "folder": root, "text": "hi", "random_id": "r7" });
    let (code, body) = cooee.post("/threads", &new);
    assert_eq!(code, 200, "{body}");
    let id = id_of(&body);

    let calls = || (shed.cli_log(), fs::read_to_string(&log).unwrap());
    let before = calls();
    assert_eq!(before.1, "checked\n", "checked once, at start");
    for _ in 0..5 {
        let (code, page) = cooee.get("/");
        assert!(code == 200 && page.contains("fake-acp"), "{page}");
        assert!(page.contains("Signed in"), "{page}");
        assert_eq!(cooee.get(&format!("/t/{id}")).0, 200);
    }
    assert_eq!(calls(), before);
}

/// R10 (herdsman#446): an ask the agent makes while its start is still running is listed and answerable.
#[test]
fn r10_an_ask_made_during_the_start_is_listed_and_answerable() {
    let s = setup("r10-early-ask");
    s.shed.gate("hold-run", true);
    thread::scope(|scope| {
        let created = scope.spawn(|| s.cooee.post("/threads", &start(&s, "hi", "r10")));
        until("the start runs", || runs(&s.shed) == 1);
        let log = s.shed.cli_log();
        let name = log
            .split("--name ")
            .nth(1)
            .unwrap()
            .split(' ')
            .next()
            .unwrap();
        s.shed.ask(name);
        s.shed.gate("hold-run", false);
        let (status, body) = created.join().unwrap();
        assert_eq!(status, 200, "{body}");
        let id = id_of(&body);
        let items = s.cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
        let ask = items
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["kind"] == "approval")
            .unwrap_or_else(|| panic!("the ask is missing: {items}"));
        assert_eq!(ask["tool"], "Edit early.rs");
        let tap = json!({ "item": ask["id"], "option": "allow" });
        let (status, body) = s.cooee.post(&format!("/threads/{id}/approve"), &tap);
        assert_eq!(status, 200, "{body}");
        let sent = s.shed.sent(&format!("cooee-{id}"));
        assert!(
            sent.contains(&"\x1e{\"approve\":\"allow\"}\n".to_owned()),
            "{sent:?}"
        );
    });
}

/// R11 (herdsman#477, #359): a start that times out is adopted once: a retry gets the same thread, shed starts one
/// terminal, the first message goes once; and a start whose CLI failed but whose terminal came up is kept.
#[test]
fn r11_a_slow_start_is_adopted_once() {
    let s = setup("r11-slow-start");
    s.shed.gate("ready-timeout", true);
    let (status, body) = s.cooee.post("/threads", &start(&s, "hi", "r11"));
    assert_eq!(status, 502, "{body}");
    let first = listed(&s.cooee);
    assert_eq!(first.len(), 1, "the slow start is listed: {first:?}");
    s.shed.gate("ready-timeout", false);
    let (status, body) = s.cooee.post("/threads", &start(&s, "hi", "r11"));
    assert_eq!(status, 200, "{body}");
    assert_eq!(id_of(&body), first[0]);
    assert_eq!(runs(&s.shed), 1);
    assert_eq!(prompts(&s.shed, &first[0]).len(), 1);
    assert_eq!(listed(&s.cooee), first);

    // The CLI gave up, but the terminal runs: kept, never started again.
    s.shed.gate("run-fails", true);
    let (status, body) = s.cooee.post("/threads", &start(&s, "hello", "r11b"));
    assert_eq!(status, 200, "{body}");
    s.shed.gate("run-fails", false);
    let id = id_of(&body);
    assert_eq!(runs(&s.shed), 2);
    assert_eq!(prompts(&s.shed, &id).len(), 1);
    assert_eq!(listed(&s.cooee).len(), 2);
}

/// R13 (herdsman#508): the gate fails closed: a malformed Host, Origin, cookie or plugin token is denied with a
/// reason, and a malformed config stops `cooee serve` with one, never serving with a default.
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

/// R14 (herdsman#509): cooee is killed after shed started the agent and before the first message went: on restart
/// the retried start adopts that thread: one terminal, one first message, one thread.
#[test]
fn r14_a_crash_between_start_and_send_is_recovered_once() {
    let dir = scratch("r14-crash");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    let body = json!({ "agent": "fake-acp", "folder": dir.join("root/project"), "text": "hi", "random_id": "r14" });
    shed.gate("hold-ready", true);
    let (addr, cookie, b) = (cooee.addr.clone(), cooee.cookie.clone(), body.clone());
    let lost = thread::spawn(move || post_raw(&addr, &cookie, "/threads", &b));
    until("the agent started", || runs(&shed) == 1);
    let killed = Command::new("kill")
        .args(["-9", &cooee.child.id().to_string()])
        .status();
    assert!(killed.unwrap().success());
    let _ = lost.join();
    drop(cooee);
    shed.gate("hold-ready", false);

    let cooee = Cooee::start(&dir, &shed.dir);
    let ids = listed(&cooee);
    assert_eq!(ids.len(), 1, "{ids:?}");
    let (status, answer) = cooee.post("/threads", &body);
    assert_eq!(status, 200, "{answer}");
    assert_eq!(id_of(&answer), ids[0]);
    assert_eq!(runs(&shed), 1);
    assert_eq!(prompts(&shed, &ids[0]).len(), 1);
    assert_eq!(listed(&cooee), ids);
}

/// R16 (herdsman#516): a stored thread whose rows repeat one id shows that row once, refreshed 5 times; and the
/// thread list shows each thread once.
#[test]
fn r16_a_repeated_id_is_one_row_however_often_it_is_refreshed() {
    let dir = scratch("r16-dup");
    let threads = dir.join("data/threads");
    fs::create_dir_all(&threads).unwrap();
    fs::write(
        threads.join("dup.json"),
        r#"{"format":1,"thread":{"id":"dup","title":"Dup","messages":[],"terminal":"cooee-dup"}}"#,
    )
    .unwrap();
    let line = |text: &str| {
        format!("{{\"id\":1,\"at\":1,\"kind\":\"you\",\"text\":\"{text}\",\"random_id\":\"r\"}}\n")
    };
    let lines: String = (0..5).map(|i| line(&format!("v{i}"))).collect();
    fs::write(threads.join("dup.jsonl"), lines).unwrap();
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    for _ in 0..5 {
        let items = cooee.json("/threads/dup/messages")["items"].clone();
        assert_eq!(items.as_array().unwrap().len(), 1, "{items}");
        assert_eq!(items[0]["text"], "v4");
        assert_eq!(listed(&cooee), ["dup"]);
    }
}

/// R17 (herdsman#547): text that names files is never a file served or an image: a message saying `/tmp/a.png`
/// or `../x.png` comes back as text, and those paths (and traversals under `/look/`) are not served.
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
        "/look/../x.png".to_owned(),
        "/look/farm/../../a.png".to_owned(),
        "/look/%2e%2e/%2e%2e/a.png".to_owned(),
        format!("/look/farm/{}", png.display()),
    ];
    for path in traversals {
        let (status, body) = s.cooee.get(&path);
        assert!(status == 404 || status == 400, "{path}: {status}");
        assert!(!body.contains("not for the page"), "{path}");
    }
}

/// R20 (herdsman#548): a delivery stuck in shed holds the thread no longer than its call's timeout: a second
/// message waiting behind it is answered "not delivered" within 30 s.
#[test]
fn r20_a_stuck_delivery_never_holds_the_next_one() {
    let s = setup("r20-stuck");
    let (status, body) = s.cooee.post("/threads", &start(&s, "hi", "r20"));
    assert_eq!(status, 200, "{body}");
    let id = id_of(&body);
    let path = format!("/threads/{id}/send");
    s.shed.gate("hang-send", true);
    thread::scope(|scope| {
        let first = scope.spawn(|| {
            s.cooee
                .post(&path, &json!({ "text": "one", "random_id": "a" }))
        });
        until("the first send is stuck in shed", || {
            prompts(&s.shed, &id).len() == 2
        });
        let at = Instant::now();
        let (status, body) = s
            .cooee
            .post(&path, &json!({ "text": "two", "random_id": "b" }));
        let took = at.elapsed();
        assert!(status >= 500, "{status}: {body}");
        assert!(took < Duration::from_secs(30), "{took:?}");
        assert_ne!(first.join().unwrap().0, 200);
    });
    s.shed.gate("hang-send", false);
    // Neither was stored as sent: the page offers a retry.
    let items = s.cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
    assert!(!items.to_string().contains("\"two\""), "{items}");
}

/// R21 (herdsman#325, #143): an answer to a session that is gone says "not delivered", and the ask stays open.
#[test]
fn r21_an_answer_to_a_gone_session_says_not_delivered() {
    let s = setup("r21-gone");
    let (status, body) = s
        .cooee
        .post("/threads", &start(&s, "please approve an edit", "r21"));
    assert_eq!(status, 200, "{body}");
    let id = id_of(&body);
    let approval = || {
        let items = s.cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
        let items = items.as_array().unwrap().clone();
        items.into_iter().find(|i| i["kind"] == "approval")
    };
    until("the agent asks", || approval().is_some());
    s.shed.vanish(&format!("cooee-{id}"));
    let ask = approval().unwrap();
    let tap = json!({ "item": ask["id"], "option": "allow" });
    let (status, body) = s.cooee.post(&format!("/threads/{id}/approve"), &tap);
    assert_eq!(
        (status, body.as_str()),
        (410, "Not delivered: this thread's agent is gone.")
    );
    let ask = approval().unwrap();
    assert!(
        ask.get("tapped").is_none() && ask.get("chosen").is_none(),
        "{ask}"
    );
    // A message too.
    let (status, body) = s.cooee.post(
        &format!("/threads/{id}/send"),
        &json!({ "text": "hi", "random_id": "z" }),
    );
    assert_eq!(
        (status, body.as_str()),
        (410, "Not delivered: this thread's agent is gone.")
    );
}
