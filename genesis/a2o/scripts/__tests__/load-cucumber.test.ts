import { strict as assert } from 'node:assert';
import { readFileSync } from 'node:fs';
import { describe, it } from 'node:test';

import { loadCucumber } from '../lib/load-cucumber.js';

const fixture = readFileSync(new URL('./fixtures/cucumber-mixed.json', import.meta.url), 'utf8');

void describe('loadCucumber', () => {
  void it('parses all scenarios with feature URI', () => {
    const results = loadCucumber(fixture);
    assert.equal(results.length, 3);
    assert.equal(results[0].name, 'Timothy completes path');
    assert.equal(results[0].feature, 'features/lamad/learning-journey.feature');
  });

  void it('classifies passed scenario', () => {
    const [passed] = loadCucumber(fixture);
    assert.equal(passed.status, 'passed');
    assert.equal(passed.failureMessage, undefined);
  });

  void it('extracts failure message from failed step', () => {
    const failed = loadCucumber(fixture).find(r => r.status === 'failed')!;
    assert.ok(failed);
    assert.match(failed.failureMessage!, /AssertionError: expected 500 to be 200/);
  });

  void it('classifies pending scenario', () => {
    const pending = loadCucumber(fixture).find(r => r.status === 'pending')!;
    assert.ok(pending);
    assert.equal(pending.failureMessage, undefined);
  });

  void it('computes summary counts', () => {
    const summary = loadCucumber(fixture).reduce(
      (acc, r) => ({ ...acc, [r.status]: (acc[r.status] || 0) + 1 }),
      {} as Record<string, number>
    );
    assert.deepEqual(summary, { passed: 1, failed: 1, pending: 1 });
  });

  void it('throws on malformed input', () => {
    assert.throws(() => loadCucumber('not-json'), /JSON/);
  });
});
