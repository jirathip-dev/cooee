// #251 Scene: herdsman's Herd strip (jirathip-dev/herdsman assets/herd.js + herd.css, #37/#71: Corral's ranch, MIT ©
// jirathip-dev), copied as the owner asked (q1323): the same seeded hills, barn, trees, fence, grass, clouds, night sky
// and its day/night colour pairs. Only the horses' places are cooee's: working on the trail, waiting at the gate, done in
// the stable, idle grazing. Weather = the paired Mac's load: calm, busy (gusty, hazy), held (storm, rain).
const M64 = (1n << 64n) - 1n;
const noise = (i, s = 0) => { const n = (BigInt(i) * 1664525n + BigInt(s) * 1013904223n) & M64; return Number(((n ^ (n >> 13n)) * 1274126177n) % 65536n) / 65535; };
const f1 = n => +n.toFixed(1);
const ell = (x, y, rx, ry, fill, op = 1) => `<ellipse cx="${f1(x)}" cy="${f1(y)}" rx="${f1(rx)}" ry="${f1(ry)}" fill="${fill}"${op < 1 ? ` opacity="${op}"` : ''}/>`;
const rect = (x, y, w, h, fill, op = 1) => `<rect x="${f1(x)}" y="${f1(y)}" width="${f1(w)}" height="${f1(h)}" fill="${fill}"${op < 1 ? ` opacity="${op}"` : ''}/>`;

// herd.css's #herd and #herd.night custom properties (Corral's RanchPainter color(day, dark) pairs).
const DAY = '--sky1:#388fc4;--sky2:#8ebbc6;--h0:#a9c9bf;--h1:#9fb9a5;--h2:#8aab88;--h3:#79966d;--lw:#d2c48c;--sh:#334c2e;--st:#e8e3b3;--bn:#a86e52;--rf:#536456;--dr:#344635;--wn:#e5dfb8;--tk:#776745;--l1:#859b5c;--l2:#66874f;--lh:#b0b67b;--fp:#9c916a;--fh:#e2d7ac;--fr:#8b825d;--fl:#d6c99f;--cl:#e3f1e9;--gs:#aeb57a;--gd1:#bcc17b;--gd2:#8e9c58;--gd3:#485e3b';
const NIGHT = '--sky1:#121a32;--sky2:#3c4961;--h0:#617484;--h1:#586c78;--h2:#4c666a;--h3:#425b5a;--lw:#789085;--sh:#172e32;--st:#b9c3b1;--bn:#705b58;--rf:#344b50;--dr:#182d35;--wn:#ceb581;--tk:#3c4c4a;--l1:#557068;--l2:#405e59;--lh:#96aea0;--fp:#73847f;--fh:#d0dbd4;--fr:#6b7d75;--fl:#c4d6e0;--cl:#a6b6c6;--gs:#90a397;--gd1:#63746a;--gd2:#415d55;--gd3:#273f3e';
const STORM = '--sky1:#1d2530;--sky2:#4f5a63;--cl:#8e979e';
const BUSY = '--sky1:#5d8fb0;--sky2:#a9bcc0;--cl:#d3dad8';

