---
id: "backlog-shift-launcher-lease-and-session-adaptation"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Bounded shift launcher is dormant until it speaks dev's steward-scoped lease protocol and native session envelope"
slug: "shift-launcher-lease-and-session-adaptation"
written: "2026-10-28"
author: "port/b-autonomous pre-landing review (steward of record: human:matthew)"
status: "backlog"
priority: "high"
jobs: []
relatedNodeIds: []
tags: [agentic, shift, lease, session, dormant]
---

# Shift launcher: what remains before it may run unattended

The launcher ported from `feat/autonomous-delivery-20261022` (`genesis/agentic/shift*.py`, `SHIFT.md`)
is landed but **dormant**: the devfile `postStart` entry `recover-authorized-shifts` is removed (the command
stays defined, run by hand), and nothing else registers it. Pre-landing review (Codex, three lenses, each
finding refuted-then-grepped) found three majors that the port did not adapt; each must be fixed and proven
before it is re-armed.

1. **No `dev-integration` lease for publication.** `genesis/agentic/shift_lease.py:37` delegates only
   `{cargo, mesh, disk-heavy}`; `.husky/pre-push.bash:261` needs `dev-integration` (plus tag-dependent
   `alpha-fleet` / `jenkins-dispatch`). The push worker inherits the guardian proxy, so pre-push refuses.
   Needed: delegate the full steward-scoped lease set dev's pre-push claims.
2. **Synthetic session conflicts with native session ownership.** `shift_supervisor.py:154` moors a synthetic
   `shift-<id>` session; `elohim/eprfs/epr-cli/src/session_envelope.rs:353` refuses with
   `envelope-owner-mismatch`. Needed: bind readiness and guardian processes through the current native session
   lifecycle instead of minting a session id.
3. **Readiness baseline lacks host lease delegation.** `shift.py:117` and the mocked `shift_lease_test.py` hide
   that nested berth checks read namespace-local PIDs as dead. Needed: delegate through the host lease and
   replace the mock with a real nested-namespace check.

Re-arm only when all three have tests that do not mock the lease/session layer, then restore the `postStart`
entry. Habit context: `measure-runs-on-a-peer`, `dataplane-convergence` evidence 2026-10-22/23 describe the
unfinished publication/CI/rehearsal obligations.

## 2026-11-02 prerequisite repair (local, awaiting integration)

The Brit native-source sprint identified the three blockers before attempting
its required shift. The launcher now opens a native process-owned envelope for
readiness and each guardian incarnation (boot, PID and start time), retains the
native open/close receipts and leaves the enclosing runtime's envelope unchanged.
The readiness wait loop serves the same authenticated host lease checks as the
guardian. Publication leases and `lane:<id>` are explicit declared resources;
delegation reads the native steward verdict, preserves the acquired generation,
and never forwards a new offline override or renewal.

Independent review found an additional expiry seam: a quiet publication worker
could outlive its lease, and a completion between polls could retain a successful
receipt. The guardian now arms its deadline at the shortest original lease budget,
checks expiry before consuming a completed worker, and refuses late success on
receipt creation and recovery. The readiness baseline uses the same budget.

Focused proof is in `genesis/agentic/shift_lease_test.py`: the existing native
envelope fixture enrolls real disposable devices, declares their peer endpoints,
and uses real `/proc`, native berth and a contained PID namespace. The first
device can read and retain its publication leases; adding a second admitted
device with no acknowledged head produces the native `unconfirmed` refusal while
leaving the lease bytes intact. The native scenario, quiet-worker expiry and
late-success recovery checks passed together (3 tests, 61.750 seconds). The
final owner gate passed 20 JavaScript tests, 161 shift tests (including the 25
`shift_process_test.ProcessRecovery` cases), and 137 berth tests. Its retained
log is `.claude/shifts/brit-native-loop-20261102/agentic-shift-attempt-3.log`;
this local evidence is not an unattended-delivery claim. A first final gate run
exposed a fixture timing assumption: the second device was still inside the
native confirmation interval. The corrected fixture observes the native
`pending` to `unconfirmed` transition before asserting refusal.

Automatic recovery remains dormant. No shift was started, accepted or published
by this repair. A separate concrete readiness boundary surfaced in the Brit
sprint: native context and habit claims are repository-local, while
`brit-native-daily-driver` belongs to the Brit submodule. The launcher has no
separate native-record root. Copying that habit into the parent would create a
second authority and is not a workaround; the original commitment must resolve
in its owning repository before acceptance-capable dispatch.

## 2026-11-02 explicit local completion boundary

The Brit sprint declares a local offline result, so it cannot truthfully satisfy
the launcher's fleet-specific delivery closure. The frozen `local-delivery` mode
now requires original native acceptance, a currently clean committed revision
(including nested gitlinks), declared owner gates and consecutive objective
measurements at that revision. It refuses household, lifecycle, landing, push
and CI operation kinds, records `local-complete`, and revalidates acceptance and
source/evidence on resume. The existing `delivery` and `rehearsal` boundaries
remain distinct. Independent technical review found no blocking findings.

Proof: `just gate agentic-shift` passed 20 JavaScript tests, 169 shift tests in
319.959 seconds, and 137 berth tests in 139.096 seconds. The exact code/test
candidate stayed unchanged during the run; retained log:
`.claude/shifts/brit-native-loop-20261102/local-delivery-gate.log`. Focused local
closure/readiness tests passed 36 cases; reconciliation, repair and input
compatibility passed 42 cases. No shift or acceptance is established by these
tests.

Python decision registration remains **unbound**: the seam-registry schema and
census accept Rust and JavaScript source/test references, not Python. The
unregistered decisions are `shift_state.py::validate_mode` and
`shift_ops.py::require_completion` / `require_delivery_evidence`. Their concrete
test bindings are `shift_test.py::ContractFixture`'s `test_local_delivery_*`,
`test_local_mode_*` and
`test_missing_unknown_or_mismatched_mode_cannot_resume_as_complete`, plus
`shift_initialization_test.py::Initialization::test_local_delivery_is_frozen_and_forbidden_effects_refuse_before_readiness`.
Extending Python symbol support belongs to the existing census mechanism; no
invalid row, fake JavaScript wrapper or second registry was created here.
