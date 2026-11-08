//! What a device shows (#49): its pick from the `cooee-look` cookie (`theme:kitchen/style:terminal/…`; unset keys
//! are the defaults) resolved against the looks (#35: Style, Theme, Figures and Colours), so the server draws the theme and scheme on the first byte. The
//! marks (`[data-st]`, `[data-w]`, `[data-s]`) are drawn here as `look.ts` draws the ones a page adds later.

use std::{collections::BTreeMap, fmt::Write as _, sync::Arc};

use crate::look::{ASCII, Looks, Scheme, Theme, plain};

/// The cookie that keeps a device's pick.
pub const COOKIE: &str = "cooee-look";

/// The keys a pick has (`scene:on` is Style Scene over the `style` it falls back to).
const KEYS: [&str; 7] = [
    "theme", "mode", "style", "scene", "figures", "dark", "light",
];

/// The state keys with a word (a key without one shows nothing).
const STATES: [&str; 9] = [
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

/// A device's pick against the looks.
pub struct Shown {
    looks: Arc<Looks>,
    pick: BTreeMap<String, String>,
}

/// `&`, `<`, `>`, `"` and `'` as character references.
pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' | '<' | '>' | '"' | '\'' => {
                let _ = write!(out, "&#{};", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out
}

/// A state's ASCII figure, its two frames in `<b>`s (`cls`: `rsa` in a row, `lsa` the big one).
fn ascii(id: &str, cls: &str) -> String {
    let frames = ASCII
        .iter()
        .find(|(k, _)| *k == id)
        .map_or(["", ""], |f| f.1);
    format!(
        "<i class=\"{cls}\" aria-hidden=\"true\"><b>{}</b><b>{}</b></i>",
        esc(frames[0]),
        esc(frames[1])
    )
}

/// A token as a CSS declaration, when it can't break out of the rule.
fn declaration(k: &str, v: &str) -> Option<String> {
    let name = k.strip_prefix("--")?;
    let safe = |s: &str, more: &str| {
        s.chars()
            .all(|c| c.is_ascii_alphanumeric() || more.contains(c))
    };
    (safe(name, "-_") && safe(v, "#-_.,()% ")).then(|| format!("{k}:{v};"))
}

impl Shown {
    /// The pick in `cookie` (the `Cookie` header's value, or none).
    #[must_use]
    pub fn new(looks: Arc<Looks>, cookie: Option<&str>) -> Self {
        let value = cookie
            .into_iter()
            .flat_map(|c| c.split(';'))
            .find_map(|c| c.trim().strip_prefix(COOKIE)?.strip_prefix('='))
            .unwrap_or_default();
        let pick = value
            .split('/')
            .filter_map(|kv| kv.split_once(':'))
            .map(|(k, v)| match (k, v) {
                // The Sprites switch before #35.
                ("sprites", "on") => ("figures", "sprite"),
                ("sprites", "off") => ("figures", "none"),
                ("sprites", v) => ("figures", v),
                ("theme", t) => ("theme", Looks::renamed(&looks.themes, t)),
                kv => kv,
            })
            .filter(|(k, _)| KEYS.contains(k))
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        Self { looks, pick }
    }

    /// The looks it picks from.
    #[must_use]
    pub fn looks(&self) -> &Looks {
        &self.looks
    }

    /// The same pick showing theme `dev.<name>`, being built in `themes-dev/`, when it passes the gate.
    #[must_use]
    pub fn preview(mut self, name: &str) -> Self {
        self.pick.insert("theme".to_owned(), format!("dev.{name}"));
        self
    }

    fn get(&self, k: &str) -> Option<&str> {
        self.pick.get(k).map(String::as_str)
    }

    /// The theme's name: the pick's, else the server's default.
    #[must_use]
    pub fn name(&self) -> &str {
        match self.get("theme") {
            Some(t) if self.looks.themes.contains_key(t) => t,
            _ => &self.looks.default,
        }
    }

    /// The theme.
    #[must_use]
    pub fn theme(&self) -> &Theme {
        &self.looks.themes[self.name()]
    }

    /// The mode the pick forces (`dark` or `light`), or none: System, which the page follows.
    #[must_use]
    pub fn mode(&self) -> Option<&str> {
        self.get("mode").filter(|m| matches!(*m, "dark" | "light"))
    }

    /// The scheme for mode `m`: the pick's (a scheme of that mode), else the theme's.
    #[must_use]
    pub fn scheme(&self, m: &str) -> &str {
        let theme = &self.theme().scheme;
        match self.get(m) {
            Some(s)
                if self
                    .looks
                    .schemes
                    .get(s)
                    .is_some_and(|s| s.color_scheme == m) =>
            {
                s
            }
            _ if m == "light" => &theme.light,
            _ => &theme.dark,
        }
    }

    /// The scheme by name.
    #[must_use]
    pub fn scheme_of(&self, name: &str) -> &Scheme {
        &self.looks.schemes[name]
    }

    /// `modern`, `terminal` or `scene` (with sprites only; else the Modern or Terminal pick it falls back to).
    #[must_use]
    pub fn style(&self) -> &str {
        if self.get("scene") == Some("on") && self.figures() == "sprite" {
            "scene"
        } else if self.get("style") == Some("terminal") {
            "terminal"
        } else {
            "modern"
        }
    }

    /// The shape pages take: Modern or Terminal; in Scene (#34) only the thread list is the scene, the rest keeps the
    /// Modern or Terminal pick it falls back to.
    #[must_use]
    pub fn shape(&self) -> &str {
        if self.get("style") == Some("terminal") {
            "terminal"
        } else {
            "modern"
        }
    }

    /// The thread list draws as the theme's scene (#34): Style Scene, offered with sprites only.
    #[must_use]
    pub fn scene(&self) -> bool {
        self.style() == "scene"
    }

    /// The styles offered: Scene only with sprites.
    #[must_use]
    pub fn styles(&self) -> Vec<&'static str> {
        let n = if self.figures() == "sprite" { 3 } else { 2 };
        ["modern", "terminal", "scene"][..n].to_vec()
    }

    /// `sprite`, `ascii` or `none`: the pick's, else `sprite`; a theme without sprites (plain) has `ascii` instead.
    #[must_use]
    pub fn figures(&self) -> &str {
        match self.get("figures") {
            Some(f @ ("ascii" | "none")) => f,
            _ if self.theme().sprites => "sprite",
            _ => "ascii",
        }
    }

    /// The figures offered: a theme without sprites has no `sprite`.
    #[must_use]
    pub fn figure_list(&self) -> Vec<&'static str> {
        ["sprite", "ascii", "none"][usize::from(!self.theme().sprites)..].to_vec()
    }

    /// The pick's Appearance: `system`, `dark` or `light`.
    #[must_use]
    pub fn appearance(&self) -> &str {
        self.mode().unwrap_or("system")
    }

    /// The scheme's tokens on `:root`; with System, the light scheme's under `prefers-color-scheme: light`.
    #[must_use]
    pub fn css(&self) -> String {
        let rule = |m: &str| {
            let s = self.scheme_of(self.scheme(m));
            let tokens: String = s
                .tokens
                .iter()
                .filter_map(|(k, v)| declaration(k, v))
                .collect();
            format!(":root{{{tokens}color-scheme:{}}}", s.color_scheme)
        };
        match self.mode() {
            Some(m) => rule(m),
            None => format!(
                "{}@media (prefers-color-scheme:light){{{}}}",
                rule("dark"),
                rule("light")
            ),
        }
    }

    fn svg(&self, id: &str, cls: &str) -> String {
        let bx = if cls == "big" || id == "loading" {
            "0 0 132 100"
        } else {
            "0 0 24 24"
        };
        format!(
            "<svg class=\"{cls}\" viewBox=\"{bx}\" aria-hidden=\"true\"><use href=\"/look/{}/sprites.svg#s-{id}\"></use></svg>",
            self.name()
        )
    }

    fn word<'b>(&'b self, k: &'b str) -> &'b str {
        self.theme().word(k)
    }

    fn plain_after(k: &str, said: &str, with: bool) -> String {
        if with && said != plain(k) {
            format!("<small class=\"pw\"> · {}</small>", plain(k))
        } else {
            String::new()
        }
    }

    /// A state's mark (`[data-st]`): the sprite or ascii figure, then the theme word (`plain`: and `· plain word`);
    /// with no figures, a dot and the plain word. `loading` (a thread connecting, #48) is the theme's loading figure
    /// at line height and its loading word, else `connecting`.
    #[must_use]
    pub fn st(&self, st: &str, plain: bool) -> String {
        if !STATES.contains(&st) {
            return String::new();
        }
        let id = if st == "working" { "edit" } else { st };
        let loading = st == "loading";
        let word = match self.theme().words.get(st) {
            None if loading => "connecting",
            _ => self.word(st),
        };
        let (mark, said) = match self.figures() {
            "none" if loading => ("<i class=\"sd\"></i>".to_owned(), "connecting"),
            "none" => ("<i class=\"sd\"></i>".to_owned(), crate::look::plain(st)),
            "ascii" if loading => (format!("{} ", ascii("read", "rsa")), word),
            "ascii" => (format!("{} ", ascii(id, "rsa")), word),
            _ if loading => (self.svg(id, "rsp ld"), word),
            _ => (self.svg(id, "rsp"), word),
        };
        format!(
            "{mark}<span class=\"thw\">{}</span>{}",
            esc(said),
            Self::plain_after(st, said, plain)
        )
    }

    /// A theme word (`[data-w]`).
    #[must_use]
    pub fn w(&self, k: &str, plain: bool) -> String {
        let said = self.word(k);
        format!("{}{}", esc(said), Self::plain_after(k, said, plain))
    }

    /// The theme's 132 × 100 figure (`[data-s]`).
    #[must_use]
    pub fn s(&self, k: &str) -> String {
        match self.figures() {
            "ascii" => ascii(k, "lsa"),
            "sprite" => self.svg(k, "big"),
            _ => String::new(),
        }
    }

    /// The cookie's value after a tap on key `k`, value `v` (`mode` `system`: none). A scheme that is the theme's
    /// default forgets the pick; one of the other mode's list switches a forced Appearance to that mode.
    #[must_use]
    pub fn with(&self, k: &str, v: &str) -> String {
        let mut pick = self.pick.clone();
        if k == "dark" || k == "light" {
            if self.theme().scheme_for(k) == v {
                pick.remove(k);
            } else {
                pick.insert(k.to_owned(), v.to_owned());
            }
            if self.mode().is_some_and(|m| m != k) {
                pick.insert("mode".to_owned(), k.to_owned());
            }
        } else if k == "style" {
            // Scene keeps the Modern or Terminal pick to fall back to.
            if v == "scene" {
                pick.insert("scene".to_owned(), "on".to_owned());
            } else {
                pick.remove("scene");
                pick.insert(k.to_owned(), v.to_owned());
            }
        } else if KEYS.contains(&k) && !(k == "mode" && v == "system") {
            pick.insert(k.to_owned(), v.to_owned());
        } else {
            pick.remove(k);
        }
        let ok = |s: &str| s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-');
        let kept: Vec<String> = pick
            .iter()
            .filter(|(_, v)| ok(v))
            .map(|(k, v)| format!("{k}:{v}"))
            .collect();
        kept.join("/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The built-in looks and the first-party packs, as installed.
    fn looks() -> Arc<Looks> {
        let packs = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../packs");
        Arc::new(Looks::load(Some(&packs), None).0)
    }

    #[test]
    fn the_cookie_picks_the_theme_scheme_style_and_figures() {
        let looks = looks();
        let cookie = "cooee=abc; cooee-look=theme:kitchen/style:terminal/figures:none/mode:dark";
        let s = Shown::new(looks.clone(), Some(cookie));
        assert_eq!(
            (s.name(), s.style(), s.figures(), s.mode()),
            ("kitchen", "terminal", "none", Some("dark"))
        );
        assert!(s.css().starts_with(":root{"), "{}", s.css());
        assert!(s.css().contains("--bg:#1a1816;"), "{}", s.css());
        assert!(!s.css().contains("@media"));
        // Figures none: a dot and the plain word, never the theme's.
        assert_eq!(
            s.st("needs", true),
            "<i class=\"sd\"></i><span class=\"thw\">needs you</span>"
        );
        assert_eq!(s.s("empty"), "");
        // No pick: the default theme, System with both schemes.
        let d = Shown::new(looks.clone(), None);
        assert_eq!(
            (d.name(), d.style(), d.figures(), d.mode()),
            ("farm", "modern", "sprite", None)
        );
        assert!(
            d.css()
                .contains("@media (prefers-color-scheme:light){:root{"),
            "{}",
            d.css()
        );
        assert!(d.st("needs", true).starts_with("<svg class=\"rsp\""));
        assert!(
            d.st("needs", true)
                .ends_with("<small class=\"pw\"> · needs you</small>")
        );
        assert_eq!(d.st("", false), "");
        // A thread connecting (#48): the loading figure at line height and the theme's loading word, else
        // `connecting`.
        assert!(
            d.st("loading", false).starts_with(
                "<svg class=\"rsp ld\" viewBox=\"0 0 132 100\" aria-hidden=\"true\"><use href=\"/look/farm/sprites.svg#s-loading\">"
            ) && d.st("loading", false).ends_with("<span class=\"thw\">saddling up</span>"),
            "{}",
            d.st("loading", false)
        );
        let t = Shown::new(looks.clone(), Some("cooee-look=theme:plain/figures:ascii"));
        assert_eq!(
            t.st("loading", false),
            "<i class=\"rsa\" aria-hidden=\"true\"><b>o.o</b><b>o.-</b></i> <span class=\"thw\">connecting</span>"
        );
    }

    #[test]
    fn a_bad_pick_falls_back_and_cant_break_out() {
        let looks = looks();
        let s = Shown::new(
            looks.clone(),
            Some("cooee-look=theme:nope/dark:paper/sprites:x"),
        );
        assert_eq!(s.name(), "farm");
        // A light scheme isn't a dark pick.
        assert_eq!(s.scheme("dark"), "farm");
        assert_eq!(s.figures(), "sprite");
        assert_eq!(declaration("--x", "red;}</style>"), None);
        assert_eq!(
            esc("<a href=\"x\">'&"),
            "&#60;a href=&#34;x&#34;&#62;&#39;&#38;"
        );
    }

    #[test]
    fn each_figures_value_draws_a_state_and_the_big_figure() {
        let looks = looks();
        let on = |pick: &str| {
            Shown::new(looks.clone(), Some(&format!("cooee-look={pick}"))).st("working", true)
        };
        assert!(on("").starts_with("<svg class=\"rsp\""), "{}", on(""));
        assert!(on("").contains("/look/farm/sprites.svg#s-edit"));
        assert_eq!(
            on("figures:ascii"),
            "<i class=\"rsa\" aria-hidden=\"true\"><b>~o=</b><b>=o~</b></i> <span class=\"thw\">on the trail</span>\
             <small class=\"pw\"> · working</small>"
        );
        assert_eq!(
            on("figures:none"),
            "<i class=\"sd\"></i><span class=\"thw\">working</span>"
        );
        let big = Shown::new(looks.clone(), Some("cooee-look=figures:ascii")).s("loading");
        assert_eq!(
            big,
            "<i class=\"lsa\" aria-hidden=\"true\"><b>( o.o )\n  ...</b><b>( o.O )\n  .. </b></i>"
        );
        // Every figure has two frames that differ, one per state a page names.
        for (id, [a, b]) in ASCII {
            assert!(!a.is_empty() && a != b, "{id}");
        }
    }

    #[test]
    fn terminal_is_plain_and_scene_needs_sprites() {
        let looks = looks();
        let s = |pick: &str| Shown::new(looks.clone(), Some(&format!("cooee-look={pick}")));
        // A saved `terminal` theme (and the Sprites switch) before #35.
        let old = s("theme:terminal/sprites:off");
        assert_eq!((old.name(), old.figures()), ("plain", "none"));
        assert_eq!(
            old.with("mode", "dark"),
            "figures:none/mode:dark/theme:plain"
        );
        assert_eq!(s("sprites:on").figures(), "sprite");
        // plain has no sprites: ASCII by default, Figures offers ASCII or None, and no Scene.
        let plain = s("theme:plain/figures:sprite/scene:on");
        assert_eq!((plain.figures(), plain.style()), ("ascii", "modern"));
        assert_eq!(plain.figure_list(), ["ascii", "none"]);
        assert_eq!(plain.styles(), ["modern", "terminal"]);
        assert_eq!(s("").figure_list(), ["sprite", "ascii", "none"]);
        // Scene with sprites; ASCII or None hides it and falls back to the Modern or Terminal pick.
        let scene = s("style:terminal");
        assert_eq!(scene.styles(), ["modern", "terminal", "scene"]);
        let tapped = scene.with("style", "scene");
        assert_eq!(tapped, "scene:on/style:terminal");
        assert_eq!(
            (s(&tapped).style(), s(&tapped).scene(), s(&tapped).shape()),
            ("scene", true, "terminal")
        );
        for f in ["ascii", "none"] {
            let off = s(&format!("{tapped}/figures:{f}"));
            assert_eq!(
                (off.style(), off.styles()),
                ("terminal", vec!["modern", "terminal"])
            );
        }
        assert_eq!(s(&tapped).with("style", "modern"), "style:modern");
    }

    #[test]
    fn a_tap_gives_the_next_cookie() {
        let looks = looks();
        let s = Shown::new(looks.clone(), Some("cooee-look=theme:kitchen/mode:light"));
        assert_eq!(
            s.with("style", "terminal"),
            "mode:light/style:terminal/theme:kitchen"
        );
        // The other mode's list switches the forced mode; the theme's default forgets the pick.
        assert_eq!(s.with("dark", "nord"), "dark:nord/mode:dark/theme:kitchen");
        assert_eq!(s.with("light", "kitchen-light"), "mode:light/theme:kitchen");
        assert_eq!(s.with("mode", "system"), "theme:kitchen");
        assert_eq!(s.with("theme", "x</"), "mode:light");
    }
}
