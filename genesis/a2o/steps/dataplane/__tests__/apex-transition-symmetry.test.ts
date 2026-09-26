/**
 * The sibling browser is held to the primary's own browser reading, not to a
 * corpus it never had (genesis/a2o/steps/dataplane/apex-transition.steps.ts,
 * "that visitor completes browser bootstrap with the same declared build
 * stamp"). Every household doorway returns the same `/epr-head/<slug>` 404s for
 * the landing's unseeded epic links; only an error the primary did NOT have is
 * a failover regression. Origins differ (primary :8888, sibling :8889), so the
 * comparison is by URL path + status.
 */
import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import { siblingWorseThanPrimary } from '../apex-transition.compare.js';

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
});
