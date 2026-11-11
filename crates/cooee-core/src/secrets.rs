//! Secrets (#167): `secret:<NAME>` values. On macOS a keychain item (service `cooee-secret`) whose access list names
//! the signed cooee that saved it, read in-process with prompts off and every call time-limited; on Linux
//! `<data>/secrets/<NAME>` (0600). A file left on a Mac is moved into the keychain by a signed cooee, then shredded.

use std::{
    fs,
    io::{self, Write as _},
    path::{Path, PathBuf},
    sync::mpsc::{self, RecvTimeoutError},
    thread,
    time::Duration,
};

/// Past this a keychain call is an error: a locked keychain can hang behind a prompt no one sees (#713).
pub const WAIT: Duration = Duration::from_secs(5);

/// The code-signing identity every install signs cooee with (herdsman#781).
pub const IDENTITY: &str = "herdsman-local";

/// Secret `name`'s file: where Linux keeps it, and where a Mac's waits to be moved.
fn file(data: &Path, name: &str) -> PathBuf {
    data.join("secrets").join(name)
}

/// The keychain account prefix of data folder `data`, so two instances never share a secret.
fn prefix(data: &Path) -> String {
    let hash = crate::sha256(data.as_os_str().as_encoded_bytes());
    format!("{}-", &hash[..16])
}

/// Runs `f` on a thread of its own and waits `wait` at most: past it the call is a [`io::ErrorKind::TimedOut`].
///
/// # Errors
/// `f`'s, or the time-out.
pub fn limited<T: Send + 'static>(
    wait: Duration,
    f: impl FnOnce() -> io::Result<T> + Send + 'static,
) -> io::Result<T> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || tx.send(f()));
    match rx.recv_timeout(wait) {
        Ok(r) => r,
        Err(RecvTimeoutError::Timeout) => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            format!(
                "the keychain didn't answer in {} s: unlock it",
                wait.as_secs()
            ),
        )),
        Err(RecvTimeoutError::Disconnected) => Err(io::Error::other("the keychain call failed")),
    }
}