function clouds(n) {
  let s = '';
  for (const copy of [0, 780]) for (let i = 0; i < n; i++) {
    const x = i * (780 / n) + 45 + copy, y = 20 + noise(i, 30) * 60;
    for (let j = 0; j < 5; j++) s += ell(x + j * 12, y - noise(j, i) * 9, 18, 5 + noise(j, 2) * 5, 'var(--cl)');
  }
  return `<div class=hcl><svg viewBox="0 0 1560 90" preserveAspectRatio=none aria-hidden=true>${s}</svg></div>`;
}
function sky() {
  let s = '';
  for (let i = 0; i < 130; i++) {
    const r = 0.35 + noise(i, 7) * 0.7;
    s += ell(i * 3 + noise(i, 8) * 3, noise(i, 3) * 225, r, r, '#e7e5f4', f1((0.3 + 0.45 * noise(i, 9) + 0.08 * Math.sin(i)) * 100) / 100);
  }
  return `<g>${s}</g>${ell(232, 106, 13, 13, '#d3dfeb', 0.84)}${ell(228, 102, 12, 12, '#1d2841')}`;
}
function scene(night) {
  let s = night ? sky() : '';
  ['var(--h0)', 'var(--h1)', 'var(--h2)', 'var(--h3)'].forEach((fill, r) => {
    let d = 'M-85 330';
    for (let i = -4; i <= 14; i++) d += `L${i * 35} ${f1(147 + r * 30 + noise(i, r + 17) * 36)}`;
    s += `<path d="${d}L475 330Z" fill="${fill}"/>`;
  });
  s += '<path d="M-60 258Q165 228 450 268L450 330L-60 330Z" fill="url(#hgr)"/>';
  for (let i = -152; i <= 1650; i++) {
    const y = 270 + noise(i, 19) * 370;
    if (y < 330) s += ell(Math.floor(i / 19) * 5 + noise(i, 8) * 5, y, 0.3 + noise(i, 2) * 0.7, 0.3, 'var(--st)', 0.13);
  }
  s += '<path d="M153 243L180 219L210 242L210 279L153 279Z" fill="var(--bn)"/><path d="M148 245L180 215L216 243L209 246L180 224L155 248Z" fill="var(--rf)"/>';
  s += rect(175, 252, 14, 27, 'var(--dr)') + rect(175, 235, 9, 9, 'var(--wn)', night ? 1 : 0.8);
  for (const [x, ty, sc] of [[7, 225, 1], [365, 228, 1.1], [238, 258, 0.5], [326, 262, 0.38]]) {
    s += rect(x - 2 * sc, ty - 8 * sc, 4 * sc, 45 * sc, 'var(--tk)');
    for (let i = 0; i < 22; i++) {
      const dx = (noise(i, 20) - 0.5) * 48 * sc, dy = (noise(i, 7) - 0.5) * 35 * sc;
      s += ell(x + dx, ty + dy, 9 * sc, 11 * sc, i % 3 === 0 ? 'var(--l1)' : 'var(--l2)', 0.9) + ell(x + dx - 2 * sc, ty + dy - 3 * sc, 5 * sc, 4 * sc, 'var(--lh)', 0.1);
    }
  }
  // The fence, with a gap at the right (x 318–372) where the gate hangs.
  for (let i = -2; i <= 9; i++) if (i * 52 < 318 || i * 52 > 372) s += rect(i * 52, 249, 6, 44, 'var(--fp)') + rect(i * 52 + 1, 249, 1.4, 44, 'var(--fh)', 0.5);
  for (const dy of [4, 22]) s += rect(-60, 256 + dy, 372, 5, 'var(--fr)') + rect(-60, 256 + dy, 372, 1.3, 'var(--fl)', 0.8);
  s += rect(312, 245, 7, 50, 'var(--fp)') + rect(372, 245, 7, 50, 'var(--fp)');
  for (const dy of [256, 268, 280]) s += rect(320, dy, 51, 4, 'var(--fr)');
  s += `<path d="M320 284L370 258" stroke="var(--fr)" stroke-width="4"/>`;
  for (let i = -5; i <= 48; i++) {
    const x = i * 9, y = 298 + noise(i, 43) * 24;
    let b = '';
    for (let k = 0; k < 3; k++) b += `M${x + 2 * k} ${f1(y)}Q${x + 2 * k - 1} ${f1(y - 5)} ${x + 2 * k - 3} ${f1(y - 8 - noise(i, 4) * 4)}`;
    s += `<path d="${b}" fill="none" stroke="var(--gs)" stroke-opacity=".45" stroke-width=".8"/>`;
  }
  return `<svg class=rs viewBox="0 0 390 320" preserveAspectRatio="xMidYMax slice" aria-hidden=true><defs><linearGradient id=hgr gradientUnits=userSpaceOnUse x1=0 y1=250 x2=0 y2=640><stop offset=0 stop-color="var(--gd1)"/><stop offset=".5" stop-color="var(--gd2)"/><stop offset=1 stop-color="var(--gd3)"/></linearGradient></defs>${s}</svg>`;
}

const SPR = '../../../apple/Sources/CooeeUI/Sprites/';
// A horse: {coat, pose, x, y (px from the strip's bottom), w, name, word, k (wk|nd|dn|id), left, badge, run}.
export const horse = h => `<a class="hh2 ${h.k || ''}${h.left ? ' l' : ''}${h.ghost ? ' gh' : ''}" href="#" style="left:${h.x}px;bottom:${h.y}px;--w:${h.w || 66}px${h.run ? `;--run:${h.run}px` : ''}">
  <img src="${SPR}horse-${h.coat}-${h.pose}.png" alt="">${h.name ? `<span class=hnt>${h.name}${h.word ? `<i${h.k === 'nd' ? ' class=r' : ''}>${h.word}</i>` : ''}</span>` : ''}${h.badge ? `<i class=bd>${h.badge}</i>` : ''}</a>`;

// The strip: {night, weather: calm|busy|held, horses, h (px tall), inner (the search field, a label)}.
export function ranch({ night = false, weather = 'calm', horses = [], h = 330, inner = '' } = {}) {
  const vars = [DAY, night ? NIGHT : '', weather === 'held' ? STORM : weather === 'busy' && !night ? BUSY : ''].join(';');
  const cls = ['hd2', night ? 'night' : '', weather].join(' ');
  return `<div class="${cls}" style="${vars};--hh:${h}px">${scene(night)}${clouds(weather === 'calm' ? 4 : 9)}<i class=trail></i><i class=fog></i>${horses.map(horse).join('')}<i class=rain></i>${inner}</div>`;
}
