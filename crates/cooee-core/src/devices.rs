//! Paired devices (herdsman's `devices.rs`): `cooee pair` opens a six-digit code for 10 minutes; the browser that
//! enters it gets a random token as its cookie. Only SHA-256 hashes of codes and tokens are kept, in
//! `<data>/devices.json` (mode 0600). Five wrong codes drop the open one. The other way round (#47): a new device asks
//! to pair and shows a check word; a paired device approves it (with its passkey) or declines; the new device then
//! collects its token with the claim it was given. Passkeys (their public keys) are kept here too.

use std::{
    fmt::Write as _,
    fs,
    io::{self, Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    sync::{
        Mutex, MutexGuard, PoisonError,
        atomic::{AtomicU64, Ordering},
    },
    time::SystemTime,
};

use serde::{Deserialize, Serialize};

/// How long a pairing code is good for, in milliseconds.
const CODE_TTL: u64 = 10 * 60 * 1000;

/// Wrong codes before the open one is dropped.
const MISSES: u32 = 5;

/// One paired browser.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    /// The first 8 hex digits of its token's hash (an approved request's: its claim's): what `cooee devices rm` takes.
    pub id: String,
    /// What it is (`iPhone`, `Android`, `Mac`, …), from its user agent.
    pub label: String,
    /// SHA-256 of its token, hex.
    pub hash: String,
    /// When it paired, in Unix milliseconds.
    pub created: u64,
    /// When it was last seen (to the hour), in Unix milliseconds.
    pub used: u64,
    /// The device that approved its request, if one did.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub by: String,
    /// Its passkey (Face ID on iPhone), registered on first use of a tab that needs one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passkey: Option<Passkey>,
}

/// A device's passkey credential: its id and its P-256 public key, both base64url.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Passkey {
    /// The credential id.
    pub id: String,
    /// The public key, as an uncompressed SEC1 point.
    pub key: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
struct File {
    devices: Vec<Device>,
    /// The open code's hash and when it was made.
    code: Option<(String, u64)>,
    misses: u32,
    asks: Vec<Request>,
    passkeys: Vec<Credential>,
}

/// A new device's open request to pair: its claim's hash, the check word it shows, what it is, when it asked and the
/// answer (`None`: waiting; the answering device's label with yes or no).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Request {
    claim: String,
    word: String,
    label: String,
    at: u64,
    answer: Option<(bool, String)>,
}

/// Open requests kept at once; a newer one drops the oldest.
const REQUESTS: usize = 4;

/// How long a request waits for an answer, in milliseconds.
const ASK_TTL: u64 = 5 * 60 * 1000;

/// What a new device's claim finds: its request gone (expired, cancelled or never made), still waiting, declined (by
/// which device), or its token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Claim {
    /// No such request.
    Gone,
    /// Not answered yet.
    Waiting,
    /// Declined on this device.
    Declined(String),
    /// Approved: the new device's token.
    Paired(String),
}

/// An open request as paired devices see it: its id, check word, what asks and when.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Asking {
    /// The first 8 hex digits of its claim's hash.
    pub id: String,
    /// The check word the new device shows.
    pub word: String,
    /// What the new device is, from its user agent.
    pub label: String,
    /// When it asked, in Unix milliseconds.
    pub at: u64,
}

/// A passkey that signs a device back in and approves new ones (`WebAuthn`, ES256): its credential id, public key
/// and the device that made it. The terminal's unlock key is a device's own [`Passkey`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Credential {
    /// The credential id, base64url.
    pub id: String,
    /// The uncompressed P-256 public key, base64url.
    pub key: String,
    /// The label of the device that made it.
    pub label: String,
    /// The id of the device that made it (the one it signed back in, after a sign-in).
    #[serde(default)]
    pub device: String,
    /// When it was made, in Unix milliseconds.
    pub created: u64,
    /// When it was last used, in Unix milliseconds (0: never).
    pub used: u64,
}

/// Two words from a fixed list, so a check word is never mistaken for a code.
const WORDS: [[&str; 16]; 2] = [
    [
        "maple", "amber", "river", "cedar", "coral", "misty", "sunny", "quiet", "swift", "lucky",
        "olive", "frosty", "golden", "silver", "rusty", "velvet",
    ],
    [
        "otter", "heron", "panda", "lynx", "badger", "falcon", "gecko", "koala", "lemur", "moose",
        "owl", "puffin", "rabbit", "salmon", "tiger", "walrus",
    ],
];

