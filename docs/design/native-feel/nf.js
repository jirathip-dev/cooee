// #232 mockup: the native iOS feel pass. It draws this design's frames, then runs rooms' rm.js (the scheme, the
// style, the phone chrome, the personas' avatars in the three styles). Fills:
//   [data-y=home]   Home as #251 draws it (two floating buttons, search, the personas row, your personas' chats,
//                   #229's Others row), with data-sw="<row>:<px>" a row swiped by px (negative: trailing actions,
//                   positive: leading), data-full="<row>" a row mid full swipe, data-hide="<row>" a row gone,
//                   data-rcn the reconnecting line, data-wide the Mac sidebar
//   [data-y=others] #229's Others screen, the same data-* as Home
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
  tray: '<path d="M3 13l3-8h12l3 8v6H3zM3 13h5l1 2h6l1-2h5"/>',
};
export const ico = (k, c = 'ico') => `<svg class="${c}" viewBox="0 0 24 24">${I[k]}</svg>`;

// The rows: id, avatar (a persona, or a neutral initial for a chat with no persona), name, sub line, last line, age,
// unread, muted, state. Home holds only your personas' chats (#229); Others is one row at its foot.
const CHATS = [
  { id: 'coder', p: 'coder', n: 'Coder', s: 'Export for Kestrel', l: "Q3 was $4.1M. It's in export/summary.md.", a: 'now', u: 1, k: 'replied' },
  { id: 'res', p: 'res', n: 'Researcher', s: 'Kestrel filings', l: 'Allow fetch on sec.gov?', a: '2m', u: 1, k: 'asks' },
  { id: 'cond', p: 'cond', n: 'Conductor', s: 'cooee queue', l: '#251 design is up for your review', a: '20m', u: 3, k: 'replying' },
  { id: 'am', p: 'am', n: 'Account Manager', s: 'Pricing', l: 'A 3-year term at 18% off list, if they sign…', a: '1h' },
  { id: 'chief', p: 'chief', n: 'Chief', s: 'Renewals', l: "Every Monday at 09:00 I'll check the renewals.", a: '2h', m: 1 },
];
// Others (#229): chats with no persona, then workflow whose source no installed plugin claims.
const OTHERS = [
  ['Chats · no persona', [
    { id: 'bonj', o: 'B', n: 'Bonjour probe', s: '~/scratch', l: '_cooee._tcp answers on the LAN.', a: '3h', u: 1 },
    { id: 'jq', o: 'J', n: 'jq filter for fleet.toml', s: '~/src/herdsman', l: '.fleet.cooee.role | keys', a: '1d' },
    { id: 'newc', o: 'N', n: 'New chat', s: '~/scratch', l: 'Hi! What can I help with?', a: '4d' },
  ]],
  ['nightly · no plugin claims it', [
    { id: 'night', o: '◷', n: 'nightly · deps bump', s: 'source nightly', l: 'cargo update -p tokio: 1.47 → 1.48', a: '6h' },
  ]],
];
// The personas row (#251): pinned first, a hairline, then the rest.
const PERSONAS = [['chief', 'Chief'], ['coder', 'Coder', 'replied', 1], ['res', 'Researcher', 'asks', 1], '|', ['am', 'Account'], ['cond', 'Conductor', 'replying', 3]];

