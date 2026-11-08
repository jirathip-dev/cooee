// #48: shoot the mockup's pages and check the shared size scale. No deps: Chrome's --remote-debugging-pipe, as
// docs/design/v1/shoot.mjs. It serves the repo root (so the pages load web/src/app.css, web/src/look.js and the
// themes as they are), with /look/<theme>/<file> mapped to web/themes/<theme>/<file> as cooee serves it.
// Usage (repo root):  node docs/design/new-thread/shoot.mjs <chrome-headless-shell> [name regexp]
//                     node docs/design/new-thread/shoot.mjs --serve   (prints the URL; open the pages yourself)
// Every screen is checked, at 390 and 1440, in each look below: no sideways scroll; every chip, button and one-line
// input in the composer, the sheets and Settings › providers is --ctl tall with the --r-ctl radius and --fs text
// (inputs 16 px); every sheet and Settings row is 44 px; a cut line is ellipsised; at 390 the composer screen fits
// one 844 px screen. Text an ellipsised ancestor cuts (the thread bar's line) may run past the edge. From 1024 px the
// sidebar shows (320 px; [+ new thread], folders and rows --ctl tall) and the header's [+] doesn't; below it, the
// reverse. No dev text anywhere: no agent id (`…-acp`), no `System default`. Prints one line per shot, exits 1 on a failure.
import { spawn } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { extname, join } from 'node:path';

