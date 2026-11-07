//! Shed connection reuse (#54): the client keeps helloed connections, reconnects once after a restart, never resends
//! a request that went out, and still checks the version on every new connection. Against the fake shed.

mod support;

use std::path::Path;

use cooee_core::{Shed, ShedError};
use support::{FakeShed, scratch, version};

fn client(dir: &Path) -> Shed {
    Shed::new(dir.join("shed"), Some(dir.to_owned()))
}

fn count(shed: &FakeShed, method: &str) -> usize {
    let requests = shed.requests.lock().unwrap();
    requests.iter().filter(|(m, _)| m == method).count()
}

#[test]
fn sequential_calls_share_one_connection_and_one_hello() {
    let dir = scratch("pool-reuse").join("shed");
    let fake = FakeShed::start(&dir, version());
    let shed = client(&dir);
    for _ in 0..5 {
        shed.get("t1").unwrap();
    }
    shed.prompt("t1", "hi").unwrap();
    shed.clone().cancel("t1").unwrap();
    assert_eq!(fake.connects(), 1);
    assert_eq!(count(&fake, "hello"), 1);
    assert_eq!(count(&fake, "get"), 5);
}

#[test]
fn a_call_after_a_restart_reconnects_and_succeeds() {
    let dir = scratch("pool-restart").join("shed");
    let old = FakeShed::start(&dir, version());
    let shed = client(&dir);
    shed.get("t1").unwrap();
    old.stop();
    let new = FakeShed::start(&dir, version());
    assert_eq!(shed.get("t1").unwrap().name, "t1");
    assert_eq!((new.connects(), count(&new, "hello")), (1, 1));
    assert_eq!(count(&old, "get"), 1);
}

#[test]
fn a_request_that_was_sent_is_never_resent() {
    let dir = scratch("pool-sent").join("shed");
    let fake = FakeShed::start(&dir, version());
    let shed = client(&dir);
    shed.get("t1").unwrap();
    fake.gate("hangup", true);
    assert!(shed.cancel("t1").is_err());
    fake.gate("hangup", false);
    assert_eq!(count(&fake, "send"), 1);
    assert_eq!(fake.connects(), 1);
}

#[test]
fn another_version_after_a_restart_is_a_version_error() {
    let dir = scratch("pool-version").join("shed");
    let old = FakeShed::start(&dir, version());
    let shed = client(&dir);
    shed.get("t1").unwrap();
    old.stop();
    let new = FakeShed::start(&dir, "0.0.0-other");
    let err = shed.get("t1").unwrap_err();
    assert!(matches!(err, ShedError::Version { .. }), "{err}");
    assert_eq!(count(&new, "get"), 0);
}
