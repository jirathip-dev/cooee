const SETS = [
  ['modern', 'Modern', 'Style Modern', [['laughing', 'Laughing'], ['perched', 'Perched'], ['glance', 'Side glance'], ['wings', 'Wings out']]],
  ['terminal', 'Terminal', 'Style Terminal', [['prompt', 'Prompt'], ['dollar', 'Dollar'], ['chevron', 'Chevron'], ['lambda', 'Lambda']]],
  ['farm', 'Farm', 'Scene · farm', [['bay', 'Bay'], ['palomino', 'Palomino'], ['black', 'Black'], ['grey', 'Grey']]],
  ['garden', 'Garden', 'Scene · garden', [['monstera', 'Monstera'], ['sunflower', 'Sunflower'], ['cactus', 'Cactus'], ['tulips', 'Tulips']]],
  ['kitchen', 'Kitchen', 'Scene · kitchen', [['head-chef', 'Head chef'], ['pastry', 'Pastry chef'], ['line-cook', 'Line cook'], ['sous', 'Sous chef']]],
  ['bush', 'Bush', 'Scene · bush (#22)', [['roo', 'Roo'], ['wombat', 'Wombat'], ['koala', 'Koala'], ['emu', 'Emu']]],
];
const src = (set, name) => `icons/${set}-${name}.${set === 'farm' ? 'png' : 'svg'}`;
const img = (set, name, s) => `<img class="ic" src="${src(set, name)}" alt=""${s ? ` style="--s:${s}px"` : ''}>`;
const q = new URLSearchParams(location.search), f = q.get('f') || (location.pathname.endsWith('picker.html') ? 'picker' : 'sheet'), mode = q.get('mode') || 'dark';
document.documentElement.className = mode;

const status = `<div class="sb"><span>9:41</span><i>●●● ▲ ▮</i></div>`;
const phone = (body, nav, title) => `<div class="ph">${status}${nav ? `<div class="nv"><b>${nav}</b></div>` : ''}${title ? `<div class="lt">${title}</div>` : ''}<div class="pg">${body}</div><div class="hi"></div></div>`;
const group = (header, body) => `<div class="gp">${header ? `<div class="gh">${header}</div>` : ''}<div class="gb">${body}</div></div>`;
const pick = (title, on) => `<div class="rw">${title}${on ? '<span class="ck">✓</span>' : ''}</div>`;

const wallpaper = (kind, title) => `<div class="wp ${kind}"><h3>${title}</h3>${SETS.map(([set, label, , icons]) =>
  `<div class="hs"><div class="st">${label}</div>${icons.map(([n, name]) => `<div class="ap">${img(set, n)}<span>${name}</span></div>`).join('')}</div>`).join('')}</div>`;

function picker(picked) {
  const follow = `<div class="rw two">${img('modern', 'laughing', 44)}<div class="tx">Follow the look<small>${picked ? "Off · would show Laughing · Modern" : "Now: Laughing · Modern"}</small></div>${picked ? '' : '<span class="ck">✓</span>'}</div>`;
  const sets = SETS.map(([set, label, , icons]) => group(label, `<div class="tg4">${icons.map(([n, name]) => {
    const cls = picked ? (picked[0] === `${set}-${n}` ? ' on' : '') : (set === 'modern' && n === 'laughing' ? ' now' : '');
    return `<div class="tl${cls}"><div class="fr">${img(set, n, 64)}</div><span>${name}</span></div>`;
  }).join('')}</div>`)).join('');
  return phone(group('', follow) + sets, 'Look', 'App icon');
}

function look() {
  const schemes = ['System', 'Light', 'Dark'].map((s, i) => pick(s, i === 0)).join('');
  const styles = ['Modern', 'Terminal', 'Scene'].map((s, i) => pick(s, i === 0)).join('');
  const icon = `<div class="rw">${img('modern', 'laughing', 30)}App icon<span class="v">Follows the look</span><span class="cv">›</span></div>`;
  return phone(group('Scheme', schemes) + group('Style', styles) + group('App icon', icon), 'Settings', 'Look');
}

const MAP = [['A fresh install', 'modern', 'laughing', 'Laughing kookaburra, cooee\'s own mascot'], ['Style Modern', 'modern', 'laughing', 'Laughing kookaburra'],
  ['Style Terminal', 'terminal', 'prompt', 'Prompt'], ['Style Scene · farm', 'farm', 'bay', 'Bay'], ['Style Scene · garden', 'garden', 'monstera', 'Monstera'],
  ['Style Scene · kitchen', 'kitchen', 'head-chef', 'Head chef'], ['Style Scene · bush', 'bush', 'roo', 'Roo (once #22 ships the Bush theme)']];
const mapping = `<div class="map">${MAP.map(([when, set, n, what]) => `<b>${when}</b>${img(set, n)}<span>${what}</span>`).join('')}</div>`;

const FRAMES = {
  sheet: () => `<div class="ph">${status}<div class="lt">App icons</div><div class="pg">${wallpaper('d', 'Dark wallpaper')}${wallpaper('l', 'Light wallpaper')}</div></div>`,
  'wide-sheet': () => `<div class="bd"><h1>cooee app icons · 24 in 6 sets of 4</h1><p>At home-screen size (60 pt), on a dark and a light wallpaper. Flat vector in Corral's look: a head-and-shoulders portrait, soft shading, one solid dark background per set. Farm is Corral's four horses, Bay and Black moved onto Farm's dark grounds. Following the look, the icon is a set's first:</p>
    <div class="cols" style="grid-template-columns:400px 1fr 1fr"><div><p class="cap">Follow the look</p>${mapping}</div>${wallpaper('d', 'Dark wallpaper')}${wallpaper('l', 'Light wallpaper')}</div></div>`,
  look, picker: () => picker(null), picked: () => picker(['bush-koala', 'Koala']),
  'wide-picker': () => {
    const at = (m, fr, cap) => `<div class="${m}"><p class="cap" style="color:${m === 'dark' ? '#a3a68a' : '#5c573f'}">${cap}</p>${FRAMES[fr]()}</div>`;
    return `<div class="bd"><h1>Look → App icon</h1><p>The App icon row on Look opens the picker: Follow the look, then all 24 by set. A picked icon stays until Follow the look is chosen again.</p>
      <div class="cols" style="grid-template-columns:repeat(3,390px);justify-content:space-between">${at('dark', 'look', 'Look, with the new App icon row (dark)')}${at('dark', 'picker', 'Following the look (dark)')}${at('light', 'picked', 'Koala picked (light)')}</div></div>`;
  },
};
document.body.innerHTML = FRAMES[f]();
await Promise.all([...document.images].map(i => i.complete ? 0 : new Promise(ok => { i.onload = i.onerror = ok; })));
document.body.dataset.ready = '1';
