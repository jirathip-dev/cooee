pub use blocks::layout::Broken;
use std::{
    fs, io,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use blocks::layout::{self, Component, Content, Contract, Layout, Node, Place, PlaceEntry};
use futures_util::{Stream, StreamExt, stream};
use serde_json::json;
use tokio::sync::broadcast;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        Body,
        content::sse::{Event, KeepAlive, Sse},
        response::Response,
        route,
    },
};

use cooee_core::plugins::{self, Manifest, PlaceKind};

use crate::{
    http::{Data, header, plain},
    hub::lock,
};

const DEFAULT: &str = include_str!("../../../apple/Sources/CooeeUI/Default/layout.json");

static COOEE: Contract = Contract {
    components: &[
        Component {
            kind: "sidebar",
            slots: &["top", "list"],
            config: &["width", "pinned"],
        },
        Component {
            kind: "chat",
            slots: &["strip", "cards"],
            config: &["composer", "stop", "back"],
        },
        Component {
            kind: "composer",
            slots: &[],
            config: &["attach", "queue", "hint"],
        },
        Component {
            kind: "terminal",
            slots: &[],
            config: &["rows", "cols", "machine"],
        },
        Component {
            kind: "scanner",
            slots: &[],
            config: &[],
        },
        Component {
            kind: "menu-host",
            slots: &["rows"],
            config: &[],
        },
        Component {
            kind: "settings",
            slots: &[],
            config: &["pages"],
        },
    ],
    locked: &["/screens/pairing", "/screens/reset-layout"],
};

/// `text` as a layout, refused by `blocks::layout`'s rules with cooee's contract; a refusal points at the offending text.
///
/// # Errors
/// The first rule `text` breaks.
pub fn check(text: &str) -> Result<Layout, Broken> {
    layout::check(text, &COOEE)
}

fn merged(l: Layout, plugins: &[Manifest]) -> (Layout, Vec<String>) {
    let places: Vec<PlaceEntry> = plugins
        .iter()
        .flat_map(|m| {
            let screens = m
                .places
                .iter()
                .filter(|p| p.screen.starts_with("/screens/"));
            screens.enumerate().map(|(i, p)| PlaceEntry {
                kind: match p.kind {
                    PlaceKind::HeaderStrip => layout::PlaceKind::HeaderStrip,
                    PlaceKind::ChatCards => layout::PlaceKind::ChatCards,
                    PlaceKind::MenuRow => layout::PlaceKind::MenuRow,
                },
                source: m.name.clone(),
                id: format!("{}-{i}", m.name),
                label: Some(p.label.clone()),
                screen: p.screen.clone(),
            })
        })
        .collect();
    let (mut l, mut skipped) = layout::merge(l, &places, &COOEE);
    prune(&mut l.root, "root", plugins, &mut skipped);
    (l, skipped)
}

fn prune(n: &mut Node, path: &str, plugins: &[Manifest], skipped: &mut Vec<String>) {
    for (slot, list) in &mut n.slots {
        let mut i = 0;
        list.retain_mut(|child| {
            let at = format!("{path}.slots.{slot}[{i}]");
            i += 1;
            if let Some(why) = refused(child, plugins) {
                skipped.push(format!("{at}: {why}"));
                return false;
            }
            prune(child, &at, plugins, skipped);
            true
        });
    }
}

fn refused(n: &Node, plugins: &[Manifest]) -> Option<String> {
    let Content::Screen { source, .. } = &n.content else {
        return None;
    };
    if source == "core" {
        return None;
    }
    let Some(m) = plugins.iter().find(|m| &m.name == source) else {
        return Some(format!("no enabled plugin {source:?}"));
    };
    let needs = |p: Place| match p {
        Place::Hidden => None,
        Place::Cards => Some(PlaceKind::ChatCards),
        Place::Menu => Some(PlaceKind::MenuRow),
        _ => Some(PlaceKind::HeaderStrip),
    };
    let bad = [n.place.compact, n.place.wide]
        .into_iter()
        .find(|p| needs(*p).is_some_and(|k| !m.permissions.places.contains(&k)))?;
    let bad = serde_json::to_string(&bad).unwrap_or_default();
    Some(format!("{source} may not be placed {bad}"))
}

struct Served {
    etag: String,
    layout: Layout,
    broken: Option<Broken>,
    seen: Option<String>,
}

