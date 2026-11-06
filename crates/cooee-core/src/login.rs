//! Provider sign-ins: each provider's own login and status commands (config), its signed-in state, and what of
//! a login terminal's screen may reach the page: tokens redacted, links found.

use std::{
    io::Read,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};

/// A provider the sign-ins sheet lists: its CLI's own login command and a read-only status command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provider {
    /// Its name on the sheet (`claude`); also part of its login terminal's name.
    pub name: String,
    /// The ACP agent id it signs in (`claude-acp`), for the new-thread picker.
    pub agent: String,
    /// The login command, run in a shed terminal.
    pub login: Vec<String>,
    /// The status command: exit 0 when signed in; its output is the status line.
    pub status: Vec<String>,
}

impl Provider {
    /// Claude Code, Codex and Gemini.
    #[must_use]
    pub fn defaults() -> Vec<Self> {
        let p = |name: &str, agent: &str, login: &[&str], status: &[&str]| Self {
            name: name.to_owned(),
            agent: agent.to_owned(),
            login: login.iter().map(|s| (*s).to_owned()).collect(),
            status: status.iter().map(|s| (*s).to_owned()).collect(),
        };
        vec![
            p(
                "claude",
                "claude-acp",
                &["claude", "auth", "login"],
                &["claude", "auth", "status"],
            ),
            p(
                "codex",
                "codex-acp",
                &["codex", "login", "--device-auth"],
                &["codex", "login", "status"],
            ),
            p(
                "gemini",
                "gemini",
                &["gemini"],
                &[
                    "sh",
                    "-c",
                    "test -s \"$HOME/.gemini/oauth_creds.json\" && echo signed in with Google",
                ],
            ),
        ]
    }

    /// Runs the status command (10 s at most): signed in when it exits 0.
    #[must_use]
    pub fn check(&self) -> Status {
        let out = |signed_in, line: &str| Status {
            signed_in,
            line: redact(&line.split_whitespace().collect::<Vec<_>>().join(" "))
                .chars()
                .take(80)
                .collect(),
        };
        let Some((bin, args)) = self.status.split_first() else {
            return out(false, "no status command");
        };
        let child = Command::new(bin)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        let mut child = match child {
            Ok(child) => child,
            Err(e) => return out(false, &format!("{bin}: {e}")),
        };
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
                    return out(false, "the status command timed out");
                }
            }
        };
        let (stdout, stderr) = (
            stdout.join().unwrap_or_default(),
            stderr.join().unwrap_or_default(),
        );
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

/// A provider's signed-in state, from its status command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Status {
    /// The status command exited 0.
    pub signed_in: bool,
    /// Its output on one line, redacted, at most 80 characters.
    pub line: String,
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

    #[test]
    fn status_from_the_exit_code_and_output() {
        let p = |status: &str| Provider {
            name: "p".to_owned(),
            agent: "a".to_owned(),
            login: Vec::new(),
            status: vec!["sh".to_owned(), "-c".to_owned(), status.to_owned()],
        };
        let s = p("echo '  Logged in\n  as owner'").check();
        assert_eq!((s.signed_in, s.line.as_str()), (true, "Logged in as owner"));
        let s = p("echo not signed in >&2; exit 1").check();
        assert_eq!((s.signed_in, s.line.as_str()), (false, "not signed in"));
        assert!(
            !Provider {
                status: vec!["/no/such/cooee-bin".to_owned()],
                ..p("")
            }
            .check()
            .signed_in
        );
    }
}
