# tests DIR: DIR/main.rs (#427), its leading `//!` lines kept, the shared support by path and a `mod` line per file of DIR.
tests() {
  dir=$1
  {
    awk '!/^\/\/!/ { exit } { print }' "$dir/main.rs"
    printf '\n#[path = "../support/mod.rs"]\nmod support;\n\n'
    for m in $(modules "$dir"); do printf 'mod %s;\n' "$m"; done
  } | rustfmt --edition 2024 >"$dir/main.rs.gen"
  mv "$dir/main.rs.gen" "$dir/main.rs"
}
