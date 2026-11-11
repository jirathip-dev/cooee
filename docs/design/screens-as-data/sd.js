// #125 mockup: plugin screens as data. One renderer draws any fixture (fixtures/*.json) on the live design system,
// #116's native shell (../native-shell/ns.css). It is the web client's half of the spec in README.md, in small: one
// function per block. The Swift and herdsman-board clients would map the same blocks to their own views.
// As ns.js: `?theme=farm&mode=dark|light`, `?f=<frame>` shows one frame alone (the shots), else a gallery. Fills:
//   [data-screen=<fixture>]                  a phone screen: the plugin's screen in the shell (data-on: the tab)
//   [data-sheet=<fixture>]                   a sheet over the screen
//   [data-live=<stream fixture>]             apply a stream's ops to the screen first (the live frame)
//   [data-confirm=<block id>]                that block's confirm, as the client's own action sheet
//   [data-cat]                               blocks.html's catalogue: each block's JSON beside its drawing
//   [data-chat=<fixture>]                    the chat's own messages and cards ({me, ai, cards}) in place of CHAT's
//   [data-scroll=<px>]                       the sheet scrolled down by that much
// #184 adds `?style=modern|terminal|scene` (#124's three styles, q947): Scene draws a row's figure with `coat` as its
// horse acting the state (apple/Sources/CooeeUI/Sprites).
// #213 adds the `diagram` block (../workflow-diagram/dg.js) and, on a host: [data-back] the pushed screen's back label,
// [data-focus=<field name>] the field a tap opened (`tap.focus`),
// [data-select=<state> data-then=<screen>] a state selected on the map, that screen's blocks beside it (1440), [data-menu=<label>] the sheet's ⋯ menu open, that row marked.
import { diagram } from '../workflow-diagram/dg.js';
const W = '../../../web/', P = '../../../packs/themes/', SPR = '../../../apple/Sources/CooeeUI/Sprites/';
const q = new URLSearchParams(location.search);
const T = q.get('theme') || 'farm', MODE = q.get('mode') || 'dark', F = q.get('f'), STYLE = q.get('style') || 'modern';
const theme = await fetch(`${P}${T}/theme.json`).then(r => r.json());
const scheme = await fetch(`${W}schemes/${theme.scheme[MODE]}.json`).then(r => r.json());
const r = document.documentElement;
for (const [k, v] of Object.entries(scheme.tokens)) r.style.setProperty(k, v);
r.style.colorScheme = scheme['color-scheme'];
Object.assign(r.dataset, { theme: T, style: STYLE === 'terminal' ? 'terminal' : 'modern', scene: STYLE === 'scene' ? 'on' : 'off', mode: MODE });
const defs = async u => (await fetch(u).then(r => r.text())).replace(/^[\s\S]*?<svg[^>]*>|<\/svg>\s*$/g, '');
document.body.insertAdjacentHTML('afterbegin', `<svg width=0 height=0 style="position:absolute" aria-hidden=true><defs>${await defs(`${P}${T}/sprites.svg`)}</defs></svg>`);
if (F) { r.dataset.one = ''; document.querySelector(`[data-f="${F}"]`)?.closest('.fr').classList.add('one'); }
const json = u => fetch(u).then(r => r.json());

// The mock clock is the status bar's 9:41; `since`/`until` are drawn relative to it, and tick on a real client.
const NOW = Date.parse('2026-11-09T09:41:00Z');
const rel = t => { const m = Math.round(Math.abs(Date.parse(t) - NOW) / 60000); return m < 1 ? 'now' : m < 60 ? `${m}m` : `${Math.round(m / 60)}h`; };
const val = v => v == null ? '' : typeof v === 'object' ? rel(v.since || v.until) : esc(v);
const esc = s => String(s).replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);
// A path on the plugin's backend (`/files/…`) goes through cooee's proxy; here it is this folder.
const src = s => s.replace(/^\/files\/125\//, '');
const ICON = { gear: '⚙', plus: '+', share: '↗', info: 'ⓘ', filter: '⏷' };
const tone = t => t ? ` data-tone=${t}` : '';
const PLAIN = { working: 'working', needs: 'needs you', done: 'done', fail: 'failed', idle: 'idle' };

// Markdown, the small subset: paragraphs, ``` fences, **bold**, `code`, [links](…).
const inl = s => esc(s).replace(/\*\*(.+?)\*\*/g, '<b>$1</b>').replace(/`([^`]+)`/g, '<code>$1</code>').replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2">$1</a>');
const md = (s, b) => s.replace('{rel}', b.until || b.since ? rel(b.until || b.since) : '').split(/\n\n+/).map(p => p.startsWith('```')
  ? `<pre class=diff>${esc(p.replace(/^```\w*\n?|\n?```$/g, ''))}</pre>` : `<p>${inl(p)}</p>`).join('');

