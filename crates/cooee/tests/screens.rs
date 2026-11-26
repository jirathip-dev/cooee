//! Plugin screens as data (#125, #140): the example plugin (docs/plugins/hello-py) added from a git repo of its own,
//! its screen and live stream through cooee's `/p/<plugin>/screens/` proxy, an action through `POST /p/<plugin>/action`
//! and one outside its `[permissions]` refused before the backend sees it, and its header-strip chip on the persona
//! it's attached to. It runs with a resolver that never answers (#388: a macOS runner's reverse DNS), so the backend
//! must start without a DNS lookup.

mod support;

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    path::Path,
    process::Command,
    thread,
    time::{Duration, Instant},
};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, scratch, version};

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
}

fn hello_repo(repo: &Path) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/plugins/hello-py");
    fs::create_dir_all(repo).unwrap();
    fs::copy(src.join("hello.py"), repo.join("hello.py")).unwrap();
    let manifest = fs::read_to_string(src.join("cooee-plugin.toml")).unwrap();
    let strip = "[[places]]\ntype = \"header-strip\"\nid = \"waves\"\npersona = \"pal\"\nlabel = \"Waves\"\n\
                 screen = \"/screens/hello\"\n\n[[places]]\ntype = \"chat-cards\"\npersona = \"pal\"\n\n\
                 [permissions]\nplaces = [\"menu-row\", \"header-strip\", \"chat-cards\"]\n";
    let manifest = manifest.replace("[permissions]\nplaces = [\"menu-row\"]\n", strip);
    let manifest = format!("claims = [\"lab\"]\n{manifest}");
    fs::write(repo.join("cooee-plugin.toml"), manifest).unwrap();
    git(repo, &["init", "--quiet", "-b", "main"]);
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "--quiet", "-m", "hello"]);
}

fn no_dns(dir: &Path) -> String {
    let py = dir.join("no-dns");
    fs::create_dir_all(&py).unwrap();
    let hang = "import socket, time\n\ndef hang(*_):\n    time.sleep(60)\n    raise OSError(\"no answer\")\n\n\
                socket.gethostbyaddr = hang\n";
    fs::write(py.join("sitecustomize.py"), hang).unwrap();
    py.to_string_lossy().into_owned()
}

struct Serve(Cooee);

impl Drop for Serve {
    fn drop(&mut self) {
        let pid = self.0.child.id().to_string();
        Command::new("kill").args(["-TERM", &pid]).status().ok();
        self.0.child.wait().ok();
    }
}

fn until(what: &str, f: impl Fn() -> Result<(), String>) {
    let mut last = String::new();
    for _ in 0..400 {
        match f() {
            Ok(()) => return,
            Err(e) => last = e,
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("never: {what}; the last try: {last}");
}

fn next_data(r: &mut BufReader<TcpStream>) -> Value {
    loop {
        let mut line = String::new();
        assert!(r.read_line(&mut line).unwrap() > 0, "the stream ended");
        if let Some(d) = line.trim_end().strip_prefix("data: ") {
            return serde_json::from_str(d).unwrap();
        }
    }
}

fn add_hello_and_wait_for_its_screen(cooee: &Cooee, dir: &Path, py: &str) {
    let url = dir.join("hello.git").to_string_lossy().into_owned();
    let add = cooee
        .command()
        .env("PYTHONPATH", py)
        .args(["plugin", "add", &url, "--yes"])
        .output()
        .unwrap();
    assert!(add.status.success(), "{add:?}");
    let out = String::from_utf8(add.stdout).unwrap();
    assert!(out.contains("screens: 3 places; actions: wave"), "{out}");
    let log = dir.join("data/plugins/hello/plugin.log");
    until("its screen", || {
        let at = Instant::now();
        let (status, body) = cooee.get("/p/hello/screens/hello");
        let took = at.elapsed();
        let log = fs::read_to_string(&log).unwrap_or_default();
        (status == 200)
            .then_some(())
            .ok_or_else(|| format!("{status} after {took:?}: {body}\nplugin.log:\n{log}"))
    });
}

fn open_hello_stream(cooee: &Cooee) -> BufReader<TcpStream> {
    let mut stream = TcpStream::connect(&cooee.addr).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(20)))
        .unwrap();
    write!(
        stream,
        "GET /p/hello/screens/hello/stream HTTP/1.0\r\nHost: {}\r\nCookie: {}\r\n\r\n",
        cooee.addr, cooee.cookie
    )
    .unwrap();
    BufReader::new(stream)
}

