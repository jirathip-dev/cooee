import { ranch } from './herd.js';

const W = '../../../web/', P = '../../../packs/themes/';
const q = new URLSearchParams(location.search);
const gal = document.querySelector('.gal'), PAGE = gal.dataset.page;
const STYLE = PAGE === 'modern' ? 'modern' : PAGE, MODE = q.get('mode') || (PAGE === 'scene' ? 'light' : 'dark'), F = q.get('f');
const theme = await fetch(`${P}farm/theme.json`).then(r => r.json());
const scheme = await fetch(`${W}schemes/${theme.scheme[MODE]}.json`).then(r => r.json());
const r = document.documentElement;
for (const [k, v] of Object.entries(scheme.tokens)) r.style.setProperty(k, v);
r.style.colorScheme = scheme['color-scheme'];
Object.assign(r.dataset, { theme: 'farm', style: STYLE, scene: STYLE === 'scene' ? 'on' : 'off', mode: MODE });
const esc = s => String(s).replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);

const SB = `<span>9:41</span><span class=si><svg viewBox="0 0 18 12"><rect y="8" width="3" height="4" rx=".7"/><rect x="5" y="5.5" width="3" height="6.5" rx=".7"/><rect x="10" y="3" width="3" height="9" rx=".7"/><rect x="15" width="3" height="12" rx=".7"/></svg><svg viewBox="0 0 16 12"><path d="M8 2.4c2.4 0 4.5.9 6.1 2.4L15.3 3.6A10.3 10.3 0 0 0 8 .6 10.3 10.3 0 0 0 .7 3.6l1.2 1.2A8.6 8.6 0 0 1 8 2.4zm0 3.5c1.4 0 2.6.5 3.6 1.4l1.2-1.2A6.8 6.8 0 0 0 8 4.1 6.8 6.8 0 0 0 3.2 6.1l1.2 1.2c1-.9 2.2-1.4 3.6-1.4zM8 11.5l2.4-2.4a3.4 3.4 0 0 0-4.8 0z"/></svg><svg viewBox="0 0 27 12" style="height:13px"><rect x=".5" y=".5" width="23" height="11" rx="3.5" fill="none" stroke="currentColor" opacity=".4"/><rect x="2" y="2" width="18" height="8" rx="2"/><path d="M25 4v4c.8-.3 1.4-1 1.4-2S25.8 4.3 25 4z" opacity=".4"/></svg></span>`;
const ICON = { new: '<path d="M12 5v14M5 12h14"/>', find: '<circle cx="10.5" cy="10.5" r="6"/><path d="M15 15l5 5"/>',
  tray: '<path d="M3 13h5l1.5 3h5l1.5-3h5"/><path d="M5.5 5h13L21 13v5a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-5z"/>' };
const ic = k => `<svg class=hic viewBox="0 0 24 24" aria-hidden=true>${ICON[k]}</svg>`;

const PERS = {
  chief: ['Chief', 'CH', '--l', 'ch', 'done', 1], coder: ['Coder', 'CO', '--c', 'gry', 'working', 1], res: ['Researcher', 'RE', '--p', 'blk', 'needs', 1],
  acct: ['Account Manager', 'AM', '--link', 'bay', 'idle'], cond: ['Conductor', 'CD', '--g', 'wht', 'working'],
  writer: ['Writer', 'WR', '--a', 'bay', 'idle'], design: ['Designer', 'DE', '--p', 'gry', 'idle'] };
const ORDER = ['chief', 'coder', 'res', 'acct', 'cond', 'writer', 'design'];
const FIG = { working: ['~o=', '=o~'], needs: ['(!)', '[!]'], done: ['\\o/', '|o|'], idle: ['-.-', '-_-'] };
const POSE = { working: 'edit', needs: 'needs', done: 'done', idle: 'idle' };
const UNREAD = { coder: 2, res: 1, cond: 3 };
const word = k => theme.words[k];
const av = id => `<i class="av" style="--av:var(${PERS[id][2]})" aria-hidden=true>${PERS[id][1]}</i>`;
const fig = (k, cls = '', badge = 0) => `<span class="tmf ${cls}" data-k=${k} aria-label="${esc(word(k))}"><b>${esc(FIG[k][0])}</b><b>${esc(FIG[k][1])}</b>${badge ? `<i class=bd>${badge}</i>` : ''}</span>`;

