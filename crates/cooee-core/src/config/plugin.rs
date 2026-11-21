use serde::Deserialize;

/// A plugin: a tab whose page is an ES module served by the plugin's own backend, which cooee reaches with the
/// plugin's token (docs/plugins/).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plugin {
    /// Its id in URLs (`/p/<name>`): lowercase letters, digits and `-`.
    pub name: String,
    /// The tab's label.
    pub label: String,
    /// Its backend, `http://host:port` (local or on the tailnet).
    pub url: String,
    /// The path of its UI module on the backend, e.g. `/tab.js`.
    pub module: String,
    /// The secret cooee and the backend share: cooee sends it with every call, and the backend with every push
    /// or choice card it posts. At least 16 characters.
    pub token: String,
}
