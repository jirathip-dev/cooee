//! cooee's web UI, HTML-first (#49): Topcoat renders every page on the server (docs/decisions/0001-ui-framework.md),
//! in the device's look from its cookie (`shown.rs`), and the sheets, menus and forms are native HTML (`sheet.rs`)
//! that work with no script. Strict TypeScript in `web/ts` (stripped by `build.rs`) does only what needs script:
//! `chat.ts` (the live chat over SSE, the scroll window, drafts, the offline copy, report mode, clipboard, push),
//! `look.ts` (a pick applied in place, the marks of what a page adds), `frame.ts` (a plugin frame's bridge) and
//! `sw.ts` (the service worker). Also the style sheet, the PWA's manifest and icons, and the themes' files.

#![forbid(unsafe_code)]

mod compose;
mod look;
mod sheet;
mod shown;

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

pub use compose::{New, named, who};
use compose::{composer, side};
use cooee_core::{Sheet, Summary, Thread, Turn};
pub use look::{Looks, Pair, Scheme, TOKENS, Theme, gate, plain};
use sheet::{sheets, shown};
pub use shown::Shown;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        Body, RouterBuilder,
        content::Form,
        error::not_found,
        path_param,
        response::{IntoResponse, Response},
        route,
    },
    view::{Child, Unescaped, View, component, view},
};

/// A `web/ts` entry as `build.rs` stripped it.
macro_rules! built {
    ($name:literal) => {
        include_str!(concat!(env!("OUT_DIR"), "/", $name))
    };
}

/// The static routes: the manifest, the icons, the style sheet, the scripts and the service worker; the look:
/// `POST /look` (a pick with no script) and each theme's data at `/look/<theme>/{theme.json,sprites.svg,icon.png}`;
/// and `POST /list`, the thread list's chip.
///
/// # Panics
///
/// Never: [`Looks`] always serialises.
#[must_use]
pub fn assets(builder: RouterBuilder, looks: Looks) -> RouterBuilder {
    // In a <script> element: no `</` may end it.
    let json = serde_json::to_string(&looks)
        .expect("looks serialise")
        .replace("</", "<\\/");
    builder
        .route(manifest)
        .route(icon)
        .route(icon_192)
        .route(icon_512)
        .route(style)
        .route(script)
        .route(worker)
        .route(look_js)
        .route(sheet::look_set)
        .route(list_set)
        .route(look_file)
        .app_context(Arc::new(LookAssets { looks, json }))
}

/// The looks, and their JSON for the pages' `<script id="looks">`, once at start.
struct LookAssets {
    looks: Looks,
    json: String,
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

path_param!(look);
path_param!(file);

/// A sprite sheet with its two-frame loop rule, since a `<use>` of another file takes that file's styles: frame 1 (`.kf1`,
/// `.f1`) shows by default, and `--o1`/`--o2`, which the page animates, inherit into each sprite.
fn with_frames(svg: &[u8]) -> Vec<u8> {
    const RULE: &[u8] =
        b"<style>.kf1,.f1{opacity:var(--o1,1)}.kf2,.f2{opacity:var(--o2,0)}</style>";
    let at = svg
        .windows(4)
        .position(|w| w == b"<svg")
        .and_then(|i| svg[i..].iter().position(|&b| b == b'>').map(|j| i + j + 1))
        .unwrap_or(0);
    [&svg[..at], RULE, &svg[at..]].concat()
}

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
            .as_deref()
            .map(|b| Owned("image/svg+xml", with_frames(b))),
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

/// The plugin API inside a plugin's frame (`frame.ts`): the server puts it before the plugin's module.
pub const FRAME_JS: &str = built!("frame.js");

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
        built!("sw.js").as_bytes(),
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
        built!("chat.js").as_bytes(),
    ))
}

#[route(GET "/look.js")]
async fn look_js() -> Result<Static> {
    Ok(Static(
        "text/javascript; charset=utf-8",
        built!("look.js").as_bytes(),
    ))
}

