//! What a tool call does, in cooee's words, whichever harness made it (#503).

/// A tool call's kind of action.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Kind {
    /// Reads a file.
    Read,
    /// Edits, deletes or moves a file.
    Edit,
    /// Runs a command.
    Run,
    /// Searches files.
    Search,
    /// Fetches a URL.
    Fetch,
    /// Calls an MCP tool.
    Tool,
    /// Anything else.
    #[default]
    Other,
}

impl Kind {
    /// Every kind.
    pub const ALL: [Self; 7] = [
        Self::Read,
        Self::Edit,
        Self::Run,
        Self::Search,
        Self::Fetch,
        Self::Tool,
        Self::Other,
    ];

    /// What an action of this kind does, as a verb phrase.
    #[must_use]
    pub const fn verb(self) -> &'static str {
        match self {
            Self::Read => "reads a file",
            Self::Edit => "edits a file",
            Self::Run => "runs a command",
            Self::Search => "searches files",
            Self::Fetch => "fetches a page",
            Self::Tool => "calls a tool",
            Self::Other => "acts",
        }
    }
}
