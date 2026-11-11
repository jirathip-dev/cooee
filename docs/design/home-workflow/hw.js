// #229 mockup: the native Home lists your personas' chats only; a session with a `source` label is workflow, drawn in
// the place of the plugin that claims that source (drover: Conductor's Workflow sheet); everything else folds into
// Others. On the live design system (web/src/app.css, the farm pack) and the native host of #116/#125 (ns.css, sd.css).
// As sd.js: `?mode=dark|light`, `?f=<frame>` one frame alone (the shots), else the page's gallery.
const W = '../../../web/', P = '../../../packs/themes/';
const q = new URLSearchParams(location.search);
const T = 'farm', MODE = q.get('mode') || 'dark', F = q.get('f');
const theme = await fetch(`${P}${T}/theme.json`).then(r => r.json());
const scheme = await fetch(`${W}schemes/${theme.scheme[MODE]}.json`).then(r => r.json());
const r = document.documentElement;
for (const [k, v] of Object.entries(scheme.tokens)) r.style.setProperty(k, v);
r.style.colorScheme = scheme['color-scheme'];
Object.assign(r.dataset, { theme: T, style: 'modern', scene: 'off', mode: MODE });
const defs = async u => (await fetch(u).then(r => r.text())).replace(/^[\s\S]*?<svg[^>]*>|<\/svg>\s*$/g, '');
document.body.insertAdjacentHTML('afterbegin', `<svg width=0 height=0 style="position:absolute" aria-hidden=true><defs>${await defs(`${P}${T}/sprites.svg`)}</defs></svg>`);
const esc = s => String(s).replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);

const SB = `<span>9:41</span><span class=si><svg viewBox="0 0 18 12"><rect y="8" width="3" height="4" rx=".7"/><rect x="5" y="5.5" width="3" height="6.5" rx=".7"/><rect x="10" y="3" width="3" height="9" rx=".7"/><rect x="15" width="3" height="12" rx=".7"/></svg><svg viewBox="0 0 16 12"><path d="M8 2.4c2.4 0 4.5.9 6.1 2.4L15.3 3.6A10.3 10.3 0 0 0 8 .6 10.3 10.3 0 0 0 .7 3.6l1.2 1.2A8.6 8.6 0 0 1 8 2.4zm0 3.5c1.4 0 2.6.5 3.6 1.4l1.2-1.2A6.8 6.8 0 0 0 8 4.1 6.8 6.8 0 0 0 3.2 6.1l1.2 1.2c1-.9 2.2-1.4 3.6-1.4zM8 11.5l2.4-2.4a3.4 3.4 0 0 0-4.8 0z"/></svg><svg viewBox="0 0 27 12" style="height:13px"><rect x=".5" y=".5" width="23" height="11" rx="3.5" fill="none" stroke="currentColor" opacity=".4"/><rect x="2" y="2" width="18" height="8" rx="2"/><path d="M25 4v4c.8-.3 1.4-1 1.4-2S25.8 4.3 25 4z" opacity=".4"/></svg></span>`;
const ICON = { new: '<path d="M12 5v14M5 12h14"/>', more: '<circle cx="5" cy="12" r="1.2"/><circle cx="12" cy="12" r="1.2"/><circle cx="19" cy="12" r="1.2"/>',
  tray: '<path d="M3 13h5l1.5 3h5l1.5-3h5"/><path d="M5.5 5h13L21 13v5a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-5z"/>',
  flow: '<circle cx="6" cy="6" r="2.2"/><circle cx="6" cy="18" r="2.2"/><circle cx="18" cy="12" r="2.2"/><path d="M6 8.2v7.6M8 6.8 16 11M8 17.2l8-4.2"/>' };
const ic = k => `<svg class=hic viewBox="0 0 24 24" aria-hidden=true>${ICON[k]}</svg>`;

