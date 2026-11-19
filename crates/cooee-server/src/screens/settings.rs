//! `GET /screens/settings/{devices,signins,machines,plugins}` (#312): each one list of rows from the data its JSON
//! route reads (`GET /devices`, the sign-ins sheet, `GET /machines`, `GET /plugins`).

use std::sync::Arc;

use blocks::{Block, Label, List, Row, Screen, Tone, Toned};
use cooee_machines::Machines;
use topcoat::{
    Result,
    context::{Cx, app_context, request_context},
    router::route,
};

use crate::{
    HubError,
    http::{Answer, Paired, blocking, manager, on_logins, refused},
    pairing,
};

fn list(id: &str, title: &str, rows: Vec<Row>) -> Block {
    let rows: Vec<Block> = rows.into_iter().map(|r| Block::Row(Box::new(r))).collect();
    let list = Block::List(Box::new(List::new(rows).id(id)));
    super::core(Screen::default().id(id).title(title).blocks(vec![list]))
}

fn row(id: impl Into<String>, title: impl Into<String>, subtitle: impl Into<String>) -> Row {
    Row::new(title)
        .id(id)
        .subtitle(Toned::Text(subtitle.into()))
}

pub(super) async fn device_rows(cx: &Cx) -> Result<Vec<Row>, HubError> {
    let paired = pairing::devices(cx);
    let me = request_context::<Paired>(cx).0.id.clone();
    blocking(cx, move |_| {
        let list = paired.list().map_err(refused)?;
        let row = |d: cooee_core::Device| {
            let row = Row::new(d.label).id(d.id.clone());
            if d.id == me {
                return row.subtitle(Toned::Text("this device".to_owned()));
            }
            row
        };
        Ok(list.into_iter().map(row).collect())
    })
    .await
}

pub(super) async fn signin_rows(cx: &Cx) -> Result<Vec<Row>, HubError> {
    on_logins(cx, |l| {
        let sheets = l.statuses();
        let rows = sheets.iter().flat_map(|s| {
            s.accounts.iter().map(move |(a, st)| {
                let (text, tone) = match (st.signed_in, st.unknown) {
                    (true, _) => ("signed in", Tone::Ok),
                    (false, true) => ("can't tell", Tone::Mute),
                    (false, false) => ("signed out", Tone::Bad),
                };
                let subtitle = Label {
                    text: text.to_owned(),
                    tone: Some(tone),
                };
                Row::new(format!("{} · {}", s.provider.name, a.shown(st)))
                    .id(format!("{}/{}", s.provider.name, a.name))
                    .subtitle(Toned::Label(subtitle))
            })
        });
        Ok(rows.collect())
    })
    .await
}

pub(super) async fn machine_rows(cx: &Cx) -> Result<Vec<Row>, HubError> {
    let all = Arc::clone(app_context::<Arc<Machines>>(cx));
    blocking(cx, move |_| {
        let cards = all.poll(cooee_machines::unix_now());
        let row = |c: cooee_machines::Card| {
            let state = if c.online { "online" } else { "offline" };
            row(c.name.clone(), c.name, state)
        };
        Ok(cards.into_iter().map(row).collect())
    })
    .await
}

pub(super) async fn plugin_rows(cx: &Cx) -> Result<Vec<Row>, HubError> {
    let m = manager(cx);
    blocking(cx, move |_| {
        let rows = m.list().map_err(refused)?.into_iter().map(|r| {
            let l = r.locked;
            row(l.name.clone(), l.name, l.version)
        });
        Ok(rows.collect())
    })
    .await
}

#[route(GET "/screens/settings/devices")]
pub async fn devices(cx: &Cx) -> Result<Answer<Block>> {
    let rows = device_rows(cx).await;
    Ok(Answer(rows.map(|r| list("devices", "Devices", r))))
}

#[route(GET "/screens/settings/signins")]
pub async fn signins(cx: &Cx) -> Result<Answer<Block>> {
    let rows = signin_rows(cx).await;
    Ok(Answer(rows.map(|r| list("signins", "Sign-ins", r))))
}

#[route(GET "/screens/settings/machines")]
pub async fn machines(cx: &Cx) -> Result<Answer<Block>> {
    let rows = machine_rows(cx).await;
    Ok(Answer(rows.map(|r| list("machines", "Machines", r))))
}

#[route(GET "/screens/settings/plugins")]
pub async fn plugins(cx: &Cx) -> Result<Answer<Block>> {
    let rows = plugin_rows(cx).await;
    Ok(Answer(rows.map(|r| list("plugins", "Plugins", r))))
}
