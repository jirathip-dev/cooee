// #181 mockup: drover's Queue with the `scene` block and fold-open rounds, drawn from fixtures/queue.json on the design
// system (web/src/app.css, #116's ns.css, #125's sd.css). One function per block, as #125's sd.js; only what #181
// adds is new here (the scene block, a row's rounds, the look's figure). The art is referenced, never copied: the
// horses and the strip are the native app's own PNGs (apple/Sources/CooeeUI/Sprites/), the row figures the farm pack's.
// `?theme=farm&mode=dark|light&look=scene|modern|terminal`, `?f=<frame>` shows one frame alone (the shots). Fills:
//   [data-host=sheet]   the Queue as a sheet over the Conductor chat; data-open: the row ids folded open; data-y: scroll
//   [data-host=lane]    a lane tapped from the strip: its chat pushed on the sheet (‹ Queue)
//   [data-wide]         the Mac window, the Queue as a centred panel
const W = '../../../web/', P = '../../../packs/themes/', SPR = '../../../apple/Sources/CooeeUI/Sprites/';
const q = new URLSearchParams(location.search);
const T = q.get('theme') || 'farm', MODE = q.get('mode') || 'dark', LOOK = q.get('look') || document.body.dataset.look || 'scene', F = q.get('f');
const json = u => fetch(u).then(r => r.json());
const theme = await json(`${P}${T}/theme.json`), scheme = await json(`${W}schemes/${theme.scheme[MODE]}.json`);
const r = document.documentElement;
for (const [k, v] of Object.entries(scheme.tokens)) r.style.setProperty(k, v);
r.style.colorScheme = scheme['color-scheme'];
// app.css knows two styles (modern, terminal); Scene is Modern's type and shapes plus the strip and the horses.
Object.assign(r.dataset, { theme: T, style: LOOK === 'terminal' ? 'terminal' : 'modern', look: LOOK, scene: 'off', mode: MODE });
const defs = async u => (await fetch(u).then(r => r.text())).replace(/^[\s\S]*?<svg[^>]*>|<\/svg>\s*$/g, '');
document.body.insertAdjacentHTML('afterbegin', `<svg width=0 height=0 style="position:absolute" aria-hidden=true><defs>${await defs(`${P}${T}/sprites.svg`)}</defs></svg>`);
if (F) { r.dataset.one = ''; document.querySelector(`[data-f="${F}"]`)?.closest('.fr').classList.add('one'); }

