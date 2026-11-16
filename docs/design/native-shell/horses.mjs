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
