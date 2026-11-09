// #125 mockup: plugin screens as data. One renderer draws any fixture (fixtures/*.json) on the live design system,
// #116's native shell (../native-shell/ns.css). It is the web client's half of the spec in README.md, in small: one
// function per block. The Swift and herdsman-board clients would map the same blocks to their own views.
// As ns.js: `?theme=farm&mode=dark|light`, `?f=<frame>` shows one frame alone (the shots), else a gallery. Fills:
//   [data-screen=<fixture>]                  a phone screen: the plugin's screen in the shell (data-on: the tab)
//   [data-sheet=<fixture>]                   a sheet over the screen
//   [data-live=<stream fixture>]             apply a stream's ops to the screen first (the live frame)
//   [data-confirm=<block id>]                that block's confirm, as the client's own action sheet
//   [data-cat]                               blocks.html's catalogue: each block's JSON beside its drawing
const W = '../../../web/', P = '../../../packs/themes/';
const q = new URLSearchParams(location.search);
const T = q.get('theme') || 'farm', MODE = q.get('mode') || 'dark', F = q.get('f');
const theme = await fetch(`${P}${T}/theme.json`).then(r => r.json());
const scheme = await fetch(`${W}schemes/${theme.scheme[MODE]}.json`).then(r => r.json());
const r = document.documentElement;
for (const [k, v] of Object.entries(scheme.tokens)) r.style.setProperty(k, v);
r.style.colorScheme = scheme['color-scheme'];
Object.assign(r.dataset, { theme: T, style: 'modern', scene: 'off', mode: MODE });
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

const fig = f => !f ? '' : f.state ? `<svg class=rsp viewBox="0 0 24 24" aria-label="${esc(theme.words[f.state] || PLAIN[f.state])}"><use href="#s-${f.state === 'working' ? f.tool || 'edit' : f.state}"/></svg>`
  : f.icon ? `<i class=ms style="--k:var(--l)">${ICON[f.icon] || esc(f.icon)}</i>` : '';
const spark = a => { const lo = Math.min(...a), hi = Math.max(...a), d = hi - lo || 1;
  return `<svg class=spk viewBox="0 0 100 22" preserveAspectRatio=none aria-hidden=true><polyline points="${a.map((v, i) => `${i * 100 / (a.length - 1)},${20 - (v - lo) / d * 18}`).join(' ')}"/></svg>`; };

