use cooee_core::{SHED_MIN_PROTOCOL, shed_proto::Hello};

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
        protocol: SHED_MIN_PROTOCOL - 1,
        ..version()
    };
    let _old = FakeShed::start(&mac.home.join("Library/Caches/shed"), old);
    let before = files(&mac.home);
    let out = mac.setup(&["--yes"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let (v, b) = (&version().version, &version().build);
    let said = format!(
        "SHED_VERSION: cooee speaks shed {v} ({b}) protocol 1; the server runs {v} (fffffff) protocol 0, older than protocol 1; {fix}\nnothing was changed"
    );
    assert!(stderr.contains(&said), "{stderr}\nwanted: {said}");
    assert_eq!(files(&mac.home), before);
    assert_eq!(mac.changes(), Vec::<String>::new());
}
