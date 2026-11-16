//! Vendors the first-party theme packs (#318): each file `packs/swag.toml` pins, from jirathip-dev/swag's `packs/` at
//! the pinned commit, into `packs/themes/` (git-ignored; fetched again when `packs/themes.commit` names another), and writes
//! `$OUT_DIR/swag.rs`, the files embedded for `cooee_core::swag`. Their sha256 is checked there, at install and by test.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

fn git(dir: &Path, args: &[&str]) -> Vec<u8> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "packs/swag.toml: git {}: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr).trim()
    );
    out.stdout
}

fn main() {
    let root = Path::new(&env::var("CARGO_MANIFEST_DIR").expect("cargo sets it")).join("../..");
    let out = PathBuf::from(env::var("OUT_DIR").expect("cargo sets OUT_DIR"));
    let (pin, packs) = (root.join("packs/swag.toml"), root.join("packs/themes"));
    let mark = root.join("packs/themes.commit");
    println!("cargo::rerun-if-changed={}", pin.display());
    println!("cargo::rerun-if-changed={}", mark.display());
    let pin: toml::Table = fs::read_to_string(&pin)
        .expect("packs/swag.toml")
        .parse()
        .expect("packs/swag.toml parses");
    let (url, commit) = (pin["url"].as_str(), pin["commit"].as_str());
    let (url, commit) = (url.expect("a url"), commit.expect("a commit"));
    let files: Vec<&String> = pin["sha256"].as_table().expect("[sha256]").keys().collect();
    if fs::read_to_string(&mark).ok().as_deref() != Some(commit) {
        let repo = out.join("swag.git");
        if !repo.exists() {
            git(&out, &["init", "--quiet", "--bare", "swag.git"]);
        }
        git(&repo, &["fetch", "--quiet", "--depth", "1", url, commit]);
        let _ = fs::remove_dir_all(&packs);
        for f in &files {
            let to = packs.join(f);
            fs::create_dir_all(to.parent().expect("a pack folder")).expect("packs/themes");
            let blob = git(&repo, &["cat-file", "blob", &format!("{commit}:packs/{f}")]);
            fs::write(to, blob).expect("packs/themes is writable");
        }
        fs::write(&mark, commit).expect("packs/themes is writable");
    }
    let entries: Vec<String> = files
        .iter()
        .map(|f| {
            let at = fs::canonicalize(packs.join(f)).expect("a vendored file");
            format!("({f:?}, include_bytes!({:?}))", at.display().to_string())
        })
        .collect();
    fs::write(out.join("swag.rs"), format!("&[{}]\n", entries.join(",\n")))
        .expect("OUT_DIR is writable");
}
