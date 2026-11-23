use std::{collections::BTreeMap, io, sync::Arc};

use cooee_core::Config;
use serde::{Deserialize, Serialize};
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        RouterBuilder,
        content::Json,
        response::{IntoResponse, Response},
        route,
    },
};

use crate::{
    contrast,
    http::{Answer, Data, Name, blocking, refused},
    look::{LookStyle, Looks, Pair, Theme},
    themes::Themes,
};

/// A theme the editor made: the pack it copies figures, scene, icon and words from, its own label, style and
/// colours per mode, and the figures and backdrop drawn over the copy.
#[derive(Debug, Clone, Deserialize)]
pub struct Draft {
    /// The installed pack it starts from, or `starter`.
    pub from: String,
    /// The name shown in the theme list.
    pub label: String,
    /// The style a device takes when it picks the theme.
    pub style: LookStyle,
    /// `--bg`, `--fg`, … in dark mode.
    pub dark: BTreeMap<String, String>,
    /// The same in light mode.
    pub light: BTreeMap<String, String>,
    /// Drawn figures by symbol id, each in place of its `<symbol>` in the copied `sprites.svg`.
    #[serde(default)]
    pub sprites: BTreeMap<String, String>,
    /// A drawn `scene.svg` of the layers `sky sea rocks sand` and `#night`, in place of the copied one.
    #[serde(default)]
    pub scene: Option<String>,
}

/// The colours of a draft, per mode: what the gate's editor shows.
#[derive(Debug, Clone, Deserialize)]
pub struct Colours {
    /// `--bg`, `--fg`, … in dark mode.
    pub dark: BTreeMap<String, String>,
    /// The same in light mode.
    pub light: BTreeMap<String, String>,
}

/// One token's gate result: its worst check, and a shade that passes when it fails.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Mark {
    /// What its worst check measures.
    pub what: String,
    /// That check's value.
    pub value: f64,
    /// That check's minimum.
    pub min: f64,
    /// The nearest passing colour, when it fails.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
    /// The token's worst ratio with that fix.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixed: Option<f64>,
}

/// The gate on one mode's colours: every failure, and each token's mark.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Report {
    /// The gate's failures, as `cooee theme check` says them.
    pub fails: Vec<String>,
    /// Per token, its worst check.
    pub marks: BTreeMap<String, Mark>,
}

fn scheme(label: &str, mode: &str, tokens: &BTreeMap<String, String>) -> serde_json::Value {
    serde_json::json!({"label": format!("{label} {mode}"), "color-scheme": mode, "tokens": tokens})
}

/// The gate on `tokens`, token by token, with a fix for each that fails.
#[must_use]
pub fn report(tokens: &BTreeMap<String, String>) -> Report {
    let all = match contrast::checks(tokens) {
        Ok(all) => all,
        Err(e) => {
            return Report {
                fails: vec![e],
                marks: BTreeMap::new(),
            };
        }
    };
    let mut marks = BTreeMap::new();
    for c in &all {
        let worse = marks
            .get(c.token)
            .is_none_or(|m: &Mark| c.value / c.min < m.value / m.min);
        if worse {
            let fix = (c.value < c.min)
                .then(|| contrast::fix(tokens, c.token))
                .flatten();
            let fixed = fix.as_ref().and_then(|f| worst(tokens, c.token, f));
            let (what, value, min) = (c.what.clone(), c.value, c.min);
            marks.insert(
                c.token.to_owned(),
                Mark {
                    what,
                    value,
                    min,
                    fix,
                    fixed,
                },
            );
        }
    }
    let fails = serde_json::from_value(scheme("", "dark", tokens))
        .map_err(|e| e.to_string())
        .and_then(|s| crate::look::gate(&s))
        .err()
        .into_iter()
        .collect();
    Report { fails, marks }
}

fn worst(tokens: &BTreeMap<String, String>, token: &str, shade: &str) -> Option<f64> {
    let mut t = tokens.clone();
    t.insert(token.to_owned(), shade.to_owned());
    contrast::checks(&t)
        .ok()?
        .into_iter()
        .filter(|c| c.token == token)
        .min_by(|a, b| (a.value / a.min).total_cmp(&(b.value / b.min)))
        .map(|c| c.value)
}

const STARTER: [(&str, &[u8]); 2] = [
    (
        "sprites.svg",
        include_bytes!("../../../packs/starter/sprites.svg"),
    ),
    (
        "scene.svg",
        include_bytes!("../../../packs/starter/scene.svg"),
    ),
];

