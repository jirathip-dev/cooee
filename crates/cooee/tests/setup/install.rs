use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

use crate::{
    setup::{asset, exe, sums},
    support::scratch,
};

#[test]
fn install_sh_downloads_the_latest_release_checks_it_and_runs_setup() {
    let home = scratch("install-sh");
    let bin = home.join("bin");
    let log = "printf '%s\\n' \"cooee $*\" >>\"$HOME/calls.log\"";
    exe(&home.join("release-cooee"), log);
    exe(
        &bin.join("curl"),
        r#"case "$*" in
*/releases/latest) echo '{"url": "x", "tag_name": "build-2222222", "draft": false}' ;;
*/build-2222222/SHA256SUMS) cat "$HOME/sums" ;;
*/build-2222222/cooee-*) for a; do [ "$prev" = -o ] && out=$a; prev=$a; done; cp "$HOME/release-cooee" "$out" ;;
*) exit 22 ;;
esac"#,
    );
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/install.sh");
    let run = || {
        Command::new("/bin/sh")
            .arg(&script)
            .args(["--instance", "drover"])
            .env("HOME", &home)
            .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
            .stdin(Stdio::null())
            .output()
            .unwrap()
    };
    let installed = home.join(".local/bin/cooee");
    let out = run();
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("tailscale not found"),
        "{out:?}"
    );
    assert!(!installed.exists());

    exe(&bin.join("tailscale"), "");
    fs::write(
        home.join("sums"),
        format!("{}  {}\n", "0".repeat(64), asset()),
    )
    .unwrap();
    let out = run();
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr)
            .contains("of build-2222222 doesn't match its SHA256SUMS"),
        "{out:?}"
    );
    assert!(!installed.exists() && !home.join("calls.log").exists());

    fs::write(home.join("sums"), sums(&home.join("release-cooee"))).unwrap();
    let out = run();
    assert!(out.status.success(), "{out:?}");
    assert_eq!(
        fs::read(&installed).unwrap(),
        fs::read(home.join("release-cooee")).unwrap()
    );
    assert_eq!(
        fs::read_to_string(home.join("calls.log")).unwrap(),
        "cooee setup --instance drover\n"
    );
}
