// Test for v1/commitments/replicates-commons.schema.json
//
// The replicates-commons Commitment is the EPR provide-loop payload shape —
// see genesis/docs/superpowers/specs/2026-06-08-epr-acquisition-slice2b-provide-loop-design.md §4.
//
// Source of truth: Holochain DHT (Mishpat zome, Commitment entry type with
// action="replicates-commons"). This schema is the wire-format projection.
// oneOf union on `variant`: "content" (pure provide, NO ratio_attestation) |
// "capacity" (hosting capacity, WITH ratio_attestation sum-to-100).

import Ajv2020 from 'ajv/dist/2020.js';
import schema from '../v1/commitments/replicates-commons.schema.json' with { type: 'json' };

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

const contentVariant = {
  action: 'replicates-commons',
  variant: 'content',
  head_ref: 'bafyhead-lamad-spa',
  closure_rule: 'transitive-1',
  reach: 'commons',
  bounds: {
    rate_per_minute: 30,
    reach_ceiling: 'commons',
  },
};

const capacityVariant = {
  action: 'replicates-commons',
  variant: 'capacity',
  commons_bytes: 50_000_000_000,
  bounds: {
    rate_per_minute: 30,
    reach_ceiling: 'commons',
  },
  ratio_attestation: {
    commons_pct: 20,
    dwelling_pct: 40,
    collective_pct: 25,
    free_pct: 15,
    effective_ratio_cid: 'bafkrei-x',
  },
};

// --- Happy paths ---
check('content variant minimal', contentVariant, true);
check(
  'content variant without optional closure_rule',
  (() => {
    const v = { ...contentVariant };
    delete v.closure_rule;
    return v;
  })(),
  true,
);
check('capacity variant minimal', capacityVariant, true);

// --- Failure paths ---
check('wrong action discriminator', { ...contentVariant, action: 'something-else' }, false);
check('unknown variant discriminator', { ...contentVariant, variant: 'bogus' }, false);
check(
  'content variant missing head_ref',
  (() => {
    const v = { ...contentVariant };
    delete v.head_ref;
    return v;
  })(),
  false,
);
check(
  'content variant reach not commons',
  { ...contentVariant, reach: 'community' },
  false,
);
check(
  'content variant carrying ratio_attestation (forbidden on content)',
  { ...contentVariant, ratio_attestation: capacityVariant.ratio_attestation },
  false,
);
check(
  'capacity variant zero commons_bytes',
  { ...capacityVariant, commons_bytes: 0 },
  false,
);
check(
  'capacity variant missing ratio_attestation',
  (() => {
    const v = { ...capacityVariant };
    delete v.ratio_attestation;
    return v;
  })(),
  false,
);
check(
  'capacity variant missing effective_ratio_cid',
  (() => {
    const v = {
      ...capacityVariant,
      ratio_attestation: { ...capacityVariant.ratio_attestation },
    };
    delete v.ratio_attestation.effective_ratio_cid;
    return v;
  })(),
  false,
);
check('extra unknown field on root', { ...contentVariant, mystery_field: 'x' }, false);

if (failures.length > 0) {
  console.error('FAIL: replicates-commons schema');
  for (const [name, errors] of failures) {
    console.error('  -', name);
    if (errors) console.error('    errors:', JSON.stringify(errors, null, 2));
  }
  process.exit(1);
}

console.log('PASS: replicates-commons schema (12 cases)');
