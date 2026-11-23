use std::{
    collections::HashMap,
    sync::{Arc, Mutex, PoisonError},
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use cooee_core::{Claim, Devices};
use p256::ecdsa::{Signature, VerifyingKey, signature::Verifier};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use topcoat::{
    Result,
    context::{Cx, app_context},
};

use crate::{
    Note,
    http::{COOKIE, Data, header, hub, now},
};

pub(crate) const CHALLENGE_TTL: u64 = 5 * 60 * 1000;

const TRIES: usize = 5;

const CLAIM: &str = "cooee_claim";

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

pub(crate) async fn store<T: Send + 'static>(
    cx: &Cx,
    f: impl FnOnce(&Devices) -> std::io::Result<T> + Send + 'static,
) -> Result<T> {
    let devices = devices(cx);
    Ok(tokio::task::spawn_blocking(move || f(&devices)).await??)
}

pub(crate) fn host(cx: &Cx) -> &str {
    header(cx, "host").unwrap_or_default()
}

pub(crate) fn rp_id(host: &str) -> &str {
    host.split(':').next().unwrap_or(host)
}

fn set(cx: &Cx, name: &str, value: &str, age: u32) -> String {
    let secure = header(cx, "origin").is_some_and(|o| o.starts_with("https://"));
    format!(
        "{name}={value}; Path=/; Max-Age={age}; HttpOnly; SameSite=Strict{}",
        if secure { "; Secure" } else { "" }
    )
}

pub(crate) fn cookie(cx: &Cx, token: &str) -> String {
    set(cx, COOKIE, token, 31_536_000)
}

pub(crate) fn claim_cookie(cx: &Cx, claim: Option<&str>) -> String {
    let (claim, age) = claim.map_or(("", 0), |c| (c, 300));
    set(cx, CLAIM, claim, age)
}

pub(crate) fn jar<'a>(cx: &'a Cx, name: &str) -> Option<&'a str> {
    header(cx, "cookie")?
        .split(';')
        .find_map(|c| c.trim().strip_prefix(name)?.strip_prefix('='))
}

pub(crate) fn claim(cx: &Cx) -> Option<String> {
    jar(cx, CLAIM).filter(|c| !c.is_empty()).map(str::to_owned)
}

fn client(cx: &Cx) -> String {
    header(cx, "x-forwarded-for")
        .and_then(|f| f.rsplit(',').next())
        .map_or("local", str::trim)
        .to_owned()
}

pub(crate) enum Tried {
    Paired(String),
    Wrong { left: usize },
    Expired,
    Limited { wait: u64 },
}

pub(crate) async fn try_code(cx: &Cx, code: &str) -> Result<Tried> {
    let (who, t) = (client(cx), now());
    let limited = {
        let mut misses = pairing(cx)
            .misses
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        misses.retain(|_, m| {
            m.retain(|at| at + 60_000 > t);
            !m.is_empty()
        });
        let mine = misses.get(&who).map_or(&[][..], Vec::as_slice);
        (mine.len() >= TRIES)
            .then(|| {
                mine.first()
                    .map(|first| (first + 60_000 - t).div_ceil(1000))
            })
            .flatten()
    };
    if let Some(wait) = limited {
        return Ok(Tried::Limited { wait });
    }
    let agent = header(cx, "user-agent").unwrap_or_default().to_owned();
    let tried = code.to_owned();
    let (claimed, open) = store(cx, move |d| {
        let open = d.code_open(t)?;
        Ok((d.claim(&tried, &agent, t)?, open))
    })
    .await?;
    if let Some(token) = claimed {
        return Ok(Tried::Paired(token));
    }
    if !open {
        return Ok(Tried::Expired);
    }
    let left = {
        let mut misses = pairing(cx)
            .misses
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let mine = misses.entry(who).or_default();
        mine.push(t);
        TRIES.saturating_sub(mine.len())
    };
    Ok(if left == 0 {
        Tried::Limited { wait: 60 }
    } else {
        Tried::Wrong { left }
    })
}

pub(crate) async fn ask(cx: &Cx) -> Result<(String, String)> {
    let agent = header(cx, "user-agent").unwrap_or_default().to_owned();
    let label = cooee_core::label(&agent);
    let (word, claim) = store(cx, move |d| d.ask(&agent, now())).await?;
    hub(cx).push().send(&Note {
        title: "Pair a new device?".to_owned(),
        body: format!("{label} · check word {word}"),
        tag: "pair".to_owned(),
        url: "/".to_owned(),
        item: 0,
        say: false,
    });
    Ok((word, claim))
}

pub(crate) async fn collect(cx: &Cx, claim: String) -> Result<Claim> {
    store(cx, move |d| d.collect(&claim, now())).await
}

pub(crate) async fn sign_in(cx: &Cx, a: Assertion) -> Result<Result<String, &'static str>> {
    let id = a.id.clone();
    if let Err(why) = assert(cx, a).await? {
        return Ok(Err(why));
    }
    let agent = header(cx, "user-agent").unwrap_or_default().to_owned();
    Ok(Ok(store(cx, move |d| d.restore(&agent, &id, now())).await?))
}

#[derive(Deserialize)]
pub(crate) struct Assertion {
    pub(crate) id: String,
    client: String,
    auth: String,
    sig: String,
}

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

#[derive(Deserialize)]
struct ClientData {
    #[serde(rename = "type")]
    kind: String,
    challenge: String,
    origin: String,
}

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
    if auth.len() < 37
        || auth.get(..32) != Some(&hash[..])
        || auth.get(32).is_none_or(|f| f & 0x05 != 0x05)
    {
        return Err("That passkey wasn't checked with Face ID or a passcode.");
    }
    Ok(())
}

pub(crate) fn spki_point(spki: &[u8]) -> Option<&[u8]> {
    const P256: [u8; 26] = [
        0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08,
        0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
    ];
    spki.strip_prefix(&P256[..]).filter(|p| p.len() == 65)
}

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
