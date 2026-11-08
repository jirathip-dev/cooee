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
    sync::{Arc, PoisonError, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use compose::composer;
pub use compose::{New, named, who};
use cooee_core::{Sheet, Summary, Thread, Turn};
pub use look::{Looks, Pair, SPRITES, Scheme, TOKENS, Theme, WORDS, check, gate, plain, read_pack};
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
/// `POST /look` (a pick with no script) and each theme's data at
/// `/look/<theme>/{theme.json,sprites.svg,scene.svg,icon.png}`; and `POST /list`, the thread list's chip.
#[must_use]
pub fn assets(builder: RouterBuilder, looks: Arc<LookAssets>) -> RouterBuilder {
    builder
        .route(manifest)
        .route(icon)
        .route(icon_192)
        .route(icon_512)
        .route(style)
        .route(script)
        .route(md_js)
        .route(worker)
        .route(look_js)
        .route(sheet::look_set)
        .route(list_set)
        .route(look_file)
        .app_context(looks)
}

/// The looks cooee serves, and their JSON for the pages' `<script id="looks">`: loaded at start and again when a
/// theme is added, removed or kept, or one in `themes-dev/` is saved.
pub struct LookAssets {
    dir: Option<PathBuf>,
    /// The index's theme packs, for Settings › look › + theme.
    index: Vec<cooee_core::plugins::Listed>,
    default: Option<String>,
    now: RwLock<(Arc<Looks>, Arc<str>)>,
}

impl LookAssets {
    /// The looks of config folder `dir` with the server's `default` theme ([`Looks::load`]) and the `index`'s theme
    /// packs, and why own files were refused.
    #[must_use]
    pub fn load(
        dir: Option<&Path>,
        default: Option<&str>,
        index: Vec<cooee_core::plugins::Listed>,
    ) -> (Arc<Self>, Vec<String>) {
        let assets = Self {
            dir: dir.map(Path::to_owned),
            index,
            default: default.map(str::to_owned),
            now: RwLock::new((Arc::new(Looks::load(None, None).0), "{}".into())),
        };
        let refused = assets.reload();
        (Arc::new(assets), refused)
    }

    /// Loads the looks again; returns why own files were refused.
    ///
    /// # Panics
    ///
    /// Never: [`Looks`] always serialises.
    pub fn reload(&self) -> Vec<String> {
        let (looks, refused) = Looks::load(self.dir.as_deref(), self.default.as_deref());
        // In a <script> element: no `</` may end it.
        let json = serde_json::to_string(&looks)
            .expect("looks serialise")
            .replace("</", "<\\/");
        *self.now.write().unwrap_or_else(PoisonError::into_inner) = (Arc::new(looks), json.into());
        refused
    }

    /// The looks now.
    #[must_use]
    pub fn looks(&self) -> Arc<Looks> {
        Arc::clone(&self.now.read().unwrap_or_else(PoisonError::into_inner).0)
    }

    fn json(&self) -> Arc<str> {
        Arc::clone(&self.now.read().unwrap_or_else(PoisonError::into_inner).1)
    }

    /// The config folder, where `themes/` and `themes-dev/` are.
    #[must_use]
    pub fn dir(&self) -> Option<&Path> {
        self.dir.as_deref()
    }
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

/// `svg` with a `<style>` rule first, since a `<use>` or `<img>` of another file takes that file's styles only.
fn with_rule(svg: &[u8], rule: &[u8]) -> Vec<u8> {
    let at = svg
        .windows(4)
        .position(|w| w == b"<svg")
        .and_then(|i| svg[i..].iter().position(|&b| b == b'>').map(|j| i + j + 1))
        .unwrap_or(0);
    [&svg[..at], rule, &svg[at..]].concat()
}

/// A sprite sheet's two-frame loop: frame 1 (`.kf1`, `.f1`) shows by default, and `--o1`/`--o2`, which the page
/// animates, inherit into each sprite.
const FRAMES: &[u8] = b"<style>.kf1,.f1{opacity:var(--o1,1)}.kf2,.f2{opacity:var(--o2,0)}</style>";

/// A scene's calm layers only (#34): herdsman's scenes draw each host-load level as a layer, and cooee has no load.
const CALM: &[u8] = b"<style>.h-busy,.h-held,.l-overcast,.l-rain{display:none}</style>";

#[route(GET "/look/{look}/{file}")]
async fn look_file(cx: &Cx) -> Result<Owned> {
    let looks = app_context::<Arc<LookAssets>>(cx).looks();
    let theme = looks
        .themes
        .get(path_param::<Look>(cx))
        .ok_or_else(not_found)?;
    let found = match path_param::<File>(cx) {
        "sprites.svg" => theme
            .sprite_sheet
            .as_deref()
            .map(|b| Owned("image/svg+xml", with_rule(b, FRAMES))),
        "scene.svg" => theme
            .scene_art
            .as_deref()
            .map(|b| Owned("image/svg+xml", with_rule(b, CALM))),
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

#[route(GET "/md.js")]
async fn md_js() -> Result<Static> {
    Ok(Static(
        "text/javascript; charset=utf-8",
        built!("md.js").as_bytes(),
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
    let json = app_context::<Arc<LookAssets>>(cx).json().to_string();
    Ok(view! {
        <!DOCTYPE html>
        <html
            lang="en"
            data-theme=(name)
            data-mode=(look.mode().map(str::to_owned))
            data-style=(look.shape().to_owned())
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
/// provider · account, or its agent `profile`'s label), the model and the folder.
fn byline(thread: &Thread, (runs, profile): (&str, &str), folder: &str) -> String {
    if let Some(w) = &thread.workflow {
        return format!("{} · round {} · ", w.role, w.round);
    }
    let runs = if profile.is_empty() { runs } else { profile };
    [runs, &thread.model, folder]
        .into_iter()
        .filter(|s| !s.is_empty())
        .flat_map(|s| [s, " · "])
        .collect()
}

/// `~/x` for a folder under `$HOME` (as spelled, or resolved: a thread keeps its folder resolved); none (empty):
/// `scratch`, a thread's own folder (#48).
fn short(folder: &Path) -> String {
    if folder.as_os_str().is_empty() {
        return "scratch".to_owned();
    }
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

/// The thread list as `GET /threads` and the page draw it: the cards, and the archived threads apart.
#[derive(serde::Serialize)]
pub struct Listed {
    /// The cards under the chip (`cards`).
    pub cards: Vec<Card>,
    /// The archived threads, newest first.
    pub archived: Vec<Summary>,
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

/// The thread list under chip `list`: its cards, and the archived threads apart.
#[must_use]
pub fn listed(rows: Vec<Summary>, providers: &[Sheet], list: &str) -> Listed {
    let (archived, rows): (Vec<_>, Vec<_>) = rows.into_iter().partition(|r| r.archived);
    Listed {
        cards: cards(rows, providers, list),
        archived,
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

/// A row's line under the title: agent · age, or a Workflow session's role · round · age.
fn meta(r: &Summary) -> String {
    match &r.workflow {
        Some(w) => format!("{} · round {} · {}", w.role, w.round, age(r.at)),
        None => format!("{} · {}", r.agent, age(r.at)),
    }
}

#[component]
async fn row(cx: &Cx, r: Summary) -> Result<impl View> {
    let st = state(&r.state);
    let mark = Unescaped::new_unchecked(shown(cx).st(st, false));
    let href = format!("/t/{}", r.id);
    let meta = meta(&r);
    Ok(view! {
        <a
            class=(if r.unread { "tr" } else { "tr rd" })
            href=(href)
            data-id=(r.id.clone())
        >
            <i class="ud">(if r.unread { "●" } else { "" })</i>
            <span class="tt">(r.title)</span>
            <span class="ta">
                (meta)
                if r.unseen > 0 {
                    <b class="bd" aria-label=(format!("{} unread", r.unseen))>
                        (r.unseen.to_string())
                    </b>
                }
            </span>
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

/// The thread list (`/`): one card per working folder, its 3 newest threads shown, the rest folded. `tabs`: the
/// plugins' names and labels.
#[must_use]
pub fn threads_page(
    cx: &Cx,
    rows: Vec<Summary>,
    providers: Vec<Sheet>,
    tabs: Vec<(String, String, bool)>,
) -> impl View {
    view! { cx => threads(rows: rows, providers: providers, tabs: tabs) }
}

/// A state's pose in Scene (#34): its 132 × 100 sprite.
fn pose(st: &str) -> &'static str {
    match st {
        "working" => "edit",
        "needs" => "needs",
        "done" => "done",
        "fail" => "fail",
        "ended" => "merged",
        _ => "idle",
    }
}

/// One session in Scene: its sprite acting out its state, and its name card (title, [`meta`], the state's word);
/// a tap opens it.
#[component]
async fn sprite(cx: &Cx, r: Summary) -> Result<impl View> {
    let look = shown(cx);
    let st = state(&r.state);
    let key = if st.is_empty() { "idle" } else { st };
    let (fig, word, meta) = (look.s(pose(st)), look.w(key, false), meta(&r));
    Ok(view! {
        <a
            class=(if r.unread { "sb" } else { "sb rd" })
            href=(format!("/t/{}", r.id))
            data-id=(r.id.clone())
        >
            <span class="sf" data-s=(pose(st))>(Unescaped::new_unchecked(fig))</span>
            <span class="sn">
                <b>(r.title)</b>
                <span>(meta)</span>
                <span class=(format!("sw {st}")) data-w=(key)>
                    (Unescaped::new_unchecked(word))
                </span>
            </span>
        </a>
    })
}

/// Scene (#34): the thread list as the theme's scene, a page per working folder swiped sideways with a dot per page
/// (a link with no script), each page the scene's strip over the sessions' sprites. `chat.ts` redraws it on refresh,
/// marks the dot in view and switches the strip to night by the device's clock.
#[component]
async fn scene(cx: &Cx, cards: Vec<Card>) -> Result<impl View> {
    let art = format!("/look/{}/scene.svg", shown(cx).name());
    let dots: Vec<(usize, String)> = cards.iter().map(|c| c.folder.clone()).enumerate().collect();
    Ok(view! {
        <div class="scn" id="scene" data-art=(art.clone())>
            <nav class="sdt" aria-label="pages" hidden=(dots.len() < 2)>
                for (i, f) in dots {
                    <a
                        href=(format!("#sp-{i}"))
                        aria-label=(f)
                        aria-current=(if i == 0 { Some("page") } else { None })
                    ></a>
                }
            </nav>
            <div class="spg">
                for (i, Card { folder, rows, done }) in cards.into_iter().enumerate() {
                    <section
                        class="sp"
                        id=(format!("sp-{i}"))
                        aria-label=(folder.clone())
                    >
                        <img class="sky" src=(art.clone()) alt="">
                        <h2 class="fh">
                            <b>(folder)</b>
                            <span class="ds">(format!("{} threads", rows.len()))</span>
                        </h2>
                        <div class="fld">
                            for r in rows.into_iter().chain(done.into_iter().flatten()) {
                                sprite(r: r)
                            }
                        </div>
                    </section>
                }
            </div>
        </div>
    })
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
    providers: Vec<Sheet>,
    tabs: Vec<(String, String, bool)>,
) -> Result<impl View> {
    let list = chip(cx);
    let Listed { cards, archived } = listed(rows, &providers, list);
    let empty = cards.is_empty();
    let look = shown(cx);
    let (figure, words) = (look.s("empty"), look.w("empty-threads", true));
    Ok(view! {
        document(
            title: "cooee",
            providers: providers.clone(),
            header(home: true, tabs: tabs, current: String::new())
            <div class="shell">
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
                    if look.scene() {
                        scene(cards: cards)
                    } else {
                        for c in cards {
                            card(c: c)
                        }
                    }
                    shelf(rows: archived)
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
                <main class="nt">
                    <div class="th">
                        <a class="gl bk" href="/" aria-label="back to threads">"‹"</a>
                        <div class="tb"><h1>"new thread"</h1></div>
                    </div>
                    <div class="ntsp"></div>
                    composer(rows: rows, new: new, providers: providers)
                    <div class="ntsp"></div>
                </main>
            </div>
        )
    })
}

/// The rename box the thread bar's title opens (`chat.ts`).
#[component]
async fn rename(title: String) -> Result<impl View> {
    Ok(view! {
        <form id="rt" class="rt" popover="auto" aria-label="rename the thread">
            <input
                name="title"
                aria-label="thread title"
                value=(title)
                maxlength="80"
                required=(true)
            >
            <button type="submit">"rename"</button>
        </form>
    })
}

/// The archived threads, folded, each with [restore] (`chat.ts`).
#[component]
async fn shelf(rows: Vec<Summary>) -> Result<impl View> {
    Ok(view! {
        if !rows.is_empty() {
            <details class="fc arc">
                <summary>(format!("{} archived", rows.len()))</summary>
                for r in rows {
                    <div class="arw">
                        <a href=(format!("/t/{}", r.id))>(r.title.clone())</a>
                        <button
                            type="button"
                            class="cn"
                            data-restore=(r.id.clone())
                            aria-label=(format!("restore {}", r.title))
                        >
                            "restore"
                        </button>
                    </div>
                }
            </details>
        }
    })
}

/// A thread's foot: the model and effort chips, the working pill, the queued messages, the picked files, the
/// composer; the long-press menu and the image viewer (`chat.ts`).
#[component]
async fn foot(
    who: String,
    ended: bool,
    pill: Unescaped<String>,
    choices: (Vec<String>, Vec<String>),
    now: (String, String),
) -> Result<impl View> {
    Ok(view! {
        if !ended && (!choices.0.is_empty() || !choices.1.is_empty()) {
            <div class="mc">
                picks(suffix: String::new(), choices: choices, now: now)
            </div>
        }
        <div class="cmp">
            <div class="pill" id="pill" hidden=(true)>
                <span id="pst" data-st="working" data-plain="">(pill)</span>
                <span id="el">"0:00"</span>
                <button type="button" class="stop">"stop"</button>
            </div>
            <ul class="qu" id="queue" aria-label="queued messages" hidden=(true)></ul>
            <div class="fis" id="picked" hidden=(true)></div>
            <form class="tw nc" id="composer">
                <label class="gl att" hidden=(ended)>
                    "📎"
                    <input
                        type="file"
                        id="files"
                        class="vh"
                        multiple=(true)
                        aria-label="attach files"
                    >
                </label>
                <textarea
                    id="box"
                    rows="1"
                    aria-label="message"
                    placeholder=(format!("message {who}"))
                    disabled=(ended)
                ></textarea>
                <button type="submit" disabled=(ended)>"send"</button>
            </form>
        </div>
        <div
            id="menu"
            class="mn"
            popover="manual"
            role="menu"
            aria-label="message actions"
        >
            <button type="button" role="menuitem" data-m="copy">"copy"</button>
            <button type="button" role="menuitem" data-m="md">
                "copy as markdown"
            </button>
            <button type="button" role="menuitem" data-m="quote">"quote"</button>
        </div>
        <div id="viewer" class="vw" popover="auto" aria-label="image"><img alt=""></div>
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

/// `[keep]`: theme `name` from `themes-dev/` into the installed ones, once it passes the theme gate (else the failures
/// go to `thread`, its chat).
#[component]
async fn keep(name: String, thread: String) -> Result<impl View> {
    Ok(view! {
        <form method="post" action="/themes/keep" class="kp">
            <input type="hidden" name="name" value=(name)>
            <input type="hidden" name="thread" value=(thread)>
            <button type="submit">"keep"</button>
        </form>
    })
}

/// The live preview of theme `name` in `themes-dev/`: its gate's `fails`, `[keep]`, and the thread list, its chat
/// (`thread`) and its scene as they look in it; `look.ts` reloads it on each save.
#[must_use]
pub fn preview_page(cx: &Cx, name: String, fails: Vec<String>, thread: String) -> impl View {
    view! { cx => preview(name: name, fails: fails, thread: thread) }
}

#[component]
async fn preview(cx: &Cx, name: String, fails: Vec<String>, thread: String) -> Result<impl View> {
    let looks = app_context::<Arc<LookAssets>>(cx).looks();
    let art = looks
        .themes
        .get(&format!("dev.{name}"))
        .is_some_and(|t| t.scene_art.is_some());
    let back = if thread.is_empty() {
        "/".to_owned()
    } else {
        format!("/t/{thread}")
    };
    let in_chat = format!("/t/{thread}?preview={name}");
    let title = format!("preview {name}");
    Ok(view! {
        document(
            title: title.as_str(),
            <main class="prv" data-preview=(name.clone()) aria-label="preview">
                <div class="th">
                    <a class="gl bk" href=(back) aria-label="back to the chat">"‹"</a>
                    <h1>(format!("preview · {name}"))</h1>
                    keep(name: name.clone(), thread: thread.clone())
                </div>
                if fails.is_empty() {
                    <p class="prg">"passes the theme gate · reloads on each save"</p>
                    <div class="prf">
                        <iframe title="thread list" src=(format!("/?preview={name}"))></iframe>
                        if !thread.is_empty() {
                            <iframe title="chat" src=(in_chat)></iframe>
                        }
                        if art {
                            <img
                                alt="scene"
                                src=(format!("/look/dev.{name}/scene.svg"))
                            >
                        }
                    </div>
                } else {
                    <p class="prg">"fails the theme gate · reloads on each save"</p>
                    <ul class="prx">
                        for f in fails {
                            <li>(f)</li>
                        }
                    </ul>
                }
            </main>
        )
    })
}

/// One thread: its bar, the items (drawn by `chat.ts`), the working pill and the composer. The bar names the provider
/// and the account (`providers`), or its agent `profile`'s label, never the agent's id; `ids` are the threads a ref in
/// it may open (#84).
#[must_use]
pub fn chat_page(
    cx: &Cx,
    (thread, turn): (Thread, Turn),
    providers: Vec<Sheet>,
    profile: String,
    choices: (Vec<String>, Vec<String>),
    ids: Vec<String>,
) -> impl View {
    view! {
        cx =>
        chat(
            thread: thread,
            turn: turn,
            providers: providers,
            profile: profile,
            choices: choices,
            ids: ids
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

/// The theme a thread in `folder` builds in `themes-dev/`, for its preview and [keep].
fn dev_theme(cx: &Cx, folder: Option<&Path>) -> Option<String> {
    app_context::<Arc<LookAssets>>(cx)
        .dir()
        .map(|d| d.join("themes-dev"))
        .zip(folder)
        .filter(|(dev, f)| f.parent() == Some(dev.as_path()))
        .and_then(|(_, f)| f.file_name()?.to_str().map(str::to_owned))
}

/// A theme thread's preview link and [keep], for a thread in `themes-dev/`.
#[component]
async fn dev_bar(cx: &Cx, thread: String, folder: Option<PathBuf>) -> Result<impl View> {
    Ok(view! {
        if let Some(name) = dev_theme(cx, folder.as_deref()) {
            <div class="tdv">
                <a href=(format!("/themes/dev/{name}?thread={thread}"))>
                    (format!("preview {name}"))
                </a>
                keep(name: name, thread: thread)
            </div>
        }
    })
}

/// `url`, when it is a web link.
fn web_url(url: &str) -> Option<String> {
    (url.starts_with("https://") || url.starts_with("http://")).then(|| url.to_owned())
}

#[component]
async fn chat(
    cx: &Cx,
    thread: Thread,
    turn: Turn,
    providers: Vec<Sheet>,
    profile: String,
    choices: (Vec<String>, Vec<String>),
    ids: Vec<String>,
) -> Result<impl View> {
    let look = shown(cx);
    let [st, wait, pill] = [
        (state(&turn.state), false),
        ("needs", true),
        ("working", true),
    ]
    .map(|(st, plain)| Unescaped::new_unchecked(look.st(st, plain)));
    let folder = short(thread.folder.as_deref().unwrap_or(Path::new("")));
    let ended = thread.closed || turn.state == "ended";
    let name = named(&thread.agent, &providers);
    let runs = compose::said(
        &name,
        &compose::who(&thread.agent, &thread.account, &providers),
    );
    // A Workflow session (#33): its role and round, and a link to what it works on; its tool closes it.
    let workflow = thread.workflow.clone();
    let link = workflow.as_ref().and_then(|w| web_url(&w.url));
    Ok(view! {
        document(
            title: thread.title.as_str(),
            providers: providers,
            header(home: false, tabs: Vec::new(), current: String::new())
            <div class="shell">
                <main
                    class="chat"
                    data-thread=(thread.id.clone())
                    data-agent=(name.clone())
                    data-who=(runs.clone())
                    data-read=(thread.read.unwrap_or(0).to_string())
                    data-state=(turn.state.clone())
                    data-ids=(ids.join(" "))
                >
                    <div class="th">
                        <a class="gl bk" href="/" aria-label="back">"‹"</a>
                        <div class="tb">
                            <div class="tn">
                                <h1>
                                    <button
                                        type="button"
                                        class="ttl"
                                        popovertarget="rt"
                                        aria-label=(format!("{} · rename", thread.title))
                                    >
                                        (thread.title.clone())
                                    </button>
                                </h1>
                                <small>(thread.id.clone())</small>
                            </div>
                            <span class="ds">
                                <i class="dt"></i>
                                <span id="net"></span>
                                (byline(&thread, (&runs, &profile), &folder))
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
                        rename(title: thread.title.clone())
                    </div>
                    dev_bar(thread: thread.id.clone(), folder: thread.folder.clone())
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
                    foot(
                        who: name,
                        ended: ended,
                        pill: pill,
                        choices: choices,
                        now: (thread.model.clone(), thread.effort.clone())
                    )
                </main>
            </div>
        )
    })
}
