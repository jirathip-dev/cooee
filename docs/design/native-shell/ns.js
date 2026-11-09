// #116 mockup: the native shell on the live design system. As chat-first's cf.js, it reads web/'s schemes and the
// farm pack and sets them as cooee's first byte does (shown.rs): `?theme=farm&mode=dark|light`, Modern (Terminal is
// not drawn). `?f=<frame>` shows that frame alone at its own size (the shots); without it the page is a gallery.
// Then it fills:
//   .ph                                   a phone screen's status bar, island and home indicator
//   [data-x=tabs|rail][data-on]           the tab bar (390) or the left rail (1440), the current tab
//   [data-x=personas|threads|scene|workflow|queue|asks|more|persona|chat]   a screen's content
//   [data-st] [data-av] [data-s]          a state's figure and word, an avatar, a 132 × 100 vector horse
const W = '../../../web/', P = '../../../packs/themes/';
const q = new URLSearchParams(location.search);
const T = q.get('theme') || 'farm', MODE = q.get('mode') || 'dark', F = q.get('f');
const theme = await fetch(`${P}${T}/theme.json`).then(r => r.json());
const scheme = await fetch(`${W}schemes/${theme.scheme[MODE]}.json`).then(r => r.json());
const r = document.documentElement;
for (const [k, v] of Object.entries(scheme.tokens)) r.style.setProperty(k, v);
r.style.colorScheme = scheme['color-scheme'];
Object.assign(r.dataset, { theme: T, style: 'modern', scene: 'off' });
const defs = async u => (await fetch(u).then(r => r.text())).replace(/^[\s\S]*?<svg[^>]*>|<\/svg>\s*$/g, '');
document.body.insertAdjacentHTML('afterbegin', `<svg width=0 height=0 style="position:absolute" aria-hidden=true><defs>${
  await defs(`${P}${T}/sprites.svg`)}${await defs('horses.svg')}</defs></svg>`);
if (F) {
  r.dataset.one = '';
  document.querySelector(`[data-f="${F}"]`)?.closest('.fr').classList.add('one');
}

const PLAIN = { working: 'working', needs: 'needs you', done: 'done', fail: 'failed', idle: 'idle' };
const word = k => theme.words[k] || PLAIN[k];
const fig = (st, tool) => `<svg class=rsp viewBox="0 0 24 24" aria-hidden=true><use href="#s-${st === 'working' ? tool || 'edit' : st}"/></svg>`;
const HORSE = { working: 'working', done: 'done', needs: 'blocked', fail: 'blocked', idle: 'graze' };

// The personas (illustrative): a name, an avatar (initials in a scheme colour unless it has a picture), Character and
// Instructions, a memory, a folder and an agent + model; each has one ongoing chat.
const PERSONAS = [
  { id: 'chief', name: 'Chief', av: 'CH', c: '--l', coat: 'ch', st: 'done', age: '2m', last: 'Every Monday at 09:00 I\'ll check the renewals and tell you here.' },
  { id: 'am', name: 'Account Manager', av: 'AM', c: '--link', coat: 'bay', st: 'done', age: '3m', last: 'Due this month: Acme 14 Nov, Birch & Co 21 Nov, Kestrel 28 Nov.' },
  { id: 'coder', name: 'Coder', av: 'CO', c: '--c', coat: 'gry', st: 'working', tool: 'run', age: 'now', last: '$ cargo test -p cooee-core', unread: 2 },
  { id: 'res', name: 'Researcher', av: 'RE', c: '--p', coat: 'blk', st: 'needs', age: '12m', last: 'Fetch sec.gov/cgi-bin/browse-edgar?company=kestrel', unread: 1 },
];
const THREADS = [['chief', 'Q3 board deck', '2d'], ['coder', 'Flaky e2e on the look sheet', '5h'], ['am', 'Birch & Co pricing', '1d']];
const per = id => PERSONAS.find(p => p.id === id);
const av = (id, cls = '') => `<i class="av ${cls}" style="--av:var(${per(id).c})" aria-hidden=true>${per(id).av}</i>`;
const st = (s, tool) => `<b data-st=${s}${tool ? ` data-tool=${tool}` : ''}></b>`;

