// #34: writes each theme's Scene backdrop, `design-system/themes/<name>/scene.svg` (390 × 320 shown at the strip's width), from
// herdsman's Herd scene: kitchen's and garden's `assets/themes/<name>/scene.svg` as they are, and farm's ranch, which
// herdsman's `assets/herd.js` draws with script (`scene()`, `night()`, `clouds()` and herd.css's light pairs, ported
// here). The ranch is still: no wind, and night is the `#night` fragment (`scene.svg#night`, by `:target`), since an
// <img> takes no page CSS. `node docs/design/scene/build.mjs <herdsman checkout>`
import { readFileSync, writeFileSync } from "node:fs";

const herdsman = process.argv[2];
if (!herdsman) throw new Error("usage: node docs/design/scene/build.mjs <herdsman checkout>");
const out = (t) => new URL(`../../../design-system/themes/${t}/scene.svg`, import.meta.url);

for (const t of ["kitchen", "garden"]) {
  const svg = readFileSync(`${herdsman}/assets/themes/${t}/scene.svg`, "utf8");
  writeFileSync(out(t), svg.replace(/^(<svg[^>]*>)/, `$1<!-- herdsman's ${t} scene; docs/design/scene/build.mjs -->`));
}

// herd.js, as it is there.
const M64 = (1n << 64n) - 1n;
const noise = (i, s = 0) => {
  const n = (BigInt(i) * 1664525n + BigInt(s) * 1013904223n) & M64;
  return Number(((n ^ (n >> 13n)) * 1274126177n) % 65536n) / 65535;
};
const f1 = (n) => +n.toFixed(1);
const ell = (x, y, rx, ry, fill, op = 1) =>
  `<ellipse cx="${f1(x)}" cy="${f1(y)}" rx="${f1(rx)}" ry="${f1(ry)}" fill="${fill}"${op < 1 ? ` opacity="${op}"` : ""}/>`;
const rect = (x, y, w, h, fill, op = 1) =>
  `<rect x="${f1(x)}" y="${f1(y)}" width="${f1(w)}" height="${f1(h)}" fill="${fill}"${op < 1 ? ` opacity="${op}"` : ""}/>`;
function clouds() {
  let s = "";
  for (let i = 0; i < 4; i++) {
    const x = i * 130 + 45 - 130 * 0.5,
      y = 36 + noise(i, 30) * 86;
    for (let j = 0; j < 5; j++) s += ell(x + j * 12, y - noise(j, i) * 9, 18, 5 + noise(j, 2) * 5, "var(--cl)");
  }
  return `<g class="hc" filter="url(#hcb)">${s}</g>`;
}
function night() {
  let s = "";
  for (const k of [-390, 0, 390])
    for (let b = 0; b < 6; b++)
      s += `<path d="M${k - 60} 220Q${k + 60} 74 ${k + 220} 3Q${k + 275} -20 ${k + 335} -32" stroke-width="${66 - b * 8}"/>`;
  s = `<g class="mw" filter="url(#hmw)">${s}</g>`;
  for (let i = 0; i < 130; i++) {
    const r = 0.35 + noise(i, 7) * 0.7;
    s += ell(i * 3 + noise(i, 8) * 3, noise(i, 3) * 225, r, r, "#e7e5f4", f1((0.3 + 0.45 * noise(i, 9) + 0.08 * Math.sin(i)) * 100) / 100);
  }
  return `<g class="sky-night">${s}${ell(306, 56, 16, 16, "#d3dfeb", 0.84)}${ell(301, 51, 15, 15, "#1d2841")}</g>`;
}
function scene() {
  let s = `<rect class="sky" width="390" height="320"/>${night()}${clouds()}`;
  ["var(--h0)", "var(--h1)", "var(--h2)", "var(--h3)"].forEach((fill, r) => {
    let d = "M-85 330";
    for (let i = -4; i <= 14; i++) d += `L${i * 35} ${f1(147 + r * 30 + noise(i, r + 17) * 36)}`;
    s += `<path d="${d}L475 330Z" fill="${fill}"/>`;
  });
  s += '<path class="gd" d="M-60 258Q165 228 450 268L450 330L-60 330Z"/>';
  for (let i = -1; i <= 2; i++) {
    const x = i * 160;
    s += `<path d="M${x + 130} 258Q${x + 40} 430 ${x + 180} 640L${x + 226} 640Q${x + 62} 438 ${x + 145} 258Z" fill="var(--lw)" opacity=".18"/>`;
    s += `<path d="M${x} 277Q${x - 35} 395 ${x + 130} 520L${x + 55} 566Q${x - 55} 389 ${x - 30} 290Z" fill="var(--sh)" opacity=".12"/>`;
  }
  for (let i = -152; i <= 1650; i++) {
    const y = 270 + noise(i, 19) * 370;
    if (y < 330) s += ell(Math.floor(i / 19) * 5 + noise(i, 8) * 5, y, 0.3 + noise(i, 2) * 0.7, 0.3, "var(--st)", 0.13);
  }
  s +=
    '<path d="M153 243L180 219L210 242L210 279L153 279Z" fill="var(--bn)"/><path d="M148 245L180 215L216 243L209 246L180 224L155 248Z" fill="var(--rf)"/>';
  s += rect(175, 252, 14, 27, "var(--dr)") + rect(175, 235, 9, 9, "var(--wn)", 0.8);
  for (const [x, ty, sc] of [
    [7, 225, 1],
    [365, 228, 1.1],
    [238, 258, 0.5],
    [326, 262, 0.38],
  ]) {
    s += rect(x - 2 * sc, ty - 8 * sc, 4 * sc, 45 * sc, "var(--tk)");
    for (let i = 0; i < 22; i++) {
      const dx = (noise(i, 20) - 0.5) * 48 * sc,
        dy = (noise(i, 7) - 0.5) * 35 * sc;
      s +=
        ell(x + dx, ty + dy, 9 * sc, 11 * sc, i % 3 === 0 ? "var(--l1)" : "var(--l2)", 0.9) +
        ell(x + dx - 2 * sc, ty + dy - 3 * sc, 5 * sc, 4 * sc, "var(--lh)", 0.1);
    }
  }
  for (let i = -2; i <= 9; i++) s += rect(i * 52, 249, 6, 44, "var(--fp)") + rect(i * 52 + 1, 249, 1.4, 44, "var(--fh)", 0.5);
  for (const dy of [4, 22]) s += rect(-60, 256 + dy, 510, 5, "var(--fr)") + rect(-60, 256 + dy, 510, 1.3, "var(--fl)", 0.8);
  for (let i = -5; i <= 48; i++) {
    const x = i * 9,
      y = 298 + noise(i, 43) * 24;
    let b = "";
    for (let k = 0; k < 3; k++) b += `M${x + 2 * k} ${f1(y)}Q${x + 2 * k - 1} ${f1(y - 5)} ${x + 2 * k - 3} ${f1(y - 8 - noise(i, 4) * 4)}`;
    s += `<path class="gt" d="${b}"/>`;
  }
  return `${s}<rect class="wa" width="390" height="320"/>`;
}