const POSE = { working: 'edit', needs: 'needs', done: 'done', fail: 'fail', idle: 'idle' };
const fig = f => !f ? '' : f.coat && STYLE === 'scene' ? `<img class=hrs src="${SPR}horse-${f.coat}-${POSE[f.state]}.png" alt="${esc(theme.words[f.state])}">` : f.state ? `<svg class=rsp viewBox="0 0 24 24" aria-label="${esc(theme.words[f.state] || PLAIN[f.state])}"><use href="#s-${f.state === 'working' ? f.tool || 'edit' : f.state}"/></svg>`
  : f.icon ? `<i class=ms style="--k:var(--l)">${ICON[f.icon] || esc(f.icon)}</i>` : '';
const spark = a => { const lo = Math.min(...a), hi = Math.max(...a), d = hi - lo || 1;
  return `<svg class=spk viewBox="0 0 100 22" preserveAspectRatio=none aria-hidden=true><polyline points="${a.map((v, i) => `${i * 100 / (a.length - 1)},${20 - (v - lo) / d * 18}`).join(' ')}"/></svg>`; };

const B = {
  section: b => `${b.header ? `<h2 class=ch>${esc(b.header)}</h2>` : ''}${all(b.blocks)}${b.footer ? `<p class=sf>${esc(b.footer)}</p>` : ''}`,
  list: b => `<div class="gs${b.rows.some(x => x.figure?.icon) ? ' ic' : ''}">${b.rows.map(B.row).join('')}</div>`,
  row: b => {
    const sub = b.subtitle && (typeof b.subtitle === 'string' ? { text: b.subtitle } : b.subtitle);
    const acc = b.accessory === 'chevron' || (b.tap && !b.accessory && b.style !== 'link');
    return `<a class="gr${sub ? ' two' : ''}${b.indent ? ' in1' : ''}${b.style === 'link' ? ' lk' : ''}${b.style === 'destructive' ? ' rd' : ''}${b.changed ? ' edt' : ''}${b.lifted ? ' lift' : ''}" href="#" data-id="${b.id || ''}">${fig(b.figure)}
      <span class=sk>${esc(b.title)}${sub ? `<small${tone(sub.tone)}>${esc(sub.text)}</small>` : ''}</span>${b.badge ? `<span class=sbd${tone(b.badge.tone)}>${esc(b.badge.text)}</span>` : ''}${
      b.value != null ? `<span class=sv>${val(b.value)}</span>` : ''}${b.button ? `<button type=button class=rbn${tone(b.button.tone)}>${esc(b.button.label)}</button>` : ''}${
      b.handle ? '<span class=ord aria-label=reorder>≡</span>' : acc ? '<span class=cv></span>' : ''}</a>`;
  },
  card: b => { const k = KIND[b.kind];
    return `<div class=ak${tone(b.tone || k?.[2])} data-id="${b.id || ''}">${k ? `<p class=ckd>${ic(k[0])}<b>${k[1]}</b>${FROM && b.kind !== 'ask' ? ` · ${esc(FROM)}` : ''}</p>` : ''}${b.title || b.meta ? `<div class=cd><span class=cdt>${esc(b.title || '')}</span>` : '<div hidden>'}${
      b.badge ? `<span class=sbd${tone(b.badge.tone)}>${esc(b.badge.text)}</span>` : ''}${b.meta ? `<span class=cdm>${esc(b.meta)}</span>` : ''}${
      b.buttons ? `<span class=cdb>${b.buttons.map(x => `<button type=button>${esc(x.label)}</button>`).join('')}</span>` : ''}${
      b.subtitle ? `<span class=cds>${esc(b.subtitle)}</span>` : ''}</div>${all(b.blocks)}</div>`; },
  // Revision 3 (q985): cooee's own cards are blocks. The app draws the lock line, Approve / Deny and the lapse itself.
  run: b => `<pre class="diff rcm">$ ${esc(b.command)}</pre><p class=rmt>${b.sha ? `<code>sha ${esc(b.sha)}</code><br>` : ''}in ${esc(b.dir)}<br>touches ${esc(b.touches)}</p>
    <p class=rbt><button type=button>Approve</button><button type=button data-tone=bad>Deny</button></p>${b.until ? `<p class=ds data-tone=warn>lapses in ${rel(b.until)}</p>` : ''}`,
  secret: b => `<p class=snm>${esc(b.label || b.name)}</p><label class=sfl><input type=password aria-label="${esc(b.name)}" value="${b.set ? '' : 'xxxxxxxxxxxxx'}" placeholder="${b.set ? 'set ✓ · paste to replace' : ''}">${ic('face')}</label>
    <p class=slk>${ic('lock')}Sent to ${FROM ? `the ${esc(FROM)} plugin` : 'this plugin'} only, never to the agent</p><button type=button class=sbp>${esc(b.submit?.label || 'Send to plugin')}</button>`,
  config: b => `<div class=cfg>${b.fields.map(f => `<label class=cfr><span>${esc(f.label)}</span>${f.type === 'toggle' ? `<i class=tg role=switch aria-checked=${!!f.value}></i>`
    : f.type === 'picker' ? `<span class=pkv>${esc(f.value)}</span>` : `<input value="${esc(f.value ?? '')}" placeholder="${esc(f.placeholder || '')}">`}</label>`).join('')}</div>
    <button type=button class=sbp>${esc(b.submit?.label || 'Save')}</button>`,
  choice: b => {
    const o = [...b.options].sort((x, y) => (y.value === b.recommended) - (x.value === b.recommended));
    return `${b.question ? `<p class="aq${inCard ? '' : ' txt'}">${esc(b.question)}</p>` : ''}<p class=chc${inCard ? '' : ' style="margin:8px 16px"'}>${o.map(x =>
      `<button type=button${x.value === b.recommended ? ' class=rec' : ''}${x.confirm?.destructive && x.value !== 'approve' ? ' data-tone=bad' : ''}>${esc(x.label)}</button>`).join('')}</p>${
      b.other ? `<p class=oth${inCard ? '' : ' style="margin:0 16px"'}><input aria-label="other answer" placeholder="${esc(b.other.placeholder)}"></p>` : ''}${
      b.default ? `<p class=ds>${esc(b.default.label.replace('{rel}', rel(b.default.until)))}</p>` : ''}`;
  },
  form: b => `<div class=gs>${b.fields.map(f => `<label class="gr${f.focus ? ' fcs' : ''}${f.doc || f.was || f.error ? ' two' : ''}${f.was ? ' edt' : ''}${f.error ? ' err' : ''}"><span class=lb>${esc(f.label)}${
    f.error ? `<small data-tone=bad>${esc(f.error)}</small>` : f.was ? `<small data-tone=accent>was ${esc(f.was)}</small>` : f.doc ? `<small>${esc(f.doc)}</small>` : ''}</span>${
    f.type === 'toggle' ? `<span class=sk></span><i class=tg role=switch aria-checked=${!!f.value}></i>`
    : f.type === 'picker' ? `<span class=sk></span><span class=pkv>${esc(f.value)}</span>${f.open ? `<span class="pmn${f.open === 'up' ? ' up' : ''}" role=menu>${f.options.map(o =>
      `<i${o === f.value ? ' class=on' : ''}>${esc(o)}</i>`).join('')}</span>` : ''}`
    : f.type === 'secret' ? (f.set ? `<span class=sk></span><span class=sv>•••••••• <small>set ✓</small></span>` : `<input type=password placeholder="${esc(f.placeholder || '')}">`)
    : `<input value="${esc(f.value || '')}" placeholder="${esc(f.placeholder || '')}">`}</label>`).join('')}</div>${
    b.submit ? B.button({ label: b.submit.label, style: 'primary' }) : ''}`,
  button: b => b.style === 'destructive' ? `<div style="height:20px"></div><div class=gs><a class="gr rd cn" href="#">${esc(b.label)}</a></div>`
    : `<button type=button class=${b.style === 'primary' ? 'sbp' : 'sbo'}>${esc(b.label)}</button>`,
  chips: b => `<div class=dch>${b.options.map(o => `<button type=button class=chip${b.selected?.includes(o.value) ? ' aria-pressed=true' : ''}>${esc(o.label)}</button>`).join('')}</div>`,
  stat: b => { const it = b.items || [b];
    return `<div class="st${it.length === 1 ? ' one' : ''}">${it.map(s => `<div class=sti${tone(s.tone)}><b>${esc(s.value)}${s.unit ? ` <small>${esc(s.unit)}</small>` : ''}</b><span>${esc(s.label)}</span>${s.spark ? spark(s.spark) : ''}</div>`).join('')}</div>`; },
  text: b => `<div class="txt${b.style === 'footnote' ? ' ds' : ''}">${md(b.markdown, b)}</div>`,
  images: b => `<div class="sim${b.size === 'large' ? ' lg' : ''}">${b.items.map(i => `<figure><img src="${src(i.src)}" alt="${esc(i.alt)}">${i.caption ? `<figcaption class=sic>${esc(i.caption)}</figcaption>` : ''}</figure>`).join('')}</div>`,
  // #184: a unified diff of a file (fleet.toml): its path, then hunks of ` kept`, `-removed` and `+added` lines.
  diff: b => `<div class=dfb><p class=dff>${esc(b.file)}${b.summary ? `<span>${esc(b.summary)}</span>` : ''}</p>${b.hunks.map(h => `<p class=dfh>${esc(h.at)}</p>${
    h.lines.map(l => `<code class=dl${l[0] === '+' ? ' data-d=add' : l[0] === '-' ? ' data-d=del' : ''}>${esc(l)}</code>`).join('')}`).join('')}</div>`,
  // #184: a text editor (fleet.toml as text), its line numbers, and each validation error inline under its line.
  editor: b => `<div class=edb>${b.text.split('\n').map((l, i) => { const e = (b.errors || []).find(x => x.line === b.first + i);
    return `<code class="el${e ? ' bad' : ''}" data-n=${b.first + i}>${esc(l) || ' '}</code>${e ? `<span class=eer>${esc(e.message)}</span>` : ''}`; }).join('')}</div>`,
  diagram: b => diagram(b, { esc, STYLE, SPR, wide: WIDE, words: theme.words }),
  web: b => `<div class=wv style="--h:${b.height || 180}px"><b>web · sandboxed</b>${esc(b.src)}</div>`,
};
let inCard = false, FROM = '', WIDE = false;
// A card's kind (q985): the app draws its label and the poster's name, which no plugin can set.
const KIND = { ask: ['hand', 'ask', 'bad'], run: ['term', 'run card', 'warn'], secret: ['key', 'secret', 'warn'], config: ['gear', 'config', 'info'],
  proposal: ['gear', 'config proposal', 'info'] };
