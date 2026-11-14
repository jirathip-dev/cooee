use std::{
    io::{self, Write},
    os::unix::fs::PermissionsExt,
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
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD as B64},
};
use cooee_core::{Apns, ApnsEnv, Body, Change, Chat, Devices, Thread, random, write_private};
use hkdf::Hkdf;
use p256::{
    PublicKey, SecretKey,
    ecdsa::{Signature, SigningKey, signature::Signer},
    elliptic_curve::sec1::ToSec1Point,
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

impl Note {
    /// `thread`'s note of `key` (`needs`, `done`, `fail`): `theme`'s words, then `who: line`, about item `item`.
    #[must_use]
    pub fn new(
        thread: &Thread,
        (key, who, line): (&str, &str, &str),
        item: u64,
        theme: &cooee_web::Theme,
    ) -> Self {
        Self {
            title: thread.title.clone(),
            body: theme
                .push_body(key, &format!("{who}: {line}"))
                .chars()
                .take(120)
                .collect(),
            tag: thread.id.clone(),
            url: format!("/t/{}", thread.id),
            item,
        }
    }

    /// This member session's note as room `room` says it (#158): titled `Room · name`, opening the room at `item`.
    #[must_use]
    pub fn in_room(self, room: &Thread, name: &str, item: u64) -> Self {
        Self {
            title: format!("{} · {name}", room.title),
            tag: room.id.clone(),
            url: format!("/t/{}", room.id),
            item,
            ..self
        }
    }
}

/// The notification for `change` in `thread`, if it is one: an approval waiting, a turn done while no page
/// watches the thread (`watched`), or an error. `was` is the state before the change. The body is `theme`'s word,
/// the plain word and the agent's line: `at the gate · needs you · acme: Edit src/pair.rs`.
#[must_use]
pub fn note(
    thread: &Thread,
    chat: &Chat,
    change: &Change,
    was: &str,
    watched: bool,
    theme: &cooee_web::Theme,
) -> Option<Note> {
    let last = chat.items.last().map_or(0, |i| i.id);
    let (key, line, item) = match change {
        Change::Item(i) => match &i.body {
            Body::Approval {
                tool,
                tapped: None,
                chosen: None,
                ..
            } => ("needs", tool.clone(), i.id),
            _ => return None,
        },
        Change::State(t) if t.state == was => return None,
        Change::State(t)
            if t.state == "done"
                && !watched
                && thread.workflow.is_none()
                && thread.routine.is_none()
                && thread.room.is_empty() =>
        {
            let reply = chat.items.iter().rev().find_map(|i| match &i.body {
                Body::Agent { text, .. } => text.lines().find(|l| !l.trim().is_empty()),
                _ => None,
            });
            ("done", reply.unwrap_or("the turn ended").to_owned(), last)
        }
        Change::State(t) if t.state == "error" => (
            "fail",
            t.note.as_deref().unwrap_or("the agent failed").to_owned(),
            last,
        ),
        Change::State(_) | Change::Text(_) => return None,
    };
    Some(Note::new(thread, (key, &thread.agent, &line), item, theme))
}

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
    data: PathBuf,
    key: Arc<Mutex<Option<SecretKey>>>,
    apns: Option<Apns>,
    bearer: Arc<Mutex<Option<(u64, String)>>>,
}

static LOCK: Mutex<()> = Mutex::new(());

fn b64(s: &str) -> io::Result<Vec<u8>> {
    B64.decode(s.trim_end_matches('='))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))
}

fn new_key() -> io::Result<SecretKey> {
    loop {
        if let Ok(k) = SecretKey::from_slice(&random(32)?) {
            return Ok(k);
        }
    }
}

fn public(k: &SecretKey) -> Vec<u8> {
    k.public_key().to_sec1_point(false).as_bytes().to_vec()
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
    hk(auth, shared.raw_secret_bytes().as_slice(), &info, &mut ikm)?;
    hk(salt, &ikm, b"Content-Encoding: aes128gcm\0", &mut cek)?;
    hk(salt, &ikm, b"Content-Encoding: nonce\0", &mut nonce)?;
    let ct = Aes128Gcm::new(GenericArray::from_slice(&cek))
        .encrypt(Nonce::from_slice(&nonce), [plain, &[2]].concat().as_slice())
        .map_err(|_| io::Error::other("aes-gcm"))?;
    Ok([salt.as_slice(), &4096u32.to_be_bytes(), &[65], &as_pub, &ct].concat())
}

