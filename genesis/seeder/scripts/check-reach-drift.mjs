#!/usr/bin/env node
// Fails (exit 1) if any seed content/path reach value is non-canonical
// (not present in the reach.schema.json enum). The canonical set is read from
// the schema JSON root of truth — no hand-rolled copy, no TS compilation needed.
import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { join } from 'node:path';

const REACH_SCHEMA = 'elohim/sdk/schemas/v1/enums/reach.schema.json';
const CONTENT_DIR = 'genesis/data/lamad/content';
const PATH_DIR = 'genesis/data/lamad/paths';
const ACCOUNT_PKG_DIR = 'genesis/data/account-packages';
// Non-package files that share the account-packages directory.
const ACCOUNT_PKG_SKIP = new Set([
  'account-package.schema.json',
  'conductor-groups.json',
  'index.json',
]);

const canonical = new Set(JSON.parse(readFileSync(REACH_SCHEMA, 'utf8')).enum);
const failures = [];

function check(value, file, field) {
  if (value == null) return; // absent reach is fine (inverted-burden default handles it)
  if (!canonical.has(value)) failures.push(`${file}: non-canonical ${field} "${value}"`);
}

for (const dir of [CONTENT_DIR, PATH_DIR]) {
  if (!existsSync(dir)) continue;
  for (const f of readdirSync(dir).filter((f) => f.endsWith('.json'))) {
    const j = JSON.parse(readFileSync(join(dir, f), 'utf8'));
    const nodes = Array.isArray(j) ? j : [j];
    for (const n of nodes) {
      if (n == null || typeof n !== 'object') continue;
      check(n.reach, f, 'reach');
      check(n.visibility, f, 'visibility'); // paths use visibility as authored reach
    }
  }
}

// Account packages: each file's `content[]` array carries the authored content
// reach (the visibility tier). `relationships[]` / `collectives[]` use the
// separate relationship/distance vocabulary and are intentionally NOT checked
// against the content-visibility enum here.
if (existsSync(ACCOUNT_PKG_DIR)) {
  for (const f of readdirSync(ACCOUNT_PKG_DIR).filter(
    (f) => f.endsWith('.json') && !ACCOUNT_PKG_SKIP.has(f),
  )) {
    const j = JSON.parse(readFileSync(join(ACCOUNT_PKG_DIR, f), 'utf8'));
    const items = Array.isArray(j.content) ? j.content : [];
    for (const item of items) {
      if (item == null || typeof item !== 'object') continue;
      check(item.reach, f, 'content[].reach');
    }
  }
}

if (failures.length) {
  console.error(
    `[reach-drift] FAILED — ${failures.length} non-canonical reach value(s):\n  ` +
      failures.join('\n  '),
  );
  process.exit(1);
}
console.log('[reach-drift] OK — all seed reach values are canonical.');