const one = b => { if (b.type !== 'card') return B[b.type](b); const was = inCard; inCard = true; const h = B.card(b); inCard = was; return h; };
const all = bs => (bs || []).map(one).join('');

// The live stream's ops, keyed by block id (README § Live).
function find(node, id, parent = null) {
  if (!node || typeof node !== 'object') return null;
  if (node.id === id) return { node, parent };
  for (const k of ['blocks', 'rows', 'items']) for (const c of node[k] || []) { const f = find(c, id, node); if (f) return f; }
  return null;
}
const kids = p => p.blocks || p.rows || p.items;

function apply(screen, ops) {
  for (const o of ops) {
    if (o.op === 'badge') { qBadge = o.count; continue; }
    const f = find(screen, o.id || o.into);
    if (!f) continue;
    if (o.op === 'merge') { if (f.node.type === 'diagram' && o.fields.counts) f.node.was = f.node.counts; Object.assign(f.node, o.fields); }
    if (o.op === 'replace') { const a = kids(f.parent); a[a.indexOf(f.node)] = o.block; }
    if (o.op === 'remove') { const a = kids(f.parent); a.splice(a.indexOf(f.node), 1); }
    if (o.op === 'insert') { const a = kids(f.node), at = a.findIndex(x => x.id === (o.before || o.after)); a.splice(at < 0 ? a.length : at + (o.after ? 1 : 0), 0, o.block); }
  }
}