/// A six-digit code.
fn six() -> io::Result<String> {
    let b = random(4)?;
    Ok(format!(
        "{:06}",
        u32::from_be_bytes([b[0], b[1], b[2], b[3]]) % 1_000_000
    ))
}

impl File {
    /// Drops the requests older than [`ASK_TTL`].
    fn expire(&mut self, now: u64) {
        self.asks.retain(|r| r.at + ASK_TTL > now);
    }

    /// Adds a device with `token`, named `label` (its id: `id`, else from the token's hash); the device.
    fn add(&mut self, token: &str, id: Option<String>, label: &str, now: u64) -> &mut Device {
        let hash = sha256(token.as_bytes());
        self.devices.push(Device {
            id: id.unwrap_or_else(|| hash[..8].to_owned()),
            label: label.to_owned(),
            hash,
            created: now,
            used: now,
            by: String::new(),
            passkey: None,
        });
        self.devices.last_mut().expect("just pushed")
    }
}

/// The paired devices of one data folder, parsed again only when the file changes (its mtime, size or inode).
pub struct Devices {
    path: PathBuf,
    cache: Mutex<Option<(Stamp, File)>>,
}

/// What tells one version of the devices file from another: its mtime, size and inode (a save renames a new file
/// in).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Stamp(SystemTime, u64, u64);

impl Stamp {
    fn of(path: &Path) -> io::Result<Self> {
        let m = fs::metadata(path)?;
        Ok(Self(m.modified()?, m.len(), m.ino()))
    }
}

/// SHA-256 of `data`, lowercase hex.
#[must_use]
pub fn sha256(data: &[u8]) -> String {
    use sha2::Digest;
    hex(&sha2::Sha256::digest(data))
}

/// Lowercase hex.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// `n` bytes from `/dev/urandom`.
///
/// # Errors
/// When `/dev/urandom` can't be read.
pub fn random(n: usize) -> io::Result<Vec<u8>> {
    let mut b = vec![0; n];
    fs::File::open("/dev/urandom")?.read_exact(&mut b)?;
    Ok(b)
}

/// Appends `line` and a newline to `path` (made 0600 if new).
///
/// # Errors
/// When the file can't be opened or written.
pub fn append_line(path: &Path, line: &str) -> io::Result<()> {
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(path)?
        .write_all(format!("{line}\n").as_bytes())
}

/// Replaces `path` with `bytes`, mode 0600: a temp file of its own (named by process and write, so concurrent
/// writers never share one) fsynced and renamed over it.
///
/// # Errors
/// When the folder or the file can't be written.
pub fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    static WRITES: AtomicU64 = AtomicU64::new(0);
    let dir = path.parent().unwrap_or(path);
    fs::create_dir_all(dir)?;
    let n = WRITES.fetch_add(1, Ordering::Relaxed);
    let tmp = path.with_extension(format!("tmp-{}-{n}", std::process::id()));
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&tmp)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    fs::rename(&tmp, path)?;
    fs::File::open(dir)?.sync_all()
}

/// Constant-time equality.
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0, |d, (x, y)| d | (x ^ y)) == 0
}

/// A short name for the browser behind `agent`.
#[must_use]
pub fn label(agent: &str) -> &'static str {
    [
        ("iPhone", "iPhone"),
        ("iPad", "iPad"),
        ("Android", "Android"),
        ("Macintosh", "Mac"),
        ("Windows", "Windows"),
        ("Linux", "Linux"),
    ]
    .into_iter()
    .find(|(k, _)| agent.contains(k))
    .map_or("browser", |(_, v)| v)
}

impl Devices {
    /// The devices kept in `data`.
    #[must_use]
    pub fn new(data: &Path) -> Self {
        Self {
            path: data.join("devices.json"),
            cache: Mutex::new(None),
        }
    }

    fn cached(&self) -> MutexGuard<'_, Option<(Stamp, File)>> {
        self.cache.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn load(&self) -> io::Result<File> {
        let stamp = match Stamp::of(&self.path) {
            Ok(s) => s,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(File::default()),
            Err(e) => return Err(e),
        };
        if let Some((_, f)) = self.cached().as_ref().filter(|(at, _)| *at == stamp) {
            return Ok(f.clone());
        }
        let file: File =
            serde_json::from_slice(&fs::read(&self.path)?).map_err(io::Error::other)?;
        *self.cached() = Some((stamp, file.clone()));
        Ok(file)
    }

