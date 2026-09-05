---
title: "Dataplane convergence — the final federation-deploy scenario measured: two doorways that disagree converge on the elected head without a re-upload"
id: dataplane-convergence-final-scenario-plan
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
domain: D2
habits: [dataplane-convergence]
graduation-trigger: the federation-deploy final scenario runs un-tagged on the household mesh with a receipt that proves the organic path (no declaration call, no per-host upload) and both doorways serve the same head; the habit's DELTA cites it
topic: [dataplane, head-convergence, carried-election, doorway, reconcile, a2o]
informed-by:
  - genesis/a2o/features/dataplane/federation-deploy.feature (the final `@wip` scenario is the standing red)
  - genesis/docs/superpowers/plans/2026-09-02-dataplane-pain-points-sprint-plan.md (T7: two green rows, one head, two blob pointers)
  - genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
  - genesis/a2o/scripts/carried-election-mesh-proof.ts (2026-08-31: stages divergent declared heads and observes both peers converge)
cites:
  - genesis/a2o/features/dataplane/federation-deploy.feature
  - "dataplane-pain-points-sprint-plan | Dataplane pain-points sprint | sha256:5c77703d703039af | path: genesis/docs/superpowers/plans/2026-09-02-dataplane-pain-points-sprint-plan.md"
  - elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md
  - genesis/a2o/scripts/carried-election-mesh-proof.ts
  - genesis/a2o/steps/dataplane/resiliency-saga.steps.ts
  - elohim/elohim-storage/src/services/head_adoption.rs
  - elohim/elohim-storage/src/sync/projector.rs
---

# Dataplane convergence — the final scenario, measured

## Why this plan exists

`dataplane-convergence` is the register's top red, and a read of its evidence (2026-09-05, Codex)
found that the red is **unmeasured, not failing**: the habit's primary check is federation-deploy,
whose final scenario ("two doorways that disagree about a page converge on the elected version
without anyone re-uploading it") is still tagged `@wip` and therefore skipped by every mesh run,
while the head-convergence saga beside it passes all seven stations including same-head doorway
serving, and the 2026-08-31 mesh proof already stages divergent declared heads, verifies the carried
election in the receiving conductor, rejects tampering, and observes both peers converge. What is
missing is a fixture that binds the story's own final assertions to that proven path. A second,
narrower finding (pain-points T7) is the last observed visitor-facing mismatch: the heads agree
but a projected blob pointer drifted, which the final scenario's "same head" assertion would not
catch — the receipt must assert served-versus-declared per doorway too.

## Global Constraints

- The receipt must prove the ORGANIC path: no declaration call from the fixture, no per-host
  upload, no doorway credential; a fixture that false-greens by checking only the final head is a
  regression, not a pass.
- Evidence travels through the peer's own conductor (the carried election is re-derived in wasm
  on the disagreeing peer); every stamp guard (never-move-backwards, tier precedence) stays untouched.
- The capability ships dormant behind `ELOHIM_OBEY_CARRIED_ELECTION`; the fixture enables it on the
  household mesh for the run and restores the config bytes after.
- One implementer per crate; claims, fulfils and verdicts through `epr flow`; the mesh is measured
  only when no other seat holds it.

---

### Task 1: bind the final scenario to the proven path

**Files:**
- Modify: `genesis/a2o/features/dataplane/federation-deploy.feature` (remove `@wip` from the final
  scenario once its steps are real; add the served-versus-declared line per doorway)
- Modify/Create: `genesis/a2o/steps/dataplane/federation-deploy.steps.ts` (bind every Given/When/Then
  of that scenario: stage the divergence with the 2026-08-31 proof's staging helpers
  (`carried-election-mesh-proof.ts`), enable the flag through the runtime-config the fixture already
  byte-restores, wait for the ORGANIC reconcile sweep (never call the declaration route), then assert:
  the disagreeing peer's served head equals the earned-tier elected head; both doorways serve the same
  head for `elohim-host-landing`; each doorway's served head equals its storage-declared head
  (`serverBlobHash` polled, not assumed); the conductor verified the carried link in wasm (the
  `obeyed{path="peer_carried"}` counter moved); reuse the cross-peer and doorway assertions in
  `resiliency-saga.steps.ts` and `dataplane.steps.ts` rather than writing new rails)
- Test: cucumber dry-run clean; then the live run on the household mesh with the receipt under
  `genesis/a2o/reports/`.

- [ ] **Task 1 deliverable: the final federation-deploy scenario runs un-tagged on the household mesh and its receipt proves organic convergence with both doorways serving the same head; the habit's DELTA cites the receipt.**

### Task 2: pain-points T7 — heal the blob pointer from the head record

**Files:**
- Modify: `elohim/elohim-storage/src/services/head_adoption.rs` (when a conductor-verified head record
  names the row's EXISTING declared head, refresh the row's blob pointer from that record without
  changing `dht_anchor_hash`; exact-head equality and own-conductor verification are mandatory — never
  an authority-laundering path), `elohim/elohim-storage/src/sync/projector.rs` (the guard stays
  green-inviolable for any head MOVE)
- Test: unit tests for the pointer refresh (same head → pointer refreshed; different head → refused by
  the existing guard; unverified record → no change).

- [ ] **Task 2 deliverable: two peers holding the same notarized head with drifted blob pointers converge on one pointer after a sweep, with no head move and no per-host write; the final scenario's served-versus-declared assertion holds on both doorways.**

---

## Self-review (2026-09-05)

- Task 1 is fixture integration of a proven mechanism; its risk is evidentiary and the constraints
  name the false-green shapes. Task 2 is the narrow product patch the last real mismatch needs.
- The habit's other checks (content-sync, blob-replication, transport parity, latency scoreboard,
  trust-priced sync) are not this plan's; the flip this plan buys is the primary check.
