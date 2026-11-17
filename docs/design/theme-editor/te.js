import { gate } from '../look/gate.mjs';

const q = new URLSearchParams(location.search);
const STYLE = q.get('style') || 'modern', F = q.get('f');
const MODE = q.get('mode') || (STYLE === 'scene' ? 'light' : 'dark');
const T = STYLE === 'terminal';
const dev = await fetch(`../../../design-system/schemes/${MODE === 'dark' ? 'farm' : 'farm-light'}.json`).then(r => r.json());
const root = document.documentElement;
for (const [k, v] of Object.entries(dev.tokens)) root.style.setProperty(k, v);
root.style.colorScheme = MODE;
Object.assign(root.dataset, { style: T ? 'terminal' : 'modern', mode: MODE });
if (F) { root.dataset.one = ''; document.querySelector(`[data-f="${F}"]`)?.closest('.fr').classList.add('one'); }

export const TIDE = {
  dark: { '--bg': '#0b1a22', '--card': '#12262f', '--edge': '#2a4a55', '--line': '#173039', '--scrim': '#00080caa', '--fg': '#dcebee', '--mute': '#93b3bb', '--l': '#5fd6c8', '--link': '#6cc7e8', '--g': '#9bd86b', '--a': '#f2c14e', '--r': '#ff6b8a', '--p': '#c3a6ff', '--c': '#8fb8ff', '--fk-l': '#ff9e80', '--fk-p': '#c3a6ff', '--fk-c': '#4fd1c5', '--fk-g': '#6fd39a', '--fk-a': '#f2c14e' },
  light: { '--bg': '#f2f7f6', '--card': '#e4eeec', '--edge': '#b9cfcc', '--line': '#d4e3e0', '--scrim': '#0b1a2266', '--fg': '#13262d', '--mute': '#4a6169', '--l': '#0b6870', '--link': '#1b6a8a', '--g': '#1f7a45', '--a': '#a88a2a', '--r': '#c0203f', '--p': '#6f45b8', '--c': '#2c55a8', '--fk-l': '#a8401f', '--fk-p': '#6f45b8', '--fk-c': '#0b6e75', '--fk-g': '#1f7a45', '--fk-a': '#7a5c00' },
};
const FIXED = { ...TIDE.light, '--a': '#8a6a00' };
const tokens = (mode, fixed) => mode === 'dark' ? TIDE.dark : fixed ? FIXED : TIDE.light;
const vars = t => Object.entries(t).map(([k, v]) => `${k}:${v}`).join(';') + ';' + inks(t);

const BODY = ['..c..........c..', '.cc..........cc.', '.c.c........c.c.', '..cc..w..w..cc..', '....c.k..k.c....', '.....cccccc.....',
  '...cccccccccc...', '..cccccccccccc..', '.cccccccccccccc.', '..cccccccccccc..', '..c.c.c..c.c.c..', '.c..c..c..c..c..'];
const UP = ['.c............c.', '.cc..........cc.', '..cc........cc..'];
const EYES = { idle: ['..cc........cc..', '....ckk..kkc....'], fail: ['..cc.x.x..x.x.cc', '....c.x..x.c....'] };
const BADGE = {
  needs: [[11, 0], [12, 0], [11, 1], [12, 1], [11, 2], [12, 2], [11, 4], [12, 4]].map(p => [...p, 'a']),
  done: [[8, 4], [9, 5], [10, 6], [11, 5], [12, 4], [13, 3], [14, 2]].map(p => [...p, 'g']),
  merged: [[12, 1], [11, 2], [12, 2], [13, 2], [12, 3], [10, 0], [14, 4]].map(p => [...p, 'p']),
  run: [[10, 1], [11, 2], [12, 3], [11, 4], [10, 5]].map(p => [...p, 'n']),
  read: [[9, 3], [10, 2], [11, 2], [12, 3], [11, 4], [10, 4], [14, 5]].map(p => [...p, 'n']),
  fail: [],
};
const INK = { c: 'var(--ik-l)', w: '#ffffff', k: '#10161a', x: 'var(--ik-r)', a: 'var(--ik-a)', g: 'var(--ik-g)', p: 'var(--ik-p)', n: 'var(--ik-c)' };
const inks = t => ['l', 'a', 'g', 'p', 'c'].map(k => `--ik-${k}:${t[`--fk-${k}`]}`).concat(`--ik-r:${t['--r']}`).join(';');
export const STATES = ['edit', 'read', 'run', 'idle', 'done', 'needs', 'merged', 'fail'];

