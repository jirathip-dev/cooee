use std::{collections::BTreeMap, fs, path::Path};

use serde::{Deserialize, Serialize};

use super::{Own, Scheme, Theme, builtin_schemes, file, read_theme};
use crate::contrast::{hex, ratio};

/// A style: the tokens the native app's renderer reads (docs/styles.md). Data only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Style {
    format: u32,
    label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    icon: Option<Icon>,
    #[serde(rename = "type")]
    design: TypeDesign,
    radius: f64,
    surface: Surface,
    fills: Fills,
    marks: Marks,
    figures: Figures,
    sections: Sections,
    #[serde(default)]
    labels: BTreeMap<LabelRole, Label>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stage: Option<Stage>,
}

/// A theme's own drawing of its Scene's host load.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoadScene {
    /// How it draws the load.
    pub kind: LoadKind,
}

/// The kinds of load drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoadKind {
    /// A Fire Danger Rating sign: `sprites.svg`'s `s-sign-moderate s-sign-high s-sign-catastrophic` (calm, busy, held).
    Sign,
}

const SIGN: [&str; 3] = ["s-sign-moderate", "s-sign-high", "s-sign-catastrophic"];

macro_rules! tokens {
    ($($name:ident { $($v:ident),* })*) => {$(
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
        #[serde(rename_all = "kebab-case")]
        enum $name { #[default] $($v),* }
    )*};
}

tokens! {
    Icon { Modern, Terminal, Farm, Garden, Kitchen, Bush }
    TypeDesign { Default, Mono }
    Surface { Glass, Dashed, Solid }
    Fills { Tint, Outline }
    Marks { Symbols, Text }
    Figures { Avatar, Ascii, Sprite }
    Sections { Title, Quiet }
    LabelRole { Kind, Header, Group, Section, Title, Button, Chip, Back, Said, Picked }
    LabelCase { AsIs, Lower, Upper }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Label {
    #[serde(default)]
    prefix: String,
    #[serde(default)]
    suffix: String,
    #[serde(default)]
    case: LabelCase,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct Stage {
    ink: String,
    paper: String,
    edge: String,
    alert: String,
    alert_paper: String,
}

/// What a pack folder holds once it passes the gate.
pub struct Pack {
    /// Its `theme.json` and art.
    pub theme: Option<Theme>,
    /// Its own schemes.
    pub schemes: BTreeMap<String, Scheme>,
    /// Its `style.json`.
    pub style: Option<Style>,
}

/// The first-party style packs (docs/styles.md), in the binary: name and `style.json`.
pub const STYLES: [(&str, &str); 2] = [
    ("scene", include_str!("../../../../packs/scene/style.json")),
    (
        "terminal",
        include_str!("../../../../packs/terminal/style.json"),
    ),
];

/// The theme gate (`cooee theme check`, CI, and every load): the pack in `dir` with its theme, its own schemes and
/// its style, or every way it fails: no symlinks, `theme.json` or `style.json` or both, and each file's own rules.
/// Its theme may name its own schemes or one of `schemes`.
///
/// # Errors
///
/// Every check that fails, one line each.
pub fn read_pack(dir: &Path, schemes: &BTreeMap<String, Scheme>) -> Result<Pack, Vec<String>> {
    let mut fails = Vec::new();
    links(dir, "", &mut fails);
    let theme = read_theme(dir, schemes).unwrap_or_else(|e| {
        fails.extend(e);
        None
    });
    fails.extend(theme.as_ref().and_then(|(t, _)| signs(t)));
    let style = read_style(dir, theme.as_ref()).unwrap_or_else(|e| {
        fails.extend(e);
        None
    });
    if theme.is_none() && style.is_none() && fails.is_empty() {
        fails.push("no theme.json or style.json".to_owned());
    }
    if !fails.is_empty() {
        return Err(fails);
    }
    let (theme, schemes) = theme.unzip();
    Ok(Pack {
        theme,
        schemes: schemes.unwrap_or_default(),
        style,
    })
}

fn signs(t: &Theme) -> Option<String> {
    t.load_scene?;
    let sheet = String::from_utf8_lossy(t.sprite_sheet.as_deref().unwrap_or_default());
    let missing: Vec<&str> = SIGN
        .into_iter()
        .filter(|s| !sheet.contains(&format!("id=\"{s}\"")))
        .collect();
    (!missing.is_empty())
        .then(|| format!("load_scene sign: sprites.svg has no {}", missing.join(", ")))
}

fn read_style(dir: &Path, theme: Option<&Own>) -> Result<Option<Style>, Vec<String>> {
    let mut fails = Vec::new();
    let Some(json) = file(dir, "style.json", &mut fails) else {
        return Ok(None);
    };
    let s: Style = serde_json::from_slice(&json).map_err(|e| vec![format!("style.json: {e}")])?;
    let mut why = |w: String| fails.push(format!("style.json: {w}"));
    if s.format != 1 {
        why(format!("format {}: only 1", s.format));
    }
    if !(1..=24).contains(&s.label.chars().count()) || s.label.chars().any(char::is_control) {
        why(format!(
            "label {:?}: 1 to 24 characters, no control character",
            s.label
        ));
    }
    if !(0.0..=1.0).contains(&s.radius) {
        why(format!("radius {}: not 0 to 1", s.radius));
    }
    let affix = |a: &str| {
        a.chars().count() <= 4
            && a.chars()
                .all(|c| c.is_ascii_punctuation() || " ‹›·•".contains(c))
    };
    for (role, l) in &s.labels {
        for (k, a) in [("prefix", &l.prefix), ("suffix", &l.suffix)] {
            if !affix(a) {
                let role = format!("{role:?}").to_lowercase();
                why(format!(
                    "labels.{role}.{k} {a:?}: punctuation and spaces only, at most 4"
                ));
            }
        }
    }
    if let Some(st) = &s.stage {
        let rgb = |v: &str| hex(v).filter(|_| v.len() == 7);
        let named = [("ink", &st.ink), ("paper", &st.paper), ("edge", &st.edge)];
        let named = named
            .into_iter()
            .chain([("alert", &st.alert), ("alert-paper", &st.alert_paper)]);
        for (k, v) in named.filter(|(_, v)| rgb(v).is_none()) {
            why(format!("stage.{k} {v:?}: not #rrggbb"));
        }
        if let (Some(ink), Some(paper), Some(alert), Some(pink)) = (
            rgb(&st.ink),
            rgb(&st.paper),
            rgb(&st.alert),
            rgb(&st.alert_paper),
        ) {
            let pairs = [
                ("ink on paper", ink, paper, 4.5),
                ("alert on paper", alert, paper, 3.0),
            ];
            for (what, a, b, min) in
                pairs
                    .into_iter()
                    .chain([("ink on alert-paper", ink, pink, 3.0)])
            {
                let r = ratio(a, b);
                if r < min {
                    why(format!("stage: {what} {r:.2} (min {min})"));
                }
            }
        }
    }
    if dir.file_name().is_some_and(|n| n == "modern") {
        why("a pack named modern can't hold a style: Modern is built in".to_owned());
    }
    if let Some((t, _)) = theme {
        if s.figures == Figures::Sprite && !t.sprites {
            why("figures sprite: theme.json has no sprites".to_owned());
        }
        if s.stage.is_some() && t.scene_art.is_none() {
            why("stage: the pack has no scene.svg".to_owned());
        }
    }
    if fails.is_empty() {
        Ok(Some(s))
    } else {
        Err(fails)
    }
}

/// The theme gate on the pack in `dir`, with the built-in schemes: what `cooee theme check` and CI run; what it holds.
///
/// # Errors
///
/// Every check that fails.
pub fn check(dir: &Path) -> Result<&'static str, Vec<String>> {
    let p = read_pack(dir, &builtin_schemes())?;
    Ok(match (p.theme, p.style) {
        (Some(_), Some(_)) => "theme, style",
        (Some(_), None) => "theme",
        _ => "style",
    })
}

fn links(dir: &Path, at: &str, fails: &mut Vec<String>) {
    for e in fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
    {
        let name = format!("{at}{}", e.file_name().to_string_lossy());
        match e.file_type() {
            Ok(t) if t.is_symlink() => {
                fails.push(format!("{name}: a symlink (a pack holds files only)"));
            }
            Ok(t) if t.is_dir() && name != ".git" => links(&e.path(), &format!("{name}/"), fails),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{farm_with, first_party, packs, set, tmp};
    use super::*;
    use crate::look::Looks;

    fn style_pack(name: &str, json: &[u8]) -> std::path::PathBuf {
        let dir = tmp(name).join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("style.json"), json).unwrap();
        dir
    }

    #[test]
    fn a_load_scene_sign_needs_its_three_levels_and_a_theme_without_one_serves_as_before() {
        let theme = |dir: &Path| read_theme(dir, &builtin_schemes()).unwrap().unwrap().0;
        let bush = theme(&first_party("sign-bush").join("themes/bush"));
        let sign = Some(LoadScene {
            kind: LoadKind::Sign,
        });
        assert_eq!(bush.load_scene, sign);
        let served = serde_json::to_value(&bush).unwrap();
        assert_eq!(served["load_scene"], serde_json::json!({"kind": "sign"}));
        let farm = farm_with("sign-none", |_| {});
        let served = serde_json::to_value(theme(&farm)).unwrap();
        assert_eq!(served.get("load_scene"), None);
        set(&farm, "load_scene", serde_json::json!({"kind": "sign"}));
        assert_eq!(
            check(&farm).unwrap_err(),
            [
                "load_scene sign: sprites.svg has no s-sign-moderate, s-sign-high, s-sign-catastrophic"
            ]
        );
        set(&farm, "load_scene", serde_json::json!({"kind": "weather"}));
        let got = check(&farm).unwrap_err().join("\n");
        assert!(
            got.starts_with("theme.json: unknown variant `weather`"),
            "{got}"
        );
    }
    #[test]
    fn the_gate_passes_both_style_packs_and_each_fixture_as_its_name_says() {
        for (name, json) in STYLES {
            assert_eq!(check(&packs().join(name)), Ok("style"), "{name}");
            assert_eq!(
                fs::read_to_string(packs().join(name).join("style.json")).unwrap(),
                json
            );
        }
        let fixtures: Vec<_> = fs::read_dir(packs().join("fixtures/styles"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        assert!(fixtures.len() >= 10, "{fixtures:?}");
        for f in fixtures {
            let stem = f.file_stem().unwrap().to_str().unwrap().to_owned();
            let got = check(&style_pack(&format!("fx-{stem}"), &fs::read(&f).unwrap()));
            if stem.starts_with("ok-") {
                assert_eq!(got, Ok("style"), "{stem}");
                continue;
            }
            let why = got.unwrap_err().join("\n");
            assert!(why.starts_with("style.json: "), "{stem}: {why}");
            for word in stem.trim_start_matches("bad-").split('-') {
                assert!(why.contains(word), "{stem}: {why}");
            }
        }
    }

    #[test]
    fn a_style_is_checked_against_its_own_theme_and_name() {
        let scene = fs::read(packs().join("scene/style.json")).unwrap();
        let fails = |dir: &Path| check(dir).unwrap_err().join("\n");
        let modern = style_pack("modern", &scene);
        assert_eq!(
            fails(&modern),
            "style.json: a pack named modern can't hold a style: Modern is built in"
        );
        let both = farm_with("both", |d| fs::write(d.join("style.json"), &scene).unwrap());
        assert_eq!(check(&both), Ok("theme, style"));
        set(&both, "sprites", serde_json::json!(false));
        fs::remove_file(both.join("scene.svg")).unwrap();
        assert_eq!(
            fails(&both),
            "style.json: figures sprite: theme.json has no sprites\nstyle.json: stage: the pack has no scene.svg"
        );
        assert_eq!(fails(&tmp("empty")), "no theme.json or style.json");
    }

    #[test]
    fn the_apple_tests_schemes_are_the_ones_served() {
        let (looks, _) = Looks::load(None, None);
        let fixture = include_str!("../../../../apple/Tests/schemes.json");
        let fixture: serde_json::Value = serde_json::from_str(fixture).unwrap();
        assert_eq!(serde_json::to_value(&looks.schemes).unwrap(), fixture);
    }

    #[test]
    fn a_broken_style_is_reported_and_leaves_its_packs_theme_served() {
        let dir = tmp("broken-style");
        let farm = farm_with("broken-style-farm", |d| {
            fs::write(d.join("style.json"), br#"{"format":1,"surface":"blur"}"#).unwrap();
        });
        fs::create_dir_all(dir.join("themes")).unwrap();
        fs::rename(&farm, dir.join("themes/farm")).unwrap();
        fs::create_dir_all(dir.join("themes/terminal")).unwrap();
        fs::copy(
            packs().join("terminal/style.json"),
            dir.join("themes/terminal/style.json"),
        )
        .unwrap();
        let (looks, refused) = Looks::load(Some(&dir), None);
        assert!(looks.themes.contains_key("farm"));
        assert_eq!(looks.styles.keys().collect::<Vec<_>>(), ["terminal"]);
        assert!(!looks.themes.contains_key("terminal"));
        let why = &looks.broken["farm"];
        assert!(
            why.starts_with("style.json: unknown variant `blur`"),
            "{why}"
        );
        assert_eq!(refused.len(), 1, "{refused:?}");
        let wire = serde_json::to_value(&looks).unwrap();
        let sent: Style = serde_json::from_value(wire["styles"]["terminal"].clone()).unwrap();
        assert_eq!(sent, serde_json::from_str::<Style>(STYLES[1].1).unwrap());
        assert_eq!(wire["broken"]["farm"], why.as_str());
    }
}
