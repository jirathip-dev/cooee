//! The sheets (#49): the look sheet, Settings and Settings › Folders as server-rendered popovers in `#sheet`, so they
//! open with no script. The look sheet is a form: with no script a key posts the pick (`POST /look` sets the cookie
//! and goes back); `look.ts` applies it in place instead and keeps the marks in step. `chat.ts` fills Settings'
//! alerts, devices and plugins, and `look.ts` Settings › Folders, from the JSON API.

use std::sync::Arc;

use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        Body,
        content::Form,
        request::{headers, uri},
        response::Response,
        route,
    },
    view::{View, component, view},
};

use crate::{
    LookAssets,
    look::{SCHEMES, Scheme, THEMES},
    shown::{COOKIE, Shown},
};

/// The device's pick for this request.
pub fn shown(cx: &Cx) -> Shown<'_> {
    let looks = &app_context::<Arc<LookAssets>>(cx).looks;
    Shown::new(
        looks,
        headers(cx).get("cookie").and_then(|v| v.to_str().ok()),
    )
}

/// Built-in themes and schemes in the design's order, then the owner's own by name.
fn sorted<'a, T>(all: impl Iterator<Item = (&'a String, &'a T)>, order: &[&str]) -> Vec<&'a String>
where
    T: 'a,
{
    let mut names: Vec<&String> = all.map(|(n, _)| n).collect();
    let rank = |n: &str| order.iter().position(|o| *o == n).unwrap_or(99);
    names.sort_by(|a, b| rank(a).cmp(&rank(b)).then(a.cmp(b)));
    names
}

const SECTIONS: [&str; 7] = [
    "sign-ins", "folders", "look", "alerts", "devices", "plugins", "report",
];

fn aria(on: bool) -> &'static str {
    if on { "true" } else { "false" }
}

/// A sheet's header: its title and ×.
#[component]
async fn head(which: &str) -> Result<impl View> {
    Ok(view! {
        <div class="hd">
            <b id=(format!("sht-{which}"))>(which)</b>
            <button
                type="button"
                class="gl"
                data-close=(true)
                popovertarget=(format!("sh-{which}"))
                popovertargetaction="hide"
                aria-label="close"
            >
                "×"
            </button>
        </div>
    })
}

/// A key's values as one radio group (`cur` checked); one not in `shown` is hidden (`look.ts` shows it once offered).
#[component]
async fn seg(
    k: &str,
    list: Vec<&'static str>,
    cur: String,
    #[default] shown: Option<Vec<&'static str>>,
) -> Result<impl View> {
    let list: Vec<(&str, bool)> = list
        .into_iter()
        .map(|v| (v, shown.as_ref().is_some_and(|s| !s.contains(&v))))
        .collect();
    Ok(view! {
        <div class="seg" role="radiogroup" aria-label=(k)>
            for (v, hidden) in list {
                <button
                    hidden=(hidden)
                    type="submit"
                    role="radio"
                    name=(k)
                    value=(v)
                    data-k=(k)
                    data-v=(v)
                    aria-checked=(aria(v == cur))
                >
                    (v)
                </button>
            }
        </div>
    })
}

#[component]
async fn swatch(s: Scheme) -> Result<impl View> {
    let t = |k: &str| s.tokens.get(k).cloned().unwrap_or_default();
    let style = format!("--sbg:{};--sfg:{};--sl:{}", t("--bg"), t("--fg"), t("--l"));
    let states = ["--g", "--a", "--r"].map(|k| format!("background:{}", t(k)));
    Ok(view! {
        <span class="swa" style=(style)>
            <b>"Aa"</b>
            for c in states {
                <i style=(c)></i>
            }
        </span>
    })
}

