//! Provider accounts end to end with fake CLIs: `cooee serve` against the fake shed (tests/support), whose plain
//! terminals really run the fake login. [+ account] runs the login in a new folder, listed only once it succeeds;
//! a thread's pick starts its agent with that folder in the provider's variable, and the composer offers the account
//! signed in (#48: no default account); [sign out] runs the
//! logout for the folder, then removes it; a JSON status shows as a plain row; no token reaches cooee's files, its
//! log or what it sends shed.

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

struct Setup {
    dir: PathBuf,
    shed: FakeShed,
    cooee: Cooee,
    /// A secret-shaped value the fake CLI prints and keeps in its folder, built at runtime.
    token: String,
    /// The config lines naming provider `fake`, to start cooee again.
    extra: String,
}

fn script(path: &Path, body: &str) {
    fs::write(path, format!("#!/bin/sh\n{body}")).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// cooee with provider `fake` for agent `fake-acp`, its config folder in `FAKE_HOME` (unset: `dir/system`). Its
/// login prints a token and reads a code (`good-code` keeps the token in the folder); its status prints JSON with
/// the folder's name as the email; its logout notes the folder in `dir/logouts`, then forgets the token.
fn setup(name: &str) -> Setup {
    let dir = scratch(name);
    let token = format!("sk-{}", "k3".repeat(16));
    let home = format!("${{FAKE_HOME:-{}}}", dir.join("system").display());
    let login = dir.join("login.sh");
    script(
        &login,
        &format!(
            "echo 'session {token}'\necho 'Paste code here >'\nread code\n\
             [ \"$code\" = good-code ] || exit 1\nmkdir -p \"{home}\" && echo '{token}' >\"{home}/creds\"\n"
        ),
    );
    let status = dir.join("status.sh");
    script(
        &status,
        &format!(
            "test -f \"{home}/creds\" || {{ echo '{{\"loggedIn\": false}}'; exit 1; }}\n\
             echo \"{{\\\"loggedIn\\\": true, \\\"authMethod\\\": \\\"fake.ai\\\", \\\"email\\\": \\\"$(basename \"{home}\")@example.com\\\"}}\"\n"
        ),
    );
    let logout = dir.join("logout.sh");
    script(
        &logout,
        &format!(
            "echo \"{home}\" >>'{}'\nrm -f \"{home}/creds\"\n",
            dir.join("logouts").display()
        ),
    );
    let shed = FakeShed::start(&dir.join("shed"), version());
    let extra = format!(
        "[[providers]]\nname = \"fake\"\nbase = \"claude\"\nagent = \"fake-acp\"\nhome = \"FAKE_HOME\"\nlogin = [{:?}]\n\
         status = [{:?}]\nlogout = [{:?}]\n",
        login.to_string_lossy(),
        status.to_string_lossy(),
        logout.to_string_lossy()
    );
    let cooee = Cooee::start_with(&dir, &shed.dir, &extra);
    Setup {
        dir,
        shed,
        cooee,
        token,
        extra,
    }
}

impl Setup {
    fn accounts(&self) -> PathBuf {
        self.dir.join("data/accounts/fake")
    }

    /// `GET /logins/fake` until `f` holds (10 s at most, for the fake program).
    fn until(&self, f: impl Fn(&Value) -> bool) -> Value {
        let start = Instant::now();
        loop {
            let v = self.cooee.json("/logins/fake");
            if f(&v) {
                return v;
            }
            assert!(start.elapsed() < Duration::from_secs(10), "{v}");
            thread::sleep(Duration::from_millis(20));
        }
    }

    /// [+ account] with `code` typed in; returns the login's `run` params and how it ended.
    fn add(&self, code: &str) -> (Value, Value) {
        let before = self.shed.runs();
        let (status, body) = self.cooee.post("/logins/fake", &json!({ "new": true }));
        assert_eq!(status, 200, "{body}");
        let v = self.until(|v| v["screen"].as_str().unwrap().contains("Paste code"));
        assert!(
            v["screen"].as_str().unwrap().contains("session [redacted]"),
            "{v}"
        );
        let terminal = self
            .shed
            .runs()
            .into_iter()
            .find(|t| !before.contains(t))
            .unwrap();
        let run = self.shed.calls("run", &terminal)[0].clone();
        let pending = self.accounts().join(".new-account-2");
        assert!(pending.is_dir(), "the login runs in a new folder");
        assert!(self.refused("account-2"), "not listed while the login runs");
        let sent = self
            .cooee
            .post("/logins/fake/send", &json!({ "text": code }));
        assert_eq!(sent.0, 200, "{}", sent.1);
        (run, self.until(|v| v["state"] == "ended"))
    }

    /// A thread on `account` is refused: the account isn't listed.
    fn refused(&self, account: &str) -> bool {
        let new = json!({ "agent": "fake-acp", "account": account, "folder": self.dir.join("root"), "text": "hi", "random_id": "r" });
        self.cooee.post("/threads", &new).0 == 400
    }

    /// The account `GET /threads` notes for thread `id`.
    fn account_of(&self, id: &str) -> Value {
        let list = self.cooee.json("/threads");
        let row = list.as_array().unwrap().iter().find(|t| t["id"] == id);
        row.unwrap()["account"].clone()
    }

    /// Starts a thread on `fake-acp` with `account` (none: the default); returns its id and its `shed run` line.
    fn thread(&self, account: Option<&str>) -> (String, String) {
        let root = self.dir.join("root");
        fs::create_dir_all(&root).unwrap();
        // Each start its own id, as the page makes one: a repeated id adopts the thread it started.
        let random = format!("start-{}", self.shed.cli_log().len());
        let new = json!({ "agent": "fake-acp", "account": account, "folder": root, "text": "hi", "random_id": random });
        let (status, body) = self.cooee.post("/threads", &new);
        assert_eq!(status, 200, "{body}");
        let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let log = self.shed.cli_log();
        let line = log.lines().rfind(|l| l.contains(" run ")).unwrap();
        assert!(line.contains(&format!("cooee-{id}")), "{line}");
        (id, line.to_owned())
    }

    /// cooee's own files under its data folder (not the CLI's account folders), its stderr, what it sent shed and
    /// the fake shed CLI's arguments.
    fn written(&self) -> String {
        let mut out = fs::read_to_string(self.dir.join("cooee.err")).unwrap();
        out.push_str(&self.shed.cli_log());
        for (method, params) in self.shed.requests.lock().unwrap().iter() {
            out.extend([method.clone(), params.to_string()]);
        }
        let mut dirs = vec![self.dir.join("data")];
        while let Some(dir) = dirs.pop() {
            for e in fs::read_dir(&dir).unwrap().flatten() {
                if e.file_type().unwrap().is_dir() {
                    if dir != self.accounts() {
                        dirs.push(e.path());
                    }
                } else {
                    out.push_str(&String::from_utf8_lossy(&fs::read(e.path()).unwrap()));
                }
            }
        }
        out
    }
}

#[test]
fn add_account_listed_only_on_success_thread_pick_spawns_with_its_folder_sign_out_removes_it() {
    let s = setup("accounts");
    let v = s.until(|v| v["status"]["unknown"] == false);
    assert_eq!(v["status"]["signed_in"], false, "{v}");
    assert_eq!(v["status"]["line"], "signed out", "{v}");

    // A failed login: not listed, the folder gone.
    let (run, v) = s.add("bad-code");
    let pending = s.accounts().join(".new-account-2");
    assert_eq!(
        run["cmd"],
        json!([
            "env",
            format!("FAKE_HOME={}", pending.display()),
            s.dir.join("login.sh")
        ])
    );
    assert_eq!(v["why"], "the login command exited 1");
    assert!(!pending.exists() && !s.accounts().join("account-2").exists());
    assert!(s.refused("account-2"));

    // A successful one: the folder is listed, its status a plain line.
    let (_, v) = s.add("good-code");
    assert_eq!(v["why"], "the login command finished");
    let folder = s.accounts().join("account-2");
    assert!(folder.join("creds").is_file() && !pending.exists());
    let v = s.until(|v| v["status"]["signed_in"] == true);
    assert_eq!(v["status"]["who"], "account-2@example.com", "{v}");
    let line = v["status"]["line"].as_str().unwrap();
    assert!(
        line.contains("account-2@example.com") && !line.contains("loggedIn") && !line.contains('{'),
        "no raw JSON: {line}"
    );
    // No account named: the system default, no folder.
    let (id, line) = s.thread(None);
    assert!(!line.contains("env"), "{line}");
    assert_eq!(s.account_of(&id), "");
    // A thread's own pick: that account's folder.
    let (id, line) = s.thread(Some("account-2"));
    assert!(
        line.ends_with(&format!(" -- env FAKE_HOME={} fake-acp", folder.display())),
        "{line}"
    );
    assert_eq!(s.account_of(&id), "account-2");
    assert!(s.refused("nope"));

    // Sign out: the logout runs for the folder, then the folder is removed.
    let (status, body) = s
        .cooee
        .post("/logins/fake/signout", &json!({ "account": "account-2" }));
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        fs::read_to_string(s.dir.join("logouts")).unwrap(),
        format!("{}\n", folder.display())
    );
    assert!(!folder.exists());
    assert!(s.refused("account-2"));

    let written = s.written();
    assert!(!written.contains(&s.token), "a token was written");
}

