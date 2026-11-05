//! Themes and colour schemes on herdsman's data-only theme contract (docs/themes.md): a theme is a folder of data
//! (`theme.json`, optional `sprites.svg` and `icon.png`), a scheme one JSON file of tokens. The built-in ones ship in
//! the binary; the owner's own come from `<config dir>/themes/<name>/` and `<config dir>/schemes/<name>.json`. Every
//! scheme passes herdsman's contrast gate, or it is refused.

use std::{collections::BTreeMap, fs, path::Path};

use serde::{Deserialize, Serialize};

/// A colour scheme: its label, `dark` or `light`, and the tokens the style sheets read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scheme {
    /// The name shown in COLOURS.
    pub label: String,
    /// `dark` or `light`: the Appearance mode it serves.
    #[serde(rename = "color-scheme")]
    pub color_scheme: String,
    /// `--bg`, `--fg`, … as `#rrggbb` (the gate's [`TOKENS`], and any others the scheme adds).
    pub tokens: BTreeMap<String, String>,
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
    /// Has `sprites.svg` (symbols `s-edit s-read s-run s-idle s-done s-needs s-merged s-fail s-loading s-empty`).
    #[serde(default)]
    pub sprites: bool,
    /// Plays the row moments.
    #[serde(default)]
    pub moments: bool,
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
}

/// The plain word for a state key: what every theme says beside its own word, and all the terminal theme says.
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

    /// A push's body: the theme word, the plain word, then the agent's line (`at the gate · needs you · codex: …`);
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
}

macro_rules! builtin_theme {
    ($name:literal, $sprites:expr) => {
        (
            $name,
            include_str!(concat!("../themes/", $name, "/theme.json")),
            $sprites,
            include_bytes!(concat!("../themes/", $name, "/icon.png")).as_slice(),
        )
    };
}

macro_rules! builtin_scheme {
    ($($name:literal),*) => {
        [$(($name, include_str!(concat!("../schemes/", $name, ".json")))),*]
    };
}

/// A built-in theme: its name, `theme.json`, `sprites.svg` and `icon.png`.
type Builtin = (
    &'static str,
    &'static str,
    Option<&'static [u8]>,
    &'static [u8],
);

const THEMES: [Builtin; 4] = [
    builtin_theme!(
        "farm",
        Some(include_bytes!("../themes/farm/sprites.svg").as_slice())
    ),
    builtin_theme!(
        "kitchen",
        Some(include_bytes!("../themes/kitchen/sprites.svg").as_slice())
    ),
    builtin_theme!(
        "garden",
        Some(include_bytes!("../themes/garden/sprites.svg").as_slice())
    ),
    builtin_theme!("terminal", None),
];

const SCHEMES: [(&str, &str); 12] = builtin_scheme!(
    "github-dark",
    "solarized-dark",
    "gruvbox",
    "nord",
    "dracula",
    "farm",
    "kitchen",
    "garden-dark",
    "paper",
    "farm-light",
    "kitchen-light",
    "garden-light"
);

/// A name for a theme or scheme: lowercase letters, digits and `-`, so it is safe in a URL and a selector.
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

impl Looks {
    /// The built-in themes and schemes, then the owner's own from `config_dir` (an own one of the same name
    /// replaces a built-in). `default` is the server's default theme (else farm). Returns why each own file was
    /// refused (unreadable, invalid, failing the gate, a theme naming an unknown scheme, a missing sprite sheet).
    ///
    /// # Panics
    ///
    /// If a built-in theme or scheme doesn't parse (a build bug the tests catch).
    #[must_use]
    pub fn load(config_dir: Option<&Path>, default: Option<&str>) -> (Self, Vec<String>) {
        let mut refused = Vec::new();
        let mut schemes: BTreeMap<String, Scheme> = SCHEMES
            .iter()
            .map(|(n, json)| ((*n).to_owned(), serde_json::from_str(json).expect(n)))
            .collect();
        let mut themes: BTreeMap<String, Theme> = THEMES
            .iter()
            .map(|(n, json, sprites, icon)| {
                let mut t: Theme = serde_json::from_str(json).expect(n);
                t.sprite_sheet = sprites.map(<[u8]>::to_vec);
                t.icon = Some(icon.to_vec());
                ((*n).to_owned(), t)
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
                match read_theme(&path, &schemes) {
                    Ok(t) if valid_name(&name) => drop(themes.insert(name, t)),
                    Ok(_) => refused.push(format!("{}: not a-z, 0-9 and -", path.display())),
                    Err(e) => refused.push(format!("{}: {e}", path.display())),
                }
            }
        }
        let default = match default {
            Some(d) if themes.contains_key(d) => d.to_owned(),
            Some(d) => {
                refused.push(format!("theme = {d:?}: no such theme, showing farm"));
                "farm".to_owned()
            }
            None => "farm".to_owned(),
        };
        (
            Self {
                default,
                themes,
                schemes,
            },
            refused,
        )
    }
}

