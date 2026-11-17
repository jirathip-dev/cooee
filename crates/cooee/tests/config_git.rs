//! #361: `cooee config init --git` makes one first commit that leaves the secrets out, and refuses a second time;
//! `cooee serve` adds a `.gitignore` to its own config folder only, never over the owner's.

mod support;

use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

use support::{Cooee, FakeShed, scratch, version};

const BIN: &str = env!("CARGO_BIN_EXE_cooee");

fn run(home: &Path, prog: &str, args: &[&str]) -> Output {
    Command::new(prog)
        .args(args)
        .env("HOME", home)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "owner")
        .env("GIT_AUTHOR_EMAIL", "owner@example.com")
        .env("GIT_COMMITTER_NAME", "owner")
        .env("GIT_COMMITTER_EMAIL", "owner@example.com")
        .output()
        .unwrap()
}

fn git(home: &Path, dir: &Path, args: &[&str]) -> String {
    let mut all = vec!["-C", dir.to_str().unwrap()];
    all.extend_from_slice(args);
    let out = run(home, "git", &all);
    assert!(out.status.success(), "{out:?}");
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn init_git_commits_the_config_without_its_secrets_once() {
    let home = scratch("config-git");
    let dir = home.join("config");
    fs::create_dir_all(dir.join("agents/coder")).unwrap();
    fs::create_dir_all(dir.join("secrets")).unwrap();
    fs::write(dir.join("config.toml"), "port = 4100\n").unwrap();
    fs::write(dir.join("agents/coder/agent.toml"), "name = \"Coder\"\n").unwrap();
    fs::write(dir.join("secrets/TOKEN"), "s3cret\n").unwrap();
    fs::write(dir.join("apns.p8"), "-----BEGIN PRIVATE KEY-----\n").unwrap();
    let config = dir.join("config.toml");
    let args = [
        "config",
        "init",
        "--git",
        "--config",
        config.to_str().unwrap(),
    ];

    let first = run(&home, BIN, &args);
    assert!(first.status.success(), "{first:?}");
    let files = git(&home, &dir, &["ls-tree", "-r", "--name-only", "HEAD"]);
    assert_eq!(
        files.lines().collect::<Vec<_>>(),
        [".gitignore", "agents/coder/agent.toml", "config.toml"]
    );
    assert_eq!(git(&home, &dir, &["rev-list", "--count", "--all"]), "1\n");
    assert_eq!(git(&home, &dir, &["remote"]), "");

    let head = git(&home, &dir, &["rev-parse", "HEAD"]);
    fs::write(dir.join("routines.toml"), "").unwrap();
    let second = run(&home, BIN, &args);
    assert!(!second.status.success(), "{second:?}");
    assert!(String::from_utf8_lossy(&second.stderr).contains("a git repo already"));
    assert_eq!(git(&home, &dir, &["rev-parse", "HEAD"]), head);
    assert_eq!(git(&home, &dir, &["rev-list", "--count", "--all"]), "1\n");
    assert_eq!(
        git(&home, &dir, &["status", "--porcelain"]),
        "?? routines.toml\n"
    );
    fs::remove_dir_all(&home).unwrap();
}

#[test]
fn init_git_keeps_the_owners_gitignore() {
    let home = scratch("config-git-own");
    let dir = home.join("config");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(".gitignore"), "mine\n").unwrap();
    fs::write(dir.join("mine"), "").unwrap();
    let config = dir.join("config.toml");
    let args = [
        "config",
        "init",
        "--git",
        "--config",
        config.to_str().unwrap(),
    ];

    let out = run(&home, BIN, &args);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(
        fs::read_to_string(dir.join(".gitignore")).unwrap(),
        "mine\n"
    );
    let files = git(&home, &dir, &["ls-tree", "-r", "--name-only", "HEAD"]);
    assert_eq!(files, ".gitignore\n");
    fs::remove_dir_all(&home).unwrap();
}

#[test]
fn serve_adds_a_gitignore_to_its_own_config_folder_only() {
    let dir = scratch("config-git-serve");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let own = dir.join("home/.config/cooee");
    fs::create_dir_all(own.join("drover")).unwrap();
    fs::write(own.join("drover/.gitignore"), "mine\n").unwrap();

    let a = Cooee::start_instance(&dir, &shed.dir, "cooee");
    let b = Cooee::start_instance(&dir, &shed.dir, "drover");
    let other = dir.join("other");
    fs::create_dir_all(&other).unwrap();
    let c = Cooee::start(&other, &shed.dir);

    let ignore = fs::read_to_string(own.join("cooee/.gitignore")).unwrap();
    assert!(ignore.lines().any(|l| l == "secrets/"), "{ignore}");
    assert_eq!(
        fs::read_to_string(own.join("drover/.gitignore")).unwrap(),
        "mine\n"
    );
    assert!(!other.join(".gitignore").exists());
    drop((a, b, c));
    fs::remove_dir_all(&dir).unwrap();
}
