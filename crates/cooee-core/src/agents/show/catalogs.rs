use std::sync::LazyLock;

use blocks::{catalog::Catalog, model};

/// The catalogs' names, in the order a picker offers them.
pub const NAMES: &[&str] = &["show", "ask"];

const FILES: [&str; 2] = [
    include_str!("catalogs/show.json"),
    include_str!("catalogs/ask.json"),
];

const LEAD: &str = "You may answer the owner with blocks as well as text: call cooee's `show` tool, whose arguments \
are the JSON object the text below calls your reply. If it refuses, fix what it lists and call it once more. The \
owner's answer to your blocks comes as your next message.";

static ALL: LazyLock<Vec<Catalog>> =
    LazyLock::new(|| FILES.iter().filter_map(|f| Catalog::read(f).ok()).collect());

/// The shipped catalog `name`.
#[must_use]
pub fn catalog(name: &str) -> Option<&'static Catalog> {
    ALL.iter().find(|c| c.name == name)
}

/// What a persona with `catalog` is told of it in its session's brief: cooee's lead, then blocks' prompt whole.
#[must_use]
pub fn brief(catalog: &Catalog) -> String {
    format!("{LEAD}\n\n{}", model::prompt(catalog))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shipped_catalog_loads_and_offers_no_secret_and_no_file() {
        for (name, file) in NAMES.iter().zip(FILES) {
            let c = catalog(name).unwrap_or_else(|| panic!("{name} doesn't load"));
            assert_eq!(Catalog::read(file).as_ref(), Ok(c));
            assert!(c.media.is_empty(), "{name}");
            for kind in ["secret", "upload", "scan", "media", "image"] {
                assert!(
                    !file.contains(&format!("\"{kind}\"")),
                    "{name} offers {kind}"
                );
            }
        }
    }
}
