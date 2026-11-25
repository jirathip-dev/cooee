# crate DIR DOC PRIVATE...: DIR/lib.rs, a `mod` line per module (`pub` unless PRIVATE names it), the root names from root.rs.
crate() {
  dir=$1 doc=$2
  shift 2
  {
    printf '//! %s\n' "$doc"
    printf '\n'
    printf '#![forbid(unsafe_code)]\n\n'
    for m in $(modules "$dir"); do
      case " $* " in *" $m "*) printf 'mod %s;\n' "$m" ;; *) printf 'pub mod %s;\n' "$m" ;; esac
    done
    printf '\npub use root::*;\n'
  } >"$dir/lib.rs.gen"
  mv "$dir/lib.rs.gen" "$dir/lib.rs"
}
