//! Provider logins end to end: `cooee serve` against the fake shed (tests/support), which really runs a fake login
//! program in a plain terminal. Its link reaches `GET /logins/{id}`, a typed code reaches the program, success shows as
//! signed in; the 10-minute timeout kills and removes the terminal; the screen is redacted and never stored.

mod support;

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, scratch, version};

const LINK: &str = "https://login.example.com/device?client_id=cooee&state=s1";

const PROVIDERS: &str = "[[providers]]\nname = \"fake\"\nagent = \"fake-acp\"\nlogin = [\"fake\"]\nstatus = [\"fake\"]\n";

struct Setup {
    dir: PathBuf,
    shed: FakeShed,
    cooee: Cooee,
}

fn setup(name: &str) -> Setup {
    let dir = scratch(name);
    let shed = FakeShed::start(&dir.join("shed"), version());
    let login = shed.dir.join("login-fake");
    fs::write(
        &login,
        format!(
            "#!/bin/sh\necho 'Open this link to sign in:'\necho '{LINK}'\necho 'Paste code here >'\nread code\n\
             if [ \"$code\" = good-code ]; then touch \"$1\"; echo 'Signed in.'; exit 0; fi\nexit 1\n"
        ),
    )
    .unwrap();
    fs::set_permissions(&login, fs::Permissions::from_mode(0o755)).unwrap();
    let cooee = Cooee::start_with(&dir, &shed.dir, PROVIDERS);
    Setup { dir, shed, cooee }
}

impl Setup {
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

    fn sign_in(&self, start: &Value, code: &str) -> (String, Value) {
        let (status, body) = self.cooee.post("/logins/fake", start);
        assert_eq!(status, 200, "{body}");
        let v = self.until(|v| v["screen"].as_str().unwrap().contains("Paste code"));
        assert_eq!(v["links"], json!([LINK]), "{v}");
        let terminal = self.shed.runs().last().unwrap().clone();
        let (status, body) = self
            .cooee
            .post("/logins/fake/send", &json!({ "text": code }));
        assert_eq!(status, 200, "{body}");
        assert_eq!(self.shed.sent(&terminal), [code]);
        (terminal, self.until(|v| v["state"] == "ended"))
    }

