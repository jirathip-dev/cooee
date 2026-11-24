use blocks::{
    Action, Block,
    catalog::Catalog,
    model::{self, Refusal},
};
use serde::{Deserialize, Serialize};

/// Blocks a persona wrote, read under its catalog: only `read`, `plain` and the store make one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Checked(Vec<Block>);

impl Checked {
    /// The blocks, as read.
    #[must_use]
    pub fn blocks(&self) -> &[Block] {
        &self.0
    }
}

/// What a `show` call came to: drawn, refused once with what to fix, stored as plain text, or not read.
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(missing_docs, reason = "the enum's line names each variant in order")]
pub enum Shown {
    Drawn,
    Again(String),
    Plain,
    Spent,
}

/// The id prefix of item `item`'s blocks: `b<item>`.
#[must_use]
pub fn prefix(item: u64) -> String {
    format!("b{item}")
}

/// `text` as item `item`'s blocks under `catalog`, or blocks' refusal.
#[expect(clippy::missing_errors_doc, reason = "the error is blocks' Refusal")]
pub fn read(catalog: &Catalog, item: u64, text: &str) -> Result<Checked, Refusal> {
    model::read(catalog, &prefix(item), text).map(|fit| Checked(fit.blocks))
}

/// What a refused reply said, as item `item`'s one prose block.
#[must_use]
pub fn plain(no: &Refusal, item: u64) -> Checked {
    Checked(vec![no.plain(&prefix(item))])
}

/// The reply an action's block belongs to, when its id is a model-written one (`b<item>.<n>`).
#[must_use]
pub fn item_of(a: &Action) -> Option<u64> {
    let (item, n) = a.block.as_deref()?.strip_prefix('b')?.split_once('.')?;
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit());
    if !digits(item) || !digits(n) {
        return None;
    }
    item.parse().ok()
}
