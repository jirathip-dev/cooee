//! Theme packs end to end (docs/themes.md): `cooee theme check` passes the first-party packs and fails a broken one;
//! `cooee theme add` from a fake index (a local swag: a git repo holding packs in folders, pinned, each fetched as a
//! shallow clone of its folder alone), the built-in farm offline, a git URL ("not reviewed") and a folder, each pinned
//! in the lock; an old full clone is replaced on start; a pack failing the gate is never installed; list and remove, which `cooee serve`
//! shows without a restart. From the phone: [remix] copies a theme into themes-dev with an agent thread; its preview's
//! events say `saved` on each save; [keep] is blocked while it fails the gate (the failures go to its chat) and moves
//! it into themes/ once it passes; [publish] makes it a git repo.

mod support;

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    path::{Path, PathBuf},
    process::Command,
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

fn packs() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs")
}

/// The swag fixture's packs (jirathip-dev/swag's `packs/`, farm aside: it ships in `packs/farm`).
fn swag() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/swag/packs")
}

/// Copies folder `from` into `to`.
fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for f in fs::read_dir(from).unwrap() {
        let f = f.unwrap();
        fs::copy(f.path(), to.join(f.file_name())).unwrap();
    }
}

/// farm's pack copied to `to`, its `s-needs` gone when `broken`.
fn farm_at(to: &Path, broken: bool) {
    fs::create_dir_all(to).unwrap();
    for f in ["theme.json", "sprites.svg", "scene.svg", "icon.png"] {
        fs::copy(packs().join("farm").join(f), to.join(f)).unwrap();
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

/// `cooee serve` with an index listing `farm`, `kitchen` and `broken`, folders of a local swag at its second commit;
/// and `sea.git`, a repo that is a pack at its root. `before` runs on the folder before the server starts.
struct Setup {
    dir: PathBuf,
    shed: FakeShed,
    cooee: Cooee,
    commit: String,
}

fn setup(name: &str) -> Setup {
    setup_then(name, |_| {})
}

fn setup_then(name: &str, before: impl FnOnce(&Path)) -> Setup {
    let dir = scratch(name);
    let shed = FakeShed::start(&dir.join("shed"), version());
    let repo = dir.join("swag.git");
    copy_dir(&swag().join("garden"), &repo.join("packs/garden"));
    git(&repo, &["init", "--quiet", "-b", "main"]);
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "--quiet", "-m", "garden"]);
    farm_at(&repo.join("packs/farm"), false);
    farm_at(&repo.join("packs/broken"), true);
    copy_dir(&swag().join("kitchen"), &repo.join("packs/kitchen"));
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
            "[[theme]]\nname = \"{n}\"\nurl = {:?}\npath = \"packs/{n}\"\ncommit = \"{commit}\"\n\
             description = \"{n} pack\"\nfirst_party = true\n\n",
            repo.to_string_lossy()
        )
    };
    let index = entry("farm") + &entry("kitchen") + &entry("broken");
    fs::write(dir.join("index.toml"), index).unwrap();
    let extra = format!(
        "plugin_index = {:?}\n",
        dir.join("index.toml").to_string_lossy()
    );
    before(&dir);
    let cooee = Cooee::start_with(&dir, &shed.dir, &extra);
    Setup {
        dir,
        shed,
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

    /// A form `body` posted to `/themes/<act>` as the paired phone: the status and where it goes.
    fn act(&self, act: &str, body: &str) -> (u16, String) {
        let headers = [
            ("Content-Type", "application/x-www-form-urlencoded"),
            ("Cookie", self.cooee.cookie.as_str()),
        ];
        let (head, text) = self
            .cooee
            .raw("POST", &format!("/themes/{act}"), &headers, body);
        let to = head
            .lines()
            .find_map(|l| l.strip_prefix("location: "))
            .map_or(text, str::to_owned);
        (head[9..12].parse().unwrap(), to)
    }

    fn serves(&self, theme: &str) -> bool {
        self.cooee.get(&format!("/look/{theme}/theme.json")).0 == 200
    }
}

