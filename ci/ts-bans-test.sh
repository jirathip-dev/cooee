#!/bin/sh
# The TypeScript bans (#203) on fixtures: each fails scripts/ts-bans.py; clean code passes.
set -eu
here=$(cd "$(dirname "$0")/.." && pwd)
t=$(mktemp -d)
trap 'rm -rf "${t:?}"' EXIT
fail() { echo "FAIL: $*"; exit 1; }

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
echo "ts-bans test ok"