const CHATS = [['coder', 'Flaky e2e on the look sheet', '$ cargo test -p cooee-core', 'now', 2, 'working'],
  ['res', 'Kestrel filings', 'Allow fetch sec.gov/cgi-bin/browse-edgar?', '12m', 1, 'needs'],
  ['cond', 'cooee queue', '#251 design is up for your review', '20m', 3, 'working'],
  ['chief', 'Renewals', "Every Monday at 09:00 I'll check the renewals.", '1h', 0, 'done'],
  ['acct', 'Birch & Co pricing', 'Due this month: Acme 14 Nov, Birch 21 Nov', '3h', 0, 'idle'],
  ['chief', 'Q3 board deck', 'Slides 4–9 are in deck/q3.key.', '2d', 0, 'done'],
  ['writer', 'Launch post', 'Draft 2 is in posts/launch.md.', '3d', 0, 'idle'],
  ['coder', 'Bonjour discovery spike', '_cooee._tcp answers on the LAN.', '4d', 0, 'done']];
const LOOSE = ['Bonjour probe', 'jq filter for fleet.toml', 'New chat'];
const mono = () => STYLE === 'terminal', sceneOn = () => STYLE === 'scene';
const mark = s => s === 'working' ? (mono() ? '' : '<i class=h2k aria-label=working>•••</i>') : s === 'needs' ? '<i class=h2n aria-label="waits on you">!</i>' : '';
const lead = (id, st) => mono() ? fig(st) : sceneOn() ? `<img class=hav src="../../../apple/Sources/CooeeUI/Sprites/horse-${PERS[id][3]}-${POSE[st]}.png" alt="">` : av(id);
const chatRow = ([id, title, last, age, ud, st], sel) => `<a class="pr h2r${sel ? ' sel' : ''}" href="#">${lead(id, st)}<span class=an><b>${esc(PERS[id][0])}</b> <small>${esc(title)}</small></span><span class=ta>${age}</span>
  <span class=tp>${mark(st)}<span>${esc(last)}</span>${ud ? `<i class=bd>${ud}</i>` : ''}</span></a>`;
const othersRow = () => `<a class="pr h2r h2o" href="#"><i class="av oav" aria-hidden=true>${mono() ? '[~]' : ic('tray')}</i><span class=an><b>Others</b></span><span class=ta><i class=h2c>${LOOSE.length}</i></span>
  <span class=tp><span>${LOOSE.join(', ')}</span></span></a>`;

const search = () => mono() ? `<p class=tmp><b>$</b>grep <i class=tmc></i></p>` : `<p class=h2q>${ic('find')}<span>Search</span></p>`;
const row = ids => {
  if (mono()) return `<div class=tmr>${ids.map(id => `<a class=pn href="#">${fig(PERS[id][4], '', UNREAD[id])}<span>${esc(PERS[id][0].split(' ')[0].toLowerCase())}</span><small${PERS[id][4] === 'needs' ? ' class=r' : ''}>${esc(word(PERS[id][4]))}</small></a>`).join('')}</div>`;
  const pins = ids.filter(id => PERS[id][5]), rest = ids.filter(id => !PERS[id][5]);
  const pn = id => `<a class="pn${PERS[id][4] === 'working' ? ' wk' : PERS[id][4] === 'needs' ? ' nd' : ''}" href="#">${av(id)}<span>${esc(PERS[id][0].split(' ')[0])}</span>${UNREAD[id] ? `<i class=bd>${UNREAD[id]}</i>` : ''}</a>`;
  return `<div class=h2p>${pins.map(pn).join('')}${pins.length && rest.length ? '<i class=sep></i>' : ''}${rest.map(pn).join('')}</div>`;
};
const HERD = { coder: { x: 0, y: 26, w: 74, k: 'wk', run: 70 }, cond: { x: 90, y: 24, w: 72, k: 'wk', run: 60 }, res: { x: 248, y: 30, w: 78, k: 'nd' },
  chief: { x: 168, y: 80, w: 42, k: 'dn' }, acct: { x: 40, y: 100, w: 44, k: 'id' }, writer: { x: 96, y: 104, w: 38, k: 'id', left: true }, design: { x: 222, y: 102, w: 38, k: 'id' } };
const herdOf = ids => ids.map(id => ({ ...HERD[id], coat: PERS[id][3], pose: POSE[PERS[id][4]], name: PERS[id][0].split(' ')[0], word: HERD[id].w > 60 ? word(PERS[id][4]) : (PERS[id][4] === 'done' ? word('done') : ''), badge: UNREAD[id] }));
const MD = 340;
const FULL = { coder: { x: 14, y: 134, w: 118 }, cond: { x: 200, y: 122, w: 112, left: true }, res: { x: 286, y: MD + 26, w: 84 },
  chief: { x: 166, y: MD + 80, w: 46 }, acct: { x: 16, y: 248, w: 72 }, writer: { x: 110, y: 276, w: 64, left: true }, design: { x: 190, y: 240, w: 68 } };