const NOW = Date.parse('2026-11-09T09:41:00Z');
const rel = t => { const m = Math.round(Math.abs(Date.parse(t) - NOW) / 60000); return m < 1 ? 'now' : m < 60 ? `${m}m` : `${Math.round(m / 60)}h`; };
const val = v => v == null ? '' : typeof v === 'object' ? rel(v.since || v.until) : esc(v);
const esc = s => String(s).replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);
const inl = s => esc(s).replace(/\*\*(.+?)\*\*/g, '<b>$1</b>').replace(/`([^`]+)`/g, '<code>$1</code>');
const tone = t => t ? ` data-tone=${t}` : '';
const PLAIN = { working: 'working', needs: 'needs you', done: 'done', fail: 'failed', idle: 'idle' };
const word = k => theme.words[k] || PLAIN[k];
// The app's, never the plugin's: the lane state's figure per look. Scene: the horse in its coat, acting the state
// (CooeeCore LaneState.pose); Terminal: LaneState.figure; Modern: the theme's row sprite.
const ASCII = { working: '~o=', needs: '(!)', done: '\\o/', idle: '-.-', fail: 'x_x' };
const COATS = ['ch', 'gry', 'blk', 'bay', 'wht'];
const coat = id => COATS[Number(String(id).match(/\d+/)?.[0] || 0) % COATS.length];
const horse = (id, st, cls = 'hf') => `<img class=${cls} src="${SPR}horse-${coat(id)}-${st === 'working' ? 'edit' : st}.png" alt="">`;
const fig = (f, id) => !f?.state ? '' : LOOK === 'scene' ? horse(id, f.state)
  : LOOK === 'terminal' ? `<span class=tf data-k=${f.state} aria-label="${esc(word(f.state))}">${esc(ASCII[f.state])}</span>`
  : `<svg class=rsp viewBox="0 0 24 24" aria-label="${esc(word(f.state))}"><use href="#s-${f.state === 'working' ? f.tool || 'edit' : f.state}"/></svg>`;
const spark = a => { const lo = Math.min(...a), hi = Math.max(...a), d = hi - lo || 1;
  return `<svg class=spk viewBox="0 0 100 22" preserveAspectRatio=none aria-hidden=true><polyline points="${a.map((v, i) => `${i * 100 / (a.length - 1)},${20 - (v - lo) / d * 18}`).join(' ')}"/></svg>`; };
const src = s => s.replace(/^\/files\/730\//, '../native-shell/');

let OPEN = new Set(), inCard = false;
// #181: the scene block. The plugin sends lanes and states only; the app draws the theme's strip, sorts the lanes
// into its places (needs at the gate first, then refused at the fence, the trail, the stable, grazing) and skips
// the block in Modern and Terminal, where the rows below already carry every lane.
const ZONE = ['needs', 'fail', 'working', 'done', 'idle'];
const scene = b => LOOK !== 'scene' ? '' : `<section class=scb aria-label="${esc(b.label || 'lanes')}"><img class=bg src="${SPR}scene-strip.png" alt="">
  <div class=fld>${[...b.lanes].sort((x, y) => ZONE.indexOf(x.state) - ZONE.indexOf(y.state)).map((l, i, a) =>
    `<a class="ln${i && a[i - 1].state !== l.state ? ' nz' : ''}" href="#" data-k=${l.state} aria-label="${esc(`${l.title} ${l.stage}, ${word(l.state)}`)}">${horse(l.id, l.state, 'hs')}
     <span class=nc><b>${esc(l.title)}</b> ${esc(l.stage)}</span><span class=nw>${esc(word(l.state))}</span></a>`).join('')}</div></section>`;

// #181: a row with `rounds` folds open in place (a client toggle, no request), as #116's Workflow row: its rounds on a
// line, each with its state's dot, summary and age; the last is current and open: its lane, its last tool lines, its
// blocks (an ask's buttons) and its links.
const round = (x, last) => `<li class="rn${last ? ' cur' : ''}" data-k=${x.state}><div class=rh><b>${esc(x.name)}</b><span class=rs>${esc(x.summary)}</span><span class="ta${!last && x.tap ? ' cv' : ''}">${rel(x.since)}</span></div>${
  last && (x.lane || x.blocks || x.links) ? `<div class=rnb>${x.lane ? `<p class=ds>${[x.lane.name, x.lane.harness, x.lane.model, x.lane.host].map(esc).join(' · ')}</p>` : ''}${
  x.lane?.lines ? `<ol class=lv>${x.lane.lines.map((l, i, a) => `<li>${i === a.length - 1 ? '▍ ' : ''}${esc(l)}</li>`).join('')}</ol>` : ''}${
  (() => { const was = inCard; inCard = true; const h = all(x.blocks); inCard = was; return h; })()}${
  x.links ? `<p class=chps>${x.links.map(k => `<a class=chip href="#">${esc(k.label)} ›</a>`).join('')}</p>` : ''}</div>` : ''}</li>`;

const B = {
  scene,
  section: b => `${b.header ? `<h2 class=ch>${esc(b.header)}</h2>` : ''}${all(b.blocks)}`,
  list: b => `<div class=gs>${b.rows.map(B.row).join('')}</div>`,
  row: b => {
    const open = OPEN.has(b.id), fold = b.rounds?.length;
    const sub = b.subtitle && (typeof b.subtitle === 'string' ? { text: b.subtitle } : b.subtitle);
    const h = `<a class="gr${sub && !open ? ' two' : ''}${b.style === 'link' ? ' lk' : ''}${open ? ' op' : ''}" href="#" data-id="${b.id || ''}"${fold ? ` aria-expanded=${open}` : ''}>${fig(b.figure, b.id)}
      <span class=sk>${esc(b.title)}${sub && !open ? `<small${tone(sub.tone)}>${esc(sub.text)}</small>` : ''}</span>${b.badge ? `<span class=sbd${tone(b.badge.tone)}>${esc(b.badge.text)}</span>` : ''}${
      b.value != null ? `<span class=sv>${val(b.value)}</span>` : ''}${fold || b.tap ? `<span class="cv${open ? ' dn' : ''}"></span>` : ''}</a>`;
    return open ? `<div class=fo>${h}<ol class=rns>${b.rounds.map((x, i) => round(x, i === b.rounds.length - 1)).join('')}</ol></div>` : h;
  },
  card: b => `<div class=ak${tone(b.tone)} data-id="${b.id || ''}"><div class=cd><span class=cdt>${esc(b.title || '')}</span>${
    b.badge ? `<span class=sbd${tone(b.badge.tone)}>${esc(b.badge.text)}</span>` : ''}${
    b.buttons ? `<span class=cdb>${b.buttons.map(x => `<button type=button>${esc(x.label)}</button>`).join('')}</span>` : ''}${
    b.subtitle ? `<span class=cds>${esc(b.subtitle)}</span>` : ''}</div>${(() => { inCard = true; const h = all(b.blocks); inCard = false; return h; })()}</div>`,
  chips: b => `<div class=dch>${b.options.map(o => `<button type=button class=chip${b.selected?.includes(o.value) ? ' aria-pressed=true' : ''}>${esc(o.label)}</button>`).join('')}</div>`,
  stat: b => `<div class=st>${b.items.map(s => `<div class=sti${tone(s.tone)}><b>${esc(s.value)}${s.unit ? ` <small>${esc(s.unit)}</small>` : ''}</b><span>${esc(s.label)}</span>${s.spark ? spark(s.spark) : ''}</div>`).join('')}</div>`,
  text: b => `<div class=txt><p>${inl(b.markdown)}</p></div>`,
  images: b => `<div class=sim>${b.items.map(i => `<figure><img src="${src(i.src)}" alt="${esc(i.alt)}"></figure>`).join('')}</div>`,
  choice: b => { const o = [...b.options].sort((x, y) => (y.value === b.recommended) - (x.value === b.recommended));
    return `<p class=chc>${o.map(x => `<button type=button${x.value === b.recommended ? ' class=rec' : ''}${x.confirm?.destructive ? ' data-tone=bad' : ''}>${esc(x.label)}</button>`).join('')}</p>${
      b.other ? `<p class=oth><input aria-label="other answer" placeholder="${esc(b.other.placeholder)}"></p>` : ''}`; },
};
const all = bs => (bs || []).map(b => B[b.type]?.(b) ?? '').join('');

const MORE = '<svg class=hic viewBox="0 0 24 24" aria-hidden=true><circle cx="5" cy="12" r="1.2"/><circle cx="12" cy="12" r="1.2"/><circle cx="19" cy="12" r="1.2"/></svg>';
const sheet = (s, y) => `<div class=dim></div><div class="ps sdx" role=dialog aria-label="${esc(s.title)}"><div class=psh><span><a href="#" aria-label=More>${MORE}</a></span><b>${esc(s.title)}</b><a href="#">Done</a></div>
  <div class=pb><div class=in${y ? ` style="--y:${y}px"` : ''}>${all(s.blocks)}</div></div></div>`;
// Behind the sheet: the Conductor chat with drover's chips on its header strip (#125 revision 3).
const behind = () => `<div class=bh><b>Conductor</b><span class=hsc>Queue<i class=hsn>7</i></span><span class=hsc>Events</span></div>`;

// A lane tapped from the strip (`tap: {thread}`): the lane's chat pushed on the sheet, back to Queue.
const laneChat = (s, id) => { const l = s.blocks.find(b => b.type === 'scene').lanes.find(x => x.id === id);
  const fig2 = LOOK === 'scene' ? horse(l.id, l.state, 'lh') : fig({ state: l.state, tool: l.tool }, l.id);
  return `<div class=dim></div><div class="ps sdx lc" role=dialog aria-label="${esc(l.tap.thread)}"><div class=psh><a href="#">‹ Queue</a><b>${esc(l.tap.thread)}</b><a href="#">Done</a></div>
  <div class=lhd>${fig2}<span><b>${esc(l.title)} · <span data-k=${l.state} class=w>${esc(l.stage)}</span></b><small>Claude Code · opus 5.5 · mini · ${rel(l.since)}</small></span><a class=chip href="#">PR #704 ›</a></div>
  <ol class=tc aria-label=messages>
    <li class="blk ai"><b class=k>herdsman-impl-641</b><div class=md>Review 1 found two things: a label that matches two machines opens the setup pane on the Mini, and no test covers it. Fixing both.</div></li>
    <li class="blk run"><span class=k>read</span><span class=cmd>crates/core/src/hosts/bootstrap.rs</span></li>
    <li class="blk run"><span class=k>edit</span><span class=cmd>hosts/bootstrap.rs: one profile per label</span></li>
    <li class="blk ai"><b class=k>herdsman-impl-641</b><div class=md>An ambiguous label now refuses with both machine names instead of picking the first. Running the hosts tests.</div></li>
    <li class="blk run live"><span class=k>run</span><span class=cmd>$ cargo test -p herdsman-core hosts::label</span></li>
  </ol>
  <form class=tw><textarea rows=1 aria-label=message placeholder="message herdsman-impl-641"></textarea><button type=button aria-label=send>↑</button></form></div>`; };

const SIDE = [['Conductor', 'drover · 3 fleets', 'now'], ['Chief', 'Every Monday at 09:00 I\'ll check the renewals.', '2m'], ['Coder', '$ cargo test -p cooee-core', '9m'], ['Researcher', 'Fetch sec.gov … company=kestrel', '12m']];
const mac = () => `<aside class=side><p class=sdh><b>cooee</b></p><div class=gs>${SIDE.map(([n, l, a], i) => `<a class="gr two${i ? '' : ' sel'}" href="#"><span class=sk>${n}<small>${esc(l)}</small></span><span class=sv>${a}</span></a>`).join('')}</div></aside>
  <main class=dtl>${behind()}<ol class=tc><li class="blk me"><div class=md>What's waiting on me?</div></li><li class="blk ai"><b class=k>Conductor</b><div class=md>#730's design and #655 at its round limit.</div></li></ol></main>`;

