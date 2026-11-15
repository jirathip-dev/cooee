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

/// `/layout/events`, read event by event: `(name, data)`.
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

/// `GET /layout` with `If-None-Match: etag`: the status, `ETag` header and body.
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

    // No file: the default, with its etag; 304 when the app already has it.
    let (code, tag, body) = layout(&c, "");
    let first: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        (code, &first["layout"]),
        (200, &serde_json::from_str(DEFAULT).unwrap()),
        "{body}"
    );
    assert_eq!(tag, format!("\"{}\"", first["etag"].as_str().unwrap()));
    let (code, again, body) = layout(&c, &tag);
    assert_eq!(
        (code, again.as_str(), body.as_str()),
        (304, tag.as_str(), "")
    );

    // A good write is served and announced.
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

    // A bad one: `broken` with its line, and the good one is still served.
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
    // A new subscriber hears it's broken at once.
    assert_eq!(Events::open(&c).next().0, "broken");

    // `cooee layout check`, the same rules: the default passes, the bad file fails at that line.
    fs::write(dir.join("default.json"), DEFAULT).unwrap();
    assert!(check(&c, &dir.join("default.json")).0);
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../apple/Tests/CooeeCoreTests/Fixtures/layouts");
    for f in fs::read_dir(fixtures).unwrap() {
        // The app's own refusals: each bad fixture fails here too; a locked screen named in a file is dropped.
        let f = f.unwrap().path();
        let ok = f.ends_with("locked-replaced.json");
        assert_eq!(check(&c, &f).0, ok, "{}", f.display());
    }
    let (ok, out) = check(&c, &file);
    assert!(
        !ok && out.contains(&format!("line {line}: ")) && out.contains("float"),
        "{out}"
    );

    // Reset: the default written, the bad file kept as .bak, served and announced.
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