// Workflow: one row per open issue; its rounds in order, the last the current one. A finished (merged) issue leaves.
const ISSUES = [
  { repo: 'cooee', n: 116, st: 'needs', age: '4m', title: 'Design: native-feel PWA shell, persona agents, one Workflow row per issue',
    rounds: [['design', 'needs', 'needs you · approve, revise or reject', '4m', 'design']] },
  { repo: 'herdsman', n: 601, st: 'fail', age: '2m', title: 'hosts: a remote host without shed is up when its herdr answers',
    rounds: [['impl', 'done', '✓ 2 commits', '48m'], ['review 1', 'fail', 'FAIL · 2 findings', '2m', 'fail']] },
  { repo: 'herdsman', n: 715, st: 'working', tool: 'read', age: '6m', title: 'drive: one row per issue, its rounds folded inside it',
    rounds: [['impl', 'done', '✓ 3 commits', '1h 12m'], ['review 1', 'fail', 'FAIL · 2 findings', '41m'], ['fix 1', 'done', '✓ 1 commit', '22m'], ['review 2', 'working', 'reading drive.rs', '6m', 'live']] },
  { repo: 'cooee', n: 115, st: 'working', tool: 'edit', age: '18m', title: 'Scene: vector horses, not row sprites blown up',
    rounds: [['impl', 'working', 'editing packs/themes/farm', '18m', 'live']] },
  { repo: 'cooee-drover', n: 22, st: 'working', tool: 'run', age: '9m', title: 'Queue: a blocked chain folds to one row',
    rounds: [['impl', 'done', '✓ 4 commits', '2h'], ['review 1', 'fail', 'FAIL · 1 finding', '1h'], ['fix 1', 'working', '$ npm test', '9m', 'live']] },
  { repo: 'cooee', n: 83, st: 'done', age: '1h', stage: 'ready', title: 'chat: herdsman run cards with command, sha, approve and deny',
    rounds: [['impl', 'done', '✓ 5 commits', '3h'], ['review 1', 'done', '✓ PASS', '2h'], ['verify 1', 'done', '✓ PASS · 4 screenshots', '1h'], ['ready', 'done', 'gates green · waits to merge', '1h']] },
];
const BODY = {
  live: i => i.n === 715 ? `<p class=ds>herdsman-rev-715-r2 · Claude Code · opus 5.5 · mini</p>
      <ol class=lv><li>read crates/core/src/drive.rs</li><li>$ cargo test -p herdsman-core drive::fold</li><li>▍ 41 passed · running</li></ol>
      <p class=chps><a class=chip href="#">lane chat ›</a><a class=chip href="#">PR #722 ›</a></p>`
    : `<p class=ds>${i.repo}-impl-${i.n} · Codex · gpt-5.6-luna · air</p><ol class=lv><li>edit packs/themes/farm/sprites.svg</li><li>▍ $ npm test</li></ol>`,
  design: () => `<div class=thb><img src="chats-390-dark.png" alt=""><img src="workflow-390-dark.png" alt=""><img src="persona-390-dark.png" alt=""><img src="row-390-light.png" alt=""></div>
      <p class=chc><button type=button class=rec>approve</button><button type=button>revise</button><button type=button>reject</button></p>`,
  fail: () => `<ul class=fnd><li><b class=r>blocking</b> [correctness] <code>crates/core/src/hosts.rs:88</code> an unreachable herdr reads as up</li>
      <li><b class=r>blocking</b> [test-coverage] <code>crates/cli/tests/shed.rs</code> no test for a host with no shed</li></ul>
      <p class=chps><a class=chip href="#">verdict ›</a><span class=ds>fix 1 next</span></p>`,
};
function row(i, open) {
  const cur = i.rounds.at(-1), stage = i.stage || cur[0];
  return `<div class="wr${open ? ' op' : ''}"><a class=wh href="#"><span class=fg>${fig(i.st, i.tool)}</span>
      <span class=wn><b>${i.repo} #${i.n}</b> · <span data-k=${i.st}>${stage}</span></span><span class="ta cv">${i.age}</span>
      <span class=wt data-k=${i.st}><span class=w>${word(i.st)}</span> · ${i.title}</span></a>${open ? `
    <ol class=rns>${i.rounds.map((x, k) => {
      const last = k === i.rounds.length - 1;
      return `<li class="rn${last ? ' cur' : ''}" data-k=${x[1]}><div class=rh><b>${x[0]}</b><span class=rs>${x[2]}</span><span class="ta${last ? '' : ' cv'}">${x[3]}</span></div>${
        last && x[4] ? `<div class=rnb>${BODY[x[4]](i)}</div>` : ''}</li>`;
    }).join('')}</ol>` : ''}</div>`;
}

