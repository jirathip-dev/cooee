use serde::Deserialize;

/// `[apns]`: the owner's APNs key's ids and the app it pushes to; none of it is secret.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Apns {
    /// The key's id (Apple Developer › Keys).
    pub key_id: String,
    /// The team id.
    pub team_id: String,
    /// The app's bundle id.
    pub topic: String,
    /// `production` (test-flight and App Store builds) or `sandbox` (Xcode debug builds): a token registered without one is this.
    pub env: ApnsEnv,
    /// The APNs server; default Apple's for each token's `env`.
    pub url: Option<String>,
}

/// Which APNs server a device token belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ApnsEnv {
    /// `api.sandbox.push.apple.com`.
    Sandbox,
    /// `api.push.apple.com`.
    Production,
}
