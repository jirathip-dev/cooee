//! The data folder's logs (#363): `cooee.log`, `terminal.log` and `perf.log` rotate at `[logs] max_mb`, keeping
//! `[logs] keep` old files (`<log>.1` the newest).

use std::{
    fs::{self, OpenOptions},
    io,
    path::{Path, PathBuf},
};

use serde::Deserialize;

/// The logs in the data folder that rotate.
pub const FILES: [&str; 3] = ["cooee.log", "terminal.log", crate::perf::FILE];

/// `[logs]`: when a log rotates and how many old ones stay.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Logs {
    /// A log bigger than this many MB rotates (default 16).
    pub max_mb: u64,
    /// The old files kept (default 3).
    pub keep: u32,
}

impl Default for Logs {
    fn default() -> Self {
        Self {
            max_mb: 16,
            keep: 3,
        }
    }
}

fn old(path: &Path, n: u32) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(format!(".{n}"));
    PathBuf::from(name)
}

impl Logs {
    /// Rotates `path` once it is bigger than `max_mb`: `<path>.1` … `<path>.<keep>` move up one, the oldest goes,
    /// and the log is copied to `<path>.1` and emptied in place, so a writer holding it open (launchd's stdout,
    /// the perf log) keeps writing to `path`.
    ///
    /// # Errors
    /// When a file can't be moved, copied or emptied.
    pub fn rotate(&self, path: &Path) -> io::Result<()> {
        if !fs::metadata(path).is_ok_and(|m| m.len() > self.max_mb << 20) {
            return Ok(());
        }
        if self.keep > 0 {
            match fs::remove_file(old(path, self.keep)) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
                _ => {}
            }
            for n in (1..self.keep).rev() {
                match fs::rename(old(path, n), old(path, n + 1)) {
                    Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
                    _ => {}
                }
            }
            fs::copy(path, old(path, 1))?;
        }
        OpenOptions::new().write(true).open(path)?.set_len(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_log_past_max_mb_keeps_exactly_keep_old_files() {
        let dir = std::env::temp_dir().join(format!("cooee-logs-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let log = dir.join("cooee.log");
        let logs = Logs { max_mb: 1, keep: 3 };
        let mut held = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log)
            .unwrap();
        for round in 0..6u8 {
            io::Write::write_all(&mut held, &vec![b'a' + round; (1 << 20) + 1]).unwrap();
            logs.rotate(&log).unwrap();
        }
        // Under max_mb: untouched.
        io::Write::write_all(&mut held, b"after").unwrap();
        logs.rotate(&log).unwrap();
        let mut names: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        assert_eq!(
            names,
            ["cooee.log", "cooee.log.1", "cooee.log.2", "cooee.log.3"]
        );
        // The newest old file is .1; the writer holding the log open still writes to it.
        assert_eq!(fs::read(old(&log, 1)).unwrap()[0], b'f');
        assert_eq!(fs::read(old(&log, 3)).unwrap()[0], b'd');
        assert_eq!(fs::read(&log).unwrap(), b"after");
        let _ = fs::remove_dir_all(&dir);
    }
}
