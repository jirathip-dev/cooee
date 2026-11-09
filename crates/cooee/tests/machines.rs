//! The first-party machines plugin in `cooee serve`: a `[[plugins]]` entry named `machines` runs it, its tab gets
//! the cards through cooee's proxy (the fake shed's `machine ls` and `machine stats`), and a machine offline puts a
//! dot on the tab.

mod support;

use std::{fs, net::TcpListener, thread, time::Duration};

use serde_json::json;
use support::{Cooee, FakeShed, scratch, version};

#[test]
fn the_machines_tab_shows_each_machine_and_a_dot_while_one_is_offline() {
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

    // The alert's dot on its row under More (#118), once the plugin has told cooee.
    let dot = "Machines</span><i class=\"r\" aria-hidden=\"true\">●</i><span class=\"vh\"> (needs you)</span>";
    let mut more = String::new();
    for _ in 0..100 {
        more = cooee.get("/more").1;
        if more.contains(dot) {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert!(more.contains(dot), "{more}");
}
