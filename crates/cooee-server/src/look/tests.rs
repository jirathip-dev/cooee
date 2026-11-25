use super::*;
use crate::contrast::{de, hex, ratio};
use cooee_core::swag::Swag;

pub(super) fn tmp(name: &str) -> std::path::PathBuf {
    let dir = cooee_core::scratch::owned(
        std::env::temp_dir().join(format!("cooee-look-{name}-{}", std::process::id())),
    );
    fs::remove_dir_all(&dir).ok();
    fs::create_dir_all(&dir).unwrap();
    dir
}

pub(super) fn packs() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs")
}

pub(crate) fn first_party(name: &str) -> std::path::PathBuf {
    let dir = tmp(name);
    let swag = Swag::pinned();
    for pack in swag.names() {
        fs::create_dir_all(dir.join("themes").join(pack)).unwrap();
        for (f, bytes) in swag.files(pack).unwrap() {
            fs::write(dir.join("themes").join(pack).join(f), bytes).unwrap();
        }
    }
    dir
}

fn token(s: &Scheme, k: &str) -> String {
    let all = s.tokens.all();
    all.iter()
        .find(|(n, _)| *n == k)
        .unwrap()
        .1
        .as_str()
        .to_owned()
}

fn murky_fg(s: &mut Scheme) {
    s.tokens.fg = "#30363d".to_owned().try_into().unwrap();
}

#[test]
fn a_server_with_no_installed_pack_offers_swags_four_with_their_art() {
    let (looks, refused) = Looks::load(Some(&tmp("built-in-swag")), None);
    assert_eq!(refused, Vec::<String>::new());
    assert_eq!(looks.default, "farm");
    assert_eq!(
        looks.themes.keys().collect::<Vec<_>>(),
        ["bush", "farm", "garden", "kitchen", "plain"]
    );
    for name in ["bush", "farm", "garden", "kitchen"] {
        let t = &looks.themes[name];
        assert!(t.sprites && t.sprite_sheet.is_some() && t.scene_art.is_some(), "{name}");
    }
    assert_eq!(looks.themes["bush"].scene, ["night", "clearing"]);
}

#[test]
fn only_plain_is_built_in_and_the_first_party_packs_load_installed() {
    let (looks, refused) = Looks::load(None, None);
    assert_eq!(refused, Vec::<String>::new());
    assert_eq!(looks.default, "plain");
    assert_eq!(looks.themes.keys().collect::<Vec<_>>(), ["plain"]);
    assert_eq!(looks.schemes.len(), 14);
    for (name, s) in &looks.schemes {
        assert_eq!(gate(s), Ok(()), "{name}");
    }
    let (looks, refused) = Looks::load(Some(&first_party("load-first")), None);
    assert_eq!(refused, Vec::<String>::new());
    assert_eq!(looks.default, "farm");
    assert_eq!(
        looks.themes.keys().collect::<Vec<_>>(),
        ["bush", "farm", "garden", "kitchen", "plain"]
    );
    assert!(
        looks
            .themes
            .values()
            .all(|t| t.sprites == t.sprite_sheet.is_some())
    );
    for (name, t) in &looks.themes {
        assert_eq!(t.sprites, t.scene_art.is_some(), "{name}");
    }
    assert_eq!(looks.themes["farm"].word("needs"), "at the gate");
    assert_eq!(looks.themes["plain"].word("needs"), "needs you");
    assert_eq!(Looks::load(None, Some("terminal")).0.default, "plain");
}

