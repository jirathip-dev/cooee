//! #150: `cooee machine check <m> [--yes]` and `cooee doctor`'s row per machine, against the fake shed. Machine `box`
//! has its own filesystem: its `sh`, `test` and `git` (tests/support's `on-box` programs) read a path under `here/`
//! as one under `box-fs/here/`. Persona `cook` runs on `box` as profile `coder`: each check passes, then fails with
//! its part missing there, and `--yes` clones the folder and copies the settings.

mod support;

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

use support::{FakeShed, scratch, version};

const SETTINGS: &str = r#"{"hooks":{"PreToolUse":[]}}"#;

struct Setup {
    dir: PathBuf,
    shed: FakeShed,
}

fn script(path: &Path, body: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, format!("#!/bin/sh\n{body}")).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn git(args: &[&str]) {
    let ok = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
        .args(args)
        .output()
        .unwrap();
    assert!(ok.status.success(), "{ok:?}");
}

fn setup(name: &str) -> Setup {
    let dir = scratch(name);
    let shed = FakeShed::start(&dir.join("shed"), version());
    shed.link("box", "up", now(), None);
    let (here, boxed) = (dir.join("here"), dir.join("box-fs"));
    fs::create_dir_all(&boxed).unwrap();
    let remap = format!(
        "n=$#\nfor a do\n  case $a in {here:?}*) a={boxed:?}$a;; esac\n  set -- \"$@\" \"$a\"\ndone\nshift $n\n\
         PATH=${{PATH#*:}} exec \"$(basename \"$0\")\" \"$@\"\n",
        here = here.to_string_lossy(),
        boxed = boxed.to_string_lossy(),
    );
    for program in ["sh", "test", "git"] {
        script(&shed.dir.join("on-box").join(program), &remap);
    }
    let origin = dir.join("origin");
    fs::create_dir_all(&origin).unwrap();
    fs::write(origin.join("README"), "the project\n").unwrap();
    let o = origin.to_str().unwrap();
    git(&["-C", o, "init", "-q"]);
    git(&["-C", o, "add", "."]);
    git(&["-C", o, "commit", "-qm", "first"]);
    git(&["clone", "-q", o, here.join("project").to_str().unwrap()]);
    fs::create_dir_all(here.join("rules")).unwrap();
    fs::write(here.join("rules/settings.json"), SETTINGS).unwrap();
    script(&dir.join("bin/fake-cli"), "exit 0\n");
    let flag = dir.join("signed-in");
    fs::write(&flag, "").unwrap();
    script(
        &dir.join("status.sh"),
        &format!("test -f {:?}\n", flag.to_string_lossy()),
    );
    let q = |p: PathBuf| format!("{:?}", p.to_string_lossy());
    fs::create_dir_all(dir.join("profiles")).unwrap();
    fs::write(
        dir.join("profiles/coder.toml"),
        format!(
            "name = \"coder\"\nlabel = \"Coder\"\nagent = \"fake-acp\"\nsettings = {}\n",
            q(here.join("rules/settings.json"))
        ),
    )
    .unwrap();
    fs::create_dir_all(dir.join("personas")).unwrap();
    fs::write(
        dir.join("personas/cook.toml"),
        format!(
            "name = \"Cook\"\nagent = \"fake-acp\"\nfolder = {}\nmachine = \"box\"\nprofile = \"coder\"\n",
            q(here.join("project"))
        ),
    )
    .unwrap();
    fs::write(
        dir.join("config.toml"),
        format!(
            "port = 0\ndata = {}\nshed = {}\nshed_dir = {}\n[[providers]]\nname = \"fake\"\nagent = \"fake-acp\"\n\
             login = [{}]\nstatus = [{}]\ninstall = \"get fake-cli\"\n",
            q(dir.join("data")),
            q(shed.dir.join("shed")),
            q(shed.dir.clone()),
            q(dir.join("bin/fake-cli")),
            q(dir.join("status.sh")),
        ),
    )
    .unwrap();
    Setup { dir, shed }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

impl Setup {
    fn cooee(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_cooee"))
            .arg("--config")
            .arg(self.dir.join("config.toml"))
            .args(args)
            .output()
            .unwrap()
    }

    fn check(&self, extra: &[&str]) -> (bool, String) {
        let out = self.cooee(&[&["machine", "check", "box"], extra].concat());
        let text = String::from_utf8(out.stdout).unwrap();
        (
            out.status.success(),
            text.replace(self.dir.to_str().unwrap(), "<dir>"),
        )
    }

    fn ready(&self) {
        let boxed = self.boxed("");
        fs::create_dir_all(boxed.join("project")).unwrap();
        fs::create_dir_all(boxed.join("rules")).unwrap();
        fs::write(boxed.join("rules/settings.json"), SETTINGS).unwrap();
    }

    fn boxed(&self, rest: &str) -> PathBuf {
        let here = self.dir.join("here").join(rest);
        self.dir
            .join("box-fs")
            .join(here.strip_prefix("/").unwrap())
    }

    fn all_on_box(&self) {
        let requests = self.shed.requests.lock().unwrap().clone();
        let runs: Vec<_> = requests.iter().filter(|(m, _)| m == "run").collect();
        assert_ne!(runs.len(), 0);
        assert!(runs.iter().all(|(_, p)| p["machine"] == "box"), "{runs:?}");
    }
}

const READY: &str = "ok   shed on box: reachable, same version
ok   cook: <dir>/bin/fake-cli installed
ok   cook: fake account default signed in
ok   cook: folder <dir>/here/project
ok   cook: rules and hooks <dir>/here/rules/settings.json synced
";

#[test]
fn a_ready_machine_passes_every_check_each_run_there() {
    let s = setup("machine-ready");
    s.ready();
    assert_eq!(s.check(&[]), (true, READY.to_owned()));
    s.all_on_box();
}

#[test]
fn each_check_fails_with_its_part_missing_there_and_says_the_fix() {
    let s = setup("machine-missing");
    fs::remove_file(s.dir.join("bin/fake-cli")).unwrap();
    fs::remove_file(s.dir.join("signed-in")).unwrap();
    let (ok, out) = s.check(&[]);
    assert!(!ok);
    assert_eq!(
        out,
        "ok   shed on box: reachable, same version
FAIL cook: <dir>/bin/fake-cli not installed
     fix: get fake-cli
FAIL cook: fake account default signed in: no
     fix: sign in to fake on box
FAIL cook: folder <dir>/here/project missing
     fix: cooee machine check box --yes (clones <dir>/origin there)
FAIL cook: rules and hooks <dir>/here/rules/settings.json not synced
     fix: cooee machine check box --yes (copies it there)
"
    );
    s.all_on_box();

    s.ready();
    fs::write(s.boxed("rules/settings.json"), "{}").unwrap();
    let (_, out) = s.check(&[]);
    assert!(out.contains("FAIL cook: rules and hooks <dir>/here/rules/settings.json not synced\n"));
}

#[test]
fn a_machine_shed_cant_reach_or_doesnt_have_fails_its_first_check_and_runs_nothing_there() {
    let s = setup("machine-down");
    s.shed
        .link("box", "down", now() - 600, Some("shed version mismatch"));
    let (ok, out) = s.check(&[]);
    assert!(!ok);
    assert_eq!(
        out,
        "FAIL shed on box: down since 10 min ago: shed version mismatch\n"
    );
    let out = s.cooee(&["machine", "check", "nowhere"]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert_eq!(text, "FAIL shed on nowhere: not added: shed machine add\n");
    assert_eq!(s.shed.runs(), Vec::<String>::new());
}

#[test]
fn yes_clones_the_folder_and_copies_the_settings_there_once() {
    let s = setup("machine-yes");
    let (ok, out) = s.check(&["--yes"]);
    assert!(ok, "{out}");
    assert!(
        out.contains("ok   cook: folder <dir>/here/project: cloned <dir>/origin\n"),
        "{out}"
    );
    assert!(
        out.contains("ok   cook: rules and hooks <dir>/here/rules/settings.json synced: copied\n")
    );
    assert_eq!(
        fs::read_to_string(s.boxed("project/README")).unwrap(),
        "the project\n"
    );
    assert_eq!(
        fs::read_to_string(s.boxed("rules/settings.json")).unwrap(),
        SETTINGS
    );
    assert_eq!(s.check(&["--yes"]), (true, READY.to_owned()));
}

#[test]
fn doctor_has_a_row_per_machine() {
    let s = setup("machine-doctor");
    let row = |s: &Setup| {
        let out = String::from_utf8(s.cooee(&["doctor"]).stdout).unwrap();
        out.lines()
            .find(|l| l.contains("machine box"))
            .unwrap_or_else(|| panic!("{out}"))
            .to_owned()
    };
    assert_eq!(
        row(&s),
        "FAIL machine box: 2 check(s) failed (cooee machine check box)"
    );
    s.ready();
    assert_eq!(row(&s), "ok   machine box ready");
}
