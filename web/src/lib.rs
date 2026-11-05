//! cooee's web UI in Topcoat (docs/decisions/0001-ui-framework.md), in the Modern style: the thread list and the
//! chat page (server-rendered), the chat's script (`chat.js`: the window of items, SSE, the outbox and drafts), the
//! style sheet, the PWA manifest, and the look (`look.rs` themes and schemes, `look.js` applying a device's pick
//! before the first paint, its look sheet and Settings).

#![forbid(unsafe_code)]

mod look;

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use cooee_core::{Agent, Summary, Thread, Turn};
pub use look::{Looks, Pair, Scheme, TOKENS, Theme, gate, plain};
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        Body, RouterBuilder,
        error::not_found,
        path_param,
        response::{IntoResponse, Response},
        route,
    },
    view::{Child, View, component, view},
};

/// The static routes: the manifest, the icon, the style sheet and the chat's script; and the look: `look.js` with
/// every theme and scheme in it, and each theme's data at
/// `/look/<theme>/{theme.json,sprites.svg,icon.png}`.
///
/// # Panics
///
/// Never: [`Looks`] always serialises.
#[must_use]
pub fn assets(builder: RouterBuilder, looks: Looks) -> RouterBuilder {
    let js = format!(
        "const LOOK = {};\n{}",
        serde_json::to_string(&looks).expect("looks serialise"),
        include_str!("look.js")
    );
    builder
        .route(manifest)
        .route(icon)
        .route(style)
        .route(script)
        .route(look_script)
        .route(look_file)
        .app_context(Arc::new(LookAssets { looks, js }))
}

/// The looks and `look.js` built from them, once at start.
struct LookAssets {
    looks: Looks,
    js: String,
}

/// A file made at start, served with its content type.
struct Owned(&'static str, Vec<u8>);

impl IntoResponse for Owned {
    fn into_response(self, _cx: &Cx) -> Result<Response> {
        Ok(Response::builder()
            .header("Content-Type", self.0)
            .header("Cache-Control", "no-cache")
            .body(Body::from(self.1))?)
    }
}

#[route(GET "/look.js")]
async fn look_script(cx: &Cx) -> Result<Owned> {
    let look = app_context::<Arc<LookAssets>>(cx);
    Ok(Owned(
        "text/javascript; charset=utf-8",
        look.js.clone().into_bytes(),
    ))
}

path_param!(look);
path_param!(file);

#[route(GET "/look/{look}/{file}")]
async fn look_file(cx: &Cx) -> Result<Owned> {
    let theme = app_context::<Arc<LookAssets>>(cx)
        .looks
        .themes
        .get(path_param::<Look>(cx))
        .ok_or_else(not_found)?;
    let found = match path_param::<File>(cx) {
        "sprites.svg" => theme
            .sprite_sheet
            .clone()
            .map(|b| Owned("image/svg+xml", b)),
        "icon.png" => Some(theme.icon.clone().map_or_else(
            || Owned("image/svg+xml", include_bytes!("icon.svg").to_vec()),
            |b| Owned("image/png", b),
        )),
        "theme.json" => serde_json::to_vec(theme)
            .ok()
            .map(|b| Owned("application/json", b)),
        _ => None,
    };
    Ok(found.ok_or_else(not_found)?)
}

/// A fixed file served with its content type.
struct Static(&'static str, &'static str);

impl IntoResponse for Static {
    fn into_response(self, _cx: &Cx) -> Result<Response> {
        Ok(Response::builder()
            .header("Content-Type", self.0)
            .body(Body::from(self.1))?)
    }
}

#[route(GET "/manifest.webmanifest")]
async fn manifest() -> Result<Static> {
    Ok(Static(
        "application/manifest+json",
        include_str!("manifest.webmanifest"),
    ))
}

#[route(GET "/icon.svg")]
async fn icon() -> Result<Static> {
    Ok(Static("image/svg+xml", include_str!("icon.svg")))
}

#[route(GET "/app.css")]
async fn style() -> Result<Static> {
    Ok(Static("text/css; charset=utf-8", include_str!("app.css")))
}

#[route(GET "/chat.js")]
async fn script() -> Result<Static> {
    Ok(Static(
        "text/javascript; charset=utf-8",
        include_str!("chat.js"),
    ))
}

#[component]
async fn document(title: &str, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8">
                <meta
                    name="viewport"
                    content="width=device-width,initial-scale=1,viewport-fit=cover,interactive-widget=resizes-content"
                >
                <meta name="theme-color" content="#0f1a13">
                <title>(title)</title>
                <link rel="manifest" href="/manifest.webmanifest">
                <link rel="icon" href="/icon.svg" type="image/svg+xml">
                <link rel="apple-touch-icon" href="/icon.svg">
                <link rel="stylesheet" href="/app.css">
                <script src="/look.js"></script>
                <script type="module" src="/chat.js"></script>
            </head>
            <body>(child)</body>
        </html>
    })
}

