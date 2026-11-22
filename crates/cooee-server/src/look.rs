use std::{collections::BTreeMap, fs, path::Path};

use blocks::theme::{Mode, Scheme};
use serde::{Deserialize, Serialize};

mod pack;

pub use pack::{LoadScene, STYLES, Style, check, read_pack};

/// blocks' contrast gate on `s`, as `cooee theme check` says a fail.
///
/// # Errors
/// Each check it fails, with its value and minimum.
pub fn gate(s: &Scheme) -> Result<(), String> {
    let fails: Vec<String> = blocks::theme::gate(s, blocks::theme::Fills::Tint)
        .iter()
        .map(|m| format!("{} {:.2} (min {})", m.what, m.got, m.min))
        .collect();
    let why = || format!("fails the contrast gate: {}", fails.join("; "));
    fails.is_empty().then_some(()).ok_or_else(why)
}

/// A theme's default scheme per mode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pair {
    /// The scheme in dark mode.
    pub dark: String,
    /// The scheme in light mode.
    pub light: String,
}

/// A theme: its words, its default schemes and whether it has sprites and moments. Data only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    /// The name shown on its tile.
    pub label: String,
    /// The default scheme per mode.
    pub scheme: Pair,
    /// The style a device takes once when it picks this theme (none: it keeps its own).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<LookStyle>,
    /// Has `sprites.svg` (symbols `s-edit s-read s-run s-idle s-done s-needs s-merged s-fail s-loading s-empty`).
    #[serde(default)]
    pub sprites: bool,
    /// Plays the row moments.
    #[serde(default)]
    pub moments: bool,
    /// The Scene's layers: element ids in `scene.svg` (`viewBox` at most 400 × 320); none: no scene.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scene: Vec<String>,
    /// What the Scene draws for its host's load; none: the weather sky.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub load_scene: Option<LoadScene>,
    /// The theme's word per state key (`working needs done fail idle merged loading empty-threads`); a missing key
    /// says the plain word.
    #[serde(default)]
    pub words: BTreeMap<String, String>,
    /// `sprites.svg`, when `sprites`.
    #[serde(skip)]
    pub sprite_sheet: Option<Vec<u8>>,
    /// `icon.png`, the home-screen icon and favicon.
    #[serde(skip)]
    pub icon: Option<Vec<u8>>,
    /// `scene.svg`, when it has a scene.
    #[serde(skip)]
    pub scene_art: Option<Vec<u8>>,
}

/// A built-in style: the shape, radius, type and buttons every scheme is drawn with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LookStyle {
    /// Cards and rounded rows.
    Modern,
    /// Monospaced, the terminal's look.
    Terminal,
    /// Home as the theme's scene.
    Scene,
}

/// The plain word for a state key: what every theme says beside its own word, and all the plain theme says.
#[must_use]
pub fn plain(key: &str) -> &str {
    match key {
        "needs" => "needs you",
        "fail" => "failed",
        "merged" => "done",
        "empty-threads" => "no threads",
        k => k,
    }
}

impl Theme {
    /// The theme's word for a state key, else the plain word.
    #[must_use]
    pub fn word<'a>(&'a self, key: &'a str) -> &'a str {
        self.words
            .get(key)
            .map_or_else(|| plain(key), String::as_str)
    }

    /// The default scheme for mode `m` (`dark` or `light`).
    #[must_use]
    pub fn scheme_for(&self, m: &str) -> &str {
        if m == "light" {
            &self.scheme.light
        } else {
            &self.scheme.dark
        }
    }

    /// A push's body: the theme word, the plain word, then the agent's line (`at the gate · needs you · acme: …`);
    /// a theme without its own word says the plain word once.
    #[must_use]
    pub fn push_body(&self, key: &str, line: &str) -> String {
        let (word, plain) = (self.word(key), plain(key));
        if word == plain {
            format!("{plain} · {line}")
        } else {
            format!("{word} · {plain} · {line}")
        }
    }
}

