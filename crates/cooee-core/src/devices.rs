//! Paired devices (herdsman's `devices.rs`): `cooee pair` opens a six-digit code for 10 minutes; the browser that
//! enters it gets a random token as its cookie. Only SHA-256 hashes of codes and tokens are kept, in
//! `<data>/devices.json` (mode 0600). Five wrong codes drop the open one. The other way round (#21): a phone asks for a
//! code, shows it, and `cooee pair <code>` on the Mac approves it; the phone then collects its token with the claim it
//! was given.

use std::{
    fmt::Write as _,
    fs,
    io::{self, Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::{
        Mutex, PoisonError,
        atomic::{AtomicU64, Ordering},
    },
};

use serde::{Deserialize, Serialize};

/// How long a pairing code is good for, in milliseconds.
const CODE_TTL: u64 = 10 * 60 * 1000;

/// Wrong codes before the open one is dropped.
const MISSES: u32 = 5;

/// One paired browser.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    /// The first 8 hex digits of its token's hash: what `cooee devices rm` takes.
    pub id: String,
    /// What it is (`iPhone`, `Android`, `Mac`, …), from its user agent.
    pub label: String,
    /// SHA-256 of its token, hex.
    pub hash: String,
    /// When it paired, in Unix milliseconds.
    pub created: u64,
    /// When it was last seen (to the hour), in Unix milliseconds.
    pub used: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct File {
    devices: Vec<Device>,
    /// The open code's hash and when it was made.
    code: Option<(String, u64)>,
    misses: u32,
    requests: Vec<Request>,
}

/// A phone's open request: its code's and claim's hashes, when it was made, and whether the Mac approved it.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Request {
    code: String,
    claim: String,
    at: u64,
    approved: bool,
}

/// Open requests kept at once; a newer one drops the oldest.
const REQUESTS: usize = 4;

/// What a phone's claim finds: its request gone (expired, cancelled or never made), still waiting, or its token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Claim {
    /// No such request.
    Gone,
    /// Not approved yet.
    Waiting,
    /// Approved: the new device's token.
    Paired(String),
}

/// A six-digit code.
fn six() -> io::Result<String> {
    let b = random(4)?;
    Ok(format!(
        "{:06}",
        u32::from_be_bytes([b[0], b[1], b[2], b[3]]) % 1_000_000
    ))
}

impl File {
    /// Adds a device for `agent`; its token.
    fn add(&mut self, token: &str, agent: &str, now: u64) {
        let hash = sha256(token.as_bytes());
        self.devices.push(Device {
            id: hash[..8].to_owned(),
            label: label(agent).to_owned(),
            hash,
            created: now,
            used: now,
        });
    }
}

/// The paired devices of one data folder.
pub struct Devices {
    path: PathBuf,
}

/// Held across a load → change → save in this process.
static LOCK: Mutex<()> = Mutex::new(());

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
fn label(agent: &str) -> &'static str {
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
        }
    }

    fn load(&self) -> io::Result<File> {
        match fs::read(&self.path) {
            Ok(b) => serde_json::from_slice(&b).map_err(io::Error::other),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(File::default()),
            Err(e) => Err(e),
        }
    }

    fn save(&self, f: &File) -> io::Result<()> {
        write_private(&self.path, &serde_json::to_vec_pretty(f)?)
    }

    fn change<T>(&self, f: impl FnOnce(&mut File) -> io::Result<T>) -> io::Result<T> {
        let _g = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
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
            f.add(&token, agent, now);
            Ok(Some(token.clone()))
        })
    }

    /// A phone's request, good for 10 minutes: the code it shows and the claim it collects its token with.
    ///
    /// # Errors
    /// When the file can't be read or written.
    pub fn request(&self, now: u64) -> io::Result<(String, String)> {
        let (code, claim) = (six()?, hex(&random(32)?));
        self.change(|f| {
            f.requests.retain(|r| r.at + CODE_TTL > now);
            if f.requests.len() >= REQUESTS {
                f.requests.remove(0);
            }
            f.requests.push(Request {
                code: sha256(code.as_bytes()),
                claim: sha256(claim.as_bytes()),
                at: now,
                approved: false,
            });
            Ok(())
        })?;
        Ok((code, claim))
    }

    /// Approves the open request showing `code` (`cooee pair <code>`); whether there was one.
    ///
    /// # Errors
    /// When the file can't be read or written.
    pub fn approve(&self, code: &str, now: u64) -> io::Result<bool> {
        let hash = sha256(code.trim().as_bytes());
        self.change(|f| {
            f.requests.retain(|r| r.at + CODE_TTL > now);
            let found = f
                .requests
                .iter_mut()
                .find(|r| !r.approved && same(&r.code, &hash));
            Ok(found.map(|r| r.approved = true).is_some())
        })
    }

    /// The phone's claim: its token once approved (once), else whether it still waits.
    ///
    /// # Errors
    /// When the file can't be read or written.
    pub fn collect(&self, claim: &str, agent: &str, now: u64) -> io::Result<Claim> {
        let (hash, token) = (sha256(claim.as_bytes()), hex(&random(32)?));
        self.change(|f| {
            f.requests.retain(|r| r.at + CODE_TTL > now);
            let Some(i) = f.requests.iter().position(|r| same(&r.claim, &hash)) else {
                return Ok(Claim::Gone);
            };
            if !f.requests[i].approved {
                return Ok(Claim::Waiting);
            }
            f.requests.remove(i);
            f.add(&token, agent, now);
            Ok(Claim::Paired(token.clone()))
        })
    }

    /// Drops the phone's request.
    ///
    /// # Errors
    /// When the file can't be read or written.
    pub fn cancel(&self, claim: &str) -> io::Result<()> {
        let hash = sha256(claim.as_bytes());
        self.change(|f| {
            f.requests.retain(|r| !same(&r.claim, &hash));
            Ok(())
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
    use super::*;

    fn scratch(name: &str) -> Devices {
        let dir = std::env::temp_dir().join(format!("cooee-devices-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        Devices::new(&dir)
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
    fn a_request_waits_for_approval_then_pairs_once() {
        let d = scratch("request");
        let (code, claim) = d.request(0).unwrap();
        assert_eq!(d.collect(&claim, "iPhone", 1).unwrap(), Claim::Waiting);
        assert!(!d.approve("not-it", 1).unwrap());
        assert!(d.approve(&code, 1).unwrap());
        let Claim::Paired(token) = d.collect(&claim, "Mozilla/5.0 (iPhone)", 2).unwrap() else {
            panic!("not paired");
        };
        assert_eq!(d.find(&token, 3).unwrap().unwrap().label, "iPhone");
        assert_eq!(d.collect(&claim, "x", 3).unwrap(), Claim::Gone);
        let disk = fs::read_to_string(&d.path).unwrap();
        assert!(!disk.contains(&claim) && !disk.contains(&token), "{disk}");

        // Expired, cancelled, and a fifth request dropping the oldest.
        let (code, claim) = d.request(0).unwrap();
        assert!(!d.approve(&code, CODE_TTL).unwrap());
        assert_eq!(d.collect(&claim, "x", CODE_TTL).unwrap(), Claim::Gone);
        let (_, claim) = d.request(0).unwrap();
        d.cancel(&claim).unwrap();
        assert_eq!(d.collect(&claim, "x", 1).unwrap(), Claim::Gone);
        let first = d.request(0).unwrap().1;
        for _ in 0..REQUESTS {
            d.request(0).unwrap();
        }
        assert_eq!(d.collect(&first, "x", 1).unwrap(), Claim::Gone);
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
