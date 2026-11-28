// #793 mockup: draws the fixtures/*.json blocks screens the way blocks' own renderer lays them out (row: title, badge,
// subtitle, value, chevron; section: header, footer; stat tiles; buttons; a confirm). Times count from NOW.
const NOW = Date.parse('2026-11-23T14:00:00Z');
const q = new URLSearchParams(location.search);
const h = (t, c, ...k) => { const e = document.createElement(t); if (c) e.className = c.split(' ').filter(Boolean).map(x => 's-' + x).join(' '); e.append(...k.filter(x => x != null)); return e; };
const span = m => { const d = Math.floor(m / 1440), hr = Math.floor(m % 1440 / 60), mn = m % 60; return d ? `${d}d ${hr}h` : hr ? `${hr}h ${mn}m` : `${mn} min`; };
const shown = v => {
  if (v == null || typeof v !== 'object') return v == null ? '' : String(v);
  if (v.until) return (v.label || '{rel}').replace('{rel}', span(Math.round((Date.parse(v.until) - NOW) / 60000)));
  if (v.since) return (v.label || '{}').replace('{}', span(Math.round((NOW - Date.parse(v.since)) / 60000)));
  return v.label || '';
};
const toned = v => v == null ? null : typeof v === 'object' && 'text' in v ? { text: shown(v.text), tone: v.tone || '' } : { text: shown(v), tone: '' };

function row(b, sel) {
  const sub = toned(b.subtitle), badge = toned(b.badge);
  const main = h('span', 'rm', h('span', 'rt', b.title || ''), badge && h('span', `tag ${badge.tone}`, badge.text));
  const r = h('div', `row${b.tap ? ' tap' : ''}${b.id === sel ? ' sel' : ''}`, h('span', 'rc', main, sub && h('span', `rs ${sub.tone}`, sub.text)),
    b.value != null ? h('span', 'rv', shown(b.value)) : null, b.accessory === 'chevron' ? h('span', 'chev', '›') : null);
  return r;
}
function block(b, sel) {
  switch (b.type) {
    case 'section': return h('section', 'sec', b.header ? h('h3', 'sh', b.header) : null, ...b.blocks.map(x => block(x, sel)), b.footer ? h('p', 'sf', b.footer) : null);
    case 'list': return h('div', 'lst', ...b.rows.map(r => row(r, sel)));
    case 'stat': return h('div', 'stat', ...b.items.map(t => h('div', `tile ${t.tone || ''}`, h('span', 'tl', t.label), h('b', 'tv', shown(t.value)))));
    case 'button': return h('button', `btn ${b.style || ''}`, b.label);
    default: return h('div', '', `[${b.type}]`);
  }
}
const find = (b, id) => b.id === id ? b : (b.blocks || b.rows || []).map(x => find(x, id)).find(Boolean);
function screen(s, sel, back = true) {
  const bar = h('header', 'nb', back && s.back ? h('span', 'bk', `‹ ${s.back}`) : h('span', 'bk'), h('h1', '', s.title), h('span', 'bk'));
  return h('div', 'scr', bar, h('div', 'body', ...s.blocks.map(b => block(b, sel))));
}
function confirm(s, id) {
  const c = find(s, id).confirm;
  return h('div', 'scrim', h('div', 'cf', h('div', 'cfx', h('b', '', c.title), c.message ? h('p', '', c.message) : null),
    h('button', `cfo${c.destructive ? ' bad' : ''}`, c.ok || 'OK'), h('button', 'cfc', 'Cancel')));
}
const load = n => fetch(`fixtures/${n}.json`).then(r => r.json());
(async () => {
  const main = await load(q.get('f') || 'signins'), side = q.get('side') && await load(q.get('side'));
  const one = h('div', `one${side ? ' split' : ''}`);
  if (side) {
    one.append(h('aside', 'nav', h('h2', '', 'Settings'), ...['Devices', 'Sign-ins', 'Machines', 'Plugins'].map(t => h('div', `ni${t === 'Sign-ins' ? ' on' : ''}`, t))),
      screen(main, q.get('sel'), false), screen(side, null, false));
  } else one.append(screen(main, null));
  if (q.get('confirm')) one.append(confirm(side || main, q.get('confirm')));
  document.body.append(one);
  document.body.dataset.ready = '1';
})();
