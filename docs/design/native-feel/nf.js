// #232 mockup: the native iOS feel pass. It draws this design's frames, then runs rooms' rm.js (the scheme, the
// style, the phone chrome, the personas' avatars in the three styles). Fills:
//   [data-y=home]   Home as CooeeUI's HomeView draws it (top bar, pinned strip, Chats, Others), with
//                   data-sw="<row>:<px>" a row swiped by px (negative: trailing actions, positive: leading),
//                   data-full="<row>" a row mid full swipe, data-pull="<px>" the list pulled, data-wide the Mac sidebar
//   [data-y=nkb]     the iOS keyboard (predictive bar, three letter rows, the bottom row)
//   [data-y=wf]     the Workflow sheet's issue rows (#229)
const q = new URLSearchParams(location.search);
const STYLE = q.get('style') || 'modern';
const T = STYLE === 'terminal';

const I = {
  pin: '<path d="M9 3h6l-1 6 4 4H6l4-4zM12 13v8"/>',
  mute: '<path d="M6 9h3l5-4v14l-5-4H6zM17 9l4 6M21 9l-4 6"/>',
  arch: '<rect x="3" y="4" width="18" height="5" rx="1"/><path d="M5 9v10h14V9M10 13h4"/>',
  read: '<circle cx="12" cy="12" r="7"/>',
  copy: '<rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3"/>',
  per: '<circle cx="12" cy="8" r="4"/><path d="M4 21a8 8 0 0 1 16 0"/>',
  open: '<path d="M7 17L17 7M9 7h8v8"/>',
};
export const ico = (k, c = 'ico') => `<svg class="${c}" viewBox="0 0 24 24">${I[k]}</svg>`;

// The rows: id, avatar (a persona, or a room's stack), name, sub line, last line, age, unread, muted, state.
const CHATS = [
  { id: 'coder', p: 'coder', n: 'Coder', s: 'Export for Kestrel', l: "Q3 was $4.1M. It's in export/summary.md.", a: 'now', u: 1, k: 'replied' },
  { id: 'res', p: 'res', n: 'Researcher', s: 'Kestrel filings', l: 'Allow fetch on sec.gov?', a: '2m', k: 'asks' },
  { id: 'am', p: 'am', n: 'Account Manager', s: 'Pricing', l: 'A 3-year term at 18% off list, if they sign…', a: '12m' },
  { id: 'chief', p: 'chief', n: 'Chief', s: 'Renewals', l: "Every Monday at 09:00 I'll check the rene…", a: '2h', m: 1 },
];
const OTHERS = [
  { id: 'room', stk: 'chief coder res am', n: 'Kestrel launch', s: '4 members', l: 'Account Manager: Pricing: a 3-year term at…', a: '4m', u: 2 },
  { id: 'lx', p: 'coder', p2: 'res', n: 'Coder ⇄ Researcher', s: 'linked', l: 'Researcher: Page four confirms 4.1 million.', a: '6m' },
  { id: 'lane', p: 'cond', n: 'herdsman #641', s: 'fix 1', l: 'impl lane: cargo test passed, pushing.', a: '9m', k: 'replying' },
];
const PINS = [['chief', 'Chief'], ['coder', 'Coder'], ['res', 'Researcher']];