const TABS = [
  ['chats', 'Chats', '<path d="M20.5 11.5a8.5 8 0 0 1-12.3 7.2L3.5 20l1.4-4A8 8 0 0 1 3.5 11.5 8.5 8 0 0 1 20.5 11.5z"/>'],
  ['workflow', 'Workflow', '<circle cx="6" cy="5.5" r="2.3"/><circle cx="6" cy="18.5" r="2.3"/><circle cx="18" cy="12" r="2.3"/><path d="M6 7.8v8.4M8.2 6.2c5 .3 8 1.7 9 3.6"/>'],
  ['queue', 'Queue', '<path d="M9.5 6h11M9.5 12h11M9.5 18h11"/><circle cx="4.5" cy="6" r="1.4"/><circle cx="4.5" cy="12" r="1.4"/><circle cx="4.5" cy="18" r="1.4"/>'],
  ['asks', 'Asks', '<circle cx="12" cy="12" r="9"/><path d="M9.6 9.4a2.5 2.5 0 1 1 3.6 2.3c-.7.4-1.2.9-1.2 1.7v.3M12 16.9v.1"/>'],
  ['more', 'More', '<circle cx="12" cy="12" r="9"/><path d="M7.8 12h.01M12 12h.01M16.2 12h.01" stroke-width="2.8"/>'],
];
const tabs = on => TABS.map(([id, label, d]) => `<a href="#"${id === on ? ' aria-current=page' : ''}><svg viewBox="0 0 24 24" aria-hidden=true>${d}</svg>${label}${
  id === 'asks' ? '<i class=tbd>3</i>' : id === 'workflow' ? '<i class=tdt aria-label="needs you"></i>' : ''}</a>`).join('');

