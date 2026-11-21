//! Desks (docs/decisions/0003-desks.md, #169): Linux desktops a persona drives, run by Apple's `container` CLI. Each
//! desk is capped in CPUs and memory and joins only cooee's host-only network, never the default NAT one; waking a
//! desk probes the addresses it must not reach (the LAN router, tailnet machines) and stops it if one answers.

#![forbid(unsafe_code)]

use std::{net::SocketAddr, path::PathBuf, process::Output};

use serde_json::Value;

/// The host-only network every desk joins.
pub const NETWORK: &str = "cooee-desks";
/// The most CPUs a desk may have.
pub const MAX_CPUS: u8 = 4;
/// The least memory a desk may have, in MiB.
pub const MIN_MEMORY_MIB: u32 = 1024;
/// The most memory a desk may have, in MiB.
pub const MAX_MEMORY_MIB: u32 = 8192;

/// Why a desk step failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The name is not 1 to 40 of `a-z`, `0-9` and `-`, starting with a letter.
    #[error("desk name {0:?}: 1 to 40 of a-z, 0-9 and -, starting with a letter")]
    Name(String),
    /// The caps are outside the allowed range.
    #[error(
        "desk caps: 1 to {MAX_CPUS} CPUs and {MIN_MEMORY_MIB} to {MAX_MEMORY_MIB} MiB of memory"
    )]
    Caps,
    /// A network named [`NETWORK`] exists but is not host-only.
    #[error("network {NETWORK} exists but is not host-only")]
    NotHostOnly,
    /// The desk reached an address it must not, so it was stopped.
    #[error("desk {desk} reached {target}, so it was stopped")]
    Reached {
        /// The desk.
        desk: String,
        /// What it reached.
        target: SocketAddr,
    },
    /// The `container` CLI failed.
    #[error("container {args}: {why}")]
    Cli {
        /// The arguments it was given.
        args: String,
        /// What it said.
        why: String,
    },
}

/// A desk's name, safe to pass to the CLI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name(String);

impl Name {
    /// `name` if it is 1 to 40 of `a-z`, `0-9` and `-`, starting with a letter.
    ///
    /// # Errors
    /// [`Error::Name`] otherwise.
    pub fn new(name: &str) -> Result<Self, Error> {
        let ok = name.len() <= 40
            && name.starts_with(|c: char| c.is_ascii_lowercase())
            && name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        ok.then(|| Self(name.to_owned()))
            .ok_or_else(|| Error::Name(name.to_owned()))
    }
}

/// A desk's resource caps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Caps {
    cpus: u8,
    memory_mib: u32,
}

impl Caps {
    /// `cpus` CPUs and `memory_mib` MiB, within [`MAX_CPUS`] and [`MIN_MEMORY_MIB`]..=[`MAX_MEMORY_MIB`].
    ///
    /// # Errors
    /// [`Error::Caps`] outside those.
    pub fn new(cpus: u8, memory_mib: u32) -> Result<Self, Error> {
        let ok = (1..=MAX_CPUS).contains(&cpus)
            && (MIN_MEMORY_MIB..=MAX_MEMORY_MIB).contains(&memory_mib);
        ok.then_some(Self { cpus, memory_mib }).ok_or(Error::Caps)
    }
}

/// Where a desk is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Not created, or destroyed.
    Gone,
    /// Created and asleep.
    Asleep,
    /// Awake.
    Awake,
}

/// The desk provider: the `container` CLI and the addresses no desk may reach.
#[derive(Debug, Clone)]
pub struct Desks {
    container: PathBuf,
    blocked: Vec<SocketAddr>,
}

impl Desks {
    /// Desks run by the `container` CLI at `container`; `blocked` is probed on every wake.
    #[must_use]
    pub fn new(container: PathBuf, blocked: Vec<SocketAddr>) -> Self {
        Self { container, blocked }
    }

