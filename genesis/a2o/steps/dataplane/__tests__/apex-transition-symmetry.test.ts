/**
 * The sibling browser is held to the primary's own browser reading, not to a
 * corpus it never had (genesis/a2o/steps/dataplane/apex-transition.steps.ts,
 * "that visitor completes browser bootstrap with the same declared build
 * stamp"). Every household doorway returns the same `/epr-head/<slug>` 404s for
 * the landing's unseeded epic links, and every household doorway 404s
 * `/assets/config.json` (alpha mounts it from a ConfigMap; the household stages
 * the dist as built, and ConfigService falls back to DEFAULT_PROD_CONFIG on that
 * 404). Only that corpus-gap class (404 under `/epr-head/`, or exactly
 * `/assets/config.json`) is forgiven when the primary shared it — every other
 * error is a failover regression, and the primary itself must be clean but for it. Origins differ (primary :8888, sibling :8889), so the
 * comparison is by URL path + status.
 */
import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import {
  primaryErrorsOutsideCorpusGap,
  siblingWorseThanPrimary,
} from '../apex-transition.compare.js';

const PRIMARY = 'http://127.0.0.1:8888';
const SIBLING = 'http://127.0.0.1:8889';

void describe('siblingWorseThanPrimary', () => {
  void it('returns nothing when both browsers saw the same errors', () => {
    const errors = [{ status: 404, url: '/epr-head/x' }];
    assert.deepEqual(siblingWorseThanPrimary(errors, errors), []);
  });

  void it('forgives an error the primary also had, across differing origins', () => {
    assert.deepEqual(
      siblingWorseThanPrimary(
        [{ status: 404, url: `${PRIMARY}/epr-head/x` }],
        [{ status: 404, url: `${SIBLING}/epr-head/x` }]
      ),
      []
    );
  });

  void it('names an asset the primary served but the sibling 404d', () => {
    assert.deepEqual(
      siblingWorseThanPrimary(
        [{ status: 404, url: `${PRIMARY}/epr-head/x` }],
        [
          { status: 404, url: `${SIBLING}/epr-head/x` },
          { status: 404, url: `${SIBLING}/main-abc.js` },
        ]
      ),
      [{ status: 404, url: '/main-abc.js' }]
    );
  });

  void it('forgives the household-absent /assets/config.json 404 when the primary shared it', () => {
    assert.deepEqual(
      primaryErrorsOutsideCorpusGap([{ status: 404, url: `${PRIMARY}/assets/config.json` }]),
      []
    );
    assert.deepEqual(
      siblingWorseThanPrimary(
        [{ status: 404, url: `${PRIMARY}/assets/config.json` }],
        [{ status: 404, url: `${SIBLING}/assets/config.json` }]
      ),
      []
    );
  });

  void it('does not forgive a sibling-only /assets/config.json 404', () => {
    assert.deepEqual(
      siblingWorseThanPrimary([], [{ status: 404, url: `${SIBLING}/assets/config.json` }]),
      [{ status: 404, url: '/assets/config.json' }]
    );
  });

  void it('does not forgive a 404 on another /assets/ path even when the primary shared it', () => {
    assert.deepEqual(
      siblingWorseThanPrimary(
        [{ status: 404, url: `${PRIMARY}/assets/other.json` }],
        [{ status: 404, url: `${SIBLING}/assets/other.json` }]
      ),
      [{ status: 404, url: '/assets/other.json' }]
    );
  });

  void it('does not forgive a 5xx on /assets/config.json even when the primary shared it', () => {
    assert.deepEqual(
      siblingWorseThanPrimary(
        [{ status: 500, url: `${PRIMARY}/assets/config.json` }],
        [{ status: 500, url: `${SIBLING}/assets/config.json` }]
      ),
      [{ status: 500, url: '/assets/config.json' }]
    );
    assert.deepEqual(
      primaryErrorsOutsideCorpusGap([{ status: 503, url: `${PRIMARY}/assets/config.json` }]),
      [{ status: 503, url: '/assets/config.json' }]
    );
  });

  void it('fails a non-corpus-gap error even when the primary had it too', () => {
    assert.deepEqual(
      siblingWorseThanPrimary(
        [{ status: 404, url: `${PRIMARY}/main-x.js` }],
        [{ status: 404, url: `${SIBLING}/main-x.js` }]
      ),
      [{ status: 404, url: '/main-x.js' }]
    );
  });

  void it('forgives only a 404 under /epr-head/, not another status there', () => {
    assert.deepEqual(
      siblingWorseThanPrimary(
        [{ status: 500, url: `${PRIMARY}/epr-head/x` }],
        [{ status: 500, url: `${SIBLING}/epr-head/x` }]
      ),
      [{ status: 500, url: '/epr-head/x' }]
    );
  });
});

void describe('primaryErrorsOutsideCorpusGap', () => {
  void it('accepts a primary whose only errors are /epr-head/ 404s', () => {
    assert.deepEqual(
      primaryErrorsOutsideCorpusGap([{ status: 404, url: `${PRIMARY}/epr-head/x` }]),
      []
    );
  });

  void it('names any other primary error', () => {
    assert.deepEqual(
      primaryErrorsOutsideCorpusGap([
        { status: 404, url: `${PRIMARY}/epr-head/x` },
        { status: 404, url: `${PRIMARY}/main-x.js` },
      ]),
      [{ status: 404, url: '/main-x.js' }]
    );
  });
});