    fn save(&self, f: &File) -> io::Result<()> {
        write_private(&self.path, &serde_json::to_vec_pretty(f)?)?;
        *self.cached() = Some((Stamp::of(&self.path)?, f.clone()));
        Ok(())
    }

    fn change<T>(&self, f: impl FnOnce(&mut File) -> io::Result<T>) -> io::Result<T> {
        fs::create_dir_all(self.path.parent().unwrap_or(&self.path))?;
        let lock = fs::File::create(self.path.with_extension("lock"))?;
        lock.lock()?;
        let mut file = self.load()?;
        let out = f(&mut file)?;
        self.save(&file)?;
        Ok(out)
    }

    /// Opens a new pairing code (replacing any open one), good for 10 minutes.
    ///
    /// # Errors
    /// When the file can't be read or written.
    pub fn pair(&self, now: u64) -> io::Result<String> {
        let code = six()?;
        self.change(|f| {
            f.code = Some((sha256(code.as_bytes()), now));
            f.misses = 0;
            Ok(())
        })?;
        Ok(code)
    }

    /// Pairs the browser that entered `code` (once): its new token, or `None` for a wrong or expired code.
    ///
    /// # Errors
    /// When the file can't be read or written.
    pub fn claim(&self, code: &str, agent: &str, now: u64) -> io::Result<Option<String>> {
        let token = hex(&random(32)?);
        self.change(|f| {
            let Some((hash, at)) = f.code.clone() else {
                return Ok(None);
            };
            if at + CODE_TTL <= now || !same(&hash, &sha256(code.trim().as_bytes())) {
                f.misses += 1;
                if f.misses >= MISSES || at + CODE_TTL <= now {
                    f.code = None;
                }
                return Ok(None);
            }
            f.code = None;
            f.add(&token, None, label(agent), now);
            Ok(Some(token.clone()))
        })
    }

    /// Whether a pairing code is open (made and not yet used, run out or dropped).
    ///
    /// # Errors
    /// When the file can't be read.
    pub fn code_open(&self, now: u64) -> io::Result<bool> {
        Ok(self.load()?.code.is_some_and(|(_, at)| at + CODE_TTL > now))
    }

    /// A new device's request to pair, good for 5 minutes: the check word it shows and the claim it collects its
    /// token with.
    ///
    /// # Errors
    /// When the file can't be read or written.
    pub fn ask(&self, agent: &str, now: u64) -> io::Result<(String, String)> {
        let (b, claim) = (random(2)?, hex(&random(32)?));
        let word = format!(
            "{} {}",
            WORDS[0][usize::from(b[0] % 16)],
            WORDS[1][usize::from(b[1] % 16)]
        );
        self.change(|f| {
            f.expire(now);
            if f.asks.len() >= REQUESTS {
                f.asks.remove(0);
            }
            f.asks.push(Request {
                claim: sha256(claim.as_bytes()),
                word: word.clone(),
                label: label(agent).to_owned(),
                at: now,
                answer: None,
            });
            Ok(())
        })?;
        Ok((word, claim))
    }

    /// The requests still waiting for an answer, oldest first.
    ///
    /// # Errors
    /// When the file can't be read.
    pub fn asking(&self, now: u64) -> io::Result<Vec<Asking>> {
        let mut f = self.load()?;
        f.expire(now);
        let open = f.asks.into_iter().filter(|r| r.answer.is_none());
        Ok(open
            .map(|r| Asking {
                id: r.claim[..8].to_owned(),
                word: r.word,
                label: r.label,
                at: r.at,
            })
            .collect())
    }

    /// Approves (`yes`) or declines the waiting request `id` on device `by`; whether there was one.
    ///
    /// # Errors
    /// When the file can't be read or written.
    pub fn answer(&self, id: &str, yes: bool, by: &str, now: u64) -> io::Result<bool> {
        self.change(|f| {
            f.expire(now);
            let found = f
                .asks
                .iter_mut()
                .find(|r| r.answer.is_none() && r.claim.starts_with(id) && id.len() == 8);
            Ok(found
                .map(|r| r.answer = Some((yes, by.to_owned())))
                .is_some())
        })
    }

