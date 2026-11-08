//! Provider sign-ins: each provider's own login, logout and status commands (config), run for one account's config
//! folder; its signed-in state, and what of a login terminal's screen may reach the page: tokens redacted, links found.

use std::{
    env,
    io::Read,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};

/// A provider the sign-ins sheet lists: its CLI's own login, logout and read-only status commands.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provider {
    /// Its name on the sheet (`claude`); also part of its login terminal's name and its accounts' folder.
    pub name: String,
    /// The ACP agent id it signs in (`claude-acp`), for the new-thread picker.
    pub agent: String,
    /// The login command, run in a shed terminal.
    pub login: Vec<String>,
    /// The status command: exit 0 when signed in; its output is the status line.
    pub status: Vec<String>,
    /// The line that installs its CLI, shown when the login command isn't found.
    #[serde(default)]
    pub install: String,
    /// The variable that points its CLI at a config folder (`CLAUDE_CONFIG_DIR`): each added account is one.
    /// Without it the provider has only the system default account.
    #[serde(default)]
    pub home: String,
    /// The logout command.
    #[serde(default)]
    pub logout: Vec<String>,
    /// The variable its ACP adapter reads an extra settings file from and enforces, hooks included. Without it an
    /// agent profile with `settings` can't start with its agent. Claude Code's is `CLAUDE_CONFIG_DIR` (herdsman#565:
    /// claude-acp enforces the user settings there), so its profile's `settings` is a config folder, in place of the
    /// account's.
    #[serde(default)]
    pub settings: String,
    /// The models a thread may pick (#553 row 34).
    #[serde(default)]
    pub models: Vec<String>,
    /// The words that start its agent on a model: `{}` is the pick; a `NAME=value` word goes to its environment,
    /// any other after its command.
    #[serde(default)]
    pub model: Vec<String>,
    /// The efforts a thread may pick.
    #[serde(default)]
    pub efforts: Vec<String>,
    /// The words for an effort, as `model`.
    #[serde(default)]
    pub effort: Vec<String>,
}

impl Provider {
    /// Claude Code, Codex and Gemini.
    #[must_use]
    pub fn defaults() -> Vec<Self> {
        let v = |a: &[&str]| a.iter().map(|s| (*s).to_owned()).collect();
        let p = |name: &str,
                 agent: &str,
                 [login, status, logout]: [&[&str]; 3],
                 npm: &str,
                 home: &str| Self {
            name: name.to_owned(),
            agent: agent.to_owned(),
            login: v(login),
            status: v(status),
            install: format!("npm install -g {npm}"),
            home: home.to_owned(),
            logout: v(logout),
            ..Self::default()
        };
        let creds = "\"${GEMINI_CLI_HOME:-$HOME}/.gemini/oauth_creds.json\"";
        let mut list = vec![
            p(
                "claude",
                "claude-acp",
                [
                    &["claude", "auth", "login"],
                    &["claude", "auth", "status"],
                    &["claude", "auth", "logout"],
                ],
                "@anthropic-ai/claude-code",
                "CLAUDE_CONFIG_DIR",
            ),
            p(
                "codex",
                "codex-acp",
                [
                    &["codex", "login", "--device-auth"],
                    &["codex", "login", "status"],
                    &["codex", "logout"],
                ],
                "@openai/codex",
                "CODEX_HOME",
            ),
            p(
                "gemini",
                "gemini",
                [
                    &["gemini"],
                    &[
                        "sh",
                        "-c",
                        &format!("test -s {creds} && echo signed in with Google"),
                    ],
                    &["sh", "-c", &format!("rm -f {creds}")],
                ],
                "@google/gemini-cli",
                "GEMINI_CLI_HOME",
            ),
        ];
        let v = |a: &[&str]| -> Vec<String> { a.iter().map(|s| (*s).to_owned()).collect() };
        list[0].models = v(&["opus", "sonnet", "haiku"]);
        list[0].model = v(&["ANTHROPIC_MODEL={}"]);
        "CLAUDE_CONFIG_DIR".clone_into(&mut list[0].settings);
        list[1].efforts = v(&["low", "medium", "high"]);
        list[1].effort = v(&["-c", "model_reasoning_effort={}"]);
        list
    }

