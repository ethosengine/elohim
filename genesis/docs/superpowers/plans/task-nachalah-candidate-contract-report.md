---
id: nachalah-candidate-contract-report
title: Non-actuating candidate contract verified after prerequisite test repair
status: DONE_WITH_CONCERNS
class: protocol-canonical
gap: nachalah-supervised-activation#1a
actor: agent:implementer@gpt-6
habits: [runtime-upgrade-propagation]
commits: []
cites:
  - "nachalah-candidate-contract-brief | Authenticate a conductor candidate without granting activation | sha256:2a4a9437f448a85f | path: genesis/docs/superpowers/plans/task-nachalah-candidate-contract-brief.md"
---

Base: `7fb0e9d0c`. No staging, commits, pushes or process-control changes were made.

Implemented the pure candidate content contract in `elohim/ark/core/src/candidate.rs`,
its module/purity-scan registration, six integration contract tests in
`elohim/ark/core/tests/candidate_contract.rs`, and three core seam-registry entries.
The result is `AuthenticatedCandidateContent`, with private fields and read-only
accessors. It confers no activation authority. The existing EPR verifier checks
canonical CID and detached Ed25519 signature against an independently supplied
issuer/schema pin; no new dependency or identity codec was introduced.

The signed content binds release action, grant entry AND exact grant action,
berth/incarnation/child, predecessor manifest and executable, candidate manifest
and executable, platform, validity and protocol capabilities. Independent expected
candidate manifest and observed executable must match. Version labels are
informational and do not decide freshness or compatibility. Both runtimes must
explicitly read and write the independently observed, identical database format.
Holochain hash strings receive lexical screening and exact independent comparison;
this does not verify their checksum, author or notarization.

Tests sign real fixtures before tampering, recompute a tampered CID without
repairing its signature, substitute an independent second key, vary independently
observed targets/predecessor/database/capabilities, and refuse incompatible signed
content, unknown schema/domain/fields and oversized payloads. Refusal labels have
an independent golden vocabulary. Candidate validation is bounded and repeatable.

Gate evidence: `just gate elohim-ark` — `EXIT=1`.
First run log: `/tmp/nachalah-candidate-contract-gate.log`.
Format and clippy passed; all six candidate contract tests passed. Existing test
`shutdown_sends_policy_signal_then_kills_after_grace` failed its timing ceiling:
“the child outlived its grace period by 1.045237673s”.

Gate evidence (one repeat to investigate that failure): `just gate elohim-ark` — `EXIT=1`.
Repeat log: `/tmp/nachalah-candidate-contract-gate-retry.log`.
Format and clippy passed; all six candidate contract tests passed again. Shutdown
timing test passed this time, but existing
`missing_driver_identity_never_falls_back_to_the_spawn_digest` and
`service_readiness_cannot_certify_a_different_executable` failed at
`elohim/ark/supervisor/tests/supervise_death.rs:813`, asserting
`witnesses[0].verdict.is_none()`. This suggests a witness-observation race; no
root cause is asserted as proven and the owning gate remains red.

Additional evidence: scoped `rustfmt --edition 2021` — `EXIT=0`; Python
`jsonschema.validate` against the existing seam-registry schema — `EXIT=0`.
Cargo berth `nachalah-candidate-contract-codex` was claimed before each gate and
released afterward. Elevated host checks confirmed competing Cargo finished
before either gate began; no other session's process was stopped.

The scoped implementation is ready for review, but #1a is not fulfilled while its
required gate is red. Supervisor prerequisite repair belongs in a separate scoped
change, followed by the owning gate. C12 remains explicitly unbound: station #1b
must verify exact notarized grant action/entry/author, issuer, recipient, target
scope, validity and revocation at the acting node. No activation vehicle, staging,
rollback journal, live household run or adoption attestation was implemented.
Earned-arc actuation remains disabled; parent station 1 and the sprint remain open.

## Resumed seat — supersedes the prior HOLD, 2026-09-06

Status: DONE_WITH_CONCERNS for #1a only. The earlier HOLD and its observation
remain evidence of the two failed owning gates; this section supersedes their
completion status without removing that history.

Root repaired only the prerequisite supervisor tests: same-timestamp witnesses
are selected by verdict state rather than arbitrary CID order, and shutdown
checks preserve the minimum grace interval separately from the bounded whole-run
I/O/poll/reap completion budget. This seat did not edit those tests or commit them.

Gate evidence: `just gate elohim-ark` — `EXIT=0`.
Log: `/tmp/nachalah-candidate-contract-gate-resumed.log`.
Format, clippy and all 126 tests passed, including all six candidate contract
tests and the eight supervisor lifecycle tests. Shared Cargo capacity was checked,
claimed and released for this run. No scoped commits were made by this seat.

The authenticated-content substation is complete and reviewable. Concerns remain
exactly outside its bounded claim: notarized grant proof (#1b), process activation,
durable rollback and repeated household ceremonies are still unimplemented here.
A valid candidate signature still grants no process permission, and no authority
or earned-arc actuation floor has been weakened.
