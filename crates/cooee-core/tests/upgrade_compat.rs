//! Upgrade compatibility: every release's thread files in `tests/fixtures/upgrade/<version>/` still load, and nothing
//! in them is lost when this build reads and writes them back. A release that changes the format adds its own
//! fixture folder; old ones are never edited. Each release's `config.toml` loads too.

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use cooee_core::{Chat, Item, Store};

/// A copy of `release`'s folder to open, so the fixture itself is never imported or changed.
fn copy(release: &Path) -> PathBuf {
    let name = release.file_name().unwrap().to_string_lossy();
    let to = env::temp_dir().join(format!("cooee-upgrade-copy-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&to);
    fs::create_dir_all(to.join("threads")).unwrap();
    for file in fs::read_dir(release.join("threads")).unwrap() {
        let file = file.unwrap().path();
        fs::copy(&file, to.join("threads").join(file.file_name().unwrap())).unwrap();
    }
    to
}

/// #221: every release's items files (`<id>.jsonl`) import into the database with every item, in the same order the
/// files gave, a torn last line skipped as before.
#[test]
fn every_earlier_release_items_import_with_nothing_lost() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/upgrade");
    let mut checked = 0;
    for release in fs::read_dir(&root).unwrap() {
        let release = release.unwrap().path();
        let store = Store::open(&copy(&release)).unwrap();
        for file in fs::read_dir(release.join("threads")).unwrap() {
            let file = file.unwrap().path();
            if file.extension().is_none_or(|e| e != "jsonl") {
                continue;
            }
            let text = fs::read_to_string(&file).unwrap();
            let lines: Vec<Item> = text
                .lines()
                .filter_map(|l| serde_json::from_str(l).ok())
                .collect();
            assert!(
                lines.len() + 1 >= text.lines().count(),
                "at most a torn last line"
            );
            let id = file.file_stem().unwrap().to_str().unwrap();
            let want = Chat::from_versions(lines).items;
            assert_eq!(store.items(id).unwrap(), want, "{}", file.display());
            checked += want.len();
        }
    }
    assert!(
        checked >= 5,
        "items fixtures missing under {}",
        root.display()
    );
}

#[test]
fn every_earlier_release_loads_without_losing_data() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/upgrade");
    let mut checked = 0;
    for release in fs::read_dir(&root).unwrap() {
        let release = release.unwrap().path();
        let store = Store::open(&copy(&release)).unwrap();
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
            // #148: every thread from before is on this host.
            assert_eq!(thread.machine, "", "{}", file.display());
            // And this build writes it back unchanged.
            rewritten.save(&thread).unwrap();
            assert_eq!(rewritten.load(&id).unwrap(), thread);
            checked += 1;
        }
    }
    assert!(checked >= 2, "fixtures missing under {}", root.display());
}

/// R2 (herdsman#539): every release's `config.toml` still loads, so an upgrade never stops the service on a key it
/// used to read. A release that retires a key must keep reading it (with a warning) before this test passes again.
#[test]
fn every_earlier_release_config_loads() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/upgrade");
    let mut checked = 0;
    for release in fs::read_dir(&root).unwrap() {
        let config = release.unwrap().path().join("config.toml");
        if config.exists() {
            let c = cooee_core::Config::load(&config).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(c.port, 4747, "{}", config.display());
            checked += 1;
        }
    }
    assert!(checked >= 1, "no config fixture under {}", root.display());
}
