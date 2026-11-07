//! cooee's web UI, HTML-first (#49): Topcoat renders every page on the server (docs/decisions/0001-ui-framework.md),
//! in the device's look from its cookie (`shown.rs`), and the sheets, menus and forms are native HTML (`sheet.rs`)
//! that work with no script. Strict TypeScript in `web/ts` (stripped by `build.rs`) does only what needs script:
//! `chat.ts` (the live chat over SSE, the scroll window, drafts, the offline copy, report mode, clipboard, push),
//! `look.ts` (a pick applied in place, the marks of what a page adds), `frame.ts` (a plugin frame's bridge) and
//! `sw.ts` (the service worker). Also the style sheet, the PWA's manifest and icons, and the themes' files.

#![forbid(unsafe_code)]

mod look;
mod sheet;
mod shown;

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use cooee_core::{
    AGENTS_FIX, Account, Agent, Profile, ROOTS_FIX, Sheet, Status, Summary, Thread, Turn,
};
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
/// `POST /look` (a pick with no script) and each theme's data at `/look/<theme>/{theme.json,sprites.svg,scene.svg,icon.png}`;
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
    let theme = app_context::<Arc<LookAssets>>(cx)
        .looks
        .themes
        .get(path_param::<Look>(cx))
        .ok_or_else(not_found)?;
    let found = match path_param::<File>(cx) {
        "sprites.svg" => theme
            .sprite_sheet
            .as_deref()
            .map(|b| Owned("image/svg+xml", with_rule(b, FRAMES))),
        "scene.svg" => theme
            .scene
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

#[route(GET "/look.js")]
async fn look_js() -> Result<Static> {
    Ok(Static(
        "text/javascript; charset=utf-8",
        built!("look.js").as_bytes(),
    ))
}

/// A page in the device's look, drawn from the first byte: the scheme's tokens, `data-*` on `<html>` and the
/// theme's icon; the sheets after the content.
#[component]
async fn document(cx: &Cx, title: &str, #[default] child: Child<'_>) -> Result<impl View> {
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
            <body data-at=(now().to_string())>
                (child)
                <p id="ld" role="status" hidden=(true)>
                    <span class="lw" data-w="loading">
                        (Unescaped::new_unchecked(look.w("loading", false)))
                    </span>
                    <span class="ago"></span>
                </p>
                <p id="toast" role="status" popover="manual"></p>
                sheets()
            </body>
        </html>
    })
}

