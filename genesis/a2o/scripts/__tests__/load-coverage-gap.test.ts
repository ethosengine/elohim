import { describe, it } from 'node:test';
import { strict as assert } from 'node:assert';
import { fileURLToPath } from 'node:url';
import { loadCoverageGap } from '../lib/load-coverage-gap.js';

const fixturePath = fileURLToPath(new URL('./fixtures/coverage-gap.json', import.meta.url));

describe('loadCoverageGap', () => {
  it('returns every gap entry with feature + missing', () => {
    const gaps = loadCoverageGap(fixturePath);
    assert.equal(gaps.length, 2);
    assert.equal(gaps[0].feature, 'features/lamad/path-adaptation.feature');
    assert.match(gaps[0].missing, /path reorders/);
  });

  it('defaults severity to "medium" when absent', () => {
    const gaps = loadCoverageGap(fixturePath);
    assert.equal(gaps[1].severity, 'medium');
  });

  it('returns empty array when file does not exist', () => {
    assert.deepEqual(loadCoverageGap('/no/such/file.json'), []);
  });
});
