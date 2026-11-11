//! Plugin screens as data (#125, #140): the example plugin (docs/plugins/hello-py) added from a git repo of its own,
//! its screen and live stream through cooee's `/p/<plugin>/screens/` proxy, an action through `POST /p/<plugin>/action`
//! and one outside its `[permissions]` refused before the backend sees it, and its header-strip chip on the persona
//! it's attached to.

mod support;

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    path::Path,
    process::Command,
    thread,
    time::Duration,
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

/// docs/plugins/hello-py in a repo of its own, its manifest with a header-strip chip for the `pal` persona too.
fn hello_repo(repo: &Path) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/plugins/hello-py");
    fs::create_dir_all(repo).unwrap();
    fs::copy(src.join("hello.py"), repo.join("hello.py")).unwrap();
    let manifest = fs::read_to_string(src.join("cooee-plugin.toml")).unwrap();
    let strip = "[[places]]\ntype = \"header-strip\"\nid = \"waves\"\npersona = \"pal\"\nlabel = \"Waves\"\n\
                 screen = \"/screens/hello\"\n\n[permissions]\nplaces = [\"menu-row\", \"header-strip\"]\n";
    let manifest = manifest.replace("[permissions]\nplaces = [\"menu-row\"]\n", strip);
    let manifest = format!("claims = [\"lab\"]\n{manifest}");
    fs::write(repo.join("cooee-plugin.toml"), manifest).unwrap();
    git(repo, &["init", "--quiet", "-b", "main"]);
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "--quiet", "-m", "hello"]);
}

/// `cooee serve`, sent SIGTERM when dropped (a failed assert too), so it stops the backends it runs.
struct Serve(Cooee);

impl Drop for Serve {
    fn drop(&mut self) {
        let pid = self.0.child.id().to_string();
        let _ = Command::new("kill").args(["-TERM", &pid]).status();
        let _ = self.0.child.wait();
    }
}

/// Waits (up to 20 s) for `f`.
fn until(what: &str, f: impl Fn() -> bool) {
    for _ in 0..400 {
        if f() {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("never: {what}");
}

/// The `data:` of the next event on an open stream.
fn next_data(r: &mut BufReader<TcpStream>) -> Value {
    loop {
        let mut line = String::new();
        assert!(r.read_line(&mut line).unwrap() > 0, "the stream ended");
        if let Some(d) = line.trim_end().strip_prefix("data: ") {
            return serde_json::from_str(d).unwrap();
        }
    }
}

#[test]
fn a_plugin_screen_streams_its_ops_and_an_action_outside_its_permissions_is_refused() {
    let dir = scratch("screens");
    let shed = FakeShed::start(&dir.join("shed"), version());
    hello_repo(&dir.join("hello.git"));
    let serve = Serve(Cooee::start(&dir, &shed.dir));
    let cooee = &serve.0;
    let url = dir.join("hello.git").to_string_lossy().into_owned();
    let out = cooee.cli(&["plugin", "add", &url, "--yes"]);
    assert!(out.contains("screens: 2 places; actions: wave"), "{out}");
    until("its screen", || {
        cooee.get("/p/hello/screens/hello").0 == 200
    });

    let screen = cooee.json("/p/hello/screens/hello");
    assert_eq!(
        (&screen["title"], &screen["blocks"][1]["value"]),
        (&json!("Hello"), &json!("0"))
    );
    // A screens-only plugin has no PWA tab.
    assert_eq!(cooee.get("/p/hello/frame").0, 404);

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
    let mut events = BufReader::new(stream);
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

    // Not in its [permissions] actions: refused here, never sent (the backend would count it as a wave).
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
    // A screen path its places don't declare is refused here too.
    for off in [
        "/p/hello/screens/other",
        "/p/hello/screens/hellox",
        "/p/hello/screens/hello/../../token",
        "/p/hello/screens/hello/%2E%2E/x",
    ] {
        let (status, body) = cooee.get(off);
        assert_eq!(status, 403, "{off}: {body}");
    }
    // A plugin that isn't installed has no screens.
    assert_eq!(cooee.post("/p/nope/action", &wave).0, 404);

    // Its header-strip chip shows on the persona it's attached to, and its permissions for the attach sheet.
    let pal = json!({ "name": "pal", "agent": "claude", "plugins": ["hello"] });
    let (status, body) = cooee.post("/personas", &pal);
    assert_eq!(status, 200, "{body}");
    let chips = cooee.json("/personas/pal/places");
    assert_eq!(
        chips,
        json!([{ "plugin": "hello", "id": "waves", "label": "Waves", "screen": "/screens/hello" }])
    );
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
