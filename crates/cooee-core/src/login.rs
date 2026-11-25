use serde::{Deserialize, Serialize};

/// A provider as cooee's policy (`[[providers]]`, a module's defaults); its program's facts are shed's record.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    /// Its name on the sheet: shed's provider of its accounts (`account.ls`).
    pub name: String,
    /// The ACP agent id it signs in, for the new-thread picker.
    pub agent: String,
    /// The dotted `_meta` key its ACP adapter takes an enforced settings object at; none: no profile `settings`.
    #[serde(default)]
    pub settings: String,
    /// The model a background run (#219) picks when it names none: its cheapest.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub cheapest: String,
    /// The registered provider whose behaviour it gets (default: the one named `name`, else none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
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
}