    fn sheet(&self) -> Vec<(String, String)> {
        let v = self.cooee.json("/screens/settings/signins");
        let rows = v["blocks"][0]["blocks"][0]["rows"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let field = |r: &Value, k: &str| r[k].as_str().map(str::to_owned).unwrap_or_default();
        rows.iter()
            .map(|r| (field(r, "title"), field(&r["subtitle"], "text")))
            .collect()
    }

    fn thread(&self, account: &str) -> String {
        fs::create_dir_all(self.dir.join("root")).unwrap();
        let new = json!({ "agent": "fake-acp", "account": account, "folder": self.dir.join("root"),
                          "text": "hi", "random_id": format!("r-{account}") });
        let (status, body) = self.cooee.post("/threads", &new);
        assert_eq!(status, 200, "{body}");
        let log = self.shed.cli_log();
        log.lines()
            .rfind(|l| l.contains(" run --acp"))
            .unwrap()
            .to_owned()
    }

    fn notes(&self, id: &str, n: usize) -> Vec<String> {
        let start = Instant::now();
        loop {
            let items = self.cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
            let notes: Vec<String> = (items.as_array().unwrap().iter())
                .filter(|i| i["kind"] == "note")
                .map(|i| i["text"].as_str().unwrap().to_owned())
                .collect();
            if notes.len() >= n || start.elapsed() > Duration::from_secs(10) {
                return notes;
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

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
fn a_new_account_signs_in_through_shed_a_thread_runs_on_it_and_sign_out_removes_it() {
    let s = setup("login-new");
    let (terminal, v) = s.sign_in(&json!({ "new": true }), "good-code");
    assert_eq!(terminal, "login-fake-account-2");
    assert_eq!(
        s.shed.calls("account.add", "account-2"),
        [json!({ "provider": "fake", "name": "account-2" })]
    );
    assert_eq!(v["why"], "the login command finished", "{v}");
    assert_eq!(
        (&v["account"]["state"], &v["account"]["who"]),
        (&json!("ready"), &json!("account-2@example.com")),
        "{v}"
    );
    assert_eq!(s.shed.calls("rm", &terminal).len(), 1);
    assert_eq!(
        s.sheet(),
        [(
            "fake · account-2@example.com".to_owned(),
            "signed in".to_owned()
        )]
    );

    let line = s.thread("account-2");
    assert!(line.contains(" --account account-2"), "{line}");
    assert!(!line.contains(" -- env"), "no folder variable: {line}");

    let (status, body) = s
        .cooee
        .post("/logins/fake/signout", &json!({ "account": "account-2" }));
    assert_eq!(status, 200, "{body}");
    assert!(s.shed.account_of("fake", "account-2").is_none());
    assert_eq!(s.sheet(), Vec::<(String, String)>::new());
    let (status, _) = s
        .cooee
        .post("/logins/fake/signout", &json!({ "account": "default" }));
    assert_eq!(status, 502, "shed keeps default");

    let written = s.written();
    for secret in ["Paste code", "good-code", LINK] {
        assert!(!written.contains(secret), "{secret} was written");
    }
    assert_eq!(
        s.cooee.post("/logins/fake/send", &json!({ "text": "x" })).0,
        400
    );
}

#[test]
fn a_thread_on_auto_runs_on_the_account_shed_picks_and_says_each_switch() {
    let s = setup("login-auto");
    s.sign_in(&json!({ "new": true }), "good-code");
    let line = s.thread("");
    assert!(line.contains(" --account auto"), "unset is auto: {line}");
    let terminal = line
        .split("--name ")
        .nth(1)
        .unwrap()
        .split(' ')
        .next()
        .unwrap();
    s.shed.on_account(terminal, "account-2", None);
    s.shed.on_account(terminal, "account-3", Some("account-2"));
    s.shed.limit(terminal);
    let said = [
        "on account-2",
        "account-2 hit its limit; now on account-3",
        "every fake-acp account is limited until 3pm",
    ];
    assert_eq!(s.notes(terminal.strip_prefix("cooee-").unwrap(), 3), said);
    let own = s.thread("default");
    assert!(!own.contains("--account"), "{own}");
}

#[test]
fn every_account_limited_says_so_with_the_first_reset_and_auto_without_a_named_account_is_the_own_login()
 {
    let s = setup("login-limited");
    s.sign_in(&json!({ "new": true }), "good-code");
    let resets = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 3600;
    let refused = format!(
        "ACCOUNT_LIMITED: every account is limited; the first resets at {resets} (unix seconds)"
    );
    fs::write(s.shed.dir.join("run-fails"), refused).unwrap();
    let new = json!({ "agent": "fake-acp", "folder": s.dir.join("root"), "text": "hi", "random_id": "limited" });
    let (status, body) = s.cooee.post("/threads", &new);
    assert_eq!(
        (status, body.as_str()),
        (429, "All accounts are limited; the first resets in 1h 00m.")
    );
    s.shed.gate("run-fails", false);
    let (status, body) = s
        .cooee
        .post("/logins/fake/signout", &json!({ "account": "account-2" }));
    assert_eq!(status, 200, "{body}");
    let line = s.thread("auto");
    assert!(!line.contains("--account"), "{line}");
}

#[test]
fn a_new_account_whose_login_fails_is_removed_again() {
    let s = setup("login-new-fails");
    let (_, v) = s.sign_in(&json!({ "new": true }), "bad-code");
    assert_eq!(v["why"], "the login command exited 1", "{v}");
    assert!(v["account"].is_null(), "{v}");
    assert_eq!(s.shed.calls("account.rm", "account-2").len(), 1);
    assert!(s.shed.account_of("fake", "account-2").is_none());
    assert_eq!(s.cooee.post("/logins/nope", &json!(null)).0, 400);
}

#[test]
fn a_login_left_for_10_minutes_ends_and_its_terminal_is_removed() {
    let s = setup("login-timeout");
    assert_eq!(s.cooee.post("/logins/fake", &json!(null)).0, 200);
    s.until(|v| v["state"] == "running");
    let terminal = "login-fake-default";
    let start = Instant::now();
    let wait = loop {
        if let Some(w) = s.shed.calls("wait", terminal).into_iter().next() {
            break w;
        }
        assert!(start.elapsed() < Duration::from_secs(10), "no wait call");
        thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(wait["timeout"], json!(630.0));
    s.shed.expire(terminal);
    let v = s.until(|v| v["state"] != "running");
    assert_eq!(v["why"], "timed out after 10 minutes");
    assert_eq!(v["account"]["state"], "signed-out", "{v}");
    assert_eq!(s.shed.calls("rm", terminal).len(), 1);
}

#[test]
fn an_older_cooees_account_folders_and_token_files_are_handed_to_shed_in_place() {
    let dir = scratch("login-adopt");
    let accounts = dir.join("data/accounts/fake");
    let elsewhere = dir.join("own-folder");
    for d in [
        accounts.join("work"),
        accounts.join(".new-account-3"),
        elsewhere.clone(),
    ] {
        fs::create_dir_all(d).unwrap();
    }
    std::os::unix::fs::symlink(&elsewhere, accounts.join("mine")).unwrap();
    fs::write(accounts.join(".cooee.json"), "{}").unwrap();
    fs::write(accounts.join("ci.token"), "t").unwrap();
    let shed = FakeShed::start(&dir.join("shed"), version());
    let added = || {
        let mut all: Vec<Value> = shed
            .requests
            .lock()
            .unwrap()
            .iter()
            .filter(|(m, _)| m == "account.add")
            .map(|(_, p)| p.clone())
            .collect();
        all.sort_by_key(|p| p["name"].as_str().unwrap().to_owned());
        all
    };
    let want = json!([
        { "provider": "fake", "name": "ci", "token_file": accounts.join("ci.token") },
        { "provider": "fake", "name": "mine", "folder": elsewhere },
        { "provider": "fake", "name": "work", "folder": accounts.join("work") },
    ]);
    let cooee = Cooee::start_with(&dir, &shed.dir, PROVIDERS);
    let start = Instant::now();
    while json!(added()) != want {
        assert!(start.elapsed() < Duration::from_secs(10), "{:?}", added());
        thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(
        shed.account_of("fake", "ci").unwrap()["state"],
        "signed-out"
    );
    drop(cooee);
    let again = Cooee::start_with(&dir, &shed.dir, PROVIDERS);
    while added().len() < 6 {
        assert!(start.elapsed() < Duration::from_secs(10), "{:?}", added());
        thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(
        shed.account_of("fake", "work").unwrap()["folder"],
        json!(accounts.join("work"))
    );
    assert!(
        accounts.join("work").is_dir() && elsewhere.is_dir(),
        "the folders stay"
    );
    drop(again);
}
