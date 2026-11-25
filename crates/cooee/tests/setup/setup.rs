use std::{
    fs,
    net::TcpListener,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use crate::support::{FakeShed, scratch, version};

pub const BIN: &str = env!("CARGO_BIN_EXE_cooee");

pub const FAKES: &[(&str, &str)] = &[
    (
        "shed",
        r#"case "$*" in
"agents add claude-acp") echo '[{"id":"claude-acp","version":"1","distribution":"npx","cmd":[]}]' >"$HOME/Library/Caches/shed/agents.json" ;;
*) exit 2 ;;
esac"#,
    ),
    (
        "launchctl",
        r#"case "$1" in
list) test -e "$HOME/loaded-$2" ;;
print) test -e "$HOME/loaded-${2##*/}" ;;
bootstrap) touch "$HOME/loaded-$(basename "$3" .plist)" ;;
bootout) rm -f "$HOME/loaded-${2##*/}" ;;
kickstart) cp "$HOME/.local/bin/cooee" "$HOME/serving" 2>/dev/null || true ;;
esac"#,
    ),
    (
        "systemctl",
        r#"case "$2" in
show) shift 8; for u; do [ -e "$HOME/loaded-${u%.*}" ] && s=active || s=inactive; printf 'ActiveState=%s\nMainPID=0\nResult=success\n\n' $s; done ;;
enable) touch "$HOME/loaded-${4%.*}" ;;
disable) rm -f "$HOME/loaded-${4%.*}" ;;
restart) cp "$HOME/.local/bin/cooee" "$HOME/serving" 2>/dev/null || true ;;
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
*127.0.0.1*) "$HOME/serving" >/dev/null && exec /usr/bin/curl "$@" ;;
*) [ -e "$HOME/tailnet" ] || exit 0
   for a; do shift; case $a in https://*) a="http://$(cat "$HOME/tailnet")/${a#https://*/}" ;; esac; set -- "$@" "$a"; done
   exec /usr/bin/curl "$@" ;;
esac"#,
    ),
];

pub const CHANGES: &[&str] = &[
    "shed agents add",
    "launchctl bootstrap",
    "launchctl kickstart",
    "launchctl bootout",
    "systemctl --user enable",
    "systemctl --user restart",
    "systemctl --user disable",
    "tailscale serve --bg",
];

pub const LAUNCHD: bool = cfg!(target_os = "macos");

pub const RESTART: &str = if LAUNCHD {
    "launchctl kickstart -k gui/"
} else {
    "systemctl --user restart dev.cooee.cooee.service"
};

pub fn job_file(home: &Path, label: &str, unit: &str) -> PathBuf {
    if LAUNCHD {
        home.join(format!("Library/LaunchAgents/{label}.plist"))
    } else {
        home.join(format!(".config/systemd/user/{label}.{unit}"))
    }
}

pub fn load_line(home: &Path, label: &str, unit: &str) -> String {
    let uid = Command::new("id").arg("-u").output().unwrap().stdout;
    if LAUNCHD {
        let file = job_file(home, label, unit);
        let uid = String::from_utf8(uid).unwrap();
        format!("launchctl bootstrap gui/{} {}", uid.trim(), file.display())
    } else {
        format!("systemctl --user enable --now {label}.{unit}")
    }
}

pub struct Mac {
    pub home: PathBuf,
    pub port: u16,
    pub listener: TcpListener,
    pub _shed: Option<FakeShed>,
}

impl Mac {
    pub fn new(name: &str, shed: bool) -> Self {
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
        if let Some(shed) = &shed {
            fs::write(shed.dir.join("agents.json"), "[]").unwrap();
        }
        let listener = std::iter::repeat_with(|| TcpListener::bind("127.0.0.1:0").unwrap())
            .find(|l| l.local_addr().unwrap().port() <= u16::MAX - 1000)
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        Self {
            home,
            port,
            listener,
            _shed: shed,
        }
    }

    pub fn setup(&self, args: &[&str]) -> Output {
        let chief = ["--chief-agent", "claude-acp", "--chief-machine", "here"];
        self.cooee(&[&["setup", "--port", &self.port.to_string()], args, &chief].concat())
    }

    pub fn chief(&self) -> PathBuf {
        self.config().with_file_name("agents/chief/agent.toml")
    }

    pub fn cooee(&self, args: &[&str]) -> Output {
        self.cooee_typing(args, None)
    }

    pub fn cooee_typing(&self, args: &[&str], input: Option<&str>) -> Output {
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

    pub fn changes(&self) -> Vec<String> {
        let log = self.home.join("calls.log");
        let calls = fs::read_to_string(&log).unwrap_or_default();
        fs::write(&log, "").unwrap();
        calls
            .lines()
            .filter(|l| CHANGES.iter().any(|c| l.starts_with(c)))
            .map(str::to_owned)
            .collect()
    }

    pub fn config(&self) -> PathBuf {
        self.home.join(".config/cooee/cooee/config.toml")
    }
}

pub fn files(dir: &Path) -> Vec<String> {
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

pub fn terminal(port: u16, config: &str) -> String {
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

pub fn exe(file: &Path, body: &str) {
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(file, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(file, fs::Permissions::from_mode(0o755)).unwrap();
}

pub fn asset() -> String {
    let os = if cfg!(target_os = "macos") {
        "apple-darwin"
    } else {
        "unknown-linux-gnu"
    };
    format!("cooee-{}-{os}", std::env::consts::ARCH)
}

pub fn sums(file: &Path) -> String {
    format!(
        "{}  {}\n",
        cooee_core::sha256(&fs::read(file).unwrap()),
        asset()
    )
}

pub const SHA: &str = "2222222222222222222222222222222222222222";

pub fn says(sha: &str) -> String {
    format!("echo \"cooee 0.1.0 ({sha})\"")
}

pub fn updating(name: &str, released: bool, new: &str) -> Mac {
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
    fs::write(mac.home.join("loaded-dev.cooee.cooee"), "").unwrap();
    let server = mac.listener.try_clone().unwrap();
    std::thread::spawn(move || {
        for mut conn in server.incoming().flatten() {
            let mut buf = [0; 4096];
            let n = std::io::Read::read(&mut conn, &mut buf).unwrap();
            let status = if buf[..n].starts_with(b"GET /pair ") {
                "405 Method Not Allowed"
            } else {
                "401 Unauthorized"
            };
            let reply =
                format!("HTTP/1.1 {status}\r\ncontent-length: 0\r\nconnection: close\r\n\r\n");
            std::io::Write::write_all(&mut conn, reply.as_bytes()).unwrap();
        }
    });
    mac
}

impl Mac {
    pub fn data(&self) -> PathBuf {
        self.home.join(".local/share/cooee/cooee")
    }

    pub fn prints(&self, name: &str) -> String {
        let out = Command::new(self.home.join(name)).output().unwrap();
        String::from_utf8(out.stdout).unwrap()
    }

    pub fn log(&self) -> String {
        fs::read_to_string(self.data().join("update.log")).unwrap()
    }
}
