// #7 mockup: apply a theme and a colour scheme as cooee's head script will. `?theme=farm|kitchen|garden|terminal`,
// `?mode=system|light|dark` (the Appearance pick; system follows prefers-color-scheme live), `?dark=<scheme>` /
// `?light=<scheme>` (a COLOURS pick per mode; else the theme's default), `?style=modern|terminal` (default modern: only
// shapes, radii, type and buttons differ, modern.css) and `?sprites=on|off` (default on; off: a coloured dot for every
// sprite or glyph, the loading and empty figures hidden). Then it fills the page from the theme:
//   [data-st=working|needs|done|fail|idle]  a state label: the sprite (the glyph; sprites off, the dot), the theme word, `· plain word`
//   [data-s=<id>]                           a sprite: edit read run idle done needs merged fail (24 px), loading empty (132 × 100)
//   [data-w=<key>]                          a theme word (data-plain adds `· <plain>`)
const q = new URLSearchParams(location.search);
const mq = matchMedia('(prefers-color-scheme: dark)');
// Data sits next to this file, so pages in other folders (docs/design/pair, #47) can import it.
const at = f => new URL(f, import.meta.url);
export const T = q.get('theme') || 'farm', MODE = q.get('mode') || 'system';
export const STYLE = q.get('style') || 'modern', SPRITES = q.get('sprites') !== 'off';
export const theme = await fetch(at(`themes/${T}.json`)).then(r => r.json());
// Sprites off keeps the theme's words and moments and drops its art: `sprites` reads false for the page.
const art = theme.sprites && SPRITES;
export const NAMES = ['github-dark', 'solarized-dark', 'gruvbox', 'nord', 'dracula', 'farm', 'kitchen', 'garden-dark',
  'paper', 'farm-light', 'kitchen-light', 'garden-light'];
export const S = Object.fromEntries(await Promise.all(NAMES.map(async n => [n, await fetch(at(`schemes/${n}.json`)).then(r => r.json())])));
export const mode = () => MODE === 'system' ? (mq.matches ? 'dark' : 'light') : MODE;
export const pick = m => q.get(m) || theme.scheme[m];
// The plain words: what each state says in every theme, and all terminal says.
export const PLAIN = { working: 'working', needs: 'needs you', done: 'done', fail: 'failed', idle: 'idle', merged: 'done',
  loading: 'loading', 'empty-threads': 'no threads' };
const GLYPH = { working: '✻', needs: '!', done: '✓', fail: '✕', idle: '·' };
const word = k => (theme.words || {})[k] || PLAIN[k];

function paint() {
  const s = S[pick(mode())], r = document.documentElement;
  for (const [k, v] of Object.entries(s.tokens)) r.style.setProperty(k, v);
  r.style.colorScheme = s['color-scheme'];
  r.dataset.theme = T;
  r.dataset.mode = mode();
  r.dataset.style = STYLE;
  r.dataset.sprites = SPRITES ? 'on' : 'off';
  document.querySelector('meta[name=theme-color]')?.setAttribute('content', s.tokens['--bg']);
}
paint();
mq.addEventListener('change', paint);

// The 24 px sprites follow the agent's current tool: a label may name it (data-tool=edit|read|run), else working is edit.
export const sprite = (id, cls = 'rsp') => !SPRITES && cls === 'rsp' ? '<i class=sd></i>' : art
  ? `<svg class=${cls} viewBox="${/^(loading|empty)$/.test(id) ? '0 0 132 100' : '0 0 24 24'}" aria-hidden=true><use href="#s-${id}"/></svg>` : '';
if (art) {
  // The pack's own sheet, as the build vendors it at swag's pin (packs/swag.toml): serve the repo root.
  const svg = await fetch(at(`../../../packs/themes/${T}/sprites.svg`)).then(r => r.text());
  document.body.insertAdjacentHTML('afterbegin', `<svg width=0 height=0 style="position:absolute" aria-hidden=true><defs>${svg.replace(/^[\s\S]*?<svg[^>]*>|<\/svg>\s*$/g, '')}</defs></svg>`);
}
export function fill(root = document) {
  for (const e of root.querySelectorAll('[data-st]')) {
    const st = e.dataset.st, tool = e.dataset.tool || 'edit', id = st === 'working' ? tool : st;
    const mark = art || !SPRITES ? sprite(id) : `<i class=sg>${GLYPH[st]}</i> `;
    const w = word(st), plain = w !== PLAIN[st] ? `<small class=pw> · ${PLAIN[st]}</small>` : '';
    e.innerHTML = `${mark}<span class=thw>${w}</span>${plain}${e.dataset.more || ''}`;
  }
  for (const e of root.querySelectorAll('[data-s]')) e.innerHTML = sprite(e.dataset.s, e.dataset.cls || 'rsp');
  for (const e of root.querySelectorAll('[data-w]')) {
    const k = e.dataset.w, w = word(k);
    e.innerHTML = w + ('plain' in e.dataset && w !== PLAIN[k] ? `<small class=pw> · ${PLAIN[k]}</small>` : '');
  }
  const has = k => k === 'sprites' ? art : theme[k];
  for (const e of root.querySelectorAll('[data-if]')) e.hidden = !has(e.dataset.if);
  for (const e of root.querySelectorAll('[data-ifnot]')) e.hidden = !!has(e.dataset.ifnot);
}
fill();
document.body.dataset.ready = 1;
