# module DIR: DIR.rs, its leading `//!` lines kept, a `mod` line per file of DIR (`tests` under `#[cfg(test)]`) and a `pub use` of each with a `pub ` item.
module() {
  dir=$1
  {
    awk '!/^\/\/!/ { exit } { print }' "$dir.rs"
    for m in $(modules "$dir"); do
      [ "$m" != tests ] || printf '#[cfg(test)]\n'
      printf 'mod %s;\n' "$m"
    done
    printf '\n'
    for m in $(modules "$dir"); do
      if grep -q '^pub ' "$dir/$m.rs" 2>/dev/null || grep -q '^pub ' "$dir/$m/mod.rs" 2>/dev/null; then printf 'pub use %s::*;\n' "$m"; fi
    done
  } | rustfmt --edition 2024 >"$dir.rs.gen"
  mv "$dir.rs.gen" "$dir.rs"
}
