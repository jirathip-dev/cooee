// #153: shoot each frame alone (`?f=`): a phone screen at 390 × 844 (DPR 2) and a window at 1440 × 900 (DPR 1), farm
// scheme, under reduced motion, in a style (Modern, Terminal, Scene). Each shot is checked: no sideways scroll, nothing
// past the right edge, a cut line ellipsised. No deps: Chrome's --remote-debugging-pipe, as native-shell/shoot.mjs.
// Usage (repo root): node docs/design/rooms/shoot.mjs <chrome-headless-shell> [regexp]
import { spawn } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { extname, join } from 'node:path';

const here = new URL('.', import.meta.url).pathname, root = join(here, '../../..');
const [chrome, re] = process.argv.slice(2);
// [page, frame, width, style, modes]: the PNG is `<frame>[-<style>]-390-<mode>.png` or `<frame>[-<style>]-1440.png`
// (Modern unnamed).
const BOTH = ['dark', 'light'];
const VIEWS = [
  ['room', 'home', 390, 'modern', ['dark']], ['room', 'create', 390, 'modern', BOTH], ['room', 'room', 390, 'modern', BOTH], ['room', 'mention', 390, 'modern', ['dark']],
  ['room', 'member', 390, 'modern', ['dark']], ['room', 'wide-room', 1440, 'modern', ['dark']],
  ['linked', 'asked', 390, 'modern', BOTH], ['linked', 'linked', 390, 'modern', ['dark']], ['linked', 'approval', 390, 'modern', ['dark']], ['linked', 'push', 390, 'modern', ['dark']],
  ['linked', 'wide-linked', 1440, 'modern', ['light']],
  ['room', 'room', 390, 'terminal', ['dark']], ['room', 'room', 390, 'scene', ['light']], ['room', 'mention', 390, 'terminal', ['dark']],
  ['linked', 'linked', 390, 'terminal', ['dark']], ['linked', 'approval', 390, 'scene', ['light']], ['room', 'wide-room', 1440, 'terminal', ['dark']],
];
const TYPES = { '.html': 'text/html', '.css': 'text/css', '.js': 'text/javascript', '.png': 'image/png', '.json': 'application/json', '.svg': 'image/svg+xml' };
const srv = createServer((q, r) => {
  try {
    const p = join(root, decodeURIComponent(new URL(q.url, 'http://x').pathname)), b = readFileSync(p);
    r.writeHead(200, { 'content-type': TYPES[extname(p)] || 'application/octet-stream' }).end(b);
  } catch { r.writeHead(404).end(); }
}).listen(0, '127.0.0.1');
await new Promise(ok => srv.on('listening', ok));
const base = `http://127.0.0.1:${srv.address().port}/docs/design/rooms/`;

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
  for (const e of document.querySelectorAll('.one *')) {
    // svg and the status bar are not text.
    if (!e.getClientRects().length || e.closest('svg,.stb,.rdim')) continue;
    const r = e.getBoundingClientRect(), s = getComputedStyle(e);
    if (s.visibility === 'hidden' || r.bottom < 0 || r.top > innerHeight) continue;
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
for (const [page, f, w, style, modes] of VIEWS) for (const mode of modes) {
  const sx = style === 'modern' ? '' : `-${style}`;
  const out = w < 1000 ? `${f}${sx}-390-${mode}.png` : `${f.replace(/^wide-/, '')}${sx}-1440.png`;
  if (re && !new RegExp(re).test(out)) continue;
  await cmd('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-color-scheme', value: mode }, { name: 'prefers-reduced-motion', value: 'reduce' }] });
  const h = w < 1000 ? 844 : 900, dpr = w < 1000 ? 2 : 1;
  await cmd('Emulation.setDeviceMetricsOverride', { width: w, height: h, deviceScaleFactor: dpr, mobile: w < 1000 });
  await cmd('Page.navigate', { url: `${base}${page}.html?f=${f}&theme=farm&mode=${mode}&style=${style}` });
  for (let i = 0; i < 50 && !(await js('document.body && document.body.dataset.ready')); i++) await sleep(100);
  await sleep(500);
  const bad = await js(`(${audit})()`);
  const { data } = await cmd('Page.captureScreenshot', { format: 'png', clip: { x: 0, y: 0, width: w, height: h, scale: 1 } });
  writeFileSync(join(here, out), Buffer.from(data, 'base64'));
  console.log(`${bad.length ? 'FAIL' : 'ok  '} ${out} farm ${style} ${mode} ${w}×${h}${bad.length ? '\n  ' + bad.join('\n  ') : ''}`);
  fail += bad.length;
}
c.kill(); srv.close();
process.exit(fail ? 1 : 0);
