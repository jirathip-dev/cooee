import { spawn } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { extname, join } from 'node:path';

const here = new URL('.', import.meta.url).pathname;
const [chrome, re] = process.argv.slice(2);
const VIEWS = [
  ['modern', '', 1440, 'dark', 'modern-1440.png'], ['modern', '', 390, 'dark', 'modern-390.png'],
  ['modern', '', 1440, 'light', 'modern-1440-light.png'],
  ['casts', '', 1440, 'dark', 'casts-1440.png'], ['casts', '', 390, 'dark', 'casts-390.png'],
  ['places', 'wide', 1440, 'dark', 'places-1440.png'],
  ['places', 'a', 390, 'dark', 'places-a-390-dark.png'], ['places', 'a', 390, 'light', 'places-a-390-light.png'],
  ['places', 'b', 390, 'dark', 'places-b-390-dark.png'], ['places', 't', 390, 'dark', 'places-terminal-390-dark.png'],
];
const CLIP = process.env.CLIP;
const TYPES = { '.html': 'text/html', '.css': 'text/css', '.js': 'text/javascript', '.png': 'image/png', '.svg': 'image/svg+xml' };
const srv = createServer((q, r) => {
  try {
    const p = join(here, decodeURIComponent(new URL(q.url, 'http://x').pathname)), b = readFileSync(p);
    r.writeHead(200, { 'content-type': TYPES[extname(p)] || 'application/octet-stream' }).end(b);
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

function audit() {
  const bad = [], W = innerWidth, name = e => e.tagName.toLowerCase() + (typeof e.className === 'string' && e.className ? '.' + e.className.trim().split(/\s+/).join('.') : '') + ` "${(e.textContent || '').trim().slice(0, 30)}"`;
  if (document.documentElement.scrollWidth > W) bad.push(`sideways scroll ${document.documentElement.scrollWidth} > ${W}`);
  for (const e of document.querySelectorAll('body *')) {
    if (!e.getClientRects().length || e.closest('svg')) continue;
    const r = e.getBoundingClientRect(), s = getComputedStyle(e);
    if (r.right > W + 0.5 && !e.closest('.bleed')) bad.push(`past the right edge ${Math.round(r.right)} · ${name(e)}`);
    const own = [...e.childNodes].some(n => n.nodeType === 3 && n.textContent.trim());
    if (own && e.scrollWidth > e.clientWidth + 1 && s.overflowX !== 'visible' && s.textOverflow !== 'ellipsis') bad.push(`text cut without an ellipsis · ${name(e)}`);
  }
  return bad;
}

const { targetId } = await send('Target.createTarget', { url: 'about:blank' });
const { sessionId: s } = await send('Target.attachToTarget', { targetId, flatten: true });
const cmd = (m, p) => send(m, p, s);
const js = async x => (await cmd('Runtime.evaluate', { expression: x, returnByValue: true, awaitPromise: true })).result.value;
await cmd('Page.enable');
const open = async (page, f, w, mode, dpr) => {
  await cmd('Emulation.setDeviceMetricsOverride', { width: w, height: 844, deviceScaleFactor: dpr, mobile: w < 1000 });
  await cmd('Page.navigate', { url: `${base}${page}.html?f=${f}&mode=${mode}` });
  for (let i = 0; i < 50 && !(await js('document.body && document.body.dataset.ready')); i++) await sleep(100);
  await sleep(300);
};
const shot = async (w, h, scale = 1) => Buffer.from((await cmd('Page.captureScreenshot', { format: 'png', captureBeyondViewport: true, clip: { x: 0, y: 0, width: w, height: h, scale } })).data, 'base64');
let fail = 0;
if (CLIP) {
  const [page, f, w, h, n, fps] = CLIP.split(':'), dir = join(tmpdir(), `cooee-clip-${process.pid}`);
  mkdirSync(dir);
  await open(page, f, +w, 'dark', 1);
  for (let i = 0; i < +n; i++) {
    await js(`setTime(${i / +fps})`);
    await sleep(30);
    writeFileSync(join(dir, `${String(i).padStart(4, '0')}.png`), await shot(+w, +h));
  }
  console.log(dir);
} else for (const [page, f, w, mode, out] of VIEWS) {
  if (re && !new RegExp(re).test(out)) continue;
  await open(page, f, w, mode, w < 1000 ? 2 : 1);
  await js('window.setTime && setTime(0)');
  const h = await js('document.body.scrollHeight');
  const bad = await js(`(${audit})()`);
  writeFileSync(join(here, out), await shot(w, h));
  console.log(`${bad.length ? 'FAIL' : 'ok  '} ${out} ${w}×${h}${bad.length ? '\n  ' + bad.join('\n  ') : ''}`);
  fail += bad.length;
}
c.kill(); srv.close();
process.exit(fail ? 1 : 0);
