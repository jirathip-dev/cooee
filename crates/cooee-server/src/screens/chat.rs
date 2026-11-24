use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use blocks::{
    Action, Attachment, Block, Card, Composer, Image, Images, Message, Prose, Queued, Role, Row,
    Screen, Section, Stage, Steps, Tap, Text, Toast, Tone, Toned, Tool, ToolState, Working,
};
use cooee_core::{Body, Item, agents::show, routines::local};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use topcoat::{
    context::{Cx, app_context, request_context},
    router::{Body as Bytes, request::Bytes as Raw, response::Response, route},
};

use crate::{
    Hub, HubError, Slice, Upload,
    http::{Answer, Data, Paired, Tag, blocking, core, header, image, refused, thread_id},
    live::{Live, PAGES, Source, Wake, Who},
    turns::{self, Again, Approve, Start},
};

const PATH: &str = "/screens/chat/";
const WINDOW: usize = 50;
const SUMMARY: usize = 80;
const DAY: u64 = 86_400_000;

/// `{type: screen, source: core, id: chat}`: thread `id`'s newest 50 items, an agent's turn as one `message`.
#[route(GET "/screens/chat/{id}")]
pub async fn chat(cx: &Cx) -> topcoat::Result<Answer<Block>> {
    let id = thread_id(cx);
    Ok(Answer(blocking(cx, move |hub| screen(hub, &id, 1)).await))
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
        build: Box::new(move |hub, pages| screen(hub, &built, pages)),
        seen: Some(Box::new(move |hub| read(hub, &seen))),
    })
}

fn read(hub: &Hub, id: &str) -> Result<(), HubError> {
    match hub.slice(id, None, 1, 0)?.items.last() {
        Some(newest) => hub.read(id, newest.id),
        None => Ok(()),
    }
}

