use std::sync::Arc;

use serde_json::{Value, json};
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

pub(super) async fn connector_rows(cx: &Cx) -> Result<Vec<Value>, HubError> {
    let c = Arc::clone(app_context::<Arc<Connectors>>(cx));
    blocking(cx, move |hub| {
        let listed = c.list(&hub.mcp().map_err(refused)?, &[]).map_err(refused)?;
        let row = |s: &Value| {
            let accounts = s["accounts"].as_array().map_or(&[][..], Vec::as_slice);
            let needs = accounts
                .iter()
                .filter(|a| a["state"] == "needs_sign_in")
                .count();
            let subtitle = match (needs, accounts.len()) {
                (0, 0) => json!({ "text": "no sign-in", "tone": "mute" }),
                (0, n) => json!({ "text": format!("{n} signed in"), "tone": "ok" }),
                (n, _) => json!({ "text": format!("{n} needs sign-in"), "tone": "warn" }),
            };
            json!({ "type": "row", "id": s["name"], "title": s["label"], "subtitle": subtitle })
        };
        let all = listed
            .get("connectors")
            .and_then(Value::as_array)
            .map_or(&[][..], Vec::as_slice);
        Ok(all.iter().map(row).collect())
    })
    .await
}

pub(super) fn menu_value(rows: &[Value]) -> (String, bool) {
    let warn = |r: &&Value| r.pointer("/subtitle/tone").is_some_and(|t| t == "warn");
    match rows.iter().filter(warn).count() {
        0 => (rows.len().to_string(), false),
        1 => ("1 needs sign-in".to_owned(), true),
        n => (format!("{n} need sign-in"), true),
    }
}

#[route(GET "/screens/settings/connectors")]
pub async fn connectors(cx: &Cx) -> Result<Answer<Value>> {
    let rows = connector_rows(cx).await;
    Ok(Answer(rows.map(|rows| {
        let blocks = json!([{ "type": "list", "id": "connectors", "rows": rows }]);
        core(json!({ "type": "screen", "id": "connectors", "title": "Connectors", "blocks": blocks }))
    })))
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(connectors)
}