/// Every theme and scheme cooee serves, and the server's default theme (a device's own pick wins).
#[derive(Debug, Clone, Serialize)]
pub struct Looks {
    /// The theme a device without a pick shows.
    pub default: String,
    /// By folder name.
    pub themes: BTreeMap<String, Theme>,
    /// By file name.
    pub schemes: BTreeMap<String, Scheme>,
    /// The ASCII figures, for `look.ts`.
    pub ascii: &'static [(&'static str, [&'static str; 2])],
    /// The style packs that pass the gate, by folder name.
    pub styles: BTreeMap<String, Style>,
    /// Why each own pack fails the gate now, by folder name: its theme, else its last good copy, is still served.
    pub broken: BTreeMap<String, String>,
}

/// Where the last copy of own pack `name` that passed the gate is kept, under `config_dir`.
#[must_use]
pub fn last_good(config_dir: &Path, name: &str) -> std::path::PathBuf {
    config_dir.join(".themes-stage").join(".good").join(name)
}

macro_rules! builtin_theme {
    ($name:literal, $sprites:expr, $scene:expr) => {
        (
            $name,
            include_str!(concat!(
                "../../../design-system/themes/",
                $name,
                "/theme.json"
            )),
            $sprites,
            include_bytes!(concat!(
                "../../../design-system/themes/",
                $name,
                "/icon.png"
            ))
            .as_slice(),
            $scene,
        )
    };
}

type Builtin = (
    &'static str,
    &'static str,
    Option<&'static [u8]>,
    &'static [u8],
    Option<&'static [u8]>,
);

const THEMES: [Builtin; 1] = [builtin_theme!("plain", None, None)];

/// The ASCII figures (herdsman#335), the same in every theme: per state (`edit read run` while working), and the
/// 132 × 100 `loading` and `empty`, two frames each that CSS swaps.
pub const ASCII: [(&str, [&str; 2]); 11] = [
    ("edit", ["~o=", "=o~"]),
    ("read", ["o.o", "o.-"]),
    ("run", ["o>_", "o> "]),
    ("needs", ["(!)", "[!]"]),
    ("done", ["\\o/", "|o|"]),
    ("merged", ["\\o/", "-o-"]),
    ("fail", ["x_x", "x.x"]),
    ("idle", ["-.-", "-_-"]),
    ("ended", ["[ ]", "[_]"]),
    ("loading", ["( o.o )\n  ...", "( o.O )\n  .. "]),
    ("empty", ["( -.- )\n  z  ", "( -.- )\n  zZ "]),
];

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

impl Looks {
    /// Theme `name`, or `plain` for the old `terminal` theme (#35) unless the owner has a theme of that name.
    #[must_use]
    pub fn renamed<'a, T>(themes: &BTreeMap<String, T>, name: &'a str) -> &'a str {
        if name == "terminal" && !themes.contains_key(name) {
            "plain"
        } else {
            name
        }
    }

    /// The built-in theme and schemes, then the owner's own from `config_dir`: schemes, the packs in `themes/` (an own
    /// one of the same name replaces a built-in; a pack's own schemes join the rest).
    /// `default` is the server's default theme (else farm, when it is installed, else plain). Returns why each own
    /// file was refused (unreadable, invalid, failing the gate); a built-in that doesn't parse (a build bug the tests
    /// catch) is left out.
    #[must_use]
    pub fn load(config_dir: Option<&Path>, default: Option<&str>) -> (Self, Vec<String>) {
        let (mut refused, mut styles, mut broken) = (Vec::new(), BTreeMap::new(), BTreeMap::new());
        let mut schemes = builtin_schemes();
        let mut themes: BTreeMap<String, Theme> = THEMES
            .iter()
            .filter_map(|(n, json, sprites, icon, scene)| {
                let mut t: Theme = serde_json::from_str(json).ok()?;
                t.sprite_sheet = sprites.map(<[u8]>::to_vec);
                t.icon = Some(icon.to_vec());
                t.scene_art = scene.map(<[u8]>::to_vec);
                Some(((*n).to_owned(), t))
            })
            .collect();
        if let Some(dir) = config_dir {
            for (name, path) in entries(&dir.join("schemes"), &mut refused) {
                let Some(name) = name.strip_suffix(".json") else {
                    continue;
                };
                match read_scheme(&path).and_then(|s| gate(&s).map(|()| s)) {
                    Ok(s) if valid_name(name) => drop(schemes.insert(name.to_owned(), s)),
                    Ok(_) => refused.push(format!("{}: not a-z, 0-9 and -", path.display())),
                    Err(e) => refused.push(format!("{}: {e}", path.display())),
                }
            }
            for (name, path) in entries(&dir.join("themes"), &mut refused) {
                match read_pack(&path, &schemes) {
                    _ if !valid_name(&name) => {
                        refused.push(format!("{}: not a-z, 0-9 and -", path.display()));
                    }
                    Ok(p) => {
                        schemes.extend(p.schemes);
                        themes.extend(p.theme.map(|t| (name.clone(), t)));
                        styles.extend(p.style.map(|s| (name, s)));
                    }
                    Err(e) => {
                        refused.push(format!("{}: {}", path.display(), e.join("; ")));
                        let style = |w: &String| w.starts_with("style.json");
                        let why = e
                            .iter()
                            .find(|w| style(w))
                            .or(e.first())
                            .cloned()
                            .unwrap_or_default();
                        if let (true, Ok(Some((t, own)))) =
                            (e.iter().all(style), read_theme(&path, &schemes))
                        {
                            schemes.extend(own);
                            themes.insert(name.clone(), t);
                            broken.insert(name, why);
                        } else if let Ok(p) = read_pack(&last_good(dir, &name), &schemes) {
                            schemes.extend(p.schemes);
                            themes.extend(p.theme.map(|t| (name.clone(), t)));
                            styles.extend(p.style.map(|s| (name.clone(), s)));
                            broken.insert(name, why);
                        } else if e.iter().any(style) {
                            broken.insert(name, why);
                        }
                    }
                }
            }
        }
        let fallback = if themes.contains_key("farm") {
            "farm"
        } else {
            "plain"
        };
        let default = match default.map(|d| Self::renamed(&themes, d)) {
            Some(d) if themes.contains_key(d) => d.to_owned(),
            Some(d) => {
                refused.push(format!("theme = {d:?}: no such theme, showing {fallback}"));
                fallback.to_owned()
            }
            None => fallback.to_owned(),
        };
        (
            Self {
                default,
                themes,
                schemes,
                ascii: &ASCII,
                styles,
                broken,
            },
            refused,
        )
    }
}

