use std::path::Path;

use blocks::{
    Attachment, Block, Card, Composer, Image, Images, Message, Prose, Role, Row, Screen, Section,
    Stage, Steps, Tap, Text, Toned, Tool, ToolState, Working,
};
use cooee_core::{Body, Item, routines::local};
use topcoat::{context::Cx, router::route};

use crate::{
    Hub, HubError, Slice,
    http::{Answer, blocking, core, image, thread_id},
    live::{Source, Wake, Who},
};

const PATH: &str = "/screens/chat/";
const WINDOW: usize = 50;
const SUMMARY: usize = 80;

/// `{type: screen, source: core, id: chat}`: thread `id`'s newest 50 items, an agent's turn as one `message`.
#[route(GET "/screens/chat/{id}")]
pub async fn chat(cx: &Cx) -> topcoat::Result<Answer<Block>> {
    let id = thread_id(cx);
    Ok(Answer(blocking(cx, move |hub| screen(hub, &id)).await))
}

pub(super) fn source(path: &str, who: &Who) -> Option<Source> {
    let id = path.strip_prefix(PATH)?;
    if id.is_empty() || id.contains(['/', '?']) {
        return None;
    }
    let Who::Device(_) = who else {
        return None;
    };
    let (built, seen) = (id.to_owned(), id.to_owned());
    Some(Source {
        wake: Wake::Thread(id.to_owned()),
        build: Box::new(move |hub| screen(hub, &built)),
        seen: Some(Box::new(move |hub| read(hub, &seen))),
    })
}

fn read(hub: &Hub, id: &str) -> Result<(), HubError> {
    match hub.slice(id, None, 1, 0)?.items.last() {
        Some(newest) => hub.read(id, newest.id),
        None => Ok(()),
    }
}

fn screen(hub: &Hub, id: &str) -> Result<Block, HubError> {
    let (meta, _) = hub.thread(id)?;
    let slice = hub.slice(id, None, WINDOW, 0)?;
    let personas = hub.personas();
    let name = |speaker: &str| {
        let who = if speaker.is_empty() {
            &meta.profile
        } else {
            speaker
        };
        let persona = personas.iter().find(|p| p.id == *who);
        persona.map_or_else(|| meta.agent.clone(), |p| p.name.clone())
    };
    let mut blocks = vec![Block::Section(Box::new(
        Section::new(log(id, &slice, &name)).id("log"),
    ))];
    blocks.extend(state(&slice));
    blocks.push(composer(&slice));
    let screen = Screen::default().id("chat").title(meta.title.clone());
    Ok(core(screen.back("/screens/sessions").blocks(blocks)))
}

struct Group {
    id: String,
    speaker: String,
    blocks: Vec<Block>,
}

fn log(thread: &str, slice: &Slice, name: &dyn Fn(&str) -> String) -> Vec<Block> {
    let mut out = vec![];
    let mut group: Option<Group> = None;
    let end = |group: &mut Option<Group>, out: &mut Vec<Block>| {
        if let Some(g) = group.take() {
            let message = Message::new(Role::Agent, name(&g.speaker)).id(g.id);
            out.push(Block::Message(Box::new(message.blocks(g.blocks))));
        }
    };
    for item in &slice.items {
        if let Some(said) = said(thread, item) {
            end(&mut group, &mut out);
            out.push(said);
            continue;
        }
        if group.as_ref().is_some_and(|g| g.speaker != item.speaker) {
            end(&mut group, &mut out);
        }
        let g = group.get_or_insert_with(|| Group {
            id: format!("m{}", item.id),
            speaker: item.speaker.clone(),
            blocks: vec![],
        });
        if let Body::Agent { text, .. } = &item.body {
            g.blocks.push(prose(format!("{}-text", g.id), text, false));
            end(&mut group, &mut out);
        } else {
            g.blocks.extend(part(thread, item));
        }
    }
    if !slice.turn.text.is_empty() {
        let next = slice.items.last().map_or(1, |i| i.id.saturating_add(1));
        let g = group.get_or_insert_with(|| Group {
            id: format!("m{next}"),
            speaker: String::new(),
            blocks: vec![],
        });
        g.blocks
            .push(prose(format!("{}-text", g.id), &slice.turn.text, true));
    }
    end(&mut group, &mut out);
    out
}

fn prose(id: String, markdown: &str, streaming: bool) -> Block {
    let prose = Prose::new(Prose::read(markdown)).id(id);
    Block::Prose(Box::new(if streaming {
        prose.streaming(true)
    } else {
        prose
    }))
}

