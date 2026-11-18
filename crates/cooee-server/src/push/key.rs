use std::{
    io,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::PoisonError,
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use cooee_core::{random, write_private};
use p256::SecretKey;

use super::{
    Push,
    crypto::{b64, new_key, public},
    subs::LOCK,
};

fn key_file(data: &Path) -> PathBuf {
    data.join("vapid.key")
}

fn find_key(data: &Path) -> io::Result<Option<String>> {
    read_private(&key_file(data))
}

pub(super) fn read_private(path: &Path) -> io::Result<Option<String>> {
    let mode = match std::fs::metadata(path) {
        Ok(m) => m.permissions().mode(),
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    if mode & 0o077 != 0 {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(Some(std::fs::read_to_string(path)?.trim().to_owned()))
}

fn keep_key(data: &Path, key: &str) -> io::Result<String> {
    let path = key_file(data);
    let tmp = path.with_extension(format!("new-{}", B64.encode(random(6)?)));
    write_private(&tmp, key.as_bytes())?;
    let linked = std::fs::hard_link(&tmp, &path);
    std::fs::remove_file(&tmp)?;
    match linked {
        Err(e) if e.kind() != io::ErrorKind::AlreadyExists => Err(e),
        _ => find_key(data)?.ok_or_else(|| io::Error::other("the VAPID key file vanished")),
    }
}

fn stored_key(data: &Path) -> io::Result<String> {
    match find_key(data)? {
        Some(k) => Ok(k),
        None => keep_key(data, &B64.encode(new_key()?.to_bytes())),
    }
}

impl Push {
    pub(super) fn vapid(&self) -> io::Result<SecretKey> {
        let mut key = self.key.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(k) = key.as_ref() {
            return Ok(k.clone());
        }
        let k = SecretKey::from_slice(&b64(&stored_key(&self.data)?)?)
            .map_err(|_| io::Error::other("the stored VAPID key is not a P-256 key"))?;
        *key = Some(k.clone());
        Ok(k)
    }

    /// Whether the VAPID key exists yet (it is made when a phone first asks for it).
    ///
    /// # Errors
    /// When the key file can't be read.
    pub fn has_key(&self) -> io::Result<bool> {
        Ok(find_key(&self.data)?.is_some())
    }

    /// The VAPID public key, base64url: the page subscribes with it.
    ///
    /// # Errors
    /// When the key can't be read or made.
    pub fn public_key(&self) -> io::Result<String> {
        Ok(B64.encode(public(&self.vapid()?)))
    }

    /// Keeps `key` (base64url), the key from before the key file, unless there is a key file already; whether
    /// it was kept.
    ///
    /// # Errors
    /// When `key` is not a P-256 key, or it can't be stored.
    pub fn adopt(&self, key: &str) -> io::Result<bool> {
        SecretKey::from_slice(&b64(key)?)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "not a P-256 key"))?;
        Ok(keep_key(&self.data, key)? == key)
    }

    /// The key from before is lost: a new one is made (unless there is one already) and every subscription, made
    /// with the old key, is set aside with `why`, so its device is asked to turn alerts on again.
    ///
    /// # Errors
    /// When the key or the subscriptions can't be stored.
    pub fn renew(&self, why: &str) -> io::Result<()> {
        stored_key(&self.data)?;
        let _g = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        let mut all = self.load();
        for s in all
            .iter_mut()
            .filter(|s| s.reset.is_none() && s.apns.is_none())
        {
            s.reset = Some(why.to_owned());
        }
        self.store(&all)
    }

    /// Why `device`'s alerts stopped, if they did and it hasn't turned them on again.
    #[must_use]
    pub fn reset(&self, device: &str) -> Option<String> {
        let mut subs = self.load().into_iter().filter(|s| s.device == device);
        subs.find_map(|s| s.reset)
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::push::Sub;

    pub(in crate::push) fn dir(name: &str) -> PathBuf {
        let dir = cooee_core::scratch::owned(
            std::env::temp_dir().join(format!("cooee-push-{name}-{}", std::process::id())),
        );
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    pub(in crate::push) fn mode(path: &Path) -> u32 {
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    #[test]
    fn the_key_is_made_in_a_0600_file() {
        let dir = dir("made");
        assert!(!Push::new(&dir).has_key().unwrap());
        let public = Push::new(&dir).public_key().unwrap();
        assert_eq!(mode(&dir.join("vapid.key")), 0o600);
        assert!(Push::new(&dir).has_key().unwrap());
        assert_eq!(Push::new(&dir).public_key().unwrap(), public);
    }

    #[test]
    fn a_0644_key_file_is_tightened() {
        let dir = dir("loose");
        let key = new_key().unwrap();
        let file = dir.join("vapid.key");
        std::fs::write(&file, B64.encode(key.to_bytes())).unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(Push::new(&dir).has_key().unwrap());
        assert_eq!(mode(&file), 0o600);
        assert_eq!(
            Push::new(&dir).public_key().unwrap(),
            B64.encode(public(&key))
        );
    }

    #[test]
    fn an_old_key_is_adopted_once() {
        let dir = dir("adopt");
        let (old, other) = (new_key().unwrap(), new_key().unwrap());
        let push = Push::new(&dir);
        assert!(push.adopt(&B64.encode(old.to_bytes())).unwrap());
        assert!(!push.adopt(&B64.encode(other.to_bytes())).unwrap());
        assert!(push.adopt("not a key").is_err());
        assert_eq!(mode(&dir.join("vapid.key")), 0o600);
        assert_eq!(push.public_key().unwrap(), B64.encode(public(&old)));
    }

    #[test]
    fn a_renewed_key_asks_each_device_to_subscribe_again() {
        let dir = dir("renew");
        let push = Push::new(&dir);
        let sub = |device: &str, endpoint: &str| {
            Sub {
            device: device.to_owned(),
            endpoint: endpoint.to_owned(),
            p256dh: "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4".into(),
            auth: "BTBZMqHH6r4Tts7J_aSIgg".into(),
            reset: None,
            apns: None,
        }
        };
        push.subscribe(sub("a", "https://push.example.com/a"))
            .unwrap();
        push.subscribe(sub("b", "https://push.example.com/b"))
            .unwrap();
        push.renew("the key changed").unwrap();
        assert!(push.has_key().unwrap());
        for d in ["a", "b"] {
            assert_eq!(push.endpoints(d), Vec::<String>::new());
            assert_eq!(push.reset(d).as_deref(), Some("the key changed"));
        }
        push.subscribe(sub("a", "https://push.example.com/a2"))
            .unwrap();
        assert_eq!(push.endpoints("a"), ["https://push.example.com/a2"]);
        assert_eq!(push.reset("a"), None);
        assert_eq!(push.reset("b").as_deref(), Some("the key changed"));
    }
}
