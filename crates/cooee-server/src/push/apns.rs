use std::{io, sync::PoisonError};

use cooee_core::{Apns, ApnsEnv};
use p256::SecretKey;

use super::{
    Note, Push, Sub, apns_jwt,
    crypto::p8_key,
    key::read_private,
    send::{curl, now},
};

impl Push {
    /// This push state sending to APNs tokens too, with `apns` (none: it sends to none).
    #[must_use]
    pub fn with_apns(mut self, apns: Option<Apns>) -> Self {
        self.apns = apns;
        self
    }

    pub(super) fn apns_key(&self) -> Result<(&Apns, SecretKey), String> {
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

    pub(super) fn apns_post(
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

#[cfg(test)]
mod tests {
    use std::{io::Write, os::unix::fs::PermissionsExt, path::Path, thread};

    use base64::{
        Engine,
        engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD as B64},
    };
    use cooee_core::{Devices, write_private};
    use p256::ecdsa::{Signature, SigningKey};

    use super::*;
    use crate::push::{
        crypto::{new_key, public},
        key::tests::{dir, mode},
        note::tests::test_note,
    };

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
}
