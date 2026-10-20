// Select public inputs without changing geometry or garment rules.
// Usage: node prepare-wasm-bench.mjs <development-40.jsonl> <output.json>
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
const marker = fileURLToPath(new URL('../../../', import.meta.url));
const [development, output] = process.argv.slice(2);
if (!development || !output) throw new Error('Pass the frozen development JSONL and output JSON paths.');
const cases = ['simon', 'teagan', 'titan'].map(name => ({
  id: `freesewing-${name}-M`,
  job: JSON.parse(readFileSync(resolve(marker, `testdata/open/freesewing/${name}-M.job.json`), 'utf8')),
}));
const rows = readFileSync(development, 'utf8').trim().split(/\r?\n/).slice(0, 10).map(line => JSON.parse(line));
if (rows.length !== 10) throw new Error('Expected ten development tasks.');
for (const row of rows) cases.push({ id: `lectra-${row.id}`, job: row.job });
mkdirSync(dirname(resolve(output)), { recursive: true });
writeFileSync(output, JSON.stringify(cases));
console.log(cases.map(c => c.id).join('\n'));
