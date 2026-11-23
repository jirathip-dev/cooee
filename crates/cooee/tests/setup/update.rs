use std::fs;

use crate::setup::{RESTART, SHA, asset, files, says, updating};

#[test]
fn update_check_says_what_it_would_install_and_changes_nothing() {
    let mac = updating("up-check", true, &says(SHA));
    fs::write(mac.home.join("calls.log"), "").unwrap();
    let before = files(&mac.home);
    let out = mac.cooee(&["update", "--check"]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        format!(
            "would install {SHA}'s release, restart dev.cooee.cooee and check that port {} answers\n",
            mac.port
        )
    );
    let calls = fs::read_to_string(mac.home.join("calls.log")).unwrap();
    assert!(
        calls
            .lines()
            .all(|l| l.starts_with("curl ") && !l.contains(" -o ")),
        "{calls}"
    );
    assert_eq!(files(&mac.home), before);
    assert_eq!(
        (mac.prints(".local/bin/cooee"), mac.prints("serving")),
        ("old\n".into(), "old\n".into())
    );
}

#[test]
fn update_installs_main_heads_release_with_no_build_or_signing_and_doctor_shows_it() {
    let mac = updating("up-green", true, &says(SHA));
    let out = mac.cooee(&["update"]);
    assert!(out.status.success(), "{out:?}");
    let calls = fs::read_to_string(mac.home.join("calls.log")).unwrap();
    let at = format!(
        "https://github.com/jirathip-dev/cooee/releases/download/build-2222222/{}",
        asset()
    );
    assert!(calls.contains(&at), "{calls}");
    assert!(calls.contains(RESTART), "{calls}");
    assert_eq!(mac.prints("serving"), format!("cooee 0.1.0 ({SHA})\n"));
    assert!(!mac.home.join(".local/bin/cooee.new").exists());
    assert!(
        mac.log().ends_with(&format!(" installed {SHA}\n")),
        "{}",
        mac.log()
    );
    let out = mac.cooee(&["doctor"]);
    assert!(
        String::from_utf8_lossy(&out.stdout).contains(
            "info update: installed unknown, main's latest release build-2222222, last check 0 min ago, auto-update on\n"
        ),
        "{out:?}"
    );
}

#[test]
fn a_head_with_no_release_yet_or_auto_update_off_installs_nothing() {
    let mac = updating("up-none", false, &says(SHA));
    let out = mac.cooee(&["update"]);
    assert!(out.status.success(), "{out:?}");
    assert!(
        mac.log()
            .ends_with(" main 2222222: no release yet, not installed\n"),
        "{}",
        mac.log()
    );
    assert!(!mac.home.join("SHA256SUMS").exists());
    fs::write(
        mac.config(),
        format!("port = {}\nauto_update = false\n", mac.port),
    )
    .unwrap();
    fs::write(mac.home.join("calls.log"), "").unwrap();
    let out = mac.cooee(&["update"]);
    assert!(
        String::from_utf8_lossy(&out.stdout).starts_with("auto-update is off"),
        "{out:?}"
    );
    assert_eq!(fs::read_to_string(mac.home.join("calls.log")).unwrap(), "");
    assert_eq!(mac.prints(".local/bin/cooee"), "old\n");
}

#[test]
fn a_download_that_fails_its_checksum_or_names_another_sha_is_never_swapped_in() {
    let other = "3333333333333333333333333333333333333333";
    for (name, new, sums, why) in [
        (
            "up-sum",
            says(SHA),
            format!("{}  {}\n", "0".repeat(64), asset()),
            "doesn't match SHA256SUMS".to_owned(),
        ),
        (
            "up-sha",
            says(other),
            String::new(),
            format!("its --version names another build: cooee 0.1.0 ({other})"),
        ),
    ] {
        let mac = updating(name, true, &new);
        if !sums.is_empty() {
            fs::write(mac.home.join("SHA256SUMS"), sums).unwrap();
        }
        let out = mac.cooee(&["update"]);
        assert_eq!(out.status.code(), Some(1), "{out:?}");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            err.contains("2222222 not installed: ") && err.contains(&why),
            "{err}"
        );
        let calls = fs::read_to_string(mac.home.join("calls.log")).unwrap();
        assert!(!calls.contains(RESTART), "{calls}");
        assert_eq!(mac.prints(".local/bin/cooee"), "old\n");
        assert!(!mac.home.join(".local/bin/cooee.new").exists());
        assert_eq!(
            fs::read_to_string(mac.data().join("update.failed")).unwrap(),
            format!("{SHA} by unknown")
        );
    }
}

#[test]
fn a_new_build_that_does_not_answer_is_rolled_back_and_the_old_one_serves_again() {
    let new = format!("[ \"$1\" = --version ] || exit 1\n{}", says(SHA));
    let mac = updating("up-rollback", true, &new);
    let out = mac.cooee(&["update"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains(&format!(
            "2222222 not installed: the service doesn't answer on port {}; rolled back to the old binary: it serves again",
            mac.port
        )),
        "{err}"
    );
    let calls = fs::read_to_string(mac.home.join("calls.log")).unwrap();
    assert_eq!(calls.matches(RESTART).count(), 2, "{calls}");
    assert_eq!(
        (mac.prints(".local/bin/cooee"), mac.prints("serving")),
        ("old\n".into(), "old\n".into())
    );
    assert_eq!(
        fs::read_to_string(mac.data().join("update.failed")).unwrap(),
        format!("{SHA} by unknown")
    );
    fs::write(mac.home.join("calls.log"), "").unwrap();
    assert!(mac.cooee(&["update"]).status.success());
    assert!(
        mac.log()
            .ends_with(" main 2222222 failed to install before: it waits for a newer one\n"),
        "{}",
        mac.log()
    );
    assert!(
        !fs::read_to_string(mac.home.join("calls.log"))
            .unwrap()
            .contains("releases/download")
    );
}
