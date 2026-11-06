//! What a device shows (#49): its pick from the `cooee-look` cookie (`theme:kitchen/style:terminal/…`; unset keys
//! are the defaults) resolved against the looks, so the server draws the theme and scheme on the first byte. The
//! marks (`[data-st]`, `[data-w]`, `[data-s]`) are drawn here as `look.ts` draws the ones a page adds later.

use std::{collections::BTreeMap, fmt::Write as _};

use crate::look::{Looks, Scheme, Theme, plain};

/// The cookie that keeps a device's pick.
pub const COOKIE: &str = "cooee-look";

/// The keys a pick has.
const KEYS: [&str; 7] = ["theme", "mode", "style", "list", "sprites", "dark", "light"];

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
pub struct Shown<'a> {
    looks: &'a Looks,
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

fn glyph(st: &str) -> &'static str {
    match st {
        "working" => "✻",
        "needs" => "!",
        "done" => "✓",
        "fail" => "✕",
        "idle" => "·",
        "ended" => "■",
        _ => "",
    }
}

fn ascii(id: &str) -> &'static str {
    match id {
        "edit" => "o_/",
        "read" => "o.o",
        "run" => "o>_",
        "needs" => "(!)",
        "done" | "merged" => "\\o/",
        "fail" => "x_x",
        "idle" => "-.-",
        "ended" => "[ ]",
        "loading" => "( o.o )\n  ...",
        "empty" => "( -.- )\n  zzz",
        _ => "",
    }
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