/// A folder's entries by name, sorted; a missing folder has none.
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
    let s: Scheme = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    if s.color_scheme != "dark" && s.color_scheme != "light" {
        return Err(format!(
            "color-scheme {:?}: not dark or light",
            s.color_scheme
        ));
    }
    Ok(s)
}

fn read_theme(dir: &Path, schemes: &BTreeMap<String, Scheme>) -> Result<Theme, String> {
    let text = fs::read_to_string(dir.join("theme.json")).map_err(|e| e.to_string())?;
    let mut t: Theme = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    for (mode, name) in [("dark", &t.scheme.dark), ("light", &t.scheme.light)] {
        match schemes.get(name) {
            Some(s) if s.color_scheme == mode => {}
            Some(_) => return Err(format!("scheme.{mode} {name:?} is not a {mode} scheme")),
            None => return Err(format!("scheme.{mode} {name:?}: no such scheme")),
        }
    }
    if t.sprites {
        t.sprite_sheet =
            Some(fs::read(dir.join("sprites.svg")).map_err(|e| format!("sprites.svg: {e}"))?);
    }
    t.icon = fs::read(dir.join("icon.png")).ok();
    Ok(t)
}

/// The tokens every scheme sets: the ones the gate checks.
pub const TOKENS: [&str; 19] = [
    "--bg", "--card", "--edge", "--line", "--scrim", "--fg", "--mute", "--l", "--link", "--g",
    "--a", "--r", "--p", "--c", "--fk-l", "--fk-p", "--fk-c", "--fk-g", "--fk-a",
];

type Rgb = [f64; 3];

fn hex(s: &str) -> Option<Rgb> {
    let n = u32::from_str_radix(s.strip_prefix('#')?.get(..6)?, 16).ok()?;
    Some([n >> 16, (n >> 8) & 255, n & 255].map(f64::from))
}

