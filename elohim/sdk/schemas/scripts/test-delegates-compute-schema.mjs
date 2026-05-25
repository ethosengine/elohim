// Test for v1/commitments/delegates-compute.schema.json
//
// The delegates-compute Commitment is the REA compute-commitment primitive's
// payload shape — see genesis/docs/architecture/rea-compute-commitment-primitive.md.
//
// Source of truth: Holochain DHT (Mishpat zome, Commitment entry type with
// action="delegates-compute"). This schema is the wire-format projection.

import Ajv2020 from 'ajv/dist/2020.js';
import schema from '../v1/commitments/delegates-compute.schema.json' with { type: 'json' };

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
  action: 'delegates-compute',
  scope: 'republish-epr',
  provider: 'agent:matthew-steward',
  recipient: 'agent:deploy-service-matthew-doorway',
  bounds: {
    epr_scope: ['epr:lamad-spa', 'epr:elohim-host-landing'],
    reach_ceiling: 'commons',
    rate_per_hour: 30,
    rotation_ttl_days: 90,
  },
  valid_from: '2026-05-25T00:00:00Z',
  valid_until: '2026-08-23T00:00:00Z',
};

// --- Happy paths ---
check('minimal valid commitment', minimal, true);
check('wildcard epr_scope', { ...minimal, bounds: { ...minimal.bounds, epr_scope: ['*'] } }, true);
check(
  'explicit reach elevation (community + acknowledged)',
  {
    ...minimal,
    bounds: { ...minimal.bounds, reach_ceiling: 'community' },
  },
  true,
);
check(
  'high reach with acknowledgement',
  {
    ...minimal,
    bounds: { ...minimal.bounds, reach_ceiling: 'private', reach_elevation_acknowledged: true },
  },
  true,
);

// --- Failure paths ---
check('wrong action discriminator', { ...minimal, action: 'something-else' }, false);
check(
  'missing recipient',
  (() => {
    const v = { ...minimal };
    delete v.recipient;
    return v;
  })(),
  false,
);
check('empty epr_scope', { ...minimal, bounds: { ...minimal.bounds, epr_scope: [] } }, false);
check(
  'missing rate_per_hour',
  (() => {
    const v = { ...minimal, bounds: { ...minimal.bounds } };
    delete v.bounds.rate_per_hour;
    return v;
  })(),
  false,
);
check('rate_per_hour=0', { ...minimal, bounds: { ...minimal.bounds, rate_per_hour: 0 } }, false);
check(
  'reach above commons without reach_elevation_acknowledged',
  { ...minimal, bounds: { ...minimal.bounds, reach_ceiling: 'private' } },
  false,
);
check(
  'extra unknown field on root',
  { ...minimal, mystery_field: 'should-be-rejected' },
  false,
);

if (failures.length > 0) {
  console.error('FAIL: delegates-compute schema');
  for (const [name, errors] of failures) {
    console.error('  -', name);
    if (errors) console.error('    errors:', JSON.stringify(errors, null, 2));
  }
  process.exit(1);
}

console.log('PASS: delegates-compute schema (11 cases)');
