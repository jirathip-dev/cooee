//! Theme packs end to end (docs/themes.md): `cooee theme check` passes the first-party packs and fails a broken one;
//! `cooee theme add` from a fake index (a git repo holding packs in folders, pinned), a git URL ("not reviewed") and a
//! folder, each pinned in the lock; a pack failing the gate, or holding a symlink, is never installed; list and remove.

mod support;

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
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

fn packs() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs")
}

/// farm's pack copied to `to`, its `s-needs` gone when `broken`.
fn farm_at(to: &Path, broken: bool) {
    fs::create_dir_all(to).unwrap();
    for f in ["theme.json", "sprites.svg", "scene.svg", "icon.png"] {
        fs::copy(packs().join("themes/farm").join(f), to.join(f)).unwrap();
    }
    if broken {
        let svg = fs::read_to_string(to.join("sprites.svg")).unwrap();
        fs::write(
            to.join("sprites.svg"),
            svg.replace("id=\"s-needs\"", "id=\"x\""),
        )
        .unwrap();
    }
}

/// `cooee serve` with an index listing `farm` and `broken`, folders of one repo at its commit; and `sea.git`, a repo
/// that is a pack at its root.
struct Setup {
    dir: PathBuf,
    _shed: FakeShed,
    cooee: Cooee,
    commit: String,
}

fn setup(name: &str) -> Setup {
    let dir = scratch(name);
    let shed = FakeShed::start(&dir.join("shed"), version());
    let repo = dir.join("packs.git");
    farm_at(&repo.join("themes/farm"), false);
    farm_at(&repo.join("themes/broken"), true);
    git(&repo, &["init", "--quiet", "-b", "main"]);
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "--quiet", "-m", "packs"]);
    let commit = git(&repo, &["rev-parse", "HEAD"]);
    let sea = dir.join("sea.git");
    farm_at(&sea, false);
    git(&sea, &["init", "--quiet", "-b", "main"]);
    git(&sea, &["add", "-A"]);
    git(&sea, &["commit", "--quiet", "-m", "sea"]);
    let entry = |n: &str| {
        format!(
            "[[theme]]\nname = \"{n}\"\nurl = {:?}\npath = \"themes/{n}\"\ncommit = \"{commit}\"\n\
             description = \"{n} pack\"\nfirst_party = true\n\n",
            repo.to_string_lossy()
        )
    };
    fs::write(dir.join("index.toml"), entry("farm") + &entry("broken")).unwrap();
    let extra = format!(
        "plugin_index = {:?}\n",
        dir.join("index.toml").to_string_lossy()
    );
    let cooee = Cooee::start_with(&dir, &shed.dir, &extra);
    Setup {
        dir,
        _shed: shed,
        cooee,
        commit,
    }
}

impl Setup {
    /// `cooee <args>`: whether it succeeded, and its stdout and stderr.
    fn run(&self, args: &[&str]) -> (bool, String) {
        let out = self.cooee.command().args(args).output().unwrap();
        let text = String::from_utf8_lossy(&out.stdout).into_owned()
            + &String::from_utf8_lossy(&out.stderr);
        (out.status.success(), text)
    }

    fn lock(&self) -> Value {
        serde_json::from_slice(&fs::read(self.dir.join("data/themes.lock")).unwrap()).unwrap()
    }
}

#[test]
fn check_passes_the_first_party_packs_and_fails_a_broken_one() {
    let s = setup("th-check");
    let first: Vec<String> = ["farm", "kitchen", "garden"]
        .map(|t| packs().join("themes").join(t).display().to_string())
        .into();
    let first: Vec<&str> = first.iter().map(String::as_str).collect();
    let (ok, out) = s.run(&[&["theme", "check"], first.as_slice()].concat());
    assert!(ok, "{out}");
    assert_eq!(out.matches("ok   ").count(), 3, "{out}");
    let broken = s.dir.join("broken");
    farm_at(&broken, true);
    let (ok, out) = s.run(&["theme", "check", &broken.display().to_string()]);
    assert!(!ok);
    assert!(
        out.contains("FAIL ") && out.contains("     sprites.svg: no s-needs\n"),
        "{out}"
    );
    assert!(out.contains("1 theme(s) fail the theme gate"), "{out}");
}

