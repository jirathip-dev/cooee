//! cooee's web UI, HTML-first (#49): Topcoat renders every page on the server (docs/decisions/0001-ui-framework.md),
//! in the device's look from its cookie (`shown.rs`), and the sheets, menus and forms are native HTML (`sheet.rs`)
//! that work with no script. Strict TypeScript in `web/ts` (stripped by `build.rs`) does only what needs script:
//! `chat.ts` (the live chat over SSE, the scroll window, drafts, the offline copy, report mode, clipboard, push),
//! `look.ts` (a pick applied in place, the marks of what a page adds), `frame.ts` (a plugin frame's bridge) and
//! `sw.ts` (the service worker). Also the style sheet, the PWA's manifest and icons, and the themes' files.

#![forbid(unsafe_code)]

mod agents;
mod look;
mod sheet;
mod shown;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

pub use agents::{Mate, Pane, Side, page};
use cooee_core::{Account, Sheet, Status, Thread, Turn};
pub use look::{Looks, Pair, Scheme, TOKENS, Theme, gate, plain};
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
/// `POST /look` (a pick with no script) and each theme's data at `/look/<theme>/{theme.json,sprites.svg,scene.svg,icon.png}`;
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

/// The chat header: the agent's name, the title (an agent's ongoing chat is titled by its name; a side thread by
/// its own, its agent first in the line), and the line before its state: a Workflow session's role and round, else
/// profile (Plain), agent, account, model and folder.
fn bar(
    thread: &Thread,
    mate: Option<&(usize, String, bool)>,
    profile: &str,
    folder: &str,
) -> (String, String, String) {
    let (name, title, by) = match mate {
        Some((_, n, true)) => (n.clone(), n.clone(), ""),
        Some((_, n, false)) => (n.clone(), thread.title.clone(), n.as_str()),
        None => (thread.agent.clone(), thread.title.clone(), ""),
    };
    if let Some(w) = &thread.workflow {
        return (name, title, format!("{} · round {} · ", w.role, w.round));
    }
    let profile = if profile.is_empty() { "Plain" } else { profile };
    let line = [
        by,
        profile,
        &thread.agent,
        &thread.account,
        &thread.model,
        folder,
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    .flat_map(|s| [s, " · "])
    .collect();
    (name, title, line)
}

/// `~/x` for a folder under `$HOME` (as given, or resolved: a folder a chat works in is stored resolved).
#[must_use]
pub fn short(folder: &Path) -> String {
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
        "error" => "fail",
        "done" => "done",
        "ended" => "ended",
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

/// One thread (the pane of `/t/{id}`): its bar (its agent's avatar and name, the title of a side thread), the
/// items (drawn by `chat.ts`), the working pill, and the composer with the folder, model and effort chips.
#[component]
async fn chat(
    cx: &Cx,
    thread: Thread,
    turn: Turn,
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
    let (name, title, said) = bar(&thread, mate.as_ref(), &profile, &folder);
    let ended = thread.closed || turn.state == "ended";
    // A Workflow session (#33): its role and round, and a link to what it works on; its tool closes it.
    let workflow = thread.workflow.clone();
    let link = workflow
        .as_ref()
        .map(|w| w.url.clone())
        .filter(|u| u.starts_with("https://") || u.starts_with("http://"));
    Ok(view! {
        <div
            class="chat"
            data-thread=(thread.id.clone())
            data-agent=(name.clone())
            data-read=(thread.read.unwrap_or(0).to_string())
            data-state=(turn.state.clone())
            data-ids=(ids.join(" "))
        >
            <div class="th">
                <a class="gl bk" href="/?list" aria-label="back to agents">"‹"</a>
                if let Some((n, _, _)) = mate {
                    agents::avatar(name: name.clone(), n: n, big: false)
                }
                <div class="tb">
                    <div class="tn">
                        <h1>(title)</h1>
                        <small>(thread.id.clone())</small>
                    </div>
                    <span class="ds">
                        <i class="dt"></i>
                        (said)
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
            if !ended {
                agents::chips(
                    places: thread.workflow.is_none().then_some(places),
                    choices: choices,
                    now: (thread.model.clone(), thread.effort.clone())
                )
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
        </div>
    })
}