fn es256(k: &SecretKey, header: &serde_json::Value, claims: &serde_json::Value) -> String {
    let msg = format!(
        "{}.{}",
        B64.encode(header.to_string()),
        B64.encode(claims.to_string())
    );
    let sig: Signature = SigningKey::from(k).sign(msg.as_bytes());
    format!("{msg}.{}", B64.encode(sig.to_bytes()))
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
    let claims = serde_json::json!({"aud": aud, "exp": now + 12 * 3600, "sub": sub});
    es256(
        k,
        &serde_json::json!({"typ": "JWT", "alg": "ES256"}),
        &claims,
    )
}

/// The APNs provider JWT: key `key_id` of team `team_id`, issued at `now`.
#[must_use]
pub fn apns_jwt(k: &SecretKey, key_id: &str, team_id: &str, now: u64) -> String {
    let header = serde_json::json!({"alg": "ES256", "kid": key_id});
    es256(k, &header, &serde_json::json!({"iss": team_id, "iat": now}))
}

fn p8_key(pem: &str) -> Option<SecretKey> {
    let b64: String = pem.lines().filter(|l| !l.starts_with("-----")).collect();
    let der = STANDARD.decode(b64.trim()).ok()?;
    let at = der.windows(5).position(|w| w == [2, 1, 1, 4, 32])? + 5;
    SecretKey::from_slice(der.get(at..at + 32)?).ok()
}

fn key_file(data: &Path) -> PathBuf {
    data.join("vapid.key")
}

fn find_key(data: &Path) -> io::Result<Option<String>> {
    read_private(&key_file(data))
}