/// `~/x` for a folder under `$HOME`.
fn short(folder: &Path) -> String {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    match home.as_deref().and_then(|h| folder.strip_prefix(h).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".to_owned(),
        Some(rest) => format!("~/{}", rest.display()),
        None => folder.display().to_string(),
    }
}

/// `now`, `5m`, `3h`, `2d` since `at` (Unix ms).
fn age(at: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    match now.saturating_sub(at / 1000) {
        s if s < 60 => "now".to_owned(),
        s if s < 3600 => format!("{}m", s / 60),
        s if s < 86_400 => format!("{}h", s / 3600),
        s => format!("{}d", s / 86_400),
    }
}

/// A turn state's key in the theme (`look.js` draws its sprite and words).
fn state(state: &str) -> &'static str {
    match state {
        "blocked" => "needs",
        "working" => "working",
        "error" => "fail",
        "done" => "done",
        "ended" => "ended",
        _ => "",
    }
}

/// The header: the app's name (the heading on the thread list, else a link to it), `+`, the look sheet and Settings.
#[component]
async fn header(home: bool) -> Result<impl View> {
    Ok(view! {
        <header class="bar">
            if home {
                <h1>"cooee"</h1>
            } else {
                <a class="brand" href="/">"cooee"</a>
            }
            <a
                class="gl"
                href=(if home { "#new" } else { "/#new" })
                aria-label="new thread"
            >
                "+"
            </a>
            <button
                type="button"
                class="gl"
                data-open="look"
                aria-label="look: theme and colours"
                aria-pressed="false"
            >
                "◐"
            </button>
            <button
                type="button"
                class="gl"
                data-open="settings"
                aria-label="settings"
                aria-pressed="false"
            >
                "⚙"
            </button>
        </header>
    })
}

#[component]
async fn row(r: Summary) -> Result<impl View> {
    let st = state(&r.state);
    let href = format!("/t/{}", r.id);
    let meta = format!("{} · {}", r.agent, age(r.at));
    Ok(view! {
        <a class=(if r.unread { "tr" } else { "tr rd" }) href=(href)>
            <i class="ud">(if r.unread { "●" } else { "" })</i>
            <span class="tt">(r.title)</span>
            <span class="ta">(meta)</span>
            <span class="tp">
                <b data-st=(st)></b>
                (r.last)
            </span>
        </a>
    })
}

/// The thread list: one card per working folder, its 3 newest threads shown, the rest folded; and the new-thread
/// form (open when there are no threads, else behind `+`).
#[must_use]
pub fn threads_page(
    cx: &Cx,
    rows: Vec<Summary>,
    agents: Result<Vec<Agent>, String>,
    folders: Vec<PathBuf>,
) -> impl View {
    view! { cx => threads(rows: rows, agents: agents, folders: folders) }
}

