// Test for v1/commitments/revokes-commitment.schema.json
//
// The revokes-commitment Commitment notarizes the retraction of a prior
// Commitment (by target_cid) — the substrate-correct revocation arm of the
// EPR provide loop. See
// genesis/docs/superpowers/specs/2026-06-08-epr-acquisition-slice2b-provide-loop-design.md §4.

import Ajv2020 from 'ajv/dist/2020.js';
import schema from '../v1/commitments/revokes-commitment.schema.json' with { type: 'json' };

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

const minimal = {
  action: 'revokes-commitment',
  target_cid: 'bafyhead-target-commitment',
  signed_at: '2026-06-10T00:00:00Z',
};

// --- Happy paths ---
check('minimal valid revocation', minimal, true);
check('with optional reason', { ...minimal, reason: 'pin removed' }, true);

// --- Failure paths ---
check('wrong action discriminator', { ...minimal, action: 'something-else' }, false);
check(
  'empty target_cid',
  { ...minimal, target_cid: '' },
  false,
);
check(
  'missing target_cid',
  (() => {
    const v = { ...minimal };
    delete v.target_cid;
    return v;
  })(),
  false,
);
check(
  'missing signed_at',
  (() => {
    const v = { ...minimal };
    delete v.signed_at;
    return v;
  })(),
  false,
);
check('extra unknown field on root', { ...minimal, mystery_field: 'x' }, false);

if (failures.length > 0) {
  console.error('FAIL: revokes-commitment schema');
  for (const [name, errors] of failures) {
    console.error('  -', name);
    if (errors) console.error('    errors:', JSON.stringify(errors, null, 2));
  }
  process.exit(1);
}

console.log('PASS: revokes-commitment schema (8 cases)');
