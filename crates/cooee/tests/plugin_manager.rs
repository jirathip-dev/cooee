//! The plugin manager end to end (docs/plugins.md): a fake plugin, the example backend (docs/plugins/hello) in a
//! git repo of its own, listed in a fake index. `cooee plugin add` clones, builds, writes its entries and token and
//! locks it, and `cooee serve` starts its backend; a failed update keeps the old build; remove; a URL plugin is "not
//! reviewed"; [improve] makes a branch whose approval builds and swaps in, and rolls back; only a paired device
//! may do any of it. The crash backoff is unit tested with a fake clock in `cooee-server/src/manage.rs`.

mod support;

use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::Duration,
};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, scratch, version};

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8(out.stdout).unwrap().trim().to_owned()
}

fn commit(repo: &Path, name: &str, v: &str, build: &str, run: &str) -> String {
    fs::create_dir_all(repo).unwrap();
    if !repo.join(".git").exists() {
        git(repo, &["init", "--quiet", "-b", "main"]);
    }
    let hello = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/plugins/hello/server.mjs");
    fs::copy(hello, repo.join("server.mjs")).unwrap();
    fs::write(
        repo.join("tab.js"),
        format!("document.body.append('{name} {v}');\n"),
    )
    .unwrap();
    let manifest = format!(
        "name = \"{name}\"\nversion = \"{v}\"\nbuild = \"{build}\"\nrun = \"{run}\"\nneeds = [\"node\"]\n\n\
         [[tabs]]\nname = \"{name}\"\nlabel = \"Greet\"\nmodule = \"/tab.js\"\n"
    );
    fs::write(repo.join("cooee-plugin.toml"), manifest).unwrap();
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "--quiet", "-m", v]);
    git(repo, &["rev-parse", "HEAD"])
}

const RUN: &str = "TOKEN=$COOEE_PLUGIN_TOKEN exec node server.mjs";
const BUILD: &str = "echo ok > built";

struct Setup {
    dir: PathBuf,
    shed: FakeShed,
    cooee: Cooee,
    repo: PathBuf,
    v1: String,
}

fn setup(name: &str) -> Setup {
    let dir = scratch(name);
    let shed = FakeShed::start(&dir.join("shed"), version());
    let repo = dir.join("greet.git");
    let v1 = commit(&repo, "greet", "v1", BUILD, RUN);
    let first = commit(&dir.join("first.git"), "first", "f1", BUILD, RUN);
    let index = format!(
        "[[plugin]]\nname = \"greet\"\nurl = {:?}\ncommit = \"{v1}\"\ndescription = \"says hello\"\n\n\
         [[plugin]]\nname = \"first\"\nurl = {:?}\ncommit = \"{first}\"\ndescription = \"ours\"\nfirst_party = true\n",
        repo.to_string_lossy(),
        dir.join("first.git").to_string_lossy()
    );
    fs::write(dir.join("index.toml"), index).unwrap();
    let extra = format!(
        "plugin_index = {:?}\n",
        dir.join("index.toml").to_string_lossy()
    );
    let cooee = Cooee::start_with(&dir, &shed.dir, &extra);
    Setup {
        dir,
        shed,
        cooee,
        repo,
        v1,
    }
}