    /// The words that start its agent on `model` and `effort` (empty: its own default); `None` when it offers no
    /// such pick.
    #[must_use]
    pub fn picks(&self, model: &str, effort: &str) -> Option<Vec<String>> {
        let mut words = Vec::new();
        for (pick, offered, template) in [
            (model, &self.models, &self.model),
            (effort, &self.efforts, &self.effort),
        ] {
            if pick.is_empty() {
                continue;
            }
            if !offered.iter().any(|o| o == pick) {
                return None;
            }
            words.extend(template.iter().map(|w| w.replace("{}", pick)));
        }
        Some(words)
    }

    /// The variable and folder that point its CLI at the account in `home` (none for the system default).
    #[must_use]
    pub fn env<'a>(&'a self, home: Option<&'a Path>) -> Option<(&'a str, &'a Path)> {
        home.filter(|_| !self.home.is_empty())
            .map(|dir| (self.home.as_str(), dir))
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
        self.login.first().is_some_and(|b| !installed(b))
    }

    /// Runs `cmd` (10 s at most) for the account in `home`: whether it exited 0, its stdout and its stderr.
    fn run(&self, cmd: &[String], home: Option<&Path>) -> Result<(bool, String, String), String> {
        let Some((bin, args)) = cmd.split_first() else {
            return Err("no command".to_owned());
        };
        let mut command = Command::new(bin);
        if let Some((var, dir)) = self.env(home) {
            command.env(var, dir);
        }
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
        let deadline = Instant::now() + Duration::from_secs(10);
        let ok = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status.success(),
                Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(20)),
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("`{}` timed out", cmd.join(" ")));
                }
            }
        };
        let (stdout, stderr) = (
            stdout.join().unwrap_or_default(),
            stderr.join().unwrap_or_default(),
        );
        Ok((ok, stdout, stderr))
    }

    /// Runs the logout command for the account in `home`.
    ///
    /// # Errors
    /// Its output on one line, redacted, when it fails or can't run.
    pub fn sign_out(&self, home: Option<&Path>) -> Result<(), String> {
        match self.run(&self.logout, home)? {
            (true, ..) => Ok(()),
            (false, out, err) => Err(one_line(if err.trim().is_empty() { &out } else { &err })),
        }
    }

    /// Runs the status command (10 s at most) for the account in `home`: signed in when it exits 0.
    #[must_use]
    pub fn check(&self, home: Option<&Path>) -> Status {
        let out = |signed_in, line: &str| Status {
            install: None,
            signed_in,
            line: one_line(line),
            who: None,
            usage: None,
        };
        if self.missing() {
            return Status {
                install: Some(self.install.clone()),
                ..out(false, "not installed on this Mac")
            };
        }
        let (ok, stdout, stderr) = match self.run(&self.status, home) {
            Ok(r) => r,
            Err(e) => return out(false, &e),
        };
        // A JSON status (`claude auth status`): its `loggedIn` as one plain line, who, and usage when it says.
        if let Ok(v @ serde_json::Value::Object(_)) = serde_json::from_str(&stdout) {
            let who = v["email"].as_str().or(v["orgName"].as_str());
            let s = match (v["loggedIn"].as_bool(), v["authMethod"].as_str(), who) {
                (Some(true), Some(how), Some(who)) => {
                    out(true, &format!("signed in · {how} ({who})"))
                }
                (Some(true), Some(how), None) => out(true, &format!("signed in · {how}")),
                (Some(true), None, _) => out(true, "signed in"),
                (Some(false), ..) => out(false, "signed out"),
                (None, ..) => out(ok, "status unknown"),
            };
            return Status {
                who: who.map(one_line),
                usage: v["usage"].as_str().map(one_line),
                ..s
            };
        }
        out(
            ok,
            if stdout.trim().is_empty() {
                &stderr
            } else {
                &stdout
            },
        )
    }
}