const herdFull = (ids, t) => herdOf(ids).map((h, i) => {
  const f = FULL[ids[i]], m = (a, b) => Math.round(a + (b - a) * t);
  return { ...h, x: m(h.x, f.x), y: m(h.y, f.y), w: m(h.w, f.w), left: t > .5 ? f.left : h.left, run: h.run && (t < 1 ? h.run : 90), word: t > .5 ? word(PERS[ids[i]][4]) : h.word };
});

const reconnecting = () => `<a class=h2x href="#" aria-label="Reconnecting, tap to retry">${mono() ? 'reconnecting…' : '<i class=spn></i>Reconnecting…'}<small>${mono() ? '[tap to retry]' : 'Tap to retry'}</small></a>`;
const home = ({ y = 0, ids = ORDER, sel, night = false, weather = 'calm', wide = false, x = false, pull = 0 } = {}) => {
  const strip = () => pull ? ranch({ night, weather, horses: herdFull(ids, pull), h: Math.round(330 + (844 - 330) * pull), meadow: Math.round(MD * pull), inner: pull < 1 ? search() : '' })
    : ranch({ night, weather, horses: herdOf(ids), h: wide ? 290 : 330, inner: search() });
  const top = sceneOn() ? (ids.length ? strip() : `<div style="height:calc(var(--sat) + 56px)"></div>${search()}`) : search() + (ids.length ? row(ids) : '');
  const t = y ? `<i class="h2t${night ? ' night' : ''}${weather === 'held' ? ' held' : ''}"></i>` : '';
  return `<div class="scr h2s"><div class=in style="--y:${y}px">${top}${CHATS.filter(c => ids.includes(c[0])).map((c, i) => chatRow(c, sel === i)).join('')}${othersRow()}</div></div>${t}
  <a class="h2b l" href="#" aria-label=You>${mono() ? '[g]' : '<i class=me>G</i>'}</a><a class="h2b r" href="#" aria-label="New chat">${mono() ? '[+]' : ic('new')}</a>${x ? reconnecting() : ''}${mono() ? status(x) : ''}`;
};
const status = x => `<p class=tms><span class="s0${x ? ' x' : ''}">${x ? 'mini ✗' : 'mini'}</span><span>${x ? 'load –' : 'load 0.42 calm'}</span><span class=sp></span><span><b>2</b> working</span><span class=wt><b>1</b> waiting</span><span>09:41</span></p>`;

const chat = () => `<div class=cvw><ol class=tc aria-label=messages><li class="blk me"><div class=md>The look sheet e2e flakes on CI. Find out why.</div><span class=ct>09:20</span></li>
  <li class="blk run"><span class=k>✓</span><span class=cmd>rg -n "waitFor" web/e2e/look.spec.ts</span></li>
  <li class="blk ai"><b class=k>Coder</b><div class=md><p>It waits for the sheet's first frame, not its fonts. Running the suite with a font-ready wait.</p></div></li>
  <li class="blk run"><span class=k>▸</span><span class=cmd>cargo test -p cooee-core</span></li></ol>
  <form class=tw><button type=button class=att aria-label=attach>+</button><textarea rows=1 aria-label=message placeholder="Message Coder"></textarea><button type=button aria-label=send>↑</button></form></div>
  <header class="nb sc ch2"><span></span><span class=ph2>${lead('coder', 'working')}<span><b>Coder</b><small class=ds>Codex · gpt-5.5</small></span></span><span></span></header>`;

const phone = (h, cls = '') => `<section class="ph ${cls}">${h}<div class=stb>${SB}</div><i class=isl></i><i class=hind></i></section>`;
const mac = (side, main) => `<section class="wd mac h2m"><aside class=side>${side}<span class=tl><i></i><i></i><i></i></span></aside><main class=dtl>${main}</main></section>`;