#[test]
fn check_passes_the_first_party_packs_and_fails_a_broken_one() {
    let s = setup("th-check");
    let first: Vec<String> = [
        packs().join("farm"),
        swag().join("kitchen"),
        swag().join("garden"),
    ]
    .map(|t| t.display().to_string())
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
        fs::read(packs().join("farm/sprites.svg")).unwrap()
    );
    assert_eq!(
        s.lock(),
        json!([{ "name": "farm", "source": "index", "commit": s.commit, "listed": true }])
    );
    // The server shows it without a restart.
    until("farm is served", || s.serves("farm"));

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
    let kitchen = swag().join("kitchen").display().to_string();
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
    until("farm is gone", || !s.serves("farm"));
    let (ok, out) = s.run(&["theme", "remove", "farm"]);
    assert!(!ok && out.contains("farm isn't installed"), "{out}");
}

#[test]
fn a_pack_with_a_symlink_is_never_installed_or_served() {
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
    let (code, body) = s.cooee.get("/look/evil/icon.png");
    assert!(code != 200 && !body.contains("TOP-SECRET"), "{code} {body}");
}

/// Reads `path`'s SSE stream until an event named `name`.
fn wait_for_event(s: &Setup, path: &str, name: &str, then: impl FnOnce()) {
    let mut stream = TcpStream::connect(&s.cooee.addr).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(20)))
        .unwrap();
    write!(
        stream,
        "GET {path} HTTP/1.0\r\nHost: {}\r\nCookie: {}\r\n\r\n",
        s.cooee.addr, s.cooee.cookie
    )
    .unwrap();
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    assert!(line.contains(" 200 "), "{line}");
    then();
    loop {
        line.clear();
        assert!(reader.read_line(&mut line).unwrap() > 0, "the stream ended");
        if line.trim_end() == format!("event: {name}") {
            return;
        }
    }
}

