use std::{
    io::{self, Write},
    process::Stdio,
    sync::PoisonError,
    thread,
    time::UNIX_EPOCH,
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use cooee_core::{Devices, random};
use p256::SecretKey;

use super::{
    Note, Push, Sub,
    crypto::{b64, encrypt, jwt, new_key, public},
    subs::LOCK,
};

impl Push {
    /// Sends `note` to every paired device's subscriptions and APNs tokens (only `to`'s, if set), in the
    /// background (its handle): a 404/410, APNs' `BadDeviceToken` or an unpaired device drops it; other failures are logged.
    pub fn send_to(&self, to: Option<&str>, note: &Note) -> thread::JoinHandle<()> {
        let (push, to, note) = (self.clone(), to.map(str::to_owned), note.clone());
        thread::spawn(move || match push.deliver(to.as_deref(), &note) {
            Ok(log) => log.iter().for_each(|l| cooee_core::note!("cooee: push{l}")),
            Err(e) => cooee_core::note!("cooee: push: {e}"),
        })
    }

    /// [`Push::send_to`] every device.
    pub fn send(&self, note: &Note) {
        self.send_to(None, note);
    }

    pub(super) fn deliver(&self, to: Option<&str>, note: &Note) -> io::Result<Vec<String>> {
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
}

pub(super) fn now() -> u64 {
    cooee_core::clock::now()
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

pub(super) fn curl<'a>(
    args: impl IntoIterator<Item = &'a str>,
    body: &[u8],
) -> io::Result<(u16, String)> {
    let mut c = cooee_core::runner::command("curl")
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
    use super::*;
    use crate::push::note::tests::test_note;

    #[test]
    fn a_push_to_one_device_reaches_only_its_endpoint() {
        use std::{net::TcpListener, sync::mpsc};
        let dir = cooee_core::scratch::owned(
            std::env::temp_dir().join(format!("cooee-push-to-{}", std::process::id())),
        );
        std::fs::remove_dir_all(&dir).ok();
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
}
