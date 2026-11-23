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

/// What a `show` call came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shown {
    /// Read and stored.
    Drawn,
    /// Refused: the persona is told this once and may send it again.
    Again(String),
    /// Refused again: what it said is stored as plain text.
    Plain,
    /// Not read: the turn had its retry.
    Spent,
}

/// The id prefix of item `item`'s blocks: `b<item>`.
#[must_use]
pub fn prefix(item: u64) -> String {
    format!("b{item}")
}

/// `text` as item `item`'s blocks under `catalog`, or blocks' refusal.
///
/// # Errors
/// What blocks' reader refused.
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
