// #47: shoot the pairing design (pair.html: an unpaired device; approve.html: a paired one) and measure each shot
// against herdsman's docs/ui.md gate 2, as the look's shoot.mjs (#7) does: no sideways scroll or box past the right
// edge, text 11/13 px (Modern 13/15/17; inputs 16), controls 28 px (Modern 36) with a ≥ 44 px tap, tap rows ≥ 44 px,
// one-line cells ellipsised. One addition: the code boxes, a shown code and the check word are --fs-code (32 px),
// herdsman's own pairing-code size. No deps: Chrome's --remote-debugging-pipe. Serves docs/design/, since the pages
// use ../look/. Usage (repo root): node docs/design/pair/shoot.mjs <chrome-headless-shell> [name regexp].
import { spawn } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { extname, join } from 'node:path';

const here = new URL('.', import.meta.url).pathname, root = new URL('..', import.meta.url).pathname;
const [chrome] = process.argv.slice(2); // and an optional regexp: shoot only the names it matches
// [page, query, name, width, height: full (the page), DPR, the system's mode]
const VIEWS = [];
for (const [w, dpr] of [[390, 2], [1440, 1]]) for (const m of ['dark', 'light']) for (const p of ['pair', 'approve'])
  VIEWS.push([`pair/${p}.html`, '?theme=farm', `${p}-${w}-${m}`, w, 'full', dpr, m]);
// The other style and themes once each: Terminal (garden on a light phone, kitchen on a dark one), and the terminal
// theme with Modern, whose figure is its icon.
VIEWS.push(['pair/pair.html', '?theme=garden&style=terminal', 'pair-terminal-390-light', 390, 'full', 2, 'light'],
  ['pair/approve.html', '?theme=kitchen&style=terminal', 'approve-terminal-390-dark', 390, 'full', 2, 'dark'],
  ['pair/pair.html', '?theme=terminal', 'pair-themeterminal-390-dark', 390, 'full', 2, 'dark']);
const only = process.argv[3] ? new RegExp(process.argv[3]) : null;
const TYPES = { '.html': 'text/html', '.css': 'text/css', '.js': 'text/javascript', '.png': 'image/png', '.json': 'application/json', '.svg': 'image/svg+xml' };
const srv = createServer((q, r) => {
  try {
    const p = join(root, decodeURIComponent(new URL(q.url, 'http://x').pathname));
    r.writeHead(200, { 'content-type': TYPES[extname(p)] || 'application/octet-stream' }).end(readFileSync(p));
  } catch { r.writeHead(404).end(); }
}).listen(0, '127.0.0.1');
await new Promise(ok => srv.on('listening', ok));
const base = `http://127.0.0.1:${srv.address().port}/`;

const c = spawn(chrome, ['--headless', '--no-sandbox', '--hide-scrollbars', '--remote-debugging-pipe', '--no-first-run',
  `--user-data-dir=${mkdtempSync(join(tmpdir(), 'cooee-prof-'))}`], { stdio: ['ignore', 'ignore', 'ignore', 'pipe', 'pipe'] });
let buf = '', id = 0;
const wait = new Map();
c.stdio[4].on('data', d => {
  buf += d;
  for (let i; (i = buf.indexOf('\0')) >= 0; buf = buf.slice(i + 1)) {
    const m = JSON.parse(buf.slice(0, i));
    if (m.id && wait.has(m.id)) { wait.get(m.id)(m); wait.delete(m.id); }
  }
});
const send = (method, params = {}, sessionId) => new Promise((ok, no) => {
  wait.set(++id, m => m.error ? no(new Error(`${method}: ${m.error.message}`)) : ok(m.result));
  c.stdio[3].write(JSON.stringify({ id, method, params, sessionId }) + '\0');
});
const sleep = ms => new Promise(ok => setTimeout(ok, ms));

