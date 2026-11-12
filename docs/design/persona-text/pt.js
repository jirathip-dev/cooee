const q = new URLSearchParams(location.search);
const T = q.get('style') === 'terminal';

const I = {
  edit: '<path d="M4 20h4L19 9l-4-4L4 16zM13 7l4 4"/>',
  chat: '<path d="M4 5h16v11H9l-5 4z"/><path d="M12 8v5M9.5 10.5h5"/>',
  spark: '<path d="M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8zM19 16l.8 2.2L22 19l-2.2.8L19 22l-.8-2.2L16 19l2.2-.8z"/>',
  list: '<path d="M4 6h16M4 12h16M4 18h16"/>',
  wave: '<path d="M3 13l4-6 4 9 4-7 3 4h3"/>',
  trash: '<path d="M4 7h16M10 7V4h4v3M6 7l1 13h10l1-13"/>',
  kbd: '<rect x="3" y="5" width="18" height="11" rx="2"/><path d="M7 9h.01M11 9h.01M15 9h.01M7 12h10M10 20l2 2 2-2"/>',
  bolt: '<path d="M13 3L5 14h6l-1 7 8-11h-6z"/>',
  check: '<path d="M5 12l4 4 10-10"/>',
};
export const ico = k => `<svg class=ico viewBox="0 0 24 24">${I[k]}</svg>`;
const chv = '<i class=chv>›</i>';

const ABOUT = {
  cond: [['Character', '', 'Calm, terse foreman. Says what changed, never how hard it was.'],
    ['Instructions', '', "Run drover's queue for cooee and herdsman."],
    ['Memory', '9 notes', 'n9 · Owner wants every design in the three Styles.'],
    ['Skills', '3', 'release-notes, triage, design-review']],
  coder: [['Character', '', 'Plain-spoken engineer. Shows the diff, not the plan.'],
    ['Instructions', '', 'Work in ~/src/studio. Run the unit tests and the e2e suite.'],
    ['Memory', '14 notes', 'n14 · Kestrel numbers come from the 10-Q, not press releases.'],
    ['Skills', '2', 'export-deck, bisect']],
  ledger: [['Character', '', 'Precise and brief. Asks before it guesses a category.'],
    ['Instructions', '', 'Receipts in, a monthly P&L out.'],
    ['Memory', '0 notes', '—'],
    ['Skills', '0', '—']],
};
function about(e) {
  const hot = e.dataset.hot;
  return `<p class=lb2>About</p><div class=rfl>` + ABOUT[e.dataset.who || 'cond'].map(([n, c, f]) =>
    `<a class="ab${hot === n ? ' hot' : ''}" href="#"><span class=abn>${n}${c ? `<small>${c}</small>` : ''}</span>${chv}<span class=abf>${f}</span></a>`).join('') + '</div>';
}
function panel(e) {
  const who = e.dataset.who || 'cond';
  const routines = who === 'coder' ? [['Nightly e2e', 'Daily 02:00 · next tonight 02:00', 1]]
    : who === 'ledger' ? []
      : [['Morning board', 'Weekdays 08:30 · next tomorrow 08:30', 1], ['Stale PR sweep', 'When CI goes red · ran 2h ago', 1], ['Weekly lessons', 'Fridays 17:00 · paused', 0]];
  const sw = on => `<span style="flex:none;width:51px;height:31px;border-radius:16px;background:${on ? 'var(--g)' : 'var(--edge)'};position:relative"><i style="position:absolute;top:2px;${on ? 'right' : 'left'}:2px;width:27px;height:27px;border-radius:50%;background:var(--knob)"></i></span>`;
  const rt = `<p class=lb2>Routines<a class=lnk2 href="#">${T ? '[+ add]' : '+ Add'}</a></p>` + (routines.length ? `<div class=rfl>${routines.map(([n, w, on]) =>
    `<div class=fr2${on ? '' : ' style="opacity:.75"'}><span class=tx><b>${n}</b><small${on ? '' : ' style="color:var(--a)"'}>${w}</small></span>${T ? `<span>[${on ? 'on ' : 'off'}]</span>` : sw(on)}</div>`).join('')}</div>` : '');
  const places = who === 'cond' ? `<p class=lb2>Places</p><div class=rfl><div class=plc><span class=pic>${ico('list')}</span><span>Queue</span><b class=cnt>6</b>${chv}</div><div class=plc><span class=pic>${ico('wave')}</span><span>Events</span><b class=cnt>12</b>${chv}</div></div>` : '';
  const ab = document.createElement('div');
  ab.dataset.who = who;
  if (e.dataset.hot) ab.dataset.hot = e.dataset.hot;
  return `<div class=in>${rt}${places}${about(ab)}</div>`;
}
const bar = () => `<a class=pbt href="#">${ico('edit')}<span>Edit</span></a><a class=pbt href="#">${ico('chat')}<span>New chat</span></a><a class=pbt href="#">${ico('spark')}<span>Refine by chat</span></a>`;

