// Test for v1/commitments/acknowledges-reach-change.schema.json
//
// Source of truth: Holochain DHT (Mishpat zome).
// Canon: genesis/docs/architecture/stewardship-over-sovereignty.md §6 rule 3.

import Ajv2020 from 'ajv/dist/2020.js';
import schema from '../v1/commitments/acknowledges-reach-change.schema.json' with { type: 'json' };

const ajv = new Ajv2020({ strict: true, allErrors: true });
const validate = ajv.compile(schema);

const failures = [];
function check(name, value, shouldPass) {
  const ok = validate(value);
  if (shouldPass && !ok) failures.push([name, validate.errors]);
  else if (!shouldPass && ok) failures.push([`${name} (should have been rejected)`, null]);
}

const minimal = {
  action: 'acknowledges-reach-change',
  acknowledger: 'agent:matthew-steward',
  target_epr_cid: 'bafy:eprhead-cid',
  new_reach: 'community',
  signed_at: '2026-05-25T08:00:00Z',
};

check('minimal valid acknowledgement', minimal, true);
check('reduction reach value', { ...minimal, new_reach: 'commons' }, true);
check('wrong action', { ...minimal, action: 'foo' }, false);
check(
  'missing acknowledger',
  (() => {
    const v = { ...minimal };
    delete v.acknowledger;
    return v;
  })(),
  false,
);
check('unknown reach value', { ...minimal, new_reach: 'super-private' }, false);
check('extra unknown field', { ...minimal, mystery: 'no' }, false);

if (failures.length > 0) {
  console.error('FAIL: acknowledges-reach-change schema');
  for (const [name, errors] of failures) {
    console.error('  -', name);
    if (errors) console.error('    errors:', JSON.stringify(errors, null, 2));
  }
  process.exit(1);
}

console.log('PASS: acknowledges-reach-change schema (6 cases)');
