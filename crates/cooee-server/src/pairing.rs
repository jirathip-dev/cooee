//! Pairing (#47): the page, a code typed or scanned (single use, 10 minutes, 5 wrong tries a minute per address), a
//! new device asking a paired one to approve it (a check word, a push to every paired device, approval needs a
//! passkey), the QR a paired device shows, and passkeys (`WebAuthn` ES256, platform authenticators): made on a paired
//! device, they sign a device back in and approve new ones. The server keeps only their public keys.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, PoisonError},
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use cooee_core::{Claim, Credential, Devices, random};
use p256::ecdsa::{Signature, VerifyingKey, signature::Verifier};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use topcoat::{
    Result,
    context::{Cx, app_context, request_context},
    router::{
        Body,
        content::{Form, Json},
        response::{IntoResponse, Response},
        route,
    },
};

use super::{COOKIE, Data, Note, Paired, header, hub, now, plain};

/// Wrong codes an address may type a minute.
const TRIES: usize = 5;

/// How long a passkey challenge is good for, in milliseconds.
const CHALLENGE_TTL: u64 = 5 * 60 * 1000;

/// The in-memory pairing state: each address's wrong codes in the last minute and the open passkey challenges.
#[derive(Default)]
pub(crate) struct Pairing {
    misses: Mutex<HashMap<String, Vec<u64>>>,
    challenges: Mutex<Vec<(String, u64)>>,
}

fn pairing(cx: &Cx) -> &Arc<Pairing> {
    app_context::<Arc<Pairing>>(cx)
}

pub(crate) fn devices(cx: &Cx) -> Devices {
    Devices::new(&app_context::<Data>(cx).0)
}

/// Runs a blocking store call off the async workers.
async fn store<T: Send + 'static>(
    cx: &Cx,
    f: impl FnOnce(&Devices) -> std::io::Result<T> + Send + 'static,
) -> Result<T> {
    let devices = devices(cx);
    Ok(tokio::task::spawn_blocking(move || f(&devices)).await??)
}

/// Who is typing: the address `tailscale serve` forwards for (the last `X-Forwarded-For` entry), else this machine.
fn client(cx: &Cx) -> String {
    header(cx, "x-forwarded-for")
        .and_then(|f| f.rsplit(',').next())
        .map_or("local", str::trim)
        .to_owned()
}

/// The request's host name (its `Host` without the port): the passkeys' relying party.
fn host(cx: &Cx) -> &str {
    header(cx, "host").unwrap_or_default()
}

fn rp_id(host: &str) -> &str {
    host.split(':').next().unwrap_or(host)
}

fn see_other(to: &str, cookie: Option<String>) -> Result<Response> {
    let mut r = Response::builder().status(303).header("Location", to);
    if let Some(c) = cookie {
        r = r.header("Set-Cookie", c);
    }
    Ok(r.body(Body::from(()))?)
}

/// The pairing cookie for `token` (`Secure` behind https).
pub(crate) fn cookie(cx: &Cx, token: &str) -> String {
    let secure = header(cx, "origin").is_some_and(|o| o.starts_with("https://"));
    format!(
        "{COOKIE}={token}; Path=/; Max-Age=31536000; HttpOnly; SameSite=Strict{}",
        if secure { "; Secure" } else { "" }
    )
}

/// `POST /pair`: the code `cooee pair` or a paired device shows, as one field or the six boxes.
#[derive(Deserialize)]
struct PairForm {
    code: String,
    #[serde(default)]
    d2: String,
    #[serde(default)]
    d3: String,
    #[serde(default)]
    d4: String,
    #[serde(default)]
    d5: String,
    #[serde(default)]
    d6: String,
}

/// Pairs on the right code (then to the list, which offers a passkey); else back to the page saying why.
#[route(POST "/pair")]
async fn pair(cx: &Cx, Form(f): Form<PairForm>) -> Result<Response> {
    let code: String = [&f.code, &f.d2, &f.d3, &f.d4, &f.d5, &f.d6]
        .iter()
        .flat_map(|d| d.chars().filter(char::is_ascii_digit))
        .collect();
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
        (mine.len() >= TRIES).then(|| (mine[0] + 60_000 - t).div_ceil(1000))
    };
    if let Some(wait) = limited {
        return see_other(&format!("/pair?bad=limit&wait={wait}"), None);
    }
    let agent = header(cx, "user-agent").unwrap_or_default().to_owned();
    let tried = code.clone();
    let (claimed, open) = store(cx, move |d| {
        let open = d.code_open(t)?;
        Ok((d.claim(&tried, &agent, t)?, open))
    })
    .await?;
    if let Some(token) = claimed {
        return see_other("/?paired", Some(cookie(cx, &token)));
    }
    if !open {
        return see_other(&format!("/pair?bad=expired&code={code}"), None);
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
    if left == 0 {
        return see_other("/pair?bad=limit&wait=60", None);
    }
    see_other(&format!("/pair?bad=wrong&left={left}"), None)
}

