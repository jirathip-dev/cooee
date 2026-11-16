import { launch } from './cdp.mjs';
import { proxy } from './proxy.mjs';
import { writeFileSync, mkdirSync } from 'node:fs';
import { loadavg, cpus } from 'node:os';

const [B, OUT = '/tmp/shots'] = process.argv.slice(2);
mkdirSync(OUT, { recursive: true });
const sleep = ms => new Promise(r => setTimeout(r, ms));
const N = 2000;
const EXPECTED = Array.from({ length: N }, (_, i) => `w${i} `).join('');
const start = () => fetch(`${B}/api/start`, { method: 'POST' });
const shot = async (page, name) => {
  const { data } = await page.send('Page.captureScreenshot', { format: 'png' });
  writeFileSync(`${OUT}/${name}.png`, Buffer.from(data, 'base64'));
};
const WATCH = `(() => {
  const E = ${JSON.stringify(EXPECTED)};
  window.__bad = []; window.__renders = 0;
  new MutationObserver(() => {
    const r = document.getElementById('reply'); if (!r) return;
    const t = r.textContent; window.__renders++;
    if (!E.startsWith(t)) window.__bad.push(t.slice(-40));
  }).observe(document.querySelector('.msg.agent'), { subtree: true, childList: true, characterData: true });
})()`;
const EVENTS = `(() => {
  window.__ev = []; window.__lt = [];
  new PerformanceObserver(l => l.getEntries().forEach(e => __ev.push({ name: e.name, d: e.duration, id: e.interactionId })))
    .observe({ type: 'event', durationThreshold: 16, buffered: true });
  new PerformanceObserver(l => l.getEntries().forEach(e => __lt.push(e.duration))).observe({ type: 'longtask', buffered: true });
})()`;
const pct = (a, p) => a.length ? [...a].sort((x, y) => x - y)[Math.min(a.length - 1, Math.floor(a.length * p))] : 0;
const res = { host_load_at_start: loadavg().map(x => +x.toFixed(1)), cpus: cpus().length };

async function typing(page, stop, name) {
  await page.eval("document.getElementById('composer').focus()");
  await page.send('Tracing.start', { categories: 'devtools.timeline,disabled-by-default-devtools.timeline', transferMode: 'ReturnAsStream' });
  const t0 = Date.now(); let typed = 0;
  while (!(await stop()) && Date.now() - t0 < 120000) {
    const ch = 'abcdefghij'[typed % 10];
    await page.send('Input.dispatchKeyEvent', { type: 'keyDown', key: ch, text: ch, code: `Key${ch.toUpperCase()}` });
    await page.send('Input.dispatchKeyEvent', { type: 'keyUp', key: ch, code: `Key${ch.toUpperCase()}` });
    typed++; await sleep(100);
  }
  await sleep(500);
  const tracingDone = new Promise(r => page.on('Tracing.tracingComplete', r));
  await page.send('Tracing.end'); const { stream } = await tracingDone;
  let trace = ''; for (;;) { const c = await page.send('IO.read', { handle: stream }); trace += c.data; if (c.eof) break; }
  writeFileSync(`${OUT}/${name}-trace.json`, trace);
  const ev = JSON.parse(trace).traceEvents;
  const toCommit = ev.filter(e => e.name === 'EventTiming' && e.ph === 'b' && e.args.data.type === 'keydown' && e.args.data.commitFinishTime)
    .map(e => e.args.data.commitFinishTime - e.args.data.timeStamp);
  const tasks = ev.filter(e => e.name === 'RunTask' && e.ph === 'X').map(e => e.dur / 1000);
  const inter = new Map();
  for (const e of ev) if (e.name === 'EventTiming' && e.ph === 'b' && e.args.data.interactionId)
    inter.set(e.args.data.interactionId, Math.max(inter.get(e.args.data.interactionId) || 0, e.args.data.duration));
  const pres = [...inter.values()];
  const value = await page.eval("document.getElementById('composer').value");
  return { seconds: (Date.now() - t0) / 1000, typed, composer_intact: value.length === typed,
    key_to_commit_p50_ms: pct(toCommit, 0.5), key_to_commit_p95_ms: pct(toCommit, 0.95), key_to_commit_max_ms: Math.max(0, ...toCommit),
    keys_over_50ms: toCommit.filter(d => d > 50).length,
    trace_tasks_over_50ms: tasks.filter(d => d > 50).length, trace_task_max_ms: Math.max(0, ...tasks.slice(-100000)),
    key_presentation_p95_ms: pct(pres, 0.95), key_presentation_max_ms: Math.max(0, ...pres) };
}