fn lin(c: f64) -> f64 {
    let c = c / 255.0;
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn lum(c: Rgb) -> f64 {
    0.2126 * lin(c[0]) + 0.7152 * lin(c[1]) + 0.0722 * lin(c[2])
}

/// WCAG contrast ratio.
fn ratio(a: Rgb, b: Rgb) -> f64 {
    let (x, y) = (lum(a), lum(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// `color-mix(in srgb, c p%, transparent)` painted over `bg`.
fn over(c: Rgb, bg: Rgb, p: f64) -> Rgb {
    [0, 1, 2].map(|i| c[i] * p + bg[i] * (1.0 - p))
}

fn lab(rgb: Rgb) -> Rgb {
    let lc = rgb.map(lin);
    let [cx, cy, cz] = [
        [0.4124, 0.3576, 0.1805],
        [0.2126, 0.7152, 0.0722],
        [0.0193, 0.1192, 0.9505],
    ]
    .map(|r| r[0] * lc[0] + r[1] * lc[1] + r[2] * lc[2]);
    let curve = |t: f64| {
        if t > 216.0 / 24389.0 {
            t.cbrt()
        } else {
            (24389.0 / 27.0 * t + 16.0) / 116.0
        }
    };
    let (fx, fy, fz) = (curve(cx / 0.950_47), curve(cy), curve(cz / 1.088_83));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

/// CIEDE2000 (Sharma et al. 2005), as herdsman's gate.
#[allow(clippy::many_single_char_names, reason = "the formula's own names")]
fn de(c1: Rgb, c2: Rgb) -> f64 {
    let ([l1, a1, b1], [l2, a2, b2]) = (lab(c1), lab(c2));
    let rad = std::f64::consts::PI / 180.0;
    let c = f64::midpoint(a1.hypot(b1), a2.hypot(b2));
    let g = 0.5 * (1.0 - (c.powi(7) / (c.powi(7) + 25f64.powi(7))).sqrt());
    let (ap1, ap2) = (a1 * (1.0 + g), a2 * (1.0 + g));
    let (cp1, cp2) = (ap1.hypot(b1), ap2.hypot(b2));
    let h = |b: f64, a: f64| {
        let v = b.atan2(a) / rad;
        if v < 0.0 { v + 360.0 } else { v }
    };
    let (h1, h2, dl, dc) = (h(b1, ap1), h(b2, ap2), l2 - l1, cp2 - cp1);
    let dh = if cp1 * cp2 == 0.0 {
        0.0
    } else if h2 - h1 > 180.0 {
        h2 - h1 - 360.0
    } else if h2 - h1 < -180.0 {
        h2 - h1 + 360.0
    } else {
        h2 - h1
    };
    let d_h = 2.0 * (cp1 * cp2).sqrt() * (dh * rad / 2.0).sin();
    let (lm, cm) = (f64::midpoint(l1, l2), f64::midpoint(cp1, cp2));
    let hm = if cp1 * cp2 == 0.0 {
        h1 + h2
    } else if (h1 - h2).abs() > 180.0 {
        (h1 + h2 + 360.0) / 2.0
    } else {
        f64::midpoint(h1, h2)
    };
    let t = 1.0 - 0.17 * ((hm - 30.0) * rad).cos()
        + 0.24 * (2.0 * hm * rad).cos()
        + 0.32 * ((3.0 * hm + 6.0) * rad).cos()
        - 0.2 * ((4.0 * hm - 63.0) * rad).cos();
    let sl = 1.0 + 0.015 * (lm - 50.0).powi(2) / (20.0 + (lm - 50.0).powi(2)).sqrt();
    let (sc, sh) = (1.0 + 0.045 * cm, 1.0 + 0.015 * cm * t);
    let rt = -2.0
        * (cm.powi(7) / (cm.powi(7) + 25f64.powi(7))).sqrt()
        * (60.0 * (-((hm - 275.0) / 25.0).powi(2)).exp() * rad).sin();
    ((dl / sl).powi(2) + (dc / sc).powi(2) + (d_h / sh).powi(2) + rt * (dc / sc) * (d_h / sh))
        .sqrt()
}

/// herdsman's scheme gate (serve.js `gate`, via docs/design/look/gate.mjs): text tokens at 4.5:1 and state colours
/// at 3:1 on the background and cards, the needs-you and group tints, and the state colours ΔE2000 ≥ 20 apart.
///
/// # Errors
///
/// The checks that fail, each with its value and minimum, or the tokens missing or not `#rrggbb`.
pub fn gate(s: &Scheme) -> Result<(), String> {
    let mut c = BTreeMap::new();
    for k in TOKENS {
        match s.tokens.get(k).and_then(|v| hex(v)) {
            Some(v) => drop(c.insert(k, v)),
            None => return Err(format!("{k}: missing or not #rrggbb")),
        }
    }
    let mut fails = Vec::new();
    let mut check = |what: String, v: f64, min: f64| {
        if v < min {
            fails.push(format!("{what} {v:.2} (min {min})"));
        }
    };
    let (bg, r, a) = (c["--bg"], c["--r"], c["--a"]);
    let tints = [
        ("the needs-you card", over(r, bg, 0.15)),
        ("a red group header", over(r, bg, 0.12)),
        ("an amber group header", over(a, bg, 0.12)),
    ];
    for b in ["--bg", "--card"] {
        for k in [
            "--fg", "--mute", "--l", "--link", "--fk-l", "--fk-p", "--fk-c", "--fk-g", "--fk-a",
        ] {
            check(format!("{k} on {b}"), ratio(c[k], c[b]), 4.5);
        }
        for k in ["--g", "--a", "--r"] {
            check(format!("{k} on {b}"), ratio(c[k], c[b]), 3.0);
        }
    }
    check("--bg on --l".to_owned(), ratio(bg, c["--l"]), 4.5);
    for k in ["--fg", "--mute", "--l"] {
        check(
            format!("{k} on {}", tints[0].0),
            ratio(c[k], tints[0].1),
            4.5,
        );
    }
    check(format!("--r on {}", tints[0].0), ratio(r, tints[0].1), 3.0);
    check(format!("--r on {}", tints[1].0), ratio(r, tints[1].1), 3.0);
    check(format!("--a on {}", tints[2].0), ratio(a, tints[2].1), 3.0);
    let mut pairs = vec![("--g", "--a"), ("--a", "--r"), ("--g", "--r")];
    for x in ["--l", "--link"] {
        pairs.extend(["--g", "--a", "--r"].map(|y| (x, y)));
    }
    for (x, y) in pairs {
        check(format!("ΔE2000 {x} ↔ {y}"), de(c[x], c[y]), 20.0);
    }
    if fails.is_empty() {
        Ok(())
    } else {
        Err(format!("fails the contrast gate: {}", fails.join("; ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("cooee-look-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn builtins_load_and_pass_the_gate() {
        let (looks, refused) = Looks::load(None, None);
        assert_eq!(refused, Vec::<String>::new());
        assert_eq!(looks.default, "farm");
        assert_eq!(
            looks.themes.keys().collect::<Vec<_>>(),
            ["farm", "garden", "kitchen", "terminal"]
        );
        assert_eq!(looks.schemes.len(), 12);
        for (name, s) in &looks.schemes {
            assert_eq!(gate(s), Ok(()), "{name}");
        }
        for (name, t) in &looks.themes {
            assert_eq!(looks.schemes[&t.scheme.dark].color_scheme, "dark", "{name}");
            assert_eq!(
                looks.schemes[&t.scheme.light].color_scheme, "light",
                "{name}"
            );
            assert_eq!(t.sprites, t.sprite_sheet.is_some(), "{name}");
            if let Some(svg) = &t.sprite_sheet {
                let svg = String::from_utf8_lossy(svg);
                for id in [
                    "edit", "read", "run", "idle", "done", "needs", "merged", "fail", "loading",
                    "empty",
                ] {
                    assert!(svg.contains(&format!("id=\"s-{id}\"")), "{name}: s-{id}");
                }
            }
        }
        assert_eq!(looks.themes["farm"].word("needs"), "at the gate");
        assert_eq!(looks.themes["terminal"].word("needs"), "needs you");
    }

    #[test]
    fn own_themes_and_schemes_load_and_a_failing_scheme_is_refused() {
        let dir = tmp("own");
        fs::create_dir_all(dir.join("schemes")).unwrap();
        fs::create_dir_all(dir.join("themes/sea")).unwrap();
        fs::create_dir_all(dir.join("themes/broken")).unwrap();
        let mut sea: Scheme = serde_json::from_str(SCHEMES[0].1).unwrap();
        sea.label = "Sea".to_owned();
        fs::write(
            dir.join("schemes/sea.json"),
            serde_json::to_string(&sea).unwrap(),
        )
        .unwrap();
        // Grey text on a grey background: fails 4.5:1.
        let mut murky = sea.clone();
        murky.tokens.insert("--fg".to_owned(), "#30363d".to_owned());
        fs::write(
            dir.join("schemes/murky.json"),
            serde_json::to_string(&murky).unwrap(),
        )
        .unwrap();
        fs::write(
            dir.join("themes/sea/theme.json"),
            r#"{"label":"sea","scheme":{"dark":"sea","light":"paper"},"words":{"needs":"ashore"}}"#,
        )
        .unwrap();
        fs::write(
            dir.join("themes/broken/theme.json"),
            r#"{"label":"broken","scheme":{"dark":"murky","light":"paper"}}"#,
        )
        .unwrap();
        let (looks, refused) = Looks::load(Some(&dir), Some("sea"));
        assert_eq!(looks.default, "sea");
        assert_eq!(looks.schemes["sea"].label, "Sea");
        assert!(!looks.schemes.contains_key("murky"));
        assert_eq!(looks.themes["sea"].word("needs"), "ashore");
        assert!(!looks.themes.contains_key("broken"));
        assert_eq!(refused.len(), 2, "{refused:?}");
        assert!(
            refused[0].contains("murky.json: fails the contrast gate: --fg on --bg"),
            "{refused:?}"
        );
        assert!(
            refused[1].contains("\"murky\": no such scheme"),
            "{refused:?}"
        );
    }

    #[test]
    fn gate_numbers_match_herdsmans() {
        // docs/design/look/README.md's tightest checks, from gate.mjs.
        let (looks, _) = Looks::load(None, None);
        let t = |s: &str, k: &str| hex(&looks.schemes[s].tokens[k]).unwrap();
        let d = de(t("farm-light", "--l"), t("farm-light", "--a"));
        assert!((d - 20.37).abs() < 0.01, "{d}");
        let r = ratio(t("kitchen-light", "--fk-a"), t("kitchen-light", "--card"));
        assert!((r - 4.67).abs() < 0.01, "{r}");
    }

    #[test]
    fn a_theme_with_sprites_needs_its_sheet() {
        let dir = tmp("sprites");
        fs::create_dir_all(dir.join("themes/bare")).unwrap();
        fs::write(
            dir.join("themes/bare/theme.json"),
            r#"{"label":"bare","scheme":{"dark":"nord","light":"paper"},"sprites":true}"#,
        )
        .unwrap();
        let (looks, refused) = Looks::load(Some(&dir), Some("nope"));
        assert!(!looks.themes.contains_key("bare"));
        assert_eq!(looks.default, "farm");
        assert!(refused[0].contains("sprites.svg"), "{refused:?}");
        assert!(refused[1].contains("no such theme"), "{refused:?}");
    }

    #[test]
    fn push_body_says_the_theme_word_and_the_plain_word() {
        let (looks, _) = Looks::load(None, None);
        assert_eq!(
            looks.themes["farm"].push_body("needs", "codex: Edit src/pair.rs?"),
            "at the gate · needs you · codex: Edit src/pair.rs?"
        );
        assert_eq!(
            looks.themes["kitchen"].push_body("done", "claude: Fixed."),
            "at the pass · done · claude: Fixed."
        );
        assert_eq!(
            looks.themes["terminal"].push_body("needs", "codex: Edit?"),
            "needs you · codex: Edit?"
        );
    }
}
