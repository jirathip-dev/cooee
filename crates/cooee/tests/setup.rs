//! `cooee setup` with fake `cargo`, `shed`, `launchctl`, `tailscale` and `curl` on `PATH` (each logs its arguments to
//! `$HOME/calls.log` and keeps its state in `$HOME`) and a fake shed server for doctor's hello: a fresh machine runs
//! every step once, in order; a second run skips them all; `--dry-run` writes nothing; a doctor failure stops with its
//! fix; an existing config keeps its keys. Also `scripts/install.sh`, and `cooee update` (#332) against a fake GitHub
//! API (`curl`), with the fake `launchctl kickstart` serving the installed binary that `curl` asks on the port.

mod support;

use std::{
    fs,
    net::TcpListener,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use cooee_core::shed_proto::Hello;
use support::{FakeShed, scratch, version};

const BIN: &str = env!("CARGO_BIN_EXE_cooee");
const REV: &str = "4ac443dbbcb51227dc9df2fb55e2f0834d6a91eb";

const FAKES: &[(&str, &str)] = &[
    (
        "cargo",
        r#"case "$*" in
"install --list") cat "$HOME/cargo.list" 2>/dev/null ;;
*jirathip-dev/cooee\ --rev*) cp "$HOME/new-cooee" "$HOME/.cargo/bin/cooee" ;;
install*) while [ $# -gt 0 ]; do [ "$1" = --rev ] && rev=$2; shift; done
  echo "shed v0.0.0 (https://github.com/jirathip-dev/shed?rev=$rev#${rev%????????????????????????????????}):" >>"$HOME/cargo.list" ;;
esac"#,
    ),
    (
        "shed",
        r#"case "$*" in
install) echo "wrote the job" ;;
ls) test -e "$HOME/loaded-dev.shed.serve" ;;
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
kickstart) cp "$HOME/.cargo/bin/cooee" "$HOME/serving" 2>/dev/null || true ;;
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
*api.github.com*) cat "$HOME/runs.json" ;;
*127.0.0.1*) "$HOME/serving" ;;
*) test ! -e "$HOME/curl-fails" ;;
esac"#,
    ),
];