/// A page in the device's look, drawn from the first byte: the scheme's tokens, `data-*` on `<html>` and the
/// theme's icon; the sheets after the content (`providers`: Settings › providers).
#[component]
async fn document(
    cx: &Cx,
    title: &str,
    #[default] providers: Vec<Sheet>,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let look = shown(cx);
    let name = look.name().to_owned();
    let bg = look
        .scheme_of(look.scheme(look.mode().unwrap_or("dark")))
        .tokens
        .get("--bg")
        .cloned();
    let ico = format!("/look/{name}/icon.png");
    let json = app_context::<Arc<LookAssets>>(cx).json.clone();
    Ok(view! {
        <!DOCTYPE html>
        <html
            lang="en"
            data-theme=(name)
            data-mode=(look.mode().map(str::to_owned))
            data-style=(look.style().to_owned())
            data-figures=(look.figures().to_owned())
            data-moments=(if look.theme().moments { "on" } else { "off" })
        >
            <head>
                <meta charset="utf-8">
                <meta
                    name="viewport"
                    content="width=device-width,initial-scale=1,viewport-fit=cover,interactive-widget=resizes-content"
                >
                <meta name="theme-color" content=(bg)>
                <title>(title)</title>
                <link rel="manifest" href="/manifest.webmanifest">
                <link rel="icon" href=(ico.clone())>
                <link rel="apple-touch-icon" href=(ico)>
                <link rel="stylesheet" href="/app.css">
                <style id="scheme">(Unescaped::new_unchecked(look.css()))</style>
                <script type="application/json" id="looks">
                    (Unescaped::new_unchecked(json))
                </script>
                <script type="module" src="/look.js"></script>
                <script type="module" src="/chat.js"></script>
            </head>
            <body class="app" data-at=(now().to_string())>
                (child)
                <p id="ld" role="status" hidden=(true)>
                    <span class="lw" data-w="loading">
                        (Unescaped::new_unchecked(look.w("loading", false)))
                    </span>
                    <span class="ago"></span>
                </p>
                <p id="toast" role="status" popover="manual"></p>
                sheets(providers: providers)
            </body>
        </html>
    })
}

/// The chat header's line before its state: a Workflow session's role and round, else who it runs as (`runs`:
/// provider · account), the model and the folder.
fn byline(thread: &Thread, runs: &str, folder: &str) -> String {
    if let Some(w) = &thread.workflow {
        return format!("{} · round {} · ", w.role, w.round);
    }
    [runs, &thread.model, folder]
        .into_iter()
        .filter(|s| !s.is_empty())
        .flat_map(|s| [s, " · "])
        .collect()
}

/// `~/x` for a folder under `$HOME` (as spelled, or resolved: a thread keeps its folder resolved).
fn short(folder: &Path) -> String {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let real = home.as_deref().and_then(|h| h.canonicalize().ok());
    let under = |h: &Path| folder.strip_prefix(h).ok();
    match home
        .as_deref()
        .and_then(under)
        .or_else(|| real.as_deref().and_then(under))
    {
        Some(rest) if rest.as_os_str().is_empty() => "~".to_owned(),
        Some(rest) => format!("~/{}", rest.display()),
        None => folder.display().to_string(),
    }
}

/// A card on the thread list: a working folder (`~/x`) and its chats, or a Workflow source and its sessions.
#[derive(serde::Serialize)]
pub struct Card {
    /// The folder, `~/` for one under `$HOME`; or the source.
    pub folder: String,
    /// Its threads, in the list's order.
    pub rows: Vec<Summary>,
    /// A source's card: its finished sessions, folded under "done today".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done: Option<Vec<Summary>>,
}

/// The cookie that keeps the thread list's chip: `chats` or `workflow` (else All).
const LIST: &str = "cooee-list";

/// The device's chip: `all`, `chats` or `workflow`.
#[must_use]
pub fn chip(cx: &Cx) -> &'static str {
    let cookie = topcoat::router::request::headers(cx)
        .get("cookie")
        .and_then(|v| v.to_str().ok());
    let value = cookie
        .into_iter()
        .flat_map(|c| c.split(';'))
        .find_map(|c| c.trim().strip_prefix(LIST)?.strip_prefix('='));
    match value {
        Some("chats") => "chats",
        Some("workflow") => "workflow",
        _ => "all",
    }
}

/// `POST /list`: the chip tapped.
#[derive(serde::Deserialize)]
struct Chip {
    list: String,
}

/// Keeps the chip tapped for this device and goes back to the list.
#[route(POST "/list")]
async fn list_set(Form(c): Form<Chip>) -> Result<Response> {
    let value = if matches!(c.list.as_str(), "chats" | "workflow") {
        c.list.as_str()
    } else {
        "all"
    };
    Ok(Response::builder()
        .status(303)
        .header("Location", "/")
        .header(
            "Set-Cookie",
            format!("{LIST}={value}; Path=/; Max-Age=315360000; SameSite=Lax"),
        )
        .body(Body::from(()))?)
}

