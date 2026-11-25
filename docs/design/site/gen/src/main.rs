mod screens;

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use blocks::site::{self, Head, Live, Look};
use blocks::theme::{PerMode, Scheme, Style};
use blocks::{Block, Built, Prose, Screen};
use serde_json::{Value, json};

const REPO: &str = "https://github.com/jirathip-dev/cooee";
const BASE: &str = "https://jirathip-dev.github.io/cooee/";
const INSTALL: &str =
    "curl -fsSL https://raw.githubusercontent.com/jirathip-dev/cooee/main/scripts/install.sh | sh";

struct Kind {
    id: &'static str,
    label: &'static str,
    style: &'static str,
    pair: (&'static str, &'static str),
    scene: bool,
}

const KINDS: [Kind; 6] = [
    Kind { id: "modern", label: "Modern", style: MODERN, pair: ("github-dark", "paper"), scene: false },
    Kind { id: "terminal", label: "Terminal", style: TERMINAL, pair: ("github-dark", "paper"), scene: false },
    Kind { id: "farm", label: "Farm", style: SCENE, pair: ("farm", "farm-light"), scene: true },
    Kind { id: "kitchen", label: "Kitchen", style: SCENE, pair: ("kitchen", "kitchen-light"), scene: true },
    Kind { id: "garden", label: "Garden", style: SCENE, pair: ("garden-dark", "garden-light"), scene: true },
    Kind { id: "bush", label: "Bush", style: SCENE, pair: ("bush-dark", "bush-light"), scene: true },
];

const MODERN: &str = r#"{"format":1,"type":"default","radius":1,"surface":"solid","fills":"tint","marks":"symbols","sections":"title","labels":{}}"#;
const SCENE: &str = include_str!("../../../../../packs/scene/style.json");
const TERMINAL: &str = include_str!("../../../../../packs/terminal/style.json");

fn scheme(name: &str) -> Result<Scheme, Box<dyn Error>> {
    let (_, text) = swag::schemes()
        .iter()
        .find(|(n, _)| *n == name)
        .ok_or_else(|| format!("swag has no scheme {name}"))?;
    Ok(serde_json::from_str(text)?)
}

fn look(k: &Kind, one: Option<&str>) -> Result<Look, Box<dyn Error>> {
    let mut v: Value = serde_json::from_str(k.style)?;
    if let Some(o) = v.as_object_mut() {
        for cooee_only in ["label", "icon", "figures", "stage"] {
            o.remove(cooee_only);
        }
    }
    let style: Style = serde_json::from_value(v)?;
    let (dark, light) = match one {
        Some(s) => (scheme(s)?, scheme(s)?),
        None => (scheme(k.pair.0)?, scheme(k.pair.1)?),
    };
    Ok(Look { schemes: PerMode { dark, light }, style })
}

pub fn b(v: Value) -> Block {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("block: {e}"))
}

pub fn prose(id: &str, md: &str) -> Block {
    Block::Prose(Box::new(Prose::new(Prose::read(md)).id(id)))
}

fn button(id: &str, label: &str, sheet: &str) -> Value {
    json!({"type": "button", "id": id, "label": label, "tap": {"sheet": sheet}})
}

fn section(id: &str, header: &str, blocks: Vec<Block>) -> Block {
    let mut v = json!({"type": "section", "id": id, "blocks": blocks});
    if !header.is_empty() {
        v["header"] = json!(header);
    }
    b(v)
}

fn text(id: &str, md: &str) -> Block {
    b(json!({"type": "text", "id": id, "markdown": md}))
}

