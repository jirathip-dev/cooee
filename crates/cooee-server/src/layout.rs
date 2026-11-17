use std::{
    collections::HashSet,
    fs, io,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use futures_util::{Stream, StreamExt, stream};
use serde_json::{Value, json};
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

const KINDS: [(&str, &[&str], &[&str]); 8] = [
    ("nav", &["side", "main", "menu"], &[]),
    ("sidebar", &["top", "list"], &["width", "pinned"]),
    ("chat", &["strip", "cards"], &["composer", "stop", "back"]),
    ("composer", &[], &["attach", "queue", "hint"]),
    ("terminal", &[], &["rows", "cols", "machine"]),
    ("scanner", &[], &[]),
    ("menu-host", &["rows"], &[]),
    ("settings", &[], &["pages"]),
];
const PLACES: [&str; 9] = [
    "push", "sheet", "panel", "column", "strip", "cards", "menu", "inline", "hidden",
];
const TRANSITIONS: [&str; 4] = ["push", "sheet", "fade", "none"];
const LOCKED: [&str; 2] = ["/screens/pairing", "/screens/reset-layout"];

/// Why a layout was refused, and where: `line` and `col` count from 1.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Broken {
    /// The line at fault.
    pub line: usize,
    /// Its column.
    pub col: usize,
    /// The rule it breaks, with the node's JSON path.
    pub why: String,
}

/// `text` as a layout, refused by the app's rules; a refusal points at the offending text.
///
/// # Errors
/// The first rule `text` breaks.
pub fn check(text: &str) -> Result<Value, Broken> {
    let at = |needle: &str, nth: usize, why: String| {
        let i = text.match_indices(needle).nth(nth).map_or(0, |(i, _)| i);
        let line = text[..i].matches('\n').count() + 1;
        let col = i - text[..i].rfind('\n').map_or(0, |n| n + 1) + 1;
        Broken { line, col, why }
    };
    if text.len() > 64 * 1024 {
        return Err(at("", 0, "over 65536 bytes".into()));
    }
    let file: Value = serde_json::from_str(text).map_err(|e| Broken {
        line: e.line(),
        col: e.column(),
        why: e.to_string(),
    })?;
    let format = file.get("format").unwrap_or(&Value::Null);
    if format.as_f64() != Some(1.0) {
        return Err(at("\"format\"", 0, format!("format {format} is not 1")));
    }
    let root = file.get("root").unwrap_or(&Value::Null);
    if root.get("type").is_none_or(|t| t != "nav") {
        return Err(at("\"root\"", 0, "root: not a nav".into()));
    }
    let mut ids: HashSet<String> = ["pairing", "reset-layout"].map(str::to_owned).into();
    node(root, "root", 1, &mut ids).map_err(|(why, bad, nth)| at(&format!("{bad:?}"), nth, why))?;
    Ok(file)
}

fn node(
    n: &Value,
    path: &str,
    depth: usize,
    ids: &mut HashSet<String>,
) -> Result<(), (String, String, usize)> {
    let fail = |what: &str, bad: &str| Err((format!("{path}: {what} {bad:?}"), bad.to_owned(), 0));
    if depth > 6 || ids.len() > 201 {
        return fail("over 6 deep or 200 nodes at", "type");
    }
    let kind = n["type"].as_str().unwrap_or_default();
    let source = n["source"].as_str().unwrap_or_default();
    let screen = n["path"].as_str().unwrap_or_default();
    let (slots, config) = if kind == "screen" {
        if source.is_empty() || !screen.starts_with("/screens/") {
            return fail(
                "a screen needs a source and a path under /screens/:",
                "screen",
            );
        }
        if LOCKED.contains(&screen) {
            return Ok(());
        }
        (&[][..], &[][..])
    } else {
        match KINDS.iter().find(|k| k.0 == kind) {
            Some((_, slots, config)) => (*slots, *config),
            None => return fail("unknown type", kind),
        }
    };
    let fallback = if kind == "screen" {
        format!("{source}:{screen}")
    } else {
        kind.to_owned()
    };
    let id = n["id"].as_str().map_or(fallback, str::to_owned);
    if !ids.insert(id.clone()) {
        return Err((format!("{path}: id {id:?} is taken"), id, 1));
    }
    let place = &n["place"];
    let named = [place, &place["compact"], &place["wide"]].map(Value::as_str);
    if let Some(p) = named.iter().flatten().find(|p| !PLACES.contains(p)) {
        return fail("unknown place", p);
    }
    if !place.is_null() && named == [None; 3] {
        return fail("no place in", "place");
    }
    if let Some(t) = n.get("transition") {
        let name = t.as_str().unwrap_or_default();
        if !TRANSITIONS.contains(&name) {
            return fail(
                "unknown transition",
                if name.is_empty() { "transition" } else { name },
            );
        }
    }
    for k in n["config"].as_object().into_iter().flat_map(|o| o.keys()) {
        if !config.contains(&k.as_str()) {
            return fail("unknown config", k);
        }
    }
    for (slot, children) in n["slots"].as_object().into_iter().flatten() {
        if !slots.contains(&slot.as_str()) {
            return fail("unknown slot", slot);
        }
        for (i, child) in children.as_array().into_iter().flatten().enumerate() {
            node(child, &format!("{path}.slots.{slot}[{i}]"), depth + 1, ids)?;
        }
    }
    Ok(())
}

