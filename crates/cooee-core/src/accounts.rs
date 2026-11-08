//! A provider's accounts: the system default login (`~/.claude`, …) and the ones added in cooee, each a config folder
//! `<data>/accounts/<provider>/<name>/` that its CLI gets through the provider's variable (`CLAUDE_CONFIG_DIR`, …).
//! The CLI keeps its own login there or in its keychain; cooee never reads or stores a token. Beside the folders,
//! `.cooee.json` keeps only when each one's login last succeeded. There is no default account: a new thread takes
//! the last one used in its folder (#48).

use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{Provider, Status, write_private};

/// A provider as the sign-ins sheet and the new-thread picker show it.
#[derive(Debug, Clone)]
pub struct Sheet {
    /// The provider.
    pub provider: Provider,
    /// Its accounts, the system default first, each with its status.
    pub accounts: Vec<(Account, Status)>,
    /// How its last login ended.
    pub why: Option<String>,
}

/// One account of a provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Account {
    /// Its name; empty for the system default.
    pub name: String,
    /// Its config folder; none for the system default.
    #[serde(skip)]
    pub dir: Option<PathBuf>,
    /// When a login in cooee last succeeded for it (ms since the epoch).
    pub signed_in_at: Option<u64>,
}

impl Account {
    /// Who it is as the page shows it (#48): the email its status reports, else its name; the system default login
    /// with no email is `main login`.
    #[must_use]
    pub fn shown(&self, status: &Status) -> String {
        match (&status.who, self.name.as_str()) {
            (Some(who), _) => who.clone(),
            (None, "") => "main login".to_owned(),
            (None, name) => name.to_owned(),
        }
    }
}

/// `.cooee.json`: the last successful logins, by name (an older one's `default` is ignored).
#[derive(Default, Serialize, Deserialize)]
struct Meta {
    #[serde(default)]
    signed_in: BTreeMap<String, u64>,
}

/// The accounts under `<data>/accounts`.
#[derive(Debug, Clone)]
pub struct Accounts {
    root: PathBuf,
}

