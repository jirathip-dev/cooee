use std::{
    env, fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde_json::Value;

use super::{Events, is_item};

/// A scratch folder, empty.
pub fn scratch(name: &str) -> PathBuf {
    let dir = cooee_core::scratch::owned(
        env::temp_dir().join(format!("cooee-{name}-{}", std::process::id())),
    );
    fs::remove_dir_all(&dir).ok();
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Agent `id`'s folder under config folder `dir` (#359): `agent.toml` holding `toml`, and `INSTRUCTIONS.md`.
pub fn agent(dir: &Path, id: &str, toml: &str, instructions: &str) -> PathBuf {
    let home = dir.join("agents").join(id);
    fs::create_dir_all(&home).unwrap();
    fs::write(home.join("agent.toml"), toml).unwrap();
    fs::write(home.join("INSTRUCTIONS.md"), instructions).unwrap();
    home
}

/// `cooee serve` on a free port with data in `dir/data`, the fake shed in `shed`, the root `dir/root` unless `extra`
/// sets `roots`, its stderr in `dir/cooee.err` and no providers unless `extra` names some; paired (`cookie` goes with every request).
pub struct Cooee {
    pub child: Child,
    pub addr: String,
    pub config: PathBuf,
    pub cookie: String,
    /// Run as `--instance <name>` with `HOME` here and no `--config`, if set.
    pub instance: Option<(String, PathBuf)>,
}

impl Cooee {
    pub fn start(dir: &Path, shed: &Path) -> Self {
        Self::start_with(dir, shed, "")
    }

    /// [`Cooee::start`] with more config lines.
    pub fn start_with(dir: &Path, shed: &Path, extra: &str) -> Self {
        Self::start_env(dir, shed, extra, &[])
    }

    /// [`Cooee::start_with`] with these variables set for `cooee serve`.
    pub fn start_env(dir: &Path, shed: &Path, extra: &str, env: &[(&str, &str)]) -> Self {
        let data = format!("data = {:?}\n{extra}", dir.join("data").to_string_lossy());
        Self::launch(dir, shed, dir.join("config.toml"), &data, None, env)
    }

    /// `cooee serve --instance <name>` with `HOME` at `dir/home`: its config at the instance's default path, its
    /// data in the instance's default folder.
    pub fn start_instance(dir: &Path, shed: &Path, name: &str) -> Self {
        let home = dir.join("home");
        let config = home.join(".config/cooee").join(name).join("config.toml");
        fs::create_dir_all(config.parent().unwrap()).unwrap();
        Self::launch(dir, shed, config, "", Some((name.to_owned(), home)), &[])
    }

    pub fn command(&self) -> Command {
        Self::command_for(&self.config, self.instance.as_ref())
    }

    fn command_for(config: &Path, instance: Option<&(String, PathBuf)>) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_cooee"));
        match instance {
            Some((name, home)) => c
                .args(["--instance", name])
                .env("HOME", home)
                .env_remove("XDG_CONFIG_HOME")
                .env_remove("XDG_DATA_HOME"),
            None => c.arg("--config").arg(config),
        };
        c
    }

    fn launch(
        dir: &Path,
        shed: &Path,
        config: PathBuf,
        extra: &str,
        instance: Option<(String, PathBuf)>,
        env: &[(&str, &str)],
    ) -> Self {
        let root = dir.join("root");
        fs::create_dir_all(root.join("project")).unwrap();
        let q = |p: &Path| format!("{:?}", p.to_string_lossy());
        let providers = if extra.contains("[[providers]]") {
            ""
        } else {
            "providers = []\n"
        };
        let roots = if extra.contains("roots =") {
            String::new()
        } else {
            format!("roots = [{}]\n", q(&root))
        };
        let toml = format!(
            "port = 0
{roots}shed = {}
shed_dir = {}
{providers}{extra}",
            q(&shed.join("shed")),
            q(shed),
        );
        fs::write(&config, toml).unwrap();
        let mut child = Self::command_for(&config, instance.as_ref())
            .arg("serve")
            .envs(env.iter().copied())
            .stdout(Stdio::piped())
            .stderr(fs::File::create(dir.join("cooee.err")).unwrap())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let addr = line
            .trim()
            .strip_prefix("cooee: listening on http://")
            .unwrap_or_else(|| panic!("{line}"))
            .to_owned();
        let mut cooee = Self {
            child,
            addr,
            config,
            cookie: String::new(),
            instance,
        };
        let token = cooee.pair().expect("pairing");
        cooee.cookie = format!("cooee={token}");
        cooee
    }

    /// Runs `cooee <args>` on our config or instance; its stdout, which must succeed.
    pub fn cli(&self, args: &[&str]) -> String {
        let out = self.command().args(args).output().unwrap();
        assert!(out.status.success(), "{out:?}");
        String::from_utf8(out.stdout).unwrap()
    }

    /// `cooee pair`, then its code entered as a browser would: the new device's token, if it paired.
    pub fn pair(&self) -> Option<String> {
        let out = self.cli(&["pair"]);
        let code: String = out
            .lines()
            .find_map(|l| l.strip_prefix("code  "))?
            .chars()
            .take(7)
            .filter(char::is_ascii_digit)
            .collect();
        let (head, _) = self.raw(
            "POST",
            "/pair",
            &[("Content-Type", "application/x-www-form-urlencoded")],
            &format!("code={code}"),
        );
        head.lines()
            .find_map(|l| l.strip_prefix("set-cookie: cooee="))
            .map(|c| c.split(';').next().unwrap().to_owned())
    }

    /// One request with exactly these headers (and `Host: <addr>` unless given): the status line and headers
    /// (lowercased names), and the body.
    pub fn raw(
        &self,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> (String, String) {
        let mut stream = TcpStream::connect(&self.addr).unwrap();
        let mut request = Vec::new();
        write!(request, "{method} {path} HTTP/1.0\r\n").unwrap();
        if !headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("host")) {
            write!(request, "Host: {}\r\n", self.addr).unwrap();
        }
        for (k, v) in headers {
            write!(request, "{k}: {v}\r\n").unwrap();
        }
        write!(request, "Content-Length: {}\r\n\r\n{body}", body.len()).unwrap();
        stream.write_all(&request).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        let (head, body) = response.split_once("\r\n\r\n").unwrap();
        let head = head
            .lines()
            .map(|l| match l.split_once(": ") {
                Some((k, v)) => format!("{}: {v}", k.to_ascii_lowercase()),
                None => l.to_owned(),
            })
            .collect::<Vec<_>>()
            .join("\n");
        (head, body.to_owned())
    }

    pub fn request(&self, method: &str, path: &str, body: Option<&Value>) -> (u16, String) {
        let body = body.map(Value::to_string).unwrap_or_default();
        let headers = [
            ("Content-Type", "application/json"),
            ("Cookie", self.cookie.as_str()),
        ];
        let (head, body) = self.raw(method, path, &headers, &body);
        (head[9..12].parse().unwrap(), body)
    }

    /// GET `path` as text.
    pub fn get(&self, path: &str) -> (u16, String) {
        self.request("GET", path, None)
    }

    /// GET `path` as JSON, which must succeed.
    pub fn json(&self, path: &str) -> Value {
        let (status, body) = self.get(path);
        assert_eq!(status, 200, "{path}: {body}");
        serde_json::from_str(&body).unwrap()
    }

    /// POST `body` to `path`.
    pub fn post(&self, path: &str, body: &Value) -> (u16, String) {
        self.request("POST", path, Some(body))
    }

    /// Opens thread `id`'s event stream, after `cursor` (sent as `Last-Event-ID` when `header`).
    pub fn events(&self, id: &str, cursor: &str, header: bool) -> Events {
        let mut stream = TcpStream::connect(&self.addr).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(20)))
            .unwrap();
        let (path, extra) = if header {
            (
                format!("/threads/{id}/events"),
                format!("Last-Event-ID: {cursor}\r\n"),
            )
        } else {
            (
                format!("/threads/{id}/events?since={cursor}"),
                String::new(),
            )
        };
        write!(
            stream,
            "GET {path} HTTP/1.0\r\nHost: {}\r\nCookie: {}\r\n{extra}\r\n",
            self.addr, self.cookie
        )
        .unwrap();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        while reader.read_line(&mut line).unwrap() > 2 {
            line.clear();
        }
        Events(reader)
    }
}