    /// Creates `name` from `image` with `caps` on the host-only network, asleep; an existing desk is kept.
    ///
    /// # Errors
    /// [`Error::NotHostOnly`] when the network is not host-only, [`Error::Cli`] when the CLI fails.
    pub fn create(&self, name: &Name, image: &str, caps: Caps) -> Result<(), Error> {
        self.network()?;
        if self.state(name)? != State::Gone {
            return Ok(());
        }
        let (cpus, memory) = (caps.cpus.to_string(), format!("{}M", caps.memory_mib));
        let args = [
            "create",
            "--name",
            &name.0,
            "--cpus",
            &cpus,
            "--memory",
            &memory,
            "--network",
            NETWORK,
            image,
        ];
        self.run(&args).map(drop)
    }

    /// Wakes `name`, then stops it again if it reaches a blocked address.
    ///
    /// # Errors
    /// [`Error::Reached`] when it reached one, [`Error::Cli`] when the CLI fails.
    pub fn wake(&self, name: &Name) -> Result<(), Error> {
        self.run(&["start", &name.0])?;
        for target in &self.blocked {
            let probe = format!("exec 3<>/dev/tcp/{}/{}", target.ip(), target.port());
            let reached = self
                .cli(&["exec", &name.0, "timeout", "4", "bash", "-c", &probe])?
                .status
                .success();
            if reached {
                self.sleep(name)?;
                return Err(Error::Reached {
                    desk: name.0.clone(),
                    target: *target,
                });
            }
        }
        Ok(())
    }

    /// Puts `name` to sleep; an asleep desk stays asleep.
    ///
    /// # Errors
    /// [`Error::Cli`] when the CLI fails.
    pub fn sleep(&self, name: &Name) -> Result<(), Error> {
        if self.state(name)? == State::Awake {
            self.run(&["stop", &name.0])?;
        }
        Ok(())
    }

    /// Deletes `name` and its disk; a gone desk stays gone.
    ///
    /// # Errors
    /// [`Error::Cli`] when the CLI fails.
    pub fn destroy(&self, name: &Name) -> Result<(), Error> {
        if self.state(name)? != State::Gone {
            self.run(&["delete", "--force", &name.0])?;
        }
        Ok(())
    }

    /// Where `name` is in its life, as the CLI lists it.
    ///
    /// # Errors
    /// [`Error::Cli`] when the CLI fails or its list is not JSON.
    pub fn state(&self, name: &Name) -> Result<State, Error> {
        let list = self.list(&["ls", "--all", "--format", "json"])?;
        let desk = list
            .iter()
            .find(|c| c.pointer("/configuration/id") == Some(&Value::from(name.0.as_str())));
        Ok(match desk.map(|c| c.get("status")) {
            None => State::Gone,
            Some(s) if s.and_then(Value::as_str) == Some("running") => State::Awake,
            Some(_) => State::Asleep,
        })
    }

    fn network(&self) -> Result<(), Error> {
        let list = self.list(&["network", "ls", "--format", "json"])?;
        match list
            .iter()
            .find(|n| n.get("id").and_then(Value::as_str) == Some(NETWORK))
        {
            None => self
                .run(&["network", "create", "--internal", NETWORK])
                .map(drop),
            Some(n)
                if n.pointer("/configuration/mode").and_then(Value::as_str) == Some("hostOnly") =>
            {
                Ok(())
            }
            Some(_) => Err(Error::NotHostOnly),
        }
    }

    fn list(&self, args: &[&str]) -> Result<Vec<Value>, Error> {
        let out = self.run(args)?;
        serde_json::from_slice(&out.stdout).map_err(|e| cli_error(args, &e.to_string()))
    }

    fn run(&self, args: &[&str]) -> Result<Output, Error> {
        let out = self.cli(args)?;
        if out.status.success() {
            Ok(out)
        } else {
            Err(cli_error(args, String::from_utf8_lossy(&out.stderr).trim()))
        }
    }

    fn cli(&self, args: &[&str]) -> Result<Output, Error> {
        cooee_core::runner::command(&self.container)
            .args(args)
            .output()
            .map_err(|e| cli_error(args, &e.to_string()))
    }
}

fn cli_error(args: &[&str], why: &str) -> Error {
    Error::Cli {
        args: args.join(" "),
        why: why.to_owned(),
    }
}
