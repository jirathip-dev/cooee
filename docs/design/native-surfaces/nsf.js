// #234 mockup: native iOS system surfaces and voice. It runs rooms' rm.js first (the scheme, the style, the phone
// chrome, the personas' avatars in the three styles), then draws the Dynamic Island in the theme's dark scheme: the
// island is always black, so its text takes the dark scheme's tokens in either mode, as iOS does.
import '../rooms/rm.js';

const q = new URLSearchParams(location.search);
const theme = await fetch(`../../../packs/themes/${q.get('theme') || 'farm'}/theme.json`).then(r => r.json());
const dark = await fetch(`../../../web/schemes/${theme.scheme.dark}.json`).then(r => r.json());
for (const e of document.querySelectorAll('.di,.lks'))
  for (const [k, v] of Object.entries(dark.tokens)) e.style.setProperty(k, v);
document.body.dataset.ready = 2;