/// One mode's COLOURS: its schemes, the pick checked and the theme's default marked.
#[component]
async fn colours(cx: &Cx, m: &str, showing: String) -> Result<impl View> {
    let look = shown(cx);
    let looks = &app_context::<Arc<LookAssets>>(cx).looks;
    let rows: Vec<(String, Scheme, bool, bool)> =
        sorted(looks.schemes.iter(), &SCHEMES.map(|s| s.0))
            .into_iter()
            .filter(|n| looks.schemes[*n].color_scheme == m)
            .map(|n| {
                let default = look.theme().scheme_for(m) == n;
                (
                    n.clone(),
                    looks.schemes[n].clone(),
                    default,
                    look.scheme(m) == n,
                )
            })
            .collect();
    Ok(view! {
        <p class="ch" data-m=(m)>
            (format!("{m} colours"))
            if m == showing {
                <span class="shw">" · showing"</span>
            }
        </p>
        <div class="lsl" role="listbox" aria-label=(format!("{m} colours"))>
            for (n, s, default, on) in rows {
                <button
                    type="submit"
                    role="option"
                    class="pick"
                    name=(m)
                    value=(n.clone())
                    data-k=(m)
                    data-v=(n)
                    aria-selected=(aria(on))
                >
                    swatch(s: s.clone())
                    <span class="nm">(s.label.to_lowercase())</span>
                    if default {
                        <small>"default"</small>
                    }
                    if on {
                        <i class="tk">"✓"</i>
                    }
                </button>
            }
        </div>
    })
}

/// The look sheet: STYLE, FIGURES (Scene and Sprite only where offered), THEME, APPEARANCE and the dark and light COLOURS.
#[component]
async fn look_sheet(cx: &Cx) -> Result<impl View> {
    let look = shown(cx);
    let looks = &app_context::<Arc<LookAssets>>(cx).looks;
    let to = uri(cx)
        .path_and_query()
        .map_or("/", |p| p.as_str())
        .to_owned();
    let showing = look.mode().unwrap_or("dark").to_owned();
    let tiles: Vec<(String, String, bool)> = sorted(looks.themes.iter(), &THEMES.map(|t| t.0))
        .into_iter()
        .map(|n| (n.clone(), looks.themes[n].label.clone(), n == look.name()))
        .collect();
    Ok(view! {
        <div
            class="sh lks"
            id="sh-look"
            popover="auto"
            role="dialog"
            aria-modal="false"
            aria-labelledby="sht-look"
        >
            head(which: "look")
            <form class="top" method="post" action="/look">
                <input type="hidden" name="to" value=(to)>
                <div class="lrw">
                    <span>"style"</span>
                    seg(
                        k: "style",
                        list: vec!["modern", "terminal", "scene"],
                        cur: look.style().to_owned(),
                        shown: Some(look.styles())
                    )
                </div>
                <div class="lrw">
                    <span>"figures"</span>
                    seg(
                        k: "figures",
                        list: vec!["sprite", "ascii", "none"],
                        cur: look.figures().to_owned(),
                        shown: Some(look.figure_list())
                    )
                </div>
                <p class="ch">"theme"</p>
                <div class="lth" role="listbox" aria-label="theme">
                    for (n, label, on) in tiles {
                        <button
                            type="submit"
                            role="option"
                            class="pick"
                            name="theme"
                            value=(n.clone())
                            data-k="theme"
                            data-v=(n.clone())
                            aria-selected=(aria(on))
                        >
                            <img
                                alt=""
                                loading="lazy"
                                src=(format!("/look/{n}/icon.png"))
                            >
                            <span>(label)</span>
                            if on {
                                <i class="tk">"✓"</i>
                            }
                        </button>
                    }
                </div>
                <p class="ch">"appearance"</p>
                seg(
                    k: "mode",
                    list: vec!["system", "light", "dark"],
                    cur: look.appearance().to_owned()
                )
                colours(m: "dark", showing: showing.clone())
                colours(m: "light", showing: showing)
                <p class="lft">
                    "this device only · iOS keeps the icon it installed with"
                </p>
            </form>
        </div>
    })
}

/// A Settings row that opens the look sheet, with its value.
#[component]
async fn look_row(k: &str, v: String) -> Result<impl View> {
    Ok(view! {
        <button type="button" class="sr" data-open="look" popovertarget="sh-look">
            <span class="sk">(k)</span>
            <span class="sv" data-sv=(k)>(v)</span>
            <i class="cv">"▸"</i>
        </button>
    })
}

