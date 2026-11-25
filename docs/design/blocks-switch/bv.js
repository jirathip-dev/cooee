// #716 design: a mockup of blocks' SwiftUI kinds (BlocksUI Kind/*.swift at blocks staging d79ce47) drawing #714's
// screens (fixtures/*.json), one function per kind, and the native chrome cooee keeps around them. Not a client.
// `_n` in a fixture is a design annotation: the number of the difference it shows. It is no part of the schema.
const esc = s => String(s ?? '').replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);
const TONE = { ok: 'g', done: 'g', warn: 'a', needs: 'a', bad: 'r', fail: 'r', info: 'link', working: 'c', accent: 'lead', mute: 'mute' };
const tone = t => TONE[t] ? `var(--${TONE[t]})` : null;
let MARKS = true;
const mark = b => MARKS && b._n ? `<i class="n at">${b._n}</i>` : '';
const wrap = (b, html) => b._n && MARKS ? `<div class="mk${b.type === 'code' ? ' code' : ''}">${html}${mark(b)}</div>` : html;

const KINDS = {
  screen: b => stack(b.blocks),
  section: b => wrap(b, `<div class="sec${b.id === 'log' ? ' log' : ''}">${b.header ? `<h4>${esc(b.header)}</h4>` : ''}${stack(b.blocks)}${b.footer ? `<div class="foot">${esc(b.footer)}</div>` : ''}</div>`),
  row: b => {
    const sub = typeof b.subtitle === 'string' ? { text: b.subtitle } : b.subtitle;
    const pill = b.badge ? `<span class="pill" style="color:${tone(b.badge.tone) ?? 'var(--mute)'};background:color-mix(in srgb,${tone(b.badge.tone) ?? 'var(--mute)'} 15%,transparent)">${esc(b.badge.text)}</span>` : '';
    return wrap(b, `<div class="row${b.style === 'link' ? ' link' : ''}${b._sel ? ' sel' : ''}"><div class="tx"><b>${esc(b.title)}</b>${sub ? `<span style="color:${tone(sub.tone) ?? 'var(--mute)'}">${esc(sub.text)}</span>` : ''}</div>${pill}${b.value ? `<span class="val">${esc(b.value)}</span>` : ''}${b.tap ? '<span class="chev">›</span>' : ''}</div>`);
  },
  message: (b, prev) => {
    const owner = b.role === 'owner', cont = prev?.type === 'message' && prev.name === b.name && prev.role === b.role;
    const head = cont ? '' : `<div class="hd"><b>${esc(b.name)}</b><span>${esc(b.role)}</span>${b.time ? `<span>${esc(b.time)}</span>` : ''}</div>`;
    const body = stack(b.blocks);
    const files = (b.files ?? []).map(f => {
      const dot = f.name.lastIndexOf('.');
      return wrap(f, `<div class="file"><b>${esc(f.name.slice(dot + 1))}</b><span>${esc(f.name.slice(0, dot))}</span></div>`);
    }).join('');
    return wrap(b, `<div class="msg ${owner ? 'owner' : ''} ${cont ? 'cont' : ''}"><div class="ava">${esc(b.name[0].toUpperCase())}</div><div class="col">${head}${owner ? `<div class="own">${body}</div>` : body}${files}</div></div>`);
  },
  prose: b => {
    const spans = ss => ss.map(s => { let t = esc(s.text); if (s.code) t = `<code class="in">${t}</code>`; if (s.bold) t = `<b>${t}</b>`; if (s.italic) t = `<i>${t}</i>`; return t; }).join('');
    const caret = b.streaming ? '<span class="caret">▍</span>' : '';
    const parts = b.parts.map((p, i) => {
      const c = i === b.parts.length - 1 ? caret : '';
      const html = p.type === 'code' ? `<pre>${esc(p.text)}${c}<i class="lang">${esc(p.lang)}</i></pre>` : `<p>${spans(p.spans)}${c}</p>`;
      return wrap(p, html);
    }).join('');
    return wrap(b, `<div class="prose">${parts}</div>`);
  },
  tool: b => {
    const glyph = { running: '↻', failed: '✕' }[b.state] ?? '✓';
    return wrap(b, `<div class="tool ${b.state}"><span class="gl" style="flex:none">${glyph}</span><b>${esc(b.name)}</b><span>${esc(b.summary)}</span>${b.body ? '<span class="chev" style="flex:none">›</span>' : ''}</div>`);
  },
  composer: b => {
    const work = b.working ? `<div class="work"><i></i>working 42s<span class="sp"></span>${b.acts?.stop ? '<a>stop</a>' : ''}</div>` : '';
    const q = (b.queued ?? []).map(x => `<div class="qd"><span>${esc(x.text)}</span><span class="now">send now</span><span>edit</span><span class="rm">remove</span></div>`).join('');
    const label = b.working && b.acts?.queue ? 'queue' : b.label ?? 'send';
    const chips = (b.chips ?? []).map(c => `<span>${esc(c.name)} ${esc(c.cur)}</span>`).join('');
    return wrap(b, `<div class="comp">${work}${q}<div class="in"><div class="fld">${esc(b.placeholder)}</div><div class="btn pri">${label}</div></div><div class="chips">${chips}</div></div>`);
  },
  form: b => `<div class="form">${b.fields.map(f => `<div class="fr"><span>${esc(f.label)}</span><b>${esc(f.placeholder)}</b></div>`).join('')}<div class="btn pri">${esc(b.submit)}</div></div>`,
  button: b => wrap(b, `<div class="btn${b.style === 'primary' ? ' pri' : b.style === 'destructive' ? ' bad' : ''}">${esc(b.label)}</div>`),
  stat: b => wrap(b, `<div class="stat"><span>${esc(b.label)}</span><b class="mono">${esc(b.value)}</b></div>`),
  island: b => b.name === 'device-key' ? wrap(b, `<div class="isl">Sign in with this device's key</div>`) : `<div class="row"><div class="tx"><b>${esc(b.fallback.title)}</b><span>${esc(b.fallback.subtitle)}</span></div></div>`,
};
function stack(blocks = []) { return `<div class="stack">${blocks.map((b, i) => KINDS[b.type](b, blocks[i - 1])).join('')}</div>`; }
const draw = (screen, marks = true) => { MARKS = marks; const html = KINDS.screen(screen); MARKS = true; return html; };

