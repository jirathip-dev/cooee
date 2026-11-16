//! Tests' temp folders and files (#202): each one a test names here is removed when its thread ends, pass or fail.

use std::{cell::RefCell, fs, path::PathBuf};

struct Owned(RefCell<Vec<PathBuf>>);

impl Drop for Owned {
    fn drop(&mut self) {
        for p in self.0.get_mut().drain(..) {
            fs::remove_dir_all(&p).or_else(|_| fs::remove_file(&p)).ok();
        }
    }
}

thread_local!(static OWNED: Owned = const { Owned(RefCell::new(Vec::new())) });

/// `path`, which this thread removes (folder or file) when it ends; libtest runs each test on a thread of its own.
#[must_use]
pub fn owned(path: PathBuf) -> PathBuf {
    OWNED.with(|o| o.0.borrow_mut().push(path.clone()));
    path
}
