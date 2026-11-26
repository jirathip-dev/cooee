// Page helpers: ?mode=dark|light, ?f=<frame> shows one frame, live figures get a stable phase from their id,
// setTime(s) freezes every live figure at s seconds (the clip's frames).
(() => {
  const q = new URLSearchParams(location.search);
  document.documentElement.className = q.get('mode') || 'dark';
  const STRIP = {
    idle: [0, 0.8, 1.6, 2.4, 3.2, 3.72], working: [0, 0.27, 0.53, 0.8, 1.07, 1.33], done: [0, 0.14, 0.28, 0.42, 0.6, 0.7],
    needs: [0, 0.16, 0.32, 0.48, 0.64, 1.2], failed: [0, 0.8, 1.6, 2.4, 3.2, 3.6],
  };
  const KEY = { idle: 0, working: 0.4, done: 0.42, needs: 0.16, failed: 0 };
  const live = (ch, state, salt = '') => Cast.svg(ch, state, (Cast.hash(ch.id + salt) % 3500) / 1000);
  const ascii = [];
  const tc = (a, state, salt = '', fixed) => {
    const fr = Cast.asciiFrames(a, state), el = document.createElement('span');
    el.className = `tc ${state}`; el.style.setProperty('--tc', `var(--${a.c})`);
    el.textContent = fr[fixed ?? 0][0];
    if (fixed != null) return el.outerHTML;
    ascii.push({ el, fr, off: Cast.hash(a.id + salt) % 3000 });
    return el.outerHTML.replace('<span', `<span data-ascii="${ascii.length - 1}"`);
  };
  const frameAt = (fr, ms) => {
    const tot = fr.reduce((s, f) => s + f[1], 0);
    let t = ms % tot;
    for (const f of fr) { if (t < f[1]) return f[0]; t -= f[1]; }
    return fr[0][0];
  };
  const tick = ms => document.querySelectorAll('[data-ascii]').forEach(e => {
    const a = ascii[+e.dataset.ascii]; if (a) e.textContent = frameAt(a.fr, ms + a.off);
  });
  let t0 = performance.now(), frozen = false;
  const loop = () => { if (!frozen) tick(performance.now() - t0); requestAnimationFrame(loop); };
  window.setTime = s => {
    frozen = true; document.body.classList.add('still');
    document.querySelectorAll('.live .ch, .ph .ch, .mac .ch').forEach(e => e.getAnimations({ subtree: true }).forEach(a => {
      a.pause(); a.currentTime = s * 1000;
    }));
    document.querySelectorAll('.paused .ch').forEach(e => e.getAnimations({ subtree: true }).forEach(a => {
      a.pause(); a.currentTime = 0;
    }));
    tick(s * 1000);
  };
  window.Page = { q, STRIP, KEY, live, tc, ready: () => { requestAnimationFrame(loop); document.body.dataset.ready = 1; } };
})();
