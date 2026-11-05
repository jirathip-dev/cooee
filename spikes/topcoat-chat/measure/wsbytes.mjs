// Bytes the browser receives over the WebSocket for one full reply. Usage: node measure/wsbytes.mjs <base url>
import { launch } from './cdp.mjs';
const B = process.argv[2];
const { page, close } = await launch();
await page.send('Network.enable');
let frames = 0, bytes = 0;
page.on('Network.webSocketFrameReceived', p => { frames++; bytes += Buffer.byteLength(p.response.payloadData); });
await fetch(`${B}/api/start`, { method: 'POST' });
await page.goto(`${B}/live`);
await page.waitFor("!!document.getElementById('done')", 300000);
const t = await page.eval("document.getElementById('reply').textContent");
const text = t.length, exact = t === Array.from({ length: 2000 }, (_, i) => `w${i} `).join('');
console.log(JSON.stringify({ frames, ws_bytes: bytes, reply_chars: text, exact }));
close();
