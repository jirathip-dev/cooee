import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const [chrome] = process.argv.slice(2);
const out = 'apple/Sources/CooeeUI/Sprites', tmp = mkdtempSync(join(tmpdir(), 'spr'));
const sprites = readFileSync('packs/themes/farm/sprites.svg', 'utf8');
const COATS = {
  ch: '--c:#c89a56;--d:#a37a3c;--m:#e8dcc0;--b:#efe7dc', bay: '--c:#8a5a35;--d:#6b4426;--m:#2e231c;--b:#efe7dc',
  gry: '--c:#b9b3a8;--d:#958e83;--m:#ece8e0;--b:#f4f1ea', blk: '--c:#4a4038;--d:#3a322b;--m:#241e19;--b:#efe7dc',
  wht: '--c:#ece6da;--d:#cdc4b4;--m:#f8f5ef;--b:#fffdf8',
};
const POSES = ['edit', 'idle', 'done', 'needs', 'fail', 'merged'];
function shoot(html, w, h, file) {
  const p = join(tmp, 'p.html');
  writeFileSync(p, html);
  execFileSync(chrome, ['--headless', '--disable-gpu', '--hide-scrollbars', '--force-device-scale-factor=3',
    '--default-background-color=00000000', `--window-size=${w},${h}`, `--screenshot=${join(out, file)}`, `file://${p}`],
    { stdio: 'ignore' });
}
for (const [coat, vars] of Object.entries(COATS)) for (const pose of POSES) {
  const html = `<style>html,body{margin:0;background:transparent}svg{display:block}.k g[style]{${vars}!important}
  .k *{animation:none!important}</style><div style="display:none">${sprites}</div>
  <svg class=k width=132 height=100 viewBox="0 0 132 100"><use href="#s-scene-${pose}"/></svg>`;
  const m = sprites.match(new RegExp(`<symbol id="s-scene-${pose}" viewBox="0 0 132 100">([\\s\\S]*?)</symbol>`));
  const body = m[1].replace(/style="--c:[^"]*"/, `style="${vars};--fk:#58a6ff"`);
  shoot(`<style>html,body{margin:0;background:transparent}svg{display:block}*{animation:none!important}</style>
  <svg width=132 height=100 viewBox="0 0 132 100">${body}</svg>`, 132, 100, `horse-${coat}-${pose}.png`);
}
shoot(`<style>html,body{margin:0}svg{display:block;width:402px;height:300px}</style>${readFileSync('packs/themes/farm/scene.svg', 'utf8')}`,
  402, 300, 'scene-strip.png');
console.log('ok');