function cells(state, frame) {
  const rows = [...BODY];
  if (state === 'edit' && frame === 2 || state === 'done' || state === 'needs' && frame === 2) rows.splice(0, 3, ...UP);
  if (EYES[state]) rows.splice(3, 2, ...EYES[state]);
  const out = [];
  rows.forEach((r, y) => [...r].forEach((ch, x) => ch !== '.' && out.push([x + 4, y + 8, ch])));
  return out.concat(BADGE[state] || []);
}
export function crab(state, frame = 1, cls = 'spr') {
  const r = cells(state, frame).map(([x, y, ch]) => `<rect x="${x}" y="${y}" width="1" height="1" fill="${INK[ch]}"/>`).join('');
  return `<svg class="${cls}" viewBox="0 0 24 24" shape-rendering="crispEdges" aria-hidden="true">${r}</svg>`;
}
export function grid(state, frame, onion) {
  const now = cells(state, frame), was = onion ? cells(state, onion) : [];
  const key = ([x, y]) => `${x},${y}`, has = new Set(now.map(key));
  const ghost = was.filter(p => !has.has(key(p))).map(([x, y]) => `<rect x="${x}" y="${y}" width="1" height="1" fill="var(--ik-l)" opacity=".22"/>`).join('');
  const lines = [...Array(25)].map((_, i) => `<path d="M${i} 0V24M0 ${i}H24"/>`).join('');
  return `<svg class="cnv" viewBox="0 0 24 24" shape-rendering="crispEdges">${ghost}${now.map(([x, y, ch]) => `<rect x="${x}" y="${y}" width="1" height="1" fill="${INK[ch]}"/>`).join('')}<g class="gl2">${lines}</g><rect class="cur" x="12" y="1" width="1" height="1"/></svg>`;
}

export const SCENE = `<svg class="bk2" viewBox="0 0 390 188" preserveAspectRatio="xMidYMax slice" aria-hidden="true">
<g id="sky"><rect width="390" height="188" fill="var(--card)"/><circle cx="330" cy="38" r="16" fill="var(--fk-a)" opacity=".85"/></g>
<g id="sea"><path d="M0 64q24-10 48 0t48 0 48 0 48 0 48 0 48 0 48 0 48 0 48 0v66H0z" fill="var(--fk-c)" opacity=".5"/><path d="M0 84q30-8 60 0t60 0 60 0 60 0 60 0 60 0 60 0v40H0z" fill="var(--fk-c)" opacity=".35"/></g>
<g id="rocks"><path d="M18 134q10-34 44-30 26 4 30 30z" fill="var(--edge)"/><path d="M286 136q8-44 52-40 34 6 36 40z" fill="var(--edge)"/><circle cx="318" cy="112" r="5" fill="var(--fk-p)" opacity=".7"/></g>
<g id="sand"><path d="M0 128q195-26 390 0v60H0z" fill="var(--fk-a)" opacity=".32"/><path d="M60 160h6M150 170h4M250 158h7M330 172h5" stroke="var(--mute)" stroke-width="2"/></g></svg>`;

export const WORDS = { working: 'scuttling', needs: 'waving', done: 'back in the pool', fail: 'washed up', idle: 'basking', merged: 'out to sea', 'empty-threads': 'the pool is still' };
const AG = [
  ['Chief', 'needs', 'needs', 'Send the Q3 deck to the board?', '1m'],
  ['Coder', 'working', 'edit', 'Edit src/export.rs', 'now'],
  ['Researcher', 'done', 'done', 'Q3 revenue $4.1M, up 12% on Q2.', '12m'],
  ['Account Manager', 'idle', 'idle', 'A 3-year term at 18% off list…', '1h'],
];
const SK = { needs: 'r', working: 'a', done: 'g', idle: 'm' };

