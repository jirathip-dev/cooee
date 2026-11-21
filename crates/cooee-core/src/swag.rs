//! The first-party theme packs (docs/themes.md, #318, #622): the jirathip-dev/swag crate's, compiled in at the commit
//! Cargo.lock pins, so an install with no network has them.

use std::io;

const LOCK: &str = include_str!("../../../Cargo.lock");

/// Where the packs come from: swag's git URL and the commit this build has.
#[derive(Debug, Clone)]
pub struct Swag {
    /// The repo's git URL.
    pub url: String,
    /// The pinned commit.
    pub commit: String,
}

impl Swag {
    /// The pin this build was made from, read from its Cargo.lock.
    #[must_use]
    pub fn pinned() -> Self {
        let url = "https://github.com/jirathip-dev/swag";
        let commit = LOCK
            .lines()
            .filter_map(|l| l.strip_prefix(&format!("source = \"git+{url}?rev=")))
            .find_map(|l| l.strip_suffix('"')?.split_once('#'))
            .map_or_else(String::new, |(_, c)| c.to_owned());
        Self {
            url: url.to_owned(),
            commit,
        }
    }

    /// The packs' names, in name order.
    #[must_use]
    pub fn names(&self) -> Vec<&'static str> {
        let mut names: Vec<_> = swag::packs().iter().map(|p| p.name).collect();
        names.sort_unstable();
        names
    }

    /// Pack `name`'s files (name in the pack, bytes).
    ///
    /// # Errors
    /// When swag has no such pack.
    pub fn files(&self, name: &str) -> io::Result<Vec<(&'static str, &'static [u8])>> {
        swag::packs()
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.files.to_vec())
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, format!("swag has no pack {name}"))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pin_is_the_locked_swag_commit_and_names_its_four_packs() {
        let swag = Swag::pinned();
        assert_eq!(swag.url, "https://github.com/jirathip-dev/swag");
        assert_eq!(swag.commit.len(), 40, "{swag:?}");
        assert!(
            include_str!("../../../Cargo.toml").contains(&format!("rev = \"{}\"", swag.commit)),
            "{swag:?}"
        );
        assert_eq!(swag.names(), ["bush", "farm", "garden", "kitchen"]);
        for name in swag.names() {
            let mut names: Vec<_> = swag.files(name).unwrap().iter().map(|(f, _)| *f).collect();
            names.sort_unstable();
            assert_eq!(
                names,
                ["icon.png", "scene.svg", "sprites.svg", "theme.json"],
                "{name}"
            );
        }
        assert_eq!(
            swag.files("drover").unwrap_err().to_string(),
            "swag has no pack drover"
        );
    }
}