// The host (cooee's native app, q946/q954): no tab bar. Home is the owner's avatar (its menu: Settings, Machines +
// Terminal, Plugins), a pinned personas row and the chat list. A plugin's places (README § Places): drover attaches
// to the Conductor persona: Queue and Events buttons in its chat header open its screens as sheets, and its asks are
// cards in that chat. A plugin's optional menu row sits under the avatar menu's Plugins (hello, drover).
const SB = `<span>9:41</span><span class=si><svg viewBox="0 0 18 12"><rect y="8" width="3" height="4" rx=".7"/><rect x="5" y="5.5" width="3" height="6.5" rx=".7"/><rect x="10" y="3" width="3" height="9" rx=".7"/><rect x="15" width="3" height="12" rx=".7"/></svg><svg viewBox="0 0 16 12"><path d="M8 2.4c2.4 0 4.5.9 6.1 2.4L15.3 3.6A10.3 10.3 0 0 0 8 .6 10.3 10.3 0 0 0 .7 3.6l1.2 1.2A8.6 8.6 0 0 1 8 2.4zm0 3.5c1.4 0 2.6.5 3.6 1.4l1.2-1.2A6.8 6.8 0 0 0 8 4.1 6.8 6.8 0 0 0 3.2 6.1l1.2 1.2c1-.9 2.2-1.4 3.6-1.4zM8 11.5l2.4-2.4a3.4 3.4 0 0 0-4.8 0z"/></svg><svg viewBox="0 0 27 12" style="height:13px"><rect x=".5" y=".5" width="23" height="11" rx="3.5" fill="none" stroke="currentColor" opacity=".4"/><rect x="2" y="2" width="18" height="8" rx="2"/><path d="M25 4v4c.8-.3 1.4-1 1.4-2S25.8 4.3 25 4z" opacity=".4"/></svg></span>`;
const PERS = [['cond', 'Conductor', 'CD', '--l'], ['chief', 'Chief', 'CH', '--link'], ['coder', 'Coder', 'CO', '--c'], ['res', 'Researcher', 'RE', '--p']];
const av = (id, cls = '') => { const p = PERS.find(x => x[0] === id); return `<i class="av ${cls}" style="--av:var(${p[3]})" aria-hidden=true>${p[2]}</i>`; };
const CHATS = [['cond', 'Conductor', 'run card c41: delete #48\'s wip snapshots', 'now', 4], ['chief', 'Chief', 'Every Monday at 09:00 I\'ll check the renewals.', '2m'],
  ['coder', 'Coder', '$ cargo test -p cooee-core', '9m', 2], ['res', 'Researcher', 'Fetch sec.gov … company=kestrel', '12m', 1],
  ['cond', 'Conductor · lifeboat page (#725)', 'herdsman#725 queued after #553', '1h'], ['chief', 'Chief · Q3 board deck', 'Draft 2 is in deck.md', '2d']];
