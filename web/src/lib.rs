//! cooee's web UI in Topcoat (docs/decisions/0001-ui-framework.md), in the Modern style: the thread list and the
//! chat page (server-rendered), the pairing page, the chat's script (`chat.js`: the window of items, SSE, the outbox,
//! drafts and turning alerts on), the style sheet and the installable PWA: its manifest, icons and the service worker
//! (`sw.js`: push only, nothing cached), plugin tabs (`chat.js` hosts a plugin's sandboxed frame), and the look (`look.rs` themes and schemes, `look.js` applying a device's
//! pick before the first paint, its look sheet and Settings).

#![forbid(unsafe_code)]

mod look;

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use cooee_core::{AGENTS_FIX, Agent, Provider, ROOTS_FIX, Status, Summary, Thread, Turn};
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

/// The static routes: the manifest, the icons, the style sheet, the chat's script and the service worker; and the
/// look: `look.js` with every theme and scheme in it, and each theme's data at
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
        .route(icon_192)
        .route(icon_512)
        .route(style)
        .route(script)
        .route(worker)
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

/// The plugin API inside a plugin's frame (`frame.js`): the server puts it before the plugin's module.
pub const FRAME_JS: &str = include_str!("frame.js");

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
                <script src="/look.js"></script>
                <script type="module" src="/chat.js"></script>
            </head>
            <body data-at=(now().to_string())>
                (child)
                <p id="ld" role="status" hidden=(true)>
                    <span class="lw" data-w="loading"></span>
                    <span class="ago"></span>
                </p>
                <p id="toast" role="status" hidden=(true)></p>
            </body>
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

/// The header: the app's name (the heading on the thread list, with the 🔔 that turns alerts on; else a link to
/// it), the plugin tabs (`current` open), `+`, the look sheet and Settings.
#[component]
async fn header(home: bool, tabs: Vec<(String, String)>, current: String) -> Result<impl View> {
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

/// Chat and the plugin tabs (`(name, label)`), when there are any; `current` is the open one (`""`: Chat).
#[component]
async fn nav(tabs: Vec<(String, String)>, current: String) -> Result<impl View> {
    Ok(view! {
        if !tabs.is_empty() {
            <nav class="tabs" aria-label="tabs">
                <a href="/" class=(if current.is_empty() { "on" } else { "" })>
                    "Chat"
                </a>
                for (name, label) in tabs {
                    <a
                        href=(format!("/p/{name}"))
                        class=(if name == current { "on" } else { "" })
                    >
                        (label)
                    </a>
                }
            </nav>
        }
    })
}

/// The thread list: one card per working folder, its 3 newest threads shown, the rest folded; the new-thread
/// form (open when there are no threads, else behind `+`), each agent with its provider's state (no agents or no
/// folders: the fix line instead); the
/// sign-ins card (`#signins`, from Settings): each provider's state, and its login's terminal (drawn by `chat.js`); `tabs`: the
/// plugins' names and labels.
#[must_use]
pub fn threads_page(
    cx: &Cx,
    rows: Vec<Summary>,
    agents: Result<Vec<Agent>, String>,
    folders: Vec<PathBuf>,
    providers: Vec<(Provider, Status)>,
    tabs: Vec<(String, String)>,
) -> impl View {
    view! {
        cx =>
        threads(
            rows: rows,
            agents: agents,
            folders: folders,
            providers: providers,
            tabs: tabs
        )
    }
}

#[component]
async fn provider(p: Provider, s: Status) -> Result<impl View> {
    let (class, label) = match (s.signed_in, &s.install) {
        (true, _) => ("g", "✓ signed in"),
        (false, None) => ("m", "signed out"),
        (false, Some(_)) => ("m", "not installed"),
    };
    Ok(view! {
        <div class="pv" data-provider=(p.name.clone())>
            <div class="fh">
                <b>(p.name.clone())</b>
                <span class=(class)>(label)</span>
                <button
                    type="button"
                    class="si"
                    hidden=(s.signed_in || s.install.is_some())
                >
                    "sign in"
                </button>
            </div>
            <p class="ds">(s.line)</p>
            if let Some(fix) = s.install.filter(|f| !f.is_empty()) {
                hint(why: "on the Mac:", fix: fix.as_str(), then: "")
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
async fn signins(providers: Vec<(Provider, Status)>) -> Result<impl View> {
    Ok(view! {
        <section class="nf" id="signins" hidden=(true)>
            <div class="fh">
                <b>"sign-ins"</b>
                <a class="gl" href="#" aria-label="close sign-ins">"×"</a>
            </div>
            for (p, s) in providers {
                provider(p: p, s: s)
            }
        </section>
    })
}

/// One agent in the new-thread picker: signed in `✓`, or signed out with a link to the sign-ins card.
#[component]
async fn agent(id: String, first: bool, signed: Option<bool>) -> Result<impl View> {
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
        if signed == Some(false) {
            <a class="so" href="#signins">(format!("sign in {id}"))</a>
        }
    })
}

/// An empty picker's row instead: why, and the line that fixes it with its copy button (`chat.js`).
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

/// The new-thread form's folder: a picker of `folders`, or the fix line when there are none.
#[component]
async fn folder_picker(folders: Vec<PathBuf>) -> Result<impl View> {
    Ok(view! {
        if folders.is_empty() {
            <p class="lb">"folder"</p>
            hint(why: "no folders set · add", fix: ROOTS_FIX, then: "to config.toml")
        } else {
            <label class="lb" for="folder">"folder"</label>
            <select id="folder" name="folder">
                for f in folders {
                    <option value=(f.display().to_string())>(short(&f))</option>
                }
            </select>
        }
    })
}

#[component]
async fn threads(
    rows: Vec<Summary>,
    agents: Result<Vec<Agent>, String>,
    folders: Vec<PathBuf>,
    providers: Vec<(Provider, Status)>,
    tabs: Vec<(String, String)>,
) -> Result<impl View> {
    let empty = rows.is_empty();
    // Start waits for an agent and a folder to pick.
    let can_start = !folders.is_empty() && agents.as_ref().is_ok_and(|a| !a.is_empty());
    let signed: BTreeMap<String, bool> = providers
        .iter()
        .rev()
        .map(|(p, s)| (p.agent.clone(), s.signed_in))
        .collect();
    let mut cards: BTreeMap<PathBuf, Vec<Summary>> = BTreeMap::new();
    for r in rows {
        cards.entry(r.folder.clone()).or_default().push(r);
    }
    Ok(view! {
        document(
            title: "cooee",
            header(home: true, tabs: tabs, current: String::new())
            <main class="list">
                signins(providers: providers)
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
                                        signed: signed.get(&a.id).copied()
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

/// Plugin `name`'s tab: its module runs in a sandboxed frame that `chat.js` hosts.
#[must_use]
pub fn plugin_page(cx: &Cx, tabs: Vec<(String, String)>, name: String, label: String) -> impl View {
    view! { cx => plugin(tabs: tabs, name: name, label: label) }
}

#[component]
async fn plugin(tabs: Vec<(String, String)>, name: String, label: String) -> Result<impl View> {
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
                    aria-label="look: theme and colours"
                    aria-pressed="false"
                >
                    "◐"
                </button>
            </header>
            <main class="list">
                <p class="hn">"This device isn't paired: threads and messages show once it is."</p>
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
                        <button type="button" class="cn" id="ask-cancel">"cancel"</button>
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
                header(home: false, tabs: Vec::new(), current: String::new())
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
