use std::{io, path::Path, process::Stdio, time::Duration};

use crate::{devices::pair, doctor::doctor};

use super::shed::shed;

/// `cooee setup`'s flags.
pub struct Setup {
    pub instance: String,
    pub port: Option<u16>,
    pub explicit: bool,
    pub dry_run: bool,
    pub yes: bool,
    pub chief_agent: Option<String>,
    pub chief_machine: Option<String>,
}

pub(crate) fn ok(argv: &[&str]) -> bool {
    let Some((prog, rest)) = argv.split_first() else {
        return false;
    };
    cooee_core::runner::command(prog)
        .args(rest)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

pub(crate) fn output(argv: &[&str]) -> Option<String> {
    let (prog, rest) = argv.split_first()?;
    let out = cooee_core::runner::command(prog).args(rest).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

pub(crate) fn wait(f: impl Fn() -> bool) -> bool {
    (0..40).any(|_| {
        let done = f();
        if !done {
            cooee_core::clock::sleep(Duration::from_millis(250));
        }
        done
    })
}

impl Setup {
    pub(super) fn exec(&self, argv: &[&str]) -> Result<(), String> {
        let line = argv.join(" ");
        if self.dry_run {
            cooee_core::say!("  would run: {line}");
            return Ok(());
        }
        cooee_core::say!("  $ {line}");
        let (prog, rest) = argv.split_first().ok_or("nothing to run")?;
        let status = cooee_core::runner::command(prog).args(rest).status();
        match status {
            Ok(s) if s.success() => Ok(()),
            Ok(s) => Err(format!("`{line}` failed ({s})")),
            Err(e) => Err(format!("`{line}`: {e}")),
        }
    }

    pub(super) fn write(
        &self,
        file: &Path,
        text: &str,
        f: impl FnOnce() -> Result<(), String>,
    ) -> Result<(), String> {
        if self.dry_run {
            cooee_core::say!("  would write {}:\n{text}", file.display());
            return Ok(());
        }
        f()?;
        cooee_core::say!("  wrote {}", file.display());
        Ok(())
    }

    pub(super) fn ask(&self, question: &str, default: &str) -> String {
        if self.yes {
            return default.to_owned();
        }
        cooee_core::say!("  {question}");
        let mut line = String::new();
        io::stdin().read_line(&mut line).ok();
        let line = line.trim();
        (if line.is_empty() { default } else { line }).to_owned()
    }

    /// Every step, in order, then the pairing code.
    pub fn run(&self, path: &Path) -> Result<(), String> {
        self.chief_flags(path)?;
        shed()?;
        let (config, changed) = self.config(path)?;
        self.themes(&config)?;
        self.agents(&config)?;
        self.chief(&config, path)?;
        self.service(&config, path, changed, "serve")?;
        self.service(&config, path, false, "update")?;
        self.tailnet(config.port)?;
        if self.dry_run {
            cooee_core::say!("would run: cooee doctor, then cooee pair");
            return Ok(());
        }
        cooee_core::say!("doctor:");
        doctor(&config)
            .map_err(|e| format!("doctor: {e}\nfix it, then run `cooee setup` again"))?;
        cooee_core::say!("\npair your phone:");
        pair(&config)
    }
}

pub(super) fn step(name: &str, done: bool) -> bool {
    cooee_core::say!("{name}:{}", if done { " done, skipped" } else { "" });
    done
}