console.error(new Date().toISOString(), "phase 0");
{
  const { page, close } = await launch();
  await page.send('Emulation.setCPUThrottlingRate', { rate: 4 });
  await page.goto(`${B}/long`); await sleep(300);
  const t0 = Date.now();
  res.control_typing_no_stream = await typing(page, async () => Date.now() - t0 > 30000, 'control');
  close();
}

console.error(new Date().toISOString(), "phase 1");
{
  const { page, close } = await launch();
  await page.send('Emulation.setCPUThrottlingRate', { rate: 4 });
  await start(); await page.goto(`${B}/live`); await sleep(300);
  await page.eval(WATCH);
  const t = await typing(page, () => page.eval("!!document.getElementById('done')"), 'stream');
  const s = await page.eval(`({ text: document.getElementById('reply').textContent, bad: __bad, renders: __renders })`);
  res.streaming = { tokens: s.text.split(' ').length - 1, exact: s.text === EXPECTED, bad_renders: s.bad.length, renders: s.renders, ...t };
  close();
}

console.error(new Date().toISOString(), "phase 2");
{
  const port = +new URL(B).port + 1000;
  const px = await proxy(port, +new URL(B).port);
  const { page, close } = await launch();
  await start(); await page.goto(`http://127.0.0.1:${port}/live`); await sleep(300);
  await page.eval(WATCH);
  await page.waitFor("((document.getElementById('reply')?.textContent ?? '').split(' ').length - 1) >= 500", 60000);
  const before = await page.eval("(document.getElementById('reply').textContent.split(' ').length - 1)");
  px.cut(); const cutAt = Date.now(); await sleep(10000); px.restore();
  const restoredAt = Date.now();
  await page.waitFor(`(document.getElementById('reply').textContent.split(' ').length - 1) > ${before + 500}`, 60000);
  const recovered = Date.now() - restoredAt;
  await shot(page, 'live-after-reconnect');
  await page.waitFor("!!document.getElementById('done')", 120000);
  const s = await page.eval(`({ text: document.getElementById('reply').textContent, bad: __bad, renders: __renders })`);
  res.reconnect = { cut_at_token: before, cut_seconds: (restoredAt - cutAt) / 1000, recovered_ms_after_restore: recovered,
    exact: s.text === EXPECTED, bad_renders: s.bad.length, renders: s.renders };
  close(); px.close();
}

console.error(new Date().toISOString(), "phase 3");
{
  const { page, close } = await launch();
  await page.send('Emulation.setCPUThrottlingRate', { rate: 4 });
  await page.goto(`${B}/long`); await sleep(1000);
  await page.eval("window.scrollTo(0, 0)"); await sleep(300);
  await page.eval(EVENTS); await sleep(300);
  await page.eval("__lt.length = 0");
  const h = await page.eval("document.documentElement.scrollHeight");
  const t0 = Date.now();
  await page.send('Input.synthesizeScrollGesture', { x: 195, y: 500, yDistance: -h, speed: 3000, gestureSourceType: 'touch', repeatCount: 0 });
  const s = await page.eval(`({ lt: __lt, y: scrollY, msgs: document.querySelectorAll('#msgs .msg').length })`);
  await page.eval("window.scrollTo(0, 0)"); await sleep(300);
  await shot(page, 'long-thread-390');
  res.long_thread = { messages: s.msgs, scroll_px: h, scrolled_to: s.y, scroll_ms: Date.now() - t0, long_tasks: s.lt.length, long_task_max_ms: Math.max(0, ...s.lt) };
  close();
}

