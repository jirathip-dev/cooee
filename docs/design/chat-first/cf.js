// #86 mockup: the chat-first pages on the live design system. It reads design-system/'s schemes and the packs' themes and sprites and
// sets them as cooee's first byte does (shown.rs): `?style=modern|terminal|scene`, `?theme=farm|kitchen|garden|plain`,
// `?mode=dark|light`. Scene draws the agents as the theme's scene (#34); the chat beside it keeps Modern.
// Then it fills the page:
//   [data-st=working|needs|done|fail|idle]  the figure (sprite, else ascii), the theme word (data-plain: `· plain`)
//   [data-s=<id>]                           a 132 × 100 figure: edit read run idle done needs merged fail
//   #agents[data-cur][data-sub]             the agents: the list (Modern, Terminal) or the scene
const W = '../../../design-system/', P = '../../../packs/themes/';
const q = new URLSearchParams(location.search);
export const STYLE = q.get('style') || 'modern', T = q.get('theme') || 'farm', MODE = q.get('mode') || 'dark';
const theme = await fetch(`${P}${T}/theme.json`).then(r => r.json());
const scheme = await fetch(`${W}schemes/${theme.scheme[MODE]}.json`).then(r => r.json());
const r = document.documentElement;
for (const [k, v] of Object.entries(scheme.tokens)) r.style.setProperty(k, v);
r.style.colorScheme = scheme['color-scheme'];
Object.assign(r.dataset, { theme: T, style: STYLE === 'terminal' ? 'terminal' : 'modern', scene: STYLE === 'scene' ? 'on' : 'off' });
if (theme.sprites) {
  const svg = await fetch(`${P}${T}/sprites.svg`).then(r => r.text());
  document.body.insertAdjacentHTML('afterbegin', `<svg width=0 height=0 style="position:absolute" aria-hidden=true><defs>${svg.replace(/^[\s\S]*?<svg[^>]*>|<\/svg>\s*$/g, '')}</defs></svg>`);
}
// crates/cooee-server/src/look.rs ASCII (herdsman#335), for a theme without sprites.
const ASCII = { edit: ['~o=', '=o~'], read: ['o.o', 'o.-'], run: ['o>_', 'o> '], needs: ['(!)', '[!]'], done: ['\\o/', '|o|'],
  merged: ['\\o/', '-o-'], fail: ['x_x', 'x.x'], idle: ['-.-', '-_-'] };
const PLAIN = { working: 'working', needs: 'needs you', done: 'done', fail: 'failed', idle: 'idle' };
const word = k => theme.words[k] || PLAIN[k];
const fig = (id, big) => theme.sprites
  ? `<svg class=${big ? 'big' : 'rsp'} viewBox="0 0 ${big ? '132 100' : '24 24'}" aria-hidden=true><use href="#s-${id}"/></svg>`
  : `<i class=${big ? 'lsa' : 'rsa'} aria-hidden=true><b>${ASCII[id][0]}</b><b>${ASCII[id][1]}</b></i>${big ? '' : ' '}`;

