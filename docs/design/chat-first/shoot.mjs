// #86: shoot the three pages in each Style at 390 (DPR 2, every phone screen in turn) and 1440 (DPR 1, the screen,
// grown until nothing scrolls inside), under reduced motion so every loop shows frame 1. Each shot is checked: no
// sideways scroll, nothing past the right edge, a cut line ellipsised. No deps: Chrome's --remote-debugging-pipe, as
// docs/design/look/shoot.mjs. Usage (repo root): node docs/design/chat-first/shoot.mjs <chrome-headless-shell> [regexp]
import { spawn } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { extname, join } from 'node:path';

const here = new URL('.', import.meta.url).pathname, root = join(here, '../../..');
const [chrome, re] = process.argv.slice(2);
// Style → [theme, mode] at 390 and at 1440, so every theme shows once and each Style in light and dark.
const LOOKS = { modern: [['farm', 'dark'], ['garden', 'light']], terminal: [['plain', 'light'], ['plain', 'dark']],
  scene: [['farm', 'light'], ['kitchen', 'dark']] };
const VIEWS = [];
for (const page of ['agents', 'new', 'received']) for (const [style, looks] of Object.entries(LOOKS))
  for (const [i, w] of [390, 1440].entries()) VIEWS.push([page, style, w, ...looks[i]]);
const TYPES = { '.html': 'text/html', '.css': 'text/css', '.js': 'text/javascript', '.png': 'image/png', '.json': 'application/json', '.svg': 'image/svg+xml' };
const missing = [];
const srv = createServer((q, r) => {
  try {
    const p = join(root, decodeURIComponent(new URL(q.url, 'http://x').pathname));
    const b = readFileSync(p);
    r.writeHead(200, { 'content-type': TYPES[extname(p)] || 'application/octet-stream' }).end(b);
  } catch { if (!q.url.endsWith('/favicon.ico')) missing.push(q.url); r.writeHead(404).end(); }
}).listen(0, '127.0.0.1');
await new Promise(ok => srv.on('listening', ok));
const base = `http://127.0.0.1:${srv.address().port}/docs/design/chat-first/`;

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
    // The scene's pages and the composer's chips scroll sideways by design.
    if (!e.getClientRects().length || e.closest('.spg,.mc,svg')) continue;
    const r = e.getBoundingClientRect(), s = getComputedStyle(e);
    if (r.right > W + 0.5) bad.push(`past the right edge ${Math.round(r.right)} · ${name(e)}`);
    const own = [...e.childNodes].some(n => n.nodeType === 3 && n.textContent.trim());
    if (own && e.scrollWidth > e.clientWidth + 1 && s.overflowX !== 'visible' && s.textOverflow !== 'ellipsis' && !e.matches('textarea,input,select')) bad.push(`text cut without an ellipsis · ${name(e)}`);
  }
  return bad;
}

const { targetId } = await send('Target.createTarget', { url: 'about:blank' });
const { sessionId: s } = await send('Target.attachToTarget', { targetId, flatten: true });
const cmd = (m, p) => send(m, p, s);
const js = async x => (await cmd('Runtime.evaluate', { expression: x, returnByValue: true, awaitPromise: true })).result.value;
await cmd('Page.enable');
let fail = 0;
for (const [page, style, w, theme, mode] of VIEWS) {
  const out = `${page}-${style}-${w}.png`;
  if (re && !new RegExp(re).test(out)) continue;
  await cmd('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-color-scheme', value: mode }, { name: 'prefers-reduced-motion', value: 'reduce' }] });
  const h0 = w < 1000 ? 844 : 900, dpr = w < 1000 ? 2 : 1;
  await cmd('Emulation.setDeviceMetricsOverride', { width: w, height: h0, deviceScaleFactor: dpr, mobile: w < 1000 });
  await cmd('Page.navigate', { url: `${base}${page}.html?style=${style}&theme=${theme}&mode=${mode}` });
  for (let i = 0; i < 50 && !(await js('document.body && document.body.dataset.ready')); i++) await sleep(100);
  await sleep(400);
  const bad = await js(`(${audit})()`);
  const h = w < 1000 ? await js('document.documentElement.scrollHeight')
    : h0 + await js("Math.max(0, ...[...document.querySelectorAll('.tc,.ags')].map(e => e.scrollHeight - e.clientHeight))");
  if (h !== h0) await cmd('Emulation.setDeviceMetricsOverride', { width: w, height: h, deviceScaleFactor: dpr, mobile: w < 1000 });
  await sleep(300);
  const { data } = await cmd('Page.captureScreenshot', { format: 'png' });
  writeFileSync(join(here, out), Buffer.from(data, 'base64'));
  bad.push(...missing.splice(0).map(u => `404 ${u}`));
  console.log(`${bad.length ? 'FAIL' : 'ok  '} ${out} ${theme} ${mode} ${w}×${h}${bad.length ? '\n  ' + bad.join('\n  ') : ''}`);
  fail += bad.length;
}
c.kill(); srv.close();
process.exit(fail ? 1 : 0);
