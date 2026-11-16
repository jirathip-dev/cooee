#!/usr/bin/env python3
"""The TypeScript bans (#203) on every tracked .ts and .tsx file (or the paths given).

Banned: `any`, the non-null `!`, `as` casts but `as const`, `@ts-ignore` / `@ts-expect-error`, `innerHTML` /
`outerHTML` / `insertAdjacentHTML`, `eval` / `new Function`, `console.log`, default exports and `enum`. Implicit any:
each file needs a tsconfig.json beside it or above it with `"strict": true` and no `"noImplicitAny": false`.
Exit 1 with `file:line: why` for each violation.
"""
import os
import re
import subprocess
import sys

STRING = re.compile(r"\"(\\.|[^\"\\])*\"|'(\\.|[^'\\])*'|`(\\.|[^`\\])*`")
DIRECTIVE = re.compile(r"@ts-(ignore|expect-error)")
BANS = [
    (re.compile(r"(?<![\w$.])any(?![\w$])"), "`any`: name the type"),
    (re.compile(r"[\w$)\]]!(?!=)"), "non-null `!`: check for null"),
    (re.compile(r"(?<![\w$.])as\s+(?!const\b)"), "`as` cast: narrow the type (only `as const`)"),
    (re.compile(r"\b(innerHTML|outerHTML|insertAdjacentHTML)\b"), "HTML from a string: build nodes"),
    (re.compile(r"(?<![\w$.])eval\s*\(|\bnew\s+Function\b"), "`eval` / `new Function`"),
    (re.compile(r"\bconsole\.log\b"), "`console.log`"),
    (re.compile(r"\bexport\s+default\b"), "default export: export by name"),
    (re.compile(r"(?<![\w$.])enum\s"), "`enum`: a union type or an `as const` object"),
]
ALIAS = re.compile(r"^\s*(import|export)\b.*\{[^}]*\bas\b")


def tsconfig(path):
    d = os.path.dirname(os.path.abspath(path))
    while True:
        c = os.path.join(d, "tsconfig.json")
        if os.path.exists(c):
            return c
        if os.path.dirname(d) == d:
            return None
        d = os.path.dirname(d)


def check(path):
    out = []
    for n, line in enumerate(open(path, encoding="utf-8").read().split("\n"), 1):
        if DIRECTIVE.search(line):
            out.append(f"{path}:{n}: `@ts-ignore` / `@ts-expect-error`: fix the type")
        code = STRING.sub('""', line.split("//", 1)[0])
        for pattern, why in BANS:
            if why.startswith("`as`") and ALIAS.search(code):
                continue
            if pattern.search(code):
                out.append(f"{path}:{n}: {why}")
    c = tsconfig(path)
    text = open(c, encoding="utf-8").read() if c else ""
    if not re.search(r'"strict"\s*:\s*true', text) or re.search(r'"noImplicitAny"\s*:\s*false', text):
        out.append(f"{path}: no tsconfig.json with \"strict\": true (implicit any)")
    return out


def main():
    paths = sys.argv[1:] or subprocess.run(
        ["git", "ls-files", "*.ts", "*.tsx"], capture_output=True, text=True, check=True
    ).stdout.split()
    bad = [v for p in paths for v in check(p)]
    for v in bad:
        print(v)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