fn read_private(path: &Path) -> io::Result<Option<String>> {
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

    /// This push state sending to APNs tokens too, with `apns` (none: it sends to none).
    #[must_use]
    pub fn with_apns(mut self, apns: Option<Apns>) -> Self {
        self.apns = apns;
        self
    }

    fn apns_key(&self) -> Result<(&Apns, SecretKey), String> {
        let a = self.apns.as_ref().ok_or("no [apns] in config.toml")?;
        let path = self.data.join("apns.p8");
        match read_private(&path) {
            Ok(Some(pem)) => p8_key(&pem)
                .map(|k| (a, k))
                .ok_or_else(|| format!("{} is not an APNs key (.p8)", path.display())),
            Ok(None) => Err(format!(
                "no {} (Apple Developer › Keys, APNs)",
                path.display()
            )),
            Err(e) => Err(format!("{}: {e}", path.display())),
        }
    }

    /// Whether APNs can send: the `[apns]` table and its key file are there; else why not.
    ///
    /// # Errors
    /// What is missing.
    pub fn apns_ready(&self) -> Result<(), String> {
        self.apns_key().map(|_| ())
    }

    fn bearer(&self, a: &Apns, k: &SecretKey, now: u64) -> String {
        let mut b = self.bearer.lock().unwrap_or_else(PoisonError::into_inner);
        match &*b {
            Some((at, t)) if now < at + 50 * 60 => t.clone(),
            _ => {
                let t = apns_jwt(k, &a.key_id, &a.team_id, now);
                *b = Some((now, t.clone()));
                t
            }
        }
    }

    /// Adds (or replaces) `device`'s APNs `token` (hex), on `env`'s server (default the config's).
    ///
    /// # Errors
    /// [`io::ErrorKind::InvalidInput`] for a bad token; else when it can't be stored.
    pub fn register_apns(&self, device: &str, token: &str, env: Option<ApnsEnv>) -> io::Result<()> {
        let env = env.or(self.apns.as_ref().map(|a| a.env));
        self.subscribe(Sub {
            device: device.to_owned(),
            endpoint: token.to_ascii_lowercase(),
            apns: Some(env.unwrap_or(ApnsEnv::Production)),
            ..Sub::default()
        })
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

    /// Sends `note` to every paired device's subscriptions and APNs tokens (only `to`'s, if set), in the
    /// background: a 404/410, APNs' `BadDeviceToken` or an unpaired device drops it; other failures are logged.
    pub fn send_to(&self, to: Option<&str>, note: &Note) {
        let (push, to, note) = (self.clone(), to.map(str::to_owned), note.clone());
        thread::spawn(move || match push.deliver(to.as_deref(), &note) {
            Ok(log) => log.iter().for_each(|l| eprintln!("cooee: push{l}")),
            Err(e) => eprintln!("cooee: push: {e}"),
        });
    }

    /// [`Push::send_to`] every device.
    pub fn send(&self, note: &Note) {
        self.send_to(None, note);
    }

    fn deliver(&self, to: Option<&str>, note: &Note) -> io::Result<Vec<String>> {
        let payload = serde_json::to_vec(note)?;
        let paired: Vec<String> = Devices::new(&self.data)
            .list()?
            .into_iter()
            .map(|d| d.id)
            .collect();
        let all = self.load();
        let (live, mut gone): (Vec<Sub>, Vec<Sub>) =
            all.into_iter().partition(|s| paired.contains(&s.device));
        let live = live.iter().filter(|s| s.reset.is_none());
        let (apns, mut log) = (self.apns_key(), Vec::new());
        for s in live.filter(|s| to.is_none_or(|d| d == s.device)) {
            let one = || -> io::Result<(u16, String)> {
                if let Some(env) = s.apns {
                    let (a, k) = apns.as_ref().map_err(|why| io::Error::other(why.clone()))?;
                    return self.apns_post(a, k, env, s, note);
                }
                let salt: [u8; 16] = random(16)?.try_into().unwrap_or_default();
                let body = encrypt(
                    &b64(&s.p256dh)?,
                    &b64(&s.auth)?,
                    &payload,
                    &new_key()?,
                    &salt,
                )?;
                post(s, &self.vapid()?, &body)
            };
            match one() {
                Ok((200..=299, _)) => {}
                Ok((404 | 410, _)) => gone.push(s.clone()),
                Ok((_, why)) if why.contains("BadDeviceToken") => gone.push(s.clone()),
                Ok((code, why)) => log.push(format!(" to device {}: HTTP {code} {why}", s.device)),
                Err(e) => log.push(format!(" to device {}: {e}", s.device)),
            }
        }
        if !gone.is_empty() {
            let _g = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
            let mut all = self.load();
            all.retain(|s| !gone.contains(s));
            self.store(&all)?;
        }
        Ok(log)
    }

    fn apns_post(
        &self,
        a: &Apns,
        k: &SecretKey,
        env: ApnsEnv,
        s: &Sub,
        note: &Note,
    ) -> io::Result<(u16, String)> {
        let host = a.url.as_deref().unwrap_or(match env {
            ApnsEnv::Sandbox => "https://api.sandbox.push.apple.com",
            ApnsEnv::Production => "https://api.push.apple.com",
        });
        let body = serde_json::json!({
            "aps": {"alert": {"title": note.title, "body": note.body}, "thread-id": note.tag, "sound": "default"},
            "url": note.url,
            "item": note.item,
        });
        let h = [
            format!("authorization: bearer {}", self.bearer(a, k, now())),
            format!("apns-topic: {}", a.topic),
            "apns-push-type: alert".to_owned(),
            "apns-priority: 10".to_owned(),
            format!("apns-collapse-id: {}", note.tag),
            "content-type: application/json".to_owned(),
        ];
        let url = format!("{host}/3/device/{}", s.endpoint);
        let args = h.iter().flat_map(|h| ["-H", h]).chain(["--http2", &url]);
        curl(args, body.to_string().as_bytes())
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn post(s: &Sub, k: &SecretKey, body: &[u8]) -> io::Result<(u16, String)> {
    let auth = format!(
        "Authorization: vapid t={}, k={}",
        jwt(k, &s.endpoint, now()),
        B64.encode(public(k))
    );
    let args = ["-H", &auth, "-H", "Content-Encoding: aes128gcm"];
    let args = args
        .into_iter()
        .chain(["-H", "Content-Type: application/octet-stream"]);
    curl(
        args.chain(["-H", "TTL: 86400", "-H", "Urgency: high", &s.endpoint]),
        body,
    )
}

fn curl<'a>(args: impl IntoIterator<Item = &'a str>, body: &[u8]) -> io::Result<(u16, String)> {
    let mut c = Command::new("curl")
        .args([
            "-sS",
            "--max-time",
            "10",
            "-X",
            "POST",
            "-w",
            "\n%{http_code}",
        ])
        .args(["--data-binary", "@-"])
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    if let Some(mut stdin) = c.stdin.take() {
        stdin.write_all(body)?;
    }
    let out = String::from_utf8_lossy(&c.wait_with_output()?.stdout).into_owned();
    let (why, code) = out.rsplit_once('\n').unwrap_or(("", &out));
    Ok((code.trim().parse().unwrap_or(0), why.trim().to_owned()))
}

#[cfg(test)]
mod tests {
    use cooee_core::{Item, Turn};

    use super::*;

    #[test]
    fn a_push_to_one_device_reaches_only_its_endpoint() {
        use std::{net::TcpListener, sync::mpsc};
        let dir = std::env::temp_dir().join(format!("cooee-push-to-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let devices = Devices::new(&dir);
        let ids: Vec<String> = (0..2)
            .map(|i| {
                let code = devices.pair(i).unwrap();
                let token = devices.claim(&code, "x", i).unwrap().unwrap();
                devices.find(&token, i).unwrap().unwrap().id
            })
            .collect();
        let (hits, got) = mpsc::channel();
        let push = Push::new(&dir);
        *push.key.lock().unwrap() = Some(new_key().unwrap());
        for id in &ids {
            let l = TcpListener::bind("127.0.0.1:0").unwrap();
            let endpoint = format!("https://127.0.0.1:{}/x", l.local_addr().unwrap().port());
            let (hits, id2) = (hits.clone(), id.clone());
            thread::spawn(move || {
                for c in l.incoming() {
                    drop(c);
                    hits.send(id2.clone()).unwrap();
                }
            });
            push.subscribe(Sub {
                device: id.clone(),
                endpoint,
                p256dh: "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4".into(),
                auth: "BTBZMqHH6r4Tts7J_aSIgg".into(),
                reset: None,
                apns: None,
            })
            .unwrap();
        }
        push.deliver(Some(&ids[1]), &test_note()).unwrap();
        let first = got.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        assert_eq!(first, ids[1]);
        assert!(got.try_iter().all(|d| d == ids[1]));

        let theirs = push.endpoints(&ids[0]).remove(0);
        assert!(
            !push.unsubscribe(&ids[1], &theirs).unwrap(),
            "not this device's"
        );
        assert!(push.unsubscribe(&ids[0], &theirs).unwrap());
        assert_eq!(push.endpoints(&ids[0]), Vec::<String>::new());
        assert_eq!(push.endpoints(&ids[1]).len(), 1);
    }

    fn test_note() -> Note {
        Note {
            title: "Fix the build".to_owned(),
            body: "needs you".to_owned(),
            tag: "t1".to_owned(),
            url: "/t/t1".to_owned(),
            item: 7,
        }
    }

    fn pem(body: &str) -> String {
        let p = "PRIVATE KEY-----";
        format!("-----BEGIN {p}\n{body}\n-----END {p}\n")
    }

    fn p8(k: &SecretKey) -> String {
        let head = "308187020100301306072a8648ce3d020106082a8648ce3d030107046d306b0201010420";
        let hex = |h: &str| {
            (0..h.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap())
                .collect::<Vec<u8>>()
        };
        let der: Vec<u8> = hex(head)
            .into_iter()
            .chain(k.to_bytes())
            .chain(hex("a144034200"))
            .chain(public(k))
            .collect();
        pem(&STANDARD.encode(der))
    }

    fn apns(url: &str) -> Apns {
        Apns {
            key_id: "KEY1234567".to_owned(),
            team_id: "TEAM123456".to_owned(),
            topic: "dev.cooee.app".to_owned(),
            env: ApnsEnv::Sandbox,
            url: Some(url.to_owned()),
        }
    }

    fn fake_apns(
        answer: fn(&str) -> (u16, &'static str),
    ) -> (String, std::sync::mpsc::Receiver<(String, String)>) {
        use std::io::{BufRead, BufReader, Read};
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://127.0.0.1:{}", l.local_addr().unwrap().port());
        let (tx, rx) = std::sync::mpsc::channel();
        thread::spawn(move || {
            for c in l.incoming() {
                let mut c = c.unwrap();
                let mut r = BufReader::new(c.try_clone().unwrap());
                let (mut head, mut line) = (String::new(), String::new());
                while r.read_line(&mut line).unwrap() > 2 {
                    head.push_str(&line.to_ascii_lowercase());
                    line.clear();
                }
                let len = head
                    .lines()
                    .find_map(|l| l.strip_prefix("content-length: "));
                let mut body = vec![0; len.map_or(0, |n| n.trim().parse().unwrap())];
                r.read_exact(&mut body).unwrap();
                let (code, why) = answer(&head);
                tx.send((head, String::from_utf8(body).unwrap())).unwrap();
                let out = format!(
                    "HTTP/1.1 {code} X\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{why}",
                    why.len()
                );
                c.write_all(out.as_bytes()).unwrap();
            }
        });
        (url, rx)
    }

    fn paired(dir: &Path, n: u64) -> Vec<String> {
        let devices = Devices::new(dir);
        (0..n)
            .map(|i| {
                let code = devices.pair(i).unwrap();
                let token = devices.claim(&code, "x", i).unwrap().unwrap();
                devices.find(&token, i).unwrap().unwrap().id
            })
            .collect()
    }

    #[test]
    fn the_apns_jwt_has_the_key_id_and_team_and_lasts_under_an_hour() {
        use p256::ecdsa::{VerifyingKey, signature::Verifier};
        let k = new_key().unwrap();
        let t = apns_jwt(&k, "KEY1234567", "TEAM123456", 1000);
        let (msg, sig) = t.rsplit_once('.').unwrap();
        let sig = Signature::from_slice(&B64.decode(sig).unwrap()).unwrap();
        VerifyingKey::from(&SigningKey::from(&k))
            .verify(msg.as_bytes(), &sig)
            .unwrap();
        let part = |i| -> serde_json::Value {
            serde_json::from_slice(&B64.decode(msg.split('.').nth(i).unwrap()).unwrap()).unwrap()
        };
        assert_eq!(
            part(0),
            serde_json::json!({"alg": "ES256", "kid": "KEY1234567"})
        );
        assert_eq!(
            part(1),
            serde_json::json!({"iss": "TEAM123456", "iat": 1000})
        );

        let push = Push::new(&dir("bearer"));
        let a = apns("http://x");
        let first = push.bearer(&a, &k, 1000);
        assert_eq!(push.bearer(&a, &k, 1000 + 49 * 60), first);
        let later = push.bearer(&a, &k, 1000 + 50 * 60);
        assert_ne!(later, first);
        let claims: serde_json::Value =
            serde_json::from_slice(&B64.decode(later.split('.').nth(1).unwrap()).unwrap()).unwrap();
        assert_eq!(claims["iat"], 4000);
    }

    #[test]
    fn a_push_goes_to_each_apns_token_and_a_410_drops_it() {
        let dir = dir("apns");
        let ids = paired(&dir, 3);
        let (url, got) = fake_apns(|head| match head.split_whitespace().nth(1).unwrap() {
            p if p.ends_with("bbbb") => (410, r#"{"reason":"Unregistered"}"#),
            p if p.ends_with("cccc") => (400, r#"{"reason":"BadDeviceToken"}"#),
            p if p.ends_with("dddd") => (500, r#"{"reason":"InternalServerError"}"#),
            _ => (200, ""),
        });
        let k = new_key().unwrap();
        let pem = p8(&k);
        write_private(&dir.join("apns.p8"), pem.as_bytes()).unwrap();
        let push = Push::new(&dir).with_apns(Some(apns(&url)));
        assert_eq!(push.apns_ready(), Ok(()));
        let tok = |c: &str| c.repeat(32);
        for (id, c) in ids.iter().zip(["a", "b", "c"]) {
            push.register_apns(id, &tok(c).to_uppercase(), None)
                .unwrap();
        }
        push.register_apns(&ids[0], &tok("d"), Some(ApnsEnv::Production))
            .unwrap();
        push.register_apns("unpaired", &tok("e"), None).unwrap();
        assert!(push.register_apns(&ids[0], "not hex", None).is_err());

        let log = push.deliver(None, &test_note()).unwrap();
        let sent: Vec<(String, String)> = got.try_iter().collect();
        let paths: Vec<&str> = sent
            .iter()
            .map(|(h, _)| h.split_whitespace().nth(1).unwrap())
            .collect();
        assert_eq!(
            paths,
            ["a", "b", "c", "d"].map(|c| format!("/3/device/{}", tok(c)))
        );
        let bearer = push.bearer(&apns(&url), &k, now());
        for (head, body) in &sent {
            assert!(head.starts_with("post "), "{head}");
            for h in [
                format!("authorization: bearer {}", bearer.to_ascii_lowercase()),
                "apns-topic: dev.cooee.app".to_owned(),
                "apns-push-type: alert".to_owned(),
                "apns-collapse-id: t1".to_owned(),
            ] {
                assert!(head.contains(&format!("{h}\r\n")), "{h} in {head}");
            }
            let body: serde_json::Value = serde_json::from_str(body).unwrap();
            assert_eq!(
                body["aps"]["alert"],
                serde_json::json!({"title": "Fix the build", "body": "needs you"})
            );
            assert_eq!(
                (&body["aps"]["thread-id"], &body["url"]),
                (&"t1".into(), &"/t/t1".into())
            );
        }
        assert_eq!(push.endpoints(&ids[0]), [tok("a"), tok("d")]);
        assert!(push.endpoints(&ids[1]).is_empty() && push.endpoints(&ids[2]).is_empty());
        assert_eq!(push.endpoints("unpaired"), Vec::<String>::new());
        assert_eq!(log.len(), 1, "{log:?}");
        assert!(log[0].contains("HTTP 500"), "{log:?}");
        let key_b64 = STANDARD.encode(k.to_bytes());
        assert!(!log[0].contains(pem.lines().nth(1).unwrap()) && !log[0].contains(&key_b64));
    }

    #[test]
    fn without_the_key_or_apns_table_nothing_goes_to_apns() {
        let dir = dir("apns-none");
        let ids = paired(&dir, 1);
        let (url, got) = fake_apns(|_| (200, ""));
        let (no_table, push) = (Push::new(&dir), Push::new(&dir).with_apns(Some(apns(&url))));
        push.register_apns(&ids[0], &"a".repeat(64), None).unwrap();
        assert_eq!(
            no_table.apns_ready(),
            Err("no [apns] in config.toml".to_owned())
        );
        assert_eq!(no_table.deliver(None, &test_note()).unwrap().len(), 1);
        let why = push.apns_ready().unwrap_err();
        assert!(
            why.starts_with("no ") && why.ends_with("apns.p8 (Apple Developer › Keys, APNs)"),
            "{why}"
        );
        assert_eq!(
            push.deliver(None, &test_note()).unwrap(),
            [format!(" to device {}: {why}", ids[0])]
        );

        let secret = pem("TOPSECRETNOTAKEY");
        let file = dir.join("apns.p8");
        std::fs::write(&file, secret).unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
        let log = push.deliver(None, &test_note()).unwrap().concat();
        assert_eq!(mode(&file), 0o600);
        assert!(
            log.contains("is not an APNs key (.p8)") && !log.contains("TOPSECRET"),
            "{log}"
        );
        assert_eq!(got.try_iter().count(), 0);
        assert_eq!(push.endpoints(&ids[0]), ["a".repeat(64)]);
    }

    #[test]
    fn a_renewed_vapid_key_keeps_apns_tokens() {
        let dir = dir("apns-renew");
        let push = Push::new(&dir);
        push.register_apns("a", &"a".repeat(64), None).unwrap();
        push.renew("the key changed").unwrap();
        assert_eq!(push.endpoints("a"), ["a".repeat(64)]);
        assert_eq!(push.reset("a"), None);
    }

    fn dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cooee-push-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn mode(path: &Path) -> u32 {
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
            agent: "codex".to_owned(),
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
                files: Vec::new(),
                routine: None,
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
                speaker: String::new(),
                body: Body::Approval {
                    tool: "Edit src/lib.rs".to_owned(),
                    options: Vec::new(),
                    tapped: tapped.map(str::to_owned),
                    chosen: None,
                    plugin: None,
                    card: cooee_core::Card::default(),
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
        let packs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs");
        let (looks, _) = cooee_web::Looks::load(Some(&packs), None);
        let (farm, plain) = (&looks.themes["farm"], &looks.themes["plain"]);
        assert_eq!(
            note(&t, &chat, &approval(None), "working", true, farm),
            Some(n("at the gate · needs you · codex: Edit src/lib.rs", 3))
        );
        assert_eq!(
            note(&t, &chat, &approval(Some("allow")), "working", false, farm),
            None
        );
        assert_eq!(
            note(&t, &chat, &turn("done", None), "working", false, farm),
            Some(n("in the stable · done · codex: All tests pass.", 2))
        );
        assert_eq!(
            note(&t, &chat, &turn("done", None), "working", true, farm),
            None
        );
        assert_eq!(
            note(&t, &chat, &turn("done", None), "done", false, farm),
            None
        );
        assert_eq!(
            note(
                &t,
                &chat,
                &turn("error", Some("rate limited")),
                "working",
                true,
                plain
            ),
            Some(n("failed · codex: rate limited", 2))
        );
        assert_eq!(
            note(&t, &chat, &turn("working", None), "done", false, farm),
            None
        );
        assert_eq!(
            note(
                &t,
                &chat,
                &Change::Text("x".to_owned()),
                "working",
                false,
                farm
            ),
            None
        );
        assert_eq!(
            serde_json::to_value(n("x", 2)).unwrap(),
            serde_json::json!({"title": "Fix the build", "body": "x", "tag": "t1", "url": "/t/t1", "item": 2})
        );
    }
}