const CHANGES: &[&str] = &[
    "cargo install --locked",
    "shed install",
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
    fn new(name: &str, dry: bool) -> Self {
        let home = scratch(name);
        let bin = home.join("bin");
        fs::create_dir_all(&bin).unwrap();
        for (name, body) in FAKES {
            let file = bin.join(name);
            let log = format!("printf '%s\\n' \"{name} $*\" >>\"$HOME/calls.log\"");
            fs::write(&file, format!("#!/bin/sh\n{log}\n{body}\n")).unwrap();
            fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let shed = (!dry).then(|| FakeShed::start(&home.join("Library/Caches/shed"), version()));
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
    let mac = Mac::new("su-fresh", false);
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
            format!(
                "cargo install --locked --git https://github.com/jirathip-dev/shed --rev {REV} shed"
            ),
            "shed install".to_owned(),
            format!("launchctl bootstrap {gui} {home}/Library/LaunchAgents/dev.shed.serve.plist"),
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
        "shed 4ac443d",
        "shed service dev.shed.serve",
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

fn old_shed(mac: &Mac) -> (FakeShed, String) {
    let at = mac.home.join("Library/Caches/shed");
    let old = Hello {
        build: "fffffff".to_owned(),
        protocol: 0,
        ..version()
    };
    let (v, b) = (&version().version, &version().build);
    let restart = if cfg!(target_os = "macos") {
        "launchctl kickstart -k gui/$(id -u)/dev.shed.serve"
    } else {
        "shed serve"
    };
    let said = format!(
        "SHED_VERSION: cooee speaks shed {v} ({b}) protocol 1; the server runs {v} (fffffff) protocol 0, an older shed; restart shed serve; \
         fix: cargo install --locked --git https://github.com/jirathip-dev/shed --rev {REV} shed && {restart}"
    );
    fs::write(
        mac.home.join("cargo.list"),
        format!(
            "shed v0.0.0 (https://github.com/jirathip-dev/shed?rev={REV}#{}):\n",
            &REV[..8]
        ),
    )
    .unwrap();
    (FakeShed::start(&at, old), said)
}

#[test]
fn a_managed_shed_on_another_protocol_is_restarted_after_a_warning() {
    let mac = Mac::new("su-shed-old", true);
    let (old, _) = old_shed(&mac);
    fs::write(mac.home.join("loaded-dev.shed.serve"), "").unwrap();
    let log = mac.home.join("calls.log");
    let swap = std::thread::spawn(move || {
        let kicked = || fs::read_to_string(&log).is_ok_and(|l| l.contains("launchctl kickstart"));
        for _ in 0..1000 {
            if kicked() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        old.restart(version())
    });
    let out = mac.setup(&["--yes"]);
    let _new = swap.join().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{out:?}");
    let warn = stdout.find("restarting shed serve ends every running shed session");
    let restart = stdout.find("$ launchctl kickstart");
    assert!(warn.is_some() && warn < restart, "{stdout}");
    let changes = mac.changes();
    let at = |c: &str| changes.iter().position(|l| l.starts_with(c));
    assert!(
        at("shed install") < at("launchctl kickstart -k")
            && at("launchctl kickstart -k") < at("launchctl bootstrap"),
        "{changes:?}"
    );
    assert!(changes[1].ends_with("/dev.shed.serve"), "{changes:?}");
}

#[test]
fn an_unmanaged_shed_on_another_protocol_holds_cooee_back_naming_the_fix() {
    let mac = Mac::new("su-shed-un", true);
    let (_old, said) = old_shed(&mac);
    let out = mac.setup(&["--yes"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains(&said), "{stderr}\nwanted: {said}");
    assert_eq!(mac.changes(), Vec::<String>::new());
}

#[test]
fn dry_run_writes_nothing_and_prints_every_command() {
    let mac = Mac::new("su-dry", true);
    let out = mac.setup(&["--dry-run", "--yes"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(mac.changes(), Vec::<String>::new());
    let fakes: Vec<String> = FAKES.iter().map(|(n, _)| format!("bin/{n}")).collect();
    let mut want = [fakes, vec!["calls.log".to_owned()]].concat();
    want.sort();
    assert_eq!(files(&mac.home), want);
    let port = mac.port;
    for line in [
        format!(
            "would run: cargo install --locked --git https://github.com/jirathip-dev/shed --rev {REV} shed"
        ),
        "would run: shed install".to_owned(),
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
fn a_doctor_failure_stops_with_its_fix() {
    let mac = Mac::new("su-doctor", false);
    fs::write(mac.home.join("curl-fails"), "").unwrap();
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
    let mac = Mac::new("su-keep", false);
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
    let mac = Mac::new("su-themes", false);
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

#[test]
fn install_sh_checks_its_tools_then_installs_cooee_and_runs_setup() {
    let home = scratch("install-sh");
    let bin = home.join("bin");
    fs::create_dir_all(&bin).unwrap();
    for name in ["cargo", "cooee", "codesign"] {
        let file = bin.join(name);
        fs::write(
            &file,
            format!("#!/bin/sh\nprintf '%s\\n' \"{name} $*\" >>\"$HOME/calls.log\"\n"),
        )
        .unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).unwrap();
    }
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
    let out = run();
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("tailscale not found"),
        "{out:?}"
    );
    assert!(!home.join("calls.log").exists());

    fs::copy(bin.join("cooee"), bin.join("tailscale")).unwrap();
    let crate_dir = script.parent().unwrap().join("../crates/cooee");
    let install = format!("cargo install --locked --path {}\n", crate_dir.display());
    let mut sign = String::new();
    if cfg!(target_os = "macos") {
        let security = bin.join("security");
        fs::write(&security, "#!/bin/sh\necho '  0 valid identities found'\n").unwrap();
        fs::set_permissions(&security, fs::Permissions::from_mode(0o755)).unwrap();
        let out = run();
        assert_eq!(out.status.code(), Some(1), "{out:?}");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            err.contains("no \"Apple Development\" signing identity"),
            "{err}"
        );
        assert_eq!(fs::read_to_string(home.join("calls.log")).unwrap(), install);
        fs::remove_file(home.join("calls.log")).unwrap();
        let id = "Apple Development: Test (ABCDE12345)";
        let found = format!("#!/bin/sh\necho '  1) 0123 \"{id}\"'\n");
        fs::write(&security, found).unwrap();
        let cooee = home.join(".cargo/bin/cooee");
        sign = format!(
            "codesign --force --sign {id} --identifier cooee {}\n",
            cooee.display()
        );
    }
    let out = run();
    assert!(out.status.success(), "{out:?}");
    assert_eq!(
        fs::read_to_string(home.join("calls.log")).unwrap(),
        format!("{install}{sign}cooee setup --instance drover\n")
    );
}

const SHA: &str = "2222222222222222222222222222222222222222";

fn updating(name: &str, conclusion: &str, new: &str) -> Mac {
    let mac = Mac::new(name, true);
    let exe = |file: &Path, body: &str| {
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(file, fs::Permissions::from_mode(0o755)).unwrap();
    };
    let id = "Apple Development: Test (ABCDE12345)";
    exe(
        &mac.home.join("bin/security"),
        &format!("echo '  1) 0123 \"{id}\"'"),
    );
    exe(
        &mac.home.join("bin/codesign"),
        "printf '%s\\n' \"codesign $*\" >>\"$HOME/calls.log\"",
    );
    exe(&mac.home.join(".cargo/bin/cooee"), "echo old");
    exe(&mac.home.join("serving"), "echo old");
    exe(&mac.home.join("new-cooee"), new);
    fs::write(mac.home.join("main.json"), format!(r#"{{"sha":"{SHA}"}}"#)).unwrap();
    let run =
        format!(r#"{{"workflow_runs":[{{"conclusion":"{conclusion}","head_sha":"{SHA}"}}]}}"#);
    fs::write(mac.home.join("runs.json"), run).unwrap();
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
}

#[test]
fn update_check_says_what_it_would_install_and_changes_nothing() {
    let mac = updating("up-check", "success", "echo new");
    fs::write(mac.home.join("calls.log"), "").unwrap();
    let before = files(&mac.home);
    let out = mac.cooee(&["update", "--check"]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        format!(
            "would install {SHA}, restart dev.cooee.cooee and check that port {} answers\n",
            mac.port
        )
    );
    let calls = fs::read_to_string(mac.home.join("calls.log")).unwrap();
    assert!(calls.lines().all(|l| l.starts_with("curl ")), "{calls}");
    assert_eq!(files(&mac.home), before);
    assert_eq!(
        (mac.prints(".cargo/bin/cooee"), mac.prints("serving")),
        ("old\n".into(), "old\n".into())
    );
}

#[test]
fn update_installs_a_new_green_head_and_doctor_shows_it() {
    let mac = updating("up-green", "success", "echo new");
    let out = mac.cooee(&["update"]);
    assert!(out.status.success(), "{out:?}");
    let calls = fs::read_to_string(mac.home.join("calls.log")).unwrap();
    assert!(
        calls.contains(&format!(
            "cargo install --locked --git https://github.com/jirathip-dev/cooee --rev {SHA} cooee\n"
        )),
        "{calls}"
    );
    assert!(calls.contains("launchctl kickstart -k gui/"), "{calls}");
    assert_eq!(mac.prints("serving"), "new\n");
    let log = fs::read_to_string(mac.data().join("update.log")).unwrap();
    assert!(log.ends_with(&format!(" installed {SHA}\n")), "{log}");
    let out = mac.cooee(&["doctor"]);
    assert!(
        String::from_utf8_lossy(&out.stdout)
            .contains("info update: installed unknown, main's green head 2222222, last check 0 min ago, auto-update on\n"),
        "{out:?}"
    );
}

#[test]
fn a_red_head_or_auto_update_off_installs_nothing() {
    let mac = updating("up-red", "failure", "echo new");
    let out = mac.cooee(&["update"]);
    assert!(out.status.success(), "{out:?}");
    let log = fs::read_to_string(mac.data().join("update.log")).unwrap();
    assert!(
        log.ends_with(" main 2222222: ci failure, not installed\n"),
        "{log}"
    );
    fs::write(
        mac.config(),
        format!("port = {}\nauto_update = false\n", mac.port),
    )
    .unwrap();
    fs::write(
        mac.home.join("runs.json"),
        r#"{"workflow_runs":[{"conclusion":"success"}]}"#,
    )
    .unwrap();
    let out = mac.cooee(&["update"]);
    assert!(
        String::from_utf8_lossy(&out.stdout).starts_with("auto-update is off"),
        "{out:?}"
    );
    assert!(
        !fs::read_to_string(mac.home.join("calls.log"))
            .unwrap()
            .contains("install")
    );
    assert_eq!(mac.prints(".cargo/bin/cooee"), "old\n");
}

#[test]
fn a_new_build_that_does_not_answer_is_rolled_back_and_the_old_one_serves_again() {
    let mac = updating("up-rollback", "success", "exit 1");
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
        (mac.prints(".cargo/bin/cooee"), mac.prints("serving")),
        ("old\n".into(), "old\n".into())
    );
    assert_eq!(
        fs::read_to_string(mac.data().join("update.failed")).unwrap(),
        SHA
    );
    fs::write(mac.home.join("calls.log"), "").unwrap();
    assert!(mac.cooee(&["update"]).status.success());
    let log = fs::read_to_string(mac.data().join("update.log")).unwrap();
    assert!(
        log.ends_with(" main 2222222 failed to install before: it waits for a newer one\n"),
        "{log}"
    );
    assert!(
        !fs::read_to_string(mac.home.join("calls.log"))
            .unwrap()
            .contains("cargo install")
    );
}

#[test]
fn setup_on_a_mac_whose_app_ran_the_server_reuses_its_config_and_data() {
    let mac = Mac::new("su-app", false);
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
    let mac = Mac::new("su-both", false);
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
