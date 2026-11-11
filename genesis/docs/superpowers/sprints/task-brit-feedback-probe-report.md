---
id: task-brit-feedback-probe-report
gap: plans__2026-11-11-brit-feedback-operation#1
actor: agent:implementer@gpt-6-astra
status: DONE_WITH_CONCERNS
commits: []
cites: []
---

# Brit feedback contract probe

Delivered the reusable probe at `genesis/a2o/scripts/verify-brit-feedback.ts`,
eight focused tests, and one additional scenario in the existing private-feedback
feature. This fulfills the diagnostic station only. Installed delivery and the
parent installation commitment remain open.

The default asks a direct native EPR resolver for both installed current paths and
CIDs, then exercises those exact executables. An inherited `EPR_BIN` can serve as
the explicitly reported bootstrap resolver; it does not become the measured EPR.
`--resolver PATH` supplies that bootstrap when necessary. Candidate execution
requires both `--epr PATH --brit PATH` and is labeled `explicit-candidate`.
The real `*_PIN_CID` and candidate overrides do not select the measured pair.
A launcher without a direct resolver is unmeasured: executing it can publish a
session pin before parsing `--current`. Native `head-absent` is also unmeasured;
an incompatible interface or malformed response fails.

The isolated fixture copies the contextual policy, performs its explicit capacity
migration, retains a successful command and parser refusal with exact invocation
correlation, reopens both, and checks that the private argument never reaches
retained bytes. Its native proposal must produce exactly one Intent whose
`inScopeOf` CID bytes equal the refused occurrence. Triage must contain both
experiences and withhold admission; no commitment or execution event is created.
Recorder failure must visibly preserve producer exits 0 and 2. Fixture cleanup
runs after the asynchronous CID decoder completes. No fixture record is written
to the real feedback store.

## Verification

Gate evidence: `just gate genesis-a2o` — **EXIT=0** on frozen final source
(`genesis/a2o/reports/recovery/brit-feedback-20261111/probe-gate-stable.log`).
`just gate gherkin-prepush-lint` — **EXIT=0** (`probe-gherkin-gate.log`).
The dirty shared tree prevents reusable committed gate evidence; neither command
is presented as publication or source attestation.

From `genesis/a2o`,
`FEEDBACK_TEST_EPR=/projects/.cargo-target-pool/family/dev/elohim__eprfs/dev/debug/epr FEEDBACK_TEST_BRIT=/projects/elohim/genesis/local-dev/feedback-delivery/standalone/bin/brit pnpm exec tsx --test scripts/__tests__/verify-brit-feedback.test.ts`
— **EXIT=0**, eight tests, zero skips (`probe-tests.log`). Independent technical
review repeated the same eight checks and found no remaining actionable issue.

The isolated Cucumber configuration retained beside the receipts ran the existing
feature with the same explicit pair and `EPR_BIN` selecting its recorder:
**EXIT=0**, four scenarios, 24 steps (`probe-a2o.log`, `probe-cucumber.json`).
Final scoped ESLint and TypeScript checks also returned **EXIT=0**. Dynamic
`import('multiformats/cid')` preserves Cucumber's CommonJS loading while reusing
the existing CID decoder; no alternate codec was added.

The default installed-current probe returns **EXIT=1**: the installed EPR lacks
the required native feedback interface (`installed-probe.json`). The explicit
candidate pair returns **EXIT=0**, with `admitted:false` and `fixtureOnly:true`
(`candidate-probe.json`). These are CheckWitness records, not another report
schema or a new acceptance authority.

## Findings preserved

An early pre-review default invocation passed through the shared launcher and
created the session's EPR pin at 18:56:46Z. The native history was not erased.
The parent retained `early-probe-pin.json`; current heads did not move. The fixed
path uses a direct resolver, the launcher-only regression proves non-execution,
and `probe-pin-preservation.json` confirms unchanged current heads and existing
session pins for the corrected live run.

Review also caught a false-green scope check: an occurrence string in
`classifiedAs` cannot prove the Intent's `inScopeOf`. The exact-byte assertion
and wrong-scope/extra-act regression now preserve that distinction. A final minor
fix separates native head absence from incompatibility. Intermediate gate and
probe logs remain beside the successful results, including the duplicate-literal
lint failure and the pre-review witnesses. No failed run was relabeled green.

Blind story review: one round, **READY**, correctness 0, interpretability 2,
preference 0. The open interpretability observations are the existing capacity
scenario's threshold/address vocabulary and the new scenario's Brit/EPR,
native-proposal and installed-delivery vocabulary. No correctness finding remains;
these observations are recorded rather than silently discharged.

No commit, global promotion, release, human acceptance or experienced recovery
is claimed. The shared Rust feedback implementation and unrelated source work
were left intact. The final native station commitment is
`bafyreidbbxdpcjkzr22t26taltgfupndmbbuwaffyotprx76hnngka222m`.
