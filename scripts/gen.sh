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

# areas DIR (#729): DIR/areas.rs, a field per file of DIR with a `pub(super) struct State`, built by its `open(seed)` (`?` when it returns a Result) or its default.
areas() {
  dir=$1 ms=
  for m in $(modules "$dir"); do
    if grep -q '^pub(super) struct State ' "$dir/$m.rs" 2>/dev/null; then ms="$ms $m"; fi
  done
  {
    printf 'use super::{HubError, Seed%s};\n\n' "$(for m in $ms; do printf ', %s' "$m"; done)"
    printf 'pub(super) struct Areas {\n'
    for m in $ms; do printf '    pub(super) %s: %s::State,\n' "$m" "$m"; done
    printf '}\n\nimpl Areas {\n'
    printf "    pub(super) fn open(seed: &Seed<'_>) -> Result<Self, HubError> {\n        Ok(Self {\n"
    for m in $ms; do
      if grep -q "fn open(seed: &Seed<'_>) -> Result<Self, HubError>" "$dir/$m.rs"; then
        printf '            %s: %s::State::open(seed)?,\n' "$m" "$m"
      elif grep -q "fn open(seed: &Seed<'_>) -> Self" "$dir/$m.rs"; then
        printf '            %s: %s::State::open(seed),\n' "$m" "$m"
      else
        printf '            %s: %s::State::default(),\n' "$m" "$m"
      fi
    done
    printf '        })\n    }\n}\n'
  } | rustfmt --edition 2024 >"$dir/areas.rs.gen"
  mv "$dir/areas.rs.gen" "$dir/areas.rs"
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

# screens DIR: DIR/mod.rs as `routes` writes it, then one `source` trying each module that has one, in name order.
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

# arch: .drover/arch.toml (#729): every `frozen`, `generated` and `shared` line of .drover/arch/*.list, sorted, then each *.toml in name order.
arch() {
  {
    printf '# Written by scripts/gen.sh from .drover/arch/ (docs/architecture/hotspots-2026-11-16). Never edit it.\n'
    for k in frozen generated shared; do
      printf '%s = [%s]\n' "$k" "$(awk -v k="$k" '$1 == k { print $2 }' .drover/arch/*.list | LC_ALL=C sort -u | awk '{ printf "%s\"%s\"", (NR > 1 ? ", " : ""), $0 }')"
    done
    printf 'generate = "scripts/gen.sh"\n'
    for f in .drover/arch/*.toml; do
      printf '\n'
      cat "$f"
    done
  } >.drover/arch.toml.gen
  mv .drover/arch.toml.gen .drover/arch.toml
}

# The inputs: each file of scripts/gen.d/ in name order, one `<kind> <args>` a line.
for f in $(printf '%s\n' scripts/gen.d/* | LC_ALL=C sort); do . "./$f"; done
arch

# today: README.md's list between its markers, from the `## Today` bullets of docs/*.md and docs/*/README.md in file-name order, their ../ links made the README's.
for f in $(printf '%s\n' docs/*.md docs/*/README.md | LC_ALL=C sort); do
  awk '/^## / { on = $0 == "## Today"; next } on && /^- / { gsub(/\]\((\.\.\/)+/, "]("); print }' "$f"
done >README.md.today
awk -v f=README.md.today '/^<!-- \/today -->$/ { while ((getline l < f) > 0) print l; skip = 0 } !skip { print } /^<!-- today:/ { skip = 1 }' \
  README.md >README.md.gen
mv README.md.gen README.md
rm README.md.today
cargo update --workspace --quiet