fn merged(mut layout: Value, plugins: &[Manifest]) -> (Value, Vec<String>) {
    let mut placed = Vec::new();
    sources(layout.get("root").unwrap_or(&Value::Null), &mut placed);
    for m in plugins.iter().filter(|m| !placed.contains(&m.name)) {
        let screens = m
            .places
            .iter()
            .filter(|p| p.screen.starts_with("/screens/"));
        for (i, p) in screens.enumerate() {
            let (kind, slot, place) = match p.kind {
                PlaceKind::HeaderStrip => ("chat", "strip", "strip"),
                PlaceKind::ChatCards => ("chat", "cards", "cards"),
                PlaceKind::MenuRow => ("nav", "menu", "menu"),
            };
            let Some(host) = layout.get_mut("root").and_then(|r| find(r, kind)) else {
                continue;
            };
            let Some(slots) = host
                .as_object_mut()
                .map(|h| h.entry("slots").or_insert(Value::Null))
            else {
                continue;
            };
            if slots.is_null() {
                *slots = json!({});
            }
            let Some(list) = slots
                .as_object_mut()
                .map(|s| s.entry(slot).or_insert(Value::Null))
            else {
                continue;
            };
            if list.is_null() {
                *list = json!([]);
            }
            if let Some(list) = list.as_array_mut() {
                list.push(
                    json!({ "type": "screen", "id": format!("{}-{i}", m.name), "source": m.name,
                                  "path": p.screen, "label": p.label, "place": place }),
                );
            }
        }
    }
    let mut skipped = Vec::new();
    if let Some(root) = layout.get_mut("root") {
        prune(root, "root", plugins, &mut skipped);
    }
    (layout, skipped)
}

fn sources(n: &Value, out: &mut Vec<String>) {
    out.extend(n["source"].as_str().map(str::to_owned));
    for children in n["slots"].as_object().into_iter().flat_map(|o| o.values()) {
        for child in children.as_array().into_iter().flatten() {
            sources(child, out);
        }
    }
}

fn find<'a>(n: &'a mut Value, kind: &str) -> Option<&'a mut Value> {
    if n["type"] == kind {
        return Some(n);
    }
    let slots = n.get_mut("slots")?.as_object_mut()?.values_mut();
    slots
        .flat_map(|c| c.as_array_mut().into_iter().flatten())
        .find_map(|c| find(c, kind))
}

fn prune(n: &mut Value, path: &str, plugins: &[Manifest], skipped: &mut Vec<String>) {
    let Some(slots) = n.get_mut("slots").and_then(Value::as_object_mut) else {
        return;
    };
    for (slot, children) in slots {
        let Some(list) = children.as_array_mut() else {
            continue;
        };
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

fn refused(n: &Value, plugins: &[Manifest]) -> Option<String> {
    let source = n["source"]
        .as_str()
        .filter(|s| n["type"] == "screen" && *s != "core")?;
    let Some(m) = plugins.iter().find(|m| m.name == source) else {
        return Some(format!("no enabled plugin {source:?}"));
    };
    let place = &n["place"];
    let named: Vec<&str> = [place, &place["compact"], &place["wide"]]
        .iter()
        .filter_map(|p| p.as_str())
        .collect();
    let needs = |p: &str| match p {
        "hidden" => None,
        "cards" => Some(PlaceKind::ChatCards),
        "menu" => Some(PlaceKind::MenuRow),
        _ => Some(PlaceKind::HeaderStrip),
    };
    let places = if named.is_empty() {
        vec!["inline"]
    } else {
        named
    };
    let bad = places
        .into_iter()
        .find(|p| needs(p).is_some_and(|k| !m.permissions.places.contains(&k)))?;
    Some(format!("{source} may not be placed {bad:?}"))
}

struct Served {
    etag: String,
    layout: Value,
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
    #[must_use]
    pub fn watch(dir: Option<PathBuf>) -> Arc<Self> {
        let me = Arc::new(Self {
            file: dir.map(|d| d.join("layout.json")),
            served: Mutex::new(Served {
                etag: cooee_core::sha256(DEFAULT.as_bytes()),
                layout: check(DEFAULT).unwrap_or_default(),
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
        me
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
    let installed = plugins::read_lock(data).unwrap_or_default();
    let enabled = installed.iter().filter(|l| l.enabled);
    enabled
        .filter_map(|l| plugins::manifest(data, l).ok())
        .collect()
}

fn answer(cx: &Cx) -> Result<Response> {
    let l = layouts(cx);
    let s = lock(&l.served);
    let (layout, skipped) = merged(s.layout.clone(), &manifests(cx));
    let etag = if layout == s.layout {
        s.etag.clone()
    } else {
        cooee_core::sha256(format!("{}{layout}", s.etag).as_bytes())
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