fn said(thread: &str, item: &Item) -> Option<Block> {
    let line =
        |text: String| Block::Text(Box::new(Text::new().id(format!("i{}", item.id)).text(text)));
    let (role, name, blocks, files) = match &item.body {
        Body::You {
            text,
            files,
            routine,
            ..
        } => {
            let name = routine.as_ref().map_or("You", |r| r.name.as_str());
            let text = prose(format!("i{}", item.id), text, false);
            (
                Role::Owner,
                name,
                text,
                files.iter().map(|f| file(thread, f)).collect(),
            )
        }
        Body::Link {
            text,
            thread: other,
            ..
        } => {
            let tap = Tap {
                open: Some(format!("{PATH}{other}")),
                ..Tap::default()
            };
            let row = Row::new(text.clone()).id(format!("i{}", item.id)).tap(tap);
            (Role::System, "cooee", Block::Row(Box::new(row)), vec![])
        }
        Body::Routine {
            name, when, place, ..
        } => {
            let text = format!("⟳ Created routine: {name} · {when} · {place}");
            (Role::System, "cooee", line(text), vec![])
        }
        Body::Note { text } | Body::Ended { text } => {
            (Role::System, "cooee", line(text.clone()), vec![])
        }
        _ => return None,
    };
    let message = Message::new(role, name)
        .id(format!("m{}", item.id))
        .time(iso(item.at))
        .blocks(vec![blocks]);
    Some(Block::Message(Box::new(if files.is_empty() {
        message
    } else {
        message.files(files)
    })))
}

fn iso(at: u64) -> String {
    let minute = local(at, 0).replacen(' ', "T", 1);
    format!("{minute}:{:02}Z", at / 1000 % 60)
}

fn file(thread: &str, name: &str) -> Attachment {
    let url = format!("/threads/{thread}/files/{name}");
    Attachment {
        name: name.to_owned(),
        src: image(Path::new(name)).map(|_| url.clone()),
        tap: Tap {
            url: Some(url),
            ..Tap::default()
        },
    }
}

fn part(thread: &str, item: &Item) -> Option<Block> {
    let id = format!("i{}", item.id);
    Some(match &item.body {
        Body::Tool {
            subject,
            action,
            status,
            ruled,
            ..
        } => {
            let state = match status.as_str() {
                "completed" => ToolState::Done,
                "failed" | "cancelled" => ToolState::Failed,
                _ => ToolState::Running,
            };
            let summary: String = subject.chars().take(SUMMARY).collect();
            let name = format!("{action:?}").to_lowercase();
            let tool = Tool::new(name, state).id(id);
            let tool = match ruled {
                Some(rule) => tool.body(format!("{subject}\n{rule}")),
                None if summary.len() < subject.len() => tool.body(subject.clone()),
                None => tool,
            };
            Block::Tool(Box::new(tool.summary(summary)))
        }
        Body::Approval { .. } => return item.block(),
        Body::Plan { entries } => {
            let steps = entries.iter().enumerate().map(|(n, e)| Stage {
                id: format!("{id}-{n}"),
                label: e["content"].as_str().unwrap_or_default().to_owned(),
                sub: e["status"].as_str().map(str::to_owned),
                tap: None,
            });
            let steps: Vec<Stage> = steps.collect();
            Block::Steps(Box::new(Steps::new().id(id).steps(steps)))
        }
        Body::Ask {
            name,
            text,
            thread: other,
            ..
        } => {
            let row = Row::new(format!("Asked {name}"))
                .id(id)
                .subtitle(Toned::Text(text.clone()));
            let tap = Tap {
                open: Some(format!("{PATH}{other}")),
                ..Tap::default()
            };
            Block::Row(Box::new(if other.is_empty() { row } else { row.tap(tap) }))
        }
        Body::Handoff { text, outcome, .. } => {
            let card = Card::new().id(id).title(text.clone());
            Block::Card(Box::new(
                card.subtitle(Toned::Text(outcome.replace('_', " "))),
            ))
        }
        Body::Frame { text, name, .. } => {
            let shot = Image {
                src: format!("/threads/{thread}/frames/{name}"),
                alt: Some(text.clone()),
                caption: Some(text.clone()),
            };
            Block::Images(Box::new(Images::new(vec![shot]).id(id)))
        }
        _ => return None,
    })
}

fn state(slice: &Slice) -> Option<Block> {
    let turn = &slice.turn;
    let title = match turn.state.as_str() {
        "unconnected" => "Not connected",
        "error" => "Failed",
        "limited" => "Limited",
        "ended" => "Ended",
        _ => return None,
    };
    let row = Row::new(title).id("state");
    Some(Block::Row(Box::new(match &turn.note {
        Some(note) => row.subtitle(Toned::Text(note.clone())),
        None => row,
    })))
}

fn composer(slice: &Slice) -> Block {
    let say = Composer::new("send").id("say");
    let running = matches!(
        slice.turn.state.as_str(),
        "working" | "connecting" | "blocked"
    );
    let since = slice.items.iter().rev().find_map(|i| match i.body {
        Body::You { .. } => Some(i.at),
        _ => None,
    });
    Block::Composer(Box::new(match since.filter(|_| running) {
        Some(at) => say.working(Working { since: iso(at) }),
        None => say,
    }))
}

pub(super) fn routes(b: topcoat::router::RouterBuilder) -> topcoat::router::RouterBuilder {
    b.route(chat)
}
