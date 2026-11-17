use std::{fs, io, path::PathBuf};

use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, generic_array::GenericArray},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use cooee_core::{Body, browser, random, secrets, write_private};

use super::{Hub, HubError, now, refused};

const KEY: &str = "FRAMES_KEY";
const KEEP: u64 = 14 * 24 * 3600 * 1000;
const NONCE: usize = 12;

fn plain(name: &str) -> bool {
    !name.starts_with('.') && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

impl Hub {
    fn frames(&self) -> PathBuf {
        self.data.join("frames")
    }

    fn sealer(&self) -> io::Result<Aes256Gcm> {
        let key = if let Some(k) = secrets::get(&self.data, KEY)? {
            k
        } else {
            let k = STANDARD.encode(random(32)?);
            secrets::set(&self.data, KEY, &k)?;
            k
        };
        let key = STANDARD.decode(key.trim()).map_err(io::Error::other)?;
        Aes256Gcm::new_from_slice(&key).map_err(io::Error::other)
    }

    /// Thread `id`'s proxy's replay frame (#161): `image` (base64) with `caption`, kept encrypted on this host as
    /// `<data>/frames/<id>/<name>` and shown in the thread; dropped while the persona's browser has a page open on
    /// a site it may not frame (a `no_frames` one, or one off its allowlist).
    ///
    /// # Errors
    /// [`HubError::Refused`] as [`Hub::token_of`], or when the frame can't be sealed or stored.
    pub fn frame(
        &self,
        (id, token): (&str, &str),
        caption: &str,
        image: &str,
    ) -> Result<(), HubError> {
        let persona = self.persona(&self.token_of(id, token)?.profile);
        let profile = browser::profile(&self.data, &persona.id);
        let cdp =
            browser::endpoint(&self.chromium()?, &profile).map_err(|e| refused(&profile, &e))?;
        let pages = browser::pages(&cdp).unwrap_or_default();
        if pages.is_empty() || !pages.iter().all(|u| browser::framed(&persona.sites, u)) {
            return Ok(());
        }
        let image = STANDARD
            .decode(image)
            .map_err(|e| HubError::Refused(format!("a frame isn't base64: {e}")))?;
        self.seal(id, caption, &image)
    }

    fn seal(&self, id: &str, caption: &str, image: &[u8]) -> Result<(), HubError> {
        let dir = self.frames().join(id);
        let sealed = random(NONCE).and_then(|nonce| {
            let mut out = self
                .sealer()?
                .encrypt(Nonce::from_slice(&nonce), image)
                .map_err(|_| io::Error::other("the frame didn't seal"))?;
            out.splice(0..0, nonce);
            Ok(out)
        });
        let sealed = sealed.map_err(|e| refused(&dir, &e))?;
        let at = now();
        let name = format!(
            "{at}-{}",
            cooee_core::sha256(&sealed)
                .chars()
                .take(12)
                .collect::<String>()
        );
        write_private(&dir.join(&name), &sealed).map_err(|e| refused(&dir, &e))?;
        self.say(
            id,
            Body::Frame {
                text: caption.to_owned(),
                name,
            },
        )
        .map(drop)
    }

    /// Frame `name` of thread `id`, decrypted.
    ///
    /// # Errors
    /// [`HubError::NoThread`] for no such thread or frame (one past its 14 days is gone); [`HubError::Refused`]
    /// when it can't be decrypted.
    pub fn frame_image(&self, id: &str, name: &str) -> Result<Vec<u8>, HubError> {
        self.live(id)?;
        let path = self.frames().join(id).join(name);
        let sealed = plain(name)
            .then(|| fs::read(&path).ok())
            .flatten()
            .ok_or_else(|| HubError::NoThread(format!("{id}/frames/{name}")))?;
        let (nonce, body) = sealed
            .split_at_checked(NONCE)
            .ok_or_else(|| HubError::Refused(format!("{}: too short", path.display())))?;
        self.sealer()
            .map_err(|e| refused(&path, &e))?
            .decrypt(GenericArray::from_slice(nonce), body)
            .map_err(|_| HubError::Refused(format!("{}: doesn't decrypt", path.display())))
    }

    pub(super) fn drop_frames(&self, now: u64) {
        let Ok(threads) = fs::read_dir(self.frames()) else {
            return;
        };
        for dir in threads.filter_map(Result::ok).map(|e| e.path()) {
            for f in fs::read_dir(&dir)
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
            {
                let name = f.file_name().to_string_lossy().into_owned();
                let at = name.split('-').next().and_then(|a| a.parse::<u64>().ok());
                if at.is_some_and(|at| now >= at.saturating_add(KEEP))
                    && let Err(e) = fs::remove_file(f.path())
                {
                    cooee_core::note!("cooee: {}: {e}", f.path().display());
                }
            }
            fs::remove_dir(&dir).ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{hub, scratch, seed};
    use super::*;

    #[test]
    fn a_frame_is_encrypted_on_disk_shown_in_the_thread_and_deleted_after_14_days() {
        let dir = scratch("frames");
        seed(&dir, "t", "");
        let secrets = dir.join("data/secrets");
        fs::create_dir_all(&secrets).unwrap();
        fs::write(secrets.join(KEY), STANDARD.encode([7u8; 32])).unwrap();
        let hub = hub(&dir);
        let t0 = now();
        hub.seal("t", "click \"Sign in\"", b"the-pixels").unwrap();
        let items = hub.slice("t", None, 50, 0).unwrap().items;
        let Some(Body::Frame { text, name }) = items.last().map(|i| i.body.clone()) else {
            panic!("{items:?}");
        };
        assert_eq!(text, "click \"Sign in\"");
        let stored = fs::read(dir.join("data/frames/t").join(&name)).unwrap();
        assert!(!stored.windows(10).any(|w| w == b"the-pixels"));
        assert_eq!(stored.len(), NONCE + b"the-pixels".len() + 16);
        assert_eq!(hub.frame_image("t", &name).unwrap(), b"the-pixels");
        assert!(hub.frame_image("t", "../t.json").is_err());

        hub.watch(t0 + KEEP - 60_000);
        assert_eq!(hub.frame_image("t", &name).unwrap(), b"the-pixels");
        hub.watch(now() + KEEP);
        assert!(matches!(
            hub.frame_image("t", &name),
            Err(HubError::NoThread(_))
        ));
        assert!(!dir.join("data/frames/t").exists());
    }
}
