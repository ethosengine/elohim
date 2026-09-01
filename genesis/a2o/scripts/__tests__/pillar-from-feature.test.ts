import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import { pillarFromFeature } from '../lib/pillar-from-feature.js';

void describe('pillarFromFeature', () => {
  const cases: [string, string][] = [
    ['features/lms/learning-journey.feature', 'lamad'],
    ['features/auth/fixture-humans.feature', 'imagodei'],
    ['features/content/content-lifecycle.feature', 'content'],
    ['features/federation/peer-advertisement.feature', 'federation'],
    ['features/delivery/peer-mesh.feature', 'delivery'],
    ['features/browser/auth-browser.feature', 'browser'],
    ['features/elohim/presence.feature', 'elohim'],
    ['features/qahal/collective-governance.feature', 'qahal'],
    ['features/rms/human-resilience.feature', 'shefa'],
    ['features/wms/project-delivery.feature', 'avodah'],
    ['features/deployment/staging-validation.feature', 'deployment'],
    ['genesis/a2o/features/lms/path-adaptation.feature', 'lamad'],
    ['/absolute/path/to/features/browser/nav.feature', 'browser'],
    ['features/weird-new-area/x.feature', 'weird-new-area'],
    ['not-a-feature-path.txt', 'unknown'],
  ];

  for (const [uri, expected] of cases) {
    void it(`"${uri}" → ${expected}`, () => {
      assert.equal(pillarFromFeature(uri), expected);
    });
  }
});
