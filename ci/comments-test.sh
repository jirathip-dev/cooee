#!/bin/sh
# scripts/comments.py on fixtures: /// on public API, a crate root's //!, // SAFETY:, // TODO(#N) and a // inside a
# string or regex pass; a plain // comment, a /// on a private item, a /* */ block and a non-crate-root //! each fail.
set -eu
here=$(cd "$(dirname "$0")/.." && pwd)
t=$(mktemp -d)
trap 'rm -rf "${t:?}"' EXIT
check() { python3 "$here/scripts/comments.py" "$@" > "$t/out" 2>&1; }
fails() {
  printf '%b' "$2" > "$t/$1"
  if check "$t/$1"; then echo "expected $1 to fail: $2"; exit 1; fi
  grep -q "^$t/$1:[0-9]*: " "$t/out" || { cat "$t/out"; exit 1; }
}
printf '/// A.\npub fn a() {\n    // SAFETY: b is set above.\n    let u = "http://x";\n}\n// TODO(#201) one rule.\n' > "$t/ok.rs"
printf '/// A.\npublic enum A {\n    /// B.\n    case b\n}\n' > "$t/ok.swift"
printf '/** A. */\nexport const a = "//";\nconst r = /\\/\\//;\n' > "$t/ok.ts"
mkdir -p "$t/src"
printf '//! The crate.\npub fn a() {}\n' > "$t/src/lib.rs"
check "$t/ok.rs" "$t/ok.swift" "$t/ok.ts" "$t/src/lib.rs" || { cat "$t/out"; exit 1; }
fails plain.rs 'pub fn a() {}\n// says why\n'
fails private.rs '/// A.\nfn a() {}\n'
fails private.swift '/// A.\nstruct A {}\n'
fails block.ts 'export const a = 1; /* note */\n'
fails inner.rs '//! The crate.\npub fn a() {}\n'
printf 'let s = """\n// TODO(#1) in a string\n"""\n// TODO(#201) one rule.\n' > "$t/todo.swift"
test "$(python3 "$here/scripts/comments.py" --todos "$t/todo.swift" "$t/ok.rs")" = 201 || { echo "expected --todos to print 201 only"; exit 1; }
echo "comments test ok"
