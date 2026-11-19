// cooee#22: regenerates every PNG here: index.html and sheet.html at 390 (DPR 2) and 1440, each in bush-light and
// bush-dark, under Reduce Motion (frame 1), and the icon PNGs from art.mjs. No dependencies: Chrome over
// --remote-debugging-pipe (cdp.mjs). `node docs/design/bush/shoot.mjs <chrome-headless-shell> [only]`.
import { writeFileSync, mkdirSync } from 'node:fs';
import { serve, chrome } from './cdp.mjs';
import { icon, favicon } from './art.mjs';

const bin = process.argv[2] || 'chrome-headless-shell', only = process.argv[3];
const root = new URL('../../../', import.meta.url).pathname, here = 'docs/design/bush';
mkdirSync(new URL('icon/', import.meta.url), { recursive: true });
for (const n of [180, 192, 512]) writeFileSync(new URL(`icon/icon-${n}.svg`, import.meta.url), icon(n));
writeFileSync(new URL('icon/favicon.svg', import.meta.url), favicon(32));
const srv = await serve(root), c = chrome(bin);
const shots = [
  ...[180, 192, 512].map(n => [`icon/icon-${n}.svg`, `icon/icon-${n}.png`, n, n, 1]),
  ['icon/favicon.svg', 'icon/favicon-32.png', 32, 32, 1], ['icon/favicon.svg', 'icon/favicon-16.png', 16, 16, 1, 'zoom'],
  ...['index', 'sheet'].flatMap(p => ['light', 'dark'].flatMap(s => [
    [`${p}.html?s=${s}&still=1`, `${p}-390-${s}.png`, 390, 844, 2, 'full'],
    [`${p}.html?s=${s}&still=1`, `${p}-1440-${s}.png`, 1440, 900, 1, 'full'],
  ])),
].filter(s => !only || s[1].includes(only));
try {
  for (const [page, out, W, H, dpr, how] of shots) {
    const p = await c.open(`${srv.base}/${here}/${page}`, W, H, dpr, { reduce: /still/.test(page), ready: page.endsWith('.svg') ? 'document.readyState === "complete"' : 'document.body.dataset.ready === "1"' });
    if (how === 'zoom') await p.cmd('Emulation.setDeviceMetricsOverride', { width: 16, height: 16, deviceScaleFactor: 0.5, mobile: false });
    const sideways = how === 'full' ? await p.ev('document.documentElement.scrollWidth > innerWidth') : false;
    writeFileSync(new URL(out, import.meta.url), how === 'full' ? await p.full() : await p.png());
    await p.done();
    console.log('wrote', out, sideways ? '(SIDEWAYS SCROLL)' : '');
  }
} finally { c.kill(); srv.close(); }
if (c.errors.length) { console.error(c.errors.join('\n')); process.exit(1); }
