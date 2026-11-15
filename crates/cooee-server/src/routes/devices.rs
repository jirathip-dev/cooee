//! Paired devices and their passkeys.

use std::sync::Arc;

use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, app_context, request_context},
    router::{
        RouterBuilder,
        content::Json,
        error::not_found,
        path_param,
        response::{IntoResponse, Response},
        route,
    },
};

use crate::{
    http::{Data, Paired, Rest, header, now, plain},
    pairing::devices,
    passkey::{self, Unlocks},
};

/// `GET /devices`: the paired devices, this one marked `me`.
#[route(GET "/devices")]
async fn device_list(cx: &Cx) -> Result<Response> {
    let (devices, me) = (devices(cx), request_context::<Paired>(cx).0.id.clone());
    let list = tokio::task::spawn_blocking(move || devices.list()).await??;
    let list: Vec<_> = list
        .into_iter()
        .map(|d| {
            serde_json::json!({ "id": d.id, "label": d.label, "created": d.created, "used": d.used, "me": d.id == me, "by": d.by })
        })
        .collect();
    Json(list).into_response(cx)
}

/// A device to unpair.
#[derive(Deserialize)]
struct DeviceId {
    id: String,
}

/// `POST /devices/remove`: unpairs a device (its pushes stop too).
#[route(POST "/devices/remove")]
async fn device_remove(cx: &Cx, Json(d): Json<DeviceId>) -> Result<Response> {
    let devices = devices(cx);
    let id = d.id.clone();
    if tokio::task::spawn_blocking(move || devices.remove(&id)).await?? {
        plain(200, &format!("removed {}", d.id))
    } else {
        plain(404, &format!("No device {}.", d.id))
    }
}

/// A device's new name.
#[derive(Deserialize)]
struct Rename {
    id: String,
    label: String,
}

/// `POST /devices/rename`: names a device (or the one an approved request becomes).
#[route(POST "/devices/rename")]
async fn device_rename(cx: &Cx, Json(r): Json<Rename>) -> Result<Response> {
    let (devices, label) = (
        devices(cx),
        r.label.trim().chars().take(40).collect::<String>(),
    );
    if label.is_empty() {
        return plain(400, "Give it a name.");
    }
    let to = label.clone();
    if tokio::task::spawn_blocking(move || devices.rename(&r.id, &to)).await?? {
        plain(200, &label)
    } else {
        plain(404, "That device isn't paired any more.")
    }
}

/// `POST /passkey/options`: a challenge for this device's passkey ceremony, and its passkey's id once it has one
/// (none: the page registers one).
#[route(POST "/passkey/options")]
async fn passkey_options(cx: &Cx) -> Result<Response> {
    let me = &request_context::<Paired>(cx).0;
    let challenge = app_context::<Arc<Unlocks>>(cx).challenge(&me.id, now())?;
    let id = me.passkey.as_ref().map(|p| p.id.clone());
    let challenge = base64::Engine::encode(&base64::prelude::BASE64_URL_SAFE_NO_PAD, challenge);
    Json(serde_json::json!({ "challenge": challenge, "id": id })).into_response(cx)
}

/// `POST /passkey/register` (the device's first passkey) and `/passkey/unlock`: either unlocks the gated tab for
/// [`passkey::IDLE`]; each unlock goes to `<data>/terminal.log` with the time and device.
#[route(POST "/passkey/{rest}")]
async fn passkey_check(cx: &Cx, Json(p): Json<passkey::Proof>) -> Result<Response> {
    let me = request_context::<Paired>(cx).0.clone();
    let (unlocks, host) = (
        app_context::<Arc<Unlocks>>(cx),
        header(cx, "host").unwrap_or_default(),
    );
    let checked = match (path_param::<Rest>(cx), &me.passkey) {
        ("register", None) => match unlocks.register(&me.id, host, &p, now()) {
            Ok(pk) => {
                let (devices, id) = (devices(cx), me.id.clone());
                if tokio::task::spawn_blocking(move || devices.set_passkey(&id, pk)).await?? {
                    unlocks.unlock(&me.id, now());
                    Ok(())
                } else {
                    Err("this device has a passkey already".to_owned())
                }
            }
            Err(e) => Err(e),
        },
        ("unlock", Some(pk)) => unlocks.assert(&me.id, pk, host, &p, now()),
        ("register", Some(_)) => Err("this device has a passkey already".to_owned()),
        ("unlock", None) => Err("register this device's passkey first".to_owned()),
        _ => return Err(not_found().into()),
    };
    if let Err(why) = checked {
        return plain(403, &format!("Not unlocked: {why}."));
    }
    let line = serde_json::json!({ "at": now(), "device": me.id, "event": "unlock" });
    let file = app_context::<Data>(cx).0.join("terminal.log");
    tokio::task::spawn_blocking(move || cooee_core::append_line(&file, &line.to_string()))
        .await??;
    plain(200, "unlocked")
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(device_list)
        .route(device_remove)
        .route(device_rename)
        .route(passkey_options)
        .route(passkey_check)
}
