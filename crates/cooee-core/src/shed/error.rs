use std::{io, path::PathBuf};

use shed_proto::{ErrorCode, RpcError};

/// Why a call to shed failed.
#[derive(Debug, thiserror::Error)]
pub enum ShedError {
    /// The socket couldn't be reached, or the connection broke.
    #[error("shed at {path}: {source}")]
    Io {
        /// The socket.
        path: PathBuf,
        /// The underlying error.
        source: io::Error,
    },
    /// shed speaks another protocol than the `shed-proto` cooee was built with.
    #[error(
        "SHED_VERSION: cooee speaks shed {ours}; {theirs}; fix: {}",
        shed_fix()
    )]
    Version {
        /// This build's shed version, build and protocol.
        ours: String,
        /// The server's refusal, or the version, build and protocol it runs.
        theirs: String,
    },
    /// shed answered with an error.
    #[error("shed: {0}")]
    Rpc(RpcError),
    /// shed's answer couldn't be read.
    #[error("shed: bad answer: {0}")]
    Bad(String),
    /// The `shed` CLI failed.
    #[error("`shed {args}`: {detail}")]
    Cli {
        /// The arguments.
        args: String,
        /// Its stderr, or why it couldn't start.
        detail: String,
    },
}

impl ShedError {
    /// Whether shed refused because the terminal doesn't exist.
    #[must_use]
    pub fn no_such_terminal(&self) -> bool {
        matches!(self, Self::Rpc(e) if e.code == ErrorCode::NoSuchTerminal.code())
    }

    /// Whether shed refused because the terminal's machine has no link up (#149).
    #[must_use]
    pub fn no_link(&self) -> bool {
        matches!(self, Self::Rpc(e) if e.code == ErrorCode::NoLink.code())
    }
}

/// shed's install command: the rev `shed-proto` is pinned to in Cargo.toml, since the hello needs the same protocol.
pub const SHED_CARGO: [&str; 8] = [
    "cargo",
    "install",
    "--locked",
    "--git",
    "https://github.com/jirathip-dev/shed",
    "--rev",
    "4ac443dbbcb51227dc9df2fb55e2f0834d6a91eb",
    "shed",
];

/// shed's launchd job on macOS (`shed install`).
pub const SHED_LABEL: &str = "dev.shed.serve";

/// The command that installs the shed this build speaks and restarts its serve onto it.
#[must_use]
pub fn shed_fix() -> String {
    let restart = if cfg!(target_os = "macos") {
        format!("launchctl kickstart -k gui/$(id -u)/{SHED_LABEL}")
    } else {
        "shed serve".to_owned()
    };
    format!("{} && {restart}", SHED_CARGO.join(" "))
}