    /// The new device's claim: its token once approved (once), else whether it waits or was declined.
    ///
    /// # Errors
    /// When the file can't be read or written.
    pub fn collect(&self, claim: &str, now: u64) -> io::Result<Claim> {
        let (hash, token) = (sha256(claim.as_bytes()), hex(&random(32)?));
        self.change(|f| {
            f.expire(now);
            let Some(i) = f.asks.iter().position(|r| same(&r.claim, &hash)) else {
                return Ok(Claim::Gone);
            };
            match f.asks[i].answer.clone() {
                None => Ok(Claim::Waiting),
                Some((false, by)) => {
                    f.asks.remove(i);
                    Ok(Claim::Declined(by))
                }
                Some((true, by)) => {
                    let r = f.asks.remove(i);
                    f.add(&token, Some(r.claim[..8].to_owned()), &r.label, now)
                        .by = by;
                    Ok(Claim::Paired(token.clone()))
                }
            }
        })
    }

    /// Drops the new device's request.
    ///
    /// # Errors
    /// When the file can't be read or written.
    pub fn cancel(&self, claim: &str) -> io::Result<()> {
        let hash = sha256(claim.as_bytes());
        self.change(|f| {
            f.asks.retain(|r| !same(&r.claim, &hash));
            Ok(())
        })
    }

    /// Every passkey, oldest first.
    ///
    /// # Errors
    /// When the file can't be read.
    pub fn passkeys(&self) -> io::Result<Vec<Credential>> {
        Ok(self.load()?.passkeys)
    }

    /// Keeps `key` (replacing one with its id).
    ///
    /// # Errors
    /// When the file can't be read or written.
    pub fn add_passkey(&self, key: Credential) -> io::Result<()> {
        self.change(|f| {
            f.passkeys.retain(|k| k.id != key.id);
            f.passkeys.push(key);
            Ok(())
        })
    }

    /// Marks passkey `id` used now.
    ///
    /// # Errors
    /// When the file can't be read or written.
    pub fn used_passkey(&self, id: &str, now: u64) -> io::Result<()> {
        self.change(|f| {
            for k in f.passkeys.iter_mut().filter(|k| k.id == id) {
                k.used = now;
            }
            Ok(())
        })
    }

    /// Removes passkey `id`; whether there was one.
    ///
    /// # Errors
    /// When the file can't be read or written.
    pub fn remove_passkey(&self, id: &str) -> io::Result<bool> {
        self.change(|f| {
            let n = f.passkeys.len();
            f.passkeys.retain(|k| k.id != id);
            Ok(f.passkeys.len() < n)
        })
    }

    /// Pairs the browser behind `agent` after a sign-in with `passkey`, which is then its: its new token.
    ///
    /// # Errors
    /// When the file can't be read or written.
    pub fn restore(&self, agent: &str, passkey: &str, now: u64) -> io::Result<String> {
        let token = hex(&random(32)?);
        self.change(|f| {
            let id = f.add(&token, None, label(agent), now).id.clone();
            for k in f.passkeys.iter_mut().filter(|k| k.id == passkey) {
                k.device.clone_from(&id);
            }
            Ok(token.clone())
        })
    }

    /// Renames device `id`, or the approved request `id` the device it becomes then takes; whether there was one.
    ///
    /// # Errors
    /// When the file can't be read or written.
    pub fn rename(&self, id: &str, to: &str) -> io::Result<bool> {
        self.change(|f| {
            let approved =
                |r: &&mut Request| matches!(r.answer, Some((true, _))) && r.claim[..8] == *id;
            let label = match f.devices.iter_mut().find(|d| d.id == id) {
                Some(d) => Some(&mut d.label),
                None => f.asks.iter_mut().find(approved).map(|r| &mut r.label),
            };
            Ok(label.map(|l| to.clone_into(l)).is_some())
        })
    }

    /// The paired device whose token this is (`None`: no such device); its `used` is updated at most hourly.
    ///
    /// # Errors
    /// When the file can't be read: that is not "unpaired".
    pub fn find(&self, token: &str, now: u64) -> io::Result<Option<Device>> {
        let hash = sha256(token.as_bytes());
        let found = self
            .load()?
            .devices
            .into_iter()
            .find(|d| same(&d.hash, &hash));
        let Some(device) = found else {
            return Ok(None);
        };
        if device.used + 3_600_000 <= now {
            let _ = self.change(|f| {
                for d in f.devices.iter_mut().filter(|d| d.hash == hash) {
                    d.used = now;
                }
                Ok(())
            });
        }
        Ok(Some(device))
    }

    /// Every paired device, oldest first.
    ///
    /// # Errors
    /// When the file can't be read.
    pub fn list(&self) -> io::Result<Vec<Device>> {
        Ok(self.load()?.devices)
    }

