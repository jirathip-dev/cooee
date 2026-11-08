//! Provider logins end to end: `cooee serve` against the fake shed (tests/support), which really runs a fake login
//! program in a plain terminal. Its link reaches the page, a typed code reaches the program, success shows as
//! signed in; the 10-minute timeout kills and removes the terminal; the screen is redacted and never stored.

mod support;

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, scratch, version};

const LINK: &str = "https://login.example.com/device?client_id=cooee&state=s1";

struct Setup {
    dir: PathBuf,
    shed: FakeShed,
    cooee: Cooee,
    /// A secret-shaped value the fake login prints, built at runtime.
    token: String,
}

fn script(path: &Path, body: &str) {
    fs::write(path, format!("#!/bin/sh\n{body}")).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// cooee with one provider, `fake`, for agent `fake-acp`: its login prints a link and a token, then reads a code
/// (`good-code` signs it in); its status succeeds once signed in.
fn setup(name: &str) -> Setup {
    setup_with(name, None)
}

/// As [`setup`], its status command's script `status` instead when given.
fn setup_with(name: &str, check: Option<&str>) -> Setup {
    let dir = scratch(name);
    let token = format!("sk-{}", "q7".repeat(16));
    let flag = dir.join("signed-in");
    let login = dir.join("login.sh");
    script(
        &login,
        &format!(
            "echo 'Open this link to sign in:'\necho '{LINK}'\necho 'session {token}'\necho 'Paste code here >'\n\
             read code\nif [ \"$code\" = good-code ]; then touch '{}'; echo 'Signed in.'; exit 0; fi\nexit 1\n",
            flag.display()
        ),
    );
    let status = dir.join("status.sh");
    let body = format!(
        "test -f '{}' || {{ echo 'Not signed in'; exit 1; }}\necho 'Signed in as owner@example.com'\n",
        flag.display()
    );
    script(&status, check.unwrap_or(&body));
    let shed = FakeShed::start(&dir.join("shed"), version());
    let extra = format!(
        "[[providers]]\nname = \"fake\"\nagent = \"fake-acp\"\nlogin = [{:?}]\nstatus = [{:?}]\n",
        login.to_string_lossy(),
        status.to_string_lossy()
    );
    let cooee = Cooee::start_with(&dir, &shed.dir, &extra);
    Setup {
        dir,
        shed,
        cooee,
        token,
    }
}

impl Setup {
    /// `GET /logins/fake` until `f` holds (15 s at most: the fake program's output, a status check's 10 s timeout).
    fn until(&self, f: impl Fn(&Value) -> bool) -> Value {
        let start = Instant::now();
        loop {
            let v = self.cooee.json("/logins/fake");
            if f(&v) {
                return v;
            }
            assert!(start.elapsed() < Duration::from_secs(15), "{v}");
            thread::sleep(Duration::from_millis(20));
        }
    }

    /// Starts the login and waits for its prompt; returns its terminal and view.
    fn start(&self) -> (String, Value) {
        assert_eq!(self.cooee.post("/logins/fake", &json!(null)).0, 200);
        let v = self.until(|v| v["screen"].as_str().unwrap().contains("Paste code"));
        let runs = self.shed.runs();
        assert_eq!(runs.len(), 1, "{runs:?}");
        (runs[0].clone(), v)
    }

    /// Every file under the data folder, and cooee's stderr.
    fn written(&self) -> String {
        let mut out = fs::read_to_string(self.dir.join("cooee.err")).unwrap();
        let mut dirs = vec![self.dir.join("data")];
        while let Some(dir) = dirs.pop() {
            for e in fs::read_dir(dir).unwrap().flatten() {
                if e.file_type().unwrap().is_dir() {
                    dirs.push(e.path());
                } else {
                    out.push_str(&String::from_utf8_lossy(&fs::read(e.path()).unwrap()));
                }
            }
        }
        out
    }
}

#[test]
fn login_shows_the_link_sends_the_code_and_detects_success_never_storing_the_screen() {
    let s = setup("login");
    s.until(|v| v["status"]["unknown"] == false);
    let (_, page) = s.cooee.get("/new");
    // Settings › providers: signed out, with [sign in] on its row.
    assert!(
        page.contains(r#"<span class="sk">main login</span><span class="st m">signed out</span><button type="button" class="si" data-provider="fake" data-account="""#),
        "{page}"
    );

    let (terminal, v) = s.start();
    assert_eq!(v["state"], "running");
    assert_eq!(v["links"], json!([LINK]));
    let screen = v["screen"].as_str().unwrap();
    assert!(
        screen.contains(LINK) && screen.contains("session [redacted]"),
        "{screen}"
    );
    assert!(!screen.contains(&s.token), "{screen}");
    // Terminal mode, not ACP, run in a plain shed terminal.
    let run = &s.shed.calls("run", &terminal)[0];
    assert!(run["acp"].is_null() && run["cmd"][0].as_str().unwrap().ends_with("login.sh"));

    let (status, body) = s
        .cooee
        .post("/logins/fake/send", &json!({ "text": "good-code" }));
    assert_eq!(status, 200, "{body}");
    assert_eq!(s.shed.sent(&terminal), ["good-code"]);
    assert_eq!(s.shed.calls("keys", &terminal).len(), 1);

    let v = s.until(|v| v["state"] != "running");
    assert_eq!(v["state"], "ended");
    assert_eq!(v["why"], "the login command finished");
    assert_eq!(
        v["status"],
        json!({ "signed_in": true, "unknown": false, "line": "Signed in as owner@example.com", "install": null, "who": null, "usage": null })
    );
    // Exited: killed (already gone) and removed.
    assert_eq!(s.shed.calls("kill", &terminal).len(), 1);
    assert_eq!(s.shed.calls("rm", &terminal).len(), 1);
    let (_, page) = s.cooee.get("/new");
    assert!(
        page.contains(r#"<span class="sk">main login</span><span class="st g">✓ signed in</span>"#)
            && !page.contains(r#"class="si" data-provider="fake" data-account="""#),
        "{page}"
    );

    let written = s.written();
    for secret in [s.token.as_str(), "Paste code", "good-code", LINK] {
        assert!(!written.contains(secret), "{secret} was written");
    }
    // No login runs now: a code has nowhere to go.
    assert_eq!(
        s.cooee.post("/logins/fake/send", &json!({ "text": "x" })).0,
        400
    );
}

#[test]
fn a_login_left_for_10_minutes_is_killed_and_removed() {
    let s = setup("login-timeout");
    let (terminal, _) = s.start();
    let wait = &s.shed.calls("wait", &terminal)[0];
    assert_eq!(wait["timeout"], json!(600.0));
    assert!(!s.shed.exited(&terminal));

    s.shed.expire(&terminal);
    let v = s.until(|v| v["state"] != "running");
    assert_eq!(v["why"], "timed out after 10 minutes");
    assert_eq!(v["status"]["signed_in"], false);
    assert_eq!(s.shed.calls("kill", &terminal).len(), 1);
    assert_eq!(s.shed.calls("rm", &terminal).len(), 1);
    let start = Instant::now();
    while !s.shed.exited(&terminal) {
        assert!(start.elapsed() < Duration::from_secs(10), "still running");
        thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(s.cooee.post("/logins/nope", &json!(null)).0, 400);
}

/// #106: while the status command hangs, requests answer at once with the last known status (unknown, no sign-in
/// offered); it runs once, however often the card opens, and its timeout reads as unknown, never signed out.
#[test]
fn a_hanging_status_check_never_holds_a_request_and_runs_once() {
    let dir = scratch("login-hang-runs");
    let runs = dir.join("runs");
    let s = setup_with(
        "login-hang",
        Some(&format!(
            "echo run >> '{}'\nexec sleep 30\n",
            runs.display()
        )),
    );
    let ran = || {
        fs::read_to_string(&runs)
            .unwrap_or_default()
            .lines()
            .count()
    };
    let start = Instant::now();
    while ran() == 0 {
        assert!(start.elapsed() < Duration::from_secs(10), "no check ran");
        thread::sleep(Duration::from_millis(20));
    }
    let start = Instant::now();
    let v = s.cooee.json("/logins/fake");
    let (_, page) = s.cooee.get("/new");
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "{:?}",
        start.elapsed()
    );
    assert_eq!(v["status"]["unknown"], true, "{v}");
    assert_eq!(v["status"]["line"], "not checked yet");
    assert!(v["age"].is_null(), "{v}");
    let no_sign_in = r#"class="si" data-provider="fake" data-account="""#;
    assert!(
        page.contains(r#"<span class="st m">not checked yet</span>"#) && !page.contains(no_sign_in),
        "{page}"
    );
    assert_eq!(s.cooee.post("/logins", &json!({})).0, 200);

    let v = s.until(|v| v["status"]["line"] != "not checked yet");
    assert_eq!(
        (&v["status"]["unknown"], &v["status"]["signed_in"]),
        (&json!(true), &json!(false)),
        "{v}"
    );
    assert_eq!(
        v["status"]["line"],
        "can't tell: the status command timed out"
    );
    let (_, page) = s.cooee.get("/new");
    assert!(
        page.contains(r#"<span class="st m">can't tell: the status command timed out</span>"#)
            && !page.contains(no_sign_in),
        "{page}"
    );
    // Backing off: the card opening again starts no second check.
    assert_eq!(s.cooee.post("/logins", &json!({})).0, 200);
    thread::sleep(Duration::from_millis(1500));
    assert_eq!(ran(), 1);
}
