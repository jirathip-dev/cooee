//! The first-party theme packs (docs/themes.md, #318): jirathip-dev/swag's `packs/<name>/` at the commit
//! `packs/swag.toml` pins. The build vendors them (`build.rs`), so they ship in the binary and an install with no
//! network has them; a file is handed out only when it matches its sha256 in the pin.

use std::{collections::BTreeMap, io};

use serde::Deserialize;

const PIN: &str = include_str!("../../../packs/swag.toml");
const FILES: &[(&str, &[u8])] = include!(concat!(env!("OUT_DIR"), "/swag.rs"));

/// The pin: where the packs come from and what each file must hash to.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Swag {
    /// The repo's git URL.
    pub url: String,
    /// The pinned commit.
    pub commit: String,
    sha256: BTreeMap<String, String>,
}

impl Swag {
    /// The pin this build was made from.
    ///
    /// # Panics
    /// Never once its test passes: the pin is compiled in.
    #[must_use]
    pub fn pinned() -> Self {
        toml::from_str(PIN).expect("packs/swag.toml parses")
    }

    /// The packs' names, in name order.
    #[must_use]
    pub fn names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self
            .sha256
            .keys()
            .filter_map(|f| f.split_once('/'))
            .map(|(n, _)| n)
            .collect();
        names.dedup();
        names
    }

    /// Pack `name`'s files (name in the pack, bytes), each checked against its sha256.
    ///
    /// # Errors
    /// When a file is missing or doesn't match the pin.
    pub fn files(&self, name: &str) -> io::Result<Vec<(&'static str, &'static [u8])>> {
        self.check(name, FILES)
    }

    fn check(
        &self,
        name: &str,
        files: &[(&'static str, &'static [u8])],
    ) -> io::Result<Vec<(&'static str, &'static [u8])>> {
        let mut out = Vec::new();
        for (path, sum) in self
            .sha256
            .iter()
            .filter(|(p, _)| p.split_once('/').is_some_and(|(n, _)| n == name))
        {
            let bad = |why: &str| {
                io::Error::new(io::ErrorKind::InvalidData, format!("swag {path}: {why}"))
            };
            let (at, bytes) = files
                .iter()
                .find(|(p, _)| p == path)
                .ok_or_else(|| bad("not vendored"))?;
            if crate::sha256(bytes) != *sum {
                return Err(bad("doesn't match its sha256 in packs/swag.toml"));
            }
            out.push((&at[name.len() + 1..], *bytes));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pinned_file_is_vendored_and_matches_its_sha256() {
        let swag = Swag::pinned();
        assert_eq!(swag.url, "https://github.com/jirathip-dev/swag");
        assert_eq!(swag.commit.len(), 40);
        assert_eq!(swag.names(), ["farm", "garden", "kitchen"]);
        for name in swag.names() {
            let files = swag.files(name).unwrap();
            let names: Vec<_> = files.iter().map(|(f, _)| *f).collect();
            assert_eq!(
                names,
                ["icon.png", "scene.svg", "sprites.svg", "theme.json"],
                "{name}"
            );
        }
    }

    #[test]
    fn a_file_that_does_not_match_its_sha256_is_refused() {
        let swag = Swag::pinned();
        let mut files = FILES.to_vec();
        let at = files
            .iter()
            .position(|(p, _)| *p == "farm/sprites.svg")
            .unwrap();
        files[at].1 = b"<svg/>";
        let e = swag.check("farm", &files).unwrap_err();
        assert_eq!(
            e.to_string(),
            "swag farm/sprites.svg: doesn't match its sha256 in packs/swag.toml"
        );
        assert!(swag.check("kitchen", &files).is_ok());
        files.retain(|(p, _)| *p != "kitchen/icon.png");
        let e = swag.check("kitchen", &files).unwrap_err();
        assert_eq!(e.to_string(), "swag kitchen/icon.png: not vendored");
    }
}