#[test]
fn add_remove_and_list_with_the_lock() {
    let s = setup("th-add");
    let (_, list) = s.run(&["theme", "list"]);
    assert!(list.starts_with("no themes installed"), "{list}");
    let (ok, out) = s.run(&["theme", "add", "farm"]);
    assert!(ok, "{out}");
    assert!(!out.contains("not reviewed"), "{out}");
    let themes = s.dir.join("themes");
    assert_eq!(
        fs::read(themes.join("farm/sprites.svg")).unwrap(),
        fs::read(packs().join("themes/farm/sprites.svg")).unwrap()
    );
    assert_eq!(
        s.lock(),
        json!([{ "name": "farm", "source": "index", "commit": s.commit, "listed": true }])
    );

    // A pack failing the gate is never installed.
    let (ok, out) = s.run(&["theme", "add", "broken"]);
    assert!(!ok);
    assert!(
        out.contains("it fails the theme gate:\n  sprites.svg: no s-needs"),
        "{out}"
    );
    assert!(!themes.join("broken").exists());

    // A git URL outside the index is "not reviewed" and needs a yes.
    let sea = s.dir.join("sea.git").display().to_string();
    let (ok, out) = s.run(&["theme", "add", &sea]);
    assert!(
        !ok && out.contains("not reviewed: this theme isn't in cooee's index"),
        "{out}"
    );
    assert!(!themes.join("sea").exists());
    let (ok, out) = s.run(&["theme", "add", &sea, "--yes"]);
    assert!(ok, "{out}");
    // A folder, pinned at its repo's commit when it is in one.
    let kitchen = packs().join("themes/kitchen").display().to_string();
    let (ok, out) = s.run(&["theme", "add", &kitchen, "--yes"]);
    assert!(ok, "{out}");
    let lock = s.lock();
    assert_eq!(
        lock[1]["source"],
        json!(fs::canonicalize(&kitchen).unwrap())
    );
    assert_eq!(lock[1]["commit"].as_str().unwrap().len(), 40);
    assert_eq!(lock[2]["name"], "sea");
    assert_eq!(
        lock[2]["commit"],
        json!(git(&s.dir.join("sea.git"), &["rev-parse", "HEAD"]))
    );

    let (_, list) = s.run(&["theme", "list"]);
    let short = &s.commit[..7];
    assert!(
        list.starts_with(&format!("farm  index  {short}\nkitchen  ")),
        "{list}"
    );
    assert!(list.contains("\nsea  /"), "{list}");

    let (ok, out) = s.run(&["theme", "remove", "farm"]);
    assert!(ok, "{out}");
    assert!(!themes.join("farm").exists());
    assert_eq!(s.lock().as_array().unwrap().len(), 2);
    let (ok, out) = s.run(&["theme", "remove", "farm"]);
    assert!(!ok && out.contains("farm isn't installed"), "{out}");
}

#[test]
fn a_pack_with_a_symlink_is_never_installed() {
    let s = setup("th-link");
    let secret = s.dir.join("secret.txt");
    fs::write(&secret, "TOP-SECRET").unwrap();
    let evil = s.dir.join("evil.git");
    farm_at(&evil, false);
    fs::remove_file(evil.join("icon.png")).unwrap();
    std::os::unix::fs::symlink(&secret, evil.join("icon.png")).unwrap();
    let (ok, out) = s.run(&["theme", "check", &evil.display().to_string()]);
    assert!(!ok && out.contains("icon.png: a symlink"), "{out}");
    // From a folder: refused, not followed.
    let (ok, out) = s.run(&["theme", "add", &evil.display().to_string(), "--yes"]);
    assert!(!ok && out.contains("icon.png: a symlink"), "{out}");
    // From a git URL: the archive keeps the link, so the gate refuses it.
    git(&evil, &["init", "--quiet", "-b", "main"]);
    git(&evil, &["add", "-A"]);
    git(&evil, &["commit", "--quiet", "-m", "evil"]);
    let url = format!("file://{}", evil.display());
    let (ok, out) = s.run(&["theme", "add", &url, "--yes"]);
    assert!(!ok && out.contains("icon.png: a symlink"), "{out}");
    assert!(!s.dir.join("themes/evil").exists());
}
