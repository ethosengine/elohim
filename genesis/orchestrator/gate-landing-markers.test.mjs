import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { MARKERS } from './gate-landing-markers.mjs';
import { SKIPPED_WHY_CODES } from './gate-landing-base.mjs';

const samples = {
  eprRefused: ['epr'],
  storeRefused: ['/store', 'epr device arrive'],
  intent: ['bafyaaaa', `I${'a'.repeat(40)}`, 'a'.repeat(12), '1', 'bafyaaaa'],
  noChangeId: ['a'.repeat(12)],
  degraded: ['not JSON'],
  skipped: ['bafyaaaa', 'dormant'],
  governance: ['a'.repeat(40), 'reuse'],
  moved: ['a'.repeat(40)],
  claimed: [],
  held: ['12'],
  overrideWait: ['2026-11-01T00:00:00Z', '12'],
  daemonDown: [],
  interrupted: ['bafyaaaa'],
  dismissed: ['bafyaaaa,bafybbbb', 'bafycap'],
  gateBanner: [],
};

test('every marker formats a line its regex matches', async function every_marker_formats_a_line_its_regex_matches(t) {
  assert.deepEqual(Object.keys(MARKERS), Object.keys(samples));
  for (const [key, marker] of Object.entries(MARKERS))
    assert.match(marker.format(...samples[key]), marker.re, key);
});
test('skipped without a reason does not match', async function skipped_without_a_reason_does_not_match(t) {
  assert.doesNotMatch('[pre-push] Phase V: skipped intent bafyaaaa — ', MARKERS.skipped.re);
});
test('cli prints the formatted line', async function cli_prints_the_formatted_line(t) {
  const module = join(dirname(fileURLToPath(import.meta.url)), 'gate-landing-markers.mjs');
  const env = { ...process.env };
  delete env.NODE_TEST_CONTEXT;
  const result = spawnSync(process.execPath, [module, 'print', 'held', '7'], {
    encoding: 'utf8',
    env,
  });
  assert.equal(result.status, 0);
  assert.equal(result.stdout, `${MARKERS.held.format('7')}\n`);
});
test('skipped vocabulary matches the picker', async function skipped_vocabulary_matches_the_picker(t) {
  const reasons = [
    ...SKIPPED_WHY_CODES,
    'advisory withdraw by did:key:z',
    'advisory landed by did:key:z',
    'advisory dismiss by did:key:z',
  ];
  for (const reason of reasons)
    assert.match(MARKERS.skipped.format('bafyaaaa', reason), MARKERS.skipped.re, reason);
});
