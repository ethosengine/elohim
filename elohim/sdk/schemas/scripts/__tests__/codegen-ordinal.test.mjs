import { test } from 'node:test';
import assert from 'node:assert/strict';
import { formatTsOrdinal } from '../codegen-ts.mjs';

test('formatTsOrdinal emits a Record + openness fn matching schema order', () => {
  const out = formatTsOrdinal('REACH_LEVELS', 'Reach',
    ['private', 'self', 'intimate', 'trusted', 'familiar', 'community', 'public', 'commons']);
  assert.match(out, /export const REACH_OPENNESS: Record<Reach, number> = \{/);
  assert.match(out, /private: 1/);
  assert.match(out, /commons: 8/);
  assert.match(out, /export function reachOpenness\(r: Reach\): number \{ return REACH_OPENNESS\[r\]; \}/);
  assert.match(out, /export function isReach\(v: string\): v is Reach \{/);
});
