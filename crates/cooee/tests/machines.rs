//! The first-party machines plugin in `cooee serve`: a `[[plugins]]` entry named `machines` runs it, its tab gets
//! the cards through cooee's proxy (the fake shed's `machine ls` and `machine stats`), a machine offline marked so.

mod support;

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::Duration,
};

use cooee_desk::{Caps, Desks, Name};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, scratch, version};

const FIXTURE: &str = include_str!("../../../docs/design/screens-as-data/fixtures/machines.json");

#[test]
fn the_machines_plugin_serves_each_machine_card_and_marks_one_offline() {
    let dir = scratch("machines");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let write = |name: &str, v: serde_json::Value| {
        fs::write(shed.dir.join(name), v.to_string()).unwrap();
    };
    write(
        "machines.json",
        json!([{ "name": "air", "target": "air.local" }]),
    );
    write(
        "stats-local.json",
        json!({ "at": 1_800_000_000u64, "load_per_cpu": 0.42, "mem_pressure_pct": 61, "swap_used_mb": 0,
                "disk_free_mb": 50_000, "cpus": 10, "uptime_s": 60, "terminals": { "working": 1 } }),
    );
    write(
        "stats-air.json",
        json!({ "machine": "air", "online": false, "last_seen": 1_700_000_000u64 }),
    );
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let entry = format!(
        "[[plugins]]\nname = \"machines\"\nlabel = \"Machines\"\nurl = \"http://127.0.0.1:{port}\"\nmodule = \"/tab.js\"\ntoken = \"machines-token-0123456789\"\n"
    );
    let cooee = Cooee::start_with(&dir, &shed.dir, &entry);
    let mut cards = String::new();
    for _ in 0..100 {
        cards = cooee.get("/p/machines/api/cards").1;
        if cards.contains("m-air") {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert!(
        cards.contains("<span class=\"k\">load/cpu</span><b>0.42</b>"),
        "{cards}"
    );
    assert!(
        cards.contains("<p class=\"tm\">terminals: 1 working</p>"),
        "{cards}"
    );
    assert!(
        cards.contains("<article class=\"m off\" aria-labelledby=\"m-air\">"),
        "{cards}"
    );
    assert!(cards.contains("offline · last seen"), "{cards}");
    let log = shed.cli_log();
    assert!(
        log.contains("--json --on air machine stats --since"),
        "{log}"
    );
}

fn fake_drover(hosts: Value) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for mut s in listener.incoming().flatten() {
            let mut head = String::new();
            let mut r = BufReader::new(&s);
            while r.read_line(&mut head).unwrap() > 2 {}
            let body = if head.starts_with("GET /api/v1/host ") {
                hosts.to_string()
            } else {
                "{}".to_owned()
            };
            write!(
                s,
                "HTTP/1.0 200 OK\r\nContent-Type: application/json\r\n\r\n{body}"
            )
            .ok();
        }
    });
    port
}

#[test]
fn each_machine_shows_its_drover_host_tags_on_its_settings_row() {
    let dir = scratch("machines-tags");
    let shed = FakeShed::start(&dir.join("shed"), version());
    fs::write(
        shed.dir.join("machines.json"),
        json!([{ "name": "d1", "target": "d1.cloud" }, { "name": "air", "target": "air.local" }])
            .to_string(),
    )
    .unwrap();
    let stats = json!({ "at": 1_800_000_000u64, "load_per_cpu": 0.2, "mem_pressure_pct": 30, "swap_used_mb": 0,
                        "disk_free_mb": 50_000, "cpus": 4, "uptime_s": 60, "terminals": {} });
    fs::write(shed.dir.join("stats-d1.json"), stats.to_string()).unwrap();
    fs::write(shed.dir.join("stats-air.json"), stats.to_string()).unwrap();
    let card = |name: &str, tags: &str| json!({ "name": name, "rows": [{ "k": "lanes", "notes": ["none"] }, { "k": "tags", "t": "m", "notes": [tags] }] });
    let port = fake_drover(json!({ "hosts": [card("d1", "gpu linux"), card("air", "arm")] }));
    let config = format!(
        "[[plugins]]\nname = \"drover\"\nlabel = \"Drover\"\nurl = \"http://127.0.0.1:{port}\"\n\
         module = \"/tab.js\"\ntoken = \"drover-0123456789abcdef\"\n"
    );
    let cooee = Cooee::start_with(&dir, &shed.dir, &config);
    let screen = cooee.json("/screens/settings/machines");
    let rows = screen["blocks"][0]["rows"].as_array().unwrap();
    let row = |id: &str| rows.iter().find(|r| r["id"] == id).unwrap().clone();
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
    for want in fixture["blocks"][0]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .skip(1)
    {
        assert_eq!(&row(want["id"].as_str().unwrap()), want, "{screen}");
    }
}

fn script(path: &Path, body: &str) {
    let mut sh = Command::new("sh")
        .args(["-c", "cat >\"$0\" && chmod 755 \"$0\""])
        .arg(path)
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    sh.stdin.take().unwrap().write_all(body.as_bytes()).unwrap();
    assert!(sh.wait().unwrap().success());
}

