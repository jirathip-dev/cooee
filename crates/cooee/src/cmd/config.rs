use std::{fs, path::Path};

use crate::cli::{Failed, Flag, Invocation, migrate};

pub(super) const USAGE: &str = "config migrate [--dry-run] | config init --git";
pub(super) const FLAGS: &[Flag] = &[Flag::Switch("--dry-run"), Flag::Switch("--git")];

pub(super) fn run(inv: &Invocation) -> Result<(), Failed> {
    match inv.rest().as_slice() {
        ["migrate"] => Ok(migrate(&inv.config()?, inv.on("--dry-run"))?),
        ["init"] if inv.on("--git") => {
            let dir = inv
                .config()?
                .dir
                .ok_or_else(|| "no config folder: set HOME".to_owned())?;
            Ok(init_git(&dir)?)
        }
        _ => Err(Failed::Usage),
    }
}

fn init_git(dir: &Path) -> Result<(), String> {
    if dir.join(".git").exists() {
        return Err(format!(
            "{} is a git repo already; nothing changed",
            dir.display()
        ));
    }
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    cooee_core::gitignore::write(dir).map_err(|e| e.to_string())?;
    git(dir, &["init", "--quiet"])?;
    git(dir, &["add", "--all"])?;
    git(dir, &["commit", "--quiet", "-m", "cooee config"])?;
    cooee_core::say!(
        "{}: a git repo with one commit; cooee never commits or pushes again",
        dir.display()
    );
    Ok(())
}

fn git(dir: &Path, args: &[&str]) -> Result<(), String> {
    let out = cooee_core::runner::command("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    Err(format!(
        "git {}: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr).trim()
    ))
}
