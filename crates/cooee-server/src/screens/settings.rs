//! `GET /screens/settings/{devices,signins,machines,plugins}` (#312): each one list of rows from the data its JSON
//! route reads (`GET /devices`, the sign-ins sheet, `GET /machines`, `GET /plugins`).

use std::sync::Arc;

use cooee_machines::Machines;
use serde_json::{Value, json};
use topcoat::{
    Result,
    context::{Cx, app_context, request_context},
    router::{RouterBuilder, route},
};

use crate::{
    HubError,
    http::{Answer, Paired, blocking, core, manager, on_logins, refused},
    pairing,
};

fn list(id: &str, title: &str, rows: &[Value]) -> Value {
    let blocks = json!([{ "type": "list", "id": id, "rows": rows }]);
    core(json!({ "type": "screen", "id": id, "title": title, "blocks": blocks }))
}

pub(super) async fn device_rows(cx: &Cx) -> Result<Vec<Value>, HubError> {
    let paired = pairing::devices(cx);
    let me = request_context::<Paired>(cx).0.id.clone();
    blocking(cx, move |_| {
        let list = paired.list().map_err(refused)?;
        let row = |d: cooee_core::Device| {
            let mut row = json!({ "type": "row", "id": d.id, "title": d.label });
            if let (true, Some(o)) = (d.id == me, row.as_object_mut()) {
                o.insert("subtitle".to_owned(), json!("this device"));
            }
            row
        };
        Ok(list.into_iter().map(row).collect())
    })
    .await
}

pub(super) async fn signin_rows(cx: &Cx) -> Result<Vec<Value>, HubError> {
    on_logins(cx, |l| {
        let sheets = l.statuses();
        let rows = sheets.iter().flat_map(|s| {
            s.accounts.iter().map(move |(a, st)| {
                let (text, tone) = match (st.signed_in, st.unknown) {
                    (true, _) => ("signed in", "ok"),
                    (false, true) => ("can't tell", "mute"),
                    (false, false) => ("signed out", "bad"),
                };
                json!({ "type": "row", "id": format!("{}/{}", s.provider.name, a.name),
                        "title": format!("{} · {}", s.provider.name, a.shown(st)),
                        "subtitle": { "text": text, "tone": tone } })
            })
        });
        Ok(rows.collect())
    })
    .await
}

pub(super) async fn machine_rows(cx: &Cx) -> Result<Vec<Value>, HubError> {
    let all = Arc::clone(app_context::<Arc<Machines>>(cx));
    blocking(cx, move |_| {
        let cards = all.poll(cooee_machines::unix_now());
        let row = |c: cooee_machines::Card| {
            let state = if c.online { "online" } else { "offline" };
            json!({ "type": "row", "id": c.name, "title": c.name, "subtitle": state })
        };
        Ok(cards.into_iter().map(row).collect())
    })
    .await
}

pub(super) async fn plugin_rows(cx: &Cx) -> Result<Vec<Value>, HubError> {
    let m = manager(cx);
    blocking(cx, move |_| {
        let rows = m.list().map_err(refused)?.into_iter().map(|r| {
            let l = r.locked;
            json!({ "type": "row", "id": l.name, "title": l.name, "subtitle": l.version })
        });
        Ok(rows.collect())
    })
    .await
}

#[route(GET "/screens/settings/devices")]
pub async fn devices(cx: &Cx) -> Result<Answer<Value>> {
    let rows = device_rows(cx).await;
    Ok(Answer(rows.map(|r| list("devices", "Devices", &r))))
}

#[route(GET "/screens/settings/signins")]
pub async fn signins(cx: &Cx) -> Result<Answer<Value>> {
    let rows = signin_rows(cx).await;
    Ok(Answer(rows.map(|r| list("signins", "Sign-ins", &r))))
}

#[route(GET "/screens/settings/machines")]
pub async fn machines(cx: &Cx) -> Result<Answer<Value>> {
    let rows = machine_rows(cx).await;
    Ok(Answer(rows.map(|r| list("machines", "Machines", &r))))
}

#[route(GET "/screens/settings/plugins")]
pub async fn plugins(cx: &Cx) -> Result<Answer<Value>> {
    let rows = plugin_rows(cx).await;
    Ok(Answer(rows.map(|r| list("plugins", "Plugins", &r))))
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(devices)
        .route(signins)
        .route(machines)
        .route(plugins)
}
