use cooee_core::{
    SHED_MIN_PROTOCOL,
    shed_proto::{Hello, PROTOCOL},
};

use crate::{
    setup::{Mac, files},
    support::{FakeShed, version},
};

#[test]
fn a_missing_or_too_old_shed_stops_setup_naming_the_fix_and_changes_nothing() {
    let mac = Mac::new("su-shed-old", false);
    let fix = format!(
        "fix: install or update shed to protocol {SHED_MIN_PROTOCOL} or newer from https://github.com/jirathip-dev/shed/releases, and start its serve"
    );
    let out = mac.setup(&["--yes"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("shed.sock") && stderr.contains(&format!("{fix}\nnothing was changed")),
        "{stderr}"
    );
    let old = Hello {
        build: "fffffff".to_owned(),
        protocol: 0,
        ..version()
    };
    let _old = FakeShed::start(&mac.home.join("Library/Caches/shed"), old);
    let before = files(&mac.home);
    let out = mac.setup(&["--yes"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let v = &version().version;
    let said = format!(
        "SHED_VERSION: this client speaks shed protocol 3 to {PROTOCOL}; the server is {v} (fffffff) protocol 1 to 0; update shed there"
    );
    assert!(stderr.contains("\nnothing was changed"), "{stderr}");
    assert!(stderr.contains(&said), "{stderr}\nwanted: {said}");
    assert_eq!(files(&mac.home), before);
    assert_eq!(mac.changes(), Vec::<String>::new());
}