const here = new URL('.', import.meta.url).pathname, root = join(here, '../../..');
const [chrome, only] = process.argv.slice(2);
const REV = 'r2'; // in every PNG name: a revised shot gets a new URL, so no viewer shows a cached older round
const TYPES = { '.html': 'text/html', '.css': 'text/css', '.js': 'text/javascript', '.json': 'application/json', '.svg': 'image/svg+xml', '.png': 'image/png' };
const srv = createServer((q, r) => {
  try {
    const path = decodeURIComponent(new URL(q.url, 'http://x').pathname).replace(/^\/look\//, '/web/themes/');
    const p = join(root, path);
    r.writeHead(200, { 'content-type': TYPES[extname(p)] || 'application/octet-stream' }).end(readFileSync(p));
  } catch { r.writeHead(404).end(); }
}).listen(0, '127.0.0.1');
await new Promise(ok => srv.on('listening', ok));
const base = `http://127.0.0.1:${srv.address().port}/docs/design/new-thread/`;
if (chrome === '--serve') { console.log(`${base}composer.html · chat.html · providers.html (?theme=…&mode=…&style=…&sprites=…)`); await new Promise(() => {}); }

const PAGES = { composer: ['composer', 'profile', 'folder', 'profiled'], chat: ['connecting', 'failed'], providers: ['providers'] };
// [file suffix, query]: farm in Modern, dark, is the main pair; then each other theme once, light and dark mixed, and
// terminal in its own style.
const LOOKS = [
  ['', 'theme=farm&mode=dark'],
  ['-kitchen-light', 'theme=kitchen&mode=light'],
  ['-garden-dark', 'theme=garden&mode=dark'],
  ['-terminal-light', 'theme=terminal&mode=light&style=terminal'],
];

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

// Runs in one screen: every rule above, as a list of "what · where" strings.
function audit(screen) {
  const bad = [], W = innerWidth, cs = getComputedStyle(document.documentElement);
  const px = k => parseFloat(cs.getPropertyValue(k)), CTL = px('--ctl'), RC = px('--r-ctl'), FS = px('--fs'), ROW = px('--row');
  const name = e => e.tagName.toLowerCase() + (typeof e.className === 'string' && e.className ? '.' + e.className.trim().split(/\s+/).join('.') : '') + ` "${(e.textContent || e.placeholder || '').trim().slice(0, 24)}"`;
  const shown = e => e.getClientRects().length && getComputedStyle(e).visibility !== 'hidden';
  if (document.documentElement.scrollWidth > W) bad.push(`sideways scroll ${document.documentElement.scrollWidth} > ${W}`);
  if (screen === 'composer' && W < 1000 && document.documentElement.scrollHeight > innerHeight) bad.push(`composer scrolls: ${document.documentElement.scrollHeight} > ${innerHeight}`);
  const near = (a, b) => Math.abs(a - b) < 0.6;
  const dev = document.body.innerText.match(/\b[\w-]+-acp\b|system default/i);
  if (dev) bad.push(`dev text "${dev[0]}"`);
  const side = document.querySelector('.side'), plus = document.querySelector('.bar .np');
  if (side && W >= 1024 !== Boolean(shown(side))) bad.push(`sidebar ${shown(side) ? 'shown' : 'hidden'} at ${W}`);
  if (side && W >= 1024 && !near(side.getBoundingClientRect().width, 320)) bad.push(`sidebar ${side.getBoundingClientRect().width}px, not 320`);
  if (plus && W >= 1024 === Boolean(shown(plus))) bad.push(`header [+] ${shown(plus) ? 'shown' : 'hidden'} at ${W}`);
  for (const e of document.querySelectorAll('body *')) {
    if (!shown(e)) continue;
    const r = e.getBoundingClientRect(), s = getComputedStyle(e);
    let clip = false;
    for (let a = e.parentElement; a && !clip; a = a.parentElement) clip = getComputedStyle(a).textOverflow === 'ellipsis';
    if (r.right > W + 0.5 && !clip) bad.push(`past the right edge ${Math.round(r.right)} · ${name(e)}`);
    const own = [...e.childNodes].some(n => n.nodeType === 3 && n.textContent.trim());
    if (own && e.scrollWidth > e.clientWidth + 1 && s.overflowX !== 'visible' && s.textOverflow !== 'ellipsis' && !e.matches('pre,textarea,input')) bad.push(`text cut without an ellipsis · ${name(e)}`);
    // The scale: controls, then rows.
    if (e.matches('.chip, :is(.nc,.pop,.acr) button:not(.sr,.gl), .pop input, .nc textarea')) {
      if (!near(r.height, CTL)) bad.push(`control ${r.height}px, not --ctl ${CTL} · ${name(e)}`);
      if (!near(parseFloat(s.borderTopLeftRadius), RC)) bad.push(`radius ${s.borderTopLeftRadius}, not --r-ctl ${RC} · ${name(e)}`);
      const want = e.matches('input,textarea') ? 16 : FS;
      if (!near(parseFloat(s.fontSize), want)) bad.push(`text ${s.fontSize}, not ${want} · ${name(e)}`);
    }
    if (e.matches('.side :is(.nw, .sb, summary)') && !near(r.height, CTL)) bad.push(`sidebar row ${r.height}px, not --ctl ${CTL} · ${name(e)}`);
    if (e.matches('.pk, .acr') && !near(r.height, ROW)) bad.push(`row ${r.height}px, not --row ${ROW} · ${name(e)}`);
    if (e.matches('.pk .sk, .acr .sk') && !near(parseFloat(s.fontSize), FS)) bad.push(`row text ${s.fontSize}, not --fs ${FS} · ${name(e)}`);
  }
  return bad;
}

const { targetId } = await send('Target.createTarget', { url: 'about:blank' });
const { sessionId: s } = await send('Target.attachToTarget', { targetId, flatten: true });
const cmd = (m, p) => send(m, p, s);
const js = async x => (await cmd('Runtime.evaluate', { expression: x, returnByValue: true, awaitPromise: true })).result.value;
const until = async x => { for (let i = 0; i < 50 && !(await js(x)); i++) await sleep(100); };
await cmd('Page.enable');
await cmd('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: 'reduce' }] });
let fail = 0;
for (const [page, screens] of Object.entries(PAGES)) for (const [suffix, look] of LOOKS) for (const [w, dpr] of [[390, 2], [1440, 1]]) {
  const out = `${page}-${w}${suffix}-${REV}.png`;
  if (only && !new RegExp(only).test(out)) continue;
  if (suffix && w === 1440 && `${page}${suffix}` !== 'chat-terminal-light') continue; // the other looks at 390 only, but the Terminal sidebar once
  const h0 = w < 1000 ? 844 : 900;
  await cmd('Emulation.setDeviceMetricsOverride', { width: w, height: h0, deviceScaleFactor: dpr, mobile: w < 1000 });
  const bad = [];
  for (const sc of screens) {
    await cmd('Page.navigate', { url: `${base}${page}.html?${look}&s=${sc}` });
    await until(`location.search.endsWith("s=${sc}") && document.readyState === "complete"`);
    await sleep(200);
    bad.push(...(await js(`(${audit})(${JSON.stringify(sc)})`)).map(b => `${sc}: ${b}`));
  }
  await cmd('Page.navigate', { url: `${base}${page}.html?${look}` });
  await until('!location.search.includes("s=") && document.body?.dataset.ready === "1"');
  const h = await js('document.documentElement.scrollHeight');
  await cmd('Emulation.setDeviceMetricsOverride', { width: w, height: h, deviceScaleFactor: dpr, mobile: w < 1000 });
  await sleep(300);
  const { data } = await cmd('Page.captureScreenshot', { format: 'png' });
  writeFileSync(join(here, out), Buffer.from(data, 'base64'));
  console.log(`${bad.length ? 'FAIL' : 'ok  '} ${out} ${w}×${h}${bad.length ? '\n  ' + bad.join('\n  ') : ''}`);
  fail += bad.length;
}
c.kill(); srv.close();
process.exit(fail ? 1 : 0);