/// The thread list's cards under chip `list` (`GET /threads` and the page draw the same): Chats, one per working
/// folder, by folder; then Workflow, one per source, needs-you first, then working, then the newest, the finished
/// ones folded. Each row's `agent` is its provider's name (`providers`), never the agent's id.
#[must_use]
pub fn cards(rows: Vec<Summary>, providers: &[Sheet], list: &str) -> Vec<Card> {
    let mut by: BTreeMap<PathBuf, Vec<Summary>> = BTreeMap::new();
    let mut sources: BTreeMap<String, Vec<Summary>> = BTreeMap::new();
    for mut r in rows {
        r.agent = named(&r.agent, providers);
        match &r.workflow {
            Some(w) => sources.entry(w.source.clone()).or_default().push(r),
            None => by.entry(r.folder.clone()).or_default().push(r),
        }
    }
    let chats = by.into_iter().map(|(f, rows)| Card {
        folder: short(&f),
        rows,
        done: None,
    });
    let workflow = sources.into_iter().map(|(source, mut rows)| {
        rows.sort_by_key(|r| match r.state.as_str() {
            "blocked" => 0,
            "working" => 1,
            _ => 2,
        });
        let (done, rows) = rows
            .into_iter()
            .partition(|r| matches!(r.state.as_str(), "done" | "ended"));
        Card {
            folder: source,
            rows,
            done: Some(done),
        }
    });
    match list {
        "chats" => chats.collect(),
        "workflow" => workflow.collect(),
        _ => chats.chain(workflow).collect(),
    }
}

/// Now, in Unix ms.
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// `now`, `5m`, `3h`, `2d` since `at` (Unix ms).
fn age(at: u64) -> String {
    match (now() / 1000).saturating_sub(at / 1000) {
        s if s < 60 => "now".to_owned(),
        s if s < 3600 => format!("{}m", s / 60),
        s if s < 86_400 => format!("{}h", s / 3600),
        s => format!("{}d", s / 86_400),
    }
}

/// A turn state's key in the theme (`shown.rs` and `look.ts` draw its sprite and words).
fn state(state: &str) -> &'static str {
    match state {
        "blocked" => "needs",
        "working" => "working",
        "error" | "unconnected" => "fail",
        "done" => "done",
        "ended" => "ended",
        "connecting" => "loading",
        _ => "",
    }
}

/// The header: the app's name (the heading on the thread list, with the 🔔 that turns alerts on; else a link to
/// it), the plugin tabs (`current` open), `+` (pressed on the composer's page, `new`), the look sheet and Settings.
#[component]
async fn header(
    home: bool,
    tabs: Vec<(String, String, bool)>,
    current: String,
    #[default] new: bool,
) -> Result<impl View> {
    Ok(view! {
        <header class="bar">
            if home {
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
            } else {
                <a class="brand" href="/">"cooee"</a>
            }
            nav(tabs: tabs, current: current)
            <button type="button" class="gl" id="rf" aria-label="refresh">"↻"</button>
            <a
                class="gl np"
                href="/new"
                aria-label="new thread"
                aria-current=(if new { Some("page") } else { None })
            >
                "+"
            </a>
            <button
                type="button"
                class="gl"
                data-open="look"
                popovertarget="sh-look"
                aria-label="look: theme and colours"
                aria-pressed="false"
            >
                "◐"
            </button>
            <button
                type="button"
                class="gl"
                data-open="settings"
                popovertarget="sh-settings"
                aria-label="settings"
                aria-pressed="false"
            >
                "⚙"
            </button>
        </header>
    })
}

#[component]
async fn row(cx: &Cx, r: Summary) -> Result<impl View> {
    let st = state(&r.state);
    let mark = Unescaped::new_unchecked(shown(cx).st(st, false));
    let href = format!("/t/{}", r.id);
    let meta = match &r.workflow {
        Some(w) => format!("{} · round {} · {}", w.role, w.round, age(r.at)),
        None => format!("{} · {}", r.agent, age(r.at)),
    };
    Ok(view! {
        <a class=(if r.unread { "tr" } else { "tr rd" }) href=(href)>
            <i class="ud">(if r.unread { "●" } else { "" })</i>
            <span class="tt">(r.title)</span>
            <span class="ta">(meta)</span>
            <span class="tp">
                <b data-st=(st)>(mark)</b>
                (r.last)
            </span>
        </a>
    })
}

