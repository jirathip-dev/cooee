import { spawn } from 'node:child_process';
import { mkdtempSync, readFileSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

export const CHROME = process.env.CHROME ||
  `${process.env.HOME}/Library/Caches/ms-playwright/chromium-1243/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing`;

export async function launch() {
  const dir = mkdtempSync(join(tmpdir(), 'cdp-'));
  const proc = spawn(CHROME, [...(process.env.HEADFUL ? ['--window-size=390,844', '--disable-backgrounding-occluded-windows', '--disable-renderer-backgrounding', '--disable-background-timer-throttling'] : ['--headless=new']), '--no-first-run', '--no-default-browser-check', '--hide-scrollbars',
    '--remote-debugging-port=0', `--user-data-dir=${dir}`, 'about:blank'], { stdio: 'ignore' });
  const f = join(dir, 'DevToolsActivePort');
  let port, path;
  for (let i = 0; i < 300 && !path; i++) {
    if (existsSync(f)) [port, path] = readFileSync(f, 'utf8').trim().split('\n');
    if (!path) await new Promise(r => setTimeout(r, 100));
  }
  const ws = new WebSocket(`ws://127.0.0.1:${port}${path}`);
  await new Promise(r => ws.addEventListener('open', r, { once: true }));
  let id = 0; const pending = new Map(); const listeners = [];
  ws.addEventListener('message', ({ data }) => {
    const m = JSON.parse(data);
    if (m.id && pending.has(m.id)) { const { ok, err } = pending.get(m.id); pending.delete(m.id); m.error ? err(new Error(m.error.message)) : ok(m.result); }
    else listeners.forEach(l => l(m));
  });
  const send = (method, params = {}, sessionId) => new Promise((ok, err) => {
    const i = ++id; pending.set(i, { ok, err }); ws.send(JSON.stringify({ id: i, method, params, sessionId }));
  });
  const { targetId } = await send('Target.createTarget', { url: 'about:blank' });
  const { sessionId } = await send('Target.attachToTarget', { targetId, flatten: true });
  const page = {
    send: (m, p) => send(m, p, sessionId),
    on: (method, fn) => listeners.push(m => m.sessionId === sessionId && m.method === method && fn(m.params)),
    eval: async (expression) => {
      const r = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }, sessionId);
      if (r.exceptionDetails) throw new Error(r.exceptionDetails.exception?.description || r.exceptionDetails.text);
      return r.result.value;
    },
    async goto(url) {
      await this.send('Page.enable'); await this.send('Runtime.enable');
      const loaded = new Promise(r => this.on('Page.loadEventFired', r));
      await this.send('Page.navigate', { url }); await loaded;
    },
    async waitFor(expression, timeout = 30000) {
      const t0 = Date.now();
      while (Date.now() - t0 < timeout) { if (await this.eval(expression)) return; await new Promise(r => setTimeout(r, 50)); }
      throw new Error(`timeout waiting for ${expression}`);
    },
  };
  await page.send('Emulation.setDeviceMetricsOverride', { width: 390, height: 844, deviceScaleFactor: 2, mobile: true });
  return { page, close: () => { ws.close(); proc.kill(); } };
}
