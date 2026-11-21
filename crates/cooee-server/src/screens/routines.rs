use blocks::{Block, Label, List, Row, Screen, Tone, Toned};
use cooee_core::routines::{Edit, Mode};
use serde_json::Value;
use topcoat::{Result, context::Cx, router::route};

use super::persona::Act;
use crate::{
    HubError,
    http::{Answer, refused},
    routines::{self, Guarded},
};

const PATH: &str = "/screens/settings/routines";

/// `{type: screen, source: core, id: routines, title, blocks}`: one list, the default cap first.
#[route(GET "/screens/settings/routines")]
pub async fn guards(cx: &Cx) -> Result<Answer<Block>> {
    Ok(Answer(fill(cx).await))
}

async fn fill(cx: &Cx) -> Result<Block, HubError> {
    let g = routines::on(cx, |r| r.guards(crate::hub::now())).await?;
    let default = format!("{} runs a day", g.per_day.get());
    let mut rows = vec![
        Row::new("Default cap")
            .id("default")
            .subtitle(Toned::Text(default)),
    ];
    rows.extend(g.rows.iter().map(row));
    let rows: Vec<Block> = rows.into_iter().map(|r| Block::Row(Box::new(r))).collect();
    let list = Block::List(Box::new(List::new(rows).id("routines")));
    let screen = Screen::default().id("routines").title("Routines");
    Ok(crate::http::core(screen.blocks(vec![list])))
}

fn row(g: &Guarded) -> Row {
    let (r, t) = (&g.routine, g.today);
    let kind = match r.script.as_ref().map(|s| s.mode) {
        Some(Mode::Wake) => " · wake script",
        Some(Mode::Post) => " · post script",
        None => "",
    };
    let cap = g.cap.map_or_else(
        || "no cap: it runs no agent".to_owned(),
        |c| {
            let whose = if r.per_day.is_some() {
                "its own"
            } else {
                "default"
            };
            let today = (t.ran, t.idle, t.capped);
            format!(
                "cap {} ({whose}) · today: ran {} · skipped {} idle, {} capped",
                c.get(),
                today.0,
                today.1,
                today.2
            )
        },
    );
    let row = Row::new(r.name.clone())
        .id(r.id.clone())
        .subtitle(Toned::Text(format!("{}{kind} · {cap}", g.when)));
    let capped = g.cap.is_some_and(|c| t.capped > 0 || t.ran >= c.get());
    if capped && r.paused.is_none() {
        let tone = Some(Tone::Warn);
        return row.badge(Label {
            text: "capped today".to_owned(),
            tone,
        });
    }
    row
}

pub(super) async fn action(cx: &Cx, a: Act) -> Result<Value, HubError> {
    if a.screen != PATH {
        return Err(refused(format!("no screen {}", a.screen)));
    }
    let value = |k: &str| match a.values.get(k) {
        Some(Value::String(s)) => s.clone(),
        Some(v @ Value::Number(_)) => v.to_string(),
        _ => String::new(),
    };
    let (per_day, id) = (value("per_day"), value("routine"));
    match a.action.as_str() {
        "default" => routines::on(cx, move |r| r.set_default(&per_day).map(|_| ())).await?,
        "cap" => {
            let e = Edit {
                per_day: Some(per_day),
                ..Edit::default()
            };
            routines::on(cx, move |r| r.edit(&id, e, crate::hub::now()).map(|_| ())).await?;
        }
        other => return Err(refused(format!("no action {other}"))),
    }
    serde_json::to_value(fill(cx).await?).map_err(refused)
}

pub(super) fn routes(b: topcoat::router::RouterBuilder) -> topcoat::router::RouterBuilder {
    b.route(guards)
}