function pv(e) {
  const style = e.dataset.ds || (STYLE === 'scene' ? 'scene' : STYLE), mode = e.dataset.mode || MODE, what = e.dataset.what || 'home';
  const fixed = 'fixed' in e.dataset, rows = Number(e.dataset.rows || 4);
  const fig = (k, cls) => style === 'terminal' && 'ascii' in e.dataset ? '' : crab(k, 1, cls);
  const row = ([n, w, k, l, a]) => `<div class="prw" data-k="${SK[w]}"><span class="pva">${fig(k)}</span><span class="pnm"><b>${n}</b><small>${a}</small></span>`
    + `<span class="pln"><em>${WORDS[w]}${w === 'needs' ? ' · needs you' : ''}</em> · ${l}</span></div>`;
  const scene = () => `<div class="psc">${SCENE}<div class="crabs">${AG.slice(0, 3).map(([n, w, k]) => `<span class="crb" data-k="${SK[w]}">${crab(k, 1, 'csb')}<span class="ncd"><b>${n}</b><small>${WORDS[w]}</small></span></span>`).join('')}</div></div>`;
  const home = (style === 'scene' ? scene() : '') + `<div class="phd">${style === 'terminal' ? '$ chats' : 'Chats'}</div>` + AG.slice(0, rows).map(row).join('');
  const chat = `<div class="pch"><p class="pme">Draft the board deck from export/summary.md.</p><p class="pai">Slides 4–9 are ready in deck/q3.key.</p>`
    + `<div class="pak"><span class="pkx">${fig('needs', 'spr sm')}<em>${WORDS.needs} · needs you</em></span><b>Send the Q3 deck to the board?</b><span class="pbt"><i>Send</i><i>Not yet</i></span></div></div>`;
  e.dataset.ds = style;
  e.setAttribute('style', vars(tokens(mode, fixed)) + ';' + (e.getAttribute('style') || ''));
  e.innerHTML = what === 'chat' ? chat : what === 'both' ? home + chat : home;
}

function tiles(e) {
  const pick = e.dataset.sel;
  const t = [...STATES.map(k => [k, crab(k), k === 'edit' || k === 'needs' ? '2 frames' : '']), ['loading', crab('run', 1), '132 × 100'], ['empty', crab('idle', 1), '132 × 100']];
  return t.map(([k, s, n]) => `<a class="tl${pick === k ? ' on' : ''}" href="#">${s}<b>s-${k}</b>${n ? `<small>${n}</small>` : ''}</a>`).join('');
}

const NAME = { '--bg': 'background', '--card': 'cards', '--edge': 'edges', '--line': 'lines', '--fg': 'text', '--mute': 'quiet text', '--l': 'accent', '--link': 'links', '--g': 'done', '--a': 'working', '--r': 'needs you · failed', '--p': 'purple', '--c': 'blue', '--fk-l': 'figure 1', '--fk-c': 'figure 2', '--fk-a': 'figure 3', '--fk-g': 'figure 4', '--fk-p': 'figure 5' };
function toks(e) {
  const t = tokens(e.dataset.mode || 'light', 'fixed' in e.dataset), rows = gate({ tokens: t });
  const worst = k => rows.filter(([n]) => n.startsWith(`${k} on`)).sort((a, b) => a[1] / a[2] - b[1] / b[2])[0];
  return e.dataset.keys.split(' ').map(k => {
    const w = worst(k), bad = w && w[1] < w[2], open = e.dataset.open === k;
    const res = w ? `<span class="gt2${bad ? ' bad' : ''}">${w[1].toFixed(1)}:1 ${bad ? `✗ needs ${w[2]}` : '✓'}</span>` : '<span class="gt2 m">—</span>';
    return `<div class="tkr${open ? ' op' : ''}"><i class="sw2" style="background:${t[k]}"></i><span class="tkn"><b>${NAME[k]}</b><small>${k} · ${t[k]}</small></span>${res}</div>`
      + (open ? `<div class="tko"><p>${w[0].replace(k, NAME[k])}: ${w[1].toFixed(2)}:1, the gate needs ${w[2]}:1.</p><div class="hsl"><label>Hue<i style="--v:14%"></i></label><label>Light<i style="--v:62%"></i></label></div><span class="fx"><button type="button">Darken to #8a6a00 · ${fixw()}</button></span></div>` : '');
  }).join('');
}
function fixw() {
  const r = gate({ tokens: FIXED }).filter(([n]) => n.startsWith('--a on')).sort((a, b) => a[1] / a[2] - b[1] / b[2])[0];
  return `${r[1].toFixed(1)}:1 ✓`;
}
function gsum(e) {
  const m = e.dataset.mode || 'light', bad = [...new Set(gate({ tokens: tokens(m, 'fixed' in e.dataset) }).filter(([, v, n]) => v < n).map(([n]) => n.match(/--[\w-]+/)[0]))];
  return bad.length ? `<span class="gs3 bad">✗ ${bad.length} to fix</span>` : '<span class="gs3">✓ passes</span>';
}
function swatches(e) {
  const t = tokens(MODE), on = e.dataset.sel || '--fk-l';
  return ['--fk-l', '--fk-c', '--fk-a', '--fk-g', '--fk-p', '--r', '--fg', '--bg'].map(k => `<i class="pal${k === on ? ' on' : ''}" style="background:${t[k]}" title="${k}"></i>`).join('')
    + '<i class="pal" style="background:#fff"></i><i class="pal" style="background:#10161a"></i>';
}
const I = {
  pen: '<path d="M4 20l4-1 11-11-3-3L5 16zM14 6l3 3"/>', fill: '<path d="M5 12l7-7 7 7-7 7zM19 15c1 2 2 3 0 4-2-1-1-2 0-4z"/>',
  erase: '<path d="M8 20h12M5 15l8-8 6 6-5 5H9z"/>', line: '<path d="M5 19L19 5"/>', pick: '<path d="M14 4l6 6-9 9H5v-6z"/>',
  undo: '<path d="M9 7L4 12l5 5M4 12h11a5 5 0 0 1 0 10"/>', imp: '<path d="M12 4v11M7 10l5 5 5-5M5 20h14"/>', mir: '<path d="M12 3v18M8 7L4 12l4 5M16 7l4 5-4 5"/>',
};
const ico = k => `<svg class="ic2" viewBox="0 0 24 24">${I[k]}</svg>`;
function tools(e) {
  const on = e.dataset.sel || 'pen', tx = { pen: 'pen', fill: 'fill', erase: 'erase', line: 'line', mir: 'mirror', pick: 'pick', undo: 'undo', imp: 'import' };
  return e.dataset.ks.split(' ').map(k => `<a class="tlb${k === on ? ' on' : ''}" href="#">${T ? `<code>${tx[k]}</code>` : `${ico(k)}<small>${tx[k]}</small>`}</a>`).join('');
}
function frames(e) {
  const s = e.dataset.st || 'needs', on = Number(e.dataset.sel || 2);
  return [1, 2].map(n => `<a class="fm${n === on ? ' on' : ''}" href="#">${crab(s, n)}<small>${n}</small></a>`).join('') + '<a class="fm add" href="#">+</a>';
}
function strip(e) {
  const on = e.dataset.sel;
  return STATES.map(k => `<a class="stc${k === on ? ' on' : ''}" href="#">${crab(k, 1)}<small>${k}</small></a>`).join('');
}