    /// Gives device `id` its passkey, unless it has one already; whether it was set.
    ///
    /// # Errors
    /// When the file can't be read or written.
    pub fn set_passkey(&self, id: &str, passkey: Passkey) -> io::Result<bool> {
        self.change(|f| {
            let found = f
                .devices
                .iter_mut()
                .find(|d| d.id == id && d.passkey.is_none());
            Ok(found.map(|d| d.passkey = Some(passkey)).is_some())
        })
    }

    /// Unpairs device `id`; whether there was one.
    ///
    /// # Errors
    /// When the file can't be read or written.
    pub fn remove(&self, id: &str) -> io::Result<bool> {
        self.change(|f| {
            let n = f.devices.len();
            f.devices.retain(|d| d.id != id);
            Ok(f.devices.len() < n)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::FileExt;

    use super::*;

    fn scratch(name: &str) -> Devices {
        let dir = std::env::temp_dir().join(format!("cooee-devices-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        Devices::new(&dir)
    }

    #[test]
    fn another_process_waits_while_a_change_is_in_progress() {
        let d = scratch("lock");
        d.pair(0).unwrap();
        let other = d.change(|_| {
            let lock = fs::File::create(d.path.with_extension("lock"))?;
            Ok(lock.try_lock())
        });
        assert!(
            matches!(other.unwrap(), Err(fs::TryLockError::WouldBlock)),
            "the CLI's approve can't land between the server's load and save"
        );
    }

    #[test]
    fn pair_use_remove_and_only_hashes_on_disk() {
        let d = scratch("flow");
        let code = d.pair(1000).unwrap();
        assert_eq!(code.len(), 6);
        let token = d
            .claim(&code, "Mozilla/5.0 (iPhone)", 2000)
            .unwrap()
            .unwrap();
        // Single use.
        assert_eq!(d.claim(&code, "x", 2000).unwrap(), None);
        let found = d.find(&token, 3000).unwrap().unwrap();
        assert_eq!(found.label, "iPhone");
        assert_eq!(d.find("not-a-token", 3000).unwrap(), None);

        let disk = fs::read_to_string(&d.path).unwrap();
        assert!(!disk.contains(&token) && !disk.contains(&code), "{disk}");
        assert!(disk.contains(&sha256(token.as_bytes())));
        let mode = fs::metadata(&d.path).unwrap().permissions();
        assert_eq!(
            std::os::unix::fs::PermissionsExt::mode(&mode) & 0o777,
            0o600
        );

        assert!(d.remove(&found.id).unwrap());
        assert_eq!(d.find(&token, 4000).unwrap(), None);
        assert_eq!(d.list().unwrap(), Vec::new());
    }

    #[test]
    fn expired_and_guessed_codes_are_refused() {
        let d = scratch("refused");
        let code = d.pair(0).unwrap();
        assert_eq!(d.claim(&code, "x", CODE_TTL).unwrap(), None);
        let code = d.pair(0).unwrap();
        let wrong = if code == "000000" { "000001" } else { "000000" };
        for _ in 0..MISSES {
            assert_eq!(d.claim(wrong, "x", 1).unwrap(), None);
        }
        // Too many misses dropped the code: even the right one fails now.
        assert_eq!(d.claim(&code, "x", 1).unwrap(), None);
    }

    #[test]
    fn a_request_waits_for_an_answer_then_pairs_once_or_is_declined() {
        let d = scratch("request");
        let (word, claim) = d.ask("Mozilla/5.0 (iPad)", 0).unwrap();
        assert_eq!(word.split(' ').count(), 2);
        assert_eq!(d.collect(&claim, 1).unwrap(), Claim::Waiting);
        let open = d.asking(1).unwrap();
        assert_eq!(
            (open[0].word.as_str(), open[0].label.as_str()),
            (word.as_str(), "iPad")
        );
        assert!(!d.answer("deadbeef", true, "iPhone", 1).unwrap());
        assert!(d.answer(&open[0].id, true, "iPhone", 1).unwrap());
        assert_eq!(d.asking(1).unwrap(), []);
        // Renamed on the approving device before and after the new one picks it up; its id is the request's.
        assert!(d.rename(&open[0].id, "Kim's iPad").unwrap());
        let Claim::Paired(token) = d.collect(&claim, 2).unwrap() else {
            panic!("not paired");
        };
        let found = d.find(&token, 3).unwrap().unwrap();
        assert_eq!(
            (found.id.as_str(), found.label.as_str()),
            (open[0].id.as_str(), "Kim's iPad")
        );
        assert!(d.rename(&open[0].id, "iPad").unwrap());
        assert_eq!(d.find(&token, 3).unwrap().unwrap().label, "iPad");
        assert!(!d.rename("deadbeef", "x").unwrap());
        assert_eq!(d.collect(&claim, 3).unwrap(), Claim::Gone);
        let disk = fs::read_to_string(&d.path).unwrap();
        assert!(!disk.contains(&claim) && !disk.contains(&token), "{disk}");

        // Declined: the new device hears who said no, once.
        let (_, claim) = d.ask("x", 0).unwrap();
        let id = d.asking(0).unwrap()[0].id.clone();
        assert!(d.answer(&id, false, "iPhone", 1).unwrap());
        assert!(!d.answer(&id, true, "iPhone", 1).unwrap(), "answered once");
        assert_eq!(
            d.collect(&claim, 2).unwrap(),
            Claim::Declined("iPhone".into())
        );
        assert_eq!(d.collect(&claim, 2).unwrap(), Claim::Gone);

        // Expired after 5 minutes, cancelled, and a fifth request dropping the oldest.
        let (_, claim) = d.ask("x", 0).unwrap();
        assert_eq!(d.asking(ASK_TTL).unwrap(), []);
        assert_eq!(d.collect(&claim, ASK_TTL).unwrap(), Claim::Gone);
        let (_, claim) = d.ask("x", 0).unwrap();
        d.cancel(&claim).unwrap();
        assert_eq!(d.collect(&claim, 1).unwrap(), Claim::Gone);
        let first = d.ask("x", 0).unwrap().1;
        for _ in 0..REQUESTS {
            d.ask("x", 0).unwrap();
        }
        assert_eq!(d.collect(&first, 1).unwrap(), Claim::Gone);
    }

    /// #294: the auth middleware's lookups parse the file once while it is unchanged; another writer's change (the
    /// CLI's pair or remove) counts on the next lookup.
    #[test]
    fn lookups_parse_an_unchanged_file_once_and_see_the_next_change() {
        let d = scratch("cache");
        let token = d.claim(&d.pair(0).unwrap(), "x", 0).unwrap().unwrap();
        assert!(d.find(&token, 0).unwrap().is_some());

        let disk = fs::OpenOptions::new().write(true).open(&d.path).unwrap();
        let (stamp, len) = (
            disk.metadata().unwrap().modified().unwrap(),
            disk.metadata().unwrap().len(),
        );
        disk.write_all_at(&vec![b' '; usize::try_from(len).unwrap()], 0)
            .unwrap();
        disk.set_modified(stamp).unwrap();
        assert!(
            d.find(&token, 0).unwrap().is_some(),
            "same stamp: not parsed again"
        );
        d.save(&d.load().unwrap()).unwrap();

        let cli = Devices::new(d.path.parent().unwrap());
        let other = cli.claim(&cli.pair(0).unwrap(), "x", 0).unwrap().unwrap();
        assert!(
            d.find(&other, 0).unwrap().is_some(),
            "an added device pairs at once"
        );
        let id = d.find(&token, 0).unwrap().unwrap().id;
        assert!(cli.remove(&id).unwrap());
        assert_eq!(
            d.find(&token, 0).unwrap(),
            None,
            "a revoked device is out at once"
        );
    }

    /// R3 (herdsman#501): a devices file that can't be read is an error, never "not paired".
    #[test]
    fn an_unreadable_store_is_an_error_not_unpaired() {
        let d = scratch("unreadable");
        let token = d.claim(&d.pair(0).unwrap(), "x", 0).unwrap().unwrap();
        fs::write(&d.path, b"{ torn").unwrap();
        assert!(d.find(&token, 1).is_err());
    }

    /// R18 (herdsman#342): parallel writes of one file each use a temp file of their own and rename it whole.
    #[test]
    fn parallel_writes_never_share_a_temp_file() {
        let dir = std::env::temp_dir().join(format!("cooee-write-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("state.json");
        std::thread::scope(|s| {
            for i in 0..16 {
                let path = &path;
                s.spawn(move || {
                    for j in 0..20 {
                        write_private(path, format!("{i}-{j}").as_bytes()).unwrap();
                    }
                });
            }
        });
        let last = fs::read_to_string(&path).unwrap();
        assert!(last.split_once('-').is_some(), "{last}");
        let left: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name())
            .collect();
        assert_eq!(left, ["state.json"]);
    }
}
