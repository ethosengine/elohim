#!/usr/bin/env node
// Copy ts-rs generated wire types from domain crates into storage-client-ts
//
// Usage: node elohim/sdk/storage-client-ts/scripts/copy-wire-types.mjs
//
// Reads from: elohim/sdk/domains/{domain}/types/bindings/*.ts
// Writes to:  elohim/sdk/storage-client-ts/src/wire-types/{domain}/

import { cpSync, mkdirSync, readdirSync, writeFileSync } from 'fs';
import { join, resolve, basename } from 'path';

const SCRIPT_DIR = import.meta.dirname;
const ROOT = resolve(SCRIPT_DIR, '../../../..');
const DOMAINS = ['imagodei', 'infrastructure', 'qahal', 'lamad', 'shefa', 'avodah'];
const OUT = join(SCRIPT_DIR, '..', 'src', 'wire-types');

for (const domain of DOMAINS) {
  const bindingsDir = join(ROOT, 'elohim/sdk/domains', domain, 'types/bindings');
  const destDir = join(OUT, domain);
  mkdirSync(destDir, { recursive: true });

  let files;
  try {
    files = readdirSync(bindingsDir).filter(f => f.endsWith('.ts'));
  } catch (e) {
    console.warn(`Warning: no bindings found for ${domain} at ${bindingsDir}`);
    continue;
  }

  for (const file of files) {
    cpSync(join(bindingsDir, file), join(destDir, file));
  }

  // Generate per-domain index.ts
  const exports = files.map(f => {
    const name = basename(f, '.ts');
    return `export type { ${name} } from './${name}';`;
  });
  writeFileSync(
    join(destDir, 'index.ts'),
    `// Generated from elohim/sdk/domains/${domain}/types via ts-rs. Do not hand-edit.\n\n` +
      exports.join('\n') +
      '\n',
  );

  console.log(`  ${domain}: ${files.length} types`);
}

// Generate top-level wire-types/index.ts
const topExports = DOMAINS.map(d => `export * as ${d} from './${d}';`);
writeFileSync(
  join(OUT, 'index.ts'),
  '// Domain wire types — coordinator I/O (snake_case, MessagePack format)\n' +
    '// Generated from Rust crates via ts-rs. Do not hand-edit.\n\n' +
    topExports.join('\n') +
    '\n',
);

console.log(`\nCopied wire types for ${DOMAINS.length} domains to ${OUT}`);
