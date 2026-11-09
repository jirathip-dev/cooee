// #116: Scene's vector horses for the mockup (cooee#115 brings them to the farm pack; until then, copied here):
// herdsman's `assets/themes/farm/sprites.svg` poses as symbols `h-<pose>`, coats left to CSS (--c --d --m --b).
// Usage (repo root): node docs/design/native-shell/horses.mjs <herdsman checkout>
import { readFileSync, writeFileSync } from "node:fs";

const src = readFileSync(`${process.argv[2]}/assets/themes/farm/sprites.svg`, "utf8").split("\n");
const out = ["working", "stand", "done", "blocked", "graze"].map((p) => {
  const g = src.find((l) => l.startsWith(`<g id="horse-${p}"`));
  if (!g) throw new Error(`no horse-${p}`);
  return `<symbol id="h-${p}" viewBox="0 0 132 100">${g.replace(/^<g id="[^"]+"/, "<g")}</symbol>`;
});
writeFileSync(new URL("horses.svg", import.meta.url),
  `<svg xmlns="http://www.w3.org/2000/svg"><!-- herdsman's vector horses (Corral, MIT); horses.mjs -->\n${out.join("\n")}\n</svg>\n`);
