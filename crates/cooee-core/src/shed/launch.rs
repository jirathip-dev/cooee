use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// An installed ACP agent, as `shed agents` lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Agent {
    /// The registry id (`<agent>-acp`, …).
    pub id: String,
    /// The pinned version.
    pub version: String,
    /// Its ACP adapter's command line.
    #[serde(default)]
    pub cmd: Vec<String>,
    /// Its models on the main login, as a picker offers them: cooee's, from its sessions (#193).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<String>,
}

/// What a thread's agent runs with besides its folder: variables on its adapter's command line, secret ones through
/// a file (see [`Shed::run_acp`](super::Shed::run_acp)), and shed's approval policy (none: shed's default, `ask`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Launch {
    /// `K=V` on the adapter's command line (an account's config folder, a profile's plain variables).
    pub env: Vec<(String, String)>,
    /// The file the secret variables pass through, and the variables.
    pub secrets: Option<(PathBuf, Vec<(String, String)>)>,
    /// `--policy`: `ask`, `allow` or `deny`.
    pub policy: Option<String>,
    /// Words after the adapter's command (a model or effort pick's).
    pub args: Vec<String>,
    /// `--mcp`: the file the session's MCP servers pass through (written here, removed once shed has read it), and
    /// the servers, in ACP's shape (#56).
    pub mcp: Option<(PathBuf, Vec<Value>)>,
    /// `--meta`: the session's options, sent as ACP `_meta` (a persona's provider's session setup, #503).
    pub meta: Option<Value>,
    /// `--resume`: the agent session to load instead of a new one (#613).
    pub resume: Option<String>,
}

impl Launch {
    /// Adds a model or effort's `words` (#553 row 34): a `NAME=value` word goes to `env`, any other to `args`.
    pub fn pick(&mut self, words: Vec<String>) {
        for w in words {
            match w.split_once('=') {
                Some((k, v))
                    if !k.is_empty()
                        && k.bytes()
                            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_') =>
                {
                    self.env.push((k.to_owned(), v.to_owned()));
                }
                _ => self.args.push(w),
            }
        }
    }
}