function av(x) {
  if (x.o) return `<span class=oav>${x.o}</span>`;
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
// The line a dropped connection shows (q1349: lists are live, no pull); a tap retries at once.
const RCN = `<a class=rcn href="#"><span class="ind sm"></span><span><b>${T ? 'reconnecting to mini' : 'Reconnecting to mini…'}</b><small>Showing 9:12 · tap to retry</small></span></a>`;
// The two buttons float over one scroll (#251): no title bar.
function top(e) {
  const wide = 'wide' in e.dataset;
  return `${wide ? '<div class=lights><i></i><i></i><i></i></div>' : ''}<div class=htop><span class=acct>${T ? '[g]' : 'G'}</span><a class=gb href="#" data-tx="[+]">+</a></div>`
    + ('rcn' in e.dataset ? RCN : '');
}
function home(e) {
  const prow = PERSONAS.map(x => x === '|' ? '<i class=psep></i>'
    : `<a class=pin2 href="#"><i class=pa data-p=${x[0]}${x[2] ? ` data-k=${x[2]}` : ''}></i>${x[3] ? `<b class=pbd>${x[3]}</b>` : ''}<span>${x[1]}</span></a>`).join('');
  const titles = OTHERS.flatMap(([, rs]) => rs.map(x => x.n)).join(', ');
  return top(e) + `<div class=scroll>`
    + `<div class=srch>${T ? '<span>$ grep <i class=cur>▌</i></span>' : `<svg class=ico viewBox="0 0 24 24"><circle cx="11" cy="11" r="6"/><path d="M20 20l-4.5-4.5"/></svg><span>Search</span>`}</div>`
    + `<div class=pins><div class=prow>${prow}</div></div>`
    + CHATS.map(x => row(x, e)).join('')
    + `<a class=hwo href="#"><span class=oav>${ico('tray')}</span><span class=hn2><b>Others</b><span>${titles}</span></span><span class=hwn>4</span></a></div>`;
}
// Others (#229): a pushed screen, the same rows and the same swipe actions.
function others(e) {
  return `<div class=nbo><a href="#">‹ Home</a><b>Others</b><i></i></div>${'rcn' in e.dataset ? RCN : ''}<div class=scroll>`
    + `<h1 class=lt2>Others</h1><p class=ods>Chats with no persona, and workflow whose source no installed plugin claims.</p>`
    + OTHERS.map(([h, rs]) => `<h3 class=sh3>${h}</h3>${rs.map(x => row(x, e)).join('')}`).join('')
    + `<p class=ofn>drover claims source drover: its sessions are in Conductor › Workflow.</p></div>`;
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

// The Workflow sheet (#229): one row per issue, grouped needs you → failed → working; the state's dot, `repo #n ·
// stage`, the state word, the title, the age.
const WF = [
  ['Needs you', [['asks', 'cooee #229', 'design 1', 'needs you', "native Home: personas' chats only", '6m']]],
  ['Failed', [['failed', 'herdsman #722', 'review 1', 'failed', 'drive: one ask per lane, not per turn', '25m']]],
  ['Working', [['replying', 'cooee #217', 'fix 1', 'working', 'personas: a history tool', '9m'],
    ['replying', 'cooee #184', 'impl', 'working', 'fleets config from the app', '40m'],
    ['replying', 'cooee #232', 'design 2', 'working', 'native feel pass', '1m']]],
];
function wf() {
  return `<p class=wfs>drover · 5 issues · <b>1 needs you</b></p>` + WF.map(([h, rs]) => `<h3 class=sh3>${h}</h3>` + rs.map(([k, n, st, w, t, a]) =>
    `<a class=wr2 data-k=${k} href="#"><i class=sd2></i><span class=t2><b>${n}</b> · <em>${st}</em></span><span class=ag2>${a} ›</span><span class=s2><em>${w}</em> · ${t}</span></a>`).join('')).join('');
}

// The system activity indicator: eight spokes, fading round.
const SPIN = `<svg viewBox="0 0 28 28">${[...Array(8)].map((_, i) => `<line x1="14" y1="3" x2="14" y2="8.5" opacity="${(1 - i * 0.11).toFixed(2)}" transform="rotate(${-i * 45} 14 14)"/>`).join('')}</svg>`;

const X = { home, others, nkb, wf };
for (const e of document.querySelectorAll("[data-y]")) e.innerHTML = X[e.dataset.y](e);
for (const e of document.querySelectorAll('.ind')) e.innerHTML = SPIN;
for (const e of document.querySelectorAll('[data-i]')) e.insertAdjacentHTML('afterbegin', ico(e.dataset.i));
await import('../rooms/rm.js');
document.body.dataset.ready = 2;