/// Chat and the plugin tabs (`(name, label, dot)`), when there are any; `current` is the open one (`""`: Chat).
#[component]
async fn nav(tabs: Vec<(String, String, bool)>, current: String) -> Result<impl View> {
    Ok(view! {
        if !tabs.is_empty() {
            <nav class="tabs" aria-label="tabs">
                <a href="/" class=(if current.is_empty() { "on" } else { "" })>
                    "Chat"
                </a>
                for (name, label, dot) in tabs {
                    <a
                        href=(format!("/p/{name}"))
                        class=(if name == current { "on" } else { "" })
                    >
                        (label)
                        if dot {
                            <i class="dot" aria-hidden="true">"●"</i>
                            <span class="vh">" (needs you)"</span>
                        }
                    </a>
                }
            </nav>
        }
    })
}

/// The thread list (`/`): one card per working folder, its 3 newest threads shown, the rest folded; from 1024 px
/// the sidebar is the list and the composer is beside it. `tabs`: the plugins' names and labels.
#[must_use]
pub fn threads_page(
    cx: &Cx,
    rows: Vec<Summary>,
    new: New,
    providers: Vec<Sheet>,
    tabs: Vec<(String, String, bool)>,
) -> impl View {
    view! { cx => threads(rows: rows, new: new, providers: providers, tabs: tabs) }
}

/// The chips over the one list (#33): All · Chats · Workflow, `list` on; a tap posts `POST /list`.
#[component]
async fn chips(list: &'static str) -> Result<impl View> {
    Ok(view! {
        <form class="seg" id="chips" method="post" action="/list" aria-label="show">
            for (v, label) in [
                ("all", "All"),
                ("chats", "Chats"),
                ("workflow", "Workflow"),
            ] {
                <button
                    type="submit"
                    role="radio"
                    name="list"
                    value=(v)
                    aria-checked=(if v == list { "true" } else { "false" })
                >
                    (label)
                </button>
            }
        </form>
    })
}

/// One card: a folder's 3 newest chats, the rest folded; or a source's sessions, the finished ones folded.
#[component]
async fn card(c: Card) -> Result<impl View> {
    let Card { folder, rows, done } = c;
    let shown = if done.is_some() { rows.len() } else { 3 };
    Ok(view! {
        <section class="fc">
            <div class="fh">
                <b>(folder)</b>
                <span class="ds">(format!("{} threads", rows.len()))</span>
            </div>
            for r in rows.iter().take(shown).cloned() {
                row(r: r)
            }
            if let Some(done) = done {
                if !done.is_empty() {
                    <details>
                        <summary>(format!("{} done today", done.len()))</summary>
                        for r in done {
                            row(r: r)
                        }
                    </details>
                }
            } else if rows.len() > 3 {
                <details>
                    <summary>(format!("{} older threads", rows.len() - 3))</summary>
                    for r in rows.iter().skip(3).cloned() {
                        row(r: r)
                    }
                </details>
            }
        </section>
    })
}

#[component]
async fn threads(
    cx: &Cx,
    rows: Vec<Summary>,
    new: New,
    providers: Vec<Sheet>,
    tabs: Vec<(String, String, bool)>,
) -> Result<impl View> {
    let list = chip(cx);
    let cards = cards(rows.clone(), &providers, list);
    let empty = cards.is_empty();
    let look = shown(cx);
    let (figure, words) = (look.s("empty"), look.w("empty-threads", true));
    Ok(view! {
        document(
            title: "cooee",
            providers: providers.clone(),
            header(home: true, tabs: tabs, current: String::new())
            <div class="shell">
                side(rows: rows.clone(), cur: "new".to_owned())
                <main class="list">
                    if empty {
                        <div class="emp">
                            <span data-s="empty">
                                (Unescaped::new_unchecked(figure))
                            </span>
                            <p data-w="empty-threads" data-plain="">
                                (Unescaped::new_unchecked(words))
                            </p>
                            <a href="/new">"start a thread"</a>
                        </div>
                    }
                    chips(list: list)
                    for c in cards {
                        card(c: c)
                    }
                </main>
                <main class="nt home">
                    <div class="th"><div class="tb"><h2>"new thread"</h2></div></div>
                    <div class="sp"></div>
                    composer(rows: rows, new: new, providers: providers)
                    <div class="sp"></div>
                </main>
            </div>
        )
    })
}