const SRC = `# Conductor

Run drover's queue for cooee and herdsman.

## Queue

- Take the top issue with no lane.
- One *design* lane at a time.
- Read \`herdsman board\` before you answer.

## Reviews

- A FAIL needs a \`Proof:\` block.
- Re-review every new commit.

### Gates

- [x] CI green at the head
- [ ] PASS at the head sha

## Tone

Short lines. Say what changed.`;
const esc = s => s.replace(/&/g, '&amp;').replace(/</g, '&lt;');
function inline(s) {
  return esc(s).replace(/`([^`]+)`/g, '<code>$1</code>').replace(/\*([^*]+)\*/g, '<em>$1</em>');
}
function md(e) {
  const out = [], heads = [];
  let list = false;
  for (const l of SRC.split('\n').slice(2)) {
    const h = /^(#{2,3}) (.*)/.exec(l);
    if (!l.startsWith('- ') && list) { out.push('</ul>'); list = false; }
    if (h) { out.push(`<h${h[1].length}>${esc(h[2])}</h${h[1].length}>`); heads.push([h[1].length, h[2]]); }
    else if (l.startsWith('- ')) {
      if (!list) { out.push('<ul>'); list = true; }
      const t = /^- \[( |x)\] (.*)/.exec(l);
      out.push(t ? `<li class=tk><input type=checkbox${t[1] === 'x' ? ' checked' : ''} disabled>${inline(t[2])}</li>` : `<li>${inline(l.slice(2))}</li>`);
    } else if (l) out.push(`<p>${inline(l)}</p>`);
  }
  if (list) out.push('</ul>');
  const toc = 'toc' in e.dataset ? `<nav class=toc><p>Contents</p>${heads.map(([n, t]) => `<a class=h${n} href="#">${esc(t)}${chv}</a>`).join('')}</nav>` : '';
  return `<div class=in>${e.dataset.title ? `<h1 class=rlt>${e.dataset.title}</h1><p class=rsb>${e.dataset.sub || ''}</p>` : ''}${toc}<div class=md>${out.join('')}</div></div>`;
}
function src(e) {
  const at = Number(e.dataset.caret || -1);
  return SRC.split('\n').map((l, i) => {
    let h = esc(l);
    if (/^#/.test(l)) h = `<span class=h>${h}</span>`;
    else h = h.replace(/^- (\[[ x]\] )?/, (m) => `<span class=m>${m}</span>`).replace(/`[^`]+`/g, m => `<span class=c>${m}</span>`).replace(/\*[^*]+\*/g, m => `<span class=m>${m}</span>`);
    return i === at ? h + '<i class=caret></i>' : h;
  }).join('\n');
}

const NOTES = [
  ['n9', 'Owner wants every design in the three Styles.', 'Conductor · 2h'],
  ['n8', 'Never merge on the implementer\'s say-so.', 'you · 1d'],
  ['n7', 'herdsman #641 needs the owner before fix 2.', 'Conductor · 2d'],
  ['n6', 'The cap ask carries `herdsman cap`.', 'Conductor · 3d'],
  ['n5', 'Reviewer is a different model from the implementer.', 'you · 4d'],
  ['n4', 'Design lanes run on mini only.', 'Conductor · 5d'],
  ['n3', 'Fridays: lessons digest at 17:00.', 'you · 1w'],
];
function swp(e, id, inner, cls) {
  const [sid, px] = (e.dataset.sw || ':').split(':');
  const del = `<div class="nsw tr"><span class="sa dl2">${T ? '' : ico('trash')}<b>Delete</b></span></div>`;
  return sid === id ? `<div class="sw on">${del}<div class=${cls} style="--x:${px}px">${inner}</div></div>` : `<div class=sw><div class=${cls}>${inner}</div></div>`;
}
function notes(e) {
  const add = 'add' in e.dataset ? `<div class="ads new"><span>Release notes go in CHANGELOG.md<i class=caret></i></span></div>` : '';
  return add + NOTES.map(([id, t, w]) => swp(e, id, `<span><i class=nid>${id}</i>${inline(t)}</span><small>${w}</small>`, 'nt')).join('');
}
const SKILLS = [
  ['release-notes', 'Draft the release notes from merged PRs since the last tag.'],
  ['triage', 'Label a new issue, set its queue flags, ask only for owner calls.'],
  ['design-review', 'Check a design ask: PNGs, README map, the three Styles.'],
];
function skills(e) {
  return SKILLS.map(([n, d]) => swp(e, n, `<span class=ski>${ico('bolt')}</span><b>${n}</b>${chv}<span>${d}</span>`, 'sk')).join('')
    + `<a class=ads href="#">${T ? '[+ new skill]' : '+ New skill'}</a>`;
}
const keys = e => ['#', '-', '*', '`', '[', ']'].map(k => `<i class=mk>${k}</i>`).join('')
  + ('mac' in e.dataset ? '<span class=sc3>⌘S save · esc cancel</span>' : `<i class="mk dn">${T ? '[v]' : ico('kbd')}</i>`);

const X = { about, panel, bar, md, src, notes, skills, keys };
for (const e of document.querySelectorAll('[data-z]')) e.innerHTML = X[e.dataset.z](e);
for (const e of document.querySelectorAll('[data-ic]')) e.insertAdjacentHTML('afterbegin', ico(e.dataset.ic));
await import('../native-feel/nf.js');
