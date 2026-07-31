// Test for v1/commitments/adopts-content.schema.json
//
// The adopts-content Commitment is the adoption ceremony's wire contract:
// consent (policy_ref, or absent = bare adoption), the epistemic timestamp
// (epistemic_digest = claim-state visible at the adopting node), and the
// per-variety lane map the aggregation boundary enforces. See
// genesis/docs/superpowers/specs/2026-07-31-care-aggregation-adoption-policy-floor-design.md §4.

import Ajv2020 from 'ajv/dist/2020.js';
import schema from '../v1/commitments/adopts-content.schema.json' with { type: 'json' };

const ajv = new Ajv2020({ strict: true, allErrors: true });
const validate = ajv.compile(schema);

const failures = [];

function check(name, value, shouldPass) {
  const ok = validate(value);
  if (shouldPass && !ok) {
    failures.push([name, validate.errors]);
  } else if (!shouldPass && ok) {
    failures.push([`${name} (should have been rejected)`, null]);
  }
}

const policied = {
  action: 'adopts-content',
  head_ref: 'bafyhead-gottman-seven-principles',
  policy_ref: 'bafypolicy-commons-aggregation-terms-v1',
  provider: 'bafyagent-household-dyad',
  epistemic_digest: 'bafydigest-visible-claimstate-at-adoption',
  signed_at: '2026-07-31T00:00:00Z',
  bounds: {
    lanes: {
      'completion-count': 'k-anonymous',
      'timing-pattern': 'suppressed',
      'content-response': 'after-graduation-only',
    },
  },
};

const bare = {
  action: 'adopts-content',
  head_ref: 'bafyhead-ungoverned-material',
  provider: 'bafyagent-household-dyad',
  epistemic_digest: 'bafydigest-empty-claimset',
  signed_at: '2026-07-31T00:00:00Z',
  bounds: {},
};

// --- Happy paths ---
check('policied adoption with lane map', policied, true);
check('bare adoption (no policy_ref, no lanes) — legible ungoverned posture', bare, true);
check('bare adoption may still narrow lanes explicitly', {
  ...bare,
  bounds: { lanes: { 'completion-count': 'suppressed' } },
}, true);
check('full six-variety lane map', {
  ...policied,
  bounds: {
    lanes: {
      'completion-count': 'k-anonymous',
      'engagement-duration': 'count-only',
      'timing-pattern': 'suppressed',
      'sequence-trace': 'suppressed',
      'content-response': 'after-graduation-only',
      'presence-coincidence': 'suppressed',
    },
  },
}, true);

// --- Failure paths ---
check('wrong action discriminator', { ...policied, action: 'adopts-something' }, false);
check('empty head_ref', { ...policied, head_ref: '' }, false);
check(
  'missing epistemic_digest (the blameless-hindsight record is not optional)',
  (() => {
    const v = { ...policied };
    delete v.epistemic_digest;
    return v;
  })(),
  false,
);
check(
  'unknown variety key in lanes (a variety no policy can name is a variety no lane can suppress)',
  { ...policied, bounds: { lanes: { 'keystroke-cadence': 'suppressed' } } },
  false,
);
check(
  'invalid lane value',
  { ...policied, bounds: { lanes: { 'completion-count': 'broadcast' } } },
  false,
);
check(
  'missing bounds',
  (() => {
    const v = { ...policied };
    delete v.bounds;
    return v;
  })(),
  false,
);
check('empty policy_ref (bare adoption omits the field, never empties it)', { ...policied, policy_ref: '' }, false);

if (failures.length > 0) {
  console.error(`adopts-content schema test: ${failures.length} failure(s)`);
  for (const [name, errors] of failures) {
    console.error(`  ✗ ${name}`);
    if (errors) console.error(`    ${JSON.stringify(errors)}`);
  }
  process.exit(1);
}
console.log('adopts-content schema test: all checks passed');
