use std::{
    env,
    io::Read,
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

use serde::{Deserialize, Serialize};

use crate::{Provider, accounts::Usage};

/// A provider as data: what `[[providers]]` sets and each provider module's defaults return.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    /// Its name on the sheet; also part of its login terminal's name and its accounts' folder.
    pub name: String,
    /// The ACP agent id it signs in, for the new-thread picker.
    pub agent: String,
    /// The login command, run in a shed terminal.
    pub login: Vec<String>,
    /// The status command: exit 0 when signed in; its output is the status line.
    pub status: Vec<String>,
    /// The line that installs its CLI, shown when the login command isn't found.
    #[serde(default)]
    pub install: String,
    /// The variable that points its CLI at a config folder: each added account is one.
    /// Without it the provider has only the system default account.
    #[serde(default)]
    pub home: String,
    /// The logout command.
    #[serde(default)]
    pub logout: Vec<String>,
    /// The variable its ACP adapter reads an extra settings file from and enforces, hooks included. Without it an
    /// agent profile with `settings` can't start with its agent.
    #[serde(default)]
    pub settings: String,
    /// The usage command (#183), its output read by the provider's module. None: no usage.
    #[serde(default)]
    pub usage: Vec<String>,
    /// The efforts a thread may pick (#553 row 34; its models are the agent's own, #193).
    #[serde(default)]
    pub efforts: Vec<String>,
    /// The words that start its agent on an effort: `{}` is the pick; a `NAME=value` word goes to its environment,
    /// any other after its command.
    #[serde(default)]
    pub effort: Vec<String>,
    /// The registered provider whose behaviour it gets (default: the one named `name`, else none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
}

/// One command's run: whether it exited 0, its stdout and its stderr.
#[derive(Debug, Clone, Default)]
pub struct Ran {
    /// It exited 0.
    pub ok: bool,
    /// Its stdout.
    pub stdout: String,
    /// Its stderr.
    pub stderr: String,
}

impl dyn Provider + '_ {
    /// The words that start its agent on `effort` (empty: its own default); `None` when it offers no such pick.
    #[must_use]
    pub fn picks(&self, effort: &str) -> Option<Vec<String>> {
        if effort.is_empty() {
            return Some(Vec::new());
        }
        let s = self.spec();
        let words = s.effort.iter().map(|w| w.replace("{}", effort));
        s.efforts
            .iter()
            .any(|o| o == effort)
            .then(|| words.collect())
    }

    /// The variable and folder that point its CLI at the account in `home` (none for the system default).
    #[must_use]
    pub fn env<'a>(&'a self, home: Option<&'a Path>) -> Option<(&'a str, &'a Path)> {
        home.filter(|_| !self.spec().home.is_empty())
            .map(|dir| (self.spec().home.as_str(), dir))
    }

    /// `cmd` for the account in `home`: `env <variable>=<folder> cmd…`, as is for the system default.
    #[must_use]
    pub fn with_home(&self, cmd: &[String], home: Option<&Path>) -> Vec<String> {
        match self.env(home) {
            Some((var, dir)) => [
                vec!["env".to_owned(), format!("{var}={}", dir.display())],
                cmd.to_vec(),
            ]
            .concat(),
            None => cmd.to_vec(),
        }
    }

    /// Whether its CLI (the login command's program) is missing.
    #[must_use]
    pub fn missing(&self) -> bool {
        self.spec().login.first().is_some_and(|b| !installed(b))
    }

    fn run(&self, cmd: &[String], home: Option<&Path>) -> Result<Option<Ran>, String> {
        let Some((bin, args)) = cmd.split_first() else {
            return Err("no command".to_owned());
        };
        let mut command = Command::new(bin);
        for var in self.scrub() {
            command.env_remove(var);
        }
        if let Some((var, dir)) = self.env(home) {
            command.env(var, dir);
        }
        #[cfg(unix)]
        command.process_group(0);
        let child = command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        let mut child = child.map_err(|e| format!("{bin}: {e}"))?;
        let drain = |r: Option<Box<dyn Read + Send>>| {
            thread::spawn(move || {
                let mut s = String::new();
                r.map(|mut r| r.read_to_string(&mut s));
                s
            })
        };
        let stdout = drain(child.stdout.take().map(|r| Box::new(r) as _));
        let stderr = drain(child.stderr.take().map(|r| Box::new(r) as _));
        let deadline = Instant::now() + LIMIT;
        let ok = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status.success(),
                Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(20)),
                _ => {
                    kill_group(&mut child);
                    let _ = child.wait();
                    return Ok(None);
                }
            }
        };
        Ok(Some(Ran {
            ok,
            stdout: stdout.join().unwrap_or_default(),
            stderr: stderr.join().unwrap_or_default(),
        }))
    }

    /// Runs the logout command for the account in `home`.
    ///
    /// # Errors
    /// Its output on one line, redacted, when it fails or can't run.
    pub fn sign_out(&self, home: Option<&Path>) -> Result<(), String> {
        let logout = &self.spec().logout;
        match self.run(logout, home)? {
            Some(r) if r.ok => Ok(()),
            Some(r) => Err(one_line(if r.stderr.trim().is_empty() {
                &r.stdout
            } else {
                &r.stderr
            })),
            None => Err(format!("`{}` timed out", logout.join(" "))),
        }
    }

    /// Runs the status command (10 s at most) for the account in `home` and reads it; unknown when it times out.
    #[must_use]
    pub fn check(&self, home: Option<&Path>) -> Status {
        if self.missing() {
            return Status {
                install: Some(self.spec().install.clone()),
                ..Status::said(false, "not installed on this Mac")
            };
        }
        match self.run(&self.spec().status, home) {
            Ok(Some(r)) => self.read_status(&r),
            Ok(None) => Status::unknown(TIMED_OUT),
            Err(e) => Status::said(false, &e),
        }
    }

    /// The usage command's reading at `now` (s) for the account in `home`; none when it fails or reads no window.
    #[must_use]
    pub fn usage(&self, home: Option<&Path>, now: u64) -> Option<Usage> {
        let cmd = &self.spec().usage;
        if cmd.is_empty() {
            return None;
        }
        match self.run(cmd, home) {
            Ok(Some(r)) if r.ok => self.read_usage(&r.stdout, now),
            _ => None,
        }
    }
}