fn builtin_schemes() -> BTreeMap<String, Scheme> {
    swag::schemes()
        .iter()
        .filter_map(|(n, json)| Some(((*n).to_owned(), serde_json::from_str(json).ok()?)))
        .collect()
}

/// The state keys a theme may word.
pub const WORDS: [&str; 9] = [
    "working",
    "needs",
    "done",
    "fail",
    "idle",
    "ended",
    "merged",
    "loading",
    "empty-threads",
];

/// The symbols `sprites.svg` must hold: `s-<state>`.
pub const SPRITES: [&str; 10] = [
    "edit", "read", "run", "idle", "done", "needs", "merged", "fail", "loading", "empty",
];

/// The largest Scene art, width × height of its `viewBox` (herdsman's scenes are 390 × 320 and 392 × 188).
pub const SCENE: (f64, f64) = (400.0, 320.0);

fn file(dir: &Path, name: &str, fails: &mut Vec<String>) -> Option<Vec<u8>> {
    let bytes = fs::read(dir.join(name)).ok()?;
    let max = match name {
        "sprites.svg" => 512,
        "scene.svg" | "icon.png" => 256,
        _ => 16,
    };
    if bytes.len() > max << 10 {
        fails.push(format!(
            "{name}: {} KiB (max {max} KiB)",
            bytes.len().div_ceil(1024)
        ));
    }
    if matches!(name, "sprites.svg" | "scene.svg") {
        for why in active(&String::from_utf8_lossy(&bytes)) {
            if !fails.contains(&format!("{name}: {why}")) {
                fails.push(format!("{name}: {why}"));
            }
        }
    }
    Some(bytes)
}

fn active(svg: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = svg;
    let mut text = String::new();
    let inert = [("<!--", "-->"), ("<![CDATA[", "]]>"), ("<?", "?>")];
    while let Some((at, open, close)) = inert
        .iter()
        .filter_map(|&(o, c)| rest.find(o).map(|i| (i, o, c)))
        .min()
    {
        text.push_str(&rest[..at]);
        rest = rest[at + open.len()..]
            .split_once(close)
            .map_or("", |(_, a)| a);
    }
    text.push_str(rest);
    for tag in text.split('<').skip(1) {
        if tag.starts_with("!ENTITY") {
            found.push("an entity declaration (active content)".to_owned());
        }
        let end = tag
            .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
            .unwrap_or(tag.len());
        let (el, mut rest) = tag.split_at(end);
        match el.rsplit(':').next().unwrap_or(el) {
            "script" => found.push("<script> (active content)".to_owned()),
            "foreignObject" => found.push("<foreignObject> (active content)".to_owned()),
            _ => {}
        }
        loop {
            rest = rest.trim_start_matches(|c: char| c.is_whitespace() || c == '/');
            if rest.is_empty() || rest.starts_with('>') {
                break;
            }
            let end = rest
                .find(|c: char| c.is_whitespace() || c == '=' || c == '>' || c == '/')
                .unwrap_or(rest.len());
            let attr = rest[..end].to_ascii_lowercase();
            rest = rest[end..].trim_start();
            let mut value = "";
            if let Some(r) = rest.strip_prefix('=') {
                let r = r.trim_start();
                (value, rest) = match r.chars().next() {
                    Some(q @ ('"' | '\'')) => r[1..].split_once(q).unwrap_or((&r[1..], "")),
                    _ => r.split_at(
                        r.find(|c: char| c.is_whitespace() || c == '>')
                            .unwrap_or(r.len()),
                    ),
                };
            }
            if attr.starts_with("on") {
                found.push(format!("an on* attribute ({attr})"));
            } else if (attr == "href" || attr.ends_with(":href")) && !value.trim().starts_with('#')
            {
                found.push(format!("a non-local href {value:?} (only #id)"));
            }
        }
    }
    found
}