const ICON2 = { queue: '<path d="M9.5 6h11M9.5 12h11M9.5 18h11"/><circle cx="4.5" cy="6" r="1.4"/><circle cx="4.5" cy="12" r="1.4"/><circle cx="4.5" cy="18" r="1.4"/>',
  events: '<path d="M3 12h4l3-7 4 14 3-7h4"/>', more: '<circle cx="5" cy="12" r="1.2"/><circle cx="12" cy="12" r="1.2"/><circle cx="19" cy="12" r="1.2"/>',
  new: '<path d="M12 5v14M5 12h14"/>', hand: '<path d="M8 13V6.5a1.5 1.5 0 0 1 3 0V12m0-7a1.5 1.5 0 0 1 3 0v7m0-5.5a1.5 1.5 0 0 1 3 0V14a6 6 0 0 1-6 6h-.5a6 6 0 0 1-4.6-2.2L3.5 15a1.5 1.5 0 0 1 2.3-1.9L8 15"/>',
  term: '<rect x="3" y="5" width="18" height="14" rx="2"/><path d="m7 10 3 2-3 2M12 15h4"/>', key: '<circle cx="8" cy="8" r="4"/><path d="M11 11l9 9M16 16l2-2M18 18l2-2"/>',
  gear: '<circle cx="12" cy="12" r="3"/><path d="M12 3v3M12 18v3M3 12h3M18 12h3M5.6 5.6l2.1 2.1M16.3 16.3l2.1 2.1M5.6 18.4l2.1-2.1M16.3 7.7l2.1-2.1"/>',
  lock: '<rect x="5" y="11" width="14" height="10" rx="2"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/>',
  face: '<path d="M4 8V6a2 2 0 0 1 2-2h2M16 4h2a2 2 0 0 1 2 2v2M20 16v2a2 2 0 0 1-2 2h-2M8 20H6a2 2 0 0 1-2-2v-2M9 9v1M15 9v1M12 9v4h-1M9 16a4 4 0 0 0 6 0"/>' };
