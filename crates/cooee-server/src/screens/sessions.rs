use blocks::{Act, Action, Answer, Block, Fields, Label, Row, Screen, Section, Tap, Tone, Toned};
use cooee_core::Summary;
use serde::Deserialize;
use serde_json::Value;
use topcoat::{
    Result,
    context::Cx,
    router::{parse_query_params, response::Response, route},
};

use crate::{
    Hub,
    http::{self, blocking, refused},
    live::{Source, Wake, Who},
};

const PATH: &str = "/screens/sessions";

#[derive(Deserialize)]
struct Archived {
    archived: Option<u8>,
}

/// `{type: screen, source: core, id: sessions}`: one section of chats per persona, then the chats with none.
#[route(GET "/screens/sessions")]
pub async fn sessions(cx: &Cx) -> Result<http::Answer<Block>> {
    let q: Option<Archived> = parse_query_params(cx).ok();
    let archived = q.and_then(|q| q.archived) == Some(1);
    Ok(http::Answer(
        blocking(cx, move |hub| Ok(screen(hub, archived))).await,
    ))
}

pub(super) fn source(path: &str, who: &Who) -> Option<Source> {
    let archived = match path.strip_prefix(PATH)? {
        "" => false,
        "?archived=1" => true,
        _ => return None,
    };
    if !matches!(who, Who::Device(_)) {
        return None;
    }
    Some(Source {
        wake: Wake::Home,
        build: Box::new(move |hub, _| Ok(screen(hub, archived))),
        seen: None,
    })
}

fn screen(hub: &Hub, archived: bool) -> Block {
    let mut rows: Vec<Summary> = hub.list();
    rows.retain(|r| r.archived == archived);
    let mut sections = vec![];
    for p in hub.personas().iter() {
        let (mine, rest) = rows.into_iter().partition(|r| r.persona == p.id);
        rows = rest;
        if mine.is_empty() {
            continue;
        }
        let mut blocks: Vec<Block> = mine.iter().map(row).collect();
        let new = Tap {
            open: Some(format!("/screens/chat?persona={}", p.id)),
            ..Tap::default()
        };
        let new = Row::new("New chat").id(format!("new-{}", p.id)).tap(new);
        blocks.push(Block::Row(Box::new(new)));
        let section = Section::new(blocks)
            .id(format!("p-{}", p.id))
            .header(p.name.clone());
        sections.push(Block::Section(Box::new(section)));
    }
    if !rows.is_empty() {
        let section = Section::new(rows.iter().map(row).collect::<Vec<_>>()).id("other");
        sections.push(Block::Section(Box::new(section.header("Other chats"))));
    }
    let title = if archived { "Archived" } else { "Chats" };
    let screen = Screen::default().id("sessions").title(title);
    crate::http::core(screen.blocks(sections))
}

fn row(s: &Summary) -> Block {
    let tone = match s.state.as_str() {
        "working" | "connecting" => Some(Tone::Working),
        "blocked" => Some(Tone::Needs),
        "done" => Some(Tone::Done),
        "error" | "unconnected" => Some(Tone::Fail),
        "limited" => Some(Tone::Warn),
        "ended" => Some(Tone::Mute),
        _ => None,
    };
    let last = Label {
        text: s.last.as_str().into(),
        tone,
    };
    let tap = Tap {
        open: Some(format!("/screens/chat/{}", s.id)),
        ..Tap::default()
    };
    let archive = Act {
        label: "Archive".into(),
        action: Some("archive".into()),
        values: Some(Fields::from_iter([(
            "thread".into(),
            Value::from(s.id.as_str()),
        )])),
        ..Act::default()
    };
    let mut row = Row::new(s.title.clone())
        .id(format!("t-{}", s.id))
        .subtitle(Toned::Label(last))
        .tap(tap)
        .swipe(vec![archive]);
    if s.unseen > 0 {
        row = row.badge(Label {
            text: s.unseen.to_string().into(),
            tone: Some(Tone::Accent),
        });
    }
    Block::Row(Box::new(row))
}

pub(super) async fn act(cx: &Cx, a: &Action) -> Option<Result<Response>> {
    a.screen.strip_prefix(PATH)?;
    let thread = a.values.get("thread").and_then(Value::as_str);
    let thread = thread.unwrap_or_default().to_owned();
    let done = match a.action.as_str() {
        "archive" => blocking(cx, move |hub| hub.archive(&thread, true)).await,
        other => Err(refused(format!("no action {other}"))),
    };
    Some(super::chat::answer(done.map(|()| Answer::default())))
}

pub(super) fn routes(b: topcoat::router::RouterBuilder) -> topcoat::router::RouterBuilder {
    b.route(sessions)
}
