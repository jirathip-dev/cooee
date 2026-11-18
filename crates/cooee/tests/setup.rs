//! `cooee setup` with fake `shed`, `launchctl`, `tailscale` and `curl` on `PATH` (each logs its arguments to
//! `$HOME/calls.log` and keeps its state in `$HOME`) and a fake shed server for doctor's hello: a fresh machine runs
//! every step once, in order; a second run skips them all; `--dry-run` writes nothing; a doctor failure stops with its
//! fix; an existing config keeps its keys; a shed below the minimum protocol stops setup before it changes anything.
//! Also `scripts/install.sh`, and `cooee update` (#332, #584) against fake GitHub releases (`curl`), with the fake
//! `launchctl kickstart` serving the installed binary that `curl` asks on the port.

mod support;

use std::{
    fs,
    net::TcpListener,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use cooee_core::{SHED_MIN_PROTOCOL, shed_proto::Hello};
use support::{FakeShed, scratch, version};

const BIN: &str = env!("CARGO_BIN_EXE_cooee");
const FAKES: &[(&str, &str)] = &[
    (
        "shed",
        r#"case "$*" in
"--json agents") cat "$HOME/agents.json" 2>/dev/null || echo '[]' ;;
"agents add claude-acp") echo '[{"id":"claude-acp","version":"1"}]' >"$HOME/agents.json" ;;
*) exit 2 ;;
esac"#,
    ),
    (
        "launchctl",
        r#"case "$1" in
list) test -e "$HOME/loaded-$2" ;;
bootstrap) touch "$HOME/loaded-$(basename "$3" .plist)" ;;
kickstart) cp "$HOME/.local/bin/cooee" "$HOME/serving" 2>/dev/null || true ;;
esac"#,
    ),
    (
        "tailscale",
        r#"case "$*" in
"status --json") echo '{"Self":{"DNSName":"mac.tailnet.example."}}' ;;
"serve status --json") cat "$HOME/served.json" 2>/dev/null || echo '{}' ;;
"serve --bg"*) port=${3#--https=}; echo "{\"TCP\":{\"$port\":{\"HTTPS\":true}}}" >"$HOME/served.json" ;;
esac"#,
    ),
    (
        "curl",
        r#"case "$*" in
*/commits/main) cat "$HOME/main.json" ;;
*/releases/latest) echo '{"tag_name":"build-2222222"}' ;;
*/releases/download/*/SHA256SUMS) cat "$HOME/SHA256SUMS" ;;
*/releases/download/*) for a; do [ "$prev" = -o ] && out=$a; prev=$a; done; cp "$HOME/new-cooee" "$out" ;;
*127.0.0.1*) "$HOME/serving" ;;
*) [ -e "$HOME/tailnet" ] || exit 0
   for a; do shift; case $a in https://*) a="http://$(cat "$HOME/tailnet")/${a#https://*/}" ;; esac; set -- "$@" "$a"; done
   exec /usr/bin/curl "$@" ;;
esac"#,
    ),
];

const CHANGES: &[&str] = &[
    "shed agents add",
    "launchctl bootstrap",
    "launchctl kickstart",
    "launchctl bootout",
    "tailscale serve --bg",
];

struct Mac {
    home: PathBuf,
    port: u16,
    _listener: TcpListener,
    _shed: Option<FakeShed>,
}