/// The files of pack `name` as `draft` makes it: `theme.json`, its two schemes, and the art of the pack it copies.
///
/// # Errors
/// When the pack it starts from isn't installed.
pub fn files(name: &str, draft: &Draft, looks: &Looks) -> io::Result<Vec<(String, Vec<u8>)>> {
    let from = copied(draft, looks)?;
    let (dark, light) = (format!("{name}-dark"), format!("{name}-light"));
    let theme = Theme {
        label: draft.label.clone(),
        scheme: Pair {
            dark: dark.clone(),
            light: light.clone(),
        },
        style: Some(draft.style),
        ..from.clone()
    };
    let mut out = vec![
        ("theme.json".to_owned(), pretty(&theme)),
        (
            format!("schemes/{dark}.json"),
            pretty(&scheme(&draft.label, "dark", &draft.dark)),
        ),
        (
            format!("schemes/{light}.json"),
            pretty(&scheme(&draft.label, "light", &draft.light)),
        ),
    ];
    for (file, bytes) in [
        ("sprites.svg", from.sprite_sheet),
        ("scene.svg", from.scene_art),
        ("icon.png", from.icon),
    ] {
        out.extend(bytes.map(|b| (file.to_owned(), b)));
    }
    Ok(out)
}

fn copied(draft: &Draft, looks: &Looks) -> io::Result<Theme> {
    let starter = Theme {
        label: String::new(),
        scheme: Pair {
            dark: String::new(),
            light: String::new(),
        },
        style: None,
        sprites: true,
        moments: false,
        scene: Vec::new(),
        load_scene: None,
        words: BTreeMap::new(),
        sprite_sheet: Some(STARTER[0].1.to_vec()),
        icon: None,
        scene_art: Some(STARTER[1].1.to_vec()),
    };
    let mut from = match looks.themes.get(&draft.from) {
        Some(t) => t.clone(),
        None if draft.from == "starter" => starter,
        None => {
            return Err(io::Error::other(format!(
                "no theme {} to start from",
                draft.from
            )));
        }
    };
    if !draft.sprites.is_empty() {
        let sheet = from.sprite_sheet.as_deref().unwrap_or(STARTER[0].1);
        from.sprite_sheet = Some(splice(sheet, &draft.sprites)?);
        from.sprites = true;
    }
    if let Some(scene) = &draft.scene {
        from.scene_art = Some(scene.clone().into_bytes());
        from.scene = ["sky", "sea", "rocks", "sand"].map(str::to_owned).to_vec();
    }
    Ok(from)
}

fn splice(sheet: &[u8], drawn: &BTreeMap<String, String>) -> io::Result<Vec<u8>> {
    const CLOSE: &str = "</symbol>";
    let mut text = String::from_utf8_lossy(sheet).into_owned();
    for (id, symbol) in drawn {
        let open = format!("<symbol id=\"{id}\"");
        let named = id.strip_prefix("s-").is_some_and(|n| {
            !n.is_empty() && n.bytes().all(|b| b.is_ascii_lowercase() || b == b'-')
        });
        if !named
            || !symbol.starts_with(&open)
            || symbol
                .strip_suffix(CLOSE)
                .is_none_or(|body| body.contains(CLOSE))
        {
            return Err(io::Error::other(format!(
                "{id}: not one <symbol id=\"{id}\">"
            )));
        }
        let old = text.find(&open).and_then(|at| {
            let end = text.get(at..)?.find(CLOSE)?;
            Some(at..at + end + CLOSE.len())
        });
        match old {
            Some(range) => text.replace_range(range, symbol),
            None => text.insert_str(text.rfind("</svg>").unwrap_or(text.len()), symbol),
        }
    }
    Ok(text.into_bytes())
}

fn pretty<T: Serialize>(v: &T) -> Vec<u8> {
    let mut text = serde_json::to_vec_pretty(v).unwrap_or_default();
    text.push(b'\n');
    text
}

#[route(POST "/api/v1/themes/check")]
async fn check(cx: &Cx, Json(c): Json<Colours>) -> Result<Response> {
    let both = BTreeMap::from([("dark", report(&c.dark)), ("light", report(&c.light))]);
    Json(both).into_response(cx)
}

/// What a save did: false when the pack on disk was the same already.
#[derive(Debug, Serialize)]
pub struct Saved {
    /// Files were written.
    pub written: bool,
}

#[route(PUT "/api/v1/themes/{name}")]
async fn save(cx: &Cx, Json(draft): Json<Draft>) -> Result<Answer<Saved>> {
    let name = topcoat::router::path_param::<Name>(cx).to_owned();
    let config = Arc::clone(app_context::<Arc<Config>>(cx));
    let dir = app_context::<Data>(cx).1.clone();
    let done = blocking(cx, move |_| {
        let (looks, _) = Looks::load(dir.as_deref(), None);
        let made = files(&name, &draft, &looks).and_then(|f| Themes::new(&config)?.make(&name, &f));
        made.map(|written| Saved { written }).map_err(refused)
    });
    Ok(Answer(done.await))
}

pub(crate) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(check).route(save)
}