impl<'a> Shown<'a> {
    /// The pick in `cookie` (the `Cookie` header's value, or none).
    #[must_use]
    pub fn new(looks: &'a Looks, cookie: Option<&str>) -> Self {
        let value = cookie
            .into_iter()
            .flat_map(|c| c.split(';'))
            .find_map(|c| c.trim().strip_prefix(COOKIE)?.strip_prefix('='))
            .unwrap_or_default();
        let pick = value
            .split('/')
            .filter_map(|kv| kv.split_once(':'))
            .filter(|(k, _)| KEYS.contains(k))
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        Self { looks, pick }
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

    /// `modern` or `terminal`.
    #[must_use]
    pub fn style(&self) -> &str {
        if self.get("style") == Some("terminal") {
            "terminal"
        } else {
            "modern"
        }
    }

    /// Style's value: Scene (#34) on the thread list, else [`Self::style`], which threads keep in Scene.
    #[must_use]
    pub fn shape(&self) -> &str {
        if self.scene() { "scene" } else { self.style() }
    }

    /// The thread list draws as the theme's scene: picked, and offered (sprites on, a theme with sprites).
    #[must_use]
    pub fn scene(&self) -> bool {
        self.get("list") == Some("scene") && self.art()
    }

    /// `on`, `ascii` or `off`.
    #[must_use]
    pub fn figs(&self) -> &str {
        match self.get("sprites") {
            Some(f @ ("ascii" | "off")) => f,
            _ => "on",
        }
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

    /// Sprites show: the theme has them and the pick is `on` (Scene is offered only then).
    #[must_use]
    pub fn art(&self) -> bool {
        self.theme().sprites && self.figs() == "on"
    }

    fn svg(&self, id: &str, cls: &str) -> String {
        let bx = if cls == "big" {
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

    /// A state's mark (`[data-st]`): the sprite, ascii figure, glyph or dot, then the theme word (`plain`: and
    /// `· plain word`).
    #[must_use]
    pub fn st(&self, st: &str, plain: bool) -> String {
        if !STATES.contains(&st) {
            return String::new();
        }
        let id = if st == "working" { "edit" } else { st };
        let mark = match self.figs() {
            "off" => "<i class=\"sd\"></i>".to_owned(),
            "ascii" => format!("<i class=\"rsa\">{}</i> ", esc(ascii(id))),
            _ if self.art() => self.svg(id, "rsp"),
            _ => format!("<i class=\"sg\">{}</i> ", glyph(st)),
        };
        let said = self.word(st);
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
        match self.figs() {
            "ascii" => format!("<i class=\"lsa\">{}</i>", esc(ascii(k))),
            _ if self.art() => self.svg(k, "big"),
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
        } else if k == "style" && v == "scene" {
            pick.insert("list".to_owned(), v.to_owned());
        } else if k == "style" {
            pick.remove("list");
            pick.insert(k.to_owned(), v.to_owned());
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

    fn looks() -> Looks {
        Looks::load(None, None).0
    }

    #[test]
    fn the_cookie_picks_the_theme_scheme_style_and_sprites() {
        let looks = looks();
        let cookie = "cooee=abc; cooee-look=theme:kitchen/style:terminal/sprites:off/mode:dark";
        let s = Shown::new(&looks, Some(cookie));
        assert_eq!(
            (s.name(), s.style(), s.figs(), s.mode()),
            ("kitchen", "terminal", "off", Some("dark"))
        );
        assert!(s.css().starts_with(":root{"), "{}", s.css());
        assert!(s.css().contains("--bg:#1a1816;"), "{}", s.css());
        assert!(!s.css().contains("@media"));
        assert_eq!(
            s.st("needs", false),
            "<i class=\"sd\"></i><span class=\"thw\">the bell</span>"
        );
        // No pick: the default theme, System with both schemes.
        let d = Shown::new(&looks, None);
        assert_eq!(
            (d.name(), d.style(), d.figs(), d.mode()),
            ("farm", "modern", "on", None)
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
    }

    #[test]
    fn a_bad_pick_falls_back_and_cant_break_out() {
        let looks = looks();
        let s = Shown::new(&looks, Some("cooee-look=theme:nope/dark:paper/sprites:x"));
        assert_eq!(s.name(), "farm");
        // A light scheme isn't a dark pick.
        assert_eq!(s.scheme("dark"), "farm");
        assert_eq!(s.figs(), "on");
        assert_eq!(declaration("--x", "red;}</style>"), None);
        assert_eq!(
            esc("<a href=\"x\">'&"),
            "&#60;a href=&#34;x&#34;&#62;&#39;&#38;"
        );
    }

    #[test]
    fn a_tap_gives_the_next_cookie() {
        let looks = looks();
        let s = Shown::new(&looks, Some("cooee-look=theme:kitchen/mode:light"));
        assert_eq!(
            s.with("style", "terminal"),
            "mode:light/style:terminal/theme:kitchen"
        );
        // The other mode's list switches the forced mode; the theme's default forgets the pick.
        assert_eq!(s.with("dark", "nord"), "dark:nord/mode:dark/theme:kitchen");
        assert_eq!(s.with("light", "kitchen-light"), "mode:light/theme:kitchen");
        assert_eq!(s.with("mode", "system"), "theme:kitchen");
        assert_eq!(s.with("theme", "x</"), "mode:light");
        // Scene keeps the threads' style; Modern or Terminal leaves it.
        let scene = Shown::new(&looks, Some("cooee-look=style:terminal"));
        assert_eq!(scene.with("style", "scene"), "list:scene/style:terminal");
        let scene = Shown::new(&looks, Some("cooee-look=list:scene/style:terminal"));
        assert_eq!(
            (scene.scene(), scene.shape(), scene.style()),
            (true, "scene", "terminal")
        );
        assert_eq!(scene.with("style", "modern"), "style:modern");
        // Sprites ascii or off, or a theme without sprites, hide Scene.
        for pick in [
            "list:scene/sprites:ascii",
            "list:scene/sprites:off",
            "list:scene/theme:terminal",
        ] {
            let s = Shown::new(&looks, Some(&format!("cooee-look={pick}")));
            assert_eq!((s.scene(), s.shape()), (false, "modern"), "{pick}");
        }
    }
}
