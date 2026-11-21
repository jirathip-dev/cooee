//! `<config>/.gitignore` (#361): what must stay out of git when the owner versions the config folder.

use std::{fs, io, path::Path};

const TEXT: &str = "\
# Written by cooee once (#361); it never rewrites this file, so edit it freely.
# Secrets, keys and tokens:
secrets/
accounts/
*.key
*.p8
*.pem
.cooee.json
devices.json
push.json
.env
# Caches, locks and half-written files:
.themes-stage/
themes-dev/
agents.migrating/
themes/*.git/
*.lock
*.tmp
*.tmp-*
*.new-*
*.log
*.log.old
.DS_Store
";

/// Writes `dir/.gitignore` unless one is there; `true` when it wrote it.
///
/// # Errors
/// When the file can't be created or written.
pub fn write(dir: &Path) -> io::Result<bool> {
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir.join(".gitignore"));
    match file {
        Ok(mut f) => io::Write::write_all(&mut f, TEXT.as_bytes()).map(|()| true),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(false),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_once_and_keeps_the_owners_file() {
        let dir = crate::scratch::owned(std::env::temp_dir().join("cooee-gitignore-unit"));
        fs::remove_dir_all(&dir).ok();
        fs::create_dir_all(&dir).unwrap();
        assert!(write(&dir).unwrap());
        fs::write(dir.join(".gitignore"), "mine\n").unwrap();
        assert!(!write(&dir).unwrap());
        assert_eq!(
            fs::read_to_string(dir.join(".gitignore")).unwrap(),
            "mine\n"
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