/// `/new`: the composer alone, under the thread bar (on a phone at the bottom, where the chat's will be).
#[must_use]
pub fn new_page(
    cx: &Cx,
    rows: Vec<Summary>,
    new: New,
    providers: Vec<Sheet>,
    tabs: Vec<(String, String, bool)>,
) -> impl View {
    view! { cx => new_thread(rows: rows, new: new, providers: providers, tabs: tabs) }
}

#[component]
async fn new_thread(
    rows: Vec<Summary>,
    new: New,
    providers: Vec<Sheet>,
    tabs: Vec<(String, String, bool)>,
) -> Result<impl View> {
    Ok(view! {
        document(
            title: "new thread · cooee",
            providers: providers.clone(),
            header(home: false, tabs: tabs, current: String::new(), new: true)
            <div class="shell">
                side(rows: rows.clone(), cur: "new".to_owned())
                <main class="nt">
                    <div class="th">
                        <a class="gl bk" href="/" aria-label="back to threads">"‹"</a>
                        <div class="tb"><h1>"new thread"</h1></div>
                    </div>
                    <div class="sp"></div>
                    composer(rows: rows, new: new, providers: providers)
                    <div class="sp"></div>
                </main>
            </div>
        )
    })
}

/// Plugin `name`'s tab: its module runs in a sandboxed frame that `chat.ts` hosts.
#[must_use]
pub fn plugin_page(
    cx: &Cx,
    tabs: Vec<(String, String, bool)>,
    name: String,
    label: String,
) -> impl View {
    view! { cx => plugin(tabs: tabs, name: name, label: label) }
}

#[component]
async fn plugin(
    tabs: Vec<(String, String, bool)>,
    name: String,
    label: String,
) -> Result<impl View> {
    let title = format!("{label} · cooee");
    Ok(view! {
        document(
            title: title.as_str(),
            <main class="pt">
                header(home: false, tabs: tabs, current: name.clone())
                <h1 class="vh">(label.clone())</h1>
                <iframe
                    class="pf"
                    title=(label)
                    sandbox="allow-scripts"
                    data-plugin=(name.clone())
                    data-src=(format!("/p/{name}/frame"))
                ></iframe>
            </main>
        )
    })
}

/// Pairing, and the app's shell for an unpaired browser (#21): the look sheet, a pair hint and the composer off, no
/// thread data. The code `cooee pair` printed (`code` prefilled from its QR link; `bad` after a wrong one), or a code
/// asked for here and approved on the Mac.
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
                <button
                    type="button"
                    class="gl"
                    data-open="look"
                    popovertarget="sh-look"
                    aria-label="look: theme and colours"
                    aria-pressed="false"
                >
                    "◐"
                </button>
            </header>
            <main class="list">
                <p class="hn">
                    "This device isn't paired: threads and messages show once it is."
                </p>
                <form class="nf" method="post" action="/pair">
                    <div class="fh"><b>"pair this device"</b></div>
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
                <section class="nf" id="ask">
                    <div class="fh"><b>"or approve it on the Mac"</b></div>
                    <p class="ds">
                        "Ask for a code here, then approve it on the Mac with cooee pair and the code."
                    </p>
                    <button type="button" id="ask-go">"ask for a code"</button>
                    <div id="ask-wait" hidden=(true)>
                        <p class="code" id="ask-code"></p>
                        <p class="hn">
                            "on the Mac:"
                            <code id="ask-cmd"></code>
                            <button type="button" id="ask-copy">"copy"</button>
                        </p>
                        <p class="ds">"waiting for the Mac…"</p>
                        <button type="button" class="cn" id="ask-cancel">
                            "cancel"
                        </button>
                    </div>
                    <p class="err" id="ask-err" hidden=(true)></p>
                </section>
                <form class="tw" id="locked">
                    <textarea
                        rows="1"
                        aria-label="message"
                        placeholder="pair this device to send"
                        disabled=(true)
                    ></textarea>
                    <button type="submit" disabled=(true)>"send"</button>
                </form>
            </main>
        )
    })
}

/// One thread: its bar, the items (drawn by `chat.ts`), the working pill and the composer; from 1024 px the
/// sidebar beside it. The bar names the provider and the account (`providers`), never the agent's id.
#[must_use]
pub fn chat_page(
    cx: &Cx,
    (thread, turn): (Thread, Turn),
    rows: Vec<Summary>,
    providers: Vec<Sheet>,
    choices: (Vec<String>, Vec<String>),
) -> impl View {
    view! {
        cx =>
        chat(
            thread: thread,
            turn: turn,
            rows: rows,
            providers: providers,
            choices: choices
        )
    }
}

