//! cooee's web UI, HTML-first (#49): Topcoat renders every page on the server (docs/decisions/0001-ui-framework.md),
//! in the device's look from its cookie (`shown.rs`), and the sheets, menus and forms are native HTML (`sheet.rs`)
//! that work with no script. Strict TypeScript in `web/ts` (stripped by `build.rs`) does only what needs script:
//! `chat.ts` (the live chat over SSE, the scroll window, drafts, the offline copy, report mode, clipboard, push),
//! `look.ts` (a pick applied in place, the marks of what a page adds), `frame.ts` (a plugin frame's bridge) and
//! `sw.ts` (the service worker). Also the style sheet, the PWA's manifest and icons, and the themes' files.

#![forbid(unsafe_code)]

mod agents;
mod compose;
mod look;
mod sheet;
mod shown;

use std::{
    path::{Path, PathBuf},
    sync::{Arc, PoisonError, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

pub use agents::{Mate, Pane, Side, page};
use compose::hint;
pub use compose::{named, who};
use cooee_core::{Sheet, Thread, Turn};
pub use look::{Looks, Pair, SPRITES, Scheme, TOKENS, Theme, WORDS, check, gate, plain, read_pack};
use sheet::{sheets, shown};
pub use shown::Shown;
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
/// `/look/<theme>/{theme.json,sprites.svg,scene.svg,icon.png}`.
#[must_use]
pub fn assets(builder: RouterBuilder, looks: Arc<LookAssets>) -> RouterBuilder {
    builder
        .route(manifest)
        .route(icon)
        .route(icon_192)
        .route(pair_js)
        .route(icon_512)
        .route(style)
        .route(script)
        .route(md_js)
        .route(worker)
        .route(look_js)
        .route(sheet::look_set)
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

#[route(GET "/pair.js")]
async fn pair_js() -> Result<Static> {
    Ok(Static(
        "text/javascript; charset=utf-8",
        built!("pair.js").as_bytes(),
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
                <script type="module" src="/pair.js"></script>
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
                sheets(providers: providers)
            </body>
        </html>
    })
}

/// The chat header: the agent's name, the title (an agent's ongoing chat is titled by its name; a side thread by
/// its own, its agent first in the line), and the line before its state: a Workflow session's role and round, else
/// its agent profile's label, or Plain and who it runs as (`runs`: provider · account, never the agent's id), and the
/// model (the folder follows it, drawn apart so Use a folder… can change it). `provider`: the provider's name.
fn bar(
    thread: &Thread,
    mate: Option<&(usize, String, bool)>,
    (profile, provider, runs): (&str, &str, &str),
) -> (String, String, String) {
    let (name, title, by) = match mate {
        Some((_, n, true)) => (n.clone(), n.clone(), ""),
        Some((_, n, false)) => (n.clone(), thread.title.clone(), n.as_str()),
        None => (provider.to_owned(), thread.title.clone(), ""),
    };
    if let Some(w) = &thread.workflow {
        return (name, title, format!("{} · round {} · ", w.role, w.round));
    }
    let (profile, runs) = if profile.is_empty() {
        ("Plain", runs)
    } else {
        (profile, "")
    };
    let line = [by, profile, runs, &thread.model]
        .into_iter()
        .filter(|s| !s.is_empty())
        .flat_map(|s| [s, " · "])
        .collect();
    (name, title, line)
}

/// `~/x` for a folder under `$HOME` (as given, or resolved: a folder a chat works in is stored resolved); none
/// (empty): `scratch`, a chat's own folder.
#[must_use]
pub fn short(folder: &Path) -> String {
    if folder.as_os_str().is_empty() {
        return "scratch".to_owned();
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let homes = home
        .iter()
        .flat_map(|h| [h.clone(), h.canonicalize().unwrap_or_default()]);
    let homes = homes.filter(|h| h.has_root());
    let rest = homes
        .into_iter()
        .find_map(|h| folder.strip_prefix(h).ok().map(Path::to_path_buf));
    match rest {
        Some(rest) if rest.as_os_str().is_empty() => "~".to_owned(),
        Some(rest) => format!("~/{}", rest.display()),
        None => folder.display().to_string(),
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

/// The header: the app's name (the heading on the agents list, with the 🔔 that turns alerts on; else a link to
/// it), the plugin tabs (`current` open), `+` (`new`: a new chat with the agent on show), the look sheet and Settings.
#[component]
async fn header(
    home: bool,
    tabs: Vec<(String, String, bool)>,
    current: String,
    new: String,
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
            <a class="gl" href=(new) aria-label="new chat">"+"</a>
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
                header(
                    home: false,
                    tabs: tabs,
                    current: name.clone(),
                    new: "/new".to_owned()
                )
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

/// What the pairing page shows besides the welcome: the code (prefilled from a QR link), why the last one was
/// refused (`wrong` with the tries `left`, `expired`, or `limit` for `wait` seconds), whether a passkey can sign in,
/// and, in Safari on an iPhone or iPad, the Home Screen hint (`Some(true)`: iOS 26, Share is in the ⋯ menu).
#[derive(Debug, Clone, Default)]
pub struct PairState {
    /// Six digits from the QR link.
    pub code: Option<String>,
    /// Why the last code was refused.
    pub bad: Option<String>,
    /// Wrong codes this address may still type this minute.
    pub left: u32,
    /// Seconds until the boxes open again.
    pub wait: u64,
    /// Whether this cooee has a passkey.
    pub passkeys: bool,
    /// Safari on iOS (iOS 26 or later).
    pub ios: Option<bool>,
}

/// Pairing, and all an unpaired browser sees (q528: the shell, no thread data): the theme's figure and why, the iOS
/// Home Screen hint, three numbered steps, the six code boxes (`pair.ts` moves between them and sends the sixth), the
/// other ways (ask to pair, sign in with a passkey) and the states `pair.ts` shows (waiting, declined, offline).
#[must_use]
pub fn pair_page(cx: &Cx, state: PairState) -> impl View {
    view! { cx => pair(state: state) }
}

/// The share icon (iOS), drawn in the text colour.
const SHARE: &str = r#"<svg class="shi" viewBox="0 0 20 24" aria-hidden="true"><path d="M10 2v13M5.5 6.5 10 2l4.5 4.5M6.5 9.5H4a1 1 0 0 0-1 1V21a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V10.5a1 1 0 0 0-1-1h-2.5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>"#;

#[component]
async fn pair(cx: &Cx, state: PairState) -> Result<impl View> {
    let waiting = shown(cx).s("loading");
    Ok(view! {
        document(
            title: "pair · cooee",
            <header class="bar">
                <h1>"cooee"</h1>
                <i class="cdot" id="cdot" role="img" aria-label="connected"></i>
            </header>
            <main class="list pg">
                <section class="wel" aria-label="pair this device">
                    welcome(ios: state.ios)
                    code_card(state: state)
                </section>
                <section
                    class="pc"
                    data-k="a"
                    id="ask-wait"
                    hidden=(true)
                    aria-live="polite"
                >
                    if !waiting.is_empty() {
                        <span class="big" data-s="loading">
                            (Unescaped::new_unchecked(waiting))
                        </span>
                    }
                    <p class="et ctr">"waiting for approval"</p>
                    <p class="ckw" id="ask-word"></p>
                    <p class="ctr">
                        "Your paired devices got a request. Approve it where it shows "
                        <b id="ask-word2"></b>
                        "."
                    </p>
                    <div class="acts ctr">
                        <span class="m" id="ask-left"></span>
                        <button type="button" id="ask-cancel">"cancel"</button>
                    </div>
                </section>
                <section class="pc" data-k="r" id="ask-no" hidden=(true) role="alert">
                    <p class="et" id="ask-by"></p>
                    <p id="ask-why"></p>
                    <div class="acts">
                        <button type="button" id="ask-again">"ask again"</button>
                        <a class="bt q" href="/pair">"use a code"</a>
                    </div>
                </section>
                <section class="pc" data-k="r" id="offline" hidden=(true) role="alert">
                    <p class="et">"Can't reach your Mac."</p>
                    <p>
                        "Turn on Tailscale on this device, check that the Mac is awake, then try again."
                    </p>
                    <div class="acts">
                        <button type="button" id="retry">"try again"</button>
                        <span class="m" id="tried"></span>
                    </div>
                </section>
            </main>
        )
    })
}

/// The welcome: the theme's figure (its icon when there is none), why, the iOS hint and the three steps.
#[component]
async fn welcome(cx: &Cx, ios: Option<bool>) -> Result<impl View> {
    let look = shown(cx);
    let figure = look.s("empty");
    let ico = format!("/look/{}/icon.png", look.name());
    Ok(view! {
        <div class="hero">
            if figure.is_empty() {
                <img class="hic" alt="" src=(ico)>
            } else {
                <span data-s="empty">(Unescaped::new_unchecked(figure))</span>
            }
            <h2>"pair this device"</h2>
            <p>
                "cooee runs on your Mac. Pair this device once and your threads show up here."
            </p>
        </div>
        if let Some(newer) = ios {
            <div class="hint">
                (Unescaped::new_unchecked(SHARE))
                <div>
                    <p><b>"Add cooee to your Home Screen first."</b></p>
                    <p>
                        "Tap "
                        (Unescaped::new_unchecked(SHARE))
                        (if newer { " Share (in the ⋯ menu)" } else { " Share" })
                        ", then Add to Home Screen, and pair in the app it adds. Safari and the Home Screen app keep separate sign-ins."
                    </p>
                </div>
            </div>
        }
        <ol class="steps">
            <li>
                <b>"Open cooee on your Mac or a paired phone"</b>
                <span>
                    "On the Mac run "
                    <code>"cooee pair"</code>
                    ". On a phone: settings › devices › pair a device."
                </span>
            </li>
            <li>
                <b>"Scan its QR code with the Camera"</b>
                <span>
                    "cooee opens here with the code filled in and pairs at once."
                </span>
            </li>
            <li>
                <b>"No camera? Approve instead"</b>
                <span>
                    "Tap ask to pair below, then approve on your paired phone with Face ID."
                </span>
            </li>
        </ol>
    })
}

/// The code card: six boxes (filled from a QR link, or the used code muted), why the last code was refused, [pair] and the other ways.
#[component]
async fn code_card(state: PairState) -> Result<impl View> {
    let bad = state.bad.clone().unwrap_or_default();
    let digits: Vec<char> = state.code.clone().unwrap_or_default().chars().collect();
    let boxes = match bad.as_str() {
        "wrong" => "cbx r",
        "expired" | "limit" => "cbx dim",
        _ if digits.len() == 6 => "cbx ok",
        _ => "cbx",
    };
    let off = bad == "limit";
    Ok(view! {
        <form class="cdc" id="code" method="post" action="/pair">
            <p class="ch">"or type the code"</p>
            <div class=(boxes) role="group" aria-label="pairing code, 6 digits">
                for i in 0..6usize {
                    <input
                        name=(if i == 0 {
                            "code".to_owned()
                        } else {
                            format!("d{}", i + 1)
                        })
                        inputmode="numeric"
                        autocomplete=(if i == 0 { "one-time-code" } else { "off" })
                        maxlength=(if i == 0 { "6" } else { "1" })
                        aria-label=(format!("digit {}", i + 1))
                        disabled=(off)
                        autofocus=(i == 0 && bad == "wrong")
                        value=(if bad.is_empty() || bad == "expired" {
                            digits.get(i).map(char::to_string).unwrap_or_default()
                        } else {
                            String::new()
                        })
                    >
                }
            </div>
            if bad.is_empty() && digits.len() == 6 {
                <p class="cdn"><span class="et g">"pairing…"</span></p>
            }
            refusal(bad: bad, left: state.left, wait: state.wait)
            <p class="cdn">"6 digits · good for 10 minutes · pairs on the sixth"</p>
            <div class="go"><button type="submit" disabled=(off)>"pair"</button></div>
            <div class="alt">
                <div class="lrw">
                    <span class="sk">"no camera nearby?"</span>
                    <button type="button" id="ask-go">"ask to pair"</button>
                </div>
                if state.passkeys {
                    <div class="lrw">
                        <span class="sk">"paired here before?"</span>
                        <button type="button" class="q" id="pk-go">
                            "sign in with passkey"
                        </button>
                    </div>
                }
            </div>
            <div class="pc" data-k="r" role="alert" id="pk-err" hidden=(true)>
                <p class="et"></p>
            </div>
        </form>
    })
}

/// Why the last code was refused: `wrong` (with the tries `left`), `expired`, or `limit` (for `wait` seconds).
#[component]
async fn refusal(bad: String, left: u32, wait: u64) -> Result<impl View> {
    let tries = format!(
        "{left} {} left this minute",
        if left == 1 { "try" } else { "tries" }
    );
    Ok(view! {
        match bad.as_str() {
            "wrong" => {
                <div class="pc" data-k="r" role="alert">
                    <p>
                        <span class="et">"That code isn't right."</span>
                        " Check the six digits on your Mac or phone and type them again."
                    </p>
                    <p class="m">(tries)</p>
                </div>
            }
            "expired" => {
                <div class="pc" data-k="r" role="alert">
                    <p>
                        <span class="et">"That code has run out."</span>
                        " A code works once, for 10 minutes. Show a new one: "
                        <code>"cooee pair"</code>
                        ", or pair a device on your phone."
                    </p>
                    <div class="acts">
                        <a class="bt" href="/pair">"type a new code"</a>
                    </div>
                </div>
            }
            "limit" => {
                <div
                    class="pc"
                    data-k="r"
                    role="alert"
                    id="limit"
                    data-wait=(wait.to_string())
                >
                    <p>
                        <span class="et">"Too many tries."</span>
                        " Five wrong codes a minute is the limit. The boxes open again in "
                        <span class="tm">
                            (format!("{}:{:02}", wait / 60, wait % 60))
                        </span>
                        "."
                    </p>
                </div>
            }
            _ => {

            }
        }
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
pub(crate) async fn shelf(rows: Vec<cooee_core::Summary>) -> Result<impl View> {
    Ok(view! {
        if !rows.is_empty() {
            <details class="arc">
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

/// A thread's foot: the folder, model and effort chips (none once it ended), the working pill, the queued messages,
/// the picked files, the composer; the long-press menu and the image viewer (`chat.ts`).
#[component]
async fn foot(
    who: String,
    ended: bool,
    pill: Unescaped<String>,
    places: Option<agents::Places>,
    choices: (Vec<String>, Vec<String>),
    now: (String, String),
) -> Result<impl View> {
    Ok(view! {
        if !ended {
            agents::chips(places: places, choices: choices, now: now)
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

/// One thread (the pane of `/t/{id}`): its bar (its agent's avatar and name, the title of a side thread), the
/// items (drawn by `chat.ts`), the working pill, and the composer with the folder, model and effort chips.
#[component]
async fn chat(
    cx: &Cx,
    thread: Thread,
    turn: Turn,
    providers: &[Sheet],
    profile: String,
    mate: Option<(usize, String, bool)>,
    places: agents::Places,
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
    let folder = places.2.as_deref().map_or("scratch".to_owned(), short);
    let provider = named(&thread.agent, providers);
    let runs = compose::said(&provider, &who(&thread.agent, &thread.account, providers));
    let (name, title, said) = bar(&thread, mate.as_ref(), (&profile, &provider, &runs));
    let ended = thread.closed || turn.state == "ended";
    let renamable = !matches!(mate, Some((_, _, true)));
    // A Workflow session (#33): its role and round, and a link to what it works on; its tool closes it.
    let workflow = thread.workflow.clone();
    let link = workflow.as_ref().and_then(|w| web_url(&w.url));
    Ok(view! {
        <div
            class="chat"
            data-thread=(thread.id.clone())
            data-agent=(name.clone())
            data-who=(runs.clone())
            data-read=(thread.read.unwrap_or(0).to_string())
            data-state=(turn.state.clone())
            data-ids=(ids.join(" "))
        >
            <div class="th">
                agents::back(key: agents::key(&thread.agent, &thread.profile))
                if let Some((n, _, _)) = mate {
                    agents::avatar(name: name.clone(), n: n, big: false)
                }
                <div class="tb">
                    <div class="tn">
                        // A side thread's title renames it (#43); an agent's ongoing chat goes by its name.
                        if renamable {
                            <h1>
                                <button
                                    type="button"
                                    class="ttl"
                                    popovertarget="rt"
                                    aria-label=(format!("{title} · rename"))
                                >
                                    (title.clone())
                                </button>
                            </h1>
                        } else {
                            <h1>(title.clone())</h1>
                        }
                        <small>(thread.id.clone())</small>
                    </div>
                    <span class="ds">
                        <i class="dt"></i>
                        <span id="net"></span>
                        (said)
                        if thread.workflow.is_none() {
                            <span id="bf">(folder)</span>
                            " · "
                        }
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
                if renamable {
                    rename(title: title.clone())
                }
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
                who: name.clone(),
                ended: ended,
                pill: pill,
                places: thread.workflow.is_none().then_some(places),
                choices: choices,
                now: (thread.model.clone(), thread.effort.clone())
            )
        </div>
    })
}