type Own = (Theme, BTreeMap<String, Scheme>);

fn read_theme(dir: &Path, schemes: &BTreeMap<String, Scheme>) -> Result<Option<Own>, Vec<String>> {
    let mut fails = Vec::new();
    let Some(json) = file(dir, "theme.json", &mut fails) else {
        return Ok(None);
    };
    let mut t: Theme =
        serde_json::from_slice(&json).map_err(|e| vec![format!("theme.json: {e}")])?;
    for k in t.words.keys().filter(|k| !WORDS.contains(&k.as_str())) {
        fails.push(format!("words.{k}: not one of {}", WORDS.join(" ")));
    }
    let mut own = BTreeMap::new();
    for (name, path) in entries(&dir.join("schemes"), &mut fails) {
        let Some(n) = name.strip_suffix(".json") else {
            continue;
        };
        let _ = file(dir, &format!("schemes/{name}"), &mut fails);
        match read_scheme(&path).and_then(|s| gate(&s).map(|()| s)) {
            Ok(s) if valid_name(n) => drop(own.insert(n.to_owned(), s)),
            Ok(_) => fails.push(format!("schemes/{name}: not a-z, 0-9 and -")),
            Err(e) => fails.push(format!("schemes/{name}: {e}")),
        }
    }
    let modes = [
        ("dark", Mode::Dark, &t.scheme.dark),
        ("light", Mode::Light, &t.scheme.light),
    ];
    for (mode, want, name) in modes {
        match own.get(name).or_else(|| schemes.get(name)) {
            Some(s) if s.mode != want => {
                fails.push(format!("scheme.{mode} {name:?} is not a {mode} scheme"));
            }
            Some(s) => {
                if let Err(e) = gate(s) {
                    fails.push(format!("scheme.{mode} {name:?} {e}"));
                }
            }
            None => fails.push(format!("scheme.{mode} {name:?}: no such scheme")),
        }
    }
    if t.sprites {
        let svg = file(dir, "sprites.svg", &mut fails);
        let text = String::from_utf8_lossy(svg.as_deref().unwrap_or_default());
        let missing: Vec<String> = SPRITES
            .iter()
            .filter(|s| !text.contains(&format!("id=\"s-{s}\"")))
            .map(|s| format!("s-{s}"))
            .collect();
        if svg.is_none() {
            fails.push("sprites.svg: missing (theme.json says sprites)".to_owned());
        } else if !missing.is_empty() {
            fails.push(format!("sprites.svg: no {}", missing.join(", ")));
        }
        t.sprite_sheet = svg;
    }
    let svg = file(dir, "scene.svg", &mut fails);
    let text = String::from_utf8_lossy(svg.as_deref().unwrap_or_default());
    let view = text
        .split_once("viewBox=\"")
        .and_then(|(_, v)| v.split_once('"'))
        .map(|(v, _)| v);
    let size: Vec<f64> = view
        .unwrap_or_default()
        .split_whitespace()
        .filter_map(|n| n.parse().ok())
        .collect();
    if svg.is_none() {
        if t.sprites || !t.scene.is_empty() {
            let why = if t.sprites { "sprites" } else { "scene" };
            fails.push(format!("scene.svg: missing (theme.json says {why})"));
        }
    } else if !matches!(size[..], [_, _, w, h] if w <= SCENE.0 && h <= SCENE.1) {
        fails.push(format!(
            "scene.svg: viewBox {:?}, over {} × {}",
            view.unwrap_or("missing"),
            SCENE.0,
            SCENE.1
        ));
    }
    for l in t
        .scene
        .iter()
        .filter(|l| !text.contains(&format!("id=\"{l}\"")))
    {
        fails.push(format!("scene.svg: no layer {l:?}"));
    }
    t.scene_art = svg;
    t.icon = file(dir, "icon.png", &mut fails);
    if fails.is_empty() {
        Ok(Some((t, own)))
    } else {
        Err(fails)
    }
}

fn entries(dir: &Path, refused: &mut Vec<String>) -> Vec<(String, std::path::PathBuf)> {
    let list = match fs::read_dir(dir) {
        Ok(list) => list,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(e) => {
            refused.push(format!("{}: {e}", dir.display()));
            return Vec::new();
        }
    };
    let mut out: Vec<_> = list
        .filter_map(Result::ok)
        .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path()))
        .collect();
    out.sort();
    out
}

fn read_scheme(path: &Path) -> Result<Scheme, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

#[cfg(test)]
pub(crate) mod tests;