// The chrome cooee keeps native: the status bar, its own bar (back, the screen's title, ⋯ for the screen's actions).
const n = (k, on = true) => on && k ? `<i class="n at">${k}</i>` : '';
const status = '<div class="sb"><span>9:41</span><i></i><span>●●● ⌔ ▬</span></div>';
const phone = (bar, body, cls = '') => `<div class="ph">${status}${bar}<div class="body ${cls}">${body}</div><div class="home"></div></div>`;
const chatBar = (title, sub, k) => `<div class="bar mk"><div class="rb">‹</div><div class="t"><b>${esc(title)}</b>${sub ? `<span>${esc(sub)}</span>` : ''}</div><div class="rb">⋯</div>${n(k)}</div>`;
const homeBar = k => `<div class="bar mk" style="border:0"><div class="av">G</div><div class="t"><b style="font-size:22px">cooee</b></div><div style="width:44px"></div>${n(k)}</div>`;
const macBar = (title, sub, k) => `<div class="bar mk"><div class="t" style="text-align:left"><b>${esc(title)}</b>${sub ? `<span>${esc(sub)}</span>` : ''}</div><div class="rb">⋯</div>${n(k)}</div>`;
const lights = '<div class="lights"><i></i><i></i><i></i></div>';
const mac = (side, detailBar, detail) => `<div class="mw"><div class="side">${lights}<div class="top"><div class="av">G</div><b>cooee</b><div class="sp"></div></div><div class="body">${side}</div></div><div class="main">${detailBar}<div class="body">${detail}</div></div></div>`;
const macSmall = (title, body) => `<div class="mw small">${lights}<div class="bar" style="border:0;justify-content:center"><b style="font-size:17px">${esc(title)}</b></div><div class="body">${body}</div></div>`;

const load = async names => Object.fromEntries(await Promise.all(names.map(async x => [x, await (await fetch(`fixtures/${x}.json`)).json()])));
const diffs = list => `<ol class="diffs">${list.map(([k, t, s, gap]) => `<li${gap ? ' class="gap"' : ''}><i class="n">${k}</i><div><b>${t}</b>${s ? `<small>${s}</small>` : ''}</div></li>`).join('')}</ol>`;
