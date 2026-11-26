//! drover's contract v1 samples (drover#1571 C1), copied by `scripts/drover-contract.sh` at the commit in `COMMIT`:
//! each parses with the types cooee reads rows with, so a drover bump that drops or retypes a key cooee reads fails here.

use std::{fs, path::Path};

use cooee_core::plugins::rows::{Row, Rows, To};
use serde_json::Value;

const DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/drover-contract"
);

fn sample(name: &str) -> Value {
    serde_json::from_str(&fs::read_to_string(Path::new(DIR).join(name)).unwrap()).unwrap()
}

fn row_samples() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(DIR)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("row-") && n.ends_with(".json"))
        .collect();
    names.sort();
    names
}

#[test]
fn the_copy_is_pinned_to_one_drover_commit() {
    let commit = fs::read_to_string(Path::new(DIR).join("COMMIT")).unwrap();
    let commit = commit.trim();
    assert!(
        commit.len() == 40 && commit.bytes().all(|b| b.is_ascii_hexdigit()),
        "{commit:?}"
    );
}

#[test]
fn every_row_sample_reads_as_cooees_row() {
    let names = row_samples();
    assert_eq!(names.len(), 11, "{names:?}");
    let mut targets = (false, false);
    for name in names {
        let rows: Vec<Row> =
            serde_json::from_value(sample(&name)).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(!rows.is_empty(), "{name}");
        for r in rows {
            targets.0 |= matches!(r.to, To::Thread { .. });
            targets.1 |= matches!(r.to, To::Desk {});
        }
    }
    assert_eq!(
        targets,
        (true, true),
        "the samples route to a chat and to the cards place"
    );
    for name in ["rows.json", "rows-open.json", "thread-rows.json"] {
        let page: Rows =
            serde_json::from_value(sample(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(page.rows.iter().all(|r| r.seq <= page.seq), "{name}");
    }
}

#[test]
fn a_row_missing_a_key_cooee_reads_fails_to_read() {
    let whole = sample("row-ask.json")[0].clone();
    for key in [
        "seq", "key", "to", "text", "block", "open", "prompt", "push",
    ] {
        let mut cut = whole.clone();
        cut.as_object_mut().unwrap().remove(key);
        assert!(
            serde_json::from_value::<Row>(cut).is_err(),
            "{key} missing still read"
        );
    }
    let mut cut = whole;
    cut.as_object_mut().unwrap().remove("handoff");
    assert!(
        serde_json::from_value::<Row>(cut).is_ok(),
        "a key cooee doesn't read may go"
    );
}
