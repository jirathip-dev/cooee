import { spawn } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { extname, join } from 'node:path';

const here = new URL('.', import.meta.url).pathname;
const [chrome] = process.argv.slice(2);
const VIEWS = [];
for (const [w, dpr] of [[390, 2], [1440, 1]]) {
  for (const m of ['dark', 'light']) {
    for (const t of ['farm', 'kitchen', 'garden', 'terminal']) VIEWS.push(['themes.html', `?theme=${t}`, `themes-${t}-${w}-${m}`, w, 'full', dpr, m]);
    VIEWS.push(['settings.html', '?theme=farm', `settings-${w}-${m}`, w, 'sheet', dpr, m]);
  }
  VIEWS.push(['look.html', '?theme=farm&mode=system', `look-${w}-dark`, w, 'screen', dpr, 'dark'],
    ['look.html', '?theme=garden&mode=light', `look-${w}-light`, w, 'screen', dpr, 'dark'],
    ['look.html', '?theme=kitchen&mode=dark&light=paper', `look-kitchen-${w}-dark`, w, 'screen', dpr, 'light'],
    ['look.html', '?theme=terminal&mode=system', `look-terminal-${w}-light`, w, 'screen', dpr, 'light']);
}
for (const v of VIEWS) v[1] += '&style=terminal';
for (const m of ['dark', 'light']) for (const st of ['modern', 'terminal']) for (const sp of ['on', 'off'])
  VIEWS.push(['themes.html', `?theme=farm&style=${st}&sprites=${sp}`, `combo-${st}-sprites-${sp}-390-${m}`, 390, 'full', 2, m]);
for (const [t, m] of [['kitchen', 'light'], ['garden', 'dark'], ['terminal', 'light']])
  VIEWS.push(['themes.html', `?theme=${t}`, `modern-${t}-390-${m}`, 390, 'full', 2, m]);
VIEWS.push(['themes.html', '?theme=farm', 'modern-farm-1440-dark', 1440, 'full', 1, 'dark'],
  ['look.html', '?theme=farm&mode=system', 'look-modern-390-dark', 390, 'screen', 2, 'dark'],
  ['look.html', '?theme=garden&mode=system', 'look-modern-1440-light', 1440, 'screen', 1, 'light'],
  ['settings.html', '?theme=farm', 'settings-modern-390-dark', 390, 'sheet', 2, 'dark'],
  ['settings.html', '?theme=kitchen', 'settings-modern-1440-light', 1440, 'sheet', 1, 'light']);
const only = process.argv[3] ? new RegExp(process.argv[3]) : null;
const TYPES = { '.html': 'text/html', '.css': 'text/css', '.js': 'text/javascript', '.png': 'image/png', '.json': 'application/json', '.svg': 'image/svg+xml' };
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
    if ((own || input) && !(input ? fs === 16 : SIZES.includes(fs))) bad.push(`font ${fs}px · ${name(e)}`);
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
    : fit === 'sheet' ? await js(`Math.max(${h0}, document.querySelector('.sl').scrollHeight + ${w < 1000 ? 110 : 260})`) : h0;
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
