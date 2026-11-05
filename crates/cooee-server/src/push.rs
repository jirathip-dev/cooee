//! Web push (herdsman's `webpush.rs`): an RFC 8291 aes128gcm body and an ES256 VAPID JWT (RFC 8292), sent by
//! `curl` with the body on stdin. The VAPID key is made once and kept in the macOS keychain, else in
//! `<data>/vapid.key` (0600); the subscriptions are in `<data>/push.json` (0600), each tied to a paired device, so
//! unpairing it stops its pushes.

use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex, PoisonError},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use aes_gcm::{
    Aes128Gcm, KeyInit, Nonce,
    aead::{Aead, generic_array::GenericArray},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use cooee_core::{Body, Change, Chat, Devices, Thread, random, write_private};
use hkdf::Hkdf;
use p256::{
    PublicKey, SecretKey,
    ecdsa::{Signature, SigningKey, signature::Signer},
    elliptic_curve::sec1::ToEncodedPoint,
};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

/// One notification: a line about a thread; tapping it opens `url`. `tag` (the thread) makes a newer one replace
/// the older, and the page closes it once read past `item`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Note {
    /// The thread's title.
    pub title: String,
    /// What happened, one line.
    pub body: String,
    /// The thread's id.
    pub tag: String,
    /// The thread's page.
    pub url: String,
    /// The item it is about.
    pub item: u64,
}

/// The notification for `change` in `thread`, if it is one: an approval waiting, a turn done while no page
/// watches the thread (`watched`), or an error. `was` is the state before the change.
#[must_use]
pub fn note(
    thread: &Thread,
    chat: &Chat,
    change: &Change,
    was: &str,
    watched: bool,
) -> Option<Note> {
    let last = chat.items.last().map_or(0, |i| i.id);
    let (body, item) = match change {
        Change::Item(i) => match &i.body {
            Body::Approval {
                tool,
                tapped: None,
                chosen: None,
                ..
            } => (format!("needs you: {tool}"), i.id),
            _ => return None,
        },
        Change::State(t) if t.state == was => return None,
        Change::State(t) if t.state == "done" && !watched => {
            let reply = chat.items.iter().rev().find_map(|i| match &i.body {
                Body::Agent { text, .. } => text.lines().find(|l| !l.trim().is_empty()),
                _ => None,
            });
            (format!("done: {}", reply.unwrap_or("the turn ended")), last)
        }
        Change::State(t) if t.state == "error" => (
            format!("error: {}", t.note.as_deref().unwrap_or("the agent failed")),
            last,
        ),
        Change::State(_) | Change::Text(_) => return None,
    };
    Some(Note {
        title: thread.title.clone(),
        body: body.chars().take(120).collect(),
        tag: thread.id.clone(),
        url: format!("/t/{}", thread.id),
        item,
    })
}

/// A browser's push subscription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sub {
    /// The paired device it belongs to.
    #[serde(default)]
    pub device: String,
    /// The push service URL.
    pub endpoint: String,
    /// The browser's P-256 key, base64url.
    pub p256dh: String,
    /// The browser's auth secret, base64url.
    pub auth: String,
}

/// The subscriptions and the VAPID key of one data folder.
#[derive(Clone)]
pub struct Push {
    data: PathBuf,
    key: Arc<Mutex<Option<SecretKey>>>,
}

static LOCK: Mutex<()> = Mutex::new(());

fn b64(s: &str) -> io::Result<Vec<u8>> {
    B64.decode(s.trim_end_matches('='))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))
}

/// A fresh P-256 key.
fn new_key() -> io::Result<SecretKey> {
    loop {
        if let Ok(k) = SecretKey::from_slice(&random(32)?) {
            return Ok(k);
        }
    }
}

/// The uncompressed public point (65 bytes).
fn public(k: &SecretKey) -> Vec<u8> {
    k.public_key().to_encoded_point(false).as_bytes().to_vec()
}