const CONTAINER: &str = r#"#!/bin/sh
d=$(dirname "$0")
case "$1 $2" in
"network ls") echo '[{"id":"cooee-desks","configuration":{"mode":"hostOnly"}}]' ;;
"ls --all") cat "$d/containers.json" ;;
"create --name")
  rest=$(cut -c2- "$d/containers.json")
  printf '[{"configuration":{"id":"%s","labels":{"%s":"%s"}},"status":"stopped"},%s' "$3" "${5%%=*}" "${5#*=}" "$rest" >"$d/next.json"
  mv "$d/next.json" "$d/containers.json" ;;
esac
"#;

#[test]
fn a_created_desk_lists_as_a_desk_of_its_provider_on_this_host_and_nothing_else_does() {
    let dir = scratch("machines-desk");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let claimed = json!({ "name": "air", "target": "air.local", "kind": "desk", "provider": "container", "parent": "mini" });
    fs::write(shed.dir.join("machines.json"), json!([claimed]).to_string()).unwrap();
    let container = dir.join("container");
    script(&container, CONTAINER);
    let by_hand =
        json!([{ "configuration": { "id": "by-hand", "labels": {} }, "status": "running" }]);
    fs::write(dir.join("containers.json"), by_hand.to_string()).unwrap();
    Desks::new(container.clone(), Vec::new())
        .create(
            &Name::new("research-1").unwrap(),
            "img",
            Caps::new(2, 2048).unwrap(),
        )
        .unwrap();

    let extra = format!("container = {:?}\n", container.to_string_lossy());
    let cooee = Cooee::start_with(&dir, &shed.dir, &extra);
    let machines = cooee.json("/machines");
    let row = |m: &Value| {
        json!([
            &m["host"],
            &m["kind"],
            &m["provider"],
            &m["parent"],
            &m["online"]
        ])
    };
    let rows: Vec<Value> = machines.as_array().unwrap().iter().map(row).collect();
    let local = &machines[0]["host"];
    assert_eq!(
        rows,
        [
            json!([local, "host", null, null, false]),
            json!(["air", "host", null, null, false]),
            json!(["research-1", "desk", "container", local, false]),
        ],
        "{machines}"
    );
}

const DESK: &str = r#"#!/bin/sh
d=$(dirname "$0")
printf '%s\n' "$*" >>"$d/calls.log"
case "$1" in
ls) echo '[{"configuration":{"id":"research-1","labels":{"cooee.provider":"container"}},"status":"running"},{"configuration":{"id":"by-hand","labels":{}},"status":"running"}]' ;;
exec) [ "$5" != import ] || printf jpeg ;;
esac
"#;

#[test]
fn a_desk_streams_through_cooee_and_takes_input_only_while_the_owner_holds_its_lease() {
    let dir = scratch("machines-desk-screen");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let container = dir.join("container");
    script(&container, DESK);
    let extra = format!("container = {:?}\n", container.to_string_lossy());
    let cooee = Cooee::start_with(&dir, &shed.dir, &extra);
    let screen = "/desks/research-1/screen";
    let inputs = || {
        let log = fs::read_to_string(dir.join("calls.log")).unwrap_or_default();
        log.lines()
            .filter(|l| l.contains("xdotool"))
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };

    let (status, body) = cooee.get("/desks/by-hand/screen");
    assert_eq!((status, body.as_str()), (400, "no desk by-hand"));
    assert_eq!(cooee.json(screen), json!({ "by": "agent" }));
    let mut live = cooee.sse(&format!("{screen}/live"), "");
    let frame = live.next();
    assert_eq!(
        (frame.event.as_str(), frame.data),
        ("frame", json!({ "jpeg": "anBlZw==" }))
    );
    drop(live);

    let click = json!({ "click": { "x": 10.0, "y": 20.0 } });
    let (status, body) = cooee.post(&format!("{screen}/input"), &click);
    assert_eq!(
        (status, body.as_str()),
        (400, "take control of desk research-1 first")
    );
    let (status, body) = cooee.post(&format!("{screen}/back"), &json!({}));
    assert_eq!(
        (status, body.as_str()),
        (400, "you don't hold desk research-1")
    );
    assert_eq!(inputs(), Vec::<String>::new());

    let (status, body) = cooee.post(&format!("{screen}/control"), &json!({}));
    assert_eq!(status, 200, "{body}");
    assert_eq!(serde_json::from_str::<Value>(&body).unwrap()["by"], "owner");
    for input in [click.clone(), json!({ "text": "hunter2" })] {
        let (status, body) = cooee.post(&format!("{screen}/input"), &input);
        assert_eq!(status, 200, "{body}");
    }
    let off = json!({ "click": { "x": -1.0, "y": 0.0 } });
    assert_eq!(cooee.post(&format!("{screen}/input"), &off).0, 400);
    assert_eq!(
        inputs(),
        [
            "exec research-1 env DISPLAY=:1 xdotool mousemove 10 20 click 1",
            "exec research-1 env DISPLAY=:1 xdotool type -- hunter2",
        ]
    );

    let (status, body) = cooee.post(&format!("{screen}/back"), &json!({}));
    assert_eq!((status, body.as_str()), (200, r#"{"by":"agent"}"#));
    assert_eq!(cooee.post(&format!("{screen}/input"), &click).0, 400);
    assert_eq!(inputs().len(), 2);
}
