//! Embeds `connectors/*.toml` (#497) as `$OUT_DIR/connectors.rs`, for `cooee_core::connectors`.

use std::{env, fs, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(&env::var("CARGO_MANIFEST_DIR")?).join("../..");
    let out = Path::new(&env::var("OUT_DIR")?).to_owned();
    let dir = root.join("connectors");
    println!("cargo::rerun-if-changed={}", dir.display());
    let mut found = Vec::new();
    for f in fs::read_dir(&dir)? {
        let at = fs::canonicalize(f?.path())?;
        if let Some(name) = at
            .file_stem()
            .filter(|_| at.extension() == Some("toml".as_ref()))
        {
            println!("cargo::rerun-if-changed={}", at.display());
            let name = name.to_string_lossy().into_owned();
            found.push(format!(
                "({name:?}, include_str!({:?}))",
                at.display().to_string()
            ));
        }
    }
    found.sort();
    fs::write(
        out.join("connectors.rs"),
        format!("&[{}]\n", found.join(",\n")),
    )?;
    Ok(())
}
