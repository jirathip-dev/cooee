// #827 agent characters as data: every figure is one flat body in one scheme token, two pill eyes and at most two
// small features, drawn in a 100 × 100 box. Colours are only scheme tokens (`--lead`, `--link`, …) or mixes of two
// tokens (`shade` toward the scheme's ink, `tint` toward its paper), never a hand-picked hex.
(() => {
  const T = n => `var(--${n})`;
  const mix = (a, b, p) => `color-mix(in srgb, ${a} ${p}%, ${b})`;
  const shade = (c, p = 62) => mix(c, T('ink'), p);
  const tint = (c, p = 55) => mix(c, T('paper'), p);
  const st = f => `style="fill:${f}"`;
  const E = (cx, cy, rx, ry, f, x = '') => `<ellipse cx="${cx}" cy="${cy}" rx="${rx}" ry="${ry}" ${st(f)} ${x}/>`;
  const C = (cx, cy, r, f) => E(cx, cy, r, r, f);
  const P = (d, f, x = '') => `<path d="${d}" ${st(f)} ${x}/>`;
  const R = (x, y, w, h, r, f, tf = '') => `<rect x="${x}" y="${y}" width="${w}" height="${h}" rx="${r}" ${st(f)}${tf ? ` transform="${tf}"` : ''}/>`;
  const S = (d, c, w) => `<path d="${d}" fill="none" stroke="${c}" stroke-width="${w}" stroke-linecap="round" stroke-linejoin="round"/>`;
  const PINK = mix(T('red'), T('fg'), 58), PERI = mix(T('link'), T('purple'), 50), BROWN = mix(T('amber'), T('ink'), 70);
  const GREY = mix(T('mute'), T('amber'), 82);

  const body = (c, cx = 52, cy = 58, rx = 32, ry = 30) => E(cx, cy, rx, ry, c, 'class="sk"');
  const feet = (c, y = 88, a = 42, b = 58) => R(a - 4, y - 2, 8, 5, 2.5, shade(c, 55)) + R(b - 4, y - 2, 8, 5, 2.5, shade(c, 55));
  const beak = (f, x = 24, y = 56, l = 14, h = 6) => P(`M${x} ${y - h} Q${x - l * 0.3} ${y - h * 0.6} ${x - l} ${y} Q${x - l * 0.3} ${y + h * 0.6} ${x} ${y + h}Z`, f);
  const eyes = (x1, x2, y, h = 13, w = 6.6) => ({ x: [x1, x2], y, h, w });

  const birds = [
    { id: 'kookaburra', name: 'Kookaburra', c: T('amber'), why: 'the logo’s own bird',
      draw: c => feet(c) + body(c) + E(50, 71, 20, 14, T('lead')) +
        P('M22 48 Q28 26 52 27 Q78 28 84 48 Q70 39 52 39 Q34 39 22 48Z', shade(c, 50)) +
        P('M27 51 L2 57 Q14 58 27 56Z', shade(c, 30)) + P('M27 56 Q14 58 4 58 Q14 62 27 63Z', T('lead')),
      eyes: eyes(40, 55, 48) },
    { id: 'galah', name: 'Galah', c: PINK, why: 'pink with a pale cap',
      draw: c => feet(c) + body(c) + P('M30 40 Q40 24 58 26 Q74 28 80 40 Q66 34 52 34 Q40 34 30 40Z', tint(T('fg'), 75), 'class="sk"') +
        P('M50 27 Q52 16 62 18 Q58 22 58 28Z', tint(T('fg'), 75)) + beak(tint(T('fg'), 80), 23, 57, 10, 5),
      eyes: eyes(40, 55, 47) },
    { id: 'rosella', name: 'Rosella', c: T('red'), why: 'red with blue cheeks',
      draw: c => R(72, 56, 10, 34, 5, shade(c, 60), 'rotate(-28 77 73)') + feet(c) + body(c) + E(38, 64, 10, 8, T('link')) +
        beak(T('lead'), 23, 56, 10, 5),
      eyes: eyes(40, 55, 47) },
    { id: 'magpie', name: 'Magpie', c: T('fg'), why: 'pied: a dark hood, light eyes', eye: T('fg'),
      draw: c => feet(T('mute')) + body(c) + P('M21 60 Q18 30 50 28 Q80 28 84 52 Q70 46 60 56 Q46 66 21 60Z', T('ink')) +
        P('M60 60 Q74 52 84 60 Q82 76 66 80 Q64 70 60 60Z', T('ink')) + beak(tint(T('mute'), 70), 23, 53, 12, 5),
      eyes: eyes(40, 55, 44) },
    { id: 'fairy-wren', name: 'Fairy-wren', c: T('link'), why: 'small, blue, tail up',
      draw: c => R(68, 18, 9, 40, 4.5, shade(c, 55), 'rotate(22 72 38)') + feet(c, 88, 44, 58) + body(c, 52, 62, 28, 26) +
        E(64, 52, 8, 7, tint(c, 45)) + beak(T('ink'), 26, 58, 9, 4),
      eyes: eyes(41, 54, 52, 11) },
    { id: 'cockatoo', name: 'Cockatoo', c: T('lead'), why: 'pale, with a yellow crest',
      draw: c => S('M50 32 Q46 12 62 6 M56 32 Q58 14 74 12 M62 34 Q68 20 82 22', T('amber'), 6) + feet(c) + body(c) +
        P('M27 50 Q14 52 16 62 Q20 58 27 62Z', shade(T('mute'), 45)),
      eyes: eyes(40, 55, 48) },
    { id: 'lorikeet', name: 'Lorikeet', c: T('green'), why: 'green, a blue head, an orange bib',
      draw: c => feet(c) + body(c) + P('M22 54 Q20 28 52 28 Q80 28 84 46 Q66 38 52 44 Q36 50 22 54Z', T('link')) +
        E(44, 72, 14, 10, T('amber')) + beak(T('red'), 24, 57, 10, 5),
      eyes: eyes(41, 55, 44) },
    { id: 'frogmouth', name: 'Frogmouth', c: GREY, why: 'grey, wide-mouthed, always sleepy',
      draw: c => feet(c, 90) + body(c, 52, 57, 30, 34) + S('M33 36 Q38 30 46 34 M52 34 Q60 30 66 36', shade(c, 55), 3.5) +
        P('M22 56 Q16 60 22 64 Q34 66 44 62 Q36 58 22 56Z', shade(c, 50)),
      eyes: eyes(39, 57, 44, 6.5, 7.5) },
  ];

  const blob = (n, a, rx, ry, rot = 0, cx = 50, cy = 56) => {
    const pts = [];
    for (let i = 0; i < 72; i++) {
      const t = (i / 72) * Math.PI * 2, r = 1 + a * Math.sin(n * t + rot);
      pts.push([cx + Math.cos(t) * rx * r, cy + Math.sin(t) * ry * r]);
    }
    const m = (p, q) => [(p[0] + q[0]) / 2, (p[1] + q[1]) / 2];
    let d = `M${m(pts[71], pts[0]).map(v => v.toFixed(1)).join(' ')}`;
    pts.forEach((p, i) => { const q = m(p, pts[(i + 1) % 72]); d += ` Q${p[0].toFixed(1)} ${p[1].toFixed(1)} ${q[0].toFixed(1)} ${q[1].toFixed(1)}`; });
    return d + 'Z';
  };
  const hint = (f, x = 48, y = 62) => P(`M${x - 4} ${y} Q${x} ${y - 1} ${x + 4} ${y} Q${x + 1} ${y + 5} ${x} ${y + 6} Q${x - 1} ${y + 5} ${x - 4} ${y}Z`, f);
  const blobs = [
    ['pip', 'Pip', T('amber'), 0, 0, 34, 31, 0, T('lead'), 'a pebble'],
    ['dew', 'Dew', T('cyan'), 1, 0.16, 30, 34, -1.6, T('lead'), 'a drop'],
    ['puff', 'Puff', T('lead'), 5, 0.07, 33, 30, 0, T('amber'), 'a puff, five lobes'],
    ['gum', 'Gum', PINK, 2, 0.09, 35, 28, 0.6, T('lead'), 'a bean'],
    ['nib', 'Nib', T('link'), 0, 0, 24, 36, 0, T('amber'), 'a tall pill'],
    ['moss', 'Moss', T('green'), 3, 0.08, 33, 31, 1.1, T('amber'), 'a soft triangle'],
    ['plum', 'Plum', T('purple'), 4, 0.05, 32, 32, 0.4, T('amber'), 'a soft square'],
    ['ember', 'Ember', T('red'), 2, 0.14, 30, 33, 2.2, T('lead'), 'a lopsided egg'],
  ].map(([id, name, c, n, a, rx, ry, rot, k, why]) => ({
    id, name, c, why,
    draw: c => P(blob(n, a, rx, ry, rot), c, 'class="sk"') + hint(k, 46, 60),
    eyes: eyes(39, 54, 49, 12),
  }));

  const farm = [
    { id: 'horse', name: 'Horse', c: T('amber'),
      draw: c => P('M30 34 L34 16 L42 30Z M60 30 L68 16 L72 34Z', shade(c, 70)) + feet(c) + body(c, 51, 58, 30, 31) +
        P('M38 30 Q50 22 64 30 Q56 36 50 46 Q46 36 38 30Z', shade(c, 50)) + E(51, 74, 17, 11, tint(c, 55)) +
        C(45, 74, 2, shade(c, 40)) + C(57, 74, 2, shade(c, 40)), eyes: eyes(42, 60, 52, 11) },
    { id: 'cow', name: 'Cow', c: T('fg'),
      draw: c => R(18, 38, 16, 8, 4, c, 'rotate(-15 26 42)') + R(68, 38, 16, 8, 4, c, 'rotate(15 76 42)') + feet(T('mute')) + body(c) +
        E(70, 46, 10, 9, T('ink')) + E(32, 70, 6, 5, T('ink')) + E(52, 74, 18, 10, PINK) + C(46, 74, 2, shade(PINK)) + C(58, 74, 2, shade(PINK)),
      eyes: eyes(43, 61, 52, 11) },
    { id: 'sheep', name: 'Sheep', c: T('lead'),
      draw: c => feet(T('ink')) + [[30, 44], [44, 34], [60, 34], [74, 44], [80, 60], [72, 76], [52, 82], [32, 76], [24, 60]].map(([x, y]) => C(x, y, 13, c)).join('') +
        body(c, 52, 58, 28, 26) + E(52, 60, 15, 18, shade(c, 25)), eyes: eyes(46, 58, 56, 10, 5.6), eye: T('lead') },
    { id: 'chook', name: 'Chook', c: T('lead'),
      draw: c => P('M40 30 Q42 18 48 26 Q52 16 56 26 Q62 18 62 32Z', T('red')) + feet(T('amber')) + body(c) +
        beak(T('amber'), 24, 54, 10, 5) + E(26, 64, 4, 6, T('red')) + P('M64 62 Q78 56 82 70 Q72 76 62 70Z', shade(c, 75)),
      eyes: eyes(40, 55, 46) },
  ];
  const kitchen = [
    { id: 'tomato', name: 'Tomato', c: T('red'),
      draw: c => body(c, 50, 58, 34, 30) + P('M50 30 L40 22 L48 28 L44 18 L52 27 L60 18 L56 28 L64 22Z', T('green')) + R(48, 18, 4, 10, 2, T('green')),
      eyes: eyes(41, 57, 54) },
    { id: 'toast', name: 'Toast', c: T('amber'),
      draw: c => P('M20 46 Q18 26 34 24 Q42 16 50 20 Q60 16 68 24 Q84 26 80 46 L80 84 Q80 88 76 88 L24 88 Q20 88 20 84Z', c, 'class="sk"') +
        P('M28 48 Q26 34 38 32 Q44 26 50 28 Q58 26 62 32 Q74 34 72 48 L72 80 L28 80Z', tint(T('lead'), 60)),
      eyes: eyes(42, 57, 50) },
    { id: 'teapot', name: 'Teapot', c: T('cyan'),
      draw: c => S('M78 46 Q92 50 86 66 Q82 72 76 72', c, 6) + P('M24 62 L8 46 Q6 44 10 44 L28 54Z', c) + body(c, 50, 62, 30, 26) +
        R(32, 34, 36, 8, 4, shade(c, 70)) + C(50, 31, 5, shade(c, 70)), eyes: eyes(42, 57, 58) },
    { id: 'dumpling', name: 'Dumpling', c: T('lead'),
      draw: c => P('M14 72 Q14 36 50 30 Q86 36 86 72 Q86 82 74 82 L26 82 Q14 82 14 72Z', c, 'class="sk"') +
        S('M36 36 Q40 42 42 34 M50 32 Q50 40 54 33 M62 36 Q62 42 66 37', shade(c, 60), 3), eyes: eyes(42, 58, 58) },
  ];
  const garden = [
    { id: 'sprout', name: 'Sprout', c: T('green'),
      draw: c => P('M50 32 Q34 14 20 22 Q30 36 50 34Z M50 32 Q66 12 82 20 Q72 36 50 34Z', tint(c, 70)) + R(48, 26, 4, 12, 2, shade(c)) +
        body(c, 50, 62, 28, 26), eyes: eyes(42, 56, 58) },
    { id: 'snail', name: 'Snail', c: T('amber'),
      draw: c => P('M12 86 Q10 66 26 62 L30 40 Q30 32 26 30 Q36 30 36 40 L38 62 L84 62 Q90 72 84 86Z', c, 'class="sk"') +
        C(58, 52, 22, T('lead')) + S('M58 52 m-4 0 a4 4 0 1 1 8 0 a9 9 0 1 1 -17 2 a14 14 0 1 1 27 -4', shade(T('lead'), 55), 3.5),
      eyes: eyes(24, 34, 70, 9, 5) },
    { id: 'bee', name: 'Bee', c: T('amber'),
      draw: c => E(38, 30, 12, 9, tint(T('fg'), 55), 'opacity=".85"') + E(64, 30, 12, 9, tint(T('fg'), 55), 'opacity=".85"') + body(c, 50, 58, 32, 28) +
        R(36, 32, 7, 52, 3.5, T('ink'), 'rotate(-12 39 58)') + R(58, 32, 7, 52, 3.5, T('ink'), 'rotate(-12 61 58)'),
      eyes: eyes(28, 50, 54, 10, 5.6), eye: T('ink') },
    { id: 'ladybird', name: 'Ladybird', c: T('red'),
      draw: c => body(c, 52, 60, 32, 28) + P('M26 46 Q30 30 48 30 Q38 38 36 54Z', T('ink')) + C(62, 46, 5, T('ink')) + C(74, 62, 5, T('ink')) +
        C(56, 74, 5, T('ink')), eyes: eyes(36, 46, 42, 8, 4.6), eye: T('fg') },
  ];
  const bush = [
    { id: 'roo', name: 'Roo', c: T('amber'),
      draw: c => R(30, 4, 10, 30, 5, c, 'rotate(-12 35 19)') + R(60, 4, 10, 30, 5, c, 'rotate(12 65 19)') + feet(c, 90, 38, 62) +
        body(c, 50, 60, 28, 30) + E(50, 74, 14, 10, tint(c, 50)) + C(50, 70, 3.5, T('ink')), eyes: eyes(42, 58, 52, 11) },
    { id: 'wombat', name: 'Wombat', c: BROWN,
      draw: c => C(28, 32, 8, c) + C(72, 32, 8, c) + feet(c, 90, 30, 70) + body(c, 50, 60, 36, 30) + E(50, 66, 11, 8, T('ink')),
      eyes: eyes(40, 60, 50, 10) },
    { id: 'koala', name: 'Koala', c: T('mute'),
      draw: c => C(22, 38, 16, c) + C(78, 38, 16, c) + C(22, 38, 9, tint(c, 55)) + C(78, 38, 9, tint(c, 55)) + body(c, 50, 60, 30, 28) +
        E(50, 66, 8, 11, T('ink')), eyes: eyes(38, 62, 54, 10) },
    { id: 'platypus', name: 'Platypus', c: T('cyan'),
      draw: c => R(68, 64, 26, 16, 8, shade(c, 70)) + feet(c) + body(c, 50, 58, 30, 28) + R(14, 60, 30, 13, 6.5, T('ink')),
      eyes: eyes(40, 56, 48) },
  ];

  const ascii = [
    { id: 'bit', name: 'bit', c: 'link', l: '[', r: ']', e: 'o', m: '_' },
    { id: 'kook', name: 'kook', c: 'amber', l: '(', r: ')', e: '°', m: 'v' },
    { id: 'moss', name: 'moss', c: 'green', l: '{', r: '}', e: '•', m: '.' },
    { id: 'zap', name: 'zap', c: 'cyan', l: '<', r: '>', e: '0', m: '_' },
    { id: 'owo', name: 'owo', c: 'purple', l: '|', r: '|', e: 'o', m: 'w' },
    { id: 'tin', name: 'tin', c: 'fg', l: '/', r: '\\', e: 'o', m: '-' },
    { id: 'dj', name: 'dj', c: 'lead', l: 'd', r: 'b', e: '·', m: '_' },
    { id: 'cat', name: 'cat', c: 'peri', l: '=', r: '=', e: 'o', m: '.' },
  ];
  const face = (a, e, m, l = a.l, r = a.r) => l + e + m + e + r;
  const asciiFrames = (a, state) => {
    const f = (e, m, ms, l, r) => [face(a, e, m, l, r), ms];
    switch (state) {
      case 'idle': return [f(a.e, a.m, 2200), f('-', a.m, 120), f(a.e, a.m, 900), f('-', a.m, 120), f(a.e, a.m, 1400), f('.', a.m, 700)];
      case 'working': return ['-', '\\', '|', '/', '-', '\\', '|', '/'].map((s, i) => f(i < 4 ? a.e : '°', s, 110));
      case 'done': return [f('^', a.m, 500), f('^', 'v', 260, '\\', '/'), f('^', a.m, 500), f('^', 'v', 260, '\\', '/'), f('^', a.m, 900)];
      case 'needs': return [f('O', 'o', 420, '!'), f('O', 'o', 420, a.l, '!'), f('O', 'o', 420), f('O', 'o', 420, '!')];
      default: return [f('x', a.m, 1800), f('x', '.', 1800)];
    }
  };

  const STATES = [
    ['idle', 'Idle', 'breathes; blinks every few seconds'],
    ['working', 'Thinking / working', 'tilts, eyes read side to side, three dots'],
    ['done', 'Replied / done', 'a little hop, happy eyes, a spark'],
    ['needs', 'Needs you', 'leans in and wiggles, eyes wide, an amber “!”'],
    ['failed', 'Failed', 'droops and greys out, x eyes, no loop'],
  ];

  let seq = 0;
  const svg = (ch, state = 'idle', phase = 0) => {
    const k = ch.eyes, id = `c${seq++}`;
    const ey = k.x.map(x => R(x - k.w / 2, k.y - k.h / 2, k.w, k.h, k.w / 2, 'var(--eye)')).join('');
    const hp = k.x.map(x => S(`M${x - 4} ${k.y + 2} Q${x} ${k.y - 5} ${x + 4} ${k.y + 2}`, 'var(--eye)', 3)).join('');
    const xx = k.x.map(x => S(`M${x - 3.5} ${k.y - 3.5} L${x + 3.5} ${k.y + 3.5} M${x + 3.5} ${k.y - 3.5} L${x - 3.5} ${k.y + 3.5}`, 'var(--eye)', 3)).join('');
    const fx = '<g class="dots">' + [0, 1, 2].map(i => C(76 + i * 8, 18, 3, T('fg'))).join('') + '</g>' +
      `<g class="bang">${R(78, 4, 18, 24, 9, T('amber'))}${R(85.5, 8, 3, 10, 1.5, T('ink'))}${C(87, 22, 1.8, T('ink'))}</g>` +
      `<g class="spark">${S('M84 12 L84 24 M78 18 L90 18', T('green'), 3)}</g>`;
    const ev = ch.eye || 'var(--ink)';
    return `<svg class="ch st-${state}" id="${id}" viewBox="0 0 100 100" style="--t:${phase}s;--eye:${ev}" role="img" aria-label="${ch.name}, ${state}">` +
      `<g class="mv"><g class="bd">${ch.draw(ch.c)}<g class="ey">${ey}</g><g class="hp">${hp}</g><g class="xx">${xx}</g></g></g>${fx}</svg>`;
  };
  const hash = s => [...s].reduce((h, c) => (h * 31 + c.charCodeAt(0)) >>> 0, 7);

  window.Cast = { birds, blobs, themes: { farm, kitchen, garden, bush }, ascii, asciiFrames, STATES, svg, hash, T };
})();
