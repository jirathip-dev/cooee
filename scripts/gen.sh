#!/bin/sh
# Writes every generated file from its folder (#356, docs/architecture/hot-files/shapes.md). Never edit what it writes:
# add a file and run `just gen`; a conflict in a generated file is fixed by taking either side and running it again.
set -eu
cd "$(dirname "$0")/.."

# The modules of a folder: each `<name>.rs` and each `<name>/mod.rs`, once, in C-locale order.
modules() {
  for p in "$1"/*.rs "$1"/*/mod.rs; do
    [ -e "$p" ] || continue
    p=${p#"$1"/}
    case $p in lib.rs | main.rs | mod.rs) ;; */mod.rs) echo "${p%/mod.rs}" ;; *) echo "${p%.rs}" ;; esac
  done | LC_ALL=C sort -u
}

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

# cmds DIR: DIR/mod.rs, a `mod` line per command file and the table `ALL`, one entry per file in name order.
cmds() {
  dir=$1
  {
    for m in $(modules "$dir"); do printf 'mod %s;\n' "$m"; done
    printf '\nuse crate::cli::Command;\n\npub(crate) const ALL: &[Command] = &[\n'
    for m in $(modules "$dir"); do
      printf '    Command {\n        name: "%s",\n        usage: %s::USAGE,\n        flags: %s::FLAGS,\n        run: %s::run,\n    },\n' "$m" "$m" "$m" "$m"
    done
    printf '];\n'
  } >"$dir/mod.rs.gen"
  mv "$dir/mod.rs.gen" "$dir/mod.rs"
}

crate crates/cooee-core/src "cooee's core: the model and its files. No HTTP." \
  accounts blocks chat config devices login providers root shed store
cmds crates/cooee/src/cmd
cargo update --workspace --quiet
