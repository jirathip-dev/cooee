const W = '../../../web/', P = '../../../packs/themes/', SPR = '../../../apple/Sources/CooeeUI/Sprites/';
const q = new URLSearchParams(location.search);
const T = q.get('theme') || 'farm', F = q.get('f');
const STYLE = q.get('style') || document.body.dataset.style || 'modern';
const MODE = q.get('mode') || document.body.dataset.mode || 'dark';
const theme = await fetch(`${P}${T}/theme.json`).then(r => r.json());
const scheme = await fetch(`${W}schemes/${theme.scheme[MODE]}.json`).then(r => r.json());
const r = document.documentElement;
for (const [k, v] of Object.entries(scheme.tokens)) r.style.setProperty(k, v);
r.style.colorScheme = scheme['color-scheme'];
Object.assign(r.dataset, { theme: T, style: STYLE === 'terminal' ? 'terminal' : 'modern', mode: MODE });
if (STYLE === 'scene') r.dataset.scene = '';
if (F) { r.dataset.one = ''; document.querySelector(`[data-f="${F}"]`)?.closest('.fr').classList.add('one'); }

const G = {
  star: '<svg class=f viewBox="0 0 24 24"><path d="M12 2.5l2.9 6.1 6.6.8-4.9 4.6 1.3 6.6L12 17.3 6.1 20.6l1.3-6.6L2.5 9.4l6.6-.8z"/></svg>',
  code: '<svg viewBox="0 0 24 24"><path d="M8 7l-5 5 5 5M16 7l5 5-5 5M14 4l-4 16"/></svg>',
  find: '<svg viewBox="0 0 24 24"><circle cx="10.5" cy="10.5" r="6.5"/><path d="M15.5 15.5L21 21"/></svg>',
  case: '<svg viewBox="0 0 24 24"><rect x="3" y="7" width="18" height="13" rx="2.5"/><path d="M9 7V5h6v2M3 12.5h18"/></svg>',
  node: '<svg viewBox="0 0 24 24"><circle cx="6" cy="6" r="2.6"/><circle cx="18" cy="6" r="2.6"/><circle cx="12" cy="18" r="2.6"/><path d="M8.6 6h6.8M7.3 8.3l3.4 7.4M16.7 8.3l-3.4 7.4" stroke-dasharray="1.6 2"/></svg>',
};
export const PER = {
  chief: { name: 'Chief', t: '--l', g: 'star', coat: 'ch', agent: 'Claude Code', model: 'opus 5.5', machine: 'mini' },
  coder: { name: 'Coder', t: '--c', g: 'code', coat: 'gry', agent: 'Codex', model: 'gpt-5.5', machine: 'studio' },
  res: { name: 'Researcher', t: '--p', g: 'find', coat: 'blk', agent: 'Claude Code', model: 'sonnet 5.5', machine: 'air' },
  am: { name: 'Account Manager', t: '--link', g: 'case', coat: 'bay', agent: 'Claude Code', model: 'sonnet 5.5', machine: 'mini' },
  cond: { name: 'Conductor', t: '--g', g: 'node', coat: 'wht', agent: 'Claude Code', model: 'opus 5.5', machine: 'mini' },
};
const KS = {
  replying: ['replying', '~o=', 'edit'], replied: ['replied', '\\o/', 'done'], passed: ['passed', '-.-', 'idle'],
  next: ['up next', '-.-', 'idle'], waiting: ['waiting', '-.-', 'idle'], asks: ['asks you', '(!)', 'needs'],
};
const FARM = { replying: 'on the trail', replied: 'in the stable', passed: 'grazing', next: 'grazing', waiting: 'grazing', asks: 'at the gate' };

function avatar(e) {
  const p = PER[e.dataset.p], k = e.dataset.k, pose = e.dataset.pose || (k ? KS[k][2] : 'done');
  e.style.setProperty('--t', `var(${p.t})`);
  if (e.dataset.s) e.style.setProperty('--s', e.dataset.s + 'px');
  e.setAttribute('aria-hidden', 'true');
  const dot = k && STYLE !== 'scene' ? `<b class=sdot data-k=${k}></b>` : '';
  e.innerHTML = (STYLE === 'scene' ? `<img src="${SPR}horse-${p.coat}-${pose}.png" alt="">`
    : STYLE === 'terminal' ? (k ? KS[k][1] : e.dataset.fig || '\\o/') : G[p.g]) + dot;
}

function strip(e) {
  return e.dataset.m.split(' ').map((m, i) => {
    const [id, k] = m.split(':'), p = PER[id];
    const w = STYLE === 'scene' ? FARM[k] : KS[k][0];
    return `<a class="mm${e.dataset.sel === id ? ' sel' : ''}" data-k=${k} href="#" style="--t:var(${p.t})"><span class=mord>${i + 1}</span><i class=pa data-p=${id} data-k=${k}></i><b>${p.name}</b><span class=mw>${w}</span></a>`;
  }).join('');
}

