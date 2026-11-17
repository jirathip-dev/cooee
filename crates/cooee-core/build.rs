//! Vendors the first-party theme packs (#318): each file `packs/swag.toml` pins, from jirathip-dev/swag's `packs/` at
//! the pinned commit, into `packs/themes/` (git-ignored; fetched again when `packs/themes.commit` names another), and writes
//! `$OUT_DIR/swag.rs`, the files embedded for `cooee_core::swag`. Their sha256 is checked there, at install and by test.
//! It also embeds `connectors/*.toml` (#497) as `$OUT_DIR/connectors.rs`, for `cooee_core::connectors`.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

type Res<T> = Result<T, Box<dyn std::error::Error>>;

fn git(dir: &Path, args: &[&str]) -> Res<Vec<u8>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()?;
    if !out.status.success() {
        return Err(format!(
            "packs/swag.toml: git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        )
        .into());
    }
    Ok(out.stdout)
}

fn main() -> Res<()> {
    let root = Path::new(&env::var("CARGO_MANIFEST_DIR")?).join("../..");
    let out = PathBuf::from(env::var("OUT_DIR")?);
    let (pin, packs) = (root.join("packs/swag.toml"), root.join("packs/themes"));
    let mark = root.join("packs/themes.commit");
    println!("cargo::rerun-if-changed={}", pin.display());
    println!("cargo::rerun-if-changed={}", mark.display());
    let pin: toml::Table = fs::read_to_string(&pin)?.parse()?;
    let field = |k: &str| pin.get(k).ok_or(format!("packs/swag.toml: no {k}"));
    let (url, commit) = (field("url")?.as_str(), field("commit")?.as_str());
    let (url, commit) = (url.ok_or("a url")?, commit.ok_or("a commit")?);
    let files: Vec<&String> = field("sha256")?
        .as_table()
        .ok_or("[sha256]")?
        .keys()
        .collect();
    if fs::read_to_string(&mark).ok().as_deref() != Some(commit) {
        let repo = out.join("swag.git");
        if !repo.exists() {
            git(&out, &["init", "--quiet", "--bare", "swag.git"])?;
        }
        git(&repo, &["fetch", "--quiet", "--depth", "1", url, commit])?;
        if packs.exists() {
            fs::remove_dir_all(&packs)?;
        }
        for f in &files {
            let to = packs.join(f);
            fs::create_dir_all(to.parent().ok_or("a pack folder")?)?;
            let blob = git(&repo, &["cat-file", "blob", &format!("{commit}:packs/{f}")])?;
            fs::write(to, blob)?;
        }
        fs::write(&mark, commit)?;
    }
    let entries = files
        .iter()
        .map(|f| {
            let at = fs::canonicalize(packs.join(f))?;
            Ok(format!(
                "({f:?}, include_bytes!({:?}))",
                at.display().to_string()
            ))
        })
        .collect::<Res<Vec<String>>>()?;
    fs::write(out.join("swag.rs"), format!("&[{}]\n", entries.join(",\n")))?;
    let dir = root.join("connectors");
    println!("cargo::rerun-if-changed={}", dir.display());
    let mut found = Vec::new();
    for f in fs::read_dir(&dir)? {
        let at = fs::canonicalize(f?.path())?;
        if let Some(name) = at
            .file_stem()
            .filter(|_| at.extension() == Some("toml".as_ref()))
        {
            println!("cargo::rerun-if-changed={}", at.display());
            let name = name.to_string_lossy().into_owned();
            found.push(format!(
                "({name:?}, include_str!({:?}))",
                at.display().to_string()
            ));
        }
    }
    found.sort();
    fs::write(
        out.join("connectors.rs"),
        format!("&[{}]\n", found.join(",\n")),
    )?;
    Ok(())
}
