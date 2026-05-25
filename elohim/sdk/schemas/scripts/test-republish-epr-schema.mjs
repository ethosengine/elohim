// Test for v1/economic-events/republish-epr.schema.json
//
// Source of truth: Holochain DHT (EPR substrate via FederatedEprStore + Kad
// StartProviding; projected to elohim-storage 'economic_events' table).
// Canon: genesis/docs/architecture/rea-compute-commitment-primitive.md.

import Ajv2020 from 'ajv/dist/2020.js';
import schema from '../v1/economic-events/republish-epr.schema.json' with { type: 'json' };

const ajv = new Ajv2020({ strict: true, allErrors: true });
const validate = ajv.compile(schema);

const failures = [];
function check(name, value, shouldPass) {
  const ok = validate(value);
  if (shouldPass && !ok) failures.push([name, validate.errors]);
  else if (!shouldPass && ok) failures.push([`${name} (should have been rejected)`, null]);
}

const minimal = {
  action: 'republish-epr',
  performer: 'agent:deploy-service-matthew-doorway',
  bounded_by: 'bafy:commitment-cid',
  target: 'bafy:new-eprhead-cid',
  supersedes: 'bafy:prev-eprhead-cid',
  payload: {
    blob_cid: 'bafy:new-blob-cid',
    epr_kind: 'Content',
    reach: 'commons',
    bundle_path: 'lamad-spa',
  },
  signed_at: '2026-05-25T07:41:14Z',
};

// Happy paths
check('minimal valid republish-epr event', minimal, true);
check('first-publish (supersedes: null)', { ...minimal, supersedes: null }, true);
check(
  'payload without optional bundle_path',
  {
    ...minimal,
    payload: { blob_cid: 'bafy:b', epr_kind: 'Content', reach: 'commons' },
  },
  true,
);
check(
  'higher reach value (substrate validator checks against bounds, not the schema)',
  {
    ...minimal,
    payload: { ...minimal.payload, reach: 'community' },
  },
  true,
);

// Failure paths
check('wrong action discriminator', { ...minimal, action: 'something-else' }, false);
check(
  'missing bounded_by (anonymous publish forbidden)',
  (() => {
    const v = { ...minimal };
    delete v.bounded_by;
    return v;
  })(),
  false,
);
check(
  'missing performer',
  (() => {
    const v = { ...minimal };
    delete v.performer;
    return v;
  })(),
  false,
);
check(
  'missing payload',
  (() => {
    const v = { ...minimal };
    delete v.payload;
    return v;
  })(),
  false,
);
check(
  'payload missing blob_cid',
  {
    ...minimal,
    payload: { epr_kind: 'Content', reach: 'commons' },
  },
  false,
);
check(
  'unknown epr_kind value',
  {
    ...minimal,
    payload: { ...minimal.payload, epr_kind: 'UnknownKind' },
  },
  false,
);
check(
  'unknown reach value',
  {
    ...minimal,
    payload: { ...minimal.payload, reach: 'super-private' },
  },
  false,
);
check('extra unknown field on root', { ...minimal, mystery_field: 'no' }, false);

if (failures.length > 0) {
  console.error('FAIL: republish-epr schema');
  for (const [name, errors] of failures) {
    console.error('  -', name);
    if (errors) console.error('    errors:', JSON.stringify(errors, null, 2));
  }
  process.exit(1);
}

console.log('PASS: republish-epr schema (12 cases)');
