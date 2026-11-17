//! Upgrade compatibility: every release's files in `tests/fixtures/upgrade/<version>/` still load, and nothing in them
//! is lost when this build reads and writes them back or moves them into agent folders (#359). A release folder is
//! its own config and data folder unless it has `config/` and `data/`. A release that changes a format adds its own
//! fixture folder; old ones are never edited. Each release's `config.toml` loads too.

use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

use cooee_core::{Chat, Item, Store, migrate};

fn releases() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/upgrade");
    let mut out: Vec<PathBuf> = fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    out.sort();
    out
}

fn folders(release: &Path) -> (PathBuf, PathBuf) {
    if release.join("config").is_dir() {
        (release.join("config"), release.join("data"))
    } else {
        (release.to_owned(), release.to_owned())
    }
}

fn tree(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for e in fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(root, &p, out);
            } else {
                out.insert(
                    p.strip_prefix(root).unwrap().to_owned(),
                    fs::read(&p).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

fn copy(from: &Path, to: &Path) {
    for (rel, bytes) in tree(from) {
        fs::create_dir_all(to.join(&rel).parent().unwrap()).unwrap();
        fs::write(to.join(rel), bytes).unwrap();
    }
}

fn opened(release: &Path, test: &str) -> PathBuf {
    let name = release.file_name().unwrap().to_string_lossy();
    let to = cooee_core::scratch::owned(env::temp_dir().join(format!(
        "cooee-upgrade-{test}-{name}-{}",
        std::process::id()
    )));
    fs::remove_dir_all(&to).ok();
    copy(&folders(release).1, &to);
    to
}

#[test]
fn every_earlier_release_items_import_with_nothing_lost() {
    let mut checked = 0;
    for release in releases() {
        let threads = folders(&release).1.join("threads");
        let store = Store::open(&opened(&release, "items")).unwrap();
        for file in fs::read_dir(&threads).into_iter().flatten().flatten() {
            let file = file.path();
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
    assert!(checked >= 5, "items fixtures missing");
}

#[test]
fn every_earlier_release_loads_without_losing_data() {
    let mut checked = 0;
    for release in releases() {
        let data = opened(&release, "loads");
        let store = Store::open(&data).unwrap();
        let scratch = cooee_core::scratch::owned(env::temp_dir().join(format!(
            "cooee-upgrade-{}-{}",
            release.file_name().unwrap().to_string_lossy(),
            std::process::id()
        )));
        fs::remove_dir_all(&scratch).ok();
        let rewritten = Store::open(&scratch).unwrap();
        for id in store.ids().unwrap() {
            let file = data.join("threads").join(format!("{id}.json"));
            let thread = store.load(&id).unwrap_or_else(|e| panic!("{e}"));
            let raw: serde_json::Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
            assert_eq!(
                serde_json::to_value(&thread).unwrap(),
                raw["thread"],
                "{}",
                file.display()
            );
            assert_eq!(thread.machine, "", "{}", file.display());
            rewritten.save(&thread).unwrap();
            assert_eq!(rewritten.load(&id).unwrap(), thread);
            checked += 1;
        }
    }
    assert!(checked >= 3, "fixtures missing");
}

#[test]
fn every_earlier_release_config_loads() {
    let mut checked = 0;
    for release in releases() {
        let config = folders(&release).0.join("config.toml");
        if config.exists() {
            let c = cooee_core::Config::load(&config).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(c.port, 4747, "{}", config.display());
            checked += 1;
        }
    }
    assert!(checked >= 2, "no config fixture");
}

fn leaves(v: &serde_json::Value, out: &mut Vec<String>) {
    match v {
        serde_json::Value::Object(m) => m.values().for_each(|v| leaves(v, out)),
        serde_json::Value::Array(a) => a.iter().for_each(|v| leaves(v, out)),
        serde_json::Value::String(s) => out.push(s.replace('\n', "<br>")),
        serde_json::Value::Null => {}
        other => out.push(other.to_string()),
    }
}

const DAY: &str = "2026-11-14";

fn migrated(config: &Path, data: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut new: BTreeMap<PathBuf, Vec<u8>> = tree(&config.join("agents"))
        .into_iter()
        .map(|(p, b)| (Path::new("agents").join(p), b))
        .collect();
    let toml = fs::read(config.join("routines.toml")).unwrap();
    new.insert("routines.toml".into(), toml);
    let runs = fs::read(data.join("routine-runs.json")).unwrap();
    new.insert("routine-runs.json".into(), runs);
    new
}

fn nothing_lost(name: &str, before: &BTreeMap<PathBuf, Vec<u8>>, new: &BTreeMap<PathBuf, Vec<u8>>) {
    let mut text = String::new();
    for (p, b) in new {
        text.extend([
            p.display().to_string(),
            String::from_utf8_lossy(b).into_owned(),
        ]);
    }
    let skipped = [
        "ghost.toml",
        "Old_One.toml",
        "vault.toml",
        "broken.toml",
        "config.toml",
    ];
    let mut values = Vec::new();
    for (file, bytes) in before {
        let s = String::from_utf8_lossy(bytes);
        let ext = file.extension().and_then(|e| e.to_str());
        if skipped.iter().any(|n| file.ends_with(n))
            || !file.starts_with("config") && ext == Some("toml")
        {
            continue;
        }
        match ext {
            Some("toml") => leaves(&toml::from_str(&s).unwrap(), &mut values),
            Some("jsonl") => s
                .lines()
                .for_each(|l| leaves(&serde_json::from_str(l).unwrap(), &mut values)),
            Some("json") if file.ends_with("routines.json") => {
                leaves(&serde_json::from_str(&s).unwrap(), &mut values);
            }
            _ => {}
        }
    }
    let overridden = ["Kai the Coder", "s-kai", "kai.md", "coder.md"];
    for v in values.iter().filter(|v| !overridden.contains(&v.as_str())) {
        assert!(text.contains(v.as_str()), "{name}: {v:?} is lost");
    }
}

#[test]
fn every_earlier_release_migrates_once_losing_nothing() {
    let mut moved = 0;
    for release in releases() {
        let name = release.file_name().unwrap().to_string_lossy().into_owned();
        let scratch = cooee_core::scratch::owned(
            env::temp_dir().join(format!("cooee-migrate-{name}-{}", std::process::id())),
        );
        fs::remove_dir_all(&scratch).ok();
        copy(&release, &scratch);
        fs::remove_dir_all(scratch.join("expected")).ok();
        let (config, data) = folders(&scratch);
        let before = tree(&scratch);
        let plan = migrate::plan(&config, &data, DAY);
        let old = plan.state() == migrate::State::Old;
        plan.apply(|_| {}).unwrap();
        let after = tree(&scratch);

        let rel = |p: &Path| p.strip_prefix(&scratch).unwrap().to_owned();
        let olds = [
            config.join("personas"),
            config.join("profiles"),
            data.join("personas"),
            data.join("routines.json"),
        ];
        for (file, bytes) in &before {
            let backup = olds.iter().map(|o| rel(o)).find(|o| file.starts_with(o));
            let now = backup.map_or_else(
                || file.clone(),
                |o| {
                    PathBuf::from(format!("{}.bak-{DAY}", o.display()))
                        .join(file.strip_prefix(&o).unwrap())
                },
            );
            assert_eq!(
                after.get(&now),
                Some(bytes),
                "{name}: {} as {}",
                file.display(),
                now.display()
            );
        }

        let expected = release.join("expected");
        if expected.exists() {
            moved += 1;
            let (new, want) = (migrated(&config, &data), tree(&expected));
            assert_eq!(
                new.keys().collect::<Vec<_>>(),
                want.keys().collect::<Vec<_>>(),
                "{name}: the files"
            );
            for (file, bytes) in &want {
                let (got, want) = (
                    String::from_utf8_lossy(&new[file]),
                    String::from_utf8_lossy(bytes),
                );
                assert_eq!(got, want, "{name}: {}", file.display());
            }
            nothing_lost(&name, &before, &new);
        }

        let again = migrate::plan(&config, &data, DAY);
        let want = if old {
            migrate::State::Done
        } else {
            migrate::State::Fresh
        };
        assert_eq!((again.state(), again.lines()), (want, Vec::new()), "{name}");
        again
            .apply(|l| panic!("{name}: a second run said {l}"))
            .unwrap();
        assert_eq!(
            tree(&scratch),
            after,
            "{name}: a second run changed the tree"
        );
        fs::remove_dir_all(&scratch).unwrap();
    }
    assert!(moved >= 1, "no release with an expected tree");
}
