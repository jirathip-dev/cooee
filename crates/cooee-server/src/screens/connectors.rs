use std::sync::Arc;

use blocks::{Block, Label, List, Row, Screen, Tone, Toned};
use serde_json::Value;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{RouterBuilder, route},
};

use crate::{
    HubError,
    connectors::Connectors,
    http::{Answer, blocking, core, refused},
};

pub(super) async fn connector_rows(cx: &Cx) -> Result<Vec<Row>, HubError> {
    let c = Arc::clone(app_context::<Arc<Connectors>>(cx));
    blocking(cx, move |hub| {
        let listed = c.list(&hub.mcp().map_err(refused)?, &[]).map_err(refused)?;
        let row = |s: &Value| {
            let accounts = s["accounts"].as_array().map_or(&[][..], Vec::as_slice);
            let needs = accounts
                .iter()
                .filter(|a| a["state"] == "needs_sign_in")
                .count();
            let (text, tone) = match (needs, accounts.len()) {
                (0, 0) => ("no sign-in".to_owned(), Tone::Mute),
                (0, n) => (format!("{n} signed in"), Tone::Ok),
                (n, _) => (format!("{n} needs sign-in"), Tone::Warn),
            };
            let title = s["label"].as_str().unwrap_or_default();
            Row::new(title)
                .id(s["name"].as_str().unwrap_or_default())
                .subtitle(Toned::Label(Label {
                    text,
                    tone: Some(tone),
                }))
        };
        let all = listed
            .get("connectors")
            .and_then(Value::as_array)
            .map_or(&[][..], Vec::as_slice);
        Ok(all.iter().map(row).collect())
    })
    .await
}

pub(super) fn menu_value(rows: &[Row]) -> (String, bool) {
    let warn = |r: &&Row| {
        matches!(
            &r.subtitle,
            Some(Toned::Label(Label {
                tone: Some(Tone::Warn),
                ..
            }))
        )
    };
    match rows.iter().filter(warn).count() {
        0 => (rows.len().to_string(), false),
        1 => ("1 needs sign-in".to_owned(), true),
        n => (format!("{n} need sign-in"), true),
    }
}

#[route(GET "/screens/settings/connectors")]
pub async fn connectors(cx: &Cx) -> Result<Answer<Block>> {
    let rows = connector_rows(cx).await;
    Ok(Answer(rows.map(|rows| {
        let rows: Vec<Block> = rows.into_iter().map(|r| Block::Row(Box::new(r))).collect();
        let list = Block::List(Box::new(List::new(rows).id("connectors")));
        core(
            Screen::default()
                .id("connectors")
                .title("Connectors")
                .blocks(vec![list]),
        )
    })))
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(connectors)
}
