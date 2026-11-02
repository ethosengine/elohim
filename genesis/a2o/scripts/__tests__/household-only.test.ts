import { strict as assert } from 'node:assert';
import { test } from 'node:test';

import {
  HOUSEHOLD_ONLY_HOLD_REASON,
  householdOnlyHoldReason,
  isHouseholdOnlyFeature,
} from '../lib/household-only.js';

void test('recognizes projected household-only feature URIs', () => {
  const feature = 'features/dataplane/epr-app-deliverability.feature';
  assert.equal(isHouseholdOnlyFeature(feature), true);
  assert.equal(isHouseholdOnlyFeature(`/checkout/genesis/a2o/${feature}`), true);
  assert.equal(
    isHouseholdOnlyFeature('features/dataplane/doorway-fixture-readiness.feature'),
    false
  );
});

void test('uses the honest label only when owned-substrate holds a household-only feature', () => {
  const feature = 'features/dataplane/epr-app-deliverability.feature';
  assert.equal(householdOnlyHoldReason(feature, ['owned-substrate']), HOUSEHOLD_ONLY_HOLD_REASON);
  assert.equal(householdOnlyHoldReason(feature, ['shem']), undefined);
  assert.equal(
    householdOnlyHoldReason('features/dataplane/doorway-fixture-readiness.feature', [
      'owned-substrate',
    ]),
    undefined
  );
});