/// Fit for a folder name: `a-z 0-9 -`, not empty.
fn safe(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

impl Accounts {
    /// The accounts in `data`.
    #[must_use]
    pub fn new(data: &Path) -> Self {
        Self {
            root: data.join("accounts"),
        }
    }

    fn meta(&self, provider: &str) -> Meta {
        let path = self.root.join(provider).join(".cooee.json");
        fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    fn save(&self, provider: &str, meta: &Meta) -> io::Result<()> {
        let json = serde_json::to_vec(meta).map_err(io::Error::other)?;
        write_private(&self.root.join(provider).join(".cooee.json"), &json)
    }

    /// Provider `provider`'s accounts: the system default first, then the added ones by name.
    #[must_use]
    pub fn list(&self, provider: &str) -> Vec<Account> {
        let meta = self.meta(provider);
        let mut names: Vec<String> = fs::read_dir(self.root.join(provider))
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| safe(n))
            .collect();
        names.sort();
        let names = std::iter::once(String::new()).chain(names);
        names
            .map(|name| Account {
                dir: (!name.is_empty()).then(|| self.root.join(provider).join(&name)),
                signed_in_at: meta.signed_in.get(&name).copied(),
                name,
            })
            .collect()
    }

    /// Account `name` of `provider` (empty: the system default).
    #[must_use]
    pub fn get(&self, provider: &str, name: &str) -> Option<Account> {
        self.list(provider).into_iter().find(|a| a.name == name)
    }

    /// Notes that a login for account `name` of `provider` succeeded at `at`.
    ///
    /// # Errors
    /// When `.cooee.json` can't be written.
    pub fn signed_in(&self, provider: &str, name: &str, at: u64) -> io::Result<()> {
        let mut meta = self.meta(provider);
        meta.signed_in.insert(name.to_owned(), at);
        self.save(provider, &meta)
    }

    /// A new account's name (`account-2`, …) and its folder while the login runs, `.new-<name>` (hidden from
    /// [`Accounts::list`] until [`Accounts::adopt`]).
    ///
    /// # Errors
    /// When the folder can't be made.
    pub fn pending(&self, provider: &str) -> io::Result<(String, PathBuf)> {
        let dir = self.root.join(provider);
        let free = |n: &String| !dir.join(n).exists() && !dir.join(format!(".new-{n}")).exists();
        let name = (2..=u16::MAX)
            .map(|i| format!("account-{i}"))
            .find(free)
            .ok_or_else(|| io::Error::other("no free account name"))?;
        let path = dir.join(format!(".new-{name}"));
        fs::create_dir_all(&path)?;
        Ok((name, path))
    }

    /// Lists new account `name` (from [`Accounts::pending`]) under its own name, its login succeeded at `at`.
    ///
    /// # Errors
    /// When the folder can't be renamed.
    pub fn adopt(&self, provider: &str, name: &str, at: u64) -> io::Result<()> {
        let dir = self.root.join(provider);
        fs::rename(dir.join(format!(".new-{name}")), dir.join(name))?;
        self.signed_in(provider, name, at)
    }

    /// Removes the folder of new account `name` whose login failed.
    pub fn drop_pending(&self, provider: &str, name: &str) {
        let _ = fs::remove_dir_all(self.root.join(provider).join(format!(".new-{name}")));
    }

    /// Removes added account `name`'s folder (the system default's is never touched), and its notes.
    ///
    /// # Errors
    /// When the folder or `.cooee.json` can't be changed.
    pub fn remove(&self, provider: &str, name: &str) -> io::Result<()> {
        let mut meta = self.meta(provider);
        meta.signed_in.remove(name);
        if !name.is_empty() && safe(name) {
            fs::remove_dir_all(self.root.join(provider).join(name))?;
        }
        self.save(provider, &meta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_first_a_new_account_listed_once_adopted_and_removal() {
        let data = std::env::temp_dir().join(format!("cooee-accounts-{}", std::process::id()));
        let a = Accounts::new(&data);
        let names = |a: &Accounts| -> Vec<String> {
            a.list("claude").into_iter().map(|a| a.name).collect()
        };
        assert_eq!(names(&a), [""]);
        assert!(a.get("claude", "").unwrap().dir.is_none());

        let (name, dir) = a.pending("claude").unwrap();
        assert_eq!((name.as_str(), dir.is_dir()), ("account-2", true));
        assert_eq!(names(&a), [""], "a pending account isn't listed");
        assert_eq!(a.pending("claude").unwrap().0, "account-3");
        a.drop_pending("claude", "account-3");
        a.adopt("claude", "account-2", 7).unwrap();
        assert_eq!(names(&a), ["", "account-2"]);
        let work = a.get("claude", "account-2").unwrap();
        assert_eq!(work.signed_in_at, Some(7));
        assert_eq!(work.dir, Some(data.join("accounts/claude/account-2")));

        a.remove("claude", "account-2").unwrap();
        assert!(!data.join("accounts/claude/account-2").exists());
        assert_eq!(names(&a), [""]);
        assert!(a.get("claude", "../x").is_none());
        fs::remove_dir_all(&data).unwrap();
    }

    /// R19 (herdsman#333, #477): a new account's folder left by a login whose cooee died is no lock: the next
    /// login takes the next name, and a failed one cleans up only its own.
    #[test]
    fn a_dead_logins_leftover_folder_never_blocks_a_new_account() {
        let data = std::env::temp_dir().join(format!("cooee-accounts-dead-{}", std::process::id()));
        let _ = fs::remove_dir_all(&data);
        let a = Accounts::new(&data);
        fs::create_dir_all(data.join("accounts/claude/.new-account-2")).unwrap();
        let (name, dir) = a.pending("claude").unwrap();
        assert_eq!((name.as_str(), dir.is_dir()), ("account-3", true));
        a.adopt("claude", "account-3", 1).unwrap();
        let names: Vec<String> = a.list("claude").into_iter().map(|a| a.name).collect();
        assert_eq!(names, ["", "account-3"]);
        fs::remove_dir_all(&data).unwrap();
    }
}
