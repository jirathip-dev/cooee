use std::sync::PoisonError;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use cooee_core::{Claim, Credential, Devices, random};
use p256::ecdsa::VerifyingKey;
use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, request_context},
    router::{
        Body, RouterBuilder,
        content::{Form, Json},
        response::{IntoResponse, Response},
        route,
    },
};

use crate::{
    Note,
    http::{Paired, header, hub, now, plain},
    pairing::{
        Assertion, CHALLENGE_TTL, assert, b64, ceremony, cookie, host, pairing, rp_id, spki_point,
        store,
    },
};

const TRIES: usize = 5;

fn client(cx: &Cx) -> String {
    header(cx, "x-forwarded-for")
        .and_then(|f| f.rsplit(',').next())
        .map_or("local", str::trim)
        .to_owned()
}

fn see_other(to: &str, cookie: Option<String>) -> Result<Response> {
    let mut r = Response::builder().status(303).header("Location", to);
    if let Some(c) = cookie {
        r = r.header("Set-Cookie", c);
    }
    Ok(r.body(Body::from(()))?)
}

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
        (mine.len() >= TRIES)
            .then(|| {
                mine.first()
                    .map(|first| (first + 60_000 - t).div_ceil(1000))
            })
            .flatten()
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

#[derive(Deserialize)]
struct ClaimBody {
    claim: String,
}

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

#[route(GET "/pair/asks")]
async fn pair_asks(cx: &Cx) -> Result<Response> {
    Json(store(cx, |d| d.asking(now())).await?).into_response(cx)
}

#[derive(Deserialize)]
struct Answer {
    id: String,
    yes: bool,
    passkey: Option<Assertion>,
}

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

#[derive(Deserialize)]
struct NewPasskey {
    id: String,
    key: String,
    client: String,
    auth: String,
}

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

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
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