fn until(what: &str, f: impl Fn() -> bool) {
    for _ in 0..400 {
        if f() {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("never: {what}");
}

impl Setup {
    fn data(&self) -> PathBuf {
        self.dir.join("data")
    }

    fn lock(&self) -> Value {
        serde_json::from_str(&fs::read_to_string(self.data().join("plugins.lock")).unwrap())
            .unwrap()
    }

    fn run(&self, args: &[&str], input: &str) -> (bool, String) {
        let mut child = self
            .cooee
            .command()
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        let text = String::from_utf8_lossy(&out.stdout).into_owned()
            + &String::from_utf8_lossy(&out.stderr);
        (out.status.success(), text)
    }

    fn tab_says(&self, name: &str, text: &str) {
        until(&format!("/p/{name} says {text}"), || {
            let (status, body) = self.cooee.get(&format!("/p/{name}/frame"));
            status == 200 && body.contains(text)
        });
    }

    fn act(&self, action: &str, body: &Value) -> (u16, String) {
        self.cooee.post(&format!("/plugins/{action}"), body)
    }

    fn job_done(&self, name: &str) -> Option<String> {
        until("the job ends", || {
            let jobs = &self.cooee.json("/plugins")["jobs"];
            !jobs[name].as_str().is_some_and(|j| j.ends_with('…'))
        });
        self.cooee.json("/plugins")["jobs"][name]
            .as_str()
            .map(str::to_owned)
    }
}

#[test]
fn add_from_the_index_clones_builds_writes_the_entry_and_token_starts_it_and_locks_it() {
    let s = setup("pm-add");
    let out = s.cooee.cli(&["plugin", "add", "greet", "--yes"]);
    assert!(out.contains(&format!("commit {}", s.v1)), "{out}");
    assert!(
        out.contains(&format!("build: {BUILD}\nrun: {RUN}")),
        "{out}"
    );
    assert!(!out.contains("not reviewed"), "{out}");

    let folder = s.data().join("plugins/greet");
    assert_eq!(git(&folder.join("repo"), &["rev-parse", "HEAD"]), s.v1);
    assert_eq!(
        fs::read_to_string(folder.join("builds").join(&s.v1).join("built")).unwrap(),
        "ok\n"
    );
    let entries = s.data().join("plugins/plugins.toml");
    assert_eq!(
        fs::metadata(&entries).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let entries = fs::read_to_string(entries).unwrap();
    assert!(
        entries.contains("[[plugins]]\nname = \"greet\"\nlabel = \"Greet\""),
        "{entries}"
    );
    let token = entries
        .split("token = \"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    assert!(
        token.len() == 64 && token.bytes().all(|b| b.is_ascii_hexdigit()),
        "{token}"
    );
    let lock = &s.lock()[0];
    assert_eq!(
        (&lock["name"], &lock["url"], &lock["commit"]),
        (&json!("greet"), &json!(s.repo), &json!(s.v1))
    );

    s.tab_says("greet", "greet v1");
    let (status, body) = s.cooee.get("/p/greet/api/whoami");
    assert_eq!(
        (status, body.as_str()),
        (200, r#"{"token":"ok","cookie":null}"#)
    );
    let log = fs::read_to_string(folder.join("plugin.log")).unwrap();
    assert!(
        log.contains(&format!("== cooee: build {}: {BUILD}", s.v1)),
        "{log}"
    );
    until("listed as running", || {
        s.cooee.json("/plugins")["installed"][0]["state"] == "running"
    });
    let list = s.cooee.cli(&["plugin", "list"]);
    assert_eq!(list, format!("greet  v1  {}  index  running\n", &s.v1[..7]));
    let (ok, out) = s.run(&["plugin", "add", "greet", "--yes"], "");
    assert!(!ok && out.contains("greet is installed already"), "{out}");
}

#[test]
fn an_index_plugin_has_no_warning_and_a_url_plugin_is_not_reviewed() {
    let s = setup("pm-review");
    let url = s.repo.to_string_lossy().into_owned();
    let (status, plan) = s.act("plan", &json!({ "source": "greet" }));
    let plan: Value = serde_json::from_str(&plan).unwrap();
    assert_eq!(
        (status, &plan["listed"], &plan["commit"]),
        (200, &json!(true), &json!(s.v1))
    );
    assert!(
        !plan["text"].as_str().unwrap().contains("not reviewed"),
        "{plan}"
    );
    let (_, plan) = s.act("plan", &json!({ "source": url }));
    let plan: Value = serde_json::from_str(&plan).unwrap();
    assert_eq!(plan["listed"], false);
    assert!(
        plan["text"].as_str().unwrap().contains("\nnot reviewed: "),
        "{plan}"
    );

    let (ok, out) = s.run(&["plugin", "add", &url], "n\n");
    assert!(
        !ok && out.contains("not reviewed") && out.contains("go ahead? [y/N]"),
        "{out}"
    );
    assert!(!s.data().join("plugins.lock").exists());
    let (ok, out) = s.run(&["plugin", "add", &url], "y\n");
    assert!(ok, "{out}");
    assert_eq!(s.lock()[0]["listed"], false);
    s.tab_says("greet", "greet v1");
    until("url source", || {
        s.cooee.cli(&["plugin", "list"]).contains("  url  running")
    });
}

#[test]
fn a_failed_update_keeps_the_old_build_and_a_good_one_swaps_in() {
    let s = setup("pm-update");
    let url = s.repo.to_string_lossy().into_owned();
    s.cooee.cli(&["plugin", "add", &url, "--yes"]);
    s.tab_says("greet", "greet v1");

    let broken = commit(&s.repo, "greet", "v2", "exit 3", RUN);
    let (ok, out) = s.run(&["plugin", "update", "greet", "--yes"], "");
    assert!(
        !ok && out.contains("the build failed") && out.contains("the old build keeps running"),
        "{out}"
    );
    let dead = commit(&s.repo, "greet", "v3", BUILD, "exit 1");
    let (ok, out) = s.run(&["plugin", "update", "--yes"], "");
    assert!(!ok && out.contains("its backend exited"), "{out}");
    assert_eq!(s.lock()[0]["commit"], json!(s.v1));
    let builds = s.data().join("plugins/greet/builds");
    assert!(!builds.join(broken).exists() && !builds.join(dead).exists());
    s.tab_says("greet", "greet v1");

    let v4 = commit(&s.repo, "greet", "v4", BUILD, RUN);
    let out = s.cooee.cli(&["plugin", "update", "greet", "--yes"]);
    assert!(
        out.contains(&format!("updated greet to {}", &v4[..7])),
        "{out}"
    );
    s.tab_says("greet", "greet v4");
    assert_eq!(
        (&s.lock()[0]["commit"], &s.lock()[0]["previous"]),
        (&json!(v4), &json!(s.v1))
    );
    assert_eq!(s.cooee.cli(&["plugin", "update"]), "greet: up to date\n");
}

#[test]
fn remove_stops_it_and_keeps_the_checkout_unless_purged() {
    let s = setup("pm-remove");
    s.cooee.cli(&["plugin", "add", "greet", "--yes"]);
    s.tab_says("greet", "greet v1");
    let port = u16::try_from(s.lock()[0]["port"].as_u64().unwrap()).unwrap();
    let out = s.cooee.cli(&["plugin", "remove", "greet"]);
    assert!(out.contains("its checkout stays"), "{out}");
    assert_eq!(s.lock(), json!([]));
    assert!(
        !fs::read_to_string(s.data().join("plugins/plugins.toml"))
            .unwrap()
            .contains("greet")
    );
    until("the tab goes", || s.cooee.get("/p/greet/frame").0 == 404);
    until("the backend stops", || {
        std::net::TcpStream::connect(("127.0.0.1", port)).is_err()
    });
    assert!(s.data().join("plugins/greet/repo/.git").exists());

    s.cooee.cli(&["plugin", "add", "greet", "--yes"]);
    s.tab_says("greet", "greet v1");
    s.cooee.cli(&["plugin", "remove", "greet", "--purge"]);
    assert!(!s.data().join("plugins/greet").exists());
    until("the tab goes", || s.cooee.get("/p/greet/frame").0 == 404);
}

#[test]
fn improve_makes_a_branch_an_approval_builds_and_swaps_it_in_and_it_rolls_back() {
    let s = setup("pm-improve");
    let (_, plan) = s.act("plan", &json!({ "source": "greet" }));
    let plan: Value = serde_json::from_str(&plan).unwrap();
    let (status, body) = s.act(
        "add",
        &json!({ "source": "greet", "commit": plan["commit"] }),
    );
    assert_eq!((status, body.as_str()), (200, "\"installing\""));
    assert_eq!(s.job_done("greet"), None);
    s.tab_says("greet", "greet v1");

    let (status, body) = s.act("improve", &json!({ "name": "greet", "text": "say hi" }));
    assert_eq!(status, 200, "{body}");
    let thread: Value = serde_json::from_str(&body).unwrap();
    let thread = thread["thread"].as_str().unwrap();
    let repo = s.data().join("plugins/greet/repo");
    assert_eq!(git(&repo, &["branch", "--show-current"]), "cooee-improve");
    let run = s.shed.cli_log();
    assert!(run.contains(&format!("--cwd {}", repo.display())), "{run}");
    let sent = s.shed.sent(&format!("cooee-{thread}"));
    assert!(sent[0].contains("say hi"), "{sent:?}");
    assert_eq!(s.lock()[0]["improving"], json!(thread));

    let manifest = fs::read_to_string(repo.join("cooee-plugin.toml")).unwrap();
    fs::write(
        repo.join("cooee-plugin.toml"),
        manifest.replace(BUILD, "exit 4"),
    )
    .unwrap();
    s.act("approve", &json!({ "name": "greet" }));
    let failed = s.job_done("greet").unwrap();
    assert!(failed.starts_with("failed: the build failed"), "{failed}");
    assert_eq!(s.lock()[0]["commit"], json!(s.v1));
    s.tab_says("greet", "greet v1");

    fs::write(repo.join("cooee-plugin.toml"), &manifest).unwrap();
    fs::write(
        repo.join("tab.js"),
        "document.body.append('greet improved');\n",
    )
    .unwrap();
    s.act("approve", &json!({ "name": "greet" }));
    assert_eq!(s.job_done("greet"), None);
    s.tab_says("greet", "greet improved");
    let lock = &s.lock()[0];
    let improved = lock["commit"].as_str().unwrap().to_owned();
    assert_eq!(
        (&lock["previous"], &lock["improving"]),
        (&json!(s.v1), &Value::Null)
    );
    assert_eq!(git(&repo, &["rev-parse", "HEAD"]), improved);

    let (status, _) = s.act("rollback", &json!({ "name": "greet" }));
    assert_eq!(status, 200);
    s.tab_says("greet", "greet v1");
    assert_eq!(
        (&s.lock()[0]["commit"], &s.lock()[0]["previous"]),
        (&json!(s.v1), &json!(improved))
    );
}

#[test]
fn only_a_paired_device_may_add_and_a_first_party_plugin_is_improved_via_an_issue() {
    let s = setup("pm-owner");
    let (_, plan) = s.act("plan", &json!({ "source": "first" }));
    let plan: Value = serde_json::from_str(&plan).unwrap();
    let add = json!({ "source": "first", "commit": plan["commit"] }).to_string();
    let json = ("Content-Type", "application/json");
    s.cooee.cli(&["plugin", "add", "greet", "--yes"]);
    let token = fs::read_to_string(s.data().join("plugins/plugins.toml")).unwrap();
    let token = token
        .split("token = \"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .to_owned();
    let bearer = format!("Bearer {token}");
    for headers in [vec![json], vec![json, ("Authorization", bearer.as_str())]] {
        let (head, _) = s.cooee.raw("POST", "/plugins/add", &headers, &add);
        assert!(head.starts_with("HTTP/1.0 401"), "{head}");
        let (head, _) = s.cooee.raw("GET", "/plugins", &headers, "");
        assert!(head.starts_with("HTTP/1.0 401"), "{head}");
    }
    assert_eq!(s.lock().as_array().unwrap().len(), 1);
    let (status, body) = s.act(
        "add",
        &json!({ "source": "first", "commit": "0".repeat(40) }),
    );
    assert_eq!(
        (status, body.as_str()),
        (400, "It changed since you confirmed: confirm again.")
    );

    s.act("add", &serde_json::from_str(&add).unwrap());
    assert_eq!(s.job_done("first"), None);
    let (status, body) = s.act("improve", &json!({ "name": "first", "text": "x" }));
    assert_eq!(status, 400);
    assert!(
        body.contains("first is first-party: improve it via an issue"),
        "{body}"
    );
    let rows = s.cooee.json("/plugins");
    assert_eq!(rows["installed"][1]["first_party"], true);
}

#[test]
fn a_backend_ends_when_cooee_is_killed_and_a_restart_starts_it_again() {
    let mut s = setup("pm-orphan");
    s.cooee.cli(&["plugin", "add", "greet", "--yes"]);
    s.tab_says("greet", "greet v1");
    let port = u16::try_from(s.lock()[0]["port"].as_u64().unwrap()).unwrap();
    s.cooee.child.kill().unwrap();
    s.cooee.child.wait().unwrap();
    until("the orphaned backend ends", || {
        std::net::TcpStream::connect(("127.0.0.1", port)).is_err()
    });

    let extra = format!(
        "plugin_index = {:?}\n",
        s.dir.join("index.toml").to_string_lossy()
    );
    s.cooee = Cooee::start_with(&s.dir, &s.shed.dir, &extra);
    s.tab_says("greet", "greet v1");
}

#[test]
fn an_attach_backend_that_answers_is_used_and_run_only_starts_once_it_is_gone() {
    let s = setup("pm-attach");
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let served = s.dir.join("served");
    fs::create_dir_all(&served).unwrap();
    fs::write(served.join("tab.js"), "document.body.append('attached');\n").unwrap();
    let hello = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/plugins/hello/server.mjs");
    fs::copy(hello, served.join("server.mjs")).unwrap();
    let token_file = s.dir.join("cooee-token");
    fs::write(
        &token_file,
        "attach-token-0123456789
",
    )
    .unwrap();
    let mut running = Command::new("node")
        .arg("server.mjs")
        .env("PORT", port.to_string())
        .env("TOKEN", "attach-token-0123456789")
        .current_dir(&served)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    until("the running backend answers", || {
        std::net::TcpStream::connect(("127.0.0.1", port)).is_ok()
    });
    let repo = s.dir.join("attach.git");
    commit(&repo, "att", "a1", BUILD, &format!("touch ran; {RUN}"));
    let manifest = fs::read_to_string(repo.join("cooee-plugin.toml")).unwrap();
    let attach = format!(
        "attach = \"http://127.0.0.1:{port}\"\nattach_token = {:?}\nneeds",
        token_file.to_string_lossy()
    );
    fs::write(
        repo.join("cooee-plugin.toml"),
        manifest.replacen("needs", &attach, 1),
    )
    .unwrap();
    git(&repo, &["commit", "--quiet", "-am", "attach"]);
    let sha = git(&repo, &["rev-parse", "HEAD"]);

    s.cooee
        .cli(&["plugin", "add", &repo.to_string_lossy(), "--yes"]);
    let build = s.data().join("plugins/att/builds").join(&sha);
    assert_eq!(s.lock()[0]["port"], port);
    s.tab_says("att", "attached");
    until("listed as attached", || {
        s.cooee.json("/plugins")["installed"][0]["state"] == "attached"
    });
    assert!(
        !build.join("ran").exists(),
        "run started beside the attach backend"
    );

    running.kill().unwrap();
    running.wait().unwrap();
    s.tab_says("att", "att a1");
    assert!(build.join("ran").exists());
}
