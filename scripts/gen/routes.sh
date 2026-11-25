# routes DIR:DIR/mod.rs, a `mod` line per area and one `routes` that chains every area's `routes`.
routes() {
  dir=$1
  set -- $(modules "$dir")
  {
    for m in "$@"; do printf 'mod %s;\n' "$m"; done
    printf '\nuse topcoat::router::RouterBuilder;\n\n'
    printf 'pub(crate) fn routes(b: RouterBuilder) -> RouterBuilder {\n'
    while [ $# -gt 1 ]; do printf '    let b = %s::routes(b);\n' "$1"; shift; done
    printf '    %s::routes(b)\n}\n' "$1"
  } >"$dir/mod.rs.gen"
  mv "$dir/mod.rs.gen" "$dir/mod.rs"
}