pub(crate) fn plain_status(ran: &Ran) -> Status {
    let out = if ran.stdout.trim().is_empty() {
        &ran.stderr
    } else {
        &ran.stdout
    };
    Status::said(ran.ok, out)
}

/// An account's signed-in state, from its provider's status command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Status {
    /// The status command exited 0; when `unknown`, as it was last known.
    pub signed_in: bool,
    /// Can't tell: the status command timed out, or hasn't run yet; `line` says which.
    pub unknown: bool,
    /// Its output on one line, redacted, at most 80 characters.
    pub line: String,
    /// The install line, when the login command isn't found.
    pub install: Option<String>,
    /// The email or org it is signed in as, when the status says.
    pub who: Option<String>,
    /// Its usage reading, when it is signed in and its provider has a usage command.
    pub usage: Option<Usage>,
}

const TIMED_OUT: &str = "can't tell: the status command timed out";

impl Status {
    /// An unknown status that says `why`, not signed in.
    #[must_use]
    pub fn unknown(why: &str) -> Self {
        Self {
            signed_in: false,
            unknown: true,
            line: why.to_owned(),
            install: None,
            who: None,
            usage: None,
        }
    }

    /// A known status, `line` on one line and redacted.
    #[must_use]
    pub fn said(signed_in: bool, line: &str) -> Self {
        Self {
            signed_in,
            unknown: false,
            line: one_line(line),
            ..Self::unknown("")
        }
    }
}

const LIMIT: Duration = Duration::from_secs(if cfg!(test) { 2 } else { 10 });

fn kill_group(child: &mut Child) {
    #[cfg(unix)]
    {
        let group = format!("-{}", child.id());
        let kill = |sig| {
            Command::new("kill")
                .args([sig, "--", &group])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
        };
        let _ = kill("-TERM");
        thread::sleep(Duration::from_millis(200));
        let _ = kill("-KILL");
    }
    let _ = child.kill();
}

pub(crate) fn one_line(text: &str) -> String {
    redact(&text.split_whitespace().collect::<Vec<_>>().join(" "))
        .chars()
        .take(80)
        .collect()
}

fn installed(bin: &str) -> bool {
    let path = Path::new(bin);
    if path.components().count() > 1 {
        return path.is_file();
    }
    env::var_os("PATH").is_some_and(|p| env::split_paths(&p).any(|d| d.join(bin).is_file()))
}

fn tokenish(c: char) -> bool {
    c.is_ascii_alphanumeric() || "-_.+=~".contains(c)
}

/// `text` with anything that looks like a token replaced by `[redacted]`: a run of 20 or more token characters
/// with both a letter and a digit, and in a link the values of `token`, `secret`, `password` and `key` parameters.
#[must_use]
pub fn redact(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for word in text.split_inclusive(char::is_whitespace) {
        let (word, space) = word.split_at(word.trim_end().len());
        if word.starts_with("https://") || word.starts_with("http://") {
            for part in word.split_inclusive(['?', '&', '#']) {
                match part.split_once('=') {
                    Some((k, v))
                        if ["token", "secret", "password", "key"]
                            .iter()
                            .any(|s| k.to_ascii_lowercase().contains(s)) =>
                    {
                        let end = v.trim_end_matches(['&', '#']);
                        out.extend([k, "=[redacted]", &v[end.len()..]]);
                    }
                    _ => out.push_str(part),
                }
            }
        } else {
            let mut run = String::new();
            for c in word.chars().chain(['\n']) {
                if tokenish(c) {
                    run.push(c);
                    continue;
                }
                let token = run.len() >= 20
                    && run.contains(|c: char| c.is_ascii_digit())
                    && run.contains(|c: char| c.is_ascii_alphabetic());
                out.push_str(if token { "[redacted]" } else { &run });
                run.clear();
                if c != '\n' {
                    out.push(c);
                }
            }
        }
        out.push_str(space);
    }
    out
}

