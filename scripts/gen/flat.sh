# flat DIR: DIR/mod.rs, a private `mod` line per module, all re-exported flat (the test support's index).
flat() {
  dir=$1 ms=$(modules "$1")
  {
    printf '#![allow(dead_code, reason = "each test file uses part of the support")]\n\n'
    for m in $ms; do printf 'mod %s;\n' "$m"; done
    printf '\n#[allow(unused_imports, reason = "a test file may use none of an area'"'"'s items")]\n'
    printf 'pub use {%s};\n' "$(for m in $ms; do printf '%s::*, ' "$m"; done | sed 's/, $//')"
  } >"$dir/mod.rs.gen"
  mv "$dir/mod.rs.gen" "$dir/mod.rs"
}
