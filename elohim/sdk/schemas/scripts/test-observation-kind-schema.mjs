import Ajv2020 from 'ajv/dist/2020.js';
import schema from '../v1/manifest/observation-kind.schema.json' with { type: 'json' };

const ajv = new Ajv2020({ strict: true, allErrors: true });
const validate = ajv.compile(schema);

const minimal = {
  kind: 'infrastructure:doorway-heartbeat',
  namespace: 'elohim/observations/infrastructure',
  schema: { doorway_id: 'Cid', peer_count: 'u32' },
  retention_class: 'operational',
  reach: 'community'
};
if (!validate(minimal)) {
  console.error(validate.errors);
  process.exit(1);
}

const withGraduation = {
  ...minimal,
  diversity_threshold: { distinct_households: 3, min_count: 5 },
  graduates_to: 'attestation:doorway-health',
  graduation_window_seconds: 3600,
  graduation_policy: 'diversity-threshold'
};
if (!validate(withGraduation)) {
  console.error(validate.errors);
  process.exit(1);
}

const invalid_reach = { ...minimal, reach: 'invalid' };
if (validate(invalid_reach)) {
  console.error('Expected reach validation to fail');
  process.exit(1);
}

console.log('observation-kind.schema.json validates');
