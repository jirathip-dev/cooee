# tools DIR TYPE: DIR/mod.rs, a `mod` line per tool file (#664, #778) and the table `ALL` of each file's `TOOL`, a TYPE, in name order.
tools() {
  dir=$1 ty=$2
  {
    for m in $(modules "$dir"); do printf 'mod %s;\n' "$m"; done
    printf '\nuse %s;\n\npub(crate) const ALL: &[%s] = &[\n' "$ty" "${ty##*::}"
    for m in $(modules "$dir"); do printf '    %s::TOOL,\n' "$m"; done
    printf '];\n'
  } >"$dir/mod.rs.gen"
  mv "$dir/mod.rs.gen" "$dir/mod.rs"
}
