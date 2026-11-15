//! Pairing (#47): the page, a code typed or scanned (single use, 10 minutes, 5 wrong tries a minute per address), a
//! new device asking a paired one to approve it (a check word, a push to every paired device, approval needs a
//! passkey), the QR a paired device shows, and passkeys (`WebAuthn` ES256, platform authenticators): made on a paired
//! device, they sign a device back in and approve new ones. The server keeps only their public keys.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, PoisonError},
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use cooee_core::Devices;
use p256::ecdsa::{Signature, VerifyingKey, signature::Verifier};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use topcoat::{
    Result,
    context::{Cx, app_context},
};

use crate::http::{COOKIE, Data, header, now};

/// How long a passkey challenge is good for, in milliseconds.
pub(crate) const CHALLENGE_TTL: u64 = 5 * 60 * 1000;

/// The in-memory pairing state: each address's wrong codes in the last minute and the open passkey challenges.
#[derive(Default)]
pub(crate) struct Pairing {
    pub(crate) misses: Mutex<HashMap<String, Vec<u64>>>,
    pub(crate) challenges: Mutex<Vec<(String, u64)>>,
}

pub(crate) fn pairing(cx: &Cx) -> &Arc<Pairing> {
    app_context::<Arc<Pairing>>(cx)
}

pub(crate) fn devices(cx: &Cx) -> Devices {
    Devices::new(&app_context::<Data>(cx).0)
}

/// Runs a blocking store call off the async workers.
pub(crate) async fn store<T: Send + 'static>(
    cx: &Cx,
    f: impl FnOnce(&Devices) -> std::io::Result<T> + Send + 'static,
) -> Result<T> {
    let devices = devices(cx);
    Ok(tokio::task::spawn_blocking(move || f(&devices)).await??)
}

/// The request's host name (its `Host` without the port): the passkeys' relying party.
pub(crate) fn host(cx: &Cx) -> &str {
    header(cx, "host").unwrap_or_default()
}

pub(crate) fn rp_id(host: &str) -> &str {
    host.split(':').next().unwrap_or(host)
}

/// The pairing cookie for `token` (`Secure` behind https).
pub(crate) fn cookie(cx: &Cx, token: &str) -> String {
    let secure = header(cx, "origin").is_some_and(|o| o.starts_with("https://"));
    format!(
        "{COOKIE}={token}; Path=/; Max-Age=31536000; HttpOnly; SameSite=Strict{}",
        if secure { "; Secure" } else { "" }
    )
}

/// A passkey's signature (`navigator.credentials.get`), base64url.
#[derive(Deserialize)]
pub(crate) struct Assertion {
    pub(crate) id: String,
    client: String,
    auth: String,
    sig: String,
}

/// Takes `challenge` if this server gave it out in the last 5 minutes: each is good once.
pub(crate) fn take(cx: &Cx, challenge: &str) -> bool {
    let mut all = pairing(cx)
        .challenges
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let t = now();
    let found = all
        .iter()
        .position(|(c, at)| c == challenge && at + CHALLENGE_TTL > t);
    found.map(|i| all.remove(i)).is_some()
}

/// What a ceremony's client data says.
#[derive(Deserialize)]
struct ClientData {
    #[serde(rename = "type")]
    kind: String,
    challenge: String,
    origin: String,
}

/// Checks a ceremony of `kind` against this server: its challenge (taken), its origin, the relying party's hash and
/// the user's presence and verification (Face ID) in the authenticator data.
pub(crate) fn ceremony(
    cx: &Cx,
    kind: &str,
    client: &[u8],
    auth: &[u8],
) -> Result<(), &'static str> {
    let c: ClientData =
        serde_json::from_slice(client).map_err(|_| "That passkey answer isn't readable.")?;
    let host = host(cx);
    if c.kind != kind || !take(cx, &c.challenge) {
        return Err("That passkey request ran out: try again.");
    }
    if c.origin != format!("https://{host}") && c.origin != format!("http://{host}") {
        return Err("That passkey is for another site.");
    }
    let hash = Sha256::digest(rp_id(host).as_bytes());
    if auth.len() < 37 || auth[..32] != hash[..] || auth[32] & 0x05 != 0x05 {
        return Err("That passkey wasn't checked with Face ID or a passcode.");
    }
    Ok(())
}

/// The P-256 point in a public key's `SubjectPublicKeyInfo` (DER), as `getPublicKey()` gives it.
pub(crate) fn spki_point(spki: &[u8]) -> Option<&[u8]> {
    const P256: [u8; 26] = [
        0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08,
        0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
    ];
    spki.strip_prefix(&P256[..]).filter(|p| p.len() == 65)
}

/// Whether `sig` is `key`'s signature over the authenticator data and the client data's hash.
fn signed(key: &[u8], auth: &[u8], client: &[u8], sig: &[u8]) -> bool {
    let (Ok(key), Ok(sig)) = (VerifyingKey::from_sec1_bytes(key), Signature::from_der(sig)) else {
        return false;
    };
    let mut msg = auth.to_vec();
    msg.extend_from_slice(&Sha256::digest(client));
    key.verify(&msg, &sig).is_ok()
}

pub(crate) fn b64(s: &str) -> Vec<u8> {
    B64.decode(s.trim_end_matches('=')).unwrap_or_default()
}

/// Checks a passkey's assertion: a known passkey signed this server's challenge with Face ID; marks it used.
pub(crate) async fn assert(cx: &Cx, a: Assertion) -> Result<Result<(), &'static str>> {
    let (client, auth) = (b64(&a.client), b64(&a.auth));
    if let Err(why) = ceremony(cx, "webauthn.get", &client, &auth) {
        return Ok(Err(why));
    }
    let id = a.id.clone();
    let found = store(cx, move |d| {
        Ok(d.passkeys()?.into_iter().find(|k| k.id == id))
    })
    .await?;
    let Some(key) = found else {
        return Ok(Err(
            "No passkey for this cooee on this device. Pair with a code instead.",
        ));
    };
    if !signed(&b64(&key.key), &auth, &client, &b64(&a.sig)) {
        return Ok(Err("That passkey's signature doesn't check out."));
    }
    store(cx, move |d| d.used_passkey(&key.id, now())).await?;
    Ok(Ok(()))
}

#[cfg(test)]
mod tests {
    use p256::{SecretKey, ecdsa::SigningKey};

    use super::*;

    #[test]
    fn a_der_signature_over_the_data_checks_out_and_a_changed_one_does_not() {
        let key = SigningKey::from(SecretKey::from_slice(&[7; 32]).unwrap());
        let point = key.verifying_key().to_sec1_point(false).as_bytes().to_vec();
        let (auth, client) = (b"auth-data".to_vec(), b"{\"client\":1}".to_vec());
        let mut msg = auth.clone();
        msg.extend_from_slice(&Sha256::digest(&client));
        let sig: Signature = p256::ecdsa::signature::Signer::sign(&key, &msg);
        let der = sig.to_der();
        assert!(signed(&point, &auth, &client, der.as_bytes()));
        assert!(!signed(&point, b"other", &client, der.as_bytes()));
        assert!(!signed(&point, &auth, &client, &der.as_bytes()[1..]));
        let mut spki = vec![
            0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01,
        ];
        spki.extend([
            0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
        ]);
        spki.extend(&point);
        assert_eq!(spki_point(&spki), Some(&point[..]));
        assert_eq!(spki_point(&spki[1..]), None);
    }
}