const X = {
  tabs: e => tabs(e.dataset.on),
  rail: e => tabs(e.dataset.on),
  personas: e => PERSONAS.map(p => `<a class="pr${p.unread ? ' ud' : ''}${p.id === e.dataset.cur ? ' prs' : ''}" href="#">${av(p.id)}
      <span class=an>${p.name}</span><span class="ta cv">${p.age}</span>
      <span class=tp>${st(p.st, p.tool)}<span>${p.last}</span>${p.unread ? `<i class=bd>${p.unread}</i>` : ''}</span></a>`).join(''),
  threads: () => `<h2 class=ch>side threads</h2>` + THREADS.map(([id, t, age]) => `<a class=sr2 href="#">${av(id, 's')}
      <span class=tt>${t}<small>${per(id).name}</small></span><span class="ta cv">${age}</span></a>`).join(''),
  scene: () => `<div class=scn><section class=sp aria-label=personas><img class=sky src="${P}${T}/scene.svg" alt="">
      <h2 class=fh><b>personas</b><span class=ds>${PERSONAS.length}</span></h2>
      <div class=fld>${PERSONAS.map(p => `<a class="sb${p.unread ? '' : ' rd'}" href="#"><span class="sf cn-${p.coat}"><svg viewBox="0 0 132 100" aria-hidden=true><use href="#h-${HORSE[p.st]}"/></svg></span>
        <span class=sn><b>${p.name}</b><span>${p.age}</span><span class="sw ${p.st}">${word(p.st)}</span></span></a>`).join('')}</div></section></div>`,
  workflow: e => {
    const open = (e.dataset.open || '').split(',').map(Number), only = e.dataset.only && e.dataset.only.split(',').map(Number);
    return (only ? only.map(n => ISSUES.find(i => i.n === n)) : ISSUES).map(i => row(i, open.includes(i.n))).join('');
  },
  queue: () => `<div class=dch><button type=button class=chip>all</button><button type=button class=chip aria-pressed=true>cooee</button><button type=button class=chip>herdsman</button><button type=button class=chip>cooee-drover</button><button type=button class=chip aria-label=legend>?</button></div>
    <div class=fcd><div class=fh><b>cooee</b><button type=button>Drive now</button><button type=button>Pause</button></div>
      <p class=dl><span class=g>●</span> driving · read 2m ago · 3 lanes on mini, air</p>
      <div class=qg><h3>needs you (1)</h3><div class=qr><b class=r>!</b> #116 <span class=r>design</span> <span class=m>4m</span> <span class=tl>Design: native-feel PWA shell</span></div></div>
      <div class=qg><h3>in progress (3)</h3>
        <div class=qr><b class=a>●</b> #115 <span class=a>impl</span> <span class=m>18m</span> <span class=tl>Scene: vector horses</span></div>
        <div class=qr><b class=a>●</b> #119 <span class=a>review(1)</span> <span class=m>7m</span> <span class=tl>look: press states</span></div>
        <div class=qr><b class=g>✓</b> #83 <span class=g>ready</span> <span class=m>1h</span> <span class=tl>chat: herdsman run cards</span></div></div>
      <div class=qg><h3>queued (3)</h3>
        <div class=qr><b class=m>○</b> #120 <span class=m>queued</span> <span class=m>#1</span> <span class=tl>shell: bottom tabs and large titles</span></div>
        <div class=qr><span class=gt>└─</span><b class=m>○</b> #121 <span class=m>queued</span> <span class="tl">persona sheet</span><span class="r">! #120</span></div>
        <div class=qr><b class=m>○</b> #122 <span class=m>queued</span> <span class=m>#2</span> <span class=tl>Workflow: one row per issue</span></div></div>
      <a class=lk href="#">done today: 4</a></div>`,
  asks: () => `<div class="ak own"><p class=ah><span><b>cooee #116</b> · design</span><span>owner · 4m</span></p>
      <p class=aq>Approve the native shell?</p>
      <div class=thb><img src="chats-390-dark.png" alt=""><img src="workflow-390-dark.png" alt=""><img src="persona-390-dark.png" alt=""></div>
      <p class=chc><button type=button class=rec>approve</button><button type=button>revise</button><button type=button>reject</button></p></div>
    <div class=ak><p class=ah><span><b>herdsman #715</b> · review 2</span><span>routine · 18m</span></p>
      <p class=aq>Fold a needs-human issue's rounds as well?</p>
      <p class=chc><button type=button class=rec>fold them</button><button type=button>keep them open</button></p>
      <p class=oth><input aria-label="other answer" placeholder="other…"></p><p class=ds>takes the recommendation in 12m</p></div>
    <div class="ak nh"><p class=ah><span><b>cooee-drover #19</b> · review 4</span><span>needs-human · 1h</span></p>
      <p class=aq>Round limit: review 4 FAIL · 1 finding</p>
      <p class=chc><button type=button>one more round</button><button type=button>merge anyway</button><button type=button>park</button></p></div>`,
  more: () => {
    const g = (h, rows, ic = true) => `${h ? `<h2 class=ch>${h}</h2>` : '<div style="height:12px"></div>'}<div class="gs${ic ? ' ic' : ''}">${rows.map(([k, c, label, v]) =>
      `<a class=gr href="#"><i class=ms style="--k:var(${k})">${c}</i><span class=sk>${label}</span><span class=sv>${v}</span><span class=cv></span></a>`).join('')}</div>`;
    return g('', [['--p', '◐', 'Look', 'Modern · Farm · auto'], ['--r', '!', 'Notifications', 'on']]) +
      g('machines', [['--c', '▣', 'mini', '<i class=dt data-sync=on></i>online'], ['--c', '▣', 'air', '<i class=dt data-sync=on></i>online'], ['--link', '+', 'Pair a device', '']]) +
      g('terminal', [['--fg', '›_', 'Terminal', 'mini']]) +
      g('fleets', [['--g', '⟳', 'cooee', '<i class=dt data-sync=on></i>driving'], ['--g', '⟳', 'herdsman', '<i class=dt data-sync=on></i>driving'], ['--a', 'Ⅱ', 'cooee-drover', '<i class=dt></i>paused']]) +
      g('settings', [['--link', '⚿', 'Sign-ins', '2 accounts'], ['--l', '⇄', 'MCP servers', '3'], ['--a', '⟲', 'Routines', '1']]);
  },
  // The persona sheet's body: new (empty, Create) or edit (Chief).
  persona: e => {
    const nw = 'new' in e.dataset;
    return `<div class=pav>${nw ? '<i class="av xl" style="--av:var(--c)" aria-hidden=true>+</i><a href="#">Choose avatar</a>' : `${av('chief', 'xl')}<a href="#">Edit</a>`}</div>
    <div class=gs><label class=gr><span class=lb>Name</span><input value="${nw ? '' : 'Chief'}" placeholder="Name"></label>
      <div class=gr><span class=lb>Agent</span><button type=button class=agc><b>Claude Code</b>· opus 5.5 · high</button></div></div>
    <h2 class=ch>character</h2><div class=gs><textarea rows=${nw ? 2 : 3} aria-label=Character placeholder="Who it is, its tone">${nw ? '' : 'Calm chief of staff. Short answers in plain words; one question at a time, never a list of them.'}</textarea></div>
    <h2 class=ch>instructions</h2><div class=gs><textarea rows=${nw ? 2 : 4} aria-label=Instructions placeholder="Standing rules">${nw ? '' : 'Ask before sending anything outside cooee.\nRenewals live in renewals.md.\nMondays 09:00: check the renewals and tell me here.'}</textarea></div>
    <h2 class=ch>memory${nw ? '' : ' · 12 notes'}</h2><div class=gs>${nw ? '<div class=gr><span class=sk>Empty</span></div>' : `
      <a class="gr mem" href="#"><span>Dana Okafor runs Kestrel's finance team, since September</span><small>from Chief · 2h</small></a>
      <a class="gr mem" href="#"><span>Renewal notes go in renewals.md, one per client</span><small>from you · 1d</small></a>
      <a class="gr mem" href="#"><span>Account Manager knows the CRM; ask it for dates and owners</span><small>from Chief · 3d</small></a>
      <a class="gr lk" href="#"><span class=sk>Show all 12</span><span class=cv></span></a>
      <a class="gr rd" href="#">Clear memory…</a>`}</div>
    <h2 class=ch>folder</h2><div class=gs><a class=gr href="#"><span class=sk>${nw ? 'scratch' : '~/Clients/crm'}</span><span class=sv>${nw ? 'each chat its own' : '3 chats'}</span><span class=cv></span></a></div>
    ${nw ? '' : '<div style="height:28px"></div><div class=gs><a class="gr rd cn" href="#">Delete Chief</a></div>'}`;
  },
  // Chief's ongoing chat (chat-first's blocks), its composer with the agent chip and the folder chip.
  chat: () => `<ol class=tc aria-label=messages>
      <li class="blk me"><div class=md>Which client renewals are due this month? Draft a short note for each.</div><span class=ct>09:12</span></li>
      <li class="blk run"><span class=k>tool</span><span class=cmd>message_agent Account Manager</span><span class="ts g">✓</span></li>
      <li class="blk run"><span class=k>tool</span><span class=cmd>Write renewals.md</span><span class="ts g">✓</span></li>
      <li class="blk ai"><b class=k>Chief</b><div class=md>Three are due: Acme on the 14th, Birch &amp; Co on the 21st and Kestrel on the 28th. The three notes are in renewals.md.</div></li>
      <li class="blk me"><div class=md>Good. Do this every Monday at 9.</div><span class=ct>09:16</span></li>
      <li class="blk ai"><b class=k>Chief</b><div class=md>Done. Every Monday at 09:00 I'll check the renewals and tell you here.</div></li>
      <li class=te>${st('done')}<span class=ct>09:17 · 5 min · 3 tools</span></li></ol>
    <div class=mc><button type=button class=agc><b>Claude Code</b>· opus 5.5 · high</button><button type=button class=fo>folder: ~/Clients/crm ▾</button></div>
    <form class=tw><textarea rows=1 aria-label=message placeholder="message Chief"></textarea><button type=button>send</button></form>`,
};