const ic = k => `<svg class=hic viewBox="0 0 24 24" aria-hidden=true>${ICON2[k]}</svg>`;
let qBadge = 7;
const home = () => `<div class=scr><div class=in><div class=pin>${PERS.map(([id, n]) => `<a href="#">${av(id)}<span>${n}</span></a>`).join('')}</div>
  ${CHATS.map(([id, n, last, age, ud]) => `<a class="pr${ud ? ' ud' : ''}" href="#">${av(id)}<span class=an>${esc(n)}</span><span class="ta cv">${age}</span>
    <span class=tp><span>${esc(last)}</span>${ud ? `<i class=bd>${ud}</i>` : ''}</span></a>`).join('')}</div></div>
  <header class="nb hm"><a class=me href="#" aria-label="you"><i class="av s" style="--av:var(--a)">G</i></a><b class=nt style="opacity:1">cooee</b><span><a class=gl href="#" aria-label="new chat">${ic('new')}</a></span></header>`;
const menu = () => `<div class=mdim></div><div class="agm am" role=menu aria-label="you"><p class=gh>Guy · mini</p>
  <a class=gr href="#"><i>⚙</i><span class=sk>Settings</span></a><a class=gr href="#"><i>▣</i><span class=sk>Machines + Terminal</span></a><div class=gp></div>
  <p class=gh>plugins</p><a class=gr href="#"><i>◎</i><span class=sk>drover</span><span class=sv>Conductor</span></a><a class=gr href="#"><i>◎</i><span class=sk>hello</span></a>
  <a class="gr lk" href="#"><i>+</i><span class=sk>Add a plugin…</span></a></div>`;
// Revision 3 (q984): a persona's plugin places are chips on a strip under its chat header (#124's chat-conductor
// frame): the header stays one line however many plugins it has; the strip scrolls sideways.
const CHAT = {
  chat: { p: 'cond', name: 'Conductor', sub: 'drover · 3 fleets', from: 'drover', cards: 'fixtures/cards.json', strip: true, me: "What's waiting on me?",
    ai: 'Four things: a run card that lapses at 09:52, #125\'s design, a routine ask on #722 and synergy #2708 at its round limit.' },
  research: { p: 'res', name: 'Researcher', sub: 'edgar · sec.gov', from: 'edgar', cards: 'fixtures/own-cards.json', me: "Pull Kestrel's last three 10-Ks.",
    ai: 'The edgar plugin needs three things from you first: its settings, its API key and its Python deps.' } };
const strip = () => `<nav class=hs aria-label="drover"><a class=hsc href="#">Queue<i class=hsn>${qBadge}</i></a><a class=hsc href="#">Events</a></nav>`;
const chathead = c => `<header class="nb sc ch2${c.strip ? ' hs2' : ''}"><a class=bk href="#"><b>‹</b></a><span class=ph2>${av(c.p)}<span><b>${c.name}</b><small class=ds>${c.sub}</small></span></span>
  <a class=hbt href="#" aria-label="more">${ic('more')}</a>${c.strip ? strip() : ''}</header>`;
