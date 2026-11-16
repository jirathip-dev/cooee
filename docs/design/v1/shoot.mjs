import { spawn } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { extname, join } from 'node:path';

const here = new URL('.', import.meta.url).pathname;
const [chrome] = process.argv.slice(2);
const VIEWS = [
  ['threads.html', '', 'threads-390', 390, true, 2], ['threads.html', '#new', 'threads-new-390', 390, true, 2],
  ['threads.html', '', 'threads-1440', 1440, true, 1],
  ['chat.html', '', 'chat-390', 390, true, 2], ['chat.html', '#working', 'chat-working-390', 390, true, 2],
  ['chat.html', '', 'chat-1440', 1440, true, 1],
  ['login.html', '', 'login-390', 390, false, 2], ['login.html', '', 'login-1440', 1440, false, 1],
];
const TYPES = { '.html': 'text/html', '.css': 'text/css', '.js': 'text/javascript', '.png': 'image/png' };
const srv = createServer((q, r) => {
  try {
    const p = join(here, decodeURIComponent(new URL(q.url, 'http://x').pathname));
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

function audit() {
  const bad = [], W = innerWidth, name = e => e.tagName.toLowerCase() + (e.className && typeof e.className === 'string' ? '.' + e.className.trim().split(/\s+/).join('.') : '') + ` "${(e.textContent || e.placeholder || '').trim().slice(0, 30)}"`;
  const shown = e => e.getClientRects().length && getComputedStyle(e).visibility !== 'hidden';
  if (document.documentElement.scrollWidth > W) bad.push(`sideways scroll ${document.documentElement.scrollWidth} > ${W}`);
  for (const e of document.querySelectorAll('body *')) {
    if (!shown(e)) continue;
    const r = e.getBoundingClientRect(), s = getComputedStyle(e);
    if (r.right > W + 0.5) bad.push(`past the right edge ${Math.round(r.right)} · ${name(e)}`);
    const own = [...e.childNodes].some(n => n.nodeType === 3 && n.textContent.trim());
    const fs = parseFloat(s.fontSize), input = e.matches('input,textarea,select');
    if ((own || input) && !(input ? fs === 16 : fs === 13 || fs === 11)) bad.push(`font ${fs}px · ${name(e)}`);
    if (e.matches('button')) {
      const choice = e.matches('[name=choice],.pick');
      if (!choice && Math.round(r.height) !== 28) bad.push(`control ${r.height}px tall · ${name(e)}`);
      if (choice && r.height > 4 * 13 * 1.35 + 28) bad.push(`choice over 4 lines · ${name(e)}`);
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
for (const theme of ['dark', 'light']) {
  await cmd('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-color-scheme', value: theme }] });
  for (const [page, hash, out, w, full, dpr] of VIEWS) {
    const h0 = w < 1000 ? 844 : 900;
    await cmd('Emulation.setDeviceMetricsOverride', { width: w, height: h0, deviceScaleFactor: dpr, mobile: w < 1000 });
    await cmd('Page.navigate', { url: base + page + hash });
    await sleep(300);
    if (full) await js("document.documentElement.classList.add('shot')");
    const bad = await js(`(${audit})()`), h = full ? await js('document.documentElement.scrollHeight') : h0;
    if (full) await cmd('Emulation.setDeviceMetricsOverride', { width: w, height: h, deviceScaleFactor: dpr, mobile: w < 1000 });
    await sleep(200);
    const { data } = await cmd('Page.captureScreenshot', { format: 'png' });
    const file = `${out}-${theme}.png`;
    writeFileSync(join(here, file), Buffer.from(data, 'base64'));
    console.log(`${bad.length ? 'FAIL' : 'ok  '} ${file} ${w}×${h}${bad.length ? '\n  ' + bad.join('\n  ') : ''}`);
    fail += bad.length;
  }
}
c.kill(); srv.close();
process.exit(fail ? 1 : 0);
