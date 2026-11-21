use std::{
    io,
    path::{Path, PathBuf},
};

use super::FORMAT;

/// Why a store operation failed.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// A file or folder couldn't be read or written.
    #[error("{path}: {source}")]
    Io {
        /// The file or folder.
        path: PathBuf,
        /// The underlying error.
        source: io::Error,
    },
    /// A thread file isn't a valid thread.
    #[error("{path}: {source}")]
    Parse {
        /// The thread file.
        path: PathBuf,
        /// The underlying error.
        source: serde_json::Error,
    },
    /// A thread file was written by a newer cooee.
    #[error("{path}: format {found} is newer than this cooee's {FORMAT}; upgrade cooee")]
    Newer {
        /// The thread file.
        path: PathBuf,
        /// Its format version.
        found: u32,
    },
    /// The items database failed.
    #[error("{path}: {source}")]
    Db {
        /// The database file.
        path: PathBuf,
        /// The underlying error.
        source: rusqlite::Error,
    },
    /// A thread id that can't name a file.
    #[error("invalid thread id {0:?}: use 1 to 64 of A-Z a-z 0-9 - _")]
    InvalidId(String),
}

pub(super) fn db_err(path: &Path) -> impl FnOnce(rusqlite::Error) -> StoreError + '_ {
    move |source| StoreError::Db {
        path: path.to_owned(),
        source,
    }
}

pub(super) fn io_err(path: &Path) -> impl FnOnce(io::Error) -> StoreError + '_ {
    move |source| StoreError::Io {
        path: path.to_owned(),
        source,
    }
}
