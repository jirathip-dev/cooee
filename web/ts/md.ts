// Markdown for the agent's replies (#43), hand-written: it builds DOM nodes (createElement, textContent), never
// HTML, so every character of the text is escaped by construction. Blocks: paragraphs, headings, fenced code (its
// language and a [copy] button, chat.ts's data-copy), ordered and unordered lists, blockquotes, rules and GitHub
// tables (in a box that scrolls sideways); inline: bold, italic, code and links, of http and https only (a new tab,
// rel=noopener). Raw HTML and images stay text. `settled` is how much of a streaming reply is whole blocks.

const el = (tag: string, cls = "", text?: string): HTMLElement => {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text !== undefined) e.textContent = text;
  return e;
};

const FENCE = /^ {0,3}(`{3,}|~{3,})\s*([^`\s]*)/;
const HEADING = /^ {0,3}(#{1,6})\s+(.*?)(?:\s+#+)?\s*$/;
const RULE = /^ {0,3}([-*_])(?:\s*\1){2,}\s*$/;
const QUOTE = /^ {0,3}> ?/;
const ITEM = /^( {0,3})([-*+]|(\d{1,9})[.)])(\s+|$)/;
const DELIM = /^\s*\|?\s*:?-+:?\s*(\|\s*:?-+:?\s*)*\|?\s*$/;

/// `src` as nodes.
export function render(src: string): DocumentFragment {
  const out = document.createDocumentFragment();
  blocks(src.replace(/\r\n?/g, "\n").split("\n"), out);
  return out;
}

/// Where the whole blocks of a reply still streaming end: after its last blank line outside a code fence.
export function settled(src: string): number {
  let at = 0;
  let pos = 0;
  let fence = "";
  for (const line of src.split("\n")) {
    const end = pos + line.length + 1;
    const f = line.match(FENCE)?.[1];
    if (fence ? f?.startsWith(fence) && !line.trim().slice(f.length) : f) fence = fence ? "" : (f ?? "");
    else if (!fence && !line.trim() && end <= src.length) at = end;
    pos = end;
  }
  return at;
}

const starts = (line: string): boolean =>
  FENCE.test(line) || HEADING.test(line) || RULE.test(line) || QUOTE.test(line) || ITEM.test(line);

