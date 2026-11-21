#!/bin/sh
# The banned APIs (#203) on fixtures: each Rust ban fails clippy (the workspace's lints and clippy.toml, on a crate of
# its own) and each TypeScript ban fails scripts/ts-bans.py; clean code passes, and a test may unwrap, panic and index.
set -eu
here=$(cd "$(dirname "$0")/.." && pwd)
t=$(mktemp -d)
trap 'rm -rf "${t:?}"' EXIT
fail() { echo "FAIL: $*"; exit 1; }

mkdir -p "$t/rs/fx/src"
cp "$here/clippy.toml" "$here/rust-toolchain.toml" "$t/rs/"
{
  printf '[workspace]\nresolver = "3"\nmembers = ["fx"]\n\n'
  sed -n '/^\[workspace\.lints\.rust\]/,$p' "$here/Cargo.toml"
} > "$t/rs/Cargo.toml"
printf '[package]\nname = "fx"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n\n[lints]\nworkspace = true\n' \
  > "$t/rs/fx/Cargo.toml"
clippy() { (cd "$t/rs" && CARGO_TARGET_DIR="$t/rs/target" cargo clippy -q --all-targets --message-format=json -- -D warnings) > "$t/out" 2>&1; }
rust() {
  printf '//! Fixture.\n\n/// Fixture.\n%b\n' "$2" > "$t/rs/fx/src/lib.rs"
  if clippy; then fail "expected $1 to fail clippy"; fi
  grep -q "\"code\":\"$1\"" "$t/out" || { cat "$t/out"; fail "expected lint $1"; }
}
rust clippy::unwrap_used 'pub fn f(x: Option<u8>) -> u8 {\n    x.unwrap()\n}'
rust clippy::expect_used 'pub fn f(x: Option<u8>) -> u8 {\n    x.expect("x")\n}'
rust clippy::panic 'pub fn f() {\n    panic!("x");\n}'
rust clippy::todo 'pub fn f() {\n    todo!();\n}'
rust clippy::unimplemented 'pub fn f() {\n    unimplemented!();\n}'
rust clippy::dbg_macro 'pub fn f(x: u8) -> u8 {\n    dbg!(x)\n}'
rust clippy::indexing_slicing 'pub fn f(x: &[u8], i: usize) -> u8 {\n    x[i]\n}'
rust clippy::as_conversions 'pub fn f(x: u8) -> u32 {\n    x as u32\n}'
rust clippy::print_stdout 'pub fn f() {\n    println!("x");\n}'
rust clippy::print_stderr 'pub fn f() {\n    eprintln!("x");\n}'
rust clippy::allow_attributes_without_reason '#[allow(clippy::needless_return)]\npub fn f() {\n    return;\n}'
rust clippy::let_underscore_must_use 'pub fn f() {\n    let _ = std::fs::remove_file("x");\n}'
rust unsafe_code 'pub fn f() {\n    unsafe {}\n}'
printf '//! Fixture.\n\n/// Fixture.\n#[must_use]\npub fn f(x: &[u8], i: usize) -> Option<u8> {\n    x.get(i).copied()\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {\n        let x = [1, 2];\n        assert_eq!(super::f(&x, 0).unwrap(), x[0]);\n        assert_eq!(super::f(&x, 1).expect("x"), 2);\n        match x.len() {\n            2 => {}\n            _ => panic!("x"),\n        }\n    }\n}\n' \
  > "$t/rs/fx/src/lib.rs"
clippy || { cat "$t/out"; fail "expected clean code, and a test that unwraps, panics and indexes, to pass clippy"; }

mkdir -p "$t/ts"
printf '{ "compilerOptions": { "strict": true } }\n' > "$t/ts/tsconfig.json"
ts() {
  printf '%b\n' "$2" > "$t/ts/f.ts"
  if "$here/scripts/ts-bans.py" "$t/ts/f.ts" > "$t/out"; then fail "expected $1 to fail ts-bans"; fi
  grep -q "$1" "$t/out" || { cat "$t/out"; fail "expected ts-bans to name $1"; }
}
ts '`any`' 'export const x: any = 1;'
ts 'non-null' 'export const f = (x: string | null): number => x!.length;'
ts '`as` cast' 'export const f = (x: unknown): string => x as string;'
ts '@ts-ignore' '// @ts-ignore\nexport const x = 1;'
ts '@ts-ignore' '// @ts-expect-error\nexport const x = 1;'
ts 'HTML from a string' 'export const f = (e: Element): void => { e.innerHTML = "x"; };'
ts 'HTML from a string' 'export const f = (e: Element): void => { e.insertAdjacentHTML("beforeend", "x"); };'
ts '`eval`' 'export const f = (s: string): unknown => eval(s);'
ts '`eval`' 'export const f = new Function("return 1");'
ts '`console.log`' 'export const f = (): void => { console.log("x"); };'
ts 'default export' 'export default 1;'
ts '`enum`' 'export enum Tone { Ok, Bad }'
printf 'import { a as b } from "./a";\n\nexport const tones = ["ok", "bad"] as const;\nexport const ok = (x: string | null): boolean => x !== null && b(x) != "";\n' > "$t/ts/f.ts"
"$here/scripts/ts-bans.py" "$t/ts/f.ts" || fail "expected clean TypeScript to pass ts-bans"
printf '{ "compilerOptions": { "strict": true, "noImplicitAny": false } }\n' > "$t/ts/tsconfig.json"
ts 'implicit any' 'export const x = 1;'
echo "bans test ok"
