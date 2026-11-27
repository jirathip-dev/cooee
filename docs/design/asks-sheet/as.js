// #852 mockup renderer: asks.json drawn as the chat (one line, the Asks chip) or the Asks sheet, per look and mode.
const q = new URLSearchParams(location.search);
const look = q.get('look') || 'modern', mode = q.get('mode') || 'dark', page = document.body.dataset.page;
document.documentElement.dataset.look = look;
document.documentElement.dataset.mode = mode;
const wide = innerWidth >= 900;
const sprite = (coat, pose) => `../../../apple/Sources/CooeeUI/Sprites/horse-${coat}-${pose}.png`;
const FIG = { fail: 'x_x', needs: '(!)', idle: '-.-' };
const esc = s => s.replace(/&/g, '&amp;').replace(/</g, '&lt;');
const md = s => esc(s).replace(/\*\*(.+?)\*\*/g, '<b>$1</b>').replace(/`(.+?)`/g, '<code>$1</code>');

const d = await (await fetch('asks.json')).json();
const sheet = page === 'sheet';
const open = d.asks.filter(a => !(sheet && a.answered));
const rank = Object.fromEntries(d.groups.map((g, i) => [g.id, i]));
const sorted = [...d.asks].sort((a, b) => rank[a.group] - rank[b.group]);

function figure(a) {
  if (look === 'scene') return `<img class="fig" src="${sprite(a.coat, a.fig)}" alt="">`;
  if (look === 'terminal') return `<span class="tf">${FIG[a.fig]}</span>`;
  return '';
}

function header() {
  const n = open.length;
  const chips = d.chat.chips.map(c => `<span class="chp">${c.label}${c.count ? `<span class="n">${c.count}</span>` : ''}</span>`).join('')
    + (n ? `<span class="chp ask${sheet ? ' on' : ''}">Asks<span class="n">${n}</span></span>` : '');
  const av = look === 'scene' ? `<span class="av"><img src="${sprite('bay', 'idle')}" alt=""></span>` : `<span class="av">${look === 'terminal' ? '~o=' : '◉'}</span>`;
  const back = wide ? '' : `<span class="rb">${look === 'terminal' ? '[&lt;]' : '‹'}</span>`;
  const more = wide ? '' : `<span class="rb">${look === 'terminal' ? '[…]' : '…'}</span>`;
  return `<div class="hd">${back}${av}<div class="ti"><b>${d.chat.title}</b><small>${d.chat.agent} · ${d.chat.model} ⌄</small></div>${more}<div class="chips">${chips}</div></div>`;
}

function thread() {
  return d.chat.thread.map(m => {
    if (m.who === 'you') return `<div class="me">${esc(m.text)}</div>`;
    if (m.who === 'tool') return `<div class="tool"><b>${look === 'terminal' ? m.name.toLowerCase() : '▤ ' + m.name}</b><code>${m.arg}</code><span class="ok">${look === 'terminal' ? '[ok]' : '✓'}</span><span class="mr">${m.more} ›</span></div>`;
    return `<div class="ag">${m.md.map(p => `<p>${md(p)}</p>`).join('')}</div>`;
  }).join('');
}

function line() {
  const n = open.length;
  if (!n) return '';
  const top = [...open].sort((a, b) => rank[a.group] - rank[b.group])[0];
  const lead = look === 'scene' ? `<img class="fig" src="${sprite(top.coat, top.fig)}" alt="">` : `<span class="hand">${look === 'terminal' ? '[!]' : '✋'}</span>`;
  return `<div class="nl">${lead}<b>${n} need you</b><span class="top">· ${top.head}: ${top.title}</span><span class="go">${look === 'terminal' ? '[&gt;]' : '›'}</span></div>`;
}

function composer() {
  return `<div class="cmp"><span class="rb">${look === 'terminal' ? '[+]' : '+'}</span><span class="in">Message ${d.chat.title}</span><span class="snd">↑</span></div>`;
}

function ask(a) {
  if (sheet && a.answered) {
    const w = a.answered;
    return `<div class="gone"><span class="ok">${look === 'terminal' ? '[x]' : '✓'}</span><span><b>${a.head}</b> ${w.option} · ${w.by} · ${w.when}</span><span class="out">leaving</span></div>`;
  }
  const imgs = a.images ? `<div class="imgs">${a.images.map(s => `<img src="${s}" alt="">`).join('')}</div>` : '';
  const ops = a.options.map(o => {
    const cls = o.primary ? 'p' : o.danger ? 'd' : o.label === 'later' ? 'l' : '';
    return `<span class="opt ${cls}">${o.label}${o.rec ? ' · recommended' : ''}</span>`;
  }).join('');
  return `<div class="ak" data-g="${a.group}"><div class="ah">${figure(a)}<b>${a.head}</b><span class="tag">${a.tag}</span><span class="age">${a.age}</span></div>`
    + `<h4>${esc(a.title)}</h4><p>${esc(a.text)}</p>${imgs}<div class="ops">${ops}</div></div>`;
}

function sheetView() {
  const groups = d.groups.map(g => {
    const rows = sorted.filter(a => a.group === g.id);
    if (!rows.length) return '';
    const live = rows.filter(a => !a.answered).length;
    return `<section class="grp" data-g="${g.id}"><h3>${g.label} · ${live}</h3>${rows.map(ask).join('')}</section>`;
  }).join('');
  return `<div class="sh">${wide ? '' : '<div class="grab"></div>'}<div class="shh"><b>Asks<span class="c">${open.length}</span></b><a>${look === 'terminal' ? '[done]' : 'Done'}</a></div><div class="sbd">${groups}</div></div>`;
}

const chat = `<div class="phone">${wide ? '' : '<div class="sb">9:41<i>▂▄▆ ◠ ▭</i></div>'}${header()}<div class="th${wide ? ' col' : ''}">${thread()}</div>${line()}${composer()}</div>`;
if (wide) {
  const side = `<aside class="sd"><h2>cooee</h2>${d.sidebar.map(s => `<div class="sr${s.sel ? ' sel' : ''}"><b>${s.name}</b><small>${s.sub}</small><span class="t">${s.age}</span></div>`).join('')}</aside>`;
  document.body.innerHTML = `<div class="mac${sheet ? ' open' : ''}">${side}${chat}${sheet ? sheetView() : ''}</div>`;
} else {
  document.body.innerHTML = chat + (sheet ? `<div class="dim"></div>${sheetView()}` : '');
}
document.body.dataset.ready = '1';