const motion = () => {
  const s = (lbl, hs, extra = '') => ranch({ h: 150, horses: hs, inner: `<span class=lbl>${lbl}</span>${extra}` });
  return `<div class="scr"><div class=mo>
    ${s(`${word('working')} · walks, turns at each end`, [{ coat: 'gry', pose: 'edit', x: 20, y: 24, w: 72, ghost: 1 }, { coat: 'gry', pose: 'edit', x: 110, y: 24, w: 72, ghost: 1 }, { coat: 'gry', pose: 'edit', x: 200, y: 24, w: 72, k: 'wk', name: 'Coder' }, { coat: 'gry', pose: 'edit', x: 300, y: 24, w: 72, ghost: 1, left: 1 }])}
    ${s(`${word('needs')} · paws at the gate`, [{ coat: 'blk', pose: 'idle', x: 160, y: 24, w: 70, ghost: 1 }, { coat: 'blk', pose: 'needs', x: 246, y: 26, w: 74, k: 'nd', name: 'Researcher', word: word('needs') }])}
    ${s(`${word('done')} · still, at the barn door`, [{ coat: 'ch', pose: 'done', x: 176, y: 52, w: 42, name: 'Chief' }])}
    ${s(`${word('idle')} · dips to the grass`, [{ coat: 'bay', pose: 'idle', x: 40, y: 26, w: 60, ghost: 1 }, { coat: 'bay', pose: 'idle', x: 120, y: 26, w: 60, k: 'id', name: 'Account' }])}
  </div></div>`;
};

const FRAMES = {
  modern: [['top', 'At the top · You and + float over the list; search, then the personas row (pinned first)', () => phone(home())],
    ['scrolled', 'Scrolled · no search, no row; the chats run under the glassy top', () => phone(home({ y: 206 }))],
    ['empty', 'No personas · search, then Others; no row (as #249)', () => phone(home({ ids: [] }))],
    ['wide-top', 'Mac · Home as the sidebar, Coder open', () => mac(home({ sel: 0, wide: true }), chat())],
    ['wide-scrolled', 'Mac · the sidebar scrolled', () => mac(home({ sel: 0, y: 250, wide: true }), chat())],
    ['reconnect', 'The link to the Mac dropped · Reconnecting…, tap to retry; no pull to refresh', () => phone(home({ x: true }))]],
  terminal: [['term-top', 'Terminal at the top · $ grep with its cursor, the figures, the status line', () => phone(home())],
    ['term-scrolled', 'Terminal scrolled · [g] and [+] on a solid top, a dashed rule', () => phone(home({ y: 236 }))],
    ['term-empty', 'Terminal, no personas', () => phone(home({ ids: [] }))],
    ['wide-term', 'Mac · Terminal', () => mac(home({ sel: 0, wide: true }), chat())],
    ['term-reconnect', 'Terminal, the link dropped · reconnecting… [tap to retry], mini ✗ on the status line', () => phone(home({ x: true }))]],
  scene: [['scene-top', 'Scene, day, calm · each persona acts its state on the ranch', () => phone(home())],
    ['scene-busy', 'Busy · the Mac is loaded: gusts, overcast, haze', () => phone(home({ weather: 'busy' }))],
    ['scene-held', 'Held · the Mac refuses new work: storm and rain', () => phone(home({ weather: 'held' }), 'dk')],
    ['scene-night', 'Night (19:00–07:00) · stars and moon, lit barn window', () => phone(home({ night: true }), 'dk')],
    ['scene-scrolled', 'Scene scrolled · the sky and a fence rail over the rows', () => phone(home({ y: 330 }))],
    ['scene-pull', 'Mid-pull · the strip stretches: sky grows, the pasture opens, the horses walk to their bands', () => phone(home({ pull: .45 }))],
    ['scene-herd', 'The full Herd · trail, gate, barn, grazing; tap a horse for its chat; scroll up snaps back', () => phone(home({ pull: 1 }))],
    ['scene-herd-night', 'The full Herd at night', () => phone(home({ pull: 1, night: true }), 'dk')],
    ['scene-reconnect', 'Scene, the link dropped · the tag on the sky', () => phone(home({ x: true }))],
    ['scene-motion', 'The motion by state (ghosts: where the loop goes)', () => phone(motion())],
    ['wide-scene', 'Mac · Scene', () => mac(home({ sel: 0, wide: true }), chat())]] };

gal.innerHTML = FRAMES[PAGE].map(([f, cap, draw]) => `<div class="fr${f.startsWith('wide-') ? ' w' : ''}" data-f="${f}"><p class=cap>${esc(cap)}</p>${draw()}</div>`).join('');
if (F) { r.dataset.one = ''; gal.querySelector(`[data-f="${F}"]`)?.classList.add('one'); }
document.body.dataset.ready = 1;
