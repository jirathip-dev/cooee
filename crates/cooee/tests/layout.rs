//! `layout.json` beside `config.toml` (#309): `GET /layout` serves it with an `ETag` (304 when it matches); a bad
//! write keeps the last good layout and fans out `broken` with its line on `/layout/events`; `POST /layout/reset`
//! writes the default and keeps the old file as `.bak`; `cooee layout check` uses the same rules.

mod support;

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    path::Path,
    time::Duration,
};

use serde_json::Value;
use support::{Cooee, FakeShed, scratch, version};

const DEFAULT: &str = include_str!("../../../apple/Sources/CooeeUI/Default/layout.json");

struct Events(BufReader<TcpStream>);

impl Events {
    fn open(c: &Cooee) -> Self {
        let mut stream = TcpStream::connect(&c.addr).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(20)))
            .unwrap();
        let (addr, cookie) = (&c.addr, &c.cookie);
        write!(
            stream,
            "GET /layout/events HTTP/1.0\r\nHost: {addr}\r\nCookie: {cookie}\r\n\r\n"
        )
        .unwrap();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert!(line.contains(" 200 "), "{line}");
        Self(reader)
    }

    fn next(&mut self) -> (String, Value) {
        let (mut name, mut line) = (String::new(), String::new());
        loop {
            line.clear();
            assert!(self.0.read_line(&mut line).unwrap() > 0, "the stream ended");
            if let Some(n) = line.trim_end().strip_prefix("event: ") {
                n.clone_into(&mut name);
            } else if let Some(data) = line.trim_end().strip_prefix("data: ") {
                return (name, serde_json::from_str(data).unwrap());
            }
        }
    }
}

fn layout(c: &Cooee, etag: &str) -> (u16, String, String) {
    let cookie = c.cookie.as_str();
    let (head, body) = c.raw(
        "GET",
        "/layout",
        &[("Cookie", cookie), ("If-None-Match", etag)],
        "",
    );
    let tag = head
        .lines()
        .find_map(|l| l.strip_prefix("etag: "))
        .unwrap_or_default();
    (head[9..12].parse().unwrap(), tag.to_owned(), body)
}

