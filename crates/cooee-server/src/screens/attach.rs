use blocks::{Block, Card, CardKind, Label, List, Row, RowStyle, Screen, Text, Tone};
use cooee_core::{agents, plugins::PlaceKind};
use serde::Deserialize;
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

use crate::{
    http::{Data, Name, manager, plain},
    manage,
};

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
            agents::find(config.as_deref(), &data, &q.persona)
                .map(|p| sheet(&row, &p.name))
                .map_err(|e| e.to_string()),
        )
    })
    .await??;
    match found {
        Ok(screen) => Json(crate::http::core(screen)).into_response(cx),
        Err(why) => plain(404, &why),
    }
}

fn may(p: &manage::Row) -> Vec<String> {
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

fn sheet(p: &manage::Row, persona: &str) -> Screen {
    let (l, perms) = (&p.locked, &p.permissions);
    let row = |title: &str| Block::Row(Box::new(Row::new(title).id(title)));
    let actions = perms.actions.iter().map(|a| {
        let r = Row::new(a.as_str()).id(a.as_str());
        if !perms.destructive.contains(a) {
            return Block::Row(Box::new(r));
        }
        let badge = Label {
            text: "always confirmed first".to_owned(),
            tone: Some(Tone::Bad),
        };
        Block::Row(Box::new(r.style(RowStyle::Destructive).badge(badge)))
    });
    let network: Vec<Block> = if perms.network.is_empty() {
        vec![row("nothing")]
    } else {
        perms.network.iter().map(|h| row(h)).collect()
    };
    let section = |id: &str, header: String, rows: Vec<Block>| {
        let header = Text::default().id(format!("{id}-header")).markdown(header);
        [
            Block::Text(Box::new(header)),
            Block::List(Box::new(List::new(rows).id(id))),
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
    let card = Card::default()
        .id("permissions")
        .kind(CardKind::Other("permissions".to_owned()))
        .title(l.name.as_str())
        .subtitle(format!("{} {} · {commit} · {source}", l.name, l.version))
        .blocks(blocks);
    Screen::default()
        .id("attach")
        .title(format!("Attach {}", l.name))
        .blocks(vec![Block::Card(Box::new(card))])
}

pub(super) fn routes(b: topcoat::router::RouterBuilder) -> topcoat::router::RouterBuilder {
    b.route(attach)
}