/// A model or effort select (#553 row 34): `name`'s choices, `default` first, `now` selected.
#[component]
async fn pick(id: String, name: &str, list: Vec<String>, now: String) -> Result<impl View> {
    Ok(view! {
        if !list.is_empty() {
            <select name=(id.clone()) id=(id) aria-label=(name)>
                <option value="">(format!("{name}: default"))</option>
                for m in list {
                    <option value=(m.clone()) selected=(m == now)>
                        (format!("{name}: {m}"))
                    </option>
                }
            </select>
        }
    })
}

/// The model and effort selects (`model<suffix>`, `effort<suffix>`), `now` selected.
#[component]
async fn picks(
    suffix: String,
    choices: (Vec<String>, Vec<String>),
    now: (String, String),
) -> Result<impl View> {
    Ok(view! {
        pick(id: format!("model{suffix}"), name: "model", list: choices.0, now: now.0)
        pick(id: format!("effort{suffix}"), name: "effort", list: choices.1, now: now.1)
    })
}

#[component]
async fn chat(
    cx: &Cx,
    thread: Thread,
    turn: Turn,
    rows: Vec<Summary>,
    providers: Vec<Sheet>,
    choices: (Vec<String>, Vec<String>),
) -> Result<impl View> {
    let look = shown(cx);
    let [st, wait, pill] = [
        (state(&turn.state), false),
        ("needs", true),
        ("working", true),
    ]
    .map(|(st, plain)| Unescaped::new_unchecked(look.st(st, plain)));
    let folder = thread.folder.as_deref().map(short).unwrap_or_default();
    let ended = thread.closed || turn.state == "ended";
    let name = named(&thread.agent, &providers);
    let runs = compose::said(
        &name,
        &compose::who(&thread.agent, &thread.account, &providers),
    );
    // A Workflow session (#33): its role and round, and a link to what it works on; its tool closes it.
    let workflow = thread.workflow.clone();
    let link = workflow
        .as_ref()
        .map(|w| w.url.clone())
        .filter(|u| u.starts_with("https://") || u.starts_with("http://"));
    Ok(view! {
        document(
            title: thread.title.as_str(),
            providers: providers,
            header(home: false, tabs: Vec::new(), current: String::new())
            <div class="shell">
                side(rows: rows, cur: thread.id.clone())
                <main
                    class="chat"
                    data-thread=(thread.id.clone())
                    data-agent=(name.clone())
                    data-who=(runs.clone())
                    data-read=(thread.read.unwrap_or(0).to_string())
                    data-state=(turn.state.clone())
                >
                    <div class="th">
                        <a class="gl bk" href="/" aria-label="back to threads">"‹"</a>
                        <div class="tb">
                            <h1>(thread.title.clone())</h1>
                            <span class="ds">
                                <i class="dt"></i>
                                (byline(&thread, &runs, &folder))
                                <span id="st" data-st=(state(&turn.state))>(st)</span>
                                if let Some(url) = link {
                                    " · "
                                    <a href=(url)>"issue & PR"</a>
                                }
                            </span>
                        </div>
                        <button
                            type="button"
                            id="close"
                            class="gl"
                            hidden=(ended || workflow.is_some())
                            aria-label="close the thread"
                        >
                            "×"
                        </button>
                    </div>
                    <button type="button" class="tlv cw" id="cw" hidden=(true)></button>
                    <ol
                        class="tc"
                        id="msgs"
                        tabindex="0"
                        aria-label="messages"
                        aria-live="polite"
                    ></ol>
                    <p class="tlv" id="wait" hidden=(true)>
                        <span data-st="needs" data-plain="">(wait)</span>
                    </p>
                    if !ended && (!choices.0.is_empty() || !choices.1.is_empty()) {
                        <div class="mc">
                            picks(
                                suffix: String::new(),
                                choices: choices,
                                now: (thread.model.clone(), thread.effort.clone())
                            )
                        </div>
                    }
                    <form class="tw" id="composer">
                        <div class="pill" id="pill" hidden=(true)>
                            <span id="pst" data-st="working" data-plain="">(pill)</span>
                            <span id="el">"0:00"</span>
                            <button type="button" class="stop">"stop"</button>
                        </div>
                        <textarea
                            id="box"
                            rows="1"
                            aria-label="message"
                            placeholder=(format!("message {name}"))
                            disabled=(ended)
                        ></textarea>
                        <button type="submit" disabled=(ended)>"send"</button>
                    </form>
                </main>
            </div>
        )
    })
}
