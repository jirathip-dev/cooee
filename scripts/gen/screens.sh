# screens DIR: DIR/mod.rs as `routes` writes it, then one `source` and one `act`, each trying every module that has one, in name order.
screens() {
  dir=$1
  routes "$dir"
  set -- $(for m in $(modules "$dir"); do if grep -q '^pub(super) fn source(' "$dir/$m.rs"; then echo "$m"; fi; done)
  {
    cat "$dir/mod.rs"
    printf '\npub(crate) fn source(path: &str, who: &crate::live::Who) -> Option<crate::live::Source> {\n'
    if [ $# -eq 0 ]; then printf '    None\n'; fi
    for m in "$@"; do
      if [ "$m" = "$1" ]; then printf '    %s::source(path, who)\n' "$m"; else printf '        .or_else(|| %s::source(path, who))\n' "$m"; fi
    done
    printf '}\n'
  } | rustfmt --edition 2024 >"$dir/mod.rs.gen"
  mv "$dir/mod.rs.gen" "$dir/mod.rs"
  set -- $(for m in $(modules "$dir"); do if grep -q '^pub(super) async fn act(' "$dir/$m.rs"; then echo "$m"; fi; done)
  {
    cat "$dir/mod.rs"
    printf '\npub(crate) async fn act(cx: &topcoat::context::Cx, a: &blocks::Action) -> Option<topcoat::Result<topcoat::router::response::Response>> {\n'
    for m in "$@"; do printf '    if let Some(done) = %s::act(cx, a).await {\n        return Some(done);\n    }\n' "$m"; done
    printf '    None\n}\n'
  } | rustfmt --edition 2024 >"$dir/mod.rs.gen"
  mv "$dir/mod.rs.gen" "$dir/mod.rs"
}