/// Secret `name`: the keychain's (macOS), else its file (its last newline dropped).
///
/// # Errors
/// When the keychain refuses this cooee (an unsigned build), doesn't answer, or the file can't be read.
pub fn get(data: &Path, name: &str) -> io::Result<Option<String>> {
    #[cfg(target_os = "macos")]
    if let Some(v) = keychain::get(prefix(data) + name)? {
        return Ok(Some(v));
    }
    match fs::read_to_string(file(data, name)) {
        Ok(v) => Ok(Some(v.strip_suffix('\n').unwrap_or(&v).to_owned())),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Saves secret `name`: in the keychain on macOS (a signed cooee only), else in its file (0600).
///
/// # Errors
/// An unsigned cooee on macOS, or a store that can't be written.
pub fn set(data: &Path, name: &str, value: &str) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    return keychain::set(prefix(data) + name, value.to_owned());
    #[cfg(not(target_os = "macos"))]
    crate::write_private(&file(data, name), value.as_bytes())
}

/// The names of the secrets saved for `data`.
///
/// # Errors
/// When the keychain or the folder can't be listed.
pub fn names(data: &Path) -> io::Result<Vec<String>> {
    #[cfg(target_os = "macos")]
    return keychain::accounts(prefix(data)).map(|a| {
        let p = prefix(data);
        a.iter()
            .filter_map(|a| a.strip_prefix(&p))
            .map(str::to_owned)
            .collect()
    });
    #[cfg(not(target_os = "macos"))]
    files(data)
}

/// The secret files in `data` (`.`-names are a launch's hand-off, not secrets).
fn files(data: &Path) -> io::Result<Vec<String>> {
    let dir = match fs::read_dir(data.join("secrets")) {
        Ok(d) => d,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut names = Vec::new();
    for e in dir {
        let e = e?;
        let name = e.file_name().to_string_lossy().into_owned();
        if !name.starts_with('.') && e.file_type()?.is_file() {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

/// Moves every secret file of `data` into the keychain (macOS), each read back before its file is shredded: the
/// names moved. Linux keeps its files.
///
/// # Errors
/// An unsigned cooee with files to move, or a secret the keychain didn't take.
pub fn migrate(data: &Path) -> io::Result<Vec<String>> {
    if !cfg!(target_os = "macos") {
        return Ok(Vec::new());
    }
    let names = files(data)?;
    for name in &names {
        let path = file(data, name);
        let v = fs::read_to_string(&path)?;
        let v = v.strip_suffix('\n').unwrap_or(&v);
        set(data, name, v)?;
        if get(data, name)?.as_deref() != Some(v) {
            return Err(io::Error::other(format!("{name}: the keychain lost it")));
        }
        shred(&path)?;
    }
    Ok(names)
}

/// Overwrites `path` with zeros, then removes it.
fn shred(path: &Path) -> io::Result<()> {
    let len = fs::metadata(path)?.len();
    let mut f = fs::OpenOptions::new().write(true).open(path)?;
    f.write_all(&vec![0; usize::try_from(len).unwrap_or(0)])?;
    f.sync_all()?;
    fs::remove_file(path)
}

#[cfg(target_os = "macos")]
mod keychain {
    use std::{io, sync::OnceLock};

    use security_framework::{
        base::Error,
        item::{ItemClass, ItemSearchOptions, Limit},
        os::macos::{
            code_signing::{Flags, SecCode, SecRequirement},
            keychain::SecKeychain,
            passwords::find_generic_password,
        },
    };

    use super::{IDENTITY, WAIT, limited};

    const SERVICE: &str = "cooee-secret";
    const NOT_FOUND: i32 = -25300;

    fn error(e: &Error) -> io::Error {
        io::Error::other(format!(
            "the keychain refused cooee ({e}): install a signed cooee (scripts/install.sh)"
        ))
    }

    /// Turns keychain prompts off for good: a prompt no one sees is an error, not a wait.
    fn quiet() {
        static OFF: OnceLock<()> = OnceLock::new();
        OFF.get_or_init(|| std::mem::forget(SecKeychain::disable_user_interaction()));
    }

    /// Whether this cooee is signed with [`IDENTITY`]: an item it saves opens to its next build.
    pub fn signed() -> bool {
        let req = format!("identifier \"cooee\" and certificate leaf[subject.CN] = \"{IDENTITY}\"");
        let req = req.parse::<SecRequirement>();
        req.and_then(|r| SecCode::for_self(Flags::NONE)?.check_validity(Flags::NONE, &r))
            .is_ok()
    }

    pub fn get(account: String) -> io::Result<Option<String>> {
        limited(WAIT, move || {
            quiet();
            match find_generic_password(None, SERVICE, &account) {
                Ok((v, _)) => Ok(Some(String::from_utf8_lossy(&v).into_owned())),
                Err(e) if e.code() == NOT_FOUND => Ok(None),
                Err(e) => Err(error(&e)),
            }
        })
    }

    pub fn set(account: String, value: String) -> io::Result<()> {
        if !signed() {
            return Err(io::Error::other(format!(
                "this cooee isn't signed with {IDENTITY}, so its next build couldn't read the secret: \
                 install it with scripts/install.sh"
            )));
        }
        limited(WAIT, move || {
            quiet();
            let k = SecKeychain::default().map_err(|e| error(&e))?;
            k.set_generic_password(SERVICE, &account, value.as_bytes())
                .map_err(|e| error(&e))
        })
    }

    pub fn accounts(prefix: String) -> io::Result<Vec<String>> {
        limited(WAIT, move || {
            quiet();
            let found = ItemSearchOptions::new()
                .class(ItemClass::generic_password())
                .service(SERVICE)
                .load_attributes(true)
                .limit(Limit::All)
                .search();
            let found = match found {
                Err(e) if e.code() == NOT_FOUND => return Ok(Vec::new()),
                f => f.map_err(|e| error(&e))?,
            };
            let mut all: Vec<String> = found
                .iter()
                .filter_map(|r| r.simplify_dict()?.remove("acct"))
                .filter(|a| a.starts_with(&prefix))
                .collect();
            all.sort();
            Ok(all)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cooee-secrets-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("secrets")).unwrap();
        d
    }

    /// #713: a keychain call that never returns is a visible error once its time is up, never a hang.
    #[test]
    fn a_keychain_call_that_hangs_is_a_time_out_error() {
        let (_hold, gate) = mpsc::channel::<()>();
        let r = limited(Duration::from_millis(20), move || {
            let _ = gate.recv();
            Ok(())
        });
        let e = r.unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::TimedOut);
        assert!(e.to_string().contains("didn't answer"), "{e}");
    }

    /// On Linux the 0600 file is the store: saved there, read from there, listed, and never moved.
    #[cfg(not(target_os = "macos"))]
    #[test]
    fn linux_keeps_secrets_in_their_0600_file() {
        use std::os::unix::fs::PermissionsExt;
        let data = scratch("linux");
        set(&data, "TOKEN", "s3cret").unwrap();
        let path = data.join("secrets/TOKEN");
        assert_eq!(fs::read_to_string(&path).unwrap(), "s3cret");
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(get(&data, "TOKEN").unwrap().as_deref(), Some("s3cret"));
        fs::write(data.join("secrets/.run-x"), "handoff").unwrap();
        assert_eq!(names(&data).unwrap(), ["TOKEN"]);
        assert!(migrate(&data).unwrap().is_empty());
        assert!(path.exists());
    }

    /// An unsigned cooee (every test build) never puts a secret in the keychain, where its next build couldn't
    /// read it: the file stays and the refusal says why.
    #[cfg(target_os = "macos")]
    #[test]
    fn an_unsigned_cooee_leaves_the_file_and_says_why() {
        let data = scratch("unsigned");
        fs::write(data.join("secrets/TOKEN"), "s3cret\n").unwrap();
        let e = migrate(&data).unwrap_err();
        assert!(e.to_string().contains("isn't signed"), "{e}");
        assert_eq!(
            fs::read_to_string(data.join("secrets/TOKEN")).unwrap(),
            "s3cret\n"
        );
        assert_eq!(get(&data, "TOKEN").unwrap().as_deref(), Some("s3cret"));
    }
}
