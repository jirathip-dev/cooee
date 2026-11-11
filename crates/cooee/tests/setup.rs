//! `cooee setup` with fake `cargo`, `shed`, `launchctl`, `tailscale` and `curl` on `PATH` (each logs its arguments to
//! `$HOME/calls.log` and keeps its state in `$HOME`) and a fake shed server for doctor's hello: a fresh machine runs
//! every step once, in order; a second run skips them all; `--dry-run` writes nothing; a doctor failure stops with its
//! fix; an existing config keeps its keys. Also `scripts/install.sh`.

mod support;

use std::{
    fs,
    net::TcpListener,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use support::{FakeShed, scratch, version};

const BIN: &str = env!("CARGO_BIN_EXE_cooee");
const REV: &str = "f026a4d35a2705e560a52515da8e3a5d32e4d659";

const FAKES: &[(&str, &str)] = &[
    (
        "cargo",
        r#"case "$*" in
"install --list") cat "$HOME/cargo.list" 2>/dev/null ;;
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
    ("curl", r#"test ! -e "$HOME/curl-fails""#),
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
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
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
            format!("tailscale serve --bg --https={port} http://127.0.0.1:{port}"),
        ]
    );
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
        "shed f026a4d",
        "shed service dev.shed.serve",
        &format!("config {}", mac.config().display()),
        "agents",
        "cooee service dev.cooee.cooee",
        &format!("tailscale serve {port}"),
    ] {
        assert!(
            stdout.contains(&format!("{step}: done, skipped\n")),
            "{step} not skipped in {stdout}"
        );
    }
    assert!(
        stdout.contains("pair your phone:\nScan it") && stdout.contains("\ncode  "),
        "{stdout}"
    );
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
fn setup_installs_the_first_party_themes_chosen_from_the_index() {
    let mac = Mac::new("su-themes", false);
    let repo = mac.home.join("packs.git");
    fs::create_dir_all(&repo).unwrap();
    let packs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/themes");
    for t in ["farm", "kitchen", "garden"] {
        fs::create_dir_all(repo.join(t)).unwrap();
        for f in ["theme.json", "sprites.svg", "scene.svg", "icon.png"] {
            fs::copy(packs.join(t).join(f), repo.join(t).join(f)).unwrap();
        }
    }
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "{out:?}");
        String::from_utf8(out.stdout).unwrap().trim().to_owned()
    };
    git(&["init", "--quiet", "-b", "main"]);
    git(&["add", "-A"]);
    git(&["commit", "--quiet", "-m", "packs"]);
    let commit = git(&["rev-parse", "HEAD"]);
    let entry = |name: &str, first: bool| {
        format!(
            "[[theme]]\nname = \"{name}\"\nurl = {:?}\npath = \"{name}\"\ncommit = \"{commit}\"\n\
             description = \"x\"\nfirst_party = {first}\n\n",
            repo.display().to_string()
        )
    };
    let index = mac.home.join("index.toml");
    let text = entry("farm", true) + &entry("kitchen", true) + &entry("garden", false);
    fs::write(&index, text).unwrap();
    fs::create_dir_all(mac.config().parent().unwrap()).unwrap();
    fs::write(
        mac.config(),
        format!(
            "port = {}\nroots = [\"/srv\"]\nplugin_index = {:?}\n",
            mac.port,
            index.display().to_string()
        ),
    )
    .unwrap();
    let args = ["setup", "--port", &mac.port.to_string()];
    let out = mac.cooee_typing(&args, Some("kitchen\n\n\n"));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{out:?}");
    assert!(
        stdout.contains(
            "themes:\n  themes to install, or none [farm kitchen]:\n  installed theme kitchen\n"
        ),
        "{stdout}"
    );
    let themes = mac.config().parent().unwrap().join("themes");
    assert_eq!(
        files(&themes),
        [
            "kitchen/icon.png",
            "kitchen/scene.svg",
            "kitchen/sprites.svg",
            "kitchen/theme.json"
        ]
    );
    let lock: serde_json::Value = serde_json::from_slice(
        &fs::read(mac.home.join(".local/share/cooee/cooee/themes.lock")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        lock,
        serde_json::json!([{ "name": "kitchen", "source": "index", "commit": commit, "listed": true }])
    );
    let out = mac.setup(&["--yes"]);
    assert!(out.status.success(), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stdout).contains("installed theme farm\n"));
    assert!(themes.join("farm/theme.json").exists() && !themes.join("garden").exists());
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
            .args(["--instance", "cooee-drover"])
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
        format!("{install}{sign}cooee setup --instance cooee-drover\n")
    );
}
