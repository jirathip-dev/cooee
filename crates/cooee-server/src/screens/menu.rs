use blocks::{Block, Label, Row, Tone, Toned};
use serde_json::Value;
use topcoat::{Result, context::Cx, router::route};

use super::{connectors, settings};
use crate::{HubError, http::Answer};

const BUNDLED: &str = include_str!("../../../../apple/Sources/CooeeUI/Default/screens/menu.json");

/// `{type: screen, source: core, id, title, blocks}`: `signins` valued with the signed-in accounts, `devices` with
/// the paired count, `machines` with the hosts, `plugins` with the installed count and `connectors` with theirs (or
/// `warn` and the count that needs sign-in).
#[route(GET "/screens/menu")]
pub async fn menu(cx: &Cx) -> Result<Answer<Block>> {
    Ok(Answer(fill(cx).await))
}

async fn fill(cx: &Cx) -> Result<Block, HubError> {
    let mut screen = crate::http::bundled(BUNDLED)?;
    let titles = |rows: &[Row]| {
        let titles: Vec<_> = rows.iter().map(|r| r.title.as_str()).collect();
        titles.join(", ")
    };
    let mut signins = settings::signin_rows(cx).await?;
    signins.retain(|r| {
        matches!(
            &r.subtitle,
            Some(Toned::Label(Label {
                tone: Some(Tone::Ok),
                ..
            }))
        )
    });
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
    let groups = screen.blocks.iter_mut().flatten().filter_map(|g| match g {
        Block::App(g) => g.more.get_mut("rows").and_then(Value::as_array_mut),
        _ => None,
    });
    for row in groups.flatten() {
        if let Some((_, v)) = values.iter().find(|(id, _)| row["id"] == *id) {
            row["value"] = Value::from(v.as_str());
        }
        if warn && row["id"] == "connectors" {
            row["warn"] = Value::Bool(true);
        }
    }
    Ok(crate::http::core(screen))
}

pub(super) fn routes(b: topcoat::router::RouterBuilder) -> topcoat::router::RouterBuilder {
    b.route(menu)
}
