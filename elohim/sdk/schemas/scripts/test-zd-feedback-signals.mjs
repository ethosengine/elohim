// Test for Z.D-introduced FeedbackSignal kinds.
//
// Three kinds tested:
//   - rate-limit-exceeded
//   - bad-custody
//   - reach-escalation-pending
//
// All ride on the existing FeedbackSignal DHT entry type via the
// signal_kind vocabulary mechanism (project_signal_kind_extensible_protocol_class).
// No new DHT entry types are required.
//
// Source of truth: Holochain DHT (elohim zome's FeedbackSignal).
// Canon: genesis/docs/architecture/rea-compute-commitment-primitive.md §3 reciprocity.

import Ajv2020 from 'ajv/dist/2020.js';
import rateLimitSchema from '../v1/feedback-signals/rate-limit-exceeded.schema.json' with { type: 'json' };
import badCustodySchema from '../v1/feedback-signals/bad-custody.schema.json' with { type: 'json' };
import reachEscalationSchema from '../v1/feedback-signals/reach-escalation-pending.schema.json' with { type: 'json' };

const ajv = new Ajv2020({ strict: true, allErrors: true });
const failures = [];

function makeChecker(name, schema) {
  const validate = ajv.compile(schema);
  return function check(caseName, value, shouldPass) {
    const ok = validate(value);
    if (shouldPass && !ok) failures.push([`${name}: ${caseName}`, validate.errors]);
    else if (!shouldPass && ok) failures.push([`${name}: ${caseName} (should reject)`, null]);
  };
}

// --- rate-limit-exceeded ---
const rateLimit = makeChecker('rate-limit-exceeded', rateLimitSchema);
const rateMinimal = {
  signal_kind: 'rate-limit-exceeded',
  target: 'agent:deploy-svc-agent',
  declarer: 'agent:storage-validator',
  evidence: { bounded_by: 'bafy:commitment', rate_per_hour: 30, recent_count: 31 },
  signed_at: '2026-05-25T08:00:00Z',
};
rateLimit('minimal valid', rateMinimal, true);
rateLimit('with severity', { ...rateMinimal, severity: 'critical' }, true);
rateLimit('wrong signal_kind', { ...rateMinimal, signal_kind: 'other' }, false);
rateLimit(
  'evidence missing bounded_by',
  { ...rateMinimal, evidence: { rate_per_hour: 30, recent_count: 31 } },
  false,
);
rateLimit('extra root field', { ...rateMinimal, mystery: 'no' }, false);

// --- bad-custody ---
const badCustody = makeChecker('bad-custody', badCustodySchema);
const custodyMinimal = {
  signal_kind: 'bad-custody',
  target: 'agent:deploy-svc-agent',
  declarer: 'agent:elohim-counsel',
  evidence: { custody_kind: 'plaintext-repo-file' },
  signed_at: '2026-05-25T08:00:00Z',
};
badCustody('minimal valid', custodyMinimal, true);
badCustody(
  'with evidence url',
  {
    ...custodyMinimal,
    evidence: { ...custodyMinimal.evidence, evidence_url: 'https://github.com/foo/bar/blob/x/key.pem' },
  },
  true,
);
badCustody(
  'unknown custody_kind',
  { ...custodyMinimal, evidence: { custody_kind: 'no-such-kind' } },
  false,
);
badCustody(
  'evidence missing custody_kind',
  { ...custodyMinimal, evidence: { notes: 'incomplete' } },
  false,
);

// --- reach-escalation-pending ---
const reachEscal = makeChecker('reach-escalation-pending', reachEscalationSchema);
const escalMinimal = {
  signal_kind: 'reach-escalation-pending',
  target: 'bafy:new-eprhead-cid',
  declarer: 'agent:doorway-svc-agent',
  evidence: {
    old_reach: 'commons',
    new_reach: 'community',
    projection_commitment: 'bafy:project-epr-commitment-cid',
  },
  signed_at: '2026-05-25T08:00:00Z',
};
reachEscal('minimal valid', escalMinimal, true);
reachEscal(
  'unknown old_reach',
  { ...escalMinimal, evidence: { ...escalMinimal.evidence, old_reach: 'mystery' } },
  false,
);
reachEscal(
  'evidence missing projection_commitment',
  {
    ...escalMinimal,
    evidence: { old_reach: 'commons', new_reach: 'community' },
  },
  false,
);

if (failures.length > 0) {
  console.error('FAIL: Z.D feedback-signals');
  for (const [name, errors] of failures) {
    console.error('  -', name);
    if (errors) console.error('    errors:', JSON.stringify(errors, null, 2));
  }
  process.exit(1);
}

console.log('PASS: Z.D feedback-signals (rate-limit-exceeded, bad-custody, reach-escalation-pending — 13 cases)');
