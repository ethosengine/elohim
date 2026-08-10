// Test for Z.D-introduced FeedbackSignal kinds.
//
// Five kinds tested:
//   - rate-limit-exceeded
//   - bad-custody
//   - reach-escalation-pending
//   - algedonic-approach
//   - algedonic-breach
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
import algedonicApproachSchema from '../v1/feedback-signals/algedonic-approach.schema.json' with { type: 'json' };
import algedonicBreachSchema from '../v1/feedback-signals/algedonic-breach.schema.json' with { type: 'json' };

const ajv = new Ajv2020({ strict: true, allErrors: true });
const failures = [];
let caseCount = 0;

function makeChecker(name, schema) {
  const validate = ajv.compile(schema);
  return function check(caseName, value, shouldPass) {
    caseCount += 1;
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

// --- algedonic-approach ---
// standing_impact is fixed 'advisory' for this kind, but per family envelope
// parity (rate-limit-exceeded / bad-custody / reach-escalation-pending all
// omit it from properties+required) it is documentation-only here, not a
// schema field — additionalProperties:false means submitting it is REJECTED,
// same as any other unrecognized property.
const algedonicApproach = makeChecker('algedonic-approach', algedonicApproachSchema);
const approachMinimal = {
  signal_kind: 'algedonic-approach',
  target: 'bafy:threatened-commitment',
  declarer: 'agent:storage-validator',
  evidence: { stock: 27, limit: 30, bound_ref: 'bafy:bounding-commitment', threshold_pct: 90 },
  signed_at: '2026-08-10T08:00:00Z',
};
algedonicApproach('minimal valid', approachMinimal, true);
algedonicApproach('with severity', { ...approachMinimal, severity: 'info' }, true);
algedonicApproach('wrong signal_kind', { ...approachMinimal, signal_kind: 'other' }, false);
algedonicApproach(
  'evidence missing threshold_pct',
  { ...approachMinimal, evidence: { stock: 27, limit: 30, bound_ref: 'bafy:bounding-commitment' } },
  false,
);
algedonicApproach(
  'rejects standing_impact (description-only per family envelope parity)',
  { ...approachMinimal, standing_impact: 'advisory' },
  false,
);
algedonicApproach('extra root field', { ...approachMinimal, mystery: 'no' }, false);
// Wire tolerance is (0,100] — the [1,100] percent floor is producer-side
// (Bound::new) discipline, not wire law. threshold_pct:0 and :101 must FAIL;
// a sub-1 fractional percent like 0.85 must still PASS at the wire.
algedonicApproach(
  'threshold_pct 0 rejected (exclusiveMinimum)',
  { ...approachMinimal, evidence: { ...approachMinimal.evidence, threshold_pct: 0 } },
  false,
);
algedonicApproach(
  'threshold_pct 101 rejected (maximum 100)',
  { ...approachMinimal, evidence: { ...approachMinimal.evidence, threshold_pct: 101 } },
  false,
);
algedonicApproach(
  'threshold_pct 0.85 passes (wire tolerance below producer floor)',
  { ...approachMinimal, evidence: { ...approachMinimal.evidence, threshold_pct: 0.85 } },
  true,
);

// --- algedonic-breach ---
// Same standing_impact=description-only convention as algedonic-approach above.
const algedonicBreach = makeChecker('algedonic-breach', algedonicBreachSchema);
const breachMinimal = {
  signal_kind: 'algedonic-breach',
  target: 'bafy:threatened-commitment',
  declarer: 'agent:storage-validator',
  evidence: { stock: 31, limit: 30, bound_ref: 'bafy:bounding-commitment' },
  signed_at: '2026-08-10T08:00:00Z',
};
algedonicBreach('minimal valid', breachMinimal, true);
algedonicBreach('with severity', { ...breachMinimal, severity: 'critical' }, true);
algedonicBreach('wrong signal_kind', { ...breachMinimal, signal_kind: 'other' }, false);
algedonicBreach(
  'evidence missing bound_ref',
  { ...breachMinimal, evidence: { stock: 31, limit: 30 } },
  false,
);
algedonicBreach(
  'rejects standing_impact (description-only per family envelope parity)',
  { ...breachMinimal, standing_impact: 'advisory' },
  false,
);
algedonicBreach('extra root field', { ...breachMinimal, mystery: 'no' }, false);

if (failures.length > 0) {
  console.error('FAIL: Z.D feedback-signals');
  for (const [name, errors] of failures) {
    console.error('  -', name);
    if (errors) console.error('    errors:', JSON.stringify(errors, null, 2));
  }
  process.exit(1);
}

console.log(
  `PASS: Z.D feedback-signals (rate-limit-exceeded, bad-custody, reach-escalation-pending, algedonic-approach, algedonic-breach — ${caseCount} cases)`,
);
