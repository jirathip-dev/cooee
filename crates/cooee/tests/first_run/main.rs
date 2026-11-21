//! A fresh instance: with no agents in shed and no `roots`, `cooee doctor` warns about both; with no roots, a chat
//! still starts (in its scratch folder) and Use a folder… is refused; a provider whose CLI is missing reports its install line; with one root, a
//! thread starts in the root.

#[path = "../support/mod.rs"]
mod support;

mod fixes;
mod roots;