// Personas (the pinned row) and their chats. A chat row is its persona's avatar, `Name  chat title`, its last line
// (`…` while working, a red `!` when it waits on you), age and unread: apple/design/v1's home-*.
const PERS = { cond: ['Conductor', 'CD', '--g'], chief: ['Chief', 'CH', '--l'], coder: ['Coder', 'CO', '--c'], res: ['Researcher', 'RE', '--p'], acct: ['Account Manager', 'AM', '--link'] };
const av = (id, cls = '') => `<i class="av ${cls}" style="--av:var(${PERS[id][2]})" aria-hidden=true>${PERS[id][1]}</i>`;
const CHATS = [['coder', 'Flaky e2e on the look sheet', '$ cargo test -p cooee-core', 'now', 2, 'working'],
  ['res', 'Kestrel filings', 'Allow fetch sec.gov/cgi-bin/browse-edgar?', '12m', 1, 'needs'],
  ['cond', 'cooee queue', '#229 design is up for your review', '20m', 3, 'working'],
  ['chief', 'Renewals', "Every Monday at 09:00 I'll check the renewals.", '1h'],
  ['acct', 'Birch & Co pricing', 'Due this month: Acme 14 Nov, Birch 21 Nov', '3h'],
  ['chief', 'Q3 board deck', 'Slides 4–9 are in deck/q3.key.', '2d']];
const mark = s => s === 'working' ? '<i class=hwk aria-label=working>•••</i>' : s === 'needs' ? '<i class=hnd aria-label="waits on you">!</i>' : '';
const chatRow = ([id, title, last, age, ud, st], sel) => `<a class="pr hwr${ud ? ' ud' : ''}${sel ? ' sel' : ''}" href="#">${av(id)}<span class=an><b>${esc(PERS[id][0])}</b> <small>${esc(title)}</small></span><span class=ta>${age}</span>
  <span class=tp>${mark(st)}<span>${esc(last)}</span>${ud ? `<i class=bd>${ud}</i>` : ''}</span></a>`;

// Others: chats with no persona, and workflow sessions whose `source` no installed plugin claims.
const LOOSE = [['Bonjour probe', 'Claude Code · scratch', '_cooee._tcp answers on the LAN.', '3h', 1],
  ['jq filter for fleet.toml', 'Codex · ~/src/cooee', '.fleet.cooee.role | keys', '1d'],
  ['New chat', 'Claude Code · scratch', 'Hi! What can I help with?', '4d']];
const NIGHTLY = [['nightly · deps bump', 'source nightly · round 1', 'cargo update -p tokio: 1.47 → 1.48', '6h', 'done']];
// drover's sessions (source=drover, group=repo, item=issue, role, round): what its Workflow sheet holds.
const ISSUES = [
  { k: 'needs', repo: 'cooee', n: 229, stage: 'design', round: 1, title: "native Home: personas' chats only; lanes in their plugin's place", age: '6m',
    rounds: [['design', 'needs', 'mockups up · approve / revise / reject', '6m']] },
  { k: 'fail', repo: 'herdsman', n: 722, stage: 'review', round: 1, title: 'drive: one ask per lane, not per turn', age: '25m',
    rounds: [['impl', 'done', '✓ 2 commits', '1h'], ['review 1', 'fail', 'FAIL · 1 finding', '25m']] },
  { k: 'working', repo: 'cooee', n: 217, stage: 'fix', round: 1, title: "personas: a history tool: search and read the persona's own past threads", age: '9m', open: true,
    rounds: [['impl', 'done', '✓ 3 commits', '2h'], ['review 1', 'fail', 'FAIL · 2 findings', '1h'], ['fix 1', 'working', 'lane live · 9m', 'now']] },
  { k: 'working', repo: 'cooee', n: 184, stage: 'impl', round: 0, title: 'fleets config: every setting as blocks', age: '40m', rounds: [] },
  { k: 'done', repo: 'cooee', n: 167, stage: 'ready', round: 0, title: 'secrets: keychain store readable only by the signed cooee', age: '2h', rounds: [] }];
