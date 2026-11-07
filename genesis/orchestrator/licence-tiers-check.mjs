#!/usr/bin/env node
// Habit check for `licence-tiers-declared` (.epr-meta/licence-tiers-declared.habit.md).
// Walks every tracked first-party Cargo.toml with a [package] table and every tracked
// first-party package.json, and refuses: a missing licence field, a licence outside the
// allowed set, CAL-1.0 anywhere but the conductor fork, or a named tier member on the
// wrong tier. Reads the tree only; writes nothing. Exit 1 on any refusal.
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const ALLOWED = new Set(['Apache-2.0', 'MPL-2.0', 'AGPL-3.0', 'MIT OR Apache-2.0']);
const UPSTREAM = [/^sophia\//, /^elohim\/holochain-conductor\//, /^elohim\/brit\//, /^elohim\/rakia\//, /^vendor\//, /^genesis\/research\//, /^genesis\/a2o\/reports\//, /^genesis\/local-dev\//];
const TIERS = {
  'MPL-2.0': [
    'elohim/epr', 'elohim/epr-index', 'elohim/epr-rea', 'crates/elohim-address', 'crates/elohim-views', 'elohim/elohim-facings',
    'elohim/sdk/domains/avodah/types', 'elohim/sdk/domains/imagodei/types', 'elohim/sdk/domains/infrastructure/types',
    'elohim/sdk/domains/lamad/types', 'elohim/sdk/domains/qahal/types', 'elohim/sdk/domains/shefa/types',
  ],
  'AGPL-3.0': ['elohim/elohim-storage', 'doorway/doorway-service', 'steward/node'],
};
const PKG_TIERS = { 'elohim/sdk/epr-ts/package.json': 'MPL-2.0' }; // the TS twin of elohim-epr
const ZOME_DIR = /^elohim\/holochain\/dna\/[^/]+\/zomes\/[^/]+$/;

const tracked = execFileSync('git', ['ls-files', '--', '*Cargo.toml', '*package.json'], { cwd: ROOT, encoding: 'utf8' })
  .split('\n').filter(Boolean)
  .filter((p) => !p.includes('node_modules/') && !p.endsWith('ng-package.json') && !UPSTREAM.some((re) => re.test(p)))
  .filter((p) => p !== 'elohim/elohim-cache-core/pkg/package.json'); // wasm-pack output

const refusals = [];
let cargo = 0, pkg = 0;

function workspaceLicence(dir) {
  // walk up to the nearest Cargo.toml with [workspace.package] license
  let d = dir;
  while (d && d !== '.') {
    d = dirname(d);
    const p = resolve(ROOT, d === '.' ? '' : d, 'Cargo.toml');
    try {
      const s = readFileSync(p, 'utf8');
      const m = s.match(/^\[workspace\.package\][^[]*?^license\s*=\s*"([^"]+)"/ms);
      if (m) return m[1];
    } catch { /* keep walking */ }
  }
  return null;
}

for (const rel of tracked) {
  const s = readFileSync(resolve(ROOT, rel), 'utf8');
  if (rel.endsWith('Cargo.toml')) {
    if (!/^\[package\]\s*$/m.test(s)) continue;
    cargo += 1;
    const dir = dirname(rel);
    let lic = null;
    const m = s.match(/^\s*license\s*=\s*"([^"]+)"/m);
    if (m) lic = m[1];
    else if (/^\s*license\.workspace\s*=\s*true/m.test(s)) lic = workspaceLicence(dir) ?? '(workspace: unresolved)';
    if (!lic) { refusals.push(`${rel}: no licence field — add license = "<SPDX>" per LICENSING.md`); continue; }
    if (!ALLOWED.has(lic)) refusals.push(`${rel}: licence ${lic} is outside the allowed set ${[...ALLOWED].join(' | ')}`);
    for (const [tier, members] of Object.entries(TIERS)) {
      if (members.includes(dir) && lic !== tier) refusals.push(`${rel}: ${dir} is a ${tier} tier member but declares ${lic}`);
    }
    if (ZOME_DIR.test(dir) && lic !== 'MPL-2.0') refusals.push(`${rel}: DNA zomes are primitives (MPL-2.0); declares ${lic}`);
  } else {
    pkg += 1;
    let d;
    try { d = JSON.parse(s); } catch { refusals.push(`${rel}: unparseable JSON`); continue; }
    if (!d.license) refusals.push(`${rel}: no "license" field — add "license": "Apache-2.0" (or the tier's licence)`);
    else if (!ALLOWED.has(d.license)) refusals.push(`${rel}: licence ${d.license} is outside the allowed set`);
    else if (PKG_TIERS[rel] && d.license !== PKG_TIERS[rel]) refusals.push(`${rel}: is a ${PKG_TIERS[rel]} tier member but declares ${d.license}`);
  }
}

for (const f of ['LICENSE', 'LICENSES/Apache-2.0.txt', 'LICENSES/MPL-2.0.txt', 'LICENSES/AGPL-3.0.txt', 'LICENSES/CC-BY-SA-4.0.txt', 'genesis/docs/content/elohim-protocol/LICENSE', 'LICENSING.md', 'EPR-LICENSE-1.0.md', '.epr-meta/commons-held-by-the-network.habit.md']) {
  try { readFileSync(resolve(ROOT, f)); } catch { refusals.push(`${f}: missing`); }
}

console.log(`licence-tiers-check: ${cargo} Cargo.toml packages, ${pkg} package.json manifests, ${refusals.length} refusal(s)`);
for (const r of refusals) console.log(`REFUSED ${r}`);
process.exit(refusals.length ? 1 : 0);
