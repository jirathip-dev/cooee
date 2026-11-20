use serde::{Deserialize, Serialize};

/// A script routine's command and what its exit does (#691).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Script {
    /// The `sh -c` command line, run as is: nothing fills into it.
    pub run: String,
    /// What its exit does.
    pub mode: Mode,
}

/// What a script routine's exit does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Its output posts in the chat; a non-zero exit pushes too.
    Post,
    /// Exit 0 stays silent; any other sends its output to the chat's agent as its prompt.
    Wake,
}

impl Script {
    pub(super) fn checked(self) -> Result<Self, String> {
        let run = self.run.trim().to_owned();
        if run.is_empty() || run.chars().count() > 2000 {
            return Err("BAD_SCRIPT: a script's run is 1 to 2,000 characters".to_owned());
        }
        Ok(Self { run, ..self })
    }
}
