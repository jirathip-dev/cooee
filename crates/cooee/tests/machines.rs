//! The first-party machines plugin in `cooee serve`: a `[[plugins]]` entry named `machines` runs it, its tab gets
//! the cards through cooee's proxy (the fake shed's `machine ls` and `machine stats`), a machine offline marked so.

mod support;

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    thread,
    time::Duration,
};

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