const ORDER = ['needs', 'fail', 'working', 'done'], WORD = { needs: 'needs you', fail: 'failed', working: 'working', done: 'ready' };
const lanes = ISSUES.flatMap(i => i.rounds.map(x => [i, x]));

const figure = k => `<svg class=rsp viewBox="0 0 24 24" aria-label="${esc(theme.words[k])}"><use href="#s-${k === 'working' ? 'edit' : k}"/></svg>`;
const issueRow = i => `<div class="wr${i.open ? ' op' : ''}"><a class=wh href="#"><span class=fg>${figure(i.k)}</span>
  <span class=wn><b>${i.repo} #${i.n}</b> · <span data-k=${i.k}>${i.stage}${i.round ? ` ${i.round}` : ''}</span></span><span class="ta cv">${i.age}</span>
  <span class=wt><span class=w data-k=${i.k}>${WORD[i.k]}</span> · ${esc(i.title)}</span></a>${i.open ? `<ol class=rns>${i.rounds.map((x, j) => {
    const cur = j === i.rounds.length - 1;
    return `<li class="rn${cur ? ' cur' : ''}" data-k=${x[1]}><a class=rh href="#"><b>${x[0]}</b><span class=rs>${esc(x[2])}</span><span class="ta cv">${x[3]}</span></a>${cur ? `<div class=rnb>
      <ul class=lv><li>✓ Read crates/cooee-core/src/history.rs</li><li>✓ Edit history.rs: cap reads at 64 KiB</li><li>▸ cargo test -p cooee-core history</li></ul>
      <p class=chps><a class=chip href="#">open chat ›</a><a class=chip href="#">PR #228 ›</a></p></div>` : ''}</li>`; }).join('')}</ol>` : ''}</div>`;
const workflowSheet = () => `<div class=dim></div><div class="ps sdx hwsh" role=dialog aria-label=Workflow><div class=psh><span></span><b>Workflow</b><a href="#">Done</a></div>
  <div class=pb><div class=in><p class=hws>drover · 5 issues · <b>1 needs you</b></p>${ORDER.map(k => {
    const is = ISSUES.filter(i => i.k === k);
    return is.length ? `<h2 class=ch>${WORD[k]}</h2><div class=hwg>${is.map(issueRow).join('')}</div>` : ''; }).join('')}</div></div></div>`;

// Screens.
const head = (back, title, extra = '') => `<header class="nb sc"><a class=bk href="#"><b>‹</b>${back}</a><b class=nt>${title}</b><span>${extra}</span></header>`;
const home = sel => `<div class=scr><div class=in><div class=pin>${['chief', 'coder', 'res', 'acct', 'cond'].map(id => `<a href="#">${av(id)}<span>${PERS[id][0].split(' ')[0]}</span></a>`).join('')}</div>
  <h2 class=lt2>Chats</h2>${CHATS.map((c, i) => chatRow(c, sel === i)).join('')}
  ${othersRow(sel === 'others')}</div></div>
  <header class="nb hm"><a class=me href="#" aria-label="you"><i class="av s" style="--av:var(--a)">G</i></a><b class=nt style="opacity:1">cooee</b><span><a class=gl href="#" aria-label="new chat">${ic('new')}</a></span></header>`;
let UNCLAIMED = false;
const othersCount = () => LOOSE.length + NIGHTLY.length + (UNCLAIMED ? lanes.length : 0);
const othersRow = sel => `<a class="pr hwr hwo${sel ? ' sel' : ''}" href="#"><i class="av oav" aria-hidden=true>${ic('tray')}</i><span class=an><b>Others</b></span><span class=ta><i class=hwn>${othersCount()}</i><span class=cv></span></span>
  <span class=tp><span>${[...LOOSE.map(l => l[0]), ...(UNCLAIMED ? ['drover lanes'] : []), 'nightly'].join(', ')}</span></span></a>`;

const looseRow = ([t, sub, last, age, ud], sel) => `<a class="pr hwr${ud ? ' ud' : ''}${sel ? ' sel' : ''}" href="#"><i class="av oav" aria-hidden=true>${t[0].toUpperCase()}</i><span class=an><b>${esc(t)}</b> <small>${esc(sub)}</small></span><span class=ta>${age}</span>
  <span class=tp><span>${esc(last)}</span>${ud ? `<i class=bd>${ud}</i>` : ''}</span></a>`;
const laneRow = ([i, x], sel) => `<a class="pr hwr${sel ? ' sel' : ''}" href="#"><span class="av oav fgv">${figure(x[1])}</span><span class=an><b>${i.repo} #${i.n} · ${x[0]}</b> <small>${esc(i.title)}</small></span><span class=ta>${x[3]}</span>
  <span class=tp><span><span class=w data-k=${x[1]}>${theme.words[x[1]]}</span> · ${esc(x[2])}</span></span></a>`;
const nightlyRow = ([t, sub, last, age, k]) => `<a class="pr hwr" href="#"><span class="av oav fgv">${figure(k)}</span><span class=an><b>${esc(t)}</b> <small>${esc(sub)}</small></span><span class=ta>${age}</span>
  <span class=tp><span>${esc(last)}</span></span></a>`;
const others = sel => `<div class="scr"><div class=in><h1 class=lt>Others</h1><p class=hws>Chats with no persona, and workflow whose source no installed plugin claims.</p>
  <h2 class=ch>chats · no persona</h2>${LOOSE.map((l, i) => looseRow(l, sel === `l${i}`)).join('')}
  ${UNCLAIMED ? `<h2 class=ch>drover · no plugin claims it</h2>${lanes.map((l, i) => laneRow(l, sel === `d${i}`)).join('')}` : ''}
  <h2 class=ch>nightly · no plugin claims it</h2>${NIGHTLY.map(nightlyRow).join('')}
  <p class=sf>${UNCLAIMED ? 'Install drover and attach it to a persona: its sessions move to that persona’s Workflow sheet.' : 'drover claims source drover: its sessions are in Conductor › Workflow.'}</p></div></div>
  ${head('Home', 'Others')}`;

// A chat (the persona's, or a round's session opened as a normal chat). The Conductor's header strip carries drover's
// places: Queue, Events and, new, Workflow (its needs-you count).
const strip = on => `<nav class=hs aria-label=drover><a class=hsc href="#">Queue<i class=hsn>7</i></a><a class=hsc href="#">Events</a><a class="hsc${on ? ' on' : ''}" href="#">${ic('flow')}Workflow<i class=hsn>1</i></a></nav>`;
const chat = c => `<div class="cvw${c.strip ? ' hs2' : ''}"><ol class=tc aria-label=messages>${c.body}</ol>
  <form class=tw><button type=button class=att aria-label=attach>+</button><textarea rows=1 aria-label=message placeholder="${esc(c.ph)}"></textarea><button type=button aria-label=send>↑</button></form></div>
  <header class="nb sc ch2${c.strip ? ' hs2' : ''}"><a class=bk href="#"><b>‹</b>${c.back || ''}</a><span class=ph2>${c.av}<span><b>${esc(c.name)}</b><small class=ds>${esc(c.sub)}</small></span></span>
  <a class=hbt href="#" aria-label="more">${ic('more')}</a>${c.strip ? strip(c.on) : ''}</header>`;
const CONDUCTOR = { strip: true, av: av('cond'), name: 'Conductor', sub: 'Claude Code · opus 5.5', ph: 'Message Conductor',
  body: `<li class="blk me"><div class=md>How's cooee's queue?</div><span class=ct>09:31</span></li>
  <li class="blk ai"><b class=k>Conductor</b><div class=md><p>Five in flight on cooee and herdsman. <b>#229</b>'s design waits on you; #217 is in fix 1 after review 1's two findings.</p></div></li>` };
const CODER = { av: av('coder'), name: 'Coder', sub: 'Codex · gpt-5.5', ph: 'Message Coder',
  body: `<li class="blk me"><div class=md>The look sheet e2e flakes on CI. Find out why.</div><span class=ct>09:20</span></li>
  <li class="blk run"><span class=k>✓</span><span class=cmd>rg -n "waitFor" web/e2e/look.spec.ts</span></li>
  <li class="blk ai"><b class=k>Coder</b><div class=md><p>It waits for the sheet's first frame, not its fonts. Running the suite with a font-ready wait.</p></div></li>
  <li class="blk run"><span class=k>▸</span><span class=cmd>cargo test -p cooee-core</span></li>` };
const ROUND = { back: 'Workflow', av: `<span class="av oav fgv">${figure('working')}</span>`, name: 'cooee #217 · fix 1', sub: 'drover · impl · mini', ph: 'Message this lane',
  body: `<li class="blk me hwb"><div class=md><b>drover</b> · fix round 1: review 1 FAIL, 2 findings. Fix them on issue-217, push, then <code>herdsman turn done</code>.</div><span class=ct>09:32</span></li>
  <li class="blk run"><span class=k>✓</span><span class=cmd>Read crates/cooee-core/src/history.rs</span></li>
  <li class="blk run"><span class=k>✓</span><span class=cmd>Edit history.rs: cap reads at 64 KiB</span></li>
  <li class="blk ai"><b class=k>cooee #217 · fix 1</b><div class=md><p>Finding 1 fixed: <code>read_thread</code> now stops at 64 KiB and says so. Running the history tests before finding 2.</p></div></li>
  <li class="blk run"><span class=k>▸</span><span class=cmd>cargo test -p cooee-core history</span></li>` };

const phone = h => `<section class=ph>${h}<div class=stb>${SB}</div><i class=isl></i><i class=hind></i></section>`;
const mac = (side, main, over = '') => `<section class="wd mac"><aside class=side>${side}</aside><main class=dtl>${main}</main>${over}</section>`;
const FRAMES = {
  home: [['home', 'Home · the pinned personas, then your personas’ chats only (no lane rows), then Others · 3', () => phone(home())],
    ['wide-home', 'Mac · Home as the sidebar, Coder’s chat open', () => mac(home(0), chat(CODER))]],
  workflow: [['workflow', 'Conductor › Workflow · one row per issue, needs you · failed · working · ready; cooee #217 unfolded', () => phone(chat({ ...CONDUCTOR, on: true }) + workflowSheet())],
    ['workflow-round', 'A round tapped (cooee #217 · fix 1): its session opens as a normal chat, back to Workflow', () => phone(chat(ROUND))],
    ['wide-workflow', 'Mac · the Workflow sheet as a panel over Conductor’s chat', () => mac(home(2), chat({ ...CONDUCTOR, on: true }), workflowSheet())]],
  others: [['others', 'Others (drover attached to Conductor) · chats with no persona, and nightly’s unclaimed session', () => phone(others())],
    ['others-unclaimed', 'Others with drover not installed · its sessions fall here, nothing is lost', () => { UNCLAIMED = true; const h = phone(others()); UNCLAIMED = false; return h; }],
    ['wide-others', 'Mac · drover not installed: Others in the sidebar, a drover round open beside it', () => { UNCLAIMED = true; const h = mac(others('d5'), chat({ ...ROUND, back: 'Others' })); UNCLAIMED = false; return h; }]] };

const gal = document.querySelector('.gal');
gal.innerHTML = FRAMES[gal.dataset.page].map(([f, cap, draw]) => `<div class="fr${f.startsWith('wide-') ? ' w' : ''}" data-f="${f}"><p class=cap>${esc(cap)}</p>${draw()}</div>`).join('');
if (F) { r.dataset.one = ''; gal.querySelector(`[data-f="${F}"]`)?.classList.add('one'); }
document.body.dataset.ready = 1;
