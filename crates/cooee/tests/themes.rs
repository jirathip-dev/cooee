//! Theme packs end to end (docs/themes.md): `cooee theme check` passes the first-party packs and fails a broken one;
//! `cooee theme add` of a first-party pack (swag's compiled-in copy, no fetch), a git URL ("not reviewed") and
//! a folder, each pinned in the lock; a pack failing the gate, or holding a symlink, is never installed; list and remove;
//! a git source is a shallow clone of its folder alone (#363), and an old full clone is replaced on start.

mod support;

use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
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

fn swag_at(to: &Path, name: &str) {
    fs::create_dir_all(to).unwrap();
    for (f, bytes) in Swag::pinned().files(name).unwrap() {
        fs::write(to.join(f), bytes).unwrap();
    }
}

fn farm_at(to: &Path, broken: bool) {
    swag_at(to, "farm");
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
    let first = Swag::pinned().names().into_iter();
    let first = first.map(|p| (format!("first-party/{p}"), Some(p)));
    for (repo, pack) in ["sea.git", "broken.git"]
        .map(|r| (r.to_owned(), None))
        .into_iter()
        .chain(first)
    {
        let repo = dir.join(repo);
        match pack {
            Some(p) => swag_at(&repo, p),
            None => farm_at(&repo, repo.ends_with("broken.git")),
        }
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
        .map(|t| s.dir.join("first-party").join(t).display().to_string())
        .into();
    let styles = ["terminal", "scene"].map(|p| packs().join(p).display().to_string());
    let first: Vec<&str> = first.iter().chain(&styles).map(String::as_str).collect();
    let (ok, out) = s.run(&[&["theme", "check"], first.as_slice()].concat());
    assert!(ok, "{out}");
    assert_eq!(out.matches(" (theme)\n").count(), 3, "{out}");
    assert!(
        out.ends_with(&format!("ok   {} (style)\n", styles[1])),
        "{out}"
    );
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

fn gzips_and_revalidates(cooee: &Cooee, head: &str, svg: &str) {
    let tag = head.lines().find_map(|l| l.strip_prefix("etag: ")).unwrap();
    assert!(head.contains("cache-control: no-cache"), "{head}");
    let gzip = [("Accept-Encoding", "gzip, br")];
    let (head, zipped) = cooee.raw_bytes("GET", "/look/farm/sprites.svg", &gzip, "");
    assert!(head.contains("content-encoding: gzip"), "{head}");
    assert!(
        zipped.len() < svg.len() / 4,
        "{} of {}",
        zipped.len(),
        svg.len()
    );
    let mut unzipped = String::new();
    flate2::read::GzDecoder::new(zipped.as_slice())
        .read_to_string(&mut unzipped)
        .unwrap();
    assert_eq!(unzipped, svg);
    let weak = head.lines().find_map(|l| l.strip_prefix("etag: ")).unwrap();
    assert_eq!(weak, format!("W/{tag}"));
    for seen in [tag, weak] {
        let (head, body) = cooee.raw(
            "GET",
            "/look/farm/sprites.svg",
            &[("If-None-Match", seen)],
            "",
        );
        assert!(
            head.starts_with("HTTP/1.1 304") || head.starts_with("HTTP/1.0 304"),
            "{head}"
        );
        assert_eq!(body, "");
    }
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
        !ok && out
            .contains("sea isn't a first-party theme (bush farm garden kitchen scene terminal)"),
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
    gzips_and_revalidates(&s.cooee, &head, &svg);
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
    let kitchen = s.dir.join("first-party/kitchen").display().to_string();
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
fn a_built_in_style_pack_adds_and_a_broken_style_leaves_its_theme_served() {
    let s = setup("th-style");
    let themes = s.dir.join("themes");
    let kitchen = s.dir.join("first-party/kitchen").display().to_string();
    assert!(s.run(&["theme", "add", &kitchen, "--yes"]).0);
    let (ok, out) = s.run(&["theme", "add", "terminal"]);
    assert!(
        ok && out.starts_with("theme terminal from built in at cooee "),
        "{out}"
    );
    assert_eq!(
        fs::read(themes.join("terminal/style.json")).unwrap(),
        fs::read(packs().join("terminal/style.json")).unwrap()
    );
    fs::write(
        themes.join("kitchen/style.json"),
        r#"{"format":1,"surface":"blur"}"#,
    )
    .unwrap();
    let looks = s.cooee.json("/api/v1/looks");
    assert_eq!(
        looks["styles"]["terminal"]["labels"]["button"]["suffix"],
        " ]"
    );
    assert!(looks["themes"].get("terminal").is_none());
    assert!(looks["themes"].get("kitchen").is_some());
    assert!(looks["styles"].get("kitchen").is_none());
    let why = looks["broken"]["kitchen"].as_str().unwrap();
    assert!(
        why.starts_with("style.json: unknown variant `blur`"),
        "{why}"
    );
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

fn until(what: &str, f: impl Fn() -> bool) {
    for _ in 0..400 {
        if f() {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("never: {what}");
}

fn files_outside_git(dir: &Path, under: &Path, out: &mut Vec<String>) {
    for e in fs::read_dir(dir).unwrap().map(Result::unwrap) {
        let path = e.path();
        if e.file_name() == ".git" {
            continue;
        }
        if path.is_dir() {
            files_outside_git(&path, under, out);
        } else {
            let rel = path.strip_prefix(under).unwrap();
            out.push(rel.display().to_string());
        }
    }
}

fn local_swag(dir: &Path) -> (PathBuf, String) {
    let repo = dir.join("swag");
    for t in ["garden", "kitchen"] {
        swag_at(&repo.join("packs").join(t), t);
        if t == "garden" {
            git(&repo, &["init", "--quiet", "-b", "main"]);
        }
        git(&repo, &["add", "-A"]);
        git(&repo, &["commit", "--quiet", "-m", t]);
    }
    let commit = git(&repo, &["rev-parse", "HEAD"]);
    (repo, commit)
}

#[test]
fn a_git_pack_is_a_shallow_clone_of_its_folder_alone() {
    let s = setup("th-shallow");
    let (repo, commit) = local_swag(&s.dir);
    let swag = Swag::pinned();
    let add = |rev: &str| {
        let out = s
            .cooee
            .command()
            .env("GIT_CONFIG_COUNT", "1")
            .env(
                "GIT_CONFIG_KEY_0",
                format!("url.{}.insteadOf", repo.display()),
            )
            .env("GIT_CONFIG_VALUE_0", &swag.url)
            .args(["theme", "add", "kitchen", "--rev", rev, "--yes"])
            .output()
            .unwrap();
        assert!(out.status.success(), "{out:?}");
    };
    add(&commit);
    assert_eq!(
        fs::read(s.dir.join("themes/kitchen/sprites.svg")).unwrap(),
        fs::read(s.dir.join("first-party/kitchen/sprites.svg")).unwrap()
    );
    let clone = s.dir.join("data/themes/kitchen");
    assert_eq!(
        git(&clone, &["rev-parse", "--is-shallow-repository"]),
        "true"
    );
    assert_eq!(git(&clone, &["rev-list", "--count", "HEAD"]), "1");
    assert_eq!(git(&clone, &["rev-parse", "HEAD"]), commit);
    let mut files = Vec::new();
    files_outside_git(&clone, &clone, &mut files);
    files.sort();
    let pack = ["icon.png", "scene.svg", "sprites.svg", "theme.json"];
    assert_eq!(files, pack.map(|f| format!("packs/kitchen/{f}")));
    assert!(s.run(&["theme", "remove", "kitchen"]).0);
    add(&commit);
    assert_eq!(git(&clone, &["rev-list", "--count", "HEAD"]), "1");
}

#[test]
fn an_old_full_clone_is_replaced_on_start_once_the_new_one_is_in_place() {
    let dir = scratch("th-migrate");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let (repo, commit) = local_swag(&dir);
    let themes = dir.join("data/themes");
    fs::create_dir_all(&themes).unwrap();
    for name in ["kitchen.git", "sea.git", "gone.git"] {
        let to = themes.join(name).display().to_string();
        git(
            &dir,
            &[
                "clone",
                "--quiet",
                "--mirror",
                &repo.display().to_string(),
                &to,
            ],
        );
    }
    let swag = Swag::pinned();
    let lock = json!([
        { "name": "gone", "source": dir.join("nowhere").display().to_string(), "commit": commit, "listed": false },
        { "name": "kitchen", "source": swag.url, "commit": swag.commit, "listed": true },
        { "name": "sea", "source": repo.display().to_string(), "commit": commit, "listed": false }
    ]);
    fs::write(dir.join("data/themes.lock"), lock.to_string()).unwrap();
    let _cooee = Cooee::start(&dir, &shed.dir);
    until("the space freed is logged", || {
        fs::read_to_string(dir.join("cooee.err"))
            .unwrap()
            .contains("cooee: themes: freed ")
    });
    assert!(!themes.join("kitchen.git").exists() && !themes.join("sea.git").exists());
    assert!(!themes.join("kitchen").exists());
    assert_eq!(git(&themes.join("sea"), &["rev-parse", "HEAD"]), commit);
    assert_eq!(
        git(
            &themes.join("sea"),
            &["rev-parse", "--is-shallow-repository"]
        ),
        "true"
    );
    let err = fs::read_to_string(dir.join("cooee.err")).unwrap();
    assert!(
        err.contains("cooee: theme gone keeps its old clone for now: "),
        "{err}"
    );
    assert!(themes.join("gone.git").is_dir());
}

#[test]
fn the_editor_checks_colours_saves_a_pack_once_and_keeps_its_last_good_copy() {
    let s = setup("th-editor");
    let (ok, out) = s.run(&["theme", "add", "farm"]);
    assert!(ok, "{out}");
    let looks = s.cooee.json("/api/v1/looks");
    let dark = looks["schemes"]["farm"]["tokens"].clone();
    let mut light = looks["schemes"]["farm-light"]["tokens"].clone();
    light["--a"] = json!("#a88a2a");
    let (status, body) = s.cooee.post(
        "/api/v1/themes/check",
        &json!({"dark": dark, "light": light}),
    );
    assert_eq!(status, 200, "{body}");
    let report: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(report["dark"]["fails"], json!([]));
    let mark = &report["light"]["marks"]["--a"];
    assert_eq!(mark["what"], "--a on --card", "{report}");
    assert!(mark["value"].as_f64().unwrap() < 3.0, "{mark}");
    let fix = mark["fix"].as_str().unwrap().to_owned();
    assert!(mark["fixed"].as_f64().unwrap() >= 3.0, "{mark}");
    assert!(
        report["light"]["fails"][0]
            .as_str()
            .unwrap()
            .contains("--a on an amber group header 2."),
        "{report}"
    );

    let draft = |light: &Value| json!({"from": "farm", "label": "Tidepool", "style": "terminal", "dark": dark, "light": light});
    let (status, body) = s
        .cooee
        .request("PUT", "/api/v1/themes/tidepool", Some(&draft(&light)));
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("--a on an amber group header"), "{body}");
    assert!(!s.dir.join("themes/tidepool").exists());

    light["--a"] = json!(fix);
    let (status, body) = s
        .cooee
        .request("PUT", "/api/v1/themes/tidepool", Some(&draft(&light)));
    assert_eq!((status, body.as_str()), (200, r#"{"written":true}"#));
    let pack = s.dir.join("themes/tidepool");
    let (ok, out) = s.run(&["theme", "check", &pack.display().to_string()]);
    assert!(ok, "{out}");
    let looks = s.cooee.json("/api/v1/looks");
    let theme = &looks["themes"]["tidepool"];
    assert_eq!(theme["label"], "Tidepool");
    assert_eq!(theme["style"], "terminal");
    assert_eq!(
        looks["schemes"]["tidepool-light"]["tokens"]["--a"],
        json!(fix)
    );
    assert!(looks["themes"]["farm"].get("style").is_none());
    assert_eq!(s.lock()[1]["source"], "made here");

    let written = fs::metadata(pack.join("theme.json"))
        .unwrap()
        .modified()
        .unwrap();
    let (status, body) = s
        .cooee
        .request("PUT", "/api/v1/themes/tidepool", Some(&draft(&light)));
    assert_eq!((status, body.as_str()), (200, r#"{"written":false}"#));
    assert_eq!(
        fs::metadata(pack.join("theme.json"))
            .unwrap()
            .modified()
            .unwrap(),
        written
    );

    for name in ["farm", "plain", "Bad/name"] {
        let path = format!("/api/v1/themes/{name}");
        let (status, body) = s.cooee.request("PUT", &path, Some(&draft(&light)));
        assert_eq!(
            status,
            if name.contains('/') { 404 } else { 400 },
            "{name}: {body}"
        );
    }

    let svg = fs::read_to_string(pack.join("sprites.svg")).unwrap();
    fs::write(
        pack.join("sprites.svg"),
        svg.replace("id=\"s-fail\"", "id=\"x\""),
    )
    .unwrap();
    let looks = s.cooee.json("/api/v1/looks");
    assert_eq!(looks["themes"]["tidepool"]["label"], "Tidepool");
    assert_eq!(looks["broken"]["tidepool"], "sprites.svg: no s-fail");
    let (_, art) = s.cooee.raw("GET", "/look/tidepool/sprites.svg", &[], "");
    assert!(
        art.contains("id=\"s-fail\""),
        "the last good sheet is served"
    );

    let (ok, out) = s.run(&["theme", "remove", "tidepool"]);
    assert!(ok, "{out}");
    assert!(!s.dir.join(".themes-stage/.good/tidepool").exists());
    assert!(
        s.cooee.json("/api/v1/looks")["themes"]
            .get("tidepool")
            .is_none()
    );
}

#[test]
fn the_editor_saves_drawn_figures_and_a_drawn_backdrop_that_pass_the_gate() {
    let s = setup("th-draw");
    let (ok, out) = s.run(&["theme", "add", "farm"]);
    assert!(ok, "{out}");
    let looks = s.cooee.json("/api/v1/looks");
    let (dark, light) = (
        looks["schemes"]["farm"]["tokens"].clone(),
        looks["schemes"]["farm-light"]["tokens"].clone(),
    );
    let needs = r##"<symbol id="s-needs" viewBox="0 0 24 24" shape-rendering="crispEdges"><g class="f1"><rect x="4" y="8" width="16" height="1" style="fill:var(--fk-l,#ff9e80)"/></g><g class="f2"><rect x="11" y="0" width="2" height="1" fill="#ffffff"/></g></symbol>"##;
    let scene = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 132 100"><style>#night{display:none}#night:target{display:inline}</style><g id="sky"><rect x="0" y="0" width="132" height="1" style="fill:var(--fk-c,#4fd1c5)"/></g><g id="sea"></g><g id="rocks"></g><g id="sand"></g><g id="night"></g></svg>"#;
    let draft = |sprites: Value| json!({"from": "farm", "label": "Rockpool", "style": "scene", "dark": dark, "light": light, "sprites": sprites, "scene": scene});

    for bad in [
        json!({"s-needs": "<symbol id=\"s-fail\"></symbol>"}),
        json!({"s-needs": format!("{needs}{needs}")}),
        json!({"S/x": "<symbol id=\"S/x\"></symbol>"}),
    ] {
        let (status, body) = s
            .cooee
            .request("PUT", "/api/v1/themes/rockpool", Some(&draft(bad)));
        assert_eq!(status, 400, "{body}");
        assert!(body.contains("not one <symbol"), "{body}");
    }
    assert!(!s.dir.join("themes/rockpool").exists());

    let (status, body) = s.cooee.request(
        "PUT",
        "/api/v1/themes/rockpool",
        Some(&draft(json!({ "s-needs": needs }))),
    );
    assert_eq!((status, body.as_str()), (200, r#"{"written":true}"#));
    let pack = s.dir.join("themes/rockpool");
    let (ok, out) = s.run(&["theme", "check", &pack.display().to_string()]);
    assert!(ok, "{out}");
    let sheet = fs::read_to_string(pack.join("sprites.svg")).unwrap();
    let farm = fs::read_to_string(s.dir.join("first-party/farm/sprites.svg")).unwrap();
    assert_eq!(sheet.matches("id=\"s-needs\"").count(), 1);
    assert!(sheet.contains(needs));
    let (before, after) = farm.split_once("<symbol id=\"s-needs\"").unwrap();
    assert!(sheet.starts_with(before), "the other symbols stay in place");
    let rest = after.split_once("</symbol>").unwrap().1;
    assert!(sheet.ends_with(rest));
    assert_eq!(fs::read_to_string(pack.join("scene.svg")).unwrap(), scene);
    let looks = s.cooee.json("/api/v1/looks");
    assert_eq!(
        looks["themes"]["rockpool"]["scene"],
        json!(["sky", "sea", "rocks", "sand"])
    );

    let (status, body) = s.cooee.request(
        "PUT",
        "/api/v1/themes/rockpool",
        Some(&draft(json!({ "s-needs": needs }))),
    );
    assert_eq!((status, body.as_str()), (200, r#"{"written":false}"#));
}

#[test]
fn an_imported_svg_kept_as_vector_in_a_figure_and_a_layer_passes_the_gate() {
    let s = setup("th-import");
    let (ok, out) = s.run(&["theme", "add", "farm"]);
    assert!(ok, "{out}");
    let looks = s.cooee.json("/api/v1/looks");
    let vector = r##"<svg viewBox="0 0 48 32" width="100%" height="100%" xmlns:xlink="http://www.w3.org/1999/xlink"><circle id="dot" cx="24" cy="16" r="10" fill="#f80"/><use xlink:href="#dot"/></svg>"##;
    let needs = format!(r#"<symbol id="s-needs" viewBox="0 0 24 24">{vector}</symbol>"#);
    let scene = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 132 100" preserveAspectRatio="xMidYMax slice" shape-rendering="crispEdges"><style>#night{{display:none}}#night:target{{display:inline}}</style><g id="sky">{vector}</g><g id="sea"></g><g id="rocks"></g><g id="sand"></g><g id="night"></g></svg>"#
    );
    let draft = json!({"from": "farm", "label": "Inked", "style": "scene",
        "dark": looks["schemes"]["farm"]["tokens"], "light": looks["schemes"]["farm-light"]["tokens"],
        "sprites": {"s-needs": needs}, "scene": scene});

    let (status, body) = s.cooee.request("PUT", "/api/v1/themes/inked", Some(&draft));
    assert_eq!((status, body.as_str()), (200, r#"{"written":true}"#));
    let pack = s.dir.join("themes/inked");
    let (ok, out) = s.run(&["theme", "check", &pack.display().to_string()]);
    assert!(ok, "{out}");
    let sheet = fs::read_to_string(pack.join("sprites.svg")).unwrap();
    assert!(sheet.contains(&needs), "{sheet}");
    assert_eq!(fs::read_to_string(pack.join("scene.svg")).unwrap(), scene);
}