/// On the main login: the only one whose status `GET /logins/{provider}` shows after a restart.
#[test]
fn signed_in_only_while_the_status_says_so_not_from_the_last_sign_in_noted() {
    let mut s = setup("accounts-stale");
    let (status, body) = s.cooee.post("/logins/fake", &json!({}));
    assert_eq!(status, 200, "{body}");
    s.until(|v| v["screen"].as_str().unwrap().contains("Paste code"));
    let sent = s
        .cooee
        .post("/logins/fake/send", &json!({ "text": "good-code" }));
    assert_eq!(sent.0, 200, "{}", sent.1);
    let v = s.until(|v| v["state"] == "ended");
    assert_eq!(v["why"], "the login command finished");
    let v = s.until(|v| v["status"]["signed_in"] == true);
    assert_eq!(v["status"]["who"], "system@example.com", "{v}");

    // The login is lost outside cooee (revoked, its Keychain item gone); the sign-in stays noted.
    fs::remove_file(s.dir.join("system/creds")).unwrap();
    let noted = fs::read_to_string(s.accounts().join(".cooee.json")).unwrap();
    assert!(noted.contains(r#""signed_in":{"":"#), "{noted}");
    s.cooee.child.kill().unwrap();
    s.cooee.child.wait().unwrap();
    // A fresh start checks the status: signed out, whatever was noted.
    s.cooee = Cooee::start_with(&s.dir, &s.shed.dir, &s.extra);
    let v = s.until(|v| v["status"]["unknown"] == false);
    assert_eq!(v["status"]["signed_in"], false, "{v}");
    assert_eq!(v["status"]["line"], "signed out", "{v}");
}