/// RFC 8291 aes128gcm body for one subscription: a single record, `salt` and `eph`'s public key in its header.
///
/// # Errors
/// When `p256dh` isn't a P-256 point.
pub fn encrypt(
    p256dh: &[u8],
    auth: &[u8],
    plain: &[u8],
    eph: &SecretKey,
    salt: &[u8; 16],
) -> io::Result<Vec<u8>> {
    let bad = |_| io::Error::new(io::ErrorKind::InvalidInput, "bad push key");
    let ua = PublicKey::from_sec1_bytes(p256dh).map_err(bad)?;
    let shared = p256::ecdh::diffie_hellman(eph.to_nonzero_scalar(), ua.as_affine());
    let as_pub = public(eph);
    let info = [b"WebPush: info\0".as_slice(), p256dh, &as_pub].concat();
    let hk = |salt: &[u8], ikm: &[u8], info: &[u8], out: &mut [u8]| {
        Hkdf::<Sha256>::new(Some(salt), ikm)
            .expand(info, out)
            .map_err(|_| io::Error::other("hkdf"))
    };
    let (mut ikm, mut cek, mut nonce) = ([0u8; 32], [0u8; 16], [0u8; 12]);
    hk(auth, shared.raw_secret_bytes(), &info, &mut ikm)?;
    hk(salt, &ikm, b"Content-Encoding: aes128gcm\0", &mut cek)?;
    hk(salt, &ikm, b"Content-Encoding: nonce\0", &mut nonce)?;
    let ct = Aes128Gcm::new(GenericArray::from_slice(&cek))
        .encrypt(Nonce::from_slice(&nonce), [plain, &[2]].concat().as_slice())
        .map_err(|_| io::Error::other("aes-gcm"))?;
    Ok([salt.as_slice(), &4096u32.to_be_bytes(), &[65], &as_pub, &ct].concat())
}

/// The ES256 VAPID JWT for `endpoint`'s origin, valid 12 h from `now` (Unix seconds).
#[must_use]
pub fn jwt(k: &SecretKey, endpoint: &str, now: u64) -> String {
    let aud = endpoint
        .splitn(4, '/')
        .take(3)
        .collect::<Vec<_>>()
        .join("/");
    let sub = "https://github.com/jirathip-dev/cooee";
    let claims = serde_json::json!({"aud": aud, "exp": now + 12 * 3600, "sub": sub}).to_string();
    let msg = format!(
        "{}.{}",
        B64.encode(r#"{"typ":"JWT","alg":"ES256"}"#),
        B64.encode(claims)
    );
    let sig: Signature = SigningKey::from(k).sign(msg.as_bytes());
    format!("{msg}.{}", B64.encode(sig.to_bytes()))
}

/// The stored VAPID key (base64url), made on first use: the keychain on macOS, else a 0600 file.
fn stored_key(data: &Path) -> io::Result<String> {
    if cfg!(target_os = "macos") {
        let find = ["find-generic-password", "-s", "cooee", "-a", "vapid", "-w"];
        let out = Command::new("security").args(find).output()?;
        if out.status.success() {
            return Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned());
        }
        let k = B64.encode(new_key()?.to_bytes());
        // `-i` reads the command from stdin, so the key never shows in a process list.
        let mut add = Command::new("security")
            .arg("-i")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()?;
        if let Some(mut stdin) = add.stdin.take() {
            writeln!(stdin, "add-generic-password -U -s cooee -a vapid -w {k}")?;
        }
        if !add.wait()?.success() {
            return Err(io::Error::other(
                "can't store the VAPID key in the keychain",
            ));
        }
        return Ok(k);
    }
    let path = data.join("vapid.key");
    match std::fs::read_to_string(&path) {
        Ok(k) => Ok(k.trim().to_owned()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let k = B64.encode(new_key()?.to_bytes());
            write_private(&path, k.as_bytes())?;
            Ok(k)
        }
        Err(e) => Err(e),
    }
}

impl Push {
    /// The push state of data folder `data`.
    #[must_use]
    pub fn new(data: &Path) -> Self {
        Self {
            data: data.to_owned(),
            key: Arc::default(),
        }
    }

    fn vapid(&self) -> io::Result<SecretKey> {
        let mut key = self.key.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(k) = key.as_ref() {
            return Ok(k.clone());
        }
        let k = SecretKey::from_slice(&b64(&stored_key(&self.data)?)?)
            .map_err(|_| io::Error::other("the stored VAPID key is not a P-256 key"))?;
        *key = Some(k.clone());
        Ok(k)
    }

    /// The VAPID public key, base64url: the page subscribes with it.
    ///
    /// # Errors
    /// When the key can't be read or made.
    pub fn public_key(&self) -> io::Result<String> {
        Ok(B64.encode(public(&self.vapid()?)))
    }

