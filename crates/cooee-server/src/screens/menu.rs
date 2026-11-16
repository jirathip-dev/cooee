//! `GET /screens/menu` (#312): the app's bundled avatar menu, the rows that open a server screen given a `value`
//! from the same data that screen lists; the device-local rows as bundled.

use serde_json::{Value, json};
use topcoat::{Result, context::Cx, router::route};

use super::settings;
use crate::{Answer, HubError, refused};

/// The menu the app draws offline.
const BUNDLED: &str = include_str!("../../../../apple/Sources/CooeeUI/Default/screens/menu.json");

/// `{type: screen, source: core, id, title, blocks}`: `signins` valued with the signed-in accounts, `devices` with
/// the paired count, `machines` with the hosts and `plugins` with the installed count.
#[route(GET "/screens/menu")]
pub async fn menu(cx: &Cx) -> Result<Answer<Value>> {
    Ok(Answer(fill(cx).await))
}

async fn fill(cx: &Cx) -> Result<Value, HubError> {
    let mut screen: Value = serde_json::from_str(BUNDLED).map_err(refused)?;
    let titles = |rows: &[Value]| {
        let titles: Vec<_> = rows.iter().filter_map(|r| r["title"].as_str()).collect();
        titles.join(", ")
    };
    let mut signins = settings::signin_rows(cx).await?;
    signins.retain(|r| r["subtitle"]["tone"] == "ok");
    let values = [
        ("signins", titles(&signins)),
        (
            "devices",
            settings::device_rows(cx).await?.len().to_string(),
        ),
        ("machines", titles(&settings::machine_rows(cx).await?)),
        (
            "plugins",
            settings::plugin_rows(cx).await?.len().to_string(),
        ),
    ];
    let groups = screen["blocks"].as_array_mut().into_iter().flatten();
    let rows = groups.filter_map(|g| g.get_mut("rows").and_then(Value::as_array_mut));
    for row in rows.flatten() {
        if let Some((_, v)) = values.iter().find(|(id, _)| row["id"] == *id) {
            row["value"] = json!(v);
        }
    }
    Ok(super::core(screen))
}
