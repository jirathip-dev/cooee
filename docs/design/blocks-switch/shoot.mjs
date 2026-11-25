// Renders each page at 390 (@2x, the iPhone) and 1440 (@1x, the Mac), full height:
//   node docs/design/blocks-switch/shoot.mjs <chrome or chrome-headless-shell> [only-regex]
import { spawn } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { extname, join } from 'node:path';

const here = new URL('.', import.meta.url).pathname;
const [chrome, only] = process.argv.slice(2);
const VIEWS = ['chat', 'sessions', 'pair'].flatMap(p => [[p, 390, 2], [p, 1440, 1]]);
const TYPES = { '.html': 'text/html', '.css': 'text/css', '.js': 'text/javascript', '.png': 'image/png', '.json': 'application/json' };
const srv = createServer((q, r) => {
  try {
    const p = join(here, decodeURIComponent(new URL(q.url, 'http://x').pathname));
    r.writeHead(200, { 'content-type': TYPES[extname(p)] || 'application/octet-stream' }).end(readFileSync(p));
  } catch { r.writeHead(404).end(); }
}).listen(0, '127.0.0.1');
await new Promise(ok => srv.on('listening', ok));
const base = `http://127.0.0.1:${srv.address().port}/`;
const c = spawn(chrome, ['--headless', '--hide-scrollbars', '--remote-debugging-pipe', '--no-first-run',
  `--user-data-dir=${mkdtempSync(join(tmpdir(), 'bs-prof-'))}`], { stdio: ['ignore', 'ignore', 'ignore', 'pipe', 'pipe'] });
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
const { targetId } = await send('Target.createTarget', { url: 'about:blank' });
const { sessionId: s } = await send('Target.attachToTarget', { targetId, flatten: true });
const cmd = (m, p) => send(m, p, s);
const js = async x => (await cmd('Runtime.evaluate', { expression: x, returnByValue: true, awaitPromise: true })).result.value;
await cmd('Page.enable');
let fail = 0;
for (const [page, w, dpr] of VIEWS) {
  const out = `${page}-${w}.png`;
  if (only && !new RegExp(only).test(out)) continue;
  await cmd('Emulation.setDeviceMetricsOverride', { width: w, height: 900, deviceScaleFactor: dpr, mobile: w < 1000 });
  await cmd('Page.navigate', { url: `${base}${page}.html` });
  for (let i = 0; i < 50 && !(await js('document.body && document.body.dataset.ready')); i++) await sleep(100);
  await js('Promise.all([...document.images].map(i => i.decode().catch(() => 0)))');
  const wide = await js('document.documentElement.scrollWidth');
  if (wide > w) { console.log(`FAIL ${out}: sideways scroll ${wide} > ${w}`); fail++; }
  const h = await js('document.documentElement.scrollHeight');
  await cmd('Emulation.setDeviceMetricsOverride', { width: w, height: h, deviceScaleFactor: dpr, mobile: w < 1000 });
  await sleep(300);
  const { data } = await cmd('Page.captureScreenshot', { format: 'png' });
  writeFileSync(join(here, out), Buffer.from(data, 'base64'));
  console.log(`ok   ${out} ${w}×${h}`);
}
c.kill(); srv.close();
process.exit(fail ? 1 : 0);
