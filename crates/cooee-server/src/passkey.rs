use std::{
    collections::HashMap,
    sync::{Mutex, PoisonError},
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use cooee_core::Passkey;
use p256::ecdsa::{Signature, VerifyingKey, signature::Verifier};
use serde::Deserialize;
use sha2::{Digest, Sha256};

/// The plugin behind the gate.
pub const GATED: &str = "terminal";

/// An unlock ends after this long without a call, in milliseconds: 10 minutes.
pub const IDLE: u64 = 10 * 60 * 1000;

const CHALLENGE_TTL: u64 = 2 * 60 * 1000;

const SPKI_P256: [u8; 26] = [
    0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08, 0x2a,
    0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
];

/// What the page sends after `navigator.credentials`: base64url fields. `key` (SPKI) on registration,
/// `sig` (DER) on unlock.
#[derive(Debug, Deserialize)]
pub struct Proof {
    pub id: String,
    pub client: String,
    pub auth: String,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub sig: Option<String>,
}

/// Each device's open challenge and its unlock (the time of its last call), in memory: a restart locks every tab.
#[derive(Default)]
pub struct Unlocks {
    challenges: Mutex<HashMap<String, (Vec<u8>, u64)>>,
    used: Mutex<HashMap<String, u64>>,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Unlocks {
    /// A new challenge for `device`, replacing its open one.
    pub fn challenge(&self, device: &str, now: u64) -> std::io::Result<Vec<u8>> {
        let c = cooee_core::random(32)?;
        lock(&self.challenges).insert(device.to_owned(), (c.clone(), now));
        Ok(c)
    }

    fn take(&self, device: &str, now: u64) -> Option<Vec<u8>> {
        let (c, at) = lock(&self.challenges).remove(device)?;
        (at + CHALLENGE_TTL > now).then_some(c)
    }

    /// Unlocks `device` now.
    pub fn unlock(&self, device: &str, now: u64) {
        lock(&self.used).insert(device.to_owned(), now);
    }

    /// Whether `device` is unlocked: its last call was under [`IDLE`] ago.
    pub fn open(&self, device: &str, now: u64) -> bool {
        lock(&self.used)
            .get(device)
            .is_some_and(|&at| at + IDLE > now)
    }

    /// Whether `device` is unlocked; if so this call counts as use.
    pub fn touch(&self, device: &str, now: u64) -> bool {
        let mut used = lock(&self.used);
        match used.get_mut(device) {
            Some(at) if *at + IDLE > now => {
                *at = now;
                true
            }
            _ => false,
        }
    }

    /// Checks a registration for `device` from page host `host`: the new passkey, or why not.
    pub fn register(
        &self,
        device: &str,
        host: &str,
        p: &Proof,
        now: u64,
    ) -> Result<Passkey, String> {
        let challenge = self
            .take(device, now)
            .ok_or("the challenge expired: try again")?;
        let auth = decode(&p.auth)?;
        ceremony(
            &decode(&p.client)?,
            &auth,
            "webauthn.create",
            &challenge,
            host,
        )?;
        let spki = decode(p.key.as_deref().unwrap_or_default())?;
        let point = spki
            .strip_prefix(SPKI_P256.as_slice())
            .filter(|k| k.len() == 65 && VerifyingKey::from_sec1_bytes(k).is_ok())
            .ok_or("only a P-256 (ES256) passkey will do")?;
        decode(&p.id)?;
        Ok(Passkey {
            id: p.id.clone(),
            key: B64.encode(point),
        })
    }

    /// Checks an unlock for `device` (its passkey `key`) from page host `host`, then unlocks it.
    pub fn assert(
        &self,
        device: &str,
        key: &Passkey,
        host: &str,
        p: &Proof,
        now: u64,
    ) -> Result<(), String> {
        let challenge = self
            .take(device, now)
            .ok_or("the challenge expired: try again")?;
        if p.id != key.id {
            return Err("not this device's passkey".to_owned());
        }
        let (client, auth) = (decode(&p.client)?, decode(&p.auth)?);
        ceremony(&client, &auth, "webauthn.get", &challenge, host)?;
        let vk = VerifyingKey::from_sec1_bytes(&decode(&key.key)?).map_err(|e| e.to_string())?;
        let sig = der(&decode(p.sig.as_deref().unwrap_or_default())?)
            .and_then(|rs| Signature::from_slice(&rs).ok())
            .ok_or("a bad signature")?;
        let signed = [auth.as_slice(), &Sha256::digest(&client)].concat();
        let sig = sig.normalize_s();
        vk.verify(&signed, &sig)
            .map_err(|_| "the signature doesn't match")?;
        self.unlock(device, now);
        Ok(())
    }
}

fn decode(s: &str) -> Result<Vec<u8>, String> {
    B64.decode(s.trim_end_matches('='))
        .map_err(|_| "bad base64url".to_owned())
}

fn name(host: &str) -> &str {
    host.rsplit_once(':')
        .filter(|(_, port)| port.bytes().all(|b| b.is_ascii_digit()))
        .map_or(host, |(n, _)| n)
}

fn ceremony(
    client: &[u8],
    auth: &[u8],
    kind: &str,
    challenge: &[u8],
    host: &str,
) -> Result<(), String> {
    let c: serde_json::Value = serde_json::from_slice(client).map_err(|e| e.to_string())?;
    if c["type"] != kind || c["challenge"] != B64.encode(challenge) {
        return Err("not this challenge".to_owned());
    }
    let origin = c["origin"].as_str().unwrap_or_default();
    let on = origin.split_once("://").map_or("", |(_, h)| name(h));
    if on != name(host) {
        return Err(format!("an origin of {origin}"));
    }
    if auth.len() < 37 || auth[..32] != Sha256::digest(name(host).as_bytes())[..] {
        return Err("another site's passkey".to_owned());
    }
    if auth[32] & 0b101 != 0b101 {
        return Err("Face ID (user verification) is needed".to_owned());
    }
    Ok(())
}

fn der(sig: &[u8]) -> Option<[u8; 64]> {
    let (&len, mut rest) = sig.strip_prefix(&[0x30])?.split_first()?;
    if usize::from(len) != rest.len() {
        return None;
    }
    let mut out = [0; 64];
    for half in out.chunks_mut(32) {
        let (&n, r) = rest.strip_prefix(&[0x02])?.split_first()?;
        let (int, r) = r.split_at_checked(usize::from(n))?;
        rest = r;
        let int = &int[int.iter().take_while(|&&b| b == 0).count()..];
        half.get_mut(32usize.checked_sub(int.len())?..)?
            .copy_from_slice(int);
    }
    rest.is_empty().then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::ecdsa::{SigningKey, signature::Signer};

    const HOST: &str = "mac.tailnet.example:4747";
    const DEVICE: &str = "d1";

    fn key() -> SigningKey {
        SigningKey::from_slice(&[7; 32]).unwrap()
    }

    fn client(kind: &str, challenge: &[u8], origin: &str) -> String {
        let c = serde_json::json!({ "type": kind, "challenge": B64.encode(challenge), "origin": origin });
        B64.encode(c.to_string())
    }

    fn auth(rp: &str, flags: u8) -> Vec<u8> {
        [
            Sha256::digest(rp.as_bytes()).as_slice(),
            &[flags, 0, 0, 0, 0],
        ]
        .concat()
    }

    fn registration(u: &Unlocks, flags: u8, origin: &str) -> Result<Passkey, String> {
        let c = u.challenge(DEVICE, 0).unwrap();
        let point = key().verifying_key().to_sec1_point(false);
        let p = Proof {
            id: B64.encode(b"cred-1"),
            client: client("webauthn.create", &c, origin),
            auth: B64.encode(auth("mac.tailnet.example", flags)),
            key: Some(B64.encode([SPKI_P256.as_slice(), point.as_bytes()].concat())),
            sig: None,
        };
        u.register(DEVICE, HOST, &p, 1)
    }

    fn unlock(u: &Unlocks, pk: &Passkey, by: &SigningKey, at: u64, now: u64) -> Result<(), String> {
        let c = u.challenge(DEVICE, at).unwrap();
        let (cl, au) = (
            client("webauthn.get", &c, "https://mac.tailnet.example:4747"),
            auth("mac.tailnet.example", 0b101),
        );
        let signed = [au.as_slice(), &Sha256::digest(B64.decode(&cl).unwrap())].concat();
        let sig: Signature = by.sign(&signed);
        let p = Proof {
            id: pk.id.clone(),
            client: cl,
            auth: B64.encode(au),
            key: None,
            sig: Some(B64.encode(sig.to_der().as_bytes())),
        };
        u.assert(DEVICE, pk, HOST, &p, now)
    }

    #[test]
    fn registration_keeps_the_point_and_wants_face_id_and_this_origin() {
        let u = Unlocks::default();
        let pk = registration(&u, 0b101, "https://mac.tailnet.example:4747").unwrap();
        let point = key().verifying_key().to_sec1_point(false);
        assert_eq!(B64.decode(&pk.key).unwrap(), point.as_bytes());
        assert_eq!(
            registration(&u, 0b001, "https://mac.tailnet.example:4747").unwrap_err(),
            "Face ID (user verification) is needed"
        );
        assert_eq!(
            registration(&u, 0b101, "https://evil.example").unwrap_err(),
            "an origin of https://evil.example"
        );
    }

    #[test]
    fn an_unlock_needs_this_passkeys_signature_and_a_fresh_challenge() {
        let u = Unlocks::default();
        let pk = registration(&u, 0b101, "https://mac.tailnet.example:4747").unwrap();
        let other = SigningKey::from_slice(&[9; 32]).unwrap();
        assert_eq!(
            unlock(&u, &pk, &other, 0, 1).unwrap_err(),
            "the signature doesn't match"
        );
        assert!(!u.open(DEVICE, 1));
        assert_eq!(
            unlock(&u, &pk, &key(), 0, CHALLENGE_TTL).unwrap_err(),
            "the challenge expired: try again"
        );
        unlock(&u, &pk, &key(), 0, 1).unwrap();
        assert!(u.open(DEVICE, 1));
    }

    #[test]
    fn ten_idle_minutes_lock_and_face_id_unlocks_again() {
        let u = Unlocks::default();
        let pk = registration(&u, 0b101, "https://mac.tailnet.example:4747").unwrap();
        assert!(!u.touch(DEVICE, 0));
        unlock(&u, &pk, &key(), 0, 0).unwrap();
        assert!(u.touch(DEVICE, IDLE - 1));
        assert!(u.open(DEVICE, 2 * IDLE - 2));
        assert!(!u.touch(DEVICE, 2 * IDLE - 1));
        assert!(!u.open(DEVICE, 2 * IDLE));
        unlock(&u, &pk, &key(), 2 * IDLE, 2 * IDLE).unwrap();
        assert!(u.touch(DEVICE, 2 * IDLE + 1));
    }

    #[test]
    fn der_signatures_pad_and_trim_their_integers() {
        let sig = [0x30, 8, 0x02, 2, 0x00, 0x80, 0x02, 2, 0x01, 0x02];
        let rs = der(&sig).unwrap();
        assert_eq!((rs[31], rs[62], rs[63]), (0x80, 1, 2));
        assert!(der(&[0x30, 3, 0x02, 1, 1]).is_none());
    }
}
