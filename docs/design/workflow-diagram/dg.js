// #213 mockup: the `diagram` block. herdsman sends Mermaid source (a stateDiagram-v2) with `counts` per state and
// `nodes` (a state's tone and tap, a timer's [limits] key, label and value). The native app parses that small subset
// and lays it out itself, no web view: a layered layout (rank by longest path, back edges reversed, long edges through
// bends, barycentre ordering), top to bottom on a phone and left to right when the sheet is wide. This is that layout,
// in small, for the frames. Styles (q947): Modern rounded, Terminal square and mono, Scene a horse acting each state.
const TONE = { working: 'working', needs: 'needs', done: 'done', fail: 'fail', idle: 'idle' };
const POSE = { working: 'edit', needs: 'needs', done: 'merged', fail: 'fail', idle: 'idle' };

// The subset: `A --> B`, `A --> B : label`, `[*]` (start before, end after), `state "Name" as id`, `state id`,
// comments. The declaration order is the reading order: an edge to a state declared earlier is drawn going back.
export function parse(src) {
  const names = {}, order = ['_start'], edges = [], add = id => { if (!order.includes(id)) order.push(id); return id; };
  for (const raw of src.split('\n')) {
    const l = raw.trim();
    let m;
    if (!l || l.startsWith('%%') || /^stateDiagram|^direction/.test(l)) continue;
    if ((m = l.match(/^state\s+"([^"]+)"\s+as\s+(\w+)/))) { names[m[2]] = m[1]; add(m[2]); continue; }
    if ((m = l.match(/^state\s+(\w+)$/))) { add(m[1]); continue; }
    if ((m = l.match(/^(\[\*\]|\w+)\s*-->\s*(\[\*\]|\w+)\s*(?::\s*(.+))?$/))) {
      const a = add(m[1] === '[*]' ? '_start' : m[1]), b = add(m[2] === '[*]' ? '_end' : m[2]);
      edges.push({ a, b, label: m[3] || '' });
    }
  }
  return { order: edges.some(e => e.a === '_start') ? order : order.slice(1), names, edges };
}

export function layout(g, size) {
  for (const e of g.edges) e.back = g.order.indexOf(e.b) <= g.order.indexOf(e.a);
  const fw = g.edges.map(e => e.back ? { ...e, a: e.b, b: e.a, e } : { ...e, e });
  const rank = Object.fromEntries(g.order.map(id => [id, 0]));
  for (let i = 0; i < g.order.length; i++) for (const e of fw) if (!e.e.back) rank[e.b] = Math.max(rank[e.b], rank[e.a] + 1);
  const R = [];
  const put = (r, item) => (R[r] ||= []).push(item);
  for (const id of g.order) put(rank[id], { id, ...size(id) });
  for (const f of fw) {
    f.via = [];
    for (let r = rank[f.a] + 1; r < rank[f.b]; r++) { const d = { id: `${f.a}>${f.b}@${r}`, dummy: true, w: 14, h: 14 }; put(r, d); f.via.push(d); }
  }
  const pos = new Map();
  const idx = () => R.forEach(row => row.forEach((n, i) => pos.set(n.id, i)));
  const ins = n => fw.flatMap(f => [f.a, ...f.via.map(v => v.id), f.b].flatMap((x, i, a) => a[i + 1] === n ? [x] : []));
  const outs = n => fw.flatMap(f => [f.a, ...f.via.map(v => v.id), f.b].flatMap((x, i, a) => a[i - 1] === n ? [x] : []));
  idx();
  for (let s = 0; s < 6; s++) {
    const down = s % 2 === 0, rows = down ? R.slice(1) : R.slice(0, -1).reverse();
    for (const row of rows) {
      const bc = n => { const nb = down ? ins(n.id) : outs(n.id); return nb.length ? nb.reduce((t, x) => t + pos.get(x), 0) / nb.length : pos.get(n.id); };
      row.sort((x, y) => bc(x) - bc(y)); idx();
    }
  }
  return { R, fw, rank };
}

export function diagram(b, { esc, STYLE, SPR, wide, words }) {
  const g = parse(b.source), nodes = b.nodes || {}, counts = b.counts || {}, was = b.was || {};
  const term = STYLE === 'terminal', scene = STYLE === 'scene', V = !wide;
  const isT = id => !!nodes[id]?.limit, label = id => g.names[id] || id;
  const size = id => id.startsWith('_') ? { w: 14, h: 14 } : { w: wide ? 112 : isT(id) ? 106 : 100, h: 52 };
  const { R, fw } = layout(g, size);
  const GAP = V ? 10 : 30, SEP = V ? 28 : 44, PAD = 12;
  const crossOf = n => V ? n.w : n.h, alongOf = n => V ? n.h : n.w;
  const span = Math.max(...R.map(row => row.reduce((t, n) => t + crossOf(n), 0) + GAP * (row.length - 1)));
  const at = new Map();
  let end = PAD - SEP;
  for (const row of R) {
    const deep = Math.max(...row.map(alongOf)), mid = end + SEP + deep / 2;
    let c = (span - (row.reduce((t, n) => t + crossOf(n), 0) + GAP * (row.length - 1))) / 2;
    for (const n of row) { at.set(n.id, { ...n, c: c + crossOf(n) / 2, r: mid }); c += crossOf(n) + GAP; }
    end = mid + deep / 2;
  }
  const XY = (c, r) => V ? [c + PAD, r] : [r, c + PAD], xy = (c, r) => XY(c, r).join(' ');
  const W = V ? span + 2 * PAD : end + PAD, H = V ? end + PAD : span + 2 * PAD;
  // Two edges both ways between two states (review ⇄ fix) are drawn as two curves side by side.
  const pair = f => fw.some(o => o !== f && o.a === f.a && o.b === f.b);
  const edges = fw.map(f => {
    const A = at.get(f.a), Bn = at.get(f.b), tl = isT(f.e.a) || isT(f.e.b), cls = `de${f.e.back ? ' bk' : ''}${tl ? ' tl' : ''}`;
    const lab = !f.e.label || (!wide && f.e.label.length > 11) ? '' : f.e.label;
    let d, lp;
    if (A.r === Bn.r) {
      // A back edge within one rank (verify → fix): from side to side, bowing past the rank.
      const [l, rt] = A.c < Bn.c ? [A, Bn] : [Bn, A], c0 = l.c + crossOf(l) / 2, c1 = rt.c - crossOf(rt) / 2, bow = alongOf(A) / 2 + 12;
      d = `M${xy(c0, A.r + 8)} C${xy(c0 + 18, A.r + bow)} ${xy(c1 - 18, A.r + bow)} ${xy(c1, A.r + 8)}`;
      lp = [(c0 + c1) / 2, A.r + bow - 2];
    } else {
      const pts = [f.a, ...f.via.map(v => v.id), f.b].map(id => at.get(id)), off = pair(f) ? (f.e.back ? -12 : 12) : 0;
      const P = pts.map((p, i) => [p.c + (i === 0 || i === pts.length - 1 ? off : 0), p.r + (i === 0 ? alongOf(p) / 2 : i === pts.length - 1 ? -alongOf(p) / 2 : 0)]);
      d = `M${xy(...P[0])}`;
      for (let i = 1; i < P.length; i++) { const [c0, r0] = P[i - 1], [c, r] = P[i], m = (r0 + r) / 2;
        d += term ? ` L${xy(c0, m)} L${xy(c, m)} L${xy(c, r)}` : ` C${xy(c0, m)} ${xy(c, m)} ${xy(c, r)}`; }
      const [s0, s1] = f.e.back ? [P[P.length - 1], P[P.length - 2]] : [P[0], P[1]];
      lp = [s0[0] + (s1[0] - s0[0]) * .62 + (V ? 6 : 0), s0[1] + (s1[1] - s0[1]) * .62 + (V ? 4 : -5)];
    }
    const [lx, ly] = XY(...lp);
    return `<path class="${cls}" d="${d}" marker-${f.e.back && A.r !== Bn.r ? 'start' : 'end'}="url(#dga)"/>${lab
      ? `<text class=del x="${lx}" y="${ly}"${V ? '' : ' text-anchor=middle'}>${esc(lab)}</text>` : ''}`;
  }).join('');
  const node = id => {
    const n = at.get(id), [x, y] = XY(n.c, n.r), x0 = x - n.w / 2, y0 = y - n.h / 2, s = nodes[id] || {}, rr = term ? 0 : isT(id) ? 26 : 12;
    if (id === '_start') return `<circle class=dst cx="${x}" cy="${y}" r=6 />`;
    if (id === '_end') return `<circle class=den cx="${x}" cy="${y}" r=7 /><circle class=dst cx="${x}" cy="${y}" r=4 />`;
    if (isT(id)) return `<g class=dt data-limit="${esc(s.limit)}" role=button aria-label="${esc(`${s.label} ${s.value}, opens its setting`)}"><rect x="${x0}" y="${y0}" width="${n.w}" height="${n.h}" rx="${rr}"/>
      <text class=dti x="${x0 + 9}" y="${y + 6}">${term ? '⧗' : '⏱'}</text><text class=dtl x="${x0 + 27}" y="${y - 4}">${esc(s.label)}</text><text class=dtv x="${x0 + 27}" y="${y + 14}">${esc(s.value)}</text></g>`;
    const k = counts[id] ?? 0, t = TONE[s.state] || 'idle', moved = id in was && was[id] !== k;
    const fig = scene ? `<image href="${SPR}horse-${s.coat || 'bay'}-${POSE[t]}.png" x="${x0 + n.w - 42}" y="${y - 6}" width="36" height="28" style="image-rendering:pixelated"><title>${esc(words[t] || t)}</title></image>` : '';
    return `<g class="dn${k ? '' : ' z'}${moved ? ' mv' : ''}${b.selected === id ? ' on' : ''}" data-tone=${t} role=button aria-label="${esc(`${label(id)}, ${k} issue${k === 1 ? '' : 's'}`)}">
      <rect x="${x0}" y="${y0}" width="${n.w}" height="${n.h}" rx="${rr}"/><text class=dnl x="${x0 + 10}" y="${y - 6}">${esc(label(id))}</text>
      <text class=dnc x="${x0 + 10}" y="${y + 16}">${term ? `[${k}]` : k}</text>${moved ? `<text class=dwas x="${x0 + 10 + (term ? 34 : 24)}" y="${y + 15}">was ${was[id]}</text>` : ''}${fig}</g>`;
  };
  return `<div class="dgm${wide ? ' wd2' : ''}" data-id="${esc(b.id || '')}"><svg viewBox="0 0 ${W} ${H}" width="${W}" height="${H}" role=img aria-label="${esc(b.title || 'workflow')}">
    <defs><marker id=dga viewBox="0 0 10 10" refX=9 refY=5 markerWidth=7 markerHeight=7 orient=auto-start-reverse><path d="M0 0 10 5 0 10z"/></marker></defs>
    ${edges}${g.order.map(node).join('')}</svg></div>`;
}