/// Settings: its index, sign-ins, folders, the look's rows, alerts, devices and report.
#[component]
async fn settings(cx: &Cx) -> Result<impl View> {
    let look = shown(cx);
    let label = |m: &str| look.scheme_of(look.scheme(m)).label.to_lowercase();
    let appearance = match look.mode() {
        Some(m) => m.to_owned(),
        None => "system · dark now".to_owned(),
    };
    let both = format!("dark {} · light {}", label("dark"), label("light"));
    Ok(view! {
        <div
            class="sh set"
            id="sh-settings"
            popover="auto"
            role="dialog"
            aria-modal="true"
            aria-labelledby="sht-settings"
        >
            head(which: "settings")
            <div class="top">
                <nav class="six" aria-label="sections">
                    for s in SECTIONS {
                        <a href=(format!("#settings/{s}"))>(s)</a>
                    }
                </nav>
                <p class="ch" id="s-sign-ins">"sign-ins"</p>
                <a class="sr" href="/#signins" data-close=(true)>
                    <span class="sk">"providers"</span>
                    <i class="cv">"▸"</i>
                </a>
                <p class="ch" id="s-folders">"folders"</p>
                <button
                    type="button"
                    class="sr"
                    data-open="folders"
                    popovertarget="sh-folders"
                >
                    <span class="sk">"roots"</span>
                    <i class="cv">"▸"</i>
                </button>
                <p class="ch" id="s-look">"look"</p>
                look_row(k: "style", v: look.style().to_owned())
                look_row(k: "figures", v: look.figures().to_owned())
                look_row(k: "theme", v: look.theme().label.clone())
                look_row(k: "appearance", v: appearance)
                look_row(k: "colours", v: both)
                <p class="ch" id="s-alerts">"alerts"</p>
                <div id="s-alerts-box"></div>
                <p class="ch" id="s-devices">"devices"</p>
                <div id="s-devices-box"></div>
                <p class="ch" id="s-plugins">"plugins"</p>
                <div id="s-plugins-box"></div>
                <p class="ch" id="s-report">"report"</p>
                <button type="button" class="sr" data-report=(true)>
                    <span class="sk">"report a problem"</span>
                    <i class="cv">"▸"</i>
                </button>
            </div>
        </div>
    })
}

/// Every sheet, closed.
#[component]
pub async fn sheets() -> Result<impl View> {
    Ok(view! {
        <div id="sheet">
            look_sheet()
            settings()
            <div
                class="sh set"
                id="sh-folders"
                popover="auto"
                role="dialog"
                aria-modal="true"
                aria-labelledby="sht-folders"
            >
                head(which: "folders")
                <div class="top"></div>
            </div>
        </div>
    })
}

/// `POST /look`, the look sheet with no script: the key tapped (one of these) and the page to go back to.
#[derive(Deserialize)]
struct Tap {
    to: String,
    theme: Option<String>,
    mode: Option<String>,
    style: Option<String>,
    figures: Option<String>,
    dark: Option<String>,
    light: Option<String>,
}

#[route(POST "/look")]
pub async fn look_set(cx: &Cx, Form(t): Form<Tap>) -> Result<Response> {
    let keys = [
        ("theme", &t.theme),
        ("mode", &t.mode),
        ("style", &t.style),
        ("figures", &t.figures),
        ("dark", &t.dark),
        ("light", &t.light),
    ];
    let look = shown(cx);
    let value = keys
        .iter()
        .find_map(|(k, v)| Some(look.with(k, v.as_deref()?)))
        .unwrap_or_default();
    // Back to a page of this server only.
    let to = if t.to.starts_with('/')
        && !t.to.starts_with("//")
        && !t.to.contains('\\')
        && !t.to.chars().any(char::is_control)
    {
        t.to.as_str()
    } else {
        "/"
    };
    Ok(Response::builder()
        .status(303)
        .header("Location", to)
        .header(
            "Set-Cookie",
            format!("{COOKIE}={value}; Path=/; Max-Age=315360000; SameSite=Lax"),
        )
        .body(Body::from(()))?)
}