root.setAttribute('style', root.getAttribute('style') + ';' + inks(MODE === 'dark' ? TIDE.dark : FIXED));
const SB = `<span>9:41</span><span class=si><svg viewBox="0 0 18 12"><rect y="8" width="3" height="4" rx=".7"/><rect x="5" y="5.5" width="3" height="6.5" rx=".7"/><rect x="10" y="3" width="3" height="9" rx=".7"/><rect x="15" width="3" height="12" rx=".7"/></svg><svg viewBox="0 0 27 12" style="height:13px"><rect x=".5" y=".5" width="23" height="11" rx="3.5" fill="none" stroke="currentColor" opacity=".4"/><rect x="2" y="2" width="18" height="8" rx="2"/></svg></span>`;
for (const p of document.querySelectorAll('.ph')) p.insertAdjacentHTML('beforeend', `<div class=stb>${SB}</div><i class=isl></i><i class=hind></i>`);
const X = { tiles, toks, gsum, swatches, tools, frames, strip };
for (const e of document.querySelectorAll('[data-y]')) e.innerHTML = X[e.dataset.y](e);
for (const e of document.querySelectorAll('.pv')) pv(e);
for (const e of document.querySelectorAll('[data-crab]')) e.innerHTML = crab(...e.dataset.crab.split(':').map((v, i) => i ? Number(v) : v));
for (const e of document.querySelectorAll('[data-grid]')) { const [s, f, o] = e.dataset.grid.split(':'); e.innerHTML = grid(s, Number(f), Number(o)); }
for (const e of document.querySelectorAll('[data-scene]')) { e.innerHTML = SCENE; e.setAttribute('style', vars(tokens(MODE))); }
for (const e of document.querySelectorAll('[data-sty]')) e.querySelector(`[data-v="${STYLE}"]`)?.setAttribute('aria-checked', 'true');
for (const e of document.querySelectorAll('.stn')) e.textContent = STYLE[0].toUpperCase() + STYLE.slice(1);
for (const e of document.querySelectorAll('[data-tx]')) if (T) e.textContent = e.dataset.tx;
document.body.dataset.ready = 1;
