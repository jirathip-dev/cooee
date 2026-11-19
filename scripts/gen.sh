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

# module DIR: DIR.rs, its leading `//!` lines kept, a `mod` line per file of DIR and a `pub use` of each with a `pub ` item.
module() {
  dir=$1
  {
    awk '!/^\/\/!/ { exit } { print }' "$dir.rs"
    for m in $(modules "$dir"); do printf 'mod %s;\n' "$m"; done
    printf '\n'
    for m in $(modules "$dir"); do
      if grep -q '^pub ' "$dir/$m.rs" 2>/dev/null || grep -q '^pub ' "$dir/$m/mod.rs" 2>/dev/null; then printf 'pub use %s::*;\n' "$m"; fi
    done
  } | rustfmt --edition 2024 >"$dir.rs.gen"
  mv "$dir.rs.gen" "$dir.rs"
}

# routes DIR: DIR/mod.rs, a `mod` line per area and one `routes` that chains every area's `routes`.
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

# tools DIR: DIR/mod.rs, a `mod` line per Chief tool file (#664) and the table `ALL` of each file's `TOOL`, in name order.
tools() {
  dir=$1
  {
    for m in $(modules "$dir"); do printf 'mod %s;\n' "$m"; done
    printf '\nuse crate::chief::Tool;\n\npub(crate) const ALL: &[Tool] = &[\n'
    for m in $(modules "$dir"); do printf '    %s::TOOL,\n' "$m"; done
    printf '];\n'
  } >"$dir/mod.rs.gen"
  mv "$dir/mod.rs.gen" "$dir/mod.rs"
}

crate crates/cooee-core/src "cooee's core: the model and its files. No HTTP." \
  accounts blocks chat config devices login providers root shed store
module crates/cooee-core/src/chat
module crates/cooee-core/src/shed
module crates/cooee-core/src/store
module crates/cooee-server/src/login
module crates/cooee-server/src/mcp
module crates/cooee-server/src/push
routes crates/cooee-server/src/routes
crate crates/cooee-server/src "cooee's HTTP server: the native app's JSON API (docs/README.md)." \
  chief connectors contrast editor gate http hub layout login look manage pairing passkey net plugin push root routes routines screens serve themes
tools crates/cooee-server/src/chief/tools
cmds crates/cooee/src/cmd
flat crates/cooee/tests/support
# today: README.md's list between its markers, from the `## Today` bullets of docs/*.md in file-name order, their ../ links made the README's.
for f in $(printf '%s\n' docs/*.md | LC_ALL=C sort); do
  awk '/^## / { on = $0 == "## Today"; next } on && /^- / { gsub(/\]\(\.\.\//, "]("); print }' "$f"
done >README.md.today
awk -v f=README.md.today '/^<!-- \/today -->$/ { while ((getline l < f) > 0) print l; skip = 0 } !skip { print } /^<!-- today:/ { skip = 1 }' \
  README.md >README.md.gen
mv README.md.gen README.md
rm README.md.today
cargo update --workspace --quiet