const B = {
  section: b => `${b.header ? `<h2 class=ch>${esc(b.header)}</h2>` : ''}${all(b.blocks)}${b.footer ? `<p class=sf>${esc(b.footer)}</p>` : ''}`,
  list: b => `<div class="gs${b.rows.some(x => x.figure?.icon) ? ' ic' : ''}">${b.rows.map(B.row).join('')}</div>`,
  row: b => {
    const sub = b.subtitle && (typeof b.subtitle === 'string' ? { text: b.subtitle } : b.subtitle);
    const acc = b.accessory === 'chevron' || (b.tap && !b.accessory && b.style !== 'link');
    return `<a class="gr${sub ? ' two' : ''}${b.indent ? ' in1' : ''}${b.style === 'link' ? ' lk' : ''}${b.style === 'destructive' ? ' rd' : ''}" href="#" data-id="${b.id || ''}">${fig(b.figure)}
      <span class=sk>${esc(b.title)}${sub ? `<small${tone(sub.tone)}>${esc(sub.text)}</small>` : ''}</span>${b.badge ? `<span class=sbd${tone(b.badge.tone)}>${esc(b.badge.text)}</span>` : ''}${
      b.value != null ? `<span class=sv>${val(b.value)}</span>` : ''}${acc ? '<span class=cv></span>' : ''}</a>`;
  },
  card: b => `<div class=ak${tone(b.tone)} data-id="${b.id || ''}"><div class=cd><span class=cdt>${esc(b.title)}</span>${
      b.badge ? `<span class=sbd${tone(b.badge.tone)}>${esc(b.badge.text)}</span>` : ''}${b.meta ? `<span class=cdm>${esc(b.meta)}</span>` : ''}${
      b.buttons ? `<span class=cdb>${b.buttons.map(x => `<button type=button>${esc(x.label)}</button>`).join('')}</span>` : ''}${
      b.subtitle ? `<span class=cds>${esc(b.subtitle)}</span>` : ''}</div>${all(b.blocks)}</div>`,
  choice: b => {
    const o = [...b.options].sort((x, y) => (y.value === b.recommended) - (x.value === b.recommended));
    return `${b.question ? `<p class="aq${inCard ? '' : ' txt'}">${esc(b.question)}</p>` : ''}<p class=chc${inCard ? '' : ' style="margin:8px 16px"'}>${o.map(x =>
      `<button type=button${x.value === b.recommended ? ' class=rec' : ''}${x.confirm?.destructive && x.value !== 'approve' ? ' data-tone=bad' : ''}>${esc(x.label)}</button>`).join('')}</p>${
      b.other ? `<p class=oth${inCard ? '' : ' style="margin:0 16px"'}><input aria-label="other answer" placeholder="${esc(b.other.placeholder)}"></p>` : ''}${
      b.default ? `<p class=ds>${esc(b.default.label.replace('{rel}', rel(b.default.until)))}</p>` : ''}`;
  },
  form: b => `<div class=gs>${b.fields.map(f => `<label class=gr><span class=lb>${esc(f.label)}</span>${
    f.type === 'toggle' ? `<span class=sk></span><i class=tg role=switch aria-checked=${!!f.value}></i>`
    : f.type === 'picker' ? `<span class=sk></span><span class=pkv>${esc(f.value)}</span>`
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
  web: b => `<div class=wv style="--h:${b.height || 180}px"><b>web · sandboxed</b>${esc(b.src)}</div>`,
};
let inCard = false;
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
let tabBadge = 4;
function apply(screen, ops) {
  for (const o of ops) {
    if (o.op === 'badge') { tabBadge = o.count; continue; }
    const f = find(screen, o.id || o.into);
    if (!f) continue;
    if (o.op === 'merge') Object.assign(f.node, o.fields);
    if (o.op === 'replace') { const a = kids(f.parent); a[a.indexOf(f.node)] = o.block; }
    if (o.op === 'remove') { const a = kids(f.parent); a.splice(a.indexOf(f.node), 1); }
    if (o.op === 'insert') { const a = kids(f.node), at = a.findIndex(x => x.id === (o.before || o.after)); a.splice(at < 0 ? a.length : at + (o.after ? 1 : 0), 0, o.block); }
  }
}

// The host's chrome: #116's tab bar (Chats · Workflow · Queue · Asks · More), status bar, island, home indicator.
const TABS = [
  ['chats', 'Chats', '<path d="M20.5 11.5a8.5 8 0 0 1-12.3 7.2L3.5 20l1.4-4A8 8 0 0 1 3.5 11.5 8.5 8 0 0 1 20.5 11.5z"/>'],
  ['workflow', 'Workflow', '<circle cx="6" cy="5.5" r="2.3"/><circle cx="6" cy="18.5" r="2.3"/><circle cx="18" cy="12" r="2.3"/><path d="M6 7.8v8.4M8.2 6.2c5 .3 8 1.7 9 3.6"/>'],
  ['queue', 'Queue', '<path d="M9.5 6h11M9.5 12h11M9.5 18h11"/><circle cx="4.5" cy="6" r="1.4"/><circle cx="4.5" cy="12" r="1.4"/><circle cx="4.5" cy="18" r="1.4"/>'],
  ['asks', 'Asks', '<circle cx="12" cy="12" r="9"/><path d="M9.6 9.4a2.5 2.5 0 1 1 3.6 2.3c-.7.4-1.2.9-1.2 1.7v.3M12 16.9v.1"/>'],
  ['more', 'More', '<circle cx="12" cy="12" r="9"/><path d="M7.8 12h.01M12 12h.01M16.2 12h.01" stroke-width="2.8"/>'],
];
const tabs = on => TABS.map(([id, label, d]) => `<a href="#"${id === on ? ' aria-current=page' : ''}><svg viewBox="0 0 24 24" aria-hidden=true>${d}</svg>${label}${
  id === 'asks' ? `<i class=tbd>${tabBadge}</i>` : id === 'workflow' ? '<i class=tdt aria-label="needs you"></i>' : ''}</a>`).join('');
const SB = `<span>9:41</span><span class=si><svg viewBox="0 0 18 12"><rect y="8" width="3" height="4" rx=".7"/><rect x="5" y="5.5" width="3" height="6.5" rx=".7"/><rect x="10" y="3" width="3" height="9" rx=".7"/><rect x="15" width="3" height="12" rx=".7"/></svg><svg viewBox="0 0 16 12"><path d="M8 2.4c2.4 0 4.5.9 6.1 2.4L15.3 3.6A10.3 10.3 0 0 0 8 .6 10.3 10.3 0 0 0 .7 3.6l1.2 1.2A8.6 8.6 0 0 1 8 2.4zm0 3.5c1.4 0 2.6.5 3.6 1.4l1.2-1.2A6.8 6.8 0 0 0 8 4.1 6.8 6.8 0 0 0 3.2 6.1l1.2 1.2c1-.9 2.2-1.4 3.6-1.4zM8 11.5l2.4-2.4a3.4 3.4 0 0 0-4.8 0z"/></svg><svg viewBox="0 0 27 12" style="height:13px"><rect x=".5" y=".5" width="23" height="11" rx="3.5" fill="none" stroke="currentColor" opacity=".4"/><rect x="2" y="2" width="18" height="8" rx="2"/><path d="M25 4v4c.8-.3 1.4-1 1.4-2S25.8 4.3 25 4z" opacity=".4"/></svg></span>`;

const nav = s => `<header class="nb${s.large === false ? ' sc' : ''}">${s.back ? `<a class=bk href="#"><b>‹</b>${esc(s.back)}</a>` : '<span></span>'}<b class=nt${s.large === false ? ' style="opacity:1"' : ''}>${esc(s.title)}</b><span>${
  (s.actions || []).map(a => `<a class=gl href="#" aria-label="${esc(a.label)}">${ICON[a.icon] || esc(a.label)}</a>`).join('')}</span></header>`;
const body = s => `${s.large === false ? '' : `<h1 class=lt>${esc(s.title)}</h1>`}${all(s.blocks)}`;
const screen = (s, on) => `<div class=scr><div class=in>${body(s)}</div></div>${nav(s)}<nav class=tbar aria-label=tabs>${tabs(on)}</nav>`;
const sheet = s => `<div class=dim></div><div class="ps sdx" role=dialog aria-label="${esc(s.title)}"><div class=psh><a href="#">${esc(s.cancel || 'Cancel')}</a><b>${esc(s.title)}</b><a href="#">${esc(s.done?.label || 'Done')}</a></div>
  <div class=pb><div class=in>${all(s.blocks)}</div></div></div>`;
const confirm = c => `<div class=mdim></div><div class=acs><div><p><b>${esc(c.title)}</b>${c.message ? `<br>${esc(c.message)}` : ''}</p><a class=rd href="#">${esc(c.ok)}</a></div><div><a href="#">Cancel</a></div></div>`;

for (const e of document.querySelectorAll('[data-screen]')) {
  const s = await json(e.dataset.screen);
  if (e.dataset.back) s.back = e.dataset.back; // the host's back button: a plugin pushed from More
  if (e.dataset.live) for (const ev of await json(e.dataset.live)) apply(s, ev.ops);
  let h = screen(s, e.dataset.on);
  if (e.dataset.sheet) h += sheet(await json(e.dataset.sheet));
  if (e.dataset.confirm) { const f = find(s, e.dataset.confirm).node; h += confirm((f.options || []).find(o => o.value === e.dataset.value)?.confirm || f.confirm); }
  e.outerHTML = h; tabBadge = 4;
}
for (const p of document.querySelectorAll('.ph')) p.insertAdjacentHTML('beforeend', `<div class=stb>${SB}</div><i class=isl></i><i class=hind></i>`);
// 1440: the rail, a screen as the list column, another (the pushed one) as the detail.
for (const e of document.querySelectorAll('[data-wide]')) {
  const [l, d] = await Promise.all(e.dataset.wide.split(' ').map(json));
  e.innerHTML = `<nav class=rail aria-label=tabs>${tabs(e.dataset.on)}</nav><div class=lst>${nav({ ...l, title: '' })}<div class=scr><div class=in>${body(l)}</div></div></div>
    <main class=dtl>${nav({ ...d, large: false })}<div class=scr><div class=in>${all(d.blocks)}</div></div></main>`;
}
// blocks.html: each example's JSON beside its drawing.
for (const e of document.querySelectorAll('[data-cat]')) {
  const cat = await json(e.dataset.cat), hello = await fetch('hello/hello.py').then(r => r.text());
  e.innerHTML = `<h1>The block set, v1 <small>Each block's JSON (left) and how cooee's web client draws it (right). The Swift app and herdsman's board draw the same JSON with their own views.</small></h1>
    <section><h2>hello.py <small>· a whole plugin, 22 lines of Python, no frontend code</small></h2><pre>${esc(hello)}</pre><div class="cv2 sdx">${all([(await json('fixtures/hello.json'))]
      .map(s => ({ type: 'section', blocks: s.blocks })))}</div></section>` +
    cat.map(x => `<section${x.wide ? ' class=wide' : ''}><h2>${esc(x.name)} <small>· ${esc(x.note)}</small></h2><pre>${esc(JSON.stringify(x.block, null, 1).replace(/\n\s+(?=[\]}])/g, ' ').replace(/\[\n\s+/g, '[ '))}</pre><div class="cv2 sdx">${
      x.block.type === 'sheet' ? sheet(x.block).replace('<div class=dim></div>', '') : x.block.type === 'screen' ? `<h1 class=lt style="font-size:28px">${esc(x.block.title)}</h1>${all(x.block.blocks)}`
      : x.block.type === 'confirm' ? confirm(x.block) : x.block.type === 'row' ? B.list({ rows: [x.block] }) : one(x.block)}</div></section>`).join('');
}
document.body.dataset.ready = 1;
