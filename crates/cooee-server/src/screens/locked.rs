//! The two locked screens (#316): `GET /screens/pairing`, the app's bundled pairing steps, and
//! `GET /screens/reset-layout`, the one button that puts the default `layout.json` back.

use serde_json::{Value, json};
use topcoat::{Result, context::Cx, router::route};

use crate::http::{Answer, refused};

const PAIRING: &str =
    include_str!("../../../../apple/Sources/CooeeUI/Default/screens/pairing.json");

/// `{type: screen, source: core, id, title, blocks}`: one `step` block per pairing step, as the app draws them.
#[route(GET "/screens/pairing")]
pub async fn pairing(_cx: &Cx) -> Result<Answer<Value>> {
    Ok(Answer(
        serde_json::from_str(PAIRING)
            .map(super::core)
            .map_err(refused),
    ))
}

/// `{type: screen, source: core, id, title, blocks}`: a `reset` button posting `POST /layout/reset`.
#[route(GET "/screens/reset-layout")]
pub async fn reset_layout(_cx: &Cx) -> Result<Answer<Value>> {
    Ok(Answer(Ok(super::core(json!({
        "type": "screen", "id": "reset-layout", "title": "Reset layout",
        "blocks": [
            { "type": "text", "id": "why",
              "text": "Puts the default layout back. The old layout.json is kept as layout.json.bak." },
            { "type": "button", "id": "reset", "label": "Reset layout", "style": "destructive",
              "action": "/layout/reset" },
        ],
    })))))
}