fn assert_actions_outside_permissions_are_refused(cooee: &Cooee, wave: &Value) {
    let drop = json!({ "screen": "hello", "block": "wave", "action": "drop" });
    let (status, body) = cooee.post("/p/hello/action", &drop);
    assert_eq!(status, 403, "{body}");
    assert!(
        body.contains("drop isn't in hello's [permissions] actions"),
        "{body}"
    );
    assert_eq!(
        cooee.json("/p/hello/screens/hello")["blocks"][1]["value"],
        "1"
    );
    for off in [
        "/p/hello/screens/other",
        "/p/hello/screens/hellox",
        "/p/hello/screens/hello/../../token",
        "/p/hello/screens/hello/%2E%2E/x",
    ] {
        let (status, body) = cooee.get(off);
        assert_eq!(status, 403, "{off}: {body}");
    }
    assert_eq!(cooee.post("/p/nope/action", wave).0, 404);
}

#[test]
fn a_plugin_screen_streams_its_ops_and_an_action_outside_its_permissions_is_refused() {
    let dir = scratch("screens");
    let shed = FakeShed::start(&dir.join("shed"), version());
    hello_repo(&dir.join("hello.git"));
    let py = no_dns(&dir);
    let serve = Serve(Cooee::start_env(
        &dir,
        &shed.dir,
        "",
        &[("PYTHONPATH", &py)],
    ));
    let cooee = &serve.0;
    add_hello_and_wait_for_its_screen(cooee, &dir, &py);

    let screen = cooee.json("/p/hello/screens/hello");
    assert_eq!(
        (&screen["title"], &screen["blocks"][1]["value"]),
        (&json!("Hello"), &json!("0"))
    );
    assert_eq!(cooee.get("/p/hello/frame").0, 404);

    let mut events = open_hello_stream(cooee);
    assert_eq!(next_data(&mut events)["seq"], 0);

    let wave = json!({ "screen": "hello", "block": "wave", "action": "wave", "values": {} });
    let (status, body) = cooee.post("/p/hello/action", &wave);
    let ops: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        (status, &ops["ops"][0]["fields"]["value"]),
        (200, &json!("1"))
    );
    let live = next_data(&mut events);
    assert_eq!(
        (&live["seq"], &live["ops"][0]["id"]),
        (&json!(1), &json!("waves"))
    );

    assert_actions_outside_permissions_are_refused(cooee, &wave);

    pals_places(cooee, &dir);
    let row = &cooee.json("/plugins")["installed"][0];
    assert_eq!(
        (
            &row["permissions"]["actions"],
            &row["places"][0]["type"],
            &row["claims"]
        ),
        (&json!(["wave"]), &json!("menu-row"), &json!(["lab"]))
    );
}

fn pals_places(cooee: &Cooee, dir: &Path) {
    support::agent(
        dir,
        "chief",
        "name = \"Chief\"\nrole = \"chief\"\nagent = \"claude\"\n",
        "",
    );
    assert_eq!(cooee.json("/personas/chief/places"), json!([]));
    let pal = json!({ "name": "pal", "agent": "claude", "plugins": ["hello"], "mode": "hand-off" });
    let (status, body) = cooee.post("/personas", &pal);
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        cooee.json("/personas/pal/places"),
        json!([
            { "type": "header-strip", "plugin": "hello", "id": "waves", "label": "Waves", "screen": "/screens/hello" },
            { "type": "chat-cards", "plugin": "hello", "id": "", "label": "", "screen": "/screens/cards" }
        ])
    );
    assert_eq!(
        cooee.json("/personas/chief/places"),
        json!([{ "type": "chat-cards", "plugin": "hello", "id": "", "label": "", "screen": "/screens/cards" }])
    );
    let (status, cards) = cooee.get("/p/hello/screens/cards");
    assert_eq!(status, 200, "{cards}");
    let served: Value = serde_json::from_str(&cards).unwrap();
    assert_eq!(served["title"], "Hello");
}