/// The chat header's line before its state: a Workflow session's role and round, else profile, agent, account, model and
/// folder.
fn byline(thread: &Thread, profile: &str, folder: &str) -> String {
    if let Some(w) = &thread.workflow {
        return format!("{} · round {} · ", w.role, w.round);
    }
    [
        profile,
        &thread.agent,
        &thread.account,
        &thread.model,
        folder,
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    .flat_map(|s| [s, " · "])
    .collect()
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
/// ones folded.
#[must_use]
pub fn cards(rows: Vec<Summary>, list: &str) -> Vec<Card> {
    let mut by: BTreeMap<PathBuf, Vec<Summary>> = BTreeMap::new();
    let mut sources: BTreeMap<String, Vec<Summary>> = BTreeMap::new();
    for r in rows {
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
        "error" => "fail",
        "done" => "done",
        "ended" => "ended",
        _ => "",
    }
}

/// The header: the app's name (the heading on the thread list, with the 🔔 that turns alerts on; else a link to
/// it), the plugin tabs (`current` open), `+`, the look sheet and Settings.
#[component]
async fn header(
    home: bool,
    tabs: Vec<(String, String, bool)>,
    current: String,
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

/// The thread list: one card per working folder, its 3 newest threads shown, the rest folded; the new-thread
/// form (open when the owner has no chats, else behind `+`), each agent with its provider's state (no agents or no
/// folders: the fix line instead); the
/// sign-ins card (`#signins`, from Settings): each provider's accounts and their state, and its login's terminal (drawn by `chat.ts`); `tabs`: the
/// plugins' names and labels. With agent `profiles`, the form's profile picker.
#[must_use]
pub fn threads_page(
    cx: &Cx,
    rows: Vec<Summary>,
    (agents, profiles): (Result<Vec<Agent>, String>, Vec<Profile>),
    folders: Vec<(PathBuf, u8)>,
    providers: Vec<Sheet>,
    tabs: Vec<(String, String, bool)>,
) -> impl View {
    view! {
        cx =>
        threads(
            rows: rows,
            agents: agents,
            profiles: profiles,
            folders: folders,
            providers: providers,
            tabs: tabs
        )
    }
}

/// One account's row: its state, who it is, its last sign-in and usage, and [sign in], [sign out], [default].
#[component]
async fn account(a: Account, s: Status) -> Result<impl View> {
    let (class, label) = match (s.signed_in, a.signed_in_at) {
        (true, _) => ("g", "✓ signed in"),
        (false, Some(_)) => ("m", "needs sign-in again"),
        (false, None) => ("m", "signed out"),
    };
    let seen = a.signed_in_at.map(|at| match age(at).as_str() {
        "now" => "last sign-in just now".to_owned(),
        ago => format!("last sign-in {ago} ago"),
    });
    let line = [s.who.or(Some(s.line)), seen, s.usage];
    let line: Vec<String> = line
        .into_iter()
        .flatten()
        .filter(|l| !l.is_empty())
        .collect();
    Ok(view! {
        <div class="ac" data-account=(a.name.clone())>
            <div class="fh">
                <b>(a.label())</b>
                <span class=(class)>(label)</span>
            </div>
            <p class="ds">(line.join(" · "))</p>
            <div class="ab">
                <button type="button" class="si" hidden=(s.signed_in)>"sign in"</button>
                if s.signed_in || a.dir.is_some() {
                    <button type="button" class="sg">"sign out"</button>
                }
                if a.default {
                    <span class="ds">"default for new threads"</span>
                } else {
                    <button type="button" class="df">"make default"</button>
                }
            </div>
        </div>
    })
}

/// One provider on the sign-ins card: its accounts (the system default first) and [+ account], or the install line
/// when its CLI is missing; the login terminal (drawn by `chat.ts`).
#[component]
async fn provider(sheet: Sheet) -> Result<impl View> {
    let p = sheet.provider;
    let install = sheet.accounts.first().and_then(|(_, s)| s.install.clone());
    let add = install.is_none() && !p.home.is_empty();
    Ok(view! {
        <div class="pv" data-provider=(p.name.clone())>
            <div class="fh">
                <b>(p.name.clone())</b>
                if install.is_some() {
                    <span class="m">"not installed"</span>
                }
                if add {
                    <button type="button" class="ad">"+ account"</button>
                }
            </div>
            if let Some(why) = sheet.why {
                <p class="ds wy">(why)</p>
            }
            match install {
                Some(fix) => {
                    <p class="ds">"not installed on this Mac"</p>
                    if !fix.is_empty() {
                        hint(why: "on the Mac:", fix: fix.as_str(), then: "")
                    }
                }
                None => {
                    for (a, s) in sheet.accounts {
                        account(a: a, s: s)
                    }
                }
            }
            <div class="lg" hidden=(true)>
                <p class="ds">(format!("{} · shed terminal", p.login.join(" ")))</p>
                <pre class="out"></pre>
                <form class="tw">
                    <input
                        aria-label=(format!("the code from the {} sign-in page", p.name))
                        placeholder="paste the code"
                        autocomplete="off"
                    >
                    <button type="submit">"send"</button>
                    <button type="button" class="cn">"cancel"</button>
                </form>
            </div>
        </div>
    })
}

#[component]
async fn signins(providers: Vec<Sheet>) -> Result<impl View> {
    Ok(view! {
        <section class="nf shut" id="signins">
            <div class="fh">
                <b>"sign-ins"</b>
                <a class="gl" href="#" aria-label="close sign-ins">"×"</a>
            </div>
            for sheet in providers {
                provider(sheet: sheet)
            }
        </section>
    })
}

/// One agent in the new-thread picker: its default account signed in `✓`, or signed out with a link to the
/// sign-ins card; and the account picker, the default preselected.
#[component]
async fn agent(
    id: String,
    first: bool,
    accounts: Option<Vec<(Account, Status)>>,
    choices: (Vec<String>, Vec<String>),
) -> Result<impl View> {
    let signed = accounts.as_ref().and_then(|list| {
        let default = list.iter().find(|(a, _)| a.default);
        default.map(|(_, s)| s.signed_in)
    });
    Ok(view! {
        <label>
            <input type="radio" name="agent" value=(id.clone()) checked=(first)>
            <span>(id.clone())</span>
            match signed {
                Some(true) => {
                    <span class="g">"✓"</span>
                }
                Some(false) => {
                    <span class="m">"signed out"</span>
                }
                None => {

                }
            }
        </label>
        if let Some(list) = accounts {
            <select name=(format!("account:{id}")) aria-label=(format!("{id} account"))>
                for (a, s) in list {
                    <option value=(a.name.clone()) selected=(a.default)>
                        (if s.signed_in {
                            a.label().to_owned()
                        } else {
                            format!("{} · signed out", a.label())
                        })
                    </option>
                }
            </select>
        }
        pick(
            id: format!("model:{id}"),
            name: "model",
            list: choices.0,
            now: String::new()
        )
        pick(
            id: format!("effort:{id}"),
            name: "effort",
            list: choices.1,
            now: String::new()
        )
        if signed == Some(false) {
            <a class="so" href="#signins">(format!("sign in {id}"))</a>
        }
    })
}

/// An empty picker's row instead: why, and the line that fixes it with its copy button (`chat.ts`).
#[component]
async fn hint(why: &str, fix: &str, then: &str) -> Result<impl View> {
    Ok(view! {
        <p class="hn">
            (why)
            <code>(fix)</code>
            (then)
            <button type="button" data-copy=(fix) aria-label=(format!("copy {fix}"))>
                "copy"
            </button>
        </p>
    })
}

/// The new-thread form's folder: the roots and the folders 1 level inside them (and 2 levels, as `chat.ts`'s search
/// finds them), with [+ new folder]; or the fix line when there are none.
#[component]
async fn folder_picker(folders: Vec<(PathBuf, u8)>) -> Result<impl View> {
    Ok(view! {
        if folders.is_empty() {
            <p class="lb">"folder"</p>
            hint(why: "no folders set · add", fix: ROOTS_FIX, then: "to config.toml")
        } else {
            <div class="fh">
                <input
                    id="fq"
                    type="search"
                    aria-label="search folders"
                    placeholder="search folders"
                    autocomplete="off"
                >
                <button type="button" id="mkdir">"+ new folder"</button>
            </div>
            <fieldset class="fl">
                <legend class="lb">"folder"</legend>
                for (i, (f, depth)) in folders.into_iter().enumerate() {
                    <label data-d=(depth.to_string()) hidden=(depth > 1)>
                        <input
                            type="radio"
                            name="folder"
                            value=(f.display().to_string())
                            checked=(i == 0)
                        >
                        (short(&f))
                    </label>
                }
            </fieldset>
        }
    })
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

/// Each folder's last-used profile (its newest thread's, if it is still one to pick), by the folder as the form names
/// it: the first folder's, and all of them as JSON.
fn last_used(
    rows: &[Summary],
    profiles: &[Profile],
    folders: &[(PathBuf, u8)],
) -> (String, String) {
    // Rows are newest first, their folders resolved.
    let mut newest = BTreeMap::new();
    for r in rows {
        if r.profile.is_empty() || profiles.iter().any(|p| p.id() == r.profile) {
            newest.entry(&r.folder).or_insert(&r.profile);
        }
    }
    let last: BTreeMap<String, String> = folders
        .iter()
        .filter_map(|(f, _)| {
            let p = newest.get(&f.canonicalize().ok()?)?;
            Some((f.display().to_string(), (*p).clone()))
        })
        .collect();
    let first = folders.first().map(|(f, _)| f.display().to_string());
    let chosen = first
        .and_then(|f| last.get(&f).cloned())
        .unwrap_or_default();
    (chosen, serde_json::to_string(&last).unwrap_or_default())
}

/// The new-thread form's agent profile picker: Plain, then each profile (a plugin's with its name), `chosen`
/// selected; `last` is each folder's last-used profile, as JSON, for `chat.ts` to pick as a folder is picked. A
/// profile picked hides the agent picker, and one with its own folder the folder picker (CSS).
#[component]
async fn profile_picker(profiles: Vec<Profile>, chosen: String, last: String) -> Result<impl View> {
    Ok(view! {
        <label class="pk">
            <span class="lb">"profile"</span>
            <select name="profile" data-last=(last)>
                <option value="" selected=(chosen.is_empty())>"Plain"</option>
                for p in profiles {
                    <option
                        value=(p.id())
                        selected=(p.id() == chosen)
                        data-folder=(p.folder.is_some())
                    >
                        (match &p.plugin {
                            Some(by) => format!("{} · {by}", p.label),
                            None => p.label.clone(),
                        })
                    </option>
                }
            </select>
        </label>
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
    agents: Result<Vec<Agent>, String>,
    profiles: Vec<Profile>,
    folders: Vec<(PathBuf, u8)>,
    providers: Vec<Sheet>,
    tabs: Vec<(String, String, bool)>,
) -> Result<impl View> {
    // Open until the owner has a chat of their own (Workflow sessions aren't theirs).
    let open = rows.iter().all(|r| r.workflow.is_some());
    let list = chip(cx);
    let (chosen, by_folder) = last_used(&rows, &profiles, &folders);
    let cards = cards(rows, list);
    let empty = cards.is_empty();
    let look = shown(cx);
    let (figure, words) = (look.s("empty"), look.w("empty-threads", true));
    // Start waits for an agent and a folder to pick.
    let can_start = !folders.is_empty() && agents.as_ref().is_ok_and(|a| !a.is_empty());
    let accounts: BTreeMap<String, Vec<(Account, Status)>> = providers
        .iter()
        .rev()
        .map(|p| (p.provider.agent.clone(), p.accounts.clone()))
        .collect();
    let picks: BTreeMap<String, (Vec<String>, Vec<String>)> = providers
        .iter()
        .rev()
        .map(|p| {
            let p = &p.provider;
            (p.agent.clone(), (p.models.clone(), p.efforts.clone()))
        })
        .collect();
    Ok(view! {
        document(
            title: "cooee",
            header(home: true, tabs: tabs, current: String::new())
            <main class="list">
                signins(providers: providers)
                if empty {
                    <div class="emp">
                        <span data-s="empty">(Unescaped::new_unchecked(figure))</span>
                        <p data-w="empty-threads" data-plain="">
                            (Unescaped::new_unchecked(words))
                        </p>
                    </div>
                }
                <form class=(if open { "nf" } else { "nf shut" }) id="new">
                    <div class="fh">
                        <b>"new thread"</b>
                        <a class="gl" href="#" aria-label="close">"×"</a>
                    </div>
                    if !profiles.is_empty() {
                        profile_picker(
                            profiles: profiles,
                            chosen: chosen,
                            last: by_folder
                        )
                    }
                    <fieldset class="ag">
                        <legend class="lb">"agent"</legend>
                        match agents {
                            Ok(list) => {
                                if list.is_empty() {
                                    hint(
                                        why: "no agents installed · on the Mac:",
                                        fix: AGENTS_FIX,
                                        then: ""
                                    )
                                }
                                for (i, a) in list.into_iter().enumerate() {
                                    agent(
                                        id: a.id.clone(),
                                        first: i == 0,
                                        accounts: accounts.get(&a.id).cloned(),
                                        choices: picks.get(&a.id).cloned().unwrap_or_default()
                                    )
                                }
                            }
                            Err(e) => {
                                <p class="err">(e)</p>
                            }
                        }
                    </fieldset>
                    folder_picker(folders: folders)
                    <div class="tw">
                        <textarea
                            name="text"
                            rows="2"
                            aria-label="first message"
                            placeholder="message"
                        ></textarea>
                        <button type="submit" disabled=(!can_start)>"start"</button>
                    </div>
                    <p class="err" id="new-err" hidden=(true)></p>
                </form>
                chips(list: list)
                if look.scene() {
                    scene(cards: cards)
                } else {
                    for c in cards {
                        card(c: c)
                    }
                }
            </main>
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

/// One thread: its bar, the items (drawn by `chat.ts`), the working pill and the composer.
#[must_use]
pub fn chat_page(
    cx: &Cx,
    thread: Thread,
    turn: Turn,
    profile: String,
    choices: (Vec<String>, Vec<String>),
) -> impl View {
    view! { cx => chat(thread: thread, turn: turn, profile: profile, choices: choices) }
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

#[component]
async fn chat(
    cx: &Cx,
    thread: Thread,
    turn: Turn,
    profile: String,
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
    // A Workflow session (#33): its role and round, and a link to what it works on; its tool closes it.
    let workflow = thread.workflow.clone();
    let link = workflow
        .as_ref()
        .map(|w| w.url.clone())
        .filter(|u| u.starts_with("https://") || u.starts_with("http://"));
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
                header(home: false, tabs: Vec::new(), current: String::new())
                <div class="th">
                    <a class="gl bk" href="/" aria-label="back to threads">"‹"</a>
                    <div class="tb">
                        <h1>(thread.title.clone())</h1>
                        <span class="ds">
                            <i class="dt"></i>
                            (byline(&thread, &profile, &folder))
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
                        pick(
                            id: "model".to_owned(),
                            name: "model",
                            list: choices.0,
                            now: thread.model.clone()
                        )
                        pick(
                            id: "effort".to_owned(),
                            name: "effort",
                            list: choices.1,
                            now: thread.effort.clone()
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
                        placeholder=(format!("message {}", thread.agent))
                        disabled=(ended)
                    ></textarea>
                    <button type="submit" disabled=(ended)>"send"</button>
                </form>
            </main>
        )
    })
}