function blocks(lines: string[], into: Node): void {
  let i = 0;
  const at = (n: number): string => lines[n] ?? "";
  while (i < lines.length) {
    const line = at(i);
    const f = line.match(FENCE);
    const h = line.match(HEADING);
    if (!line.trim()) {
      i++;
    } else if (f) {
      const fence = f[1] ?? "";
      const body: string[] = [];
      for (i++; i < lines.length; i++) {
        const close = at(i).trim();
        if (close.startsWith(fence) && !close.slice(fence.length).replace(/[`~]/g, "")) break;
        body.push(at(i));
      }
      i++;
      into.appendChild(code(body.join("\n"), f[2] ?? ""));
    } else if (h) {
      const head = el(`h${(h[1] ?? "#").length}`);
      inline(h[2] ?? "", head);
      into.appendChild(head);
      i++;
    } else if (RULE.test(line)) {
      into.appendChild(el("hr"));
      i++;
    } else if (QUOTE.test(line)) {
      const inner: string[] = [];
      for (; i < lines.length && at(i).trim() && (QUOTE.test(at(i)) || !starts(at(i))); i++)
        inner.push(at(i).replace(QUOTE, ""));
      const q = el("blockquote");
      blocks(inner, q);
      into.appendChild(q);
    } else if (ITEM.test(line)) {
      i = list(lines, i, into);
    } else if (line.includes("|") && DELIM.test(at(i + 1)) && cells(line).length === cells(at(i + 1)).length) {
      i = table(lines, i, into);
    } else {
      const para: string[] = [];
      for (; i < lines.length && at(i).trim() && (para.length === 0 || !starts(at(i))); i++) para.push(at(i).trim());
      const p = el("p");
      inline(para.join("\n"), p);
      into.appendChild(p);
    }
  }
}

/// A list from line `i`; returns the line after it. Its items' lines are indented past the marker (or lazy
/// continuations of their text); a blank line between items makes it loose (each item's text a paragraph).
function list(lines: string[], i: number, into: Node): number {
  const first = (lines[i] ?? "").match(ITEM);
  const ordered = first?.[3] !== undefined;
  const box = el(ordered ? "ol" : "ul") as HTMLOListElement;
  if (ordered && Number(first?.[3]) !== 1) box.start = Number(first?.[3]);
  let loose = false;
  while (i < lines.length) {
    const m = (lines[i] ?? "").match(ITEM);
    if (!m || (m[3] !== undefined) !== ordered) break;
    const pad = (m[0] ?? "").length;
    const body = [(lines[i] ?? "").slice(pad)];
    for (i++; i < lines.length; i++) {
      const line = lines[i] ?? "";
      const indent = line.length - line.trimStart().length;
      if (!line.trim()) {
        const next = lines[i + 1] ?? "";
        const nextIndent = next.length - next.trimStart().length;
        if (next.trim() && nextIndent >= Math.min(pad, 4)) body.push("");
        else break;
      } else if (indent >= Math.min(pad, 4)) body.push(line.slice(Math.min(indent, pad)));
      else if (!starts(line) && body.at(-1)) body.push(line.trim());
      else break;
    }
    const next = (lines[i + 1] ?? "").match(ITEM);
    if (!(lines[i] ?? "").trim() && next && (next[3] !== undefined) === ordered) {
      loose = true;
      i++;
    }
    const li = el("li");
    blocks(body, li);
    box.appendChild(li);
  }
  // A tight list's items hold their text, not paragraphs.
  if (!loose) for (const p of box.querySelectorAll(":scope > li > p")) p.replaceWith(...p.childNodes);
  into.appendChild(box);
  return i;
}

/// A row's cells: split on `|` outside code and escapes, the outer pipes dropped.
function cells(line: string): string[] {
  const out: string[] = [];
  let cell = "";
  let tick = false;
  const s = line.trim().replace(/^\|/, "");
  for (let i = 0; i < s.length; i++) {
    const c = s[i] ?? "";
    if (c === "\\" && s[i + 1] === "|") {
      cell += "|";
      i++;
    } else if (c === "`") {
      tick = !tick;
      cell += c;
    } else if (c === "|" && !tick) {
      out.push(cell.trim());
      cell = "";
    } else cell += c;
  }
  if (cell.trim() || !s.endsWith("|")) out.push(cell.trim());
  return out;
}

function table(lines: string[], i: number, into: Node): number {
  const align = cells(lines[i + 1] ?? "").map((d) =>
    d.endsWith(":") ? (d.startsWith(":") ? "center" : "right") : d.startsWith(":") ? "left" : "",
  );
  const row = (line: string, tag: string): HTMLElement => {
    const tr = el("tr");
    const cs = cells(line);
    align.forEach((a, n) => {
      const c = el(tag);
      if (a) c.style.textAlign = a;
      inline(cs[n] ?? "", c);
      tr.appendChild(c);
    });
    return tr;
  };
  const t = el("table");
  const head = el("thead");
  head.appendChild(row(lines[i] ?? "", "th"));
  const body = el("tbody");
  for (i += 2; i < lines.length && (lines[i] ?? "").includes("|") && (lines[i] ?? "").trim(); i++)
    body.appendChild(row(lines[i] ?? "", "td"));
  t.append(head, body);
  const box = el("div", "tbl");
  box.appendChild(t);
  into.appendChild(box);
  return i;
}

/// A fenced block: its language (when the fence names one), [copy] and the code, as it is.
function code(text: string, lang: string): HTMLElement {
  const box = el("div", "cb");
  const bar = el("div", "cbh");
  const copy = el("button", "cp", "copy");
  copy.setAttribute("type", "button");
  copy.setAttribute("aria-label", lang ? `copy the ${lang} code` : "copy the code");
  copy.dataset.copy = text;
  bar.append(el("span", "", lang), copy);
  const pre = el("pre");
  pre.appendChild(el("code", "", text));
  box.append(bar, pre);
  return box;
}

const link = (url: string): HTMLAnchorElement => {
  const a = el("a") as HTMLAnchorElement;
  a.href = url;
  a.target = "_blank";
  a.rel = "noopener";
  return a;
};

// Each is tried at a character that can start it (`prev`: the one before); `y` matches at lastIndex only.
type Rule = [(c: string, prev: string) => boolean, RegExp, (m: RegExpExecArray) => Node];
const wrap = (tag: string, m: RegExpExecArray): Node => {
  const e = el(tag);
  inline(m[2] ?? "", e);
  return e;
};
const RULES: Rule[] = [
  [(c) => c === "`", /(`+)([\s\S]*?[^`])\1(?!`)/y, (m) => el("code", "", (m[2] ?? "").replace(/^ (.+) $/, "$1"))],
  [
    (c, prev) => c === "[" && prev !== "!",
    /\[([^\]\n]+)\]\((https?:\/\/[^\s()]+)\)/y,
    (m) => {
      const a = link(m[2] ?? "");
      inline(m[1] ?? "", a);
      return a;
    },
  ],
  [
    (c, prev) => c === "h" && !/\w/.test(prev),
    /https?:\/\/[^\s<>]*[^\s<>.,;:!?)\]'"]/y,
    (m) => {
      const a = link(m[0]);
      a.textContent = m[0];
      return a;
    },
  ],
  [(c) => c === "*" || c === "_", /(\*\*|__)(?=\S)([\s\S]*?\S)\1/y, (m) => wrap("strong", m)],
  [
    (c, prev) => c === "*" || (c === "_" && !/\w/.test(prev)),
    /([*_])(?=[^\s*_])([\s\S]*?[^\s*_])\1(?![*_\w])/y,
    (m) => wrap("em", m),
  ],
];

function inline(s: string, into: Node): void {
  let text = "";
  const flush = (): void => {
    if (text) into.appendChild(document.createTextNode(text));
    text = "";
  };
  for (let i = 0; i < s.length; ) {
    const c = s[i] ?? "";
    const prev = s[i - 1] ?? "";
    if (c === "\\" && /[\\`*_[\]()#+\-.!|>~]/.test(s[i + 1] ?? "")) {
      text += s[i + 1];
      i += 2;
      continue;
    }
    if (c === "\n") {
      flush();
      into.appendChild(el("br"));
      i++;
      continue;
    }
    let made: [number, Node] | null = null;
    for (const [can, re, make] of RULES) {
      if (!can(c, prev)) continue;
      re.lastIndex = i;
      const m = re.exec(s);
      if (m) {
        made = [m[0].length, make(m)];
        break;
      }
    }
    if (made) {
      flush();
      into.appendChild(made[1]);
      i += made[0];
    } else {
      text += c;
      i++;
    }
  }
  flush();
}