impl Mac {
    fn new(name: &str, shed: bool) -> Self {
        let home = scratch(name);
        let bin = home.join("bin");
        fs::create_dir_all(&bin).unwrap();
        for (name, body) in FAKES {
            let file = bin.join(name);
            let log = format!("printf '%s\\n' \"{name} $*\" >>\"$HOME/calls.log\"");
            fs::write(&file, format!("#!/bin/sh\n{log}\n{body}\n")).unwrap();
            fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let shed = shed.then(|| FakeShed::start(&home.join("Library/Caches/shed"), version()));
        let listener = std::iter::repeat_with(|| TcpListener::bind("127.0.0.1:0").unwrap())
            .find(|l| l.local_addr().unwrap().port() <= u16::MAX - 1000)
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        Self {
            home,
            port,
            _listener: listener,
            _shed: shed,
        }
    }

    fn setup(&self, args: &[&str]) -> Output {
        self.cooee(&[&["setup", "--port", &self.port.to_string()], args].concat())
    }

    fn cooee(&self, args: &[&str]) -> Output {
        self.cooee_typing(args, None)
    }

    fn cooee_typing(&self, args: &[&str], input: Option<&str>) -> Output {
        let mut child = Command::new(BIN)
            .args(args)
            .env("GIT_CONFIG_COUNT", "1")
            .env(
                "GIT_CONFIG_KEY_0",
                format!("url.{}/offline/.insteadOf", self.home.display()),
            )
            .env("GIT_CONFIG_VALUE_0", "https://github.com/")
            .env("HOME", &self.home)
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", self.home.join("bin").display()),
            )
            .env("XDG_RUNTIME_DIR", self.home.join("Library/Caches/shed"))
            .env_remove("XDG_CONFIG_HOME")
            .env_remove("XDG_DATA_HOME")
            .stdin(if input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        if let Some(input) = input {
            let mut stdin = child.stdin.take().unwrap();
            std::io::Write::write_all(&mut stdin, input.as_bytes()).unwrap();
        }
        child.wait_with_output().unwrap()
    }

    fn changes(&self) -> Vec<String> {
        let log = self.home.join("calls.log");
        let calls = fs::read_to_string(&log).unwrap_or_default();
        fs::write(&log, "").unwrap();
        calls
            .lines()
            .filter(|l| CHANGES.iter().any(|c| l.starts_with(c)))
            .map(str::to_owned)
            .collect()
    }

    fn config(&self) -> PathBuf {
        self.home.join(".config/cooee/cooee/config.toml")
    }
}

fn files(dir: &Path) -> Vec<String> {
    let mut all = Vec::new();
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            all.extend(
                files(&path)
                    .into_iter()
                    .map(|f| format!("{}/{f}", entry.file_name().display())),
            );
        } else {
            all.push(entry.file_name().display().to_string());
        }
    }
    all.sort();
    all
}

fn terminal(port: u16, config: &str) -> String {
    let token = config
        .rsplit_once("token = \"")
        .unwrap()
        .1
        .split('"')
        .next()
        .unwrap();
    assert!(
        token.len() == 64 && token.bytes().all(|b| b.is_ascii_hexdigit()),
        "{token}"
    );
    format!(
        "\n[[plugins]]\nname = \"terminal\"\nlabel = \"Terminal\"\nurl = \"http://127.0.0.1:{}\"\nmodule = \"/tab.js\"\ntoken = \"{token}\"\n",
        port + 1000
    )
}