// herd.css: Corral's day and night pairs, the gradients herd.js puts in its defs, and the sky gradient as two.
const grad = (id, x2, y2, stops, x1 = 0, y1 = 0) =>
  `<linearGradient id="${id}" gradientUnits="userSpaceOnUse" x1="${x1}" y1="${y1}" x2="${x2}" y2="${y2}">${stops
    .map(([o, c, op]) => `<stop offset="${o}" stop-color="${c}"${op === undefined ? "" : ` stop-opacity="${op}"`}/>`)
    .join("")}</linearGradient>`;
const defs =
  grad("hgr", 0, 640, [[0, "#bcc17b"], [0.5, "#8e9c58"], [1, "#485e3b"]], 0, 250) +
  grad("hgrn", 0, 640, [[0, "#63746a"], [0.5, "#415d55"], [1, "#273f3e"]], 0, 250) +
  grad("hwd", 195, 500, [[0, "#ffeeab", 0.03], [1, "#ffeeab", 0]]) +
  grad("hwn", 195, 500, [[0, "#bbcfe4", 0.03], [1, "#bbcfe4", 0]], 390) +
  grad("skd", 0, 320, [[0, "#388fc4"], [1, "#8ebbc6"]]) +
  grad("skn", 0, 320, [[0, "#121a32"], [1, "#3c4961"]]) +
  '<filter id="hmw" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="6"/></filter>' +
  '<filter id="hcb" x="-20%" y="-50%" width="140%" height="200%"><feGaussianBlur stdDeviation="3"/></filter>';
const style = `#night{--h0:#a9c9bf;--h1:#9fb9a5;--h2:#8aab88;--h3:#79966d;--lw:#d2c48c;--sh:#334c2e;--st:#e8e3b3;--bn:#a86e52;--rf:#536456;--dr:#344635;--wn:#e5dfb8;--tk:#776745;--l1:#859b5c;--l2:#66874f;--lh:#b0b67b;--fp:#9c916a;--fh:#e2d7ac;--fr:#8b825d;--fl:#d6c99f;--cl:#e3f1e9;--gs:#aeb57a}
#night:target{--h0:#617484;--h1:#586c78;--h2:#4c666a;--h3:#425b5a;--lw:#789085;--sh:#172e32;--st:#b9c3b1;--bn:#705b58;--rf:#344b50;--dr:#182d35;--wn:#ceb581;--tk:#3c4c4a;--l1:#557068;--l2:#405e59;--lh:#96aea0;--fp:#73847f;--fh:#d0dbd4;--fr:#6b7d75;--fl:#c4d6e0;--cl:#a6b6c6;--gs:#90a397}
.sky{fill:url(#skd)}#night:target .sky{fill:url(#skn)}.gd{fill:url(#hgr)}#night:target .gd{fill:url(#hgrn)}
.wa{fill:url(#hwd)}#night:target .wa{fill:url(#hwn)}.sky-night{display:none}#night:target .sky-night{display:inline}
.mw path{fill:none;stroke:#bab8db;stroke-opacity:.022;stroke-linecap:round}.hc{fill-opacity:.2}#night:target .hc{fill-opacity:.055}
.gt{fill:none;stroke:var(--gs);stroke-opacity:.45;stroke-width:.8}`;
writeFileSync(
  out("farm"),
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 390 320" preserveAspectRatio="xMidYMax slice" aria-hidden="true"><!-- herdsman's ranch (herd.js scene()); docs/design/scene/build.mjs --><defs>${defs}</defs><style>${style}</style><g id="night">${scene()}</g></svg>\n`,
);
