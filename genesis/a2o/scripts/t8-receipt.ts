/**
 * Join completed T8 halves: pnpm exec tsx scripts/t8-receipt.ts <run id>
 * Run from genesis/a2o. Halves may arrive on one checkout by git AFTER both runs:
 * they are evidence, never coordination records. The running halves never read these files.
 * Reports are gitignored; deliberate evidence transfer must include the two JSON files.
 * Missing/invalid halves never print PASS. Run ids are URL-encoded as directory components.
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import { joinHalves, runDirectory, type T8Half } from './lib/t8-receipt.js';

try {
  const runId = process.argv[2];
  if (!runId || process.argv.length !== 3) throw new Error('Usage: t8-receipt.ts <run id>');
  const directory = join('reports/recovery/devices-share-one-berth', runDirectory(runId));
  const read = (role: string) =>
    JSON.parse(readFileSync(join(directory, `${role}.json`), 'utf8')) as T8Half;
  const receipt = joinHalves(read('holder'), read('contender'));
  if (receipt.runId !== runId) throw new Error('receipt run id differs from requested directory');
  const path = join(directory, 'receipt.json');
  writeFileSync(path, `${JSON.stringify(receipt, null, 2)}\n`);
  console.log(`PASS ${path}`);
} catch (error) {
  console.error(`FAIL ${error instanceof Error ? error.message : String(error)}`);
  process.exitCode = 1;
}
