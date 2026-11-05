//! cooee's web UI in Topcoat (docs/decisions/0001-ui-framework.md), in the Modern style: the thread list and the
//! chat page (server-rendered), the pairing page, the chat's script (`chat.js`: the window of items, SSE, the outbox,
//! drafts and turning alerts on), the style sheet and the installable PWA: its manifest, icons and the service worker
//! (`sw.js`: push only, nothing cached).

#![forbid(unsafe_code)]

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use cooee_core::{Agent, Summary, Thread, Turn};
use topcoat::{
    Result,
    context::Cx,
    router::{
        Body, RouterBuilder,
        response::{IntoResponse, Response},
        route,
    },
    view::{Child, View, component, view},
};

/// The static routes: the manifest, the icons, the style sheet, the chat's script and the service worker.
#[must_use]
pub fn assets(builder: RouterBuilder) -> RouterBuilder {
    builder
        .route(manifest)
        .route(icon)
        .route(icon_192)
        .route(icon_512)
        .route(style)
        .route(script)
        .route(worker)
}

/// A fixed file served with its content type.
struct Static(&'static str, &'static [u8]);

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
        include_bytes!("manifest.webmanifest"),
    ))
}

#[route(GET "/icon.svg")]
async fn icon() -> Result<Static> {
    Ok(Static("image/svg+xml", include_bytes!("icon.svg")))
}

#[route(GET "/icon-192.png")]
async fn icon_192() -> Result<Static> {
    Ok(Static("image/png", include_bytes!("../icons/icon-192.png")))
}

#[route(GET "/icon-512.png")]
async fn icon_512() -> Result<Static> {
    Ok(Static("image/png", include_bytes!("../icons/icon-512.png")))
}

#[route(GET "/sw.js")]
async fn worker() -> Result<Static> {
    Ok(Static(
        "text/javascript; charset=utf-8",
        include_bytes!("sw.js"),
    ))
}

#[route(GET "/app.css")]
async fn style() -> Result<Static> {
    Ok(Static("text/css; charset=utf-8", include_bytes!("app.css")))
}

#[route(GET "/chat.js")]
async fn script() -> Result<Static> {
    Ok(Static(
        "text/javascript; charset=utf-8",
        include_bytes!("chat.js"),
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
                <link rel="apple-touch-icon" href="/icon-192.png">
                <link rel="stylesheet" href="/app.css">
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

/// A state's colour class and label.
fn state(state: &str) -> (&'static str, &'static str) {
    match state {
        "blocked" => ("r", "! needs you"),
        "working" => ("a", "✻ working"),
        "error" => ("r", "✕ error"),
        "done" => ("m", "✓ done"),
        "ended" => ("m", "■ ended"),
        _ => ("m", ""),
    }
}

#[component]
async fn row(r: Summary) -> Result<impl View> {
    let (class, label) = state(&r.state);
    let href = format!("/t/{}", r.id);
    let meta = format!("{} · {}", r.agent, age(r.at));
    Ok(view! {
        <a class=(if r.unread { "tr" } else { "tr rd" }) href=(href)>
            <i class="ud">(if r.unread { "●" } else { "" })</i>
            <span class="tt">(r.title)</span>
            <span class="ta">(meta)</span>
            <span class="tp">
                <b class=(class)>(label)</b>
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
            <header class="bar">
                <h1>"cooee"</h1>
                <button
                    type="button"
                    class="gl"
                    id="alerts"
                    hidden=(true)
                    aria-label="turn on notifications"
                >
                    "🔔"
                </button>
                <a class="gl" href="#new" aria-label="new thread">"+"</a>
            </header>
            <main class="list">
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

/// Pairing: the code `cooee pair` printed (`code` prefilled from its QR link; `bad` after a wrong one).
#[must_use]
pub fn pair_page(cx: &Cx, code: String, bad: bool) -> impl View {
    view! { cx => pair(code: code, bad: bad) }
}

#[component]
async fn pair(code: String, bad: bool) -> Result<impl View> {
    Ok(view! {
        document(
            title: "pair · cooee",
            <header class="bar">
                <h1>"cooee"</h1>
            </header>
            <main class="list">
                <form class="nf" method="post" action="/pair">
                    <div class="fh">
                        <b>"pair this device"</b>
                    </div>
                    <p class="ds">
                        "Run cooee pair on your machine and enter the code it prints. On a phone, add cooee to the Home Screen first and pair there."
                    </p>
                    <div class="tw">
                        <input
                            name="code"
                            aria-label="pairing code"
                            inputmode="numeric"
                            autocomplete="one-time-code"
                            maxlength="6"
                            required=(true)
                            value=(code)
                        >
                        <button type="submit">"pair"</button>
                    </div>
                    <p class="err" hidden=(!bad)>
                        "That code is wrong or expired: run cooee pair again."
                    </p>
                </form>
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
    let (_, label) = state(&turn.state);
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
                <div class="th">
                    <a class="gl bk" href="/" aria-label="back to threads">"‹"</a>
                    <div class="tb">
                        <h1>(thread.title.clone())</h1>
                        <span class="ds">
                            <i class="dt"></i>
                            (format!("{} · {folder} · ", thread.agent))
                            <span id="st">(label)</span>
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
                <p class="tlv" id="wait" hidden=(true)>"● waiting for you"</p>
                <form class="tw" id="composer">
                    <div class="pill" id="pill" hidden=(true)>
                        "✻ working · "
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
