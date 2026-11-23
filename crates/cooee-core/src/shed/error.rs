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
    /// shed is not shed, or speaks a protocol older than [`SHED_MIN_PROTOCOL`].
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

/// The oldest shed protocol cooee accepts (5: a run on a named account); any newer one is accepted too.
pub const SHED_MIN_PROTOCOL: u32 = 5;

/// What to do when shed is missing or too old: cooee never installs, updates or restarts shed itself.
#[must_use]
pub fn shed_fix() -> String {
    format!(
        "install or update shed to protocol {SHED_MIN_PROTOCOL} or newer from https://github.com/jirathip-dev/shed/releases, \
         and start its serve"
    )
}