// The agents (illustrative): each a #40 profile, a name and an avatar (its initials in a scheme colour when the
// profile names no picture), with one ongoing chat and its side threads.
const AGENTS = [
  { id: 'chief', name: 'Chief', av: 'CH', c: '--l', profile: 'Plain', st: 'done', age: '2m', last: 'Every Monday at 09:00 I\'ll check the renewals and tell you here.',
    subs: [['Q3 board deck', '2d']] },
  { id: 'am', name: 'Account Manager', av: 'AM', c: '--link', profile: 'crm', st: 'done', age: '3m', last: 'Due this month: Acme 14 Nov, Birch & Co 21 Nov, Kestrel 28 Nov.' },
  { id: 'coder', name: 'Coder', av: 'CO', c: '--c', profile: 'coder', st: 'working', tool: 'run', age: 'now', last: '$ cargo test -p cooee-core', unread: 1 },
  { id: 'res', name: 'Researcher', av: 'RE', c: '--p', profile: 'Plain', st: 'needs', age: '12m', last: 'Fetch https://www.sec.gov/cgi-bin/browse-edgar?company=kestrel', unread: 1 },
];
export const av = (id, cls = '') => {
  const a = AGENTS.find(a => a.id === id);
  return `<i class="av ${cls}" style="--av:var(${a.c})" aria-hidden=true>${a.av}</i>`;
};
function list(cur, sub) {
  return `<h2 class=ch>agents</h2>` + AGENTS.map(a => `
    <a class="agr${a.unread ? '' : ' rd'}" href="#"${a.id === cur && !sub ? ' aria-current=page' : ''}>
      ${av(a.id)}<span class=an>${a.name}</span><span class=ta>${a.age}</span>
      <span class=tp><b data-st=${a.st}${a.tool ? ` data-tool=${a.tool}` : ''}></b>${a.last}</span>
    </a>${a.id === cur || a.subs ? `<div class=sth>${(a.subs || []).map(([t, age]) =>
      `<a class=str href="#"><span>${t}</span><span class=ta>${age}</span></a>`).join('')}${sub && a.id === cur
      ? `<a class=str href="#" aria-current=page><span>New chat</span><span class=ta>now</span></a>` : ''}${a.id === cur
      ? `<a class="str nw" href="#">+ new chat</a>` : ''}</div>` : ''}`).join('') +
    `<a class=agn href="#">+ new agent</a>`;
}
// Scene: one page, the agents as the scene's sprites over their name cards (#34's markup); the current one's chats
// as a row of chips under the scene.
function scene(cur, sub) {
  const pose = a => a.st === 'working' ? a.tool || 'edit' : a.st;
  const c = AGENTS.find(a => a.id === cur);
  return `<div class=scn><div class=spg><section class=sp aria-label=agents>
    <img class=sky src="scene/${T}.svg" alt="">
    <h2 class=fh><b>agents</b><span class=ds>${AGENTS.length} agents</span></h2>
    <div class=fld>${AGENTS.map(a => `
      <a class="sb${a.unread ? '' : ' rd'}" href="#"${a.id === cur ? ' aria-current=page' : ''}>
        <span class=sf>${fig(pose(a), true)}</span>
        <span class=sn><b>${a.name}</b><span>${a.profile} · ${a.age}</span><span class="sw ${a.st}">${word(a.st)}</span></span>
      </a>`).join('')}
    </div></section></div></div>
    <nav class=scc aria-label="${c.name}'s chats"><a class=chip href="#"${sub ? '' : ' aria-current=page'}>${c.name}</a>${(c.subs || [])
      .map(([t]) => `<a class=chip href="#">${t}</a>`).join('')}${sub ? '<a class=chip href="#" aria-current=page>New chat</a>' : ''}<a class=chip href="#">+ new chat</a></nav>`;
}

// A 390 shot shows each phone screen in turn: [data-twin] is drawn again under its caption, there with its sheet open.
for (const e of document.querySelectorAll('[data-twin]')) {
  const c = e.cloneNode(true);
  c.classList.add('ph390');
  delete c.dataset.twin;
  e.after(Object.assign(document.createElement('p'), { className: 'cap ph390', textContent: e.dataset.twin }), c);
}
const g = document.getElementById('agents');
if (g) g.innerHTML = STYLE === 'scene' ? scene(g.dataset.cur, 'sub' in g.dataset) : list(g.dataset.cur, 'sub' in g.dataset);
for (const e of document.querySelectorAll('[data-av]')) e.outerHTML = av(e.dataset.av, e.dataset.cls || '');
for (const e of document.querySelectorAll('[data-st]')) {
  const st = e.dataset.st, w = word(st);
  e.innerHTML = `${fig(st === 'working' ? e.dataset.tool || 'edit' : st)}<span class=thw>${w}</span>${'plain' in e.dataset && w !== PLAIN[st] ? `<small class=pw> · ${PLAIN[st]}</small>` : ''}`;
}
for (const e of document.querySelectorAll('[data-s]')) e.innerHTML = fig(e.dataset.s, true);
document.body.dataset.ready = 1;