impl Drop for Cooee {
    fn drop(&mut self) {
        if !matches!(self.child.try_wait(), Ok(None)) {
            return;
        }
        Command::new("kill")
            .args(["-TERM", &self.child.id().to_string()])
            .status()
            .ok();
        for _ in 0..100 {
            if !matches!(self.child.try_wait(), Ok(None)) {
                return;
            }
            thread::sleep(Duration::from_millis(50));
        }
        self.child.kill().ok();
        self.child.wait().ok();
    }
}

/// The longest a test plugin's call hangs.
pub const HANG: Duration = Duration::from_secs(10);

/// #288: while a call for another thread hangs, thread `other` goes on: its send, the thread list and the home page
/// each answer well before the hang's [`HANG`] is up, and its reply streams in.
pub fn others_go_on(cooee: &Cooee, other: &str) {
    let cursor = cooee.json(&format!("/threads/{other}/messages"))["cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut events = cooee.events(other, &cursor, false);
    let (path, send) = (
        format!("/threads/{other}/send"),
        serde_json::json!({ "text": "Say hi again", "random_id": "go-on" }),
    );
    let quick = |what: &str, call: &dyn Fn() -> (u16, String)| {
        let at = Instant::now();
        let (status, body) = call();
        let took = at.elapsed();
        assert_eq!(status, 200, "{what}: {body}");
        assert!(took < HANG / 2, "{what} took {took:?}");
    };
    quick("send", &|| cooee.post(&path, &send));
    quick("list", &|| cooee.get("/threads"));
    quick("messages", &|| {
        cooee.get(&format!("/threads/{other}/messages"))
    });
    events.until(|e| is_item(e, "agent"));
}