fn screen(hub: &Hub, id: &str, pages: usize) -> Result<Block, HubError> {
    let (meta, _) = hub.thread(id)?;
    let slice = hub.slice(id, None, WINDOW.saturating_mul(pages), 0)?;
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
    let mut blocks = vec![];
    if slice.skipped_before > 0 && pages < PAGES {
        let tap = Tap {
            action: Some("older".to_owned()),
            ..Tap::default()
        };
        let older = Row::new("Older messages").id("older").tap(tap);
        blocks.push(Block::Row(Box::new(older)));
    }
    let log = Section::new(log(id, &slice, &name)).id("log");
    blocks.push(Block::Section(Box::new(log)));
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
        } else if let Body::Blocks(show::Reply { blocks, sent }) = &item.body {
            g.blocks.extend(blocks.blocks().iter().cloned());
            g.blocks.extend(sent.as_ref().map(|s| {
                let line = format!("✓ {} · {}", s.line, s.by);
                Block::Text(Box::new(
                    Text::new().id(format!("b{}-sent", item.id)).text(line),
                ))
            }));
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

pub(super) fn iso(at: u64) -> String {
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

fn tool_part(
    id: String,
    (subject, action): (&str, &str),
    status: &str,
    ruled: Option<&str>,
) -> Block {
    let state = match status {
        "completed" => ToolState::Done,
        "failed" | "cancelled" => ToolState::Failed,
        _ => ToolState::Running,
    };
    let summary: String = subject.chars().take(SUMMARY).collect();
    let name = action.to_lowercase();
    let tool = Tool::new(name, state).id(id);
    let tool = match ruled {
        Some(rule) => tool.body(format!("{subject}\n{rule}")),
        None if summary.len() < subject.len() => tool.body(subject.to_owned()),
        None => tool,
    };
    Block::Tool(Box::new(tool.summary(summary)))
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
        } => tool_part(
            id,
            (subject, &format!("{action:?}")),
            status,
            ruled.as_deref(),
        ),
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

fn q(random_id: &str) -> u64 {
    let hash = Sha256::digest(random_id.as_bytes());
    hash.iter().take(6).fold(0, |n, b| n << 8 | u64::from(*b))
}

fn composer(slice: &Slice) -> Block {
    let acts = ["queue", "steer", "unqueue", "stop"].map(|a| (a.to_owned(), a.to_owned()));
    let queued = slice.queue.iter().map(|m| Queued {
        q: q(&m.random_id),
        text: m.text.clone(),
    });
    let say = Composer::new("send")
        .id("say")
        .acts(BTreeMap::from(acts))
        .queued(queued.collect::<Vec<_>>());
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

pub(super) async fn act(cx: &Cx, a: &Action) -> Option<topcoat::Result<Response>> {
    let done = if let Some(persona) = a.screen.strip_prefix("/screens/chat?persona=") {
        begin(cx, persona.to_owned(), a).await
    } else {
        let id = a.screen.strip_prefix(PATH)?.to_owned();
        on(cx, id, a).await.map(|()| blocks::Answer::default())
    };
    Some(answer(done))
}

pub(super) fn answer(done: Result<blocks::Answer, HubError>) -> topcoat::Result<Response> {
    let (status, a) = match done {
        Ok(a) => (200, a),
        Err(e) => {
            let toast = Toast::new(e.to_string()).tone(Tone::Bad);
            let status = if matches!(e, HubError::NoThread(_)) {
                404
            } else {
                400
            };
            (
                status,
                blocks::Answer {
                    toast: Some(toast),
                    ..blocks::Answer::default()
                },
            )
        }
    };
    Ok(Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Bytes::from(serde_json::to_string(&a).unwrap_or_default()))?)
}

fn word(a: &Action, key: &str) -> String {
    let found = a.values.get(key).and_then(Value::as_str);
    found.unwrap_or_default().to_owned()
}

fn uploads(cx: &Cx) -> PathBuf {
    app_context::<Data>(cx).0.join("uploads")
}

async fn begin(cx: &Cx, persona: String, a: &Action) -> Result<blocks::Answer, HubError> {
    let nonce = word(a, "nonce");
    if a.action != "send" || nonce.is_empty() {
        return Err(refused("a new chat starts with a send and its nonce"));
    }
    let picked = blocking(cx, move |hub| {
        let all = hub.personas();
        let p = all.iter().find(|p| p.id == persona);
        p.map(|p| (p.id.clone(), p.agent.clone()))
            .ok_or_else(|| refused(format!("no persona {persona}")))
    })
    .await?;
    let start = Start {
        agent: picked.1,
        folder: PathBuf::new(),
        text: word(a, "text"),
        random_id: Tag(nonce),
        account: None,
        background: true,
        profile: Some(picked.0),
        model: String::new(),
        effort: String::new(),
        machine: None,
        setup: false,
    };
    let id = turns::start(cx, start).await?;
    Ok(blocks::Answer {
        open: Some(format!("{PATH}{id}")),
        ..blocks::Answer::default()
    })
}

async fn on(cx: &Cx, id: String, a: &Action) -> Result<(), HubError> {
    if let Some(item) = show::item_of(a) {
        let by = request_context::<Paired>(cx).0.label.clone();
        let (t, a) = (Again::of(cx, id), a.clone());
        return turns::off(move || t.act(item, &a, &by)).await;
    }
    let (text, nonce, dir) = (word(a, "text"), word(a, "nonce"), uploads(cx));
    let shown = a.values.get("q").and_then(Value::as_u64);
    let files = picked_files(a);
    let sent = move || {
        if nonce.is_empty() {
            return Err(refused("a send needs its nonce"));
        }
        Ok((nonce, held(&dir, &files)?))
    };
    let steer = a.action == "steer";
    match a.action.as_str() {
        "send" => {
            let t = Again::of(cx, id);
            turns::off(move || {
                let (nonce, files) = sent()?;
                t.send((&text, &nonce), &files, None).map(drop)
            })
            .await
        }
        "queue" | "steer" => {
            blocking(cx, move |hub| {
                queue_or_steer(hub, (&id, &text), shown, steer, sent)
            })
            .await
        }
        "unqueue" => {
            blocking(cx, move |hub| {
                let random_id = queued(hub, &id, shown.unwrap_or_default())?;
                hub.unqueue(&id, &random_id, false).map(drop)
            })
            .await
        }
        "stop" => blocking(cx, move |hub| hub.cancel(&id)).await,
        "answer" => card(cx, id, a).await,
        "handback" => {
            blocking(cx, move |hub| {
                let (nonce, _) = sent()?;
                hub.resume_handoff(&id, &text, &nonce).map(drop)
            })
            .await
        }
        "retry" => {
            let t = Again::of(cx, id);
            turns::off(move || t.retry()).await
        }
        "older" => {
            app_context::<Arc<Live>>(cx).more(&a.screen);
            Ok(())
        }
        "archive" => blocking(cx, move |hub| hub.archive(&id, true)).await,
        "close" => blocking(cx, move |hub| hub.close(&id)).await,
        other => Err(refused(format!("no action {other}"))),
    }
}

fn picked_files(a: &Action) -> Vec<String> {
    a.values
        .get("files")
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |all| {
            let id = |f: &Value| {
                f.as_str()
                    .or_else(|| f.get("id")?.as_str())
                    .map(str::to_owned)
            };
            all.iter().filter_map(id).collect()
        })
}

fn queue_or_steer(
    hub: &Hub,
    (id, text): (&str, &str),
    shown: Option<u64>,
    steer: bool,
    sent: impl FnOnce() -> Result<(String, Vec<Upload>), HubError>,
) -> Result<(), HubError> {
    if let Some(shown) = shown {
        let random_id = queued(hub, id, shown)?;
        hub.unqueue(id, &random_id, steer)?;
        if steer {
            return Ok(());
        }
    }
    let (nonce, files) = sent()?;
    hub.queue(id, text, &nonce, &files, None)?;
    if steer {
        hub.unqueue(id, &nonce, true)?;
    }
    Ok(())
}

fn queued(hub: &Hub, id: &str, shown: u64) -> Result<String, HubError> {
    let slice = hub.slice(id, None, 0, 0)?;
    let found = slice.queue.into_iter().find(|m| q(&m.random_id) == shown);
    found
        .map(|m| m.random_id)
        .ok_or_else(|| refused("that message is no longer queued"))
}

async fn card(cx: &Cx, id: String, a: &Action) -> Result<(), HubError> {
    let item = a.values.get("item").and_then(Value::as_u64).or_else(|| {
        let block = a.block.as_deref()?.strip_suffix("-answer")?;
        block.parse().ok()
    });
    let value = ["value", "other", "answer"]
        .iter()
        .find_map(|k| a.values.get(*k).and_then(Value::as_str));
    let approve = Approve {
        item: item.ok_or_else(|| refused("an answer names its card"))?,
        option: word(a, "choice"),
        value: value.map(str::to_owned),
        passkey: serde_json::from_value(Value::Object(a.values.clone())).ok(),
    };
    match turns::answer(cx, id, approve).await {
        Ok(Ok(done)) => done.map(drop),
        Ok(Err(why)) => Err(refused(why)),
        Err(e) => Err(refused(e)),
    }
}

#[derive(Serialize)]
struct Kept {
    id: String,
    name: String,
}

/// `POST /screens/files`: the composer's file handler; the body is the file, `X-Name` its name.
#[route(POST "/screens/files")]
pub async fn upload(cx: &Cx, body: Raw) -> topcoat::Result<Answer<Kept>> {
    let name = header(cx, "x-name").unwrap_or_default().to_owned();
    let dir = uploads(cx);
    Ok(Answer(turns::off(move || keep(&dir, name, &body)).await))
}

fn keep(dir: &Path, name: String, data: &[u8]) -> Result<Kept, HubError> {
    if name.is_empty() || name.starts_with('.') || name.contains(['/', '\\']) {
        return Err(refused("X-Name must be a file name"));
    }
    let now = crate::http::now();
    if let Ok(all) = fs::read_dir(dir) {
        for old in all.flatten() {
            let name = old.file_name();
            let at = name
                .to_str()
                .and_then(|n| n.split('-').next()?.parse::<u64>().ok());
            if at.is_some_and(|at| at.saturating_add(DAY) < now) {
                fs::remove_dir_all(old.path()).ok();
            }
        }
    }
    let random = cooee_core::random(6).map_err(refused)?;
    let random = random.iter().fold(0, |n, b| n << 8 | u64::from(*b));
    let id = format!("{now}-{random:x}");
    let held = dir.join(&id);
    fs::create_dir_all(&held).map_err(refused)?;
    fs::write(held.join(&name), data).map_err(refused)?;
    Ok(Kept { id, name })
}

fn held(dir: &Path, ids: &[String]) -> Result<Vec<Upload>, HubError> {
    let one = |id: &String| {
        let gone = || refused(format!("file {id} is gone: attach it again"));
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
            return Err(gone());
        }
        let mut held = fs::read_dir(dir.join(id)).map_err(|_| gone())?.flatten();
        let file = held.next().ok_or_else(gone)?;
        let data = fs::read(file.path()).map_err(refused)?;
        let name = file.file_name().to_string_lossy().into_owned();
        Ok(Upload { name, data })
    };
    ids.iter().map(one).collect()
}

pub(super) fn routes(b: topcoat::router::RouterBuilder) -> topcoat::router::RouterBuilder {
    b.route(chat).route(upload)
}
