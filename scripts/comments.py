#!/usr/bin/env python3
"""The comment rule (#201) on every tracked Rust, Swift, TypeScript and JavaScript file, tests included.

No // or /* */ comments. A /// (TypeScript and JavaScript: /** */) doc comment only right above a public
declaration: Rust `pub`, Swift `public`/`open`, TypeScript `export`, or a case, field or member of such an
enum, trait, protocol or interface; Rust's //! only on a crate root or a `pub mod`. Allowed: `// SAFETY: ...`,
`// TODO(#N) ...` (`just comments` checks #N is open), Swift's `// swift-tools-version:` first line and Biome's
`// biome-ignore <rule>: <reason>`.
Exit 1 with `file:line: why` for each violation; paths default to `git ls-files`. `--todos` prints the N of
each `// TODO(#N)` comment instead.
"""
import re
import subprocess
import sys

EXTS = {".rs": "rust", ".swift": "swift", ".ts": "js", ".js": "js", ".mjs": "js"}
ALLOWED = re.compile(r"// (SAFETY: |TODO\(#[0-9]+\))")
PUBLIC = {
    "rust": re.compile(r"(pub|pub unsafe|pub const|pub async|pub extern \S+)\s"),
    "swift": re.compile(r"(@\S+\s+)*((final|static|nonisolated|@\S+)\s+)*(public|open)\s"),
    "js": re.compile(r"export\s"),
}
CONTAINER = {
    "rust": re.compile(r"pub (enum|trait)\s"),
    "swift": re.compile(r"(@\S+\s+)*(public|open) (enum|protocol)\s"),
    "js": re.compile(r"export (enum|interface|class|type)\s"),
}
ATTRIBUTE = {"rust": re.compile(r"#!?\["), "swift": re.compile(r"@\w+(\(.*\))?$"), "js": re.compile(r"@\w")}
REGEX_BEFORE = set("(,=:[!&|?{};+-*%<>~^")


class Lexer:
    def __init__(self, src, lang):
        self.s, self.lang, self.comments = src, lang, []

    def code(self, i, close=None):
        s, depth = self.s, 0
        while i < len(s):
            c, two = s[i], s[i : i + 2]
            if two == "//":
                end = s.find("\n", i)
                end = len(s) if end < 0 else end
                self.comments.append((i, end))
                i = end
            elif two == "/*":
                i = self.block(i)
            elif close and c in "({[":
                depth += 1
                i += 1
            elif close and c == close and depth == 0:
                return i + 1
            elif close and c in ")}]":
                depth -= 1
                i += 1
            elif self.lang == "js" and c == "/" and self.regex_here(i):
                i = self.regex(i)
            elif c in "\"'`#rb" and self.string_start(i):
                i = self.string(i)
            else:
                i += 1
        return i

    def regex_here(self, i):
        before = self.s[max(0, i - 200) : i].rstrip()
        return not before or before[-1] in REGEX_BEFORE or re.search(r"\b(return|typeof|case|in|of|void|yield|await)$", before) is not None

    def string_start(self, i):
        s, c = self.s, self.s[i]
        if c == '"' or (c == "`" and self.lang == "js"):
            return True
        if c == "'":
            return self.lang == "js" or (self.lang == "rust" and self.rust_char(i) is not None)
        if self.lang == "rust" and c in "rb":
            return re.match(r'(br|r|b)(#*)"', s[i:]) is not None and (i == 0 or not (s[i - 1].isalnum() or s[i - 1] == "_"))
        if self.lang == "swift" and c == "#":
            return re.match(r'#+"', s[i:]) is not None
        return False

    def rust_char(self, i):
        m = re.match(r"'(\\u\{[0-9a-fA-F]+\}|\\.|[^\\'\n])'", self.s[i:])
        return i + m.end() if m else None

    def block(self, i):
        start, depth, i = i, 1, i + 2
        while i < len(self.s) and depth:
            if self.s.startswith("*/", i):
                depth, i = depth - 1, i + 2
            elif self.s.startswith("/*", i) and self.lang != "js":
                depth, i = depth + 1, i + 2
            else:
                i += 1
        self.comments.append((start, i))
        return i

    def regex(self, i):
        s, i, cls = self.s, i + 1, False
        while i < len(s) and s[i] != "\n":
            if s[i] == "\\":
                i += 2
                continue
            if s[i] == "[":
                cls = True
            elif s[i] == "]":
                cls = False
            elif s[i] == "/" and not cls:
                return i + 1
            i += 1
        return i

    def string(self, i):
        s = self.s
        if s[i] == "'" and self.lang == "rust":
            return self.rust_char(i)
        if self.lang == "rust":
            m = re.match(r'(br|r|b)?(#*)"', s[i:])
            raw, pounds = "r" in (m.group(1) or ""), m.group(2)
            i += m.end()
            while i < len(s):
                if s[i] == "\\" and not raw:
                    i += 2
                elif s.startswith('"' + pounds, i):
                    return i + 1 + len(pounds)
                else:
                    i += 1
            return i
        if self.lang == "swift":
            m = re.match(r'(#*)("""|")', s[i:])
            pounds, quote = m.group(1), m.group(2)
            i += m.end()
            escape = "\\" + pounds
            while i < len(s):
                if s.startswith(escape + "(", i):
                    i = self.code(i + len(escape) + 1, ")")
                elif s.startswith(escape, i):
                    i += len(escape) + 1
                elif s.startswith(quote + pounds, i):
                    return i + len(quote) + len(pounds)
                else:
                    i += 1
            return i
        quote, i = s[i], i + 1
        while i < len(s):
            if s[i] == "\\":
                i += 2
            elif quote == "`" and s.startswith("${", i):
                i = self.code(i + 2, "}")
            elif s[i] == quote:
                return i + 1
            else:
                i += 1
        return i


