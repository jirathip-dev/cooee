//! Folders from the phone: Settings › Folders browses the home folder (folder names only) and adds or removes roots,
//! saved in the config file; the new-thread picker lists the roots 2 levels deep and makes a new folder inside one;
//! nothing outside the home folder (browsing, roots) or the roots (the picker, new folders, threads) is accepted.

mod support;

use std::{fs, os::unix::fs::symlink, path::Path};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, scratch, version};

/// An instance with `HOME` at `dir/home`, holding `Projects/app/src`, a hidden folder, a file and a symlink out.
fn start(name: &str) -> (std::path::PathBuf, FakeShed, Cooee) {
    let dir = scratch(name);
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start_instance(&dir, &shed.dir, "cooee");
    let home = dir.join("home");
    fs::create_dir_all(home.join("Projects/app/src")).unwrap();
    fs::create_dir_all(home.join(".hidden")).unwrap();
    fs::create_dir_all(dir.join("outside")).unwrap();
    fs::write(home.join("notes.txt"), "secret").unwrap();
    symlink(dir.join("outside"), home.join("out")).unwrap();
    symlink(dir.join("outside"), home.join("Projects/out")).unwrap();
    (home, shed, cooee)
}

fn names(v: &Value) -> Vec<&str> {
    v["dirs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d.as_str().unwrap())
        .collect()
}

fn q(p: &Path) -> String {
    p.display().to_string().replace('/', "%2F")
}

#[test]
fn browse_lists_only_folders_inside_home() {
    let (home, _shed, cooee) = start("folders-browse");
    let v = cooee.json("/folders");
    assert_eq!(v["path"], json!(home));
    assert_eq!(v["home"], json!(home));
    assert_eq!(names(&v), ["Projects"]);
    let v = cooee.json("/folders?hidden=1");
    assert_eq!(names(&v), [".config", ".hidden", ".local", "Projects"]);
    let v = cooee.json(&format!("/folders?path={}", q(&home.join("Projects"))));
    assert_eq!(names(&v), ["app"]);

    for path in [
        home.join(".."),
        home.join("Projects/../.."),
        "/etc".into(),
        home.join("out"),
        home.join("notes.txt"),
    ] {
        let (status, body) = cooee.get(&format!("/folders?path={}", q(&path)));
        assert_eq!(status, 400, "{}: {body}", path.display());
        assert!(
            body.contains("is not a folder in your home folder"),
            "{body}"
        );
    }
    let (head, _) = cooee.raw("GET", "/folders", &[], "");
    assert!(head.starts_with("HTTP/1.0 401"), "{head}");
}

#[test]
fn add_and_remove_root_persists_in_the_config() {
    let (home, _shed, cooee) = start("folders-roots");
    let projects = home.join("Projects");
    let (status, body) = cooee.post("/roots", &json!({ "path": projects }));
    assert_eq!(status, 200, "{body}");
    assert!(body.contains(&*projects.to_string_lossy()), "{body}");
    let text = fs::read_to_string(&cooee.config).unwrap();
    assert!(
        text.contains(r#""~/Projects"]"#) && text.contains("shed_dir = "),
        "{text}"
    );
    let (_, page) = cooee.get("/");
    let sub = format!(
        r#"name="folder" value="{}""#,
        projects.join("app").display()
    );
    assert!(page.contains(&sub), "{page}");

    for path in [home.join("out"), home.join(".."), "/etc".into()] {
        let (status, body) = cooee.post("/roots", &json!({ "path": path }));
        assert_eq!(status, 400, "{}: {body}", path.display());
    }
    assert!(
        !fs::read_to_string(&cooee.config)
            .unwrap()
            .contains("outside")
    );

    let (status, body) = cooee.post("/roots", &json!({ "path": projects, "remove": true }));
    assert_eq!(status, 200, "{body}");
    assert!(!body.contains("Projects"), "{body}");
    assert!(
        !fs::read_to_string(&cooee.config)
            .unwrap()
            .contains("Projects")
    );
    let (_, page) = cooee.get("/");
    assert!(!page.contains("Projects"), "{page}");
}

#[test]
fn picker_lists_sub_folders_two_levels_deep_never_symlinks() {
    let (home, _shed, cooee) = start("folders-picker");
    let projects = home.join("Projects");
    cooee.post("/roots", &json!({ "path": projects }));
    let (_, page) = cooee.get("/");
    let row = |p: &Path, d: u8| {
        let hidden = if d > 1 { r#" hidden="""# } else { "" };
        format!(
            r#"<label data-d="{d}"{hidden}><input type="radio" name="folder" value="{}""#,
            p.display()
        )
    };
    for (p, d) in [
        (projects.clone(), 0),
        (projects.join("app"), 1),
        (projects.join("app/src"), 2),
    ] {
        assert!(page.contains(&row(&p, d)), "{}: {page}", p.display());
    }
    assert!(page.contains("~/Projects/app/src<span"), "{page}");
    assert!(page.contains(r#"id="fq" type="search""#), "{page}");
    assert!(!page.contains("Projects/out"), "{page}");
}

#[test]
fn new_folder_is_made_inside_a_root_and_a_thread_starts_there() {
    let (home, shed, cooee) = start("folders-new");
    let projects = home.join("Projects");
    cooee.post("/roots", &json!({ "path": projects }));
    let (status, body) = cooee.post(
        "/folders/new",
        &json!({ "parent": projects.join("app"), "name": "fresh" }),
    );
    assert_eq!(status, 200, "{body}");
    let fresh = projects.join("app/fresh");
    assert_eq!(body, json!(fresh).to_string());
    assert!(fresh.is_dir());
    let (status, _) = cooee.post(
        "/folders/new",
        &json!({ "parent": projects.join("app"), "name": "fresh" }),
    );
    assert_eq!(status, 400);

    let new = json!({ "agent": "fake-acp", "folder": fresh, "text": "hi", "random_id": "r0" });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    assert!(
        shed.cli_log().contains(&format!(
            "--cwd {} ",
            fresh.canonicalize().unwrap().display()
        )),
        "{}",
        shed.cli_log()
    );
}

#[test]
fn dot_dot_symlink_and_absolute_escapes_are_refused() {
    let (home, _shed, cooee) = start("folders-escape");
    let projects = home.join("Projects");
    cooee.post("/roots", &json!({ "path": projects }));
    let outside = home.parent().unwrap().join("outside");
    for name in ["../x", "..", ".", "/tmp/x", "a/b", ".x", ""] {
        let (status, body) =
            cooee.post("/folders/new", &json!({ "parent": projects, "name": name }));
        assert_eq!(status, 400, "{name}: {body}");
    }
    for parent in [projects.join("out"), projects.join(".."), outside.clone()] {
        let (status, body) = cooee.post("/folders/new", &json!({ "parent": parent, "name": "x" }));
        assert_eq!(status, 400, "{}: {body}", parent.display());
        assert!(body.contains("is not inside a configured root"), "{body}");
    }
    assert!(!projects.parent().unwrap().join("x").exists());
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
    for folder in [projects.join("out"), projects.join("../.."), "/".into()] {
        let new = json!({ "agent": "fake-acp", "folder": folder, "text": "hi", "random_id": "r1" });
        let (status, body) = cooee.post("/threads", &new);
        assert_eq!(status, 400, "{}: {body}", folder.display());
    }
}
