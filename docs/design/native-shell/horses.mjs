// #116: Scene's vector horses for the mockup (cooee#115 brings them to the farm pack; until then, copied here):
// herdsman's `assets/themes/farm/sprites.svg` poses as symbols `h-<pose>`, coats left to CSS (--c --d --m --b, --fk
// the tack). Revision 1 (q902): each state acts its theme word: on the trail ploughs, at the gate rears at the gate,
// in the stable stands in it, grazing grazes, refused refuses the fence, over the fence jumps it.
// Usage (repo root): node docs/design/native-shell/horses.mjs <herdsman checkout>
import { readFileSync, writeFileSync } from "node:fs";

const src = readFileSync(`${process.argv[2]}/assets/themes/farm/sprites.svg`, "utf8").split("\n");
const out = ["horse-plough", "horse-rear", "horse-stable", "horse-graze", "refuse-plant", "jump-over"].map((id) => {
  const g = src.find((l) => l.startsWith(`<g id="${id}"`));
  if (!g) throw new Error(`no ${id}`);
  const p = id.replace(/^horse-/, "");
  return `<symbol id="h-${p}" viewBox="0 0 132 100">${g.replace(/^<g id="[^"]+"/, "<g")}</symbol>`;
});
writeFileSync(new URL("horses.svg", import.meta.url),
  `<svg xmlns="http://www.w3.org/2000/svg"><!-- herdsman's vector horses (Corral, MIT); horses.mjs -->\n${out.join("\n")}\n</svg>\n`);