const chat = async (k = 'chat', own) => { const o = own ? await json(own) : {}, c = { ...CHAT[k], ...o }, cards = o.cards || await json(c.cards); FROM = c.from;
  const h = `<div class="cvw${c.strip ? ' hs2' : ''}"><ol class=tc aria-label=messages>
    <li class="blk me"><div class=md>${esc(c.me)}</div><span class=ct>09:37</span></li>
    <li class="blk ai"><b class=k>${c.name}</b><div class=md>${esc(c.ai)}</div></li>
    ${cards.map(x => `<li class=ccd>${one(x)}</li>`).join('')}</ol>
    <form class=tw><button type=button class=att aria-label=attach>+</button><textarea rows=1 aria-label=message placeholder="message ${c.name}"></textarea><button type=button aria-label=send>↑</button></form></div>${chathead(c)}`;
  FROM = ''; return h; };
// A plugin screen in a sheet: its title, its toolbar (⋯ menus), Done; the body is the screen's blocks.
const sheet = (s, off) => `<div class=dim></div><div class="ps sdx${off ? ' off' : ''}${s.blocks.some(x => x.type === 'diagram') ? ' dgw' : ''}" role=dialog aria-label="${esc(s.title)}"><div class=psh>${s.type === 'sheet'
  ? `<a href="#">${esc(s.cancel || 'Cancel')}</a><b>${esc(s.title)}</b><a href="#">${esc(s.done?.label || 'Done')}</a>`
  : `<span>${s.back ? `<a href="#" class=bk2>‹ ${esc(s.back)}</a>` : ''}${(s.actions || []).map(a => `<a href="#" aria-label="${esc(a.label)}">${ic(a.icon) }</a>`).join('')}</span><b>${esc(s.title)}</b><a href="#"${s.done?.off ? ' class=off aria-disabled=true' : ''}>${esc(s.done?.label || 'Done')}</a>`}</div>
  <div class=pb><div class=in>${off ? `<div class=ofl role=status><b>Mac unreachable</b><span>drover's Queue as of ${off} ago · actions are off until it's back</span><a href="#">Retry</a></div>` : ''}${all(s.blocks)}</div></div>${s.pending ? `<div class=pnd><span><b>${esc(s.pending.count)}</b> · ${esc(s.pending.text)}</span><a href="#">Discard</a><button type=button>Review changes</button></div>` : ''}</div>`;
// The attach confirm (revision 2): the app's own sheet, drawn from the manifest's [permissions], never from the plugin.
const attach = m => { const p = m.permissions, row = (ic, t, sub, tone) => ({ type: 'row', figure: { icon: ic }, title: t, subtitle: sub && (tone ? { text: sub, tone } : sub) });
  return sheet({ type: 'sheet', title: 'Attach drover', cancel: 'Cancel', done: { label: 'Attach' }, blocks: [
    { type: 'text', markdown: `**cooee-drover ${m.version}** · \`${m.commit}\` · from the index\n\nAttach it to **Conductor**. It may do only what is listed; cooee refuses anything else.` },
    { type: 'section', header: 'it may', blocks: [{ type: 'list', rows: [
      row('▤', 'Chips on Conductor\'s header strip', 'Queue, Events'), row('▭', 'Post cards in Conductor\'s chats', 'asks, run cards, design reviews'),
      row('!', 'Send notifications', 'tag: drover'), row('≡', 'A row in Plugins', 'drover')] }] },
    { type: 'section', header: 'actions · 12', blocks: [{ type: 'list', rows: [
      row('⚠', 'Always confirm first', p.destructive.join(' · '), 'bad'), row('↩', 'Others', p.actions.filter(a => !p.destructive.includes(a)).join(' · '))] }] },
    { type: 'section', header: 'network', footer: 'Its backend runs on mini as you: node server.mjs. Images and web blocks load only from these hosts.', blocks: [{ type: 'list', rows: p.network.map(h => row('⇄', h)) }] }] }); };
