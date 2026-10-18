/**
 * The sibling browser is held to the same corpus-gap rule as the primary's own
 * browser reading (genesis/a2o/steps/dataplane/apex-transition.steps.ts,
 * "that visitor completes browser bootstrap with the same declared build
 * stamp"). Every household doorway returns the same `/epr-head/<slug>` 404s for
 * the landing's unseeded epic links, and every household doorway 404s
 * `/assets/config.json` (alpha mounts it from a ConfigMap; the household stages
 * the dist as built, and ConfigService falls back to DEFAULT_PROD_CONFIG on that
 * 404). That corpus-gap class (404 under `/epr-head/`, or exactly
 * `/assets/config.json`) is forgiven for either browser whether or not the
 * other's capture recorded it: a browser may serve config.json from cache on a
 * repeat visit, so the primary lacking it is not a failover fact. Every other
 * error is a failover regression, and the primary itself must be clean but for
 * the class. Origins differ (primary :8888, sibling :8889), so errors are
 * reported by URL path + status.
 */
import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import {
  primaryErrorsOutsideCorpusGap,
  siblingErrorsOutsideCorpusGap,
} from '../apex-transition.compare.js';

const PRIMARY = 'http://127.0.0.1:8888';
const SIBLING = 'http://127.0.0.1:8889';

void describe('siblingErrorsOutsideCorpusGap', () => {
  void it('forgives an /epr-head/ 404 across origins', () => {
    assert.deepEqual(
      siblingErrorsOutsideCorpusGap([{ status: 404, url: `${SIBLING}/epr-head/x` }]),
      []
    );
  });

  void it('forgives the household-absent /assets/config.json 404', () => {
    assert.deepEqual(
      primaryErrorsOutsideCorpusGap([{ status: 404, url: `${PRIMARY}/assets/config.json` }]),
      []
    );
    assert.deepEqual(
      siblingErrorsOutsideCorpusGap([{ status: 404, url: `${SIBLING}/assets/config.json` }]),
      []
    );
  });

  void it('forgives the declared class with no primary capture to compare against', () => {
    // Regression: the primary's capture may lack config.json (cache on repeat visit); the sibling
    // is still forgiven because the path is in the declared class, not because the primary had it.
    assert.deepEqual(
      siblingErrorsOutsideCorpusGap([
        { status: 404, url: `${SIBLING}/assets/config.json` },
        { status: 404, url: `${SIBLING}/epr-head/y` },
      ]),
      []
    );
  });

  void it('names a main bundle the sibling 404d', () => {
    assert.deepEqual(
      siblingErrorsOutsideCorpusGap([
        { status: 404, url: `${SIBLING}/epr-head/x` },
        { status: 404, url: `${SIBLING}/main-abc.js` },
      ]),
      [{ status: 404, url: '/main-abc.js' }]
    );
  });

  void it('does not forgive a 404 on another /assets/ path', () => {
    assert.deepEqual(
      siblingErrorsOutsideCorpusGap([{ status: 404, url: `${SIBLING}/assets/other.json` }]),
      [{ status: 404, url: '/assets/other.json' }]
    );
  });

  void it('does not forgive a 5xx on /assets/config.json', () => {
    assert.deepEqual(
      siblingErrorsOutsideCorpusGap([{ status: 500, url: `${SIBLING}/assets/config.json` }]),
      [{ status: 500, url: '/assets/config.json' }]
    );
    assert.deepEqual(
      primaryErrorsOutsideCorpusGap([{ status: 503, url: `${PRIMARY}/assets/config.json` }]),
      [{ status: 503, url: '/assets/config.json' }]
    );
  });

  void it('forgives only a 404 under /epr-head/, not another status there', () => {
    assert.deepEqual(
      siblingErrorsOutsideCorpusGap([{ status: 500, url: `${SIBLING}/epr-head/x` }]),
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
