use serde_json::{Value, json};
use topcoat::{
    Result,
    context::Cx,
    router::{RouterBuilder, route},
};

use super::{connectors, settings};
use crate::{
    HubError,
    http::{Answer, core, refused},
};

const BUNDLED: &str = include_str!("../../../../apple/Sources/CooeeUI/Default/screens/menu.json");

/// `{type: screen, source: core, id, title, blocks}`: `signins` valued with the signed-in accounts, `devices` with
/// the paired count, `machines` with the hosts, `plugins` with the installed count and `connectors` with theirs (or
/// `warn` and the count that needs sign-in).
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
    signins.retain(|r| r.pointer("/subtitle/tone").is_some_and(|t| t == "ok"));
    let (linked, warn) = connectors::menu_value(&connectors::connector_rows(cx).await?);
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
        ("connectors", linked),
    ];
    let groups = screen
        .get_mut("blocks")
        .and_then(Value::as_array_mut)
        .into_iter()
        .flatten();
    let rows = groups.filter_map(|g| g.get_mut("rows").and_then(Value::as_array_mut));
    for row in rows.flatten() {
        if let Some((_, v)) = values.iter().find(|(id, _)| row["id"] == *id) {
            row["value"] = json!(v);
        }
        if warn && row["id"] == "connectors" {
            row["warn"] = json!(true);
        }
    }
    Ok(core(screen))
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(menu)
}