const confirm = c => `<div class=mdim></div><div class=acs><div><p><b>${esc(c.title)}</b>${c.message ? `<br>${esc(c.message)}` : ''}</p><a class=rd href="#">${esc(c.ok)}</a></div><div><a href="#">Cancel</a></div></div>`;

for (const e of document.querySelectorAll('[data-host]')) {
  let over = '', sc;
  WIDE = !!e.dataset.wide;
  if (e.dataset.screen) {
    sc = await json(e.dataset.screen);
    if (e.dataset.back) sc.back = e.dataset.back;
    const deep = b => [b, ...(b.blocks || []).flatMap(deep)];
    // At 1440 a tapped state lists its issues under the map, not on a pushed screen.
    if (e.dataset.select) { sc.blocks.find(x => x.type === 'diagram').selected = e.dataset.select; sc.blocks.push(...(await json(e.dataset.then)).blocks); }
    if (e.dataset.focus) for (const fl of sc.blocks.flatMap(deep).flatMap(x => x.fields || [])) fl.focus = fl.name === e.dataset.focus;
    if (e.dataset.live) for (const ev of await json(e.dataset.live)) apply(sc, ev.ops);
    over += sheet(sc, e.dataset.offline);
  }
  const base = e.dataset.host === 'home' ? home() : await chat(e.dataset.host, e.dataset.chat);
  if ('menu' in e.dataset) over += sc ? `<div class="agm sm" role=menu>${sc.actions[0].menu.map(m => `<a class="gr${m.label === e.dataset.menu ? ' nw' : ''}" href="#"><span class=sk>${esc(m.label)}</span></a>`).join('')}</div>` : menu();
  if (e.dataset.attach) over += attach(await json(e.dataset.attach));
  if (e.dataset.confirm) { const f = find(sc || { blocks: await json('fixtures/cards.json') }, e.dataset.confirm).node; over += confirm(f.confirm || f.options.find(o => o.value === e.dataset.value).confirm); }
  if (e.dataset.wide) e.innerHTML = `<aside class=side>${home()}</aside><main class=dtl>${base}</main>${over}`;
  else e.outerHTML = base + over;
  qBadge = 7;
}
for (const p of document.querySelectorAll('[data-scroll]')) p.querySelector('.pb').style.setProperty('--y', p.dataset.scroll + 'px');
for (const p of document.querySelectorAll('.ph')) p.insertAdjacentHTML('beforeend', `<div class=stb>${SB}</div><i class=isl></i><i class=hind></i>`);
// blocks.html: each example's JSON beside its drawing.
for (const e of document.querySelectorAll('[data-cat]')) {
  const cat = await json(e.dataset.cat), hello = await fetch('../../plugins/hello-py/hello.py').then(r => r.text());
  e.innerHTML = `<h1>The block set, v1 <small>Each block's JSON (left) and this mockup's renderer draws it (right). The native app (SwiftUI, iPhone + Mac) draws the same JSON with its own views.</small></h1>
    <section><h2>hello.py <small>· a whole plugin, 22 lines of Python, no frontend code</small></h2><pre>${esc(hello)}</pre><div class="cv2 sdx">${all([(await json('fixtures/hello.json'))]
      .map(s => ({ type: 'section', blocks: s.blocks })))}</div></section>` +
    cat.map(x => `<section${x.wide ? ' class=wide' : ''}><h2>${esc(x.name)} <small>· ${esc(x.note)}</small></h2><pre>${esc(JSON.stringify(x.block, null, 1).replace(/\n\s+(?=[\]}])/g, ' ').replace(/\[\n\s+/g, '[ '))}</pre><div class="cv2 sdx">${
      x.block.type === 'sheet' ? sheet(x.block).replace('<div class=dim></div>', '') : x.block.type === 'screen' ? `<h1 class=lt style="font-size:28px">${esc(x.block.title)}</h1>${all(x.block.blocks)}`
      : x.block.type === 'confirm' ? confirm(x.block) : x.block.type === 'row' ? B.list({ rows: [x.block] }) : one(x.block)}</div></section>`).join('');
}
document.body.dataset.ready = 1;