def public(lines, n, lang):
    while n < len(lines) and (not lines[n].strip() or ATTRIBUTE[lang].match(lines[n].strip()) or lines[n].strip().startswith("///")):
        if lang == "rust" and lines[n].strip().startswith("#[") and not lines[n].rstrip().endswith("]"):
            while n < len(lines) and not lines[n].rstrip().endswith("]"):
                n += 1
        n += 1
    if n >= len(lines):
        return False
    line = lines[n]
    if PUBLIC[lang].match(line.strip()):
        return True
    indent = len(line) - len(line.lstrip())
    for above in reversed(lines[:n]):
        if above.strip() and len(above) - len(above.lstrip()) < indent:
            if lang == "rust" and re.match(r"[A-Z]\w* \{$", above.strip()):
                indent = len(above) - len(above.lstrip())
                continue
            return CONTAINER[lang].match(above.strip()) is not None
    return False


def public_module(path):
    if re.search(r"(^|/)(src/(lib|main)\.rs|build\.rs|(tests|examples|benches|src/bin)/[^/]+\.rs)$", path):
        return True
    parts = path.split("/")
    name, parent = (parts[-2], parts[:-2]) if parts[-1] == "mod.rs" else (parts[-1][:-3], parts[:-1])
    candidates = ["/".join(parent + [f]) for f in ("mod.rs", "lib.rs", "main.rs")] + ["/".join(parent) + ".rs"]
    for candidate in candidates:
        try:
            with open(candidate, encoding="utf-8") as f:
                if re.search(rf"^\s*pub mod {name}\s*;", f.read(), re.M):
                    return True
        except OSError:
            continue
    return False


def violations(path, src):
    lang = EXTS["." + path.rsplit(".", 1)[-1]]
    lexer = Lexer(src, lang)
    lexer.code(0)
    lines, found = src.split("\n"), []
    for start, end in lexer.comments:
        text = src[start:end]
        line = src.count("\n", 0, start)
        leading = not src[src.rfind("\n", 0, start) + 1 : start].strip()
        doc = (text.startswith("///") and not text.startswith("////")) or (
            lang == "js" and text.startswith("/**") and not text.startswith("/**/")
        )
        if ALLOWED.match(text) and leading:
            continue
        if lang == "swift" and start == 0 and text.startswith("// swift-tools-version:"):
            continue
        if lang == "js" and re.match(r"// biome-ignore \S+: \S", text) and leading:
            continue
        if lang == "rust" and text.startswith("//!") and leading and public_module(path):
            continue
        if doc and leading and public(lines, src.count("\n", 0, end) + 1, lang):
            continue
        why = "doc comment on a non-public item" if doc else "comment"
        found.append((path, line + 1, end, why, text.split("\n")[0]))
    return found


def todos(paths):
    numbers = set()
    for path in paths:
        with open(path, encoding="utf-8") as f:
            src = f.read()
        lexer = Lexer(src, EXTS["." + path.rsplit(".", 1)[-1]])
        lexer.code(0)
        numbers |= {m.group(1) for s, e in lexer.comments for m in [re.match(r"// TODO\(#([0-9]+)\)", src[s:e])] if m}
    print("\n".join(sorted(numbers, key=int)))
    return 0


def main(args):
    paths = [a for a in args if a != "--todos"]
    if not paths:
        out = subprocess.run(["git", "ls-files", *("*" + e for e in EXTS)], capture_output=True, text=True, check=True)
        paths = out.stdout.split()
    if "--todos" in args:
        return todos(paths)
    found = []
    for path in paths:
        with open(path, encoding="utf-8") as f:
            found += violations(path, f.read())
    for path, line, _, why, text in found:
        print(f"{path}:{line}: {why}: {text.strip()}")
    if found:
        print("comments break the rule (#201): say it in a name, a type or a test; /// only on public API")
        return 1
    print("no comments break the rule")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
