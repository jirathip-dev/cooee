use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

/// The fix for an empty `roots`, as the folder picker and `cooee doctor` print it.
pub const ROOTS_FIX: &str = r#"roots = ["~/Projects"]"#;

/// Writes `roots` (`~/x` under `$HOME`) into the config file at `path`, replacing only its `roots` key (or adding
/// it at the top), so every other line and comment stays; atomically.
///
/// # Errors
/// When the file can't be read, parsed or written.
pub fn save_roots(path: &Path, roots: &[PathBuf]) -> io::Result<()> {
    let text = match fs::read_to_string(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
        text => text?,
    };
    let home = env::var_os("HOME").map(PathBuf::from);
    let roots: Vec<String> = roots
        .iter()
        .map(
            |r| match home.as_deref().and_then(|h| r.strip_prefix(h).ok()) {
                Some(rest) => Path::new("~").join(rest).display().to_string(),
                None => r.display().to_string(),
            },
        )
        .collect();
    let line = format!("roots = {}", serde_json::to_string(&roots)?);
    let table = toml::de::DeTable::parse(&text).map_err(io::Error::other)?;
    let old = table.get_ref().iter().find(|(k, _)| k.get_ref() == "roots");
    let text = match old {
        Some((k, v)) => format!("{}{line}{}", &text[..k.span().start], &text[v.span().end..]),
        None => format!("{line}\n{text}"),
    };
    crate::write_private(path, text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;

    #[test]
    fn saving_roots_keeps_the_other_lines() {
        let path = crate::scratch::owned(
            env::temp_dir().join(format!("cooee-save-{}.toml", std::process::id())),
        );
        let home = PathBuf::from(env::var_os("HOME").unwrap());
        let text = "# mine\nport = 1\nroots = [\n  \"/a\",\n] # old\n\n[[plugins]]\nname = \"x\"\n";
        fs::write(&path, text).unwrap();
        save_roots(&path, &[home.join("P q"), PathBuf::from("/b\"c")]).unwrap();
        let saved = fs::read_to_string(&path).unwrap();
        assert_eq!(
            saved,
            "# mine\nport = 1\nroots = [\"~/P q\",\"/b\\\"c\"] # old\n\n[[plugins]]\nname = \"x\"\n"
        );
        fs::write(&path, "port = 1\n").unwrap();
        save_roots(&path, std::slice::from_ref(&home)).unwrap();
        assert_eq!(Config::load(&path).unwrap().roots, [home]);
        fs::remove_file(&path).unwrap();
    }
}
