import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { formatTsOrdinal } from '../codegen-ts.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '../../../../../');
const GENERATED_ENUMS = resolve(REPO_ROOT, 'genesis/seeder/src/generated/schema-enums.ts');

test('formatTsOrdinal emits a Record + openness fn matching schema order', () => {
  const out = formatTsOrdinal('REACH_LEVELS', 'Reach',
    ['private', 'self', 'intimate', 'trusted', 'familiar', 'community', 'public', 'commons']);
  assert.match(out, /export const REACH_OPENNESS: Record<Reach, number> = \{/);
  assert.match(out, /private: 1/);
  assert.match(out, /commons: 8/);
  assert.match(out, /export function reachOpenness\(r: Reach\): number \{ return REACH_OPENNESS\[r\]; \}/);
  assert.match(out, /export function isReach\(v: string\): v is Reach \{/);
});

// Ordinal codegen is gated on the schema's explicit `_ordinal: true` marker, NOT a
// name heuristic. reach.schema.json carries the marker (true monotonic ordinal);
// mastery-level.schema.json does NOT (its legacy alias values recognize/recall/
// synthesize make positional ordinals semantically wrong). The generated file must
// therefore contain REACH_OPENNESS but never MASTERY_OPENNESS.
test('generated schema-enums emits REACH_OPENNESS but not MASTERY_OPENNESS', async () => {
  const generated = await readFile(GENERATED_ENUMS, 'utf8');
  assert.ok(
    generated.includes('REACH_OPENNESS'),
    'expected REACH_OPENNESS in generated schema-enums.ts (reach has _ordinal: true)',
  );
  assert.ok(
    !generated.includes('MASTERY_OPENNESS'),
    'MASTERY_OPENNESS must NOT be generated (mastery-level lacks _ordinal; alias values break positional ordinals)',
  );
});
