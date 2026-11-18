use std::{
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, PoisonError},
};

use cooee_core::{Apns, ApnsEnv, write_private};
use p256::{PublicKey, SecretKey};
use serde::{Deserialize, Serialize};

use super::crypto::b64;

/// A browser's push subscription, or a native device's APNs token.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sub {
    /// The paired device it belongs to.
    #[serde(default)]
    pub device: String,
    /// The push service URL (the APNs device token, hex, for an APNs one).
    pub endpoint: String,
    /// The browser's P-256 key, base64url.
    #[serde(default)]
    pub p256dh: String,
    /// The browser's auth secret, base64url.
    #[serde(default)]
    pub auth: String,
    /// Why it no longer works (the key it was made with is gone): its device is asked to turn alerts on again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reset: Option<String>,
    /// An APNs token's server.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apns: Option<ApnsEnv>,
}

/// The subscriptions and the VAPID key of one data folder.
#[derive(Clone)]
pub struct Push {
    pub(super) data: PathBuf,
    pub(super) key: Arc<Mutex<Option<SecretKey>>>,
    pub(super) apns: Option<Apns>,
    pub(super) bearer: Arc<Mutex<Option<(u64, String)>>>,
}

pub(super) static LOCK: Mutex<()> = Mutex::new(());

impl Push {
    /// The push state of data folder `data`.
    #[must_use]
    pub fn new(data: &Path) -> Self {
        Self {
            data: data.to_owned(),
            key: Arc::default(),
            apns: None,
            bearer: Arc::default(),
        }
    }

    fn path(&self) -> PathBuf {
        self.data.join("push.json")
    }

    pub(super) fn load(&self) -> Vec<Sub> {
        std::fs::read(self.path())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub(super) fn store(&self, subs: &[Sub]) -> io::Result<()> {
        write_private(&self.path(), &serde_json::to_vec_pretty(subs)?)
    }

    /// Adds (or replaces) `sub`, checked: an `https://` endpoint, a P-256 key and a 16-byte auth secret.
    ///
    /// # Errors
    /// [`io::ErrorKind::InvalidInput`] for a bad subscription; else when it can't be stored.
    pub fn subscribe(&self, sub: Sub) -> io::Result<()> {
        let bad = |m: &str| io::Error::new(io::ErrorKind::InvalidInput, m.to_owned());
        let hex = sub.endpoint.bytes().all(|b| b.is_ascii_hexdigit());
        if sub.apns.is_some() {
            if !hex || !(32..=200).contains(&sub.endpoint.len()) {
                return Err(bad("the token must be the APNs device token, in hex"));
            }
        } else if !sub.endpoint.starts_with("https://")
            || sub.endpoint.chars().any(char::is_whitespace)
        {
            return Err(bad("the endpoint must be an https:// URL"));
        } else if PublicKey::from_sec1_bytes(&b64(&sub.p256dh)?).is_err()
            || b64(&sub.auth)?.len() != 16
        {
            return Err(bad("bad p256dh or auth key"));
        }
        let _g = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        let mut all = self.load();
        all.retain(|s| s.endpoint != sub.endpoint && (s.device != sub.device || s.reset.is_none()));
        all.push(sub);
        self.store(&all)
    }

    /// Drops the subscription at `endpoint` if it is `device`'s; whether there was one.
    ///
    /// # Errors
    /// When it can't be stored.
    pub fn unsubscribe(&self, device: &str, endpoint: &str) -> io::Result<bool> {
        let _g = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        let mut all = self.load();
        let n = all.len();
        all.retain(|s| s.device != device || s.endpoint != endpoint);
        self.store(&all)?;
        Ok(all.len() < n)
    }

    /// `device`'s subscriptions' endpoints (and APNs tokens).
    #[must_use]
    pub fn endpoints(&self, device: &str) -> Vec<String> {
        let subs = self.load().into_iter();
        let subs = subs.filter(|s| s.device == device && s.reset.is_none());
        subs.map(|s| s.endpoint).collect()
    }
}