fn switcher(schemes: &[&str]) -> Block {
    let styles = KINDS.iter().map(|k| b(button(&format!("look-{}", k.id), k.label, &format!("look:{}", k.id)))).collect();
    let mut rows = vec![section("styles", "Look", styles)];
    for k in &KINDS {
        let mut set = vec![b(button(&format!("sch-{}-own", k.id), "its own", &format!("look:{}", k.id)))];
        set.extend(schemes.iter().map(|s| {
            b(button(&format!("sch-{}-{s}", k.id), &s.replace('-', " "), &format!("look:{}~{s}", k.id)))
        }));
        rows.push(section(&format!("sch-{}", k.id), "Colours", set));
    }
    section("switcher", "", rows)
}

fn top() -> Block {
    let link = |l: &str, h: &str| json!({"label": l, "href": h});
    b(json!({"type": "nav", "id": "site-nav", "brand": link("cooee", "#hero"), "links": [
        link("features", "#features"), link("themes", "#themes"), link("install", "#install"),
        link("docs", &format!("{REPO}/tree/main/docs")), link("github", REPO)
    ]}))
}

fn hero() -> Block {
    let hero = b(json!({"type": "hero", "id": "hero-copy", "title": "cooee",
        "subtitle": "Your agents, personas and machines in one chat, on your iPhone and Mac.",
        "actions": [
            {"label": "Install", "tap": {"open": "#install"}},
            {"label": "GitHub", "tap": {"url": REPO}}
        ]}));
    section("hero", "", vec![hero, screens::chat()])
}

fn feature(id: &str, name: &str, line: &str, screen: Block) -> Block {
    section(&format!("f-{id}"), name, vec![text(&format!("f-{id}-line"), line), screen])
}

fn features() -> Block {
    section("features", "Features", vec![
        feature("chat", "Chat and personas", "Each persona keeps its own chats, tools and memory.", screens::sessions()),
        feature("chief", "Chief routing", "Ask Chief; it hands the chat to the persona that fits.", screens::routed()),
        feature("plugins", "Plugins like drover", "Plugins draw their own screens, like drover's queue.", screens::drover()),
        feature("machines", "Machines", "Run agents on any Mac or Linux box you pair.", screens::machines()),
        feature("terminal", "Modern terminal", "Your Mac's shells, live on your phone.", screens::terminal()),
        feature("apps", "iPhone and Mac apps", "Native apps on iPhone and Mac, one server.", screens::home()),
    ])
}

fn themes() -> Block {
    let tiles = KINDS
        .iter()
        .map(|k| section(&format!("s-{}", k.id), k.label, vec![screens::tile(k.id)]))
        .collect();
    section("themes", "Themes", vec![
        text("themes-line", "Four packs, two styles, every colour scheme: pick one above and the whole page wears it."),
        section("tiles", "", tiles),
    ])
}

fn install() -> Block {
    let app = b(json!({"type": "card", "id": "i-app", "title": "The app", "blocks": [
        {"type": "row", "id": "i-testflight", "title": "iPhone and Mac · TestFlight", "subtitle": "join the beta", "accessory": "chevron", "tap": {"url": "https://testflight.apple.com/"}},
        {"type": "row", "id": "i-dmg", "title": "Mac · cooee.dmg", "subtitle": {"text": "with the next release", "tone": "mute"}}
    ]}));
    let server = b(json!({"type": "card", "id": "i-server", "title": "The server, on a Mac", "blocks": [
        prose("i-server-steps", &format!("```sh\n{INSTALL}\ncooee setup\n```")),
        {"type": "row", "id": "i-docs", "title": "docs/install.md", "subtitle": "shed and Tailscale first", "accessory": "chevron", "tap": {"url": format!("{REPO}/blob/main/docs/install.md")}}
    ]}));
    section("install", "Install", vec![app, server])
}

fn foot() -> Block {
    text("foot", &format!("[GitHub]({REPO}) · [Docs]({REPO}/tree/main/docs) · drawn by cooee's own screens, on blocks"))
}