const screen = await json('fixtures/queue.json');
for (const e of document.querySelectorAll('[data-host]')) {
  OPEN = new Set((e.dataset.open || '').split(' ').filter(Boolean));
  const over = e.dataset.host === 'lane' ? laneChat(screen, e.dataset.lane) : sheet(screen, e.dataset.y);
  if (e.dataset.wide) e.innerHTML = mac() + over;
  else e.outerHTML = behind() + over;
}
const SB = `<span>9:41</span><span class=si><svg viewBox="0 0 18 12"><rect y="8" width="3" height="4" rx=".7"/><rect x="5" y="5.5" width="3" height="6.5" rx=".7"/><rect x="10" y="3" width="3" height="9" rx=".7"/><rect x="15" width="3" height="12" rx=".7"/></svg><svg viewBox="0 0 16 12"><path d="M8 2.4c2.4 0 4.5.9 6.1 2.4L15.3 3.6A10.3 10.3 0 0 0 8 .6 10.3 10.3 0 0 0 .7 3.6l1.2 1.2A8.6 8.6 0 0 1 8 2.4zm0 3.5c1.4 0 2.6.5 3.6 1.4l1.2-1.2A6.8 6.8 0 0 0 8 4.1 6.8 6.8 0 0 0 3.2 6.1l1.2 1.2c1-.9 2.2-1.4 3.6-1.4zM8 11.5l2.4-2.4a3.4 3.4 0 0 0-4.8 0z"/></svg><svg viewBox="0 0 27 12" style="height:13px"><rect x=".5" y=".5" width="23" height="11" rx="3.5" fill="none" stroke="currentColor" opacity=".4"/><rect x="2" y="2" width="18" height="8" rx="2"/><path d="M25 4v4c.8-.3 1.4-1 1.4-2S25.8 4.3 25 4z" opacity=".4"/></svg></span>`;
for (const p of document.querySelectorAll('.ph')) p.insertAdjacentHTML('beforeend', `<div class=stb>${SB}</div><i class=isl></i><i class=hind></i>`);
document.body.dataset.ready = 1;