/// One instance's layout: its file (none: no config folder, the default only) and its subscribers.
pub struct Layouts {
    file: Option<PathBuf>,
    served: Mutex<Served>,
    tx: broadcast::Sender<(&'static str, String)>,
}

impl Layouts {
    /// `layout.json` in `dir`, read now and every second from now on.
    ///
    /// # Errors
    /// The bundled default breaks a rule.
    pub fn watch(dir: Option<PathBuf>) -> Result<Arc<Self>, Broken> {
        let me = Arc::new(Self {
            file: dir.map(|d| d.join("layout.json")),
            served: Mutex::new(Served {
                etag: cooee_core::sha256(DEFAULT.as_bytes()),
                layout: check(DEFAULT)?,
                broken: None,
                seen: None,
            }),
            tx: broadcast::channel(16).0,
        });
        let again = Arc::clone(&me);
        std::thread::spawn(move || {
            loop {
                again.reload();
                cooee_core::clock::sleep(Duration::from_secs(1));
            }
        });
        Ok(me)
    }

    fn reload(&self) {
        let text = self.file.as_ref().and_then(|f| fs::read_to_string(f).ok());
        let mut s = lock(&self.served);
        if text == s.seen {
            return;
        }
        s.seen.clone_from(&text);
        let text = text.as_deref().unwrap_or(DEFAULT);
        let said = match check(text) {
            Ok(layout) => {
                (s.etag, s.layout, s.broken) = (cooee_core::sha256(text.as_bytes()), layout, None);
                ("changed", json!({ "etag": s.etag }))
            }
            Err(b) => ("broken", json!(s.broken.insert(b))),
        };
        self.tx.send((said.0, said.1.to_string())).ok();
    }

    fn reset(&self) -> io::Result<()> {
        let Some(file) = &self.file else {
            return Err(io::Error::other("no config folder"));
        };
        if let Err(e) = fs::copy(file, file.with_extension("json.bak"))
            && e.kind() != io::ErrorKind::NotFound
        {
            return Err(e);
        }
        cooee_core::write_private(file, DEFAULT.as_bytes())?;
        self.reload();
        Ok(())
    }
}

fn layouts(cx: &Cx) -> Arc<Layouts> {
    Arc::clone(app_context::<Arc<Layouts>>(cx))
}

/// `{etag, layout}` as served, with its `ETag`; 304 when `If-None-Match` names it.
#[route(GET "/layout")]
pub async fn get(cx: &Cx) -> Result<Response> {
    answer(cx)
}

fn manifests(cx: &Cx) -> Vec<Manifest> {
    let data = &app_context::<Data>(cx).0;
    let installed = plugins::installed(data).unwrap_or_default();
    installed
        .iter()
        .filter(|(l, _)| l.enabled)
        .map(|(_, m)| m.clone())
        .collect()
}

fn answer(cx: &Cx) -> Result<Response> {
    let l = layouts(cx);
    let s = lock(&l.served);
    let (layout, skipped) = merged(s.layout.clone(), &manifests(cx));
    let etag = if layout == s.layout {
        s.etag.clone()
    } else {
        let json = serde_json::to_string(&layout).unwrap_or_default();
        cooee_core::sha256(format!("{}{json}", s.etag).as_bytes())
    };
    let tag = format!("\"{etag}\"");
    let same = header(cx, "if-none-match").is_some_and(|m| m.split(',').any(|m| m.trim() == tag));
    let body = if same {
        String::new()
    } else {
        json!({ "etag": etag, "layout": layout, "skipped": skipped }).to_string()
    };
    Ok(Response::builder()
        .status(if same { 304 } else { 200 })
        .header("ETag", tag)
        .header("Content-Type", "application/json")
        .body(Body::from(body))?)
}

/// SSE: `broken` first while the file is, then `changed {etag}` and `broken {line, col, why}` as they happen.
#[route(GET "/layout/events")]
pub async fn events(cx: &Cx) -> Result<Sse<impl Stream<Item = Result<Event>> + use<>>> {
    let l = layouts(cx);
    let rx = l.tx.subscribe();
    let first = lock(&l.served)
        .broken
        .as_ref()
        .map(|b| ("broken", json!(b).to_string()));
    let rest = stream::unfold(
        rx,
        |mut rx| async move { rx.recv().await.ok().map(|e| (e, rx)) },
    );
    let all = stream::iter(first).chain(rest);
    let all = all.map(|(name, data)| Ok(Event::new().event(name).data(data)));
    Ok(Sse::new(all).keep_alive(KeepAlive::new()))
}

/// Writes the default (the old file kept as `.bak`) and answers it as `GET /layout` does.
#[route(POST "/layout/reset")]
pub async fn reset(cx: &Cx) -> Result<Response> {
    let l = layouts(cx);
    match tokio::task::spawn_blocking(move || l.reset()).await? {
        Ok(()) => answer(cx),
        Err(e) => plain(409, &format!("layout.json can't be reset: {e}")),
    }
}