#[test]
fn remix_preview_keep_and_publish_from_the_phone() {
    let s = setup("th-remix");
    assert!(s.run(&["theme", "add", "farm"]).0);
    until("farm is served", || s.serves("farm"));
    // Settings › look › themes lists it with [remix], and the index's others to add.
    let (_, page) = s.cooee.get("/");
    assert!(
        page.contains(r#"<input type="hidden" name="from" value="farm">"#)
            && page.contains(r#"<input type="hidden" name="source" value="broken">"#),
        "{page}"
    );

    let (status, to) = s.act("remix", "from=farm&name=sea-farm&text=make+it+blue");
    assert_eq!(status, 303, "{to}");
    let thread = to.strip_prefix("/t/").unwrap().to_owned();
    let dev = s.dir.join("themes-dev/sea-farm");
    assert_eq!(
        fs::read(dev.join("sprites.svg")).unwrap(),
        fs::read(s.dir.join("themes/farm/sprites.svg")).unwrap()
    );
    let run = s.shed.cli_log();
    assert!(run.contains(&format!("--cwd {}", dev.display())), "{run}");
    let sent = s.shed.sent(&format!("cooee-{thread}"));
    assert!(
        sent[0]
            .contains("Theme sea-farm: make it blue\\n\\nYou're building the cooee theme sea-farm"),
        "{sent:?}"
    );
    // Its chat has the preview and [keep]; the preview shows it in the list and the chat.
    let (_, chat) = s.cooee.get(&to);
    assert!(
        chat.contains(&format!(r#"href="/themes/dev/sea-farm?thread={thread}""#)),
        "{chat}"
    );
    let (status, preview) = s
        .cooee
        .get(&format!("/themes/dev/sea-farm?thread={thread}"));
    assert_eq!(status, 200, "{preview}");
    assert!(
        preview.contains("passes the theme gate")
            && preview.contains(&format!(r#"src="/t/{thread}?preview=sea-farm""#)),
        "{preview}"
    );
    let (_, list) = s.cooee.get("/?preview=sea-farm");
    assert!(list.contains(r#"data-theme="dev.sea-farm""#), "{list}");

    // The agent saves a change that breaks a state: the preview's stream says `saved`.
    wait_for_event(&s, "/themes/dev/sea-farm/events", "saved", || {
        let svg = fs::read_to_string(dev.join("sprites.svg")).unwrap();
        fs::write(
            dev.join("sprites.svg"),
            svg.replace("id=\"s-needs\"", "id=\"x\""),
        )
        .unwrap();
    });
    // [keep] is blocked: the failures go to the chat, for the agent.
    let body = format!("name=sea-farm&thread={thread}");
    assert_eq!(s.act("keep", &body), (303, to.clone()));
    assert!(dev.exists() && !s.dir.join("themes/sea-farm").exists());
    until("the failures reach the agent", || {
        s.shed
            .sent(&format!("cooee-{thread}"))
            .iter()
            .any(|m| m.contains("[keep] it fails the theme gate:\\n  sprites.svg: no s-needs"))
    });
    let (_, preview) = s.cooee.get("/themes/dev/sea-farm");
    assert!(
        preview.contains("fails the theme gate")
            && preview.contains("<li>sprites.svg: no s-needs</li>"),
        "{preview}"
    );

    // Fixed, [keep] moves it into themes/: installed, pinned, served.
    farm_at(&dev, false);
    assert_eq!(s.act("keep", &body), (303, "/#settings/look".to_owned()));
    assert!(!dev.exists() && s.dir.join("themes/sea-farm/theme.json").exists());
    assert_eq!(
        s.lock()[1],
        json!({ "name": "sea-farm", "source": "made here", "commit": "", "listed": false })
    );
    assert!(s.serves("sea-farm"));

    let (status, said) = s.act("publish", "name=sea-farm");
    assert_eq!(status, 200, "{said}");
    let repo = s.dir.join("themes/sea-farm");
    let commit = git(&repo, &["rev-parse", "HEAD"]);
    assert!(
        said.contains(&format!("is a git repo now, at {}", &commit[..7])),
        "{said}"
    );
    assert_eq!(s.lock()[1]["commit"], json!(commit));

    // [new theme] from the starter keeps as it is, with the Scene art its sprites need.
    let (status, to) = s.act("new", "name=fresh");
    assert_eq!(status, 303, "{to}");
    let thread = to.strip_prefix("/t/").unwrap();
    let body = format!("name=fresh&thread={thread}");
    assert_eq!(s.act("keep", &body), (303, "/#settings/look".to_owned()));
    let (status, art) = s.cooee.get("/look/fresh/scene.svg");
    assert!(
        status == 200 && art.contains("id=\"night\""),
        "{status} {art}"
    );

    // A name taken or not a theme's is refused.
    assert_eq!(s.act("new", "name=sea-farm").0, 409);
    assert_eq!(s.act("new", "name=..").0, 409);
}

/// The files under `dir` but its `.git`, relative to `under`.
fn files_outside_git(dir: &Path, under: &Path, out: &mut Vec<String>) {
    for e in fs::read_dir(dir).unwrap().map(Result::unwrap) {
        if e.file_name() == ".git" {
            continue;
        }
        if e.file_type().unwrap().is_dir() {
            files_outside_git(&e.path(), under, out);
        } else {
            let rel = e.path().strip_prefix(under).unwrap().to_owned();
            out.push(rel.display().to_string());
        }
    }
}

#[test]
fn an_index_pack_is_a_shallow_clone_of_its_folder_alone() {
    let s = setup("th-shallow");
    let (ok, out) = s.run(&["theme", "add", "kitchen"]);
    assert!(ok, "{out}");
    assert_eq!(
        fs::read(s.dir.join("themes/kitchen/sprites.svg")).unwrap(),
        fs::read(swag().join("kitchen/sprites.svg")).unwrap()
    );
    // Its clone: depth 1 of a repo with two commits, only packs/kitchen checked out.
    let clone = s.dir.join("data/themes/kitchen");
    assert_eq!(
        git(&clone, &["rev-parse", "--is-shallow-repository"]),
        "true"
    );
    assert_eq!(git(&clone, &["rev-list", "--count", "HEAD"]), "1");
    assert_eq!(git(&clone, &["rev-parse", "HEAD"]), s.commit);
    let mut files = Vec::new();
    files_outside_git(&clone, &clone, &mut files);
    files.sort();
    let pack = [
        "SHA256SUMS",
        "SOURCE",
        "icon.png",
        "scene.svg",
        "sprites.svg",
        "theme.json",
    ];
    assert_eq!(files, pack.map(|f| format!("packs/kitchen/{f}")));
    // Added again after a remove, the same clone fetches again.
    assert!(s.run(&["theme", "remove", "kitchen"]).0);
    let (ok, out) = s.run(&["theme", "add", "kitchen"]);
    assert!(ok, "{out}");
    assert_eq!(git(&clone, &["rev-list", "--count", "HEAD"]), "1");
}

#[test]
fn farm_ships_in_the_build_so_a_first_start_offline_shows_it() {
    let dir = scratch("th-offline");
    let shed = FakeShed::start(&dir.join("shed"), version());
    // The built-in index; git can't reach GitHub.
    let cooee = Cooee::start(&dir, &shed.dir);
    let out = cooee
        .command()
        .env("GIT_CONFIG_COUNT", "1")
        .env(
            "GIT_CONFIG_KEY_0",
            format!("url.{}/offline/.insteadOf", dir.display()),
        )
        .env("GIT_CONFIG_VALUE_0", "https://github.com/")
        .args(["theme", "add", "kitchen"])
        .output()
        .unwrap();
    assert!(!out.status.success(), "{out:?}");
    let out = cooee
        .command()
        .env("GIT_CONFIG_COUNT", "1")
        .env(
            "GIT_CONFIG_KEY_0",
            format!("url.{}/offline/.insteadOf", dir.display()),
        )
        .env("GIT_CONFIG_VALUE_0", "https://github.com/")
        .args(["theme", "add", "farm"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    assert!(!dir.join("data/themes/farm").exists());
    assert_eq!(
        fs::read(dir.join("themes/farm/sprites.svg")).unwrap(),
        fs::read(packs().join("farm/sprites.svg")).unwrap()
    );
    until("farm shows", || {
        cooee.get("/").1.contains(r#"data-theme="farm""#)
    });
}

#[test]
fn an_old_full_clone_is_replaced_on_start_once_the_new_one_is_in_place() {
    let s = setup_then("th-migrate", |dir| {
        let themes = dir.join("data/themes");
        fs::create_dir_all(&themes).unwrap();
        let repo = dir.join("swag.git").display().to_string();
        for name in ["kitchen.git", "gone.git"] {
            let to = themes.join(name).display().to_string();
            git(dir, &["clone", "--quiet", "--mirror", &repo, &to]);
        }
        let pin = |n: &str| json!({ "name": n, "source": "index", "commit": "0".repeat(40), "listed": true });
        let lock = json!([pin("gone"), pin("kitchen")]).to_string();
        fs::write(dir.join("data/themes.lock"), lock).unwrap();
    });
    let themes = s.dir.join("data/themes");
    until("kitchen's old clone is gone", || {
        !themes.join("kitchen.git").exists()
    });
    let clone = themes.join("kitchen");
    assert_eq!(
        git(&clone, &["rev-parse", "--is-shallow-repository"]),
        "true"
    );
    assert_eq!(git(&clone, &["rev-parse", "HEAD"]), s.commit);
    // gone isn't in the index any more: no new form, so its old clone stays.
    until("the space freed is logged", || {
        fs::read_to_string(s.dir.join("cooee.err"))
            .unwrap()
            .contains("cooee: themes: freed ")
    });
    let err = fs::read_to_string(s.dir.join("cooee.err")).unwrap();
    assert!(
        err.contains("cooee: theme gone keeps its old clone for now: gone isn't in the index"),
        "{err}"
    );
    assert!(themes.join("gone.git").is_dir());
}
