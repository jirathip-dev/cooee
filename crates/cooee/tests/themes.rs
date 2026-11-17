//! Theme packs end to end (docs/themes.md): `cooee theme check` passes the first-party packs and fails a broken one;
//! `cooee theme add` of a first-party pack (the build's copy at swag's pin, no fetch), a git URL ("not reviewed") and
//! a folder, each pinned in the lock; a pack failing the gate, or holding a symlink, is never installed; list and remove.

mod support;

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use cooee_core::swag::Swag;
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

struct Setup {
    dir: PathBuf,
    _shed: FakeShed,
    cooee: Cooee,
}

fn setup(name: &str) -> Setup {
    let dir = scratch(name);
    let shed = FakeShed::start(&dir.join("shed"), version());
    for (repo, broken) in [("sea.git", false), ("broken.git", true)] {
        let repo = dir.join(repo);
        farm_at(&repo, broken);
        git(&repo, &["init", "--quiet", "-b", "main"]);
        git(&repo, &["add", "-A"]);
        git(&repo, &["commit", "--quiet", "-m", "pack"]);
    }
    let cooee = Cooee::start(&dir, &shed.dir);
    Setup {
        dir,
        _shed: shed,
        cooee,
    }
}

impl Setup {
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
    let swag = Swag::pinned();
    let (ok, out) = s.run(&["theme", "add", "farm"]);
    assert!(ok, "{out}");
    assert!(
        out.starts_with(&format!(
            "theme farm from {} (packs/farm) at {}\n",
            swag.url, swag.commit
        )),
        "{out}"
    );
    assert!(!s.dir.join("data/themes/farm.git").exists());
    let themes = s.dir.join("themes");
    for (f, bytes) in swag.files("farm").unwrap() {
        assert_eq!(fs::read(themes.join("farm").join(f)).unwrap(), bytes, "{f}");
    }
    assert_eq!(
        s.lock(),
        json!([{ "name": "farm", "source": swag.url, "commit": swag.commit, "listed": true }])
    );
    let (ok, out) = s.run(&["theme", "add", "sea"]);
    assert!(
        !ok && out.contains("sea isn't a first-party theme (farm garden kitchen)"),
        "{out}"
    );
    let looks = s.cooee.json("/api/v1/looks");
    assert_eq!(
        looks["themes"]["farm"]["scheme"],
        json!({"dark": "farm", "light": "farm-light"})
    );
    assert_eq!(looks["schemes"]["farm"]["tokens"]["--bg"], "#0f1a13");
    let (head, svg) = s.cooee.raw("GET", "/look/farm/sprites.svg", &[], "");
    assert!(head.contains("image/svg+xml"), "{head}");
    assert!(
        svg.contains("id=\"s-needs\"") && svg.contains("--o1"),
        "{svg}"
    );
    let (head, _) = s.cooee.raw("GET", "/look/farm/theme.json", &[], "");
    assert!(
        head.starts_with("HTTP/1.0 404") || head.starts_with("HTTP/1.1 404"),
        "{head}"
    );

    let broken = s.dir.join("broken.git").display().to_string();
    let (ok, out) = s.run(&["theme", "add", &broken, "--yes"]);
    assert!(!ok);
    assert!(
        out.contains("it fails the theme gate:\n  sprites.svg: no s-needs"),
        "{out}"
    );
    assert!(!themes.join("broken").exists());

    let sea = s.dir.join("sea.git").display().to_string();
    let (ok, out) = s.run(&["theme", "add", &sea]);
    assert!(
        !ok && out.contains("not reviewed: this isn't a first-party theme at cooee's pin"),
        "{out}"
    );
    assert!(!themes.join("sea").exists());
    let (ok, out) = s.run(&["theme", "add", &sea, "--yes"]);
    assert!(ok, "{out}");
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
    let short = &swag.commit[..7];
    assert!(
        list.starts_with(&format!("farm  {}  {short}\nkitchen  ", swag.url)),
        "{list}"
    );
    assert!(list.contains("\nsea  /"), "{list}");

    let (ok, out) = s.run(&["theme", "remove", "farm"]);
    assert!(ok, "{out}");
    assert!(!themes.join("farm").exists());
    assert_eq!(s.lock().as_array().unwrap().len(), 2);
    assert!(
        s.cooee.json("/api/v1/looks")["themes"]
            .get("farm")
            .is_none()
    );
    assert!(
        s.cooee.json("/api/v1/looks")["themes"]
            .get("kitchen")
            .is_some()
    );
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
    let (ok, out) = s.run(&["theme", "add", &evil.display().to_string(), "--yes"]);
    assert!(!ok && out.contains("icon.png: a symlink"), "{out}");
    git(&evil, &["init", "--quiet", "-b", "main"]);
    git(&evil, &["add", "-A"]);
    git(&evil, &["commit", "--quiet", "-m", "evil"]);
    let url = format!("file://{}", evil.display());
    let (ok, out) = s.run(&["theme", "add", &url, "--yes"]);
    assert!(!ok && out.contains("icon.png: a symlink"), "{out}");
    assert!(!s.dir.join("themes/evil").exists());
}

#[test]
fn the_gate_fails_active_content_in_a_packs_svg() {
    let s = setup("th-active");
    let cases = [
        ("sprites.svg", "<script>alert(1)</script>", "<script>"),
        ("scene.svg", "<g onload=\"alert(1)\"/>", "an on* attribute"),
        ("sprites.svg", "<foreignObject/>", "<foreignObject>"),
        (
            "scene.svg",
            "<use xlink:href=\"https://evil.example/x.svg#a\"/>",
            "a non-local href",
        ),
        (
            "sprites.svg",
            "<a href=\"javascript:alert(1)\"/>",
            "a non-local href",
        ),
        (
            "sprites.svg",
            "<![CDATA[ <!-- ]]><script>alert(1)</script><![CDATA[ --> ]]>",
            "<script>",
        ),
        (
            "scene.svg",
            "<?x <!-- ?><script>alert(1)</script><?x --> ?>",
            "<script>",
        ),
    ];
    for (i, (file, bad, why)) in cases.iter().enumerate() {
        let dir = s.dir.join(format!("active{i}"));
        farm_at(&dir, false);
        let svg = fs::read_to_string(dir.join(file)).unwrap();
        fs::write(
            dir.join(file),
            svg.replacen("</svg>", &format!("{bad}</svg>"), 1),
        )
        .unwrap();
        let (ok, out) = s.run(&["theme", "check", &dir.display().to_string()]);
        assert!(!ok, "{bad}: {out}");
        assert!(out.contains(&format!("     {file}: {why}")), "{bad}: {out}");
    }
    let dir = s.dir.join("local");
    farm_at(&dir, false);
    let svg = fs::read_to_string(dir.join("sprites.svg")).unwrap();
    fs::write(
        dir.join("sprites.svg"),
        svg.replacen("</svg>", "<use href=\"#s-edit\"/></svg>", 1),
    )
    .unwrap();
    let (ok, out) = s.run(&["theme", "check", &dir.display().to_string()]);
    assert!(ok, "{out}");
}