function av(x) {
  if (x.stk) return `<span class="stk v">${x.stk.split(' ').map(p => `<i class=pa data-p=${p}></i>`).join('')}</span>`;
  if (x.p2) return `<span class=stk><i class=pa data-p=${x.p}></i><i class=pa data-p=${x.p2}></i></span>`;
  return `<i class=pa data-p=${x.p}${x.k ? ` data-k=${x.k}` : ''}></i>`;
}
function row(x, e) {
  if ((e.dataset.hide || '').split(' ').includes(x.id)) return '';
  const [sid, px] = (e.dataset.sw || ':').split(':'), full = e.dataset.full === x.id, sel = e.dataset.sel === x.id;
  const lifted = e.dataset.lift === x.id;
  const tail = `${x.m ? `<span class=mut2>${ico('mute')}</span>` : ''}${x.u ? `<span class=bd2>${x.u}</span>` : ''}`;
  const r = `<a class="hr${sel ? ' sel' : ''}${lifted ? ' lift' : ''}" href="#" style="--x:${full ? -300 : sid === x.id ? px : 0}px"><span class=av2>${av(x)}</span>`
    + `<span class=hn2><b>${x.n}</b><span>${x.s}</span></span><span class="age${x.u ? ' u' : ''}">${x.a}</span>`
    + `<span class="ln${x.k === 'asks' ? ' ak2' : ''}">${x.l}</span><span class=rtl>${tail}</span></a>`;
  if (sid !== x.id && !full) return `<div class=sw>${r}</div>`;
  const n = Number(px) || 0;
  const acts = full ? `<div class="nsw tr full"><span class="sa ar">${ico('arch')}<b>Archive</b></span></div>`
    : n < 0 ? `<div class="nsw tr"><span class="sa mu">${ico('mute')}<b>${x.m ? 'Unmute' : 'Mute'}</b></span><span class="sa ar">${ico('arch')}<b>Archive</b></span></div>`
      : `<div class="nsw ld"><span class="sa pn">${ico('pin')}<b>Pin</b></span></div>`;
  return `<div class="sw on${full ? ' fs' : ''}">${acts}${r}</div>`;
}
function home(e) {
  const pull = Number(e.dataset.pull || 0), wide = 'wide' in e.dataset;
  const top = wide ? '<div class=lights><i></i><i></i><i></i></div>' : '';
  return `${top}<div class=htop><span class=acct>${T ? '[g]' : 'G'}</span><b>${T ? '~/cooee' : 'cooee'}</b><a class=gb href="#" data-tx="[+]">+</a></div>`
    + `<div class=scroll style="--pull:${pull}px">${e.dataset.top || ''}`
    + `<div class=pins>${T ? '<span class=sh2># pinned</span>' : ''}<div class=prow>${PINS.map(([p, n]) => `<a class=pin2 href="#"><i class=pa data-p=${p}></i><span>${n}</span></a>`).join('')}</div></div>`
    + `<h3 class=sh2>${T ? '# chats' : 'Chats'}</h3>${CHATS.map(x => row(x, e)).join('')}`
    + `<h3 class=sh2>${T ? '# others' : 'Others'}</h3>${OTHERS.map(x => row(x, e)).join('')}</div>`;
}

// The iOS keyboard, at 390: the predictive bar, QWERTY, the bottom row; its colours are the system's on the scheme.
function nkb(e) {
  const rows = ['qwertyuiop', 'asdfghjkl', 'zxcvbnm'];
  const k = c => `<i>${c}</i>`;
  const pred = (e.dataset.pred || '"filing"|filing|filings').split('|').map(w => `<span>${w}</span>`).join('');
  return `<div class=pred>${pred}</div><div class=nkr>${[...rows[0]].map(k).join('')}</div><div class="nkr nk2">${[...rows[1]].map(k).join('')}</div>`
    + `<div class=nkr><i class=kfn>⇧</i>${[...rows[2]].map(k).join('')}<i class=kfn>⌫</i></div>`
    + `<div class="nkr kb4"><i class=kfn>123</i><i class=kfn>☺</i><i class=spc>space</i><i class="kfn kret">return</i></div><div class=kbf><span>🌐</span><span>🎙</span></div>`;
}

// The Workflow sheet's rows (#229): the dot in its state, number, title, stage.
const WF = [
  ['asks', '#241', 'Fleet pause from the phone', 'design · asks you'], ['replying', '#238', 'Routines engine', 'impl · live'],
  ['asks', '#236', 'Persona panel', 'review 1 · FAIL'], ['replying', '#232', 'Native feel pass', 'design · live'],
  ['replied', '#228', 'History tool', 'merged · 2h'], ['replied', '#222', 'Many orchestrators', 'merged · 1d'],
  ['passed', '#219', 'Sign-ins on iPad', 'queued'],
];
function wf() {
  return WF.map(([k, n, t, s]) => `<a class=wr2 data-k=${k} href="#"><i class=sd2></i><span class=n2>${n}</span><span class=t2>${t}</span><span class=s2>${s}</span></a>`).join('');
}

// The system activity indicator: eight spokes, fading round.
const SPIN = `<svg viewBox="0 0 28 28">${[...Array(8)].map((_, i) => `<line x1="14" y1="3" x2="14" y2="8.5" opacity="${(1 - i * 0.11).toFixed(2)}" transform="rotate(${-i * 45} 14 14)"/>`).join('')}</svg>`;
for (const e of document.querySelectorAll('.ind')) e.innerHTML = SPIN;

const X = { home, nkb, wf };
for (const e of document.querySelectorAll("[data-y]")) e.innerHTML = X[e.dataset.y](e);
for (const e of document.querySelectorAll('[data-i]')) e.insertAdjacentHTML('afterbegin', ico(e.dataset.i));
await import('../rooms/rm.js');
document.body.dataset.ready = 2;