fn css(kinds: &[(String, Look)], schemes: &[&str]) -> String {
    let mut out = String::new();
    for k in &KINDS {
        let on = format!("html[data-look=\"{0}\"],html[data-look^=\"{0}~\"]", k.id);
        let default = if k.id == "modern" { "html:not([data-look])," } else { "" };
        out.push_str(&format!(
            "[data-id=switcher] .sec[data-id=sch-{0}]{{display:none}}:is({default}{on}) [data-id=switcher] .sec[data-id=sch-{0}]{{display:flex}}\n:is({default}{on}) [data-id=look-{0}]{{background:var(--l);color:var(--bg)}}\n",
            k.id
        ));
        if k.scene {
            out.push_str(&format!(
                ":is({on}) [data-id=hero]{{background-image:url(scenes/{0}.svg)}}\n[data-id=s-{0}] .scr{{background-image:url(scenes/{0}.svg)}}\n",
                k.id
            ));
        }
        for s in schemes {
            out.push_str(&format!(
                "html[data-look=\"{0}~{s}\"] [data-id=sch-{0}-{s}]{{background:var(--l);color:var(--bg)}}\n",
                k.id
            ));
        }
        out.push_str(&format!(":is({default}html[data-look=\"{0}\"]) [data-id=sch-{0}-own]{{background:var(--l);color:var(--bg)}}\n", k.id));
    }
    for (name, l) in kinds.iter().filter(|(n, _)| !n.contains('~')) {
        out.push_str(&format!(
            "[data-id=s-{name}]{{{}}}\n@media (prefers-color-scheme: dark){{[data-id=s-{name}]{{{}}}}}\n",
            vars(&l.schemes.light),
            vars(&l.schemes.dark)
        ));
    }
    out + include_str!("../site.css")
}

fn vars(s: &Scheme) -> String {
    s.tokens.all().iter().map(|(k, v)| format!("{k}:{};", v.as_str())).collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = std::env::args().nth(1).map_or_else(|| here.join("../page"), PathBuf::from);
    let blocks_web = std::env::args().nth(2).map(PathBuf::from).ok_or("pass blocks' web/dist folder")?;
    let names: Vec<&str> = swag::schemes().iter().map(|(n, _)| *n).collect();
    let mut looks: Vec<(String, Look)> = vec![];
    for k in &KINDS {
        looks.push((k.id.to_owned(), look(k, None)?));
        for s in &names {
            looks.push((format!("{}~{s}", k.id), look(k, Some(s))?));
        }
    }
    let screen = Screen::new()
        .id("site")
        .title("cooee")
        .blocks(vec![section("top", "", vec![top(), switcher(&names)]), hero(), features(), themes(), install(), foot()]);
    let built = Built::of(&screen);
    let faults = built.check(&[]);
    if !faults.is_empty() {
        return Err(format!("{faults:?}").into());
    }
    let live: Vec<(&str, &Look)> = looks.iter().map(|(n, l)| (n.as_str(), l)).collect();
    let page = site::page(&built, &site::Site {
        head: Head {
            title: "cooee: your agents in one chat",
            description: "Your agents, personas and machines in one chat, on your iPhone and Mac.",
            canonical: BASE,
            image: None,
            lang: "en",
        },
        look: &looks[0].1,
        css: &["blocks.css", "site.css"],
        action: "",
        live: Some(Live { js: "site.js", stream: "", looks: &live }),
    });
    fs::create_dir_all(out.join("scenes"))?;
    fs::write(out.join("index.html"), page + "\n")?;
    fs::write(out.join("site.css"), css(&looks, &names))?;
    fs::write(out.join("site.js"), include_str!("../site.js"))?;
    for f in ["blocks.js", "blocks.css"] {
        fs::copy(blocks_web.join(f), out.join(f))?;
    }
    for p in swag::packs() {
        if let Some((_, svg)) = p.files.iter().find(|(f, _)| *f == "scene.svg") {
            fs::write(out.join("scenes").join(format!("{}.svg", p.name)), svg)?;
        }
    }
    Ok(())
}
