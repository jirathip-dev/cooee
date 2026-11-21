use serde::Deserialize;

/// `[perf]`: how cooee times itself (#53).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Perf {
    /// A request taking this many milliseconds or more is logged as slow (default 250).
    pub slow_ms: u64,
}

impl Default for Perf {
    fn default() -> Self {
        Self { slow_ms: 250 }
    }
}