const ROWS = [
  { room: 'kestrel', stk: 'chief coder res am', n: 'Kestrel launch', s: '4 members', l: 'Account Manager: Pricing: a 3-year term at…', a: 'now', u: 1 },
  { p: 'coder', n: 'Coder', s: 'Export for Kestrel', l: "Q3 was $4.1M. It's in export/summary.md.", a: '1m', u: 0 },
  { sub: 'lx1', p: 'coder', p2: 'res', n: 'Coder ⇄ Researcher', a: '1m' },
  { sub: 'lx2', p: 'res', p2: 'am', n: 'Researcher ⇄ Account Manager', a: '2m' },
  { room: 'deck', stk: 'chief am', n: 'Board deck', s: '2 members', l: 'Chief: Slides 4–9 are in deck/q3.key.', a: '2d' },
  { p: 'res', n: 'Researcher', s: 'Kestrel filings', l: 'Q3 revenue $4.1M, up 12% on Q2.', a: '12m' },
  { p: 'chief', n: 'Chief', s: 'Renewals', l: "Every Monday at 09:00 I'll check the rene…", a: '2h' },
];
function side(e) {
  const sel = e.dataset.sel;
  const row = x => x.sub
    ? `<a class="hr sub${sel === x.sub ? ' sel' : ''}" href="#"><span class=av2><span class="stk"><i class=pa data-p=${x.p}></i><i class=pa data-p=${x.p2}></i></span></span><span class=hn2><span class=lk2>⇄</span> ${x.n}</span><span class=age>${x.a}</span></a>`
    : `<a class="hr${sel === (x.room || x.p) ? ' sel' : ''}" href="#"><span class=av2>${x.stk
      ? `<span class="stk v">${x.stk.split(' ').map(p => `<i class=pa data-p=${p}></i>`).join('')}</span>` : `<i class=pa data-p=${x.p}></i>`}</span>
      <span class=hn2><b>${x.n}</b><span>${x.s}</span></span><span class="age${x.u ? ' u' : ''}">${x.a}</span><span class=ln>${x.l}</span>${x.u ? `<span class=bd2>${x.u}</span>` : '<span></span>'}</a>`;
  return `${'ph' in e.dataset ? '' : '<div class=lights><i></i><i></i><i></i></div>'}<div class=sdtop><span class=gb>G</span><b>cooee</b><span class=gb>+</span></div><h2 class=lt2>Chats</h2>${ROWS.map(row).join('')}`;
}

const SB = `<span>9:41</span><span class=si><svg viewBox="0 0 18 12"><rect y="8" width="3" height="4" rx=".7"/><rect x="5" y="5.5" width="3" height="6.5" rx=".7"/><rect x="10" y="3" width="3" height="9" rx=".7"/><rect x="15" width="3" height="12" rx=".7"/></svg><svg viewBox="0 0 16 12"><path d="M8 2.4c2.4 0 4.5.9 6.1 2.4L15.3 3.6A10.3 10.3 0 0 0 8 .6 10.3 10.3 0 0 0 .7 3.6l1.2 1.2A8.6 8.6 0 0 1 8 2.4zm0 3.5c1.4 0 2.6.5 3.6 1.4l1.2-1.2A6.8 6.8 0 0 0 8 4.1 6.8 6.8 0 0 0 3.2 6.1l1.2 1.2c1-.9 2.2-1.4 3.6-1.4zM8 11.5l2.4-2.4a3.4 3.4 0 0 0-4.8 0z"/></svg><svg viewBox="0 0 27 12" style="height:13px"><rect x=".5" y=".5" width="23" height="11" rx="3.5" fill="none" stroke="currentColor" opacity=".4"/><rect x="2" y="2" width="18" height="8" rx="2"/><path d="M25 4v4c.8-.3 1.4-1 1.4-2S25.8 4.3 25 4z" opacity=".4"/></svg></span>`;
for (const p of document.querySelectorAll('.ph')) p.insertAdjacentHTML('beforeend', `<div class=stb>${SB}</div><i class=isl></i><i class=hind></i>`);
if (STYLE === 'scene') for (const e of document.querySelectorAll('[data-x=strip]')) e.classList.add('scn2');
const X = { strip, side };
for (const e of document.querySelectorAll('[data-x]')) e.innerHTML = X[e.dataset.x](e);
for (const e of document.querySelectorAll('[data-t]')) e.style.setProperty('--t', `var(${PER[e.dataset.t].t})`);
for (const e of document.querySelectorAll('.pa[data-p]')) avatar(e);
if (STYLE === 'terminal') for (const e of document.querySelectorAll('.gb[data-tx]')) e.textContent = e.dataset.tx;
document.body.dataset.ready = 1;
