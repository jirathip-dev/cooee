// #7: herdsman's scheme gate (gate.mjs, serve.js's line for line) over every scheme cooee ships. `node check.mjs`
// prints PASS/FAIL and each check within 12 % of its minimum; exit 1 on a fail.
import { readdirSync, readFileSync } from 'node:fs';
import { gate } from './gate.mjs';

const dir = new URL('schemes/', import.meta.url);
let bad = 0;
for (const f of readdirSync(dir).filter(f => f.endsWith('.json')).sort()) {
  const rows = gate(JSON.parse(readFileSync(new URL(f, dir)))), ok = rows.every(([, v, m]) => v >= m);
  bad += !ok;
  console.log(`${ok ? 'PASS' : 'FAIL'} ${f.slice(0, -5)}`);
  for (const [w, v, m] of rows) if (v < m || v / m < 1.12) console.log(`  ${v < m ? '✗' : '·'} ${w} ${v.toFixed(2)} (min ${m})`);
}
process.exit(bad ? 1 : 0);
