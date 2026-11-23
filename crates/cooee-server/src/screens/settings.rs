use std::{collections::HashMap, sync::Arc, time::Duration};

use blocks::{Block, Label, List, Row, Screen, Tone, Toned};
use cooee_core::shed_proto::AccountState;
use cooee_machines::Machines;
use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, app_context, request_context},
    router::route,
};

use crate::{
    Hub, HubError,
    http::{Answer, Paired, blocking, manager, on_logins, refused},
    pairing, plugin,
};

const DROVER: Duration = Duration::from_secs(2);

fn list(id: &str, title: &str, rows: Vec<Row>) -> Block {
    let rows: Vec<Block> = rows.into_iter().map(|r| Block::Row(Box::new(r))).collect();
    let list = Block::List(Box::new(List::new(rows).id(id)));
    crate::http::core(Screen::default().id(id).title(title).blocks(vec![list]))
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
        let sheets = l.sheets();
        let rows = sheets.iter().flat_map(|s| {
            s.accounts.iter().map(move |a| {
                let (text, tone) = match a.state {
                    AccountState::Ready => ("signed in", Tone::Ok),
                    AccountState::Limited => ("at its limit", Tone::Warn),
                    AccountState::SignedOut => ("signed out", Tone::Bad),
                };
                let subtitle = Label {
                    text: text.into(),
                    tone: Some(tone),
                };
                let who = match (&a.who, a.name.as_str()) {
                    (Some(who), _) => who.as_str(),
                    (None, "default") => "main login",
                    (None, name) => name,
                };
                Row::new(format!("{} · {who}", s.provider.name))
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
    blocking(cx, move |hub| {
        let cards = all.poll(cooee_machines::unix_now());
        let mut tags = drover_tags(hub);
        let row = |c: cooee_machines::Card| {
            let state = if c.online { "online" } else { "offline" };
            let subtitle = match tags.remove(&c.name) {
                Some(t) => format!("{state} · {}", t.join(" · ")),
                None => state.to_owned(),
            };
            row(c.name.clone(), c.name, subtitle)
        };
        Ok(cards.into_iter().map(row).collect())
    })
    .await
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct HostTab {
    hosts: Vec<HostCard>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct HostCard {
    name: String,
    rows: Vec<Cell>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Cell {
    k: String,
    notes: Vec<String>,
}

fn drover_tags(hub: &Hub) -> HashMap<String, Vec<String>> {
    let drover = hub
        .plugins()
        .into_iter()
        .find(|p| hub.plugin_of(&p.name) == "drover");
    let Some(reply) =
        drover.and_then(|p| plugin::call(&p, "GET", "/api/v1/host", &[], &[], DROVER).ok())
    else {
        return HashMap::new();
    };
    let ok = reply.status == 200;
    let tab: HostTab = reply
        .bytes()
        .ok()
        .filter(|_| ok)
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let tags = |h: HostCard| {
        let row = h.rows.into_iter().find(|r| r.k == "tags")?;
        let tags: Vec<String> = row
            .notes
            .iter()
            .flat_map(|n| n.split_whitespace())
            .map(str::to_owned)
            .collect();
        (!tags.is_empty()).then_some((h.name, tags))
    };
    tab.hosts.into_iter().filter_map(tags).collect()
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

pub(super) fn routes(b: topcoat::router::RouterBuilder) -> topcoat::router::RouterBuilder {
    b.route(devices)
        .route(signins)
        .route(machines)
        .route(plugins)
}