fn check(c: &Cooee, file: &Path) -> (bool, String) {
    let out = c
        .command()
        .args(["layout", "check"])
        .arg(file)
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

#[test]
fn a_bad_write_keeps_the_last_good_layout_and_reset_restores_the_default() {
    let dir = scratch("layout");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let c = Cooee::start(&dir, &shed.dir);
    let file = dir.join("layout.json");

    let (code, tag, body) = layout(&c, "");
    let first: Value = serde_json::from_str(&body).unwrap();
    let checked = serde_json::to_value(cooee_server::layout_check(DEFAULT).unwrap()).unwrap();
    assert_eq!((code, &first["layout"]), (200, &checked), "{body}");
    assert_eq!(tag, format!("\"{}\"", first["etag"].as_str().unwrap()));
    let (code, again, body) = layout(&c, &tag);
    assert_eq!(
        (code, again.as_str(), body.as_str()),
        (304, tag.as_str(), "")
    );

    let mut events = Events::open(&c);
    let good = DEFAULT.replace(r#""id": "home""#, r#""id": "home", "label": "Home""#);
    fs::write(&file, &good).unwrap();
    let (name, data) = events.next();
    assert_eq!(name, "changed", "{data}");
    let (code, good_tag, body) = layout(&c, &tag);
    let served: Value = serde_json::from_str(&body).unwrap();
    assert_eq!((code, &served["etag"]), (200, &data["etag"]), "{body}");
    assert_eq!(
        served["layout"]["root"]["slots"]["main"][0]["label"],
        "Home"
    );

    let bad = good.replace(r#""place": "sheet" }"#, r#""place": "float" }"#);
    fs::write(&file, &bad).unwrap();
    let line = bad.lines().position(|l| l.contains("float")).unwrap() + 1;
    let (name, data) = events.next();
    assert_eq!(name, "broken", "{data}");
    assert_eq!(data["line"], line, "{data}");
    assert!(
        data["why"]
            .as_str()
            .unwrap()
            .contains(r#"unknown place "float""#),
        "{data}"
    );
    assert_eq!(layout(&c, &good_tag).0, 304);
    assert_eq!(Events::open(&c).next().0, "broken");

    assert_layout_check_uses_the_same_rules(&c, &dir, &file, line);

    let (code, body) = c.post("/layout/reset", &Value::Null);
    let reset: Value = serde_json::from_str(&body).unwrap();
    assert_eq!((code, &reset["etag"]), (200, &first["etag"]), "{body}");
    assert_eq!(fs::read_to_string(&file).unwrap(), DEFAULT);
    assert_eq!(
        fs::read_to_string(dir.join("layout.json.bak")).unwrap(),
        bad
    );
    assert_eq!(events.next().0, "changed");
    assert_eq!(layout(&c, &tag).0, 304);
}

fn assert_layout_check_uses_the_same_rules(c: &Cooee, dir: &Path, file: &Path, line: usize) {
    fs::write(dir.join("default.json"), DEFAULT).unwrap();
    assert!(check(c, &dir.join("default.json")).0);
    let faded = DEFAULT.replace(
        r#""place": "sheet" }"#,
        r#""place": "sheet", "transition": "fade" }"#,
    );
    assert_ne!(faded, DEFAULT);
    fs::write(dir.join("faded.json"), &faded).unwrap();
    assert!(check(c, &dir.join("faded.json")).0);
    fs::write(
        dir.join("duplicate.json"),
        r#"{ "format": 1, "root": { "type": "nav", "slots": { "main": [{ "type": "nav" }] } } }"#,
    )
    .unwrap();
    let (ok, out) = check(c, &dir.join("duplicate.json"));
    assert!(!ok && out.contains(r#"id "nav" is taken"#), "{out}");
    let (ok, out) = check(c, file);
    assert!(
        !ok && out.contains(&format!("line {line}: ")) && out.contains("float"),
        "{out}"
    );
}

const MANIFEST: &str = "name = \"drover\"\nversion = \"0.1.0\"\nbuild = \"true\"\nrun = \"true\"\n\n[[places]]\n\
                        type = \"header-strip\"\nlabel = \"Queue\"\nscreen = \"/screens/queue\"\n\n[permissions]\n\
                        places = [\"header-strip\", \"menu-row\"]\n";

#[test]
fn a_plugins_places_are_merged_and_a_place_its_manifest_lacks_is_skipped() {
    let dir = scratch("layout-places");
    let build = dir.join("data/plugins/drover/builds/c0ffee12345");
    fs::create_dir_all(&build).unwrap();
    fs::write(build.join("cooee-plugin.toml"), MANIFEST).unwrap();
    let lock = serde_json::json!([{ "name": "drover", "url": "https://example.com/drover", "commit": "c0ffee12345",
                                    "version": "0.1.0", "listed": true, "tabs": [], "port": 1, "enabled": true }]);
    fs::write(dir.join("data/plugins.lock"), lock.to_string()).unwrap();
    let shed = FakeShed::start(&dir.join("shed"), version());
    let c = Cooee::start(&dir, &shed.dir);

    let first = served(&c);
    let strip = &first["layout"]["root"]["slots"]["main"][1]["slots"]["strip"];
    assert_eq!(
        strip,
        &serde_json::json!([{ "type": "screen", "id": "drover-0", "source": "drover", "path": "/screens/queue",
                              "label": "Queue", "place": { "compact": "strip", "wide": "strip" }, "transition": "push" }]),
        "{first}"
    );
    assert_eq!(first["skipped"], serde_json::json!([]));

    let mut events = Events::open(&c);
    let moved = moved_layout();
    fs::write(dir.join("layout.json"), &moved).unwrap();
    assert_eq!(events.next().0, "changed");
    let answer = served(&c);
    let root = &answer["layout"]["root"];
    assert_eq!(
        root["slots"]["side"][0]["slots"]["top"][0]["place"],
        serde_json::json!({ "wide": "column", "compact": "menu" }),
        "{answer}"
    );
    assert_eq!(
        root["slots"]["main"][1]["slots"],
        serde_json::json!({ "cards": [] })
    );
    assert_eq!(root["slots"]["main"][0]["id"], "home");
    assert_eq!(
        answer["skipped"],
        serde_json::json!([
            r#"root.slots.main[1].slots.cards[0]: drover may not be placed "cards""#
        ])
    );
    assert_ne!(answer["etag"], first["etag"]);

    assert_a_gone_manifest_keeps_and_a_disabled_plugin_skips(&c, &dir, &build, &lock, &answer);
}

const PAIRING: &str = include_str!("../../../apple/Sources/CooeeUI/Default/screens/pairing.json");

#[test]
fn a_layout_that_replaces_a_locked_screen_is_refused_and_reset_still_restores_the_default() {
    let dir = scratch("layout-locked");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let c = Cooee::start(&dir, &shed.dir);
    let file = dir.join("layout.json");

    let pairing = c.json("/screens/pairing");
    let mut bundled: Value = serde_json::from_str(PAIRING).unwrap();
    bundled["source"] = "core".into();
    assert_eq!(pairing, bundled);
    let reset = c.json("/screens/reset-layout");
    assert_eq!(
        (&reset["source"], &reset["id"]),
        (&"core".into(), &"reset-layout".into())
    );
    assert_eq!(reset["blocks"][1]["action"], "/layout/reset", "{reset}");

    let (_, tag, _) = layout(&c, "");
    let mut events = Events::open(&c);
    let fake = DEFAULT.replace(
        r#""path": "/screens/menu""#,
        r#""path": "/screens/reset-layout""#,
    );
    let hidden = DEFAULT.replace(r#""type": "nav","#, r#""type": "nav", "place": "hidden","#);
    for bad in [&fake, &hidden] {
        assert_ne!(bad, DEFAULT);
        fs::write(&file, bad).unwrap();
        let (name, data) = events.next();
        assert_eq!(name, "broken", "{data}");
        assert_eq!(layout(&c, &tag).0, 304, "the last good layout is kept");
    }
    fs::write(dir.join("fake.json"), &fake).unwrap();
    let (ok, out) = check(&c, &dir.join("fake.json"));
    assert!(!ok && out.contains("a locked screen"), "{out}");

    let (code, body) = c.post("/layout/reset", &Value::Null);
    assert_eq!(code, 200, "{body}");
    assert_eq!(fs::read_to_string(&file).unwrap(), DEFAULT);
    assert_eq!(events.next().0, "changed");
    assert_eq!(layout(&c, &tag).0, 304);
}

#[test]
fn the_app_and_the_server_read_layouts_with_the_same_blocks() {
    let pin = |file: &str, key: &str| {
        let line = file
            .lines()
            .find(|l| l.contains("jirathip-dev/blocks"))
            .unwrap();
        line.split(key)
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .to_owned()
    };
    let cargo = pin(include_str!("../../../Cargo.toml"), "rev = \"");
    let swift = pin(include_str!("../../../apple/Package.swift"), "revision: \"");
    assert_eq!(cargo.len(), 40, "{cargo}");
    assert_eq!(swift, cargo);
}

fn served(c: &Cooee) -> Value {
    serde_json::from_str::<Value>(&layout(c, "").2).unwrap()
}

fn moved_layout() -> String {
    DEFAULT
        .replace(
            r#"{ "type": "sidebar", "#,
            r#"{ "type": "sidebar", "slots": { "top": [{ "type": "screen", "source": "drover", "path": "/screens/queue", "label": "Queue", "place": { "wide": "column", "compact": "menu" } }] }, "#,
        )
        .replace(
            r#""config": { "composer": true"#,
            r#""slots": { "cards": [{ "type": "screen", "source": "drover", "path": "/screens/cards", "place": "cards" }] }, "config": { "composer": true"#,
        )
}

fn assert_a_gone_manifest_keeps_and_a_disabled_plugin_skips(
    c: &Cooee,
    dir: &Path,
    build: &Path,
    lock: &Value,
    answer: &Value,
) {
    fs::remove_file(build.join("cooee-plugin.toml")).unwrap();
    for _ in 0..100 {
        assert_eq!(&served(c), answer);
    }
    let off = lock
        .to_string()
        .replace("\"enabled\":true", "\"enabled\":false");
    fs::write(dir.join("data/plugins.lock"), off).unwrap();
    let off = served(c);
    assert_eq!(
        off["skipped"],
        serde_json::json!([
            r#"root.slots.main[1].slots.cards[0]: no enabled plugin "drover""#,
            r#"root.slots.side[0].slots.top[0]: no enabled plugin "drover""#
        ])
    );
}