    fn path(&self) -> PathBuf {
        self.data.join("push.json")
    }

    fn load(&self) -> Vec<Sub> {
        std::fs::read(self.path())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    fn store(&self, subs: &[Sub]) -> io::Result<()> {
        write_private(&self.path(), &serde_json::to_vec_pretty(subs)?)
    }

    /// Adds (or replaces) `sub`, checked: an `https://` endpoint, a P-256 key and a 16-byte auth secret.
    ///
    /// # Errors
    /// [`io::ErrorKind::InvalidInput`] for a bad subscription; else when it can't be stored.
    pub fn subscribe(&self, sub: Sub) -> io::Result<()> {
        let bad = |m: &str| io::Error::new(io::ErrorKind::InvalidInput, m.to_owned());
        if !sub.endpoint.starts_with("https://") || sub.endpoint.chars().any(char::is_whitespace) {
            return Err(bad("the endpoint must be an https:// URL"));
        }
        if PublicKey::from_sec1_bytes(&b64(&sub.p256dh)?).is_err() || b64(&sub.auth)?.len() != 16 {
            return Err(bad("bad p256dh or auth key"));
        }
        let _g = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        let mut all = self.load();
        all.retain(|s| s.endpoint != sub.endpoint);
        all.push(sub);
        self.store(&all)
    }

    /// Sends `note` to every paired device's subscriptions, in the background: a 404/410 or an unpaired device
    /// drops the subscription; other failures are logged.
    pub fn send(&self, note: &Note) {
        let push = self.clone();
        let payload = serde_json::to_vec(note).unwrap_or_default();
        thread::spawn(move || {
            if let Err(e) = push.deliver(&payload) {
                eprintln!("cooee: push: {e}");
            }
        });
    }

    fn deliver(&self, payload: &[u8]) -> io::Result<()> {
        let paired: Vec<String> = Devices::new(&self.data)
            .list()?
            .into_iter()
            .map(|d| d.id)
            .collect();
        let all = self.load();
        let (live, mut gone): (Vec<Sub>, Vec<Sub>) =
            all.into_iter().partition(|s| paired.contains(&s.device));
        for s in &live {
            let key = self.vapid()?;
            let one = || -> io::Result<u16> {
                let salt: [u8; 16] = random(16)?.try_into().unwrap_or_default();
                let body = encrypt(
                    &b64(&s.p256dh)?,
                    &b64(&s.auth)?,
                    payload,
                    &new_key()?,
                    &salt,
                )?;
                post(s, &key, &body)
            };
            match one() {
                Ok(200..=299) => {}
                Ok(404 | 410) => gone.push(s.clone()),
                Ok(code) => eprintln!("cooee: push to device {}: HTTP {code}", s.device),
                Err(e) => eprintln!("cooee: push to device {}: {e}", s.device),
            }
        }
        if !gone.is_empty() {
            let _g = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
            let mut all = self.load();
            all.retain(|s| !gone.contains(s));
            self.store(&all)?;
        }
        Ok(())
    }
}

/// POSTs one push with curl; the HTTP status (0: no response).
fn post(s: &Sub, k: &SecretKey, body: &[u8]) -> io::Result<u16> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let auth = format!(
        "Authorization: vapid t={}, k={}",
        jwt(k, &s.endpoint, now),
        B64.encode(public(k))
    );
    let mut c = Command::new("curl")
        .args(["-sS", "--max-time", "10", "-X", "POST", "-o", "/dev/null"])
        .args(["-w", "%{http_code}", "--data-binary", "@-", "-H", &auth])
        .args(["-H", "Content-Encoding: aes128gcm"])
        .args(["-H", "Content-Type: application/octet-stream"])
        .args(["-H", "TTL: 86400", "-H", "Urgency: high", &s.endpoint])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    if let Some(mut stdin) = c.stdin.take() {
        stdin.write_all(body)?;
    }
    Ok(String::from_utf8_lossy(&c.wait_with_output()?.stdout)
        .trim()
        .parse()
        .unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use cooee_core::{Item, Turn};

    use super::*;

    /// RFC 8291 appendix A.
    #[test]
    fn encrypt_matches_rfc_8291_vector() {
        let d = |s: &str| B64.decode(s).unwrap();
        let eph = SecretKey::from_slice(&d("yfWPiYE-n46HLnH0KqZOF1fJJU3MYrct3AELtAQ-oRw")).unwrap();
        let salt: [u8; 16] = d("DGv6ra1nlYgDCS1FRnbzlw").try_into().unwrap();
        let ua = d(
            "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4",
        );
        let body = encrypt(
            &ua,
            &d("BTBZMqHH6r4Tts7J_aSIgg"),
            b"When I grow up, I want to be a watermelon",
            &eph,
            &salt,
        )
        .unwrap();
        assert_eq!(
            B64.encode(body),
            "DGv6ra1nlYgDCS1FRnbzlwAAEABBBP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A_yl95bQpu6cVPTpK4Mqgkf1CXztLVBSt2Ks3oZwbuwXPXLWyouBWLVWGNWQexSgSxsj_Qulcy4a-fN"
        );
    }

    #[test]
    fn jwt_is_signed_by_the_key_for_the_endpoint_origin() {
        use p256::ecdsa::{VerifyingKey, signature::Verifier};
        let k = new_key().unwrap();
        let t = jwt(&k, "https://push.example.com/send/abc", 1000);
        let (msg, sig) = t.rsplit_once('.').unwrap();
        let sig = Signature::from_slice(&B64.decode(sig).unwrap()).unwrap();
        VerifyingKey::from(&SigningKey::from(&k))
            .verify(msg.as_bytes(), &sig)
            .unwrap();
        let claims = B64.decode(msg.split('.').nth(1).unwrap()).unwrap();
        let claims: serde_json::Value = serde_json::from_slice(&claims).unwrap();
        assert_eq!(claims["aud"], "https://push.example.com");
        assert_eq!(claims["exp"], 1000 + 12 * 3600);
    }

    fn thread() -> Thread {
        Thread {
            id: "t1".to_owned(),
            title: "Fix the build".to_owned(),
            ..Thread::default()
        }
    }

    fn turn(state: &str, note: Option<&str>) -> Change {
        Change::State(Turn {
            state: state.to_owned(),
            note: note.map(str::to_owned),
            ..Turn::default()
        })
    }

    #[test]
    fn a_push_for_each_trigger_and_none_otherwise() {
        let mut chat = Chat::default();
        chat.push(
            1,
            0,
            Body::You {
                text: "hi".to_owned(),
                random_id: "r".to_owned(),
            },
        );
        chat.push(
            2,
            1,
            Body::Agent {
                text: "\nAll tests pass.\nMore.".to_owned(),
                stop_reason: "end_turn".to_owned(),
            },
        );
        let approval = |tapped: Option<&str>| {
            Change::Item(Item {
                id: 3,
                at: 3,
                seq: 2,
                body: Body::Approval {
                    tool: "Edit src/lib.rs".to_owned(),
                    options: Vec::new(),
                    tapped: tapped.map(str::to_owned),
                    chosen: None,
                },
            })
        };
        let n = |body: &str, item| Note {
            title: "Fix the build".to_owned(),
            body: body.to_owned(),
            tag: "t1".to_owned(),
            url: "/t/t1".to_owned(),
            item,
        };
        let t = thread();
        assert_eq!(
            note(&t, &chat, &approval(None), "working", true),
            Some(n("needs you: Edit src/lib.rs", 3))
        );
        assert_eq!(
            note(&t, &chat, &approval(Some("allow")), "working", false),
            None
        );
        assert_eq!(
            note(&t, &chat, &turn("done", None), "working", false),
            Some(n("done: All tests pass.", 2))
        );
        // A page has the thread open: no push for a finished turn; nor for a state that didn't change.
        assert_eq!(note(&t, &chat, &turn("done", None), "working", true), None);
        assert_eq!(note(&t, &chat, &turn("done", None), "done", false), None);
        assert_eq!(
            note(
                &t,
                &chat,
                &turn("error", Some("rate limited")),
                "working",
                true
            ),
            Some(n("error: rate limited", 2))
        );
        assert_eq!(note(&t, &chat, &turn("working", None), "done", false), None);
        assert_eq!(
            note(&t, &chat, &Change::Text("x".to_owned()), "working", false),
            None
        );
        assert_eq!(
            serde_json::to_value(n("x", 2)).unwrap(),
            serde_json::json!({"title": "Fix the build", "body": "x", "tag": "t1", "url": "/t/t1", "item": 2})
        );
    }
}