/// `POST /pair/request`: a new device asks to pair. Every paired device gets a push and the card; the new device
/// shows the check word and collects its cookie with the claim.
#[route(POST "/pair/request")]
async fn pair_request(cx: &Cx) -> Result<Response> {
    let agent = header(cx, "user-agent").unwrap_or_default().to_owned();
    let label = cooee_core::label(&agent);
    let (word, claim) = store(cx, move |d| d.ask(&agent, now())).await?;
    hub(cx).push().send(&Note {
        title: "Pair a new device?".to_owned(),
        body: format!("{label} · check word {word}"),
        tag: "pair".to_owned(),
        url: "/".to_owned(),
        item: 0,
    });
    Json(serde_json::json!({ "word": word, "claim": claim })).into_response(cx)
}

/// The new device's claim on its request.
#[derive(Deserialize)]
struct ClaimBody {
    claim: String,
}

/// `POST /pair/claim`: 200 with the cookie once approved, 202 while it waits, 403 declined, 410 when it is gone.
#[route(POST "/pair/claim")]
async fn pair_claim(cx: &Cx, Json(c): Json<ClaimBody>) -> Result<Response> {
    match store(cx, move |d| d.collect(&c.claim, now())).await? {
        Claim::Paired(token) => Ok(Response::builder()
            .header("Set-Cookie", cookie(cx, &token))
            .body(Body::from(()))?),
        Claim::Waiting => plain(202, "waiting for approval"),
        Claim::Declined(by) => plain(403, &format!("{by} said no.")),
        Claim::Gone => plain(410, "The request ran out: ask again."),
    }
}

#[route(POST "/pair/cancel")]
async fn pair_cancel(cx: &Cx, Json(c): Json<ClaimBody>) -> Result<Response> {
    store(cx, move |d| d.cancel(&c.claim)).await?;
    plain(200, "cancelled")
}

/// `GET /pair/asks`: the requests waiting for an answer (the cards on a paired device).
#[route(GET "/pair/asks")]
async fn pair_asks(cx: &Cx) -> Result<Response> {
    Json(store(cx, |d| d.asking(now())).await?).into_response(cx)
}

/// A passkey's signature (`navigator.credentials.get`), base64url.
#[derive(Deserialize)]
pub(crate) struct Assertion {
    id: String,
    client: String,
    auth: String,
    sig: String,
}

/// An answer to a request: approving needs a passkey's assertion.
#[derive(Deserialize)]
struct Answer {
    id: String,
    yes: bool,
    passkey: Option<Assertion>,
}

/// `POST /pair/answer`: approves (with a passkey) or declines a waiting request.
#[route(POST "/pair/answer")]
async fn pair_answer(cx: &Cx, Json(a): Json<Answer>) -> Result<Response> {
    if a.yes {
        let Some(p) = a.passkey else {
            return plain(400, "Approving needs a passkey.");
        };
        if let Err(why) = assert(cx, p).await? {
            return plain(403, why);
        }
    }
    let by = request_context::<Paired>(cx).0.label.clone();
    if store(cx, move |d| d.answer(&a.id, a.yes, &by, now())).await? {
        plain(200, if a.yes { "approved" } else { "declined" })
    } else {
        plain(410, "That request ran out or was answered already.")
    }
}

/// `POST /devices/pair`: a new code (dropping the open one), its link and the link as a QR code (SVG).
#[route(POST "/devices/pair")]
async fn devices_pair(cx: &Cx) -> Result<Response> {
    let code = store(cx, |d| d.pair(now())).await?;
    let base = header(cx, "origin").map_or_else(|| format!("https://{}", host(cx)), str::to_owned);
    let link = format!("{base}/pair?code={code}");
    let svg = qrcode::QrCode::new(&link).map(|q| {
        q.render::<qrcode::render::svg::Color>()
            .min_dimensions(200, 200)
            .build()
    });
    Json(serde_json::json!({
        "code": code, "link": link, "svg": svg.unwrap_or_default(), "host": rp_id(host(cx)),
    }))
    .into_response(cx)
}