const SB = `<span>9:41</span><span class=si><svg viewBox="0 0 18 12"><rect y="8" width="3" height="4" rx=".7"/><rect x="5" y="5.5" width="3" height="6.5" rx=".7"/><rect x="10" y="3" width="3" height="9" rx=".7"/><rect x="15" width="3" height="12" rx=".7"/></svg><svg viewBox="0 0 16 12"><path d="M8 2.4c2.4 0 4.5.9 6.1 2.4L15.3 3.6A10.3 10.3 0 0 0 8 .6 10.3 10.3 0 0 0 .7 3.6l1.2 1.2A8.6 8.6 0 0 1 8 2.4zm0 3.5c1.4 0 2.6.5 3.6 1.4l1.2-1.2A6.8 6.8 0 0 0 8 4.1 6.8 6.8 0 0 0 3.2 6.1l1.2 1.2c1-.9 2.2-1.4 3.6-1.4zM8 11.5l2.4-2.4a3.4 3.4 0 0 0-4.8 0z"/></svg><svg viewBox="0 0 27 12" style="height:13px"><rect x=".5" y=".5" width="23" height="11" rx="3.5" fill="none" stroke="currentColor" opacity=".4"/><rect x="2" y="2" width="18" height="8" rx="2"/><path d="M25 4v4c.8-.3 1.4-1 1.4-2S25.8 4.3 25 4z" opacity=".4"/></svg></span>`;
for (const p of document.querySelectorAll('.ph')) p.insertAdjacentHTML('beforeend', `<div class=stb>${SB}</div><i class=isl></i><i class=hind></i>`);
for (const e of document.querySelectorAll('[data-x]')) e.innerHTML = X[e.dataset.x](e);
for (const e of document.querySelectorAll('[data-av]')) e.outerHTML = av(e.dataset.av, e.dataset.cls || '');
for (const e of document.querySelectorAll('[data-st]')) {
  const s = e.dataset.st;
  e.innerHTML = `${fig(s, e.dataset.tool)}<span class=thw>${word(s)}</span>`;
}
document.body.dataset.ready = 1;
