---
id: "backlog-task-peer-carried-sweep-mesh-receipt"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Task: sweep-exercise the peer_carried election-supply arm on the mesh — a gossip-dead conductor's storage adopts the elected head via a peer-carried record, driven by the reconcile sweep, not an explicit call"
slug: "task-peer-carried-sweep-mesh-receipt"
written: "2026-09-01"
author: "session-2026-09-01-integrator"
status: "open"
priority: "high"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-mesh-fixture-fidelity-regimes"
  - "backlog-late-joiner-peer-discovery-boot-only-board"
  - "habit:dataplane-convergence"
tags: [dataplane, election, carried-record, mesh, fixtures, receipt, delegable]
claimedBy: "codex"
---

**Claimable by any implementation agent. Fixture/probe work over a landed
mechanism — `carried-election-mesh-proof.ts` proved the arm cross-conductor by
EXPLICIT calls; the fleet is waiting on the SWEEP taking that path
organically, and the mesh has never exercised it.**

## Why

The habit ledger (dataplane-convergence, 2026-08-31) records: "the
peer_carried supply arm (for gossip-dead conductors) is proven
cross-conductor but not yet sweep-exercised (mesh gossip works — the mesh
cannot fake the fleet's arc-Empty regime)." The fleet enactment is live
(ELOHIM_OBEY_CARRIED_ELECTION=true on 7/7, supplier W2 standing) and the
first `elohim_content_election_obeyed_total{path="peer_carried"}` observation
is the top red's missing receipt. The mesh CAN fake the one thing the arm
needs — a conductor that cannot answer the election query — by taking the
adopting peer's conductor down (or otherwise making its DHT reads fail) while
its storage keeps sweeping. That is regime 2 of
`mesh-fixture-fidelity-regimes` at the fidelity level this arm requires:
"conductor cannot supply," which arc-Empty and a stopped conductor share.

## P2P design-gate decision

- **Classification:** Ephemeral (C) — fixture staging + probe. The elected
  head, election evidence, and adoption path all exist; nothing new is
  persisted or notarized by this task. DNA-hash-neutral, zero head-plane cost.
- **Identity/address:** existing content ids and `uhCkk…` action hashes from
  the carried-election arc; no new identity or join.
- **Concern canon:** C4 = the probe must distinguish "adopted via
  peer_carried" from "adopted via the DHT election that happened to recover"
  — the receipt is the LABELED counter (`path="peer_carried"`) moving plus
  the served head, never the head alone; C5 = the carried record is verified
  in wasm by the adopter's own conductor when reachable — if the fixture
  removes the verifying conductor entirely, the probe must exercise whatever
  verification posture the production arm actually has and record it
  honestly, not assert around it; C14 = a sweep that never takes the arm
  exits nonzero naming where it stalled. Remaining concerns n/a.

## Scope

1. New probe `genesis/a2o/scripts/peer-carried-sweep-receipt.ts` (tsx,
   composing the authoring steps from `carried-election-mesh-proof.ts` rather
   than re-deriving them), against a 3-peer mesh with
   `ELOHIM_OBEY_CARRIED_ELECTION=true` exported for the run:
   - Author divergent declared heads for one id on two peers; declare the
     EARNED canonical on peer A (the supplier) — the proof script's stages
     (1)-(2).
   - Fixture the regime: make the ADOPTING peer's (B's) conductor unable to
     supply the election — stop its conductor process (the harness already
     restarts arms independently; a stop is the same lever half). B's storage
     keeps running and sweeping.
   - Do NOT call any election function on B. Wait on B's reconcile sweep
     (bounded: a few sweep cadences) to source the hint from A's inventory,
     fetch the carried record, and adopt.
   - Receipt = B's `elohim_content_election_obeyed_total{path="peer_carried"}`
     incremented AND B serving the elected head, with the pre/post counter
     values and time-to-adopt printed. Exit 0 only on both; exit 2 naming the
     stalled station (no hint / hint-no-fetch / fetch-no-obey / obey-no-serve).
   - Restore B's conductor afterward (clean teardown).
2. If the sweep structurally cannot reach the arm without a code change (the
   supply arm is only plumbed for a shape the mesh can't produce), STOP at
   the probe evidence and report the missing station as a story-graph node
   (chain / between sweep-hint→peer_carried-adopt / missing node: assertion +
   probe + current state) — do not modify `services/head_adoption.rs` or any
   sweep production logic under this task.
3. Close the loop: append the receipt as a one-line DELTA (NO status flip —
   flip authority is the fleet observation) to
   `elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md` and run
   `.claude/scripts/habits-project.py`.

## Disjointness contract

- MAY create `genesis/a2o/scripts/peer-carried-sweep-receipt.ts`, edit this
  atom, and append to the habit ledger.
- MUST NOT edit any Rust source (in particular `services/head_adoption.rs`,
  the reconcile sweep, or election/coordinator zomes), any DNA workspace,
  `carried-election-mesh-proof.ts` (frozen oracle for the 2026-08-31 shift),
  `dataplane-convergence-measure.ts` (frozen fleet oracle),
  `app/elohim-app/scripts/hc-mesh.sh` (the late-joiner task owns harness
  edits — use its existing verbs), any Jenkinsfile, or deployment manifests.

## DoD + verification

- `peer-carried-sweep-receipt.ts` exits 0 on a fresh mesh run: counter
  labeled `path="peer_carried"` moved on the adopter, elected head served,
  stations and timings printed. 2 consecutive runs (fresh id each).
- The negative control is stated in the transcript: with B's conductor UP,
  the same flow adopts via a non-peer_carried path (or the run records why
  the label is unaffected) — proving the fixture, not luck, selected the arm.
- Habit DELTA appended; `habits-project.py --check` clean.
- The fleet observation (W2 poll → obeyed{path="peer_carried"} on alpha →
  both doorways on uhCkk1kms…) remains the integrator's watch via
  `dataplane-convergence-measure.ts` — do not claim it here.

## 2026-09-01 attempt — structural stop

`peer-carried-sweep-receipt.ts` now stages fresh divergent heads, proves the
conductor-up negative control, records the labeled counter deltas, pauses the
adopter conductor while storage remains the sweep owner, and always resumes it.
The negative control converged to the elected head with
`peer_carried_delta=0`.

The proposed stopped-conductor regime is not equivalent to the fleet's
arc-Empty regime. In `try_obey_visible_election`, a local election answer of
`Absent` is the ONLY branch that calls `try_carried_election_supply`; an
`Unreachable`/blocked conductor returns before the peer fetch. Even the
`Absent` branch then needs that same local conductor for
`verify_carried_election` and `validate_carried_head_record`. Removing or
pausing the conductor therefore removes both the election supplier AND the
required verifier; the sweep cannot reach `path="peer_carried"` under this
fixture without changing production logic, which this task forbids. The run
stopped without a receipt or habit-status flip.

**Story-graph node:** chain / between sweep-hint→peer_carried-adopt / missing
node: fixture an observed-absent canonical-election read while the adopter
conductor remains callable for `verify_carried_election` +
`validate_carried_head_record`; assertion = hint observed, local answer absent,
peer evidence fetched, wasm verification succeeds, `peer_carried` increments;
probe = `peer-carried-sweep-receipt.ts`; current state = stopped/paused
conductor makes the local answer unreachable or blocked and the arm stalls
before peer fetch.