/// The `http(s)://` links in `text`, in order, without the punctuation around them.
#[must_use]
pub fn links(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|w| w.trim_start_matches(['(', '[', '<', '"', '\'']))
        .filter(|w| w.starts_with("https://") || w.starts_with("http://"))
        .map(|w| {
            w.trim_end_matches(['.', ',', ';', ')', ']', '>', '"', '\''])
                .to_owned()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_tokens_keeps_words_paths_codes_and_links() {
        let token = format!("sk-{}", "a1".repeat(12));
        let jwt = format!("eyJ{}.{}", "x7".repeat(10), "y8".repeat(10));
        let screen = format!(
            "token: {token}\nid {jwt}.\nOpen https://example.com/auth?client_id=abc&state=s1&access_token={token}#x\ncode ABCD-1234 in ~/.config/cooee\n"
        );
        assert_eq!(
            redact(&screen),
            "token: [redacted]\nid [redacted]\nOpen https://example.com/auth?client_id=abc&state=s1&access_token=[redacted]#x\ncode ABCD-1234 in ~/.config/cooee\n"
        );
        let long_link = format!("https://example.com/a?code_challenge={}", "Q9".repeat(20));
        assert_eq!(redact(&long_link), long_link);
        assert_eq!(redact("Logged in using ChatGPT"), "Logged in using ChatGPT");
    }

    #[test]
    fn finds_links_without_trailing_punctuation() {
        assert_eq!(
            links("visit https://example.com/a?b=1. or (http://localhost:1455/cb) now"),
            ["https://example.com/a?b=1", "http://localhost:1455/cb"]
        );
    }

    fn plain(spec: Spec) -> Box<dyn Provider> {
        Box::new(crate::Plain(spec))
    }

    #[test]
    fn status_from_the_exit_code_and_output() {
        let spec = |status: &str| Spec {
            name: "p".to_owned(),
            agent: "a".to_owned(),
            status: vec!["sh".to_owned(), "-c".to_owned(), status.to_owned()],
            install: "npm install -g p".to_owned(),
            home: "COOEE_TEST_HOME".to_owned(),
            ..Spec::default()
        };
        let p = |status: &str| plain(spec(status));
        let s = p("echo '  Logged in\n  as owner'").check(None);
        assert_eq!((s.signed_in, s.line.as_str()), (true, "Logged in as owner"));
        let s = p("echo not signed in >&2; exit 1").check(None);
        assert_eq!((s.signed_in, s.line.as_str()), (false, "not signed in"));
        let s = p("echo \"in $COOEE_TEST_HOME\"").check(Some(Path::new("/x/work")));
        assert_eq!((s.signed_in, s.line.as_str()), (true, "in /x/work"));
        assert_eq!(p("echo \"$COOEE_TEST_HOME\"").check(None).line, "");
        assert_eq!(
            p("").with_home(&["c".to_owned()], Some(Path::new("/x/work"))),
            ["env", "COOEE_TEST_HOME=/x/work", "c"]
        );
        let s = plain(Spec {
            login: vec!["no-such-cooee-cli".to_owned()],
            ..spec("exit 0")
        })
        .check(None);
        assert_eq!(
            (s.signed_in, s.line.as_str(), s.install.as_deref()),
            (false, "not installed on this Mac", Some("npm install -g p"))
        );
        assert!(
            !plain(Spec {
                status: vec!["/no/such/cooee-bin".to_owned()],
                ..spec("")
            })
            .check(None)
            .signed_in
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_timeout_kills_the_command_and_what_it_started() {
        let pids = env::temp_dir().join(format!("cooee-hung-{}", std::process::id()));
        let p = plain(Spec {
            status: ["sh", "-c", r#"sleep 600 & echo $$ $! > "$H"; sleep 600"#]
                .map(String::from)
                .into(),
            home: "H".to_owned(),
            ..Spec::default()
        });
        let started = Instant::now();
        let s = p.check(Some(&pids));
        assert!(started.elapsed() < LIMIT + Duration::from_secs(2));
        assert_eq!(s, Status::unknown(TIMED_OUT));
        let pids = std::fs::read_to_string(&pids).unwrap();
        let alive = |pid: &str| {
            Command::new("kill")
                .args(["-0", pid])
                .stderr(Stdio::null())
                .status()
                .unwrap()
                .success()
        };
        let gone = Instant::now() + Duration::from_secs(2);
        for pid in pids.split_whitespace() {
            while alive(pid) && Instant::now() < gone {
                thread::sleep(Duration::from_millis(20));
            }
            assert!(!alive(pid), "pid {pid} still alive");
        }
    }
}
