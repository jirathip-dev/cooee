// #213 mockup: the `diagram` block. herdsman sends Mermaid source (a stateDiagram-v2) with `counts` per state and
// `nodes` (a state's tone and tap, a timer's [limits] key, label and value). The native app parses that small subset
// and lays it out itself, no web view, always top to bottom (q1298): one straight spine, the states beside it in two
// side columns, short unlabelled lines that never cross. Styles (q947): Modern rounded, Terminal square and mono, Scene
// a horse acting each state.
const TONE = { working: 'working', needs: 'needs', done: 'done', fail: 'fail', idle: 'idle' };
let seq = 0;
const POSE = { working: 'edit', needs: 'needs', done: 'merged', fail: 'fail', idle: 'idle' };

// The subset: `A --> B`, `A --> B : label` (the label is not drawn), `[*]`, `state "Name" as id`, `state id`,
// comments. The declaration order is the reading order: an edge to a state declared earlier goes back.
export function parse(src) {
  const names = {}, order = [], edges = [], add = id => { if (!order.includes(id)) order.push(id); return id; };
  for (const raw of src.split('\n')) {
    const l = raw.trim();
    let m;
    if (!l || l.startsWith('%%') || /^stateDiagram|^direction/.test(l)) continue;
    if ((m = l.match(/^state\s+"([^"]+)"\s+as\s+(\w+)/))) { names[m[2]] = m[1]; add(m[2]); continue; }
    if ((m = l.match(/^state\s+(\w+)$/))) { add(m[1]); continue; }
    if ((m = l.match(/^(\[\*\]|\w+)\s*-->\s*(\[\*\]|\w+)/))) {
      const a = m[1] === '[*]' ? '_start' : add(m[1]), b = m[2] === '[*]' ? '_end' : add(m[2]);
      edges.push({ a, b });
    }
  }
  return { order, names, edges };
}

// The rules, so the app needs no layout heuristic and herdsman's source alone decides the picture:
// 1. The spine is the longest forward path from the state `[*]` enters (the first edge wins a tie). It is one column.
// 2. A forward edge that jumps over spine states is not drawn; the states it jumps are marked "optional".
// 3. Every other state sits beside the first spine state it touches: right (side 1) if it leads back into the spine
//    (blocked, fix), left (side -1) if it leads away (the timers); one touching no spine state goes under its
//    predecessors (needs human). Two edges between the same states are one line with two heads.
export function layout(g) {
  const at = id => g.order.indexOf(id), fwd = g.edges.filter(e => at(e.a) >= 0 && at(e.b) > at(e.a));
  const len = {}, next = {};
  for (const id of [...g.order].reverse()) {
    len[id] = 1;
    for (const e of fwd) if (e.a === id && len[e.b] + 1 > len[id]) { len[id] = len[e.b] + 1; next[id] = e.b; }
  }
  const spine = [];
  for (let id = g.edges.find(e => e.a === '_start')?.b || g.order[0]; id; id = next[id]) spine.push(id);
  const row = {}, side = {}, optional = new Set(), taken = new Set();
  spine.forEach((id, i) => { row[id] = i; side[id] = 0; taken.add(`0:${i}`); });
  for (const e of fwd) {
    const i = spine.indexOf(e.a), j = spine.indexOf(e.b);
    if (i >= 0 && j > i + 1) spine.slice(i + 1, j).forEach(x => optional.add(x));
  }
  for (const id of g.order.filter(s => !spine.includes(s))) {
    const touch = g.edges.flatMap(e => e.a === id ? [e.b] : e.b === id ? [e.a] : []).filter(x => side[x] === 0);
    const pre = g.edges.filter(e => e.b === id && e.a in row).map(e => e.a);
    let s = 1, r = 0;
    if (touch.length) { r = Math.min(...touch.map(x => row[x])); s = g.edges.some(e => e.a === id && side[e.b] === 0) ? 1 : -1; }
    else if (pre.length) { r = Math.max(...pre.map(x => row[x])) + 1; s = side[pre[0]] || 1; }
    while (taken.has(`${s}:${r}`)) r++;
    row[id] = r; side[id] = s; taken.add(`${s}:${r}`);
  }
  const edges = [];
  for (const e of g.edges) {
    if (!(e.a in row && e.b in row) || (side[e.a] === 0 && side[e.b] === 0 && Math.abs(row[e.a] - row[e.b]) > 1)) continue;
    const twin = edges.find(d => d.a === e.b && d.b === e.a);
    if (twin) twin.both = true; else edges.push({ a: e.a, b: e.b });
  }
  return { spine, row, side, optional, edges };
}

// A polyline with its corners rounded by r (0 in Terminal).
const poly = (P, r) => P.reduce((d, [x, y], i) => {
  if (i === 0) return `M${x} ${y}`;
  if (i === P.length - 1 || !r) return `${d} L${x} ${y}`;
  const [px, py] = P[i - 1], [nx, ny] = P[i + 1], k = (a, b) => Math.sign(b - a) * Math.min(r, Math.abs(b - a) / 2);
  return `${d} L${x - k(px, x)} ${y - k(py, y)} Q${x} ${y} ${x + k(x, nx)} ${y + k(y, ny)}`;
}, '');

export function diagram(b, { esc, STYLE, SPR, wide, words }) {
  const g = parse(b.source), nodes = b.nodes || {}, counts = b.counts || {}, was = b.was || {};
  const term = STYLE === 'terminal', scene = STYLE === 'scene';
  const isT = id => !!nodes[id]?.limit, label = id => g.names[id] || id;
  const { row, side, optional, edges, spine } = layout(g);
  const used = new Set(Object.values(side));
  const CW = wide ? { '-1': 156, 0: 176, 1: 140 } : { '-1': 108, 0: 100, 1: 78 };
  const GAP = wide ? 40 : 22, GUT = wide ? 18 : 12, NH = wide ? 54 : 46, RG = wide ? 22 : 20, TOP = 34;
  const cols = [-1, 0, 1].filter(s => used.has(s));
  const cx = {};
  let x = GUT;
  for (const s of cols) { cx[s] = x + CW[s] / 2; x += CW[s] + GAP; }
  const W = x - GAP + GUT, rows = Math.max(...Object.values(row)) + 1, H = TOP + rows * (NH + RG) - RG + 12;
  const box = id => { const w = CW[side[id]], X = cx[side[id]], Y = TOP + row[id] * (NH + RG) + NH / 2; return { X, Y, w, l: X - w / 2, r: X + w / 2, t: Y - NH / 2, b: Y + NH / 2 }; };
  const R = term ? 0 : 8, mk = `dga${++seq}`;
  const lines = edges.map(({ a, b: to, both }) => {
    const A = box(a), B = box(to), cls = `de${isT(a) || isT(to) ? ' tl' : ''}`;
    let P;
    if (A.Y === B.Y) P = A.X < B.X ? [[A.r, A.Y], [B.l, B.Y]] : [[A.l, A.Y], [B.r, B.Y]];
    else if (A.X === B.X && Math.abs(row[a] - row[to]) === 1) P = A.Y < B.Y ? [[A.X, A.b], [B.X, B.t]] : [[A.X, A.t], [B.X, B.b]];
    else if (A.X === B.X) {
      // Down the outer gutter, past the states between them in that column.
      const o = side[a] < 0 ? A.l : A.r, gx = side[a] < 0 ? o - GUT + 3 : o + GUT - 3;
      P = [[o, A.Y], [gx, A.Y], [gx, B.Y], [o, B.Y]];
    } else if (side[a] === 0) P = [[B.X > A.X ? A.r : A.l, A.Y], [B.X, A.Y], [B.X, B.Y > A.Y ? B.t : B.b]];
    else P = [[A.X, B.Y > A.Y ? A.b : A.t], [A.X, B.Y], [B.X > A.X ? B.l : B.r, B.Y]];
    return `<path class="${cls}" d="${poly(P, R)}" marker-end="url(#${mk})"${both ? ` marker-start="url(#${mk})"` : ''}/>`;
  }).join('');
  const S = box(spine[0]);
  const start = `<circle class=dst cx="${S.X}" cy="10" r=6 /><path class=de d="M${S.X} 16 L${S.X} ${S.t}" marker-end="url(#${mk})"/>`;
  const node = id => {
    const n = box(id), s = nodes[id] || {}, rr = term ? 0 : isT(id) ? NH / 2 : 12;
    if (isT(id)) return `<g class=dt data-limit="${esc(s.limit)}" role=button aria-label="${esc(`${s.label} ${s.value}, opens its setting`)}"><rect x="${n.l}" y="${n.t}" width="${n.w}" height="${NH}" rx="${rr}"/>
      <text class=dti x="${n.l + 10}" y="${n.Y + 5}">${term ? '⧗' : '⏱'}</text><text class=dtl x="${n.l + 26}" y="${n.Y - 4}">${esc(s.label)}</text><text class=dtv x="${n.l + 26}" y="${n.Y + 13}">${esc(s.value)}</text></g>`;
    const k = counts[id] ?? 0, t = TONE[s.state] || 'idle', moved = id in was && was[id] !== k, cw = (term ? 10 : 12) * String(term ? `[${k}]` : k).length;
    const fig = scene && k ? `<image href="${SPR}horse-${s.coat || 'bay'}-${POSE[t]}.png" x="${n.r - 28}" y="${n.Y - 1}" width="24" height="18" style="image-rendering:pixelated"><title>${esc(words[t] || t)}</title></image>` : '';
    const tail = moved ? `<text class=dwas x="${n.l + 16 + cw}" y="${n.Y + 15}">was ${was[id]}</text>` : optional.has(id) ? `<text class=dop x="${n.l + 16 + cw}" y="${n.Y + 15}">optional</text>` : '';
    return `<g class="dn${k ? '' : ' z'}${moved ? ' mv' : ''}${b.selected === id ? ' on' : ''}" data-tone=${t} role=button aria-label="${esc(`${label(id)}, ${k} issue${k === 1 ? '' : 's'}${optional.has(id) ? ', optional' : ''}`)}">
      <rect x="${n.l}" y="${n.t}" width="${n.w}" height="${NH}" rx="${rr}"/><text class=dnl x="${n.l + 10}" y="${n.Y - 6}">${esc(label(id))}</text>
      <text class=dnc x="${n.l + 10}" y="${n.Y + 16}">${term ? `[${k}]` : k}</text>${tail}${fig}</g>`;
  };
  return `<div class="dgm${wide ? ' wd2' : ''}" data-id="${esc(b.id || '')}"><svg viewBox="0 0 ${W} ${H}" width="${W}" height="${H}" role=img aria-label="${esc(b.title || 'Workflow')}">
    <defs><marker id=${mk} viewBox="0 0 10 10" refX=9 refY=5 markerWidth=7 markerHeight=7 orient=auto-start-reverse><path d="M0 0 10 5 0 10z"/></marker></defs>
    ${start}${lines}${g.order.map(node).join('')}</svg></div>`;
}