console.error(new Date().toISOString(), "phase 4");
{
  const { page, close } = await launch();
  await start(); await page.goto(`${B}/live`);
  await page.waitFor("!!document.querySelector('#approval button')", 60000);
  await page.eval("document.getElementById('approval').scrollIntoView({ block: 'center' })"); await sleep(200);
  await shot(page, 'live-approval-390');
  const ms = await page.eval(`new Promise(ok => {
    const t0 = performance.now();
    new MutationObserver((_, o) => { if (document.getElementById('chosen')) { o.disconnect(); requestAnimationFrame(() => ok(performance.now() - t0)); } })
      .observe(document.body, { subtree: true, childList: true });
    document.querySelector('#approval button').click();
  })`);
  res.approval = { tap_to_row_ms: ms, chosen: await page.eval("document.getElementById('chosen').textContent") };
  await shot(page, 'live-approved-390');
  close();
}

console.error(new Date().toISOString(), "phase 5");
{
  const THAI = 'สวัสดีครับ ที่นี่';
  const { page, close } = await launch();
  await start(); await page.goto(`${B}/live`); await sleep(1000);
  await page.eval(`(() => { const c = document.getElementById('composer'); window.__ce = 0;
    c.addEventListener('compositionend', () => __ce++); c.focus(); c.value = ''; })()`);
  for (const ch of THAI) { await page.send('Input.insertText', { text: ch }); await sleep(30); }
  const a = await page.eval("document.getElementById('composer').value");
  await page.eval("(() => { const c = document.getElementById('composer'); c.select(); })()");
  await page.send('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Backspace', code: 'Backspace', windowsVirtualKeyCode: 8 });
  await page.send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Backspace', code: 'Backspace', windowsVirtualKeyCode: 8 });
  await sleep(100);
  for (const word of THAI.split(' ')) {
    let pre = '';
    for (const ch of word) { pre += ch; await page.send('Input.imeSetComposition', { text: pre, selectionStart: pre.length, selectionEnd: pre.length }); await sleep(30); }
    await page.send('Input.insertText', { text: word });
    if (!THAI.endsWith(word)) await page.send('Input.insertText', { text: ' ' });
  }
  await sleep(200);
  const b = await page.eval("({ v: document.getElementById('composer').value, ce: __ce })");
  await shot(page, 'composer-thai-390');
  await page.eval("document.getElementById('send').click()");
  await page.waitFor("[...document.querySelectorAll('.msg.you')].some(m => m.textContent === " + JSON.stringify(THAI) + ")", 10000);
  const after = await page.waitFor("document.getElementById('composer').value === ''", 5000).then(() => '', () => 'not cleared');
  res.thai_chrome = { keystrokes_exact: a === THAI, keystrokes_value: a, ime_exact: b.v === THAI, ime_value: b.v,
    compositionend_events: b.ce, words: THAI.split(' ').length, sent_exact: true, composer_cleared: after === '' };
  close();
}

console.error(new Date().toISOString(), "phase 6");
{
  const { page, close } = await launch();
  await page.goto(`${B}/`); await sleep(700); await shot(page, 'threads-390');
  await page.goto(`${B}/live`); await sleep(1500); await shot(page, 'live-390');
  const f = [];
  for (let i = 0; i < 4; i++) { f.push(await page.eval("getComputedStyle(document.querySelector('.rsp')).getPropertyValue('--o1')")); await sleep(300); }
  res.theme = { sprite_frames: f };
  close();
}
res.host_load_at_end = loadavg().map(x => +x.toFixed(1));
console.log(JSON.stringify(res, null, 1));