#[test]
fn a_fresh_machine_runs_every_step_once_in_order_and_a_second_run_skips_them() {
    let mac = Mac::new("su-fresh", true);
    let out = mac.setup(&["--yes"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{out:?}");
    let (port, home) = (mac.port, mac.home.display());
    let plist = format!("{home}/Library/LaunchAgents/dev.cooee.cooee.plist");
    let gui = format!(
        "gui/{}",
        String::from_utf8(Command::new("id").arg("-u").output().unwrap().stdout)
            .unwrap()
            .trim()
    );
    assert_eq!(
        mac.changes(),
        [
            "shed agents add claude-acp".to_owned(),
            format!("launchctl bootstrap {gui} {plist}"),
            format!(
                "launchctl bootstrap {gui} {home}/Library/LaunchAgents/dev.cooee.cooee.update.plist"
            ),
            format!("tailscale serve --bg --https={port} http://127.0.0.1:{port}"),
        ]
    );
    let ignore = fs::read_to_string(mac.config().with_file_name(".gitignore")).unwrap();
    assert!(ignore.lines().any(|l| l == "secrets/"), "{ignore}");
    let config = fs::read_to_string(mac.config()).unwrap();
    assert_eq!(
        config,
        format!(
            "port = {port}\nallowed_hosts = [\"mac.tailnet.example\"]\nroots = [\"~/Projects\"]\n{}",
            terminal(port, &config)
        )
    );
    assert!(
        fs::read_to_string(&plist)
            .unwrap()
            .contains("<string>dev.cooee.cooee</string>")
    );
    let timer = fs::read_to_string(
        mac.home
            .join("Library/LaunchAgents/dev.cooee.cooee.update.plist"),
    )
    .unwrap();
    assert!(
        timer.contains("<string>update</string>")
            && timer.contains("<key>StartInterval</key>\n  <integer>900</integer>"),
        "{timer}"
    );
    assert!(
        stdout.contains("doctor:\nok   service dev.cooee.cooee loaded")
            && stdout.contains(&format!(
                "link  https://mac.tailnet.example:{port}/pair?code="
            )),
        "{stdout}"
    );

    let out = mac.setup(&["--yes"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(mac.changes(), Vec::<String>::new());
    for step in [
        &format!("shed protocol {SHED_MIN_PROTOCOL}+"),
        &format!("config {}", mac.config().display()),
        "agents",
        "cooee service dev.cooee.cooee",
        "cooee service dev.cooee.cooee.update",
        &format!("tailscale serve {port}"),
    ] {
        assert!(
            stdout.contains(&format!("{step}: done, skipped\n")),
            "{step} not skipped in {stdout}"
        );
    }
    assert!(
        stdout.contains("pair your phone:\nType the link") && stdout.contains("\ncode  "),
        "{stdout}"
    );
}

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

#[test]
fn dry_run_writes_nothing_and_prints_every_command() {
    let mac = Mac::new("su-dry", true);
    let mut want = files(&mac.home);
    let out = mac.setup(&["--dry-run", "--yes"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(mac.changes(), Vec::<String>::new());
    want.push("calls.log".to_owned());
    want.sort();
    assert_eq!(files(&mac.home), want);
    let port = mac.port;
    for line in [
        format!("would write {}:\nport = {port}\n", mac.config().display()),
        "would run: shed agents add claude-acp".to_owned(),
        "would write ".to_owned()
            + &mac
                .home
                .join("Library/LaunchAgents/dev.cooee.cooee.plist")
                .display()
                .to_string(),
        format!("would run: tailscale serve --bg --https={port} http://127.0.0.1:{port}"),
        "would run: cooee doctor, then cooee pair".to_owned(),
    ] {
        assert!(stdout.contains(&line), "{line} not in {stdout}");
    }
}

#[test]
fn a_tailnet_url_answering_405_on_get_pair_passes_doctor() {
    let mac = Mac::new("su-405", true);
    let server = TcpListener::bind("127.0.0.1:0").unwrap();
    fs::write(
        mac.home.join("tailnet"),
        server.local_addr().unwrap().to_string(),
    )
    .unwrap();
    std::thread::spawn(move || {
        for mut conn in server.incoming().flatten() {
            let mut buf = [0; 4096];
            std::io::Read::read(&mut conn, &mut buf).unwrap();
            std::io::Write::write_all(
                &mut conn,
                b"HTTP/1.1 405 Method Not Allowed\r\nallow: POST\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
            )
            .unwrap();
        }
    });
    let out = mac.setup(&["--yes"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{out:?}");
    let port = mac.port;
    assert!(
        stdout.contains(&format!(
            "ok   https://mac.tailnet.example:{port}/ (else: tailscale serve --bg --https={port} http://127.0.0.1:{port})"
        )),
        "{stdout}"
    );
}

#[test]
fn a_doctor_failure_stops_with_its_fix() {
    let mac = Mac::new("su-doctor", true);
    let unused = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap();
    fs::write(mac.home.join("tailnet"), unused.to_string()).unwrap();
    let out = mac.setup(&["--yes"]);
    let (stdout, stderr) = (
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let port = mac.port;
    let fail = format!(
        "FAIL https://mac.tailnet.example:{port}/ (else: tailscale serve --bg --https={port} http://127.0.0.1:{port})"
    );
    assert!(
        stderr.contains(&format!("doctor: 1 check(s) failed:\n{fail}\n")),
        "{stderr}"
    );
    assert!(!stdout.contains("code  "), "{stdout}");
}

#[test]
fn an_existing_config_keeps_its_keys() {
    let mac = Mac::new("su-keep", true);
    let old = format!(
        "port = {}\nroots = [\"/srv/code\"]\ntheme = \"farm\"\n\n[[plugins]]\nname = \"board\"\nlabel = \"Board\"\n\
         url = \"http://127.0.0.1:1\"\nmodule = \"/tab.js\"\ntoken = \"0123456789abcdef\"\n",
        mac.port
    );
    fs::create_dir_all(mac.config().parent().unwrap()).unwrap();
    fs::write(mac.config(), &old).unwrap();
    let out = mac.setup(&["--yes"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{out:?}");
    let config = fs::read_to_string(mac.config()).unwrap();
    let old = format!("{old}{}", terminal(mac.port, &config));
    assert_eq!(
        config,
        format!("allowed_hosts = [\"mac.tailnet.example\"]\n{old}")
    );
    assert!(
        stdout.contains("adding allowed_hosts = [\"mac.tailnet.example\"], the terminal plugin\n"),
        "{stdout}"
    );

    let out = mac.cooee(&["setup", "--yes", "--port", "1"]);
    assert!(
        String::from_utf8_lossy(&out.stderr).contains(&format!("says port = {}", mac.port)),
        "{out:?}"
    );
    assert_eq!(
        fs::read_to_string(mac.config()).unwrap(),
        format!("allowed_hosts = [\"mac.tailnet.example\"]\n{old}")
    );
}

#[test]
fn setup_installs_the_first_party_themes_chosen_with_no_network() {
    let mac = Mac::new("su-themes", true);
    let swag = cooee_core::swag::Swag::pinned();
    fs::create_dir_all(mac.config().parent().unwrap()).unwrap();
    fs::write(
        mac.config(),
        format!("port = {}\nroots = [\"/srv\"]\n", mac.port),
    )
    .unwrap();
    let args = ["setup", "--port", &mac.port.to_string()];
    let out = mac.cooee_typing(&args, Some("kitchen\n\n\n"));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{out:?}");
    assert!(
        stdout.contains(
            "themes:\n  themes to install, or none [garden kitchen]:\n  installed theme farm\n  installed theme scene\n  installed theme terminal\n  installed theme kitchen\n"
        ),
        "{stdout}"
    );
    let themes = mac.config().parent().unwrap().join("themes");
    assert_eq!(
        files(&themes),
        [
            "farm/icon.png",
            "farm/scene.svg",
            "farm/sprites.svg",
            "farm/theme.json",
            "kitchen/icon.png",
            "kitchen/scene.svg",
            "kitchen/sprites.svg",
            "kitchen/theme.json",
            "scene/style.json",
            "terminal/style.json"
        ]
    );
    let lock: serde_json::Value = serde_json::from_slice(
        &fs::read(mac.home.join(".local/share/cooee/cooee/themes.lock")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        lock,
        serde_json::json!([
            { "name": "farm", "source": swag.url, "commit": swag.commit, "listed": true },
            { "name": "kitchen", "source": swag.url, "commit": swag.commit, "listed": true },
            { "name": "scene", "source": "built in", "commit": "", "listed": true },
            { "name": "terminal", "source": "built in", "commit": "", "listed": true }
        ])
    );
    assert!(
        !mac.home
            .join(".local/share/cooee/cooee/themes/kitchen.git")
            .exists()
    );
    let out = mac.setup(&["--yes"]);
    assert!(out.status.success(), "{out:?}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("themes:\n  installed theme garden\n"),
        "{stdout}"
    );
    assert!(themes.join("garden/theme.json").exists());
    let out = mac.setup(&["--yes"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("themes: done, skipped\n"));
}

fn exe(file: &Path, body: &str) {
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(file, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(file, fs::Permissions::from_mode(0o755)).unwrap();
}

fn asset() -> String {
    let os = if cfg!(target_os = "macos") {
        "apple-darwin"
    } else {
        "unknown-linux-gnu"
    };
    format!("cooee-{}-{os}", std::env::consts::ARCH)
}

fn sums(file: &Path) -> String {
    format!(
        "{}  {}\n",
        cooee_core::sha256(&fs::read(file).unwrap()),
        asset()
    )
}

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

const SHA: &str = "2222222222222222222222222222222222222222";

fn says(sha: &str) -> String {
    format!("echo \"cooee 0.1.0 ({sha})\"")
}

fn updating(name: &str, released: bool, new: &str) -> Mac {
    let mac = Mac::new(name, false);
    exe(&mac.home.join(".local/bin/cooee"), "echo old");
    exe(&mac.home.join("serving"), "echo old");
    exe(&mac.home.join("new-cooee"), new);
    if released {
        let sums = sums(&mac.home.join("new-cooee"));
        fs::write(mac.home.join("SHA256SUMS"), sums).unwrap();
    }
    fs::write(mac.home.join("main.json"), format!(r#"{{"sha":"{SHA}"}}"#)).unwrap();
    fs::create_dir_all(mac.config().parent().unwrap()).unwrap();
    fs::write(mac.config(), format!("port = {}\n", mac.port)).unwrap();
    fs::create_dir_all(mac.data()).unwrap();
    mac
}

impl Mac {
    fn data(&self) -> PathBuf {
        self.home.join(".local/share/cooee/cooee")
    }

    fn prints(&self, name: &str) -> String {
        let out = Command::new(self.home.join(name)).output().unwrap();
        String::from_utf8(out.stdout).unwrap()
    }

    fn log(&self) -> String {
        fs::read_to_string(self.data().join("update.log")).unwrap()
    }
}

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
    assert!(calls.contains("launchctl kickstart -k gui/"), "{calls}");
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
        assert!(!calls.contains("launchctl kickstart"), "{calls}");
        assert_eq!(mac.prints(".local/bin/cooee"), "old\n");
        assert!(!mac.home.join(".local/bin/cooee.new").exists());
        assert_eq!(
            fs::read_to_string(mac.data().join("update.failed")).unwrap(),
            SHA
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
    assert_eq!(
        calls.matches("launchctl kickstart -k").count(),
        2,
        "{calls}"
    );
    assert_eq!(
        (mac.prints(".local/bin/cooee"), mac.prints("serving")),
        ("old\n".into(), "old\n".into())
    );
    assert_eq!(
        fs::read_to_string(mac.data().join("update.failed")).unwrap(),
        SHA
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

#[test]
fn setup_on_a_mac_whose_app_ran_the_server_reuses_its_config_and_data() {
    let mac = Mac::new("su-app", true);
    for item in ["com.jirathip.cooee.serve", "com.jirathip.cooee.shed"] {
        fs::write(mac.home.join(format!("loaded-{item}")), "").unwrap();
    }
    let kept = format!(
        "roots = [\"~/Projects\"]\nport = {}\nallowed_hosts = [\"mac.tailnet.example\"]\n",
        mac.port
    );
    fs::create_dir_all(mac.config().parent().unwrap()).unwrap();
    fs::write(
        mac.config(),
        format!("shed = \"/Applications/Cooee.app/Contents/Helpers/shed\"\n{kept}"),
    )
    .unwrap();
    fs::create_dir_all(mac.data()).unwrap();
    let hash = "ab".repeat(32);
    let devices = format!(
        r#"{{"devices":[{{"id":"abababab","label":"iPhone","hash":"{hash}","created":1,"used":1}}]}}"#
    );
    fs::write(mac.data().join("devices.json"), devices).unwrap();
    let out = mac.setup(&["--yes"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{out:?}");
    assert!(
        stdout.contains("adding the terminal plugin, shed from PATH, not the old Mac app's\n"),
        "{stdout}"
    );
    let config = fs::read_to_string(mac.config()).unwrap();
    assert_eq!(config, format!("{kept}{}", terminal(mac.port, &config)));
    let listed = mac.cooee(&["devices"]);
    let listed = String::from_utf8_lossy(&listed.stdout);
    assert!(
        listed.contains("abababab") && listed.contains("iPhone"),
        "{listed}"
    );
    assert!(mac.home.join("loaded-dev.cooee.cooee").exists());
    assert!(mac.setup(&["--yes"]).status.success());
    assert_eq!(fs::read_to_string(mac.config()).unwrap(), config);
}

#[test]
fn doctor_warns_when_both_the_cli_and_the_mac_app_are_installed() {
    let mac = Mac::new("su-both", true);
    let doctor = |mac: &Mac| {
        let out = mac.cooee(&["doctor", "--config", &mac.config().display().to_string()]);
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    fs::create_dir_all(mac.config().parent().unwrap()).unwrap();
    fs::write(mac.config(), format!("port = {}\n", mac.port)).unwrap();
    fs::write(mac.home.join("loaded-dev.cooee.cooee"), "").unwrap();
    assert!(!doctor(&mac).contains("warn both"));
    fs::write(mac.home.join("loaded-com.jirathip.cooee.serve"), "").unwrap();
    let stdout = doctor(&mac);
    assert!(
        stdout.contains(
            "ok   service dev.cooee.cooee loaded (else: cooee install)\nwarn both dev.cooee.cooee and the Mac \
             app's login items (com.jirathip.cooee.serve) are installed: two servers on one Mac\n"
        ),
        "{stdout}"
    );
}