#[component]
async fn threads(
    rows: Vec<Summary>,
    agents: Result<Vec<Agent>, String>,
    folders: Vec<PathBuf>,
) -> Result<impl View> {
    let empty = rows.is_empty();
    let mut cards: BTreeMap<PathBuf, Vec<Summary>> = BTreeMap::new();
    for r in rows {
        cards.entry(r.folder.clone()).or_default().push(r);
    }
    Ok(view! {
        document(
            title: "cooee",
            header(home: true)
            <main class="list">
                if empty {
                    <div class="emp">
                        <span data-s="empty"></span>
                        <p data-w="empty-threads" data-plain=""></p>
                    </div>
                }
                <form class="nf" id="new" hidden=(!empty)>
                    <div class="fh">
                        <b>"new thread"</b>
                        <a class="gl" href="#" aria-label="close">"×"</a>
                    </div>
                    <fieldset class="ag">
                        <legend class="lb">"agent"</legend>
                        match agents {
                            Ok(list) => {
                                for (i, a) in list.into_iter().enumerate() {
                                    <label>
                                        <input
                                            type="radio"
                                            name="agent"
                                            value=(a.id.clone())
                                            checked=(i == 0)
                                        >
                                        <span>(a.id)</span>
                                    </label>
                                }
                            }
                            Err(e) => {
                                <p class="err">(e)</p>
                            }
                        }
                    </fieldset>
                    <label class="lb" for="folder">"folder"</label>
                    <select id="folder" name="folder">
                        for f in folders {
                            <option value=(f.display().to_string())>(short(&f))</option>
                        }
                    </select>
                    <div class="tw">
                        <textarea
                            name="text"
                            rows="2"
                            aria-label="first message"
                            placeholder="message"
                        ></textarea>
                        <button type="submit">"start"</button>
                    </div>
                    <p class="err" id="new-err" hidden=(true)></p>
                </form>
                for (folder, rows) in cards {
                    <section class="fc">
                        <div class="fh">
                            <b>(short(&folder))</b>
                            <span class="ds">(format!("{} threads", rows.len()))</span>
                        </div>
                        for r in rows.iter().take(3).cloned() {
                            row(r: r)
                        }
                        if rows.len() > 3 {
                            <details>
                                <summary>
                                    (format!("{} older threads", rows.len() - 3))
                                </summary>
                                for r in rows.iter().skip(3).cloned() {
                                    row(r: r)
                                }
                            </details>
                        }
                    </section>
                }
            </main>
        )
    })
}

/// One thread: its bar, the items (drawn by `chat.js`), the working pill and the composer.
#[must_use]
pub fn chat_page(cx: &Cx, thread: Thread, turn: Turn) -> impl View {
    view! { cx => chat(thread: thread, turn: turn) }
}

#[component]
async fn chat(thread: Thread, turn: Turn) -> Result<impl View> {
    let folder = thread.folder.as_deref().map(short).unwrap_or_default();
    let ended = thread.closed || turn.state == "ended";
    Ok(view! {
        document(
            title: thread.title.as_str(),
            <main
                class="chat"
                data-thread=(thread.id.clone())
                data-agent=(thread.agent.clone())
                data-read=(thread.read.unwrap_or(0).to_string())
                data-state=(turn.state.clone())
            >
                header(home: false)
                <div class="th">
                    <a class="gl bk" href="/" aria-label="back to threads">"‹"</a>
                    <div class="tb">
                        <h1>(thread.title.clone())</h1>
                        <span class="ds">
                            <i class="dt"></i>
                            (format!("{} · {folder} · ", thread.agent))
                            <span id="st" data-st=(state(&turn.state))></span>
                        </span>
                    </div>
                    <button
                        type="button"
                        id="close"
                        class="gl"
                        hidden=(ended)
                        aria-label="close the thread"
                    >
                        "×"
                    </button>
                </div>
                <ol
                    class="tc"
                    id="msgs"
                    tabindex="0"
                    aria-label="messages"
                    aria-live="polite"
                ></ol>
                <p class="tlv" id="wait" hidden=(true)>
                    <span data-st="needs" data-plain=""></span>
                </p>
                <form class="tw" id="composer">
                    <div class="pill" id="pill" hidden=(true)>
                        <span id="pst" data-st="working" data-plain=""></span>
                        <span id="el">"0:00"</span>
                        <button type="button" class="stop">"stop"</button>
                    </div>
                    <textarea
                        id="box"
                        rows="1"
                        aria-label="message"
                        placeholder=(format!("message {}", thread.agent))
                        disabled=(ended)
                    ></textarea>
                    <button type="submit" disabled=(ended)>"send"</button>
                </form>
            </main>
        )
    })
}
