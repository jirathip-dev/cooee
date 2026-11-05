//! Upgrade compatibility: every release's thread files in `tests/fixtures/upgrade/<version>/` still load, and nothing
//! in them is lost when this build reads and writes them back. A release that changes the format adds its own
//! fixture folder; old ones are never edited.

use std::{env, fs, path::Path};

use cooee_core::Store;

#[test]
fn every_earlier_release_loads_without_losing_data() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/upgrade");
    let mut checked = 0;
    for release in fs::read_dir(&root).unwrap() {
        let release = release.unwrap().path();
        let store = Store::open(&release).unwrap();
        let scratch = env::temp_dir().join(format!(
            "cooee-upgrade-{}-{}",
            release.file_name().unwrap().to_string_lossy(),
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&scratch);
        let rewritten = Store::open(&scratch).unwrap();
        for id in store.ids().unwrap() {
            let file = release.join("threads").join(format!("{id}.json"));
            let thread = store.load(&id).unwrap_or_else(|e| panic!("{e}"));
            // Every field of the old file survives into the typed thread.
            let raw: serde_json::Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
            assert_eq!(
                serde_json::to_value(&thread).unwrap(),
                raw["thread"],
                "{}",
                file.display()
            );
            // And this build writes it back unchanged.
            rewritten.save(&thread).unwrap();
            assert_eq!(rewritten.load(&id).unwrap(), thread);
            checked += 1;
        }
    }
    assert!(checked >= 2, "fixtures missing under {}", root.display());
}