/// An account's signed-in state, from its provider's status command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Status {
    /// The status command exited 0.
    pub signed_in: bool,
    /// Its output on one line, redacted, at most 80 characters.
    pub line: String,
    /// The install line, when the login command isn't found.
    pub install: Option<String>,
    /// The email or org it is signed in as, when the status says.
    pub who: Option<String>,
    /// Its usage, when the status reports it.
    pub usage: Option<String>,
}

/// `text` on one line, redacted, at most 80 characters.
fn one_line(text: &str) -> String {
    redact(&text.split_whitespace().collect::<Vec<_>>().join(" "))
        .chars()
        .take(80)
        .collect()
}

/// Whether `bin` is a file, or one on `PATH` when it is a bare name.
fn installed(bin: &str) -> bool {
    let path = Path::new(bin);
    if path.components().count() > 1 {
        return path.is_file();
    }
    env::var_os("PATH").is_some_and(|p| env::split_paths(&p).any(|d| d.join(bin).is_file()))
}

/// Whether `c` may be part of a token.
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

    /// #553 row 34: a pick's words, `{}` filled in; nothing for the defaults; `None` for a pick not offered.
    #[test]
    fn picks_fill_the_providers_words() {
        let codex = &Provider::defaults()[1];
        assert_eq!(
            codex.picks("", "high"),
            Some(vec![
                "-c".to_owned(),
                "model_reasoning_effort=high".to_owned()
            ])
        );
        assert_eq!(codex.picks("", ""), Some(Vec::new()));
        assert_eq!(codex.picks("", "max"), None);
        assert_eq!(codex.picks("opus", ""), None);
    }

    #[test]
    fn status_from_the_exit_code_and_output() {
        let p = |status: &str| Provider {
            name: "p".to_owned(),
            agent: "a".to_owned(),
            login: Vec::new(),
            status: vec!["sh".to_owned(), "-c".to_owned(), status.to_owned()],
            install: "npm install -g p".to_owned(),
            home: "COOEE_TEST_HOME".to_owned(),
            logout: Vec::new(),
            ..Provider::default()
        };
        let s = p("echo '  Logged in\n  as owner'").check(None);
        assert_eq!((s.signed_in, s.line.as_str()), (true, "Logged in as owner"));
        let s = p("echo not signed in >&2; exit 1").check(None);
        assert_eq!((s.signed_in, s.line.as_str()), (false, "not signed in"));
        let s =
            p(r#"echo '{"loggedIn": true, "authMethod": "claude.ai", "email": "o@example.com"}'"#)
                .check(None);
        assert_eq!(
            (s.signed_in, s.line.as_str()),
            (true, "signed in · claude.ai (o@example.com)")
        );
        assert_eq!(s.who.as_deref(), Some("o@example.com"));
        // An account's status runs with its folder in the provider's variable.
        let s = p(r#"echo "{\"loggedIn\": true, \"orgName\": \"Acme\", \"usage\": \"$COOEE_TEST_HOME 40% of the week\"}""#)
            .check(Some(Path::new("/x/work")));
        assert_eq!(
            (s.line.as_str(), s.who.as_deref(), s.usage.as_deref()),
            ("signed in", Some("Acme"), Some("/x/work 40% of the week"))
        );
        assert_eq!(p("echo \"$COOEE_TEST_HOME\"").check(None).line, "");
        assert_eq!(
            p("").with_home(&["c".to_owned()], Some(Path::new("/x/work"))),
            ["env", "COOEE_TEST_HOME=/x/work", "c"]
        );
        let s = p(r#"echo '{"loggedIn": false, "authMethod": "none"}'; exit 1"#).check(None);
        assert_eq!((s.signed_in, s.line.as_str()), (false, "signed out"));
        assert_eq!(p(r#"echo '{"a": 1}'"#).check(None).line, "status unknown");
        let s = Provider {
            login: vec!["no-such-cooee-cli".to_owned()],
            ..p("exit 0")
        }
        .check(None);
        assert_eq!(
            (s.signed_in, s.line.as_str(), s.install.as_deref()),
            (false, "not installed on this Mac", Some("npm install -g p"))
        );
        assert!(
            !Provider {
                status: vec!["/no/such/cooee-bin".to_owned()],
                ..p("")
            }
            .check(None)
            .signed_in
        );
    }
}