/// `POST /passkey/challenge`: a challenge for one passkey ceremony, good for 5 minutes, and the passkeys' ids.
#[route(POST "/passkey/challenge")]
async fn passkey_challenge(cx: &Cx) -> Result<Response> {
    let c = B64.encode(random(32)?);
    {
        let mut all = pairing(cx)
            .challenges
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let t = now();
        all.retain(|(_, at)| at + CHALLENGE_TTL > t);
        if all.len() >= 32 {
            all.remove(0);
        }
        all.push((c.clone(), t));
    }
    let ids: Vec<String> = store(cx, Devices::passkeys)
        .await?
        .into_iter()
        .map(|k| k.id)
        .collect();
    Json(serde_json::json!({ "challenge": c, "ids": ids })).into_response(cx)
}

/// Takes `challenge` if this server gave it out in the last 5 minutes: each is good once.
fn take(cx: &Cx, challenge: &str) -> bool {
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
fn ceremony(cx: &Cx, kind: &str, client: &[u8], auth: &[u8]) -> Result<(), &'static str> {
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
fn spki_point(spki: &[u8]) -> Option<&[u8]> {
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

fn b64(s: &str) -> Vec<u8> {
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

/// A new passkey (`navigator.credentials.create`), base64url: its id, its public key (SPKI) and what it signed.
#[derive(Deserialize)]
struct NewPasskey {
    id: String,
    key: String,
    client: String,
    auth: String,
}

/// `POST /passkeys`: keeps this device's new passkey.
#[route(POST "/passkeys")]
async fn passkey_add(cx: &Cx, Json(p): Json<NewPasskey>) -> Result<Response> {
    let (client, auth, spki) = (b64(&p.client), b64(&p.auth), b64(&p.key));
    if let Err(why) = ceremony(cx, "webauthn.create", &client, &auth) {
        return plain(400, why);
    }
    let Some(point) = spki_point(&spki).filter(|p| VerifyingKey::from_sec1_bytes(p).is_ok()) else {
        return plain(400, "That passkey isn't a P-256 key.");
    };
    if p.id.is_empty() || p.id.len() > 512 {
        return plain(400, "That passkey has no id.");
    }
    let key = Credential {
        id: p.id,
        key: B64.encode(point),
        label: request_context::<Paired>(cx).0.label.clone(),
        device: request_context::<Paired>(cx).0.id.clone(),
        created: now(),
        used: 0,
    };
    store(cx, move |d| d.add_passkey(key)).await?;
    plain(200, "passkey saved")
}

/// `GET /passkeys`: every passkey: its id, the device that made it (and whether that is this one), when it was made
/// and last used.
#[route(GET "/passkeys")]
async fn passkey_list(cx: &Cx) -> Result<Response> {
    let me = request_context::<Paired>(cx).0.id.clone();
    let list: Vec<_> = store(cx, Devices::passkeys)
        .await?
        .into_iter()
        .map(|k| serde_json::json!({ "id": k.id, "label": k.label, "created": k.created, "used": k.used, "mine": k.device == me }))
        .collect();
    Json(list).into_response(cx)
}

/// A passkey to remove.
#[derive(Deserialize)]
struct PasskeyId {
    id: String,
}

#[route(POST "/passkeys/remove")]
async fn passkey_remove(cx: &Cx, Json(p): Json<PasskeyId>) -> Result<Response> {
    if store(cx, move |d| d.remove_passkey(&p.id)).await? {
        plain(200, "passkey removed")
    } else {
        plain(404, "No such passkey.")
    }
}

/// `POST /passkey/signin`: a device that lost its cookie signs back in with its passkey: paired again.
#[route(POST "/passkey/signin")]
async fn passkey_signin(cx: &Cx, Json(a): Json<Assertion>) -> Result<Response> {
    let id = a.id.clone();
    if let Err(why) = assert(cx, a).await? {
        return plain(403, why);
    }
    let agent = header(cx, "user-agent").unwrap_or_default().to_owned();
    let token = store(cx, move |d| d.restore(&agent, &id, now())).await?;
    Ok(Response::builder()
        .header("Set-Cookie", cookie(cx, &token))
        .body(Body::from(()))?)
}

/// The pairing routes.
pub(crate) fn routes(b: topcoat::router::RouterBuilder) -> topcoat::router::RouterBuilder {
    b.route(pair)
        .route(pair_request)
        .route(pair_claim)
        .route(pair_cancel)
        .route(pair_asks)
        .route(pair_answer)
        .route(devices_pair)
        .route(passkey_challenge)
        .route(passkey_add)
        .route(passkey_list)
        .route(passkey_remove)
        .route(passkey_signin)
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