#[test]
fn the_theme_gate_passes_the_first_party_packs_and_the_starter() {
    let mut dirs: Vec<_> = fs::read_dir(first_party("gate-first").join("themes"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    dirs.push(packs().join("starter"));
    assert_eq!(dirs.len(), 5, "{dirs:?}");
    for dir in dirs {
        assert_eq!(check(&dir), Ok("theme"), "{}", dir.display());
    }
}

pub(super) fn farm_with(name: &str, edit: impl FnOnce(&Path)) -> std::path::PathBuf {
    let dir = first_party(name).join("themes/farm");
    edit(&dir);
    dir
}

pub(super) fn set(dir: &Path, key: &str, value: serde_json::Value) {
    let path = dir.join("theme.json");
    let mut t: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    t[key] = value;
    fs::write(path, t.to_string()).unwrap();
}

#[test]
fn the_theme_gate_fails_a_missing_state_a_missing_scene_and_low_contrast() {
    let fails = |dir: &Path| check(dir).unwrap_err().join("\n");
    let gone = farm_with("gate-state", |d| {
        let svg = fs::read_to_string(d.join("sprites.svg")).unwrap();
        fs::write(
            d.join("sprites.svg"),
            svg.replace("id=\"s-needs\"", "id=\"x\""),
        )
        .unwrap();
    });
    assert_eq!(fails(&gone), "sprites.svg: no s-needs");
    let bare = farm_with("gate-bare", |d| {
        fs::remove_file(d.join("scene.svg")).unwrap();
    });
    assert_eq!(fails(&bare), "scene.svg: missing (theme.json says sprites)");
    let murky = farm_with("gate-contrast", |d| {
        let mut s = builtin_schemes()["github-dark"].clone();
        murky_fg(&mut s);
        fs::create_dir_all(d.join("schemes")).unwrap();
        fs::write(
            d.join("schemes/murky.json"),
            serde_json::to_string(&s).unwrap(),
        )
        .unwrap();
        set(
            d,
            "scheme",
            serde_json::json!({"dark": "murky", "light": "paper"}),
        );
    });
    let got = fails(&murky);
    assert!(
        got.starts_with("schemes/murky.json: fails the contrast gate: --fg on --bg 1."),
        "{got}"
    );
    assert!(
        got.ends_with("scheme.dark \"murky\": no such scheme"),
        "{got}"
    );
}

#[test]
fn the_theme_gate_fails_an_oversize_scene_a_missing_layer_and_unknown_words() {
    let fails = |dir: &Path| check(dir).unwrap_err().join("\n");
    let scene = |view: &str| {
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{view}\"><g id=\"sky\"/></svg>"
        )
    };
    let big = farm_with("gate-scene", |d| {
        set(d, "scene", serde_json::json!(["sky", "ground"]));
        fs::write(d.join("scene.svg"), scene("0 0 800 600")).unwrap();
    });
    assert_eq!(
        fails(&big),
        "scene.svg: viewBox \"0 0 800 600\", over 400 × 320\nscene.svg: no layer \"ground\""
    );
    fs::write(
        big.join("scene.svg"),
        scene("0 0 390 320").replace("/>", "/><g id=\"ground\"/>"),
    )
    .unwrap();
    assert_eq!(check(&big), Ok("theme"));
    let odd = farm_with("gate-words", |d| {
        set(d, "words", serde_json::json!({"needs": "x", "horses": "y"}));
        fs::write(d.join("icon.png"), vec![0; 300 << 10]).unwrap();
    });
    assert_eq!(
        fails(&odd),
        format!(
            "words.horses: not one of {}\nicon.png: 300 KiB (max 256 KiB)",
            WORDS.join(" ")
        )
    );
}

#[test]
fn own_themes_and_schemes_load_and_a_failing_scheme_is_refused() {
    let dir = tmp("own");
    fs::create_dir_all(dir.join("schemes")).unwrap();
    fs::create_dir_all(dir.join("themes/sea")).unwrap();
    fs::create_dir_all(dir.join("themes/broken")).unwrap();
    let mut sea = builtin_schemes()["github-dark"].clone();
    sea.label = "Sea".to_owned();
    fs::write(
        dir.join("schemes/sea.json"),
        serde_json::to_string(&sea).unwrap(),
    )
    .unwrap();
    let mut murky = sea.clone();
    murky_fg(&mut murky);
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
    let (looks, _) = Looks::load(None, None);
    let t = |s: &str, k: &str| hex(&token(&looks.schemes[s], k)).unwrap();
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
    assert_eq!(looks.default, "plain");
    assert!(refused[0].contains("sprites.svg"), "{refused:?}");
    assert!(refused[1].contains("no such theme"), "{refused:?}");
}

#[test]
fn push_body_says_the_theme_word_and_the_plain_word() {
    let (looks, _) = Looks::load(Some(&first_party("push-body")), None);
    assert_eq!(
        looks.themes["farm"].push_body("needs", "codex: Edit src/pair.rs?"),
        "at the gate · needs you · codex: Edit src/pair.rs?"
    );
    assert_eq!(
        looks.themes["kitchen"].push_body("done", "claude: Fixed."),
        "at the pass · done · claude: Fixed."
    );
    assert_eq!(
        looks.themes["plain"].push_body("needs", "codex: Edit?"),
        "needs you · codex: Edit?"
    );
}
