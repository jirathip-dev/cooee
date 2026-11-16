//! `GET /screens/plugins/{name}/attach?persona=` (#312): the attach sheet as data, one `permissions` card of what
//! installed plugin `name` may do for the persona. Only core serves this card kind.

use cooee_core::{personas, plugins::PlaceKind};
use serde::Deserialize;
use serde_json::{Value, json};
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        content::Json,
        parse_query_params, path_param,
        response::{IntoResponse, Response},
        route,
    },
};

use crate::{Data, Name, manage::Row, manager, plain};

#[derive(Deserialize)]
struct Query {
    persona: String,
}

/// The sheet, or 404 for a plugin that isn't installed or a persona that doesn't exist.
#[route(GET "/screens/plugins/{name}/attach")]
pub async fn attach(cx: &Cx) -> Result<Response> {
    let q: Query =
        parse_query_params(cx).map_err(|e| topcoat::router::error::bad_request(e.to_string()))?;
    let name = path_param::<Name>(cx).to_owned();
    let d = app_context::<Data>(cx);
    let (data, config, m) = (d.0.clone(), d.1.clone(), manager(cx));
    let found = tokio::task::spawn_blocking(move || {
        let Some(row) = m.list()?.into_iter().find(|r| r.locked.name == name) else {
            return Ok(Err(format!("{name} isn't installed.")));
        };
        Ok::<_, std::io::Error>(
            personas::find(config.as_deref(), &data, &q.persona)
                .map(|p| sheet(&row, &p.name))
                .map_err(|e| e.to_string()),
        )
    })
    .await??;
    match found {
        Ok(screen) => Json(super::core(screen)).into_response(cx),
        Err(why) => plain(404, &why),
    }
}

fn may(p: &Row) -> Vec<String> {
    let places = p.places.iter().map(|place| match place.kind {
        PlaceKind::HeaderStrip => format!("{} chip on the header strip", place.label),
        PlaceKind::ChatCards => "cards in its chats".to_owned(),
        PlaceKind::MenuRow => format!("{} row in Plugins", place.label),
    });
    let claims =
        (p.claims.iter()).map(|c| format!("sessions labelled source={c} in its Workflow sheet"));
    let push = p.permissions.push.then(|| "notifications".to_owned());
    places.chain(claims).chain(push).collect()
}

fn sheet(p: &Row, persona: &str) -> Value {
    let (l, perms) = (&p.locked, &p.permissions);
    let row = |title: &str| json!({ "type": "row", "id": title, "title": title });
    let actions = perms.actions.iter().map(|a| {
        let mut r = row(a);
        if perms.destructive.contains(a) {
            r["style"] = json!("destructive");
            r["badge"] = json!({ "text": "always confirmed first", "tone": "bad" });
        }
        r
    });
    let network: Vec<Value> = if perms.network.is_empty() {
        vec![row("nothing")]
    } else {
        perms.network.iter().map(|h| row(h)).collect()
    };
    let section = |id: &str, header: String, rows: Vec<Value>| {
        [
            json!({ "type": "text", "id": format!("{id}-header"), "markdown": header }),
            json!({ "type": "list", "id": id, "rows": rows }),
        ]
    };
    let blocks = [
        section(
            "may",
            format!("To {persona}, it may"),
            may(p).iter().map(|m| row(m)).collect(),
        ),
        section("actions", "Actions".to_owned(), actions.collect()),
        section("network", "Its backend reaches".to_owned(), network),
    ]
    .concat();
    let source = if l.listed { "cooee's index" } else { &l.url };
    let commit: String = l.commit.chars().take(7).collect();
    let card = json!({ "type": "card", "id": "permissions", "kind": "permissions", "title": l.name,
                       "subtitle": format!("{} {} · {commit} · {source}", l.name, l.version), "blocks": blocks });
    json!({ "type": "screen", "id": "attach", "title": format!("Attach {}", l.name), "blocks": [card] })
}