// Runs in the page: every rule above, as a list of "what · where" strings.
function audit() {
  const modern = document.documentElement.dataset.style === 'modern', SIZES = modern ? [13, 15, 17] : [11, 13], CTL = modern ? 36 : 28;
  const bad = [], W = innerWidth, name = e => e.tagName.toLowerCase() + (e.className && typeof e.className === 'string' ? '.' + e.className.trim().split(/\s+/).join('.') : '') + ` "${(e.textContent || e.placeholder || '').trim().slice(0, 30)}"`;
  const shown = e => e.getClientRects().length && getComputedStyle(e).visibility !== 'hidden';
  if (document.documentElement.scrollWidth > W) bad.push(`sideways scroll ${document.documentElement.scrollWidth} > ${W}`);
  for (const e of document.querySelectorAll('body *')) {
    if (!shown(e)) continue;
    const r = e.getBoundingClientRect(), s = getComputedStyle(e);
    if (r.right > W + 0.5) bad.push(`past the right edge ${Math.round(r.right)} · ${name(e)}`);
    const own = [...e.childNodes].some(n => n.nodeType === 3 && n.textContent.trim());
    const fs = parseFloat(s.fontSize), input = e.matches('input,textarea,select');
    const code = e.matches('.cb input,.cw,.bc') && fs === 32;
    if ((own || input) && !code && !(input ? fs === 16 : SIZES.includes(fs))) bad.push(`font ${fs}px · ${name(e)}`);
    if (e.matches('button')) {
      const choice = e.matches('[name=choice],.pick');
      if (!choice && Math.round(r.height) !== CTL) bad.push(`control ${r.height}px tall · ${name(e)}`);
      if (choice && r.height > 4 * SIZES[1] * 1.35 + CTL) bad.push(`choice over 4 lines · ${name(e)}`);
      if (r.width + 16 < 44) bad.push(`tap ${r.width + 16}px wide · ${name(e)}`);
    }
    if (e.matches('.tr,.blk>summary,.fc>.done>summary') && r.height + (e.matches('.blk>summary') ? 16 : 0) < 44) bad.push(`tap row ${r.height}px · ${name(e)}`);
    if (own && e.scrollWidth > e.clientWidth + 1 && s.overflowX !== 'visible' && s.textOverflow !== 'ellipsis' && !e.matches('pre,textarea,input')) bad.push(`text cut without an ellipsis · ${name(e)}`);
  }
  return bad;
}

const { targetId } = await send('Target.createTarget', { url: 'about:blank' });
const { sessionId: s } = await send('Target.attachToTarget', { targetId, flatten: true });
const cmd = (m, p) => send(m, p, s);
const js = async x => (await cmd('Runtime.evaluate', { expression: x, returnByValue: true, awaitPromise: true })).result.value;
await cmd('Page.enable');
let fail = 0;
await cmd('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: 'reduce' }] });
for (const [page, query, out, w, fit, dpr, sys] of VIEWS) {
  if (only && !only.test(out)) continue;
  await cmd('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-color-scheme', value: sys }, { name: 'prefers-reduced-motion', value: 'reduce' }] });
  const h0 = w < 1000 ? 844 : 900;
  await cmd('Emulation.setDeviceMetricsOverride', { width: w, height: h0, deviceScaleFactor: dpr, mobile: w < 1000 });
  await cmd('Page.navigate', { url: base + page + query });
  for (let i = 0; i < 50 && !(await js('document.body && document.body.dataset.ready')); i++) await sleep(100);
  await sleep(300);
  if (fit === 'full') await js("document.documentElement.classList.add('shot')");
  const bad = await js(`(${audit})()`);
  const h = fit === 'full' ? await js('document.documentElement.scrollHeight')
    : h0;
  if (h !== h0) await cmd('Emulation.setDeviceMetricsOverride', { width: w, height: h, deviceScaleFactor: dpr, mobile: w < 1000 });
  await sleep(200);
  const { data } = await cmd('Page.captureScreenshot', { format: 'png' });
  const file = `${out}.png`;
  writeFileSync(join(here, file), Buffer.from(data, 'base64'));
  console.log(`${bad.length ? 'FAIL' : 'ok  '} ${file} ${w}×${h}${bad.length ? '\n  ' + bad.join('\n  ') : ''}`);
  fail += bad.length;
}
c.kill(); srv.close();
process.exit(fail ? 1 : 0);
