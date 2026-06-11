---
id: "backlog-doorway-recovery-reconstruction-residue"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Recovery sprint-plan Phases 3-5 residue: shard-verification fns are zero-consumer, recovery drills are unimplemented-but-cited-as-existing"
slug: "doorway-recovery-reconstruction-residue"
written: "2026-06-11"
author: "claude (doorway island recompose)"
status: "backlog"
priority: "low"
tags: [recovery, shard-verification, recovery-drills, node-registry, doorway, residue, zero-consumer]
derived_from:
  - doorway/doorway-service/RECOVERY-SPRINT-PLAN.md   # retired to git 2026-06-11 (doorway island recompose) — Phases 3-5 "Not Started" status table + task checklists
  - doorway/doorway-service/RECOVERY-PROTOCOL.md      # retired to git 2026-06-11 (doorway island recompose) — Phases 3-5 full designs; arc preserved in 2026-06-11-doorway-recovery-protocol-arc.md
cites:
  - doorway-recovery-protocol-arc | the history arc distilled from the same retiring sprint plan — full Phases 3-5 design context lives there | sha256:651437c4d2610b79 | path: genesis/docs/content/elohim-protocol/history/2026-06-11-doorway-recovery-protocol-arc.md
  - recovery-protocol-phase-2-revised-design | current canonical recovery design; defers content-shard reassembly to its own Phase 3 | sha256:9d1844484ed64de4 | path: genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md
  - genesis/data/timeline/backlog/lift-wip-revocation-self-full-recovery.md
  - elohim/holochain/dna/node-registry/zomes/node_registry_coordinator/src/lib.rs
  - elohim/elohim-storage/src/node_registry_api.rs
  - elohim/elohim-storage/src/http.rs
  - elohim/elohim-storage/src/sharding.rs
  - elohim/elohim-storage/src/shard_service.rs
  - elohim/elohim-storage/src/services/distribution_view.rs
  - genesis/docs/superpowers/specs/2026-05-02-blob-custody-reconciliation-design.md
  - qahal-homepage-ux-design | the live spec whose Sprint-3 drill UI builds on the false drill-already-exists claim this entry corrects | sha256:2ce1cfd684d41eea | path: genesis/docs/superpowers/specs/2026-05-22-qahal-homepage-ux-design.md
  - doorway/doorway-service/src/orchestrator/disaster_recovery.rs
shift_objective: |
  Close the two untracked Phase-5 residue items from the retired recovery sprint plan:
  (1) either wire a periodic custodied-shard verification job that calls node-registry's
  existing-but-orphaned `update_shard_verified_at` / `update_shard_status` coordinator fns
  (elohim/holochain/dna/node-registry/zomes/node_registry_coordinator/src/lib.rs:979,991)
  from elohim-storage, or explicitly fold shard-integrity verification into the custody-
  reconciliation layer's design and retire the orphaned fns; (2) implement (or descope) the
  recovery-drill operation that genesis/docs/superpowers/specs/2026-05-22-qahal-homepage-ux-design.md
  (:246, :620) wires UI to as if it already exists — repo-wide there is no drill implementation.
  Correct the qahal spec's "the drill operation itself already exists" claim either way.
---

# Recovery sprint-plan Phases 3-5 residue (verify-first audit, 2026-06-11)

The retiring `doorway/doorway-service/RECOVERY-SPRINT-PLAN.md` listed Phases 3-5 as "Not Started."
Per-item audit before retirement — most items are implemented elsewhere or tracked open; **two are
genuinely untracked residue** (bolded (c) rows).

| Sprint-plan item | Verdict | Evidence |
|---|---|---|
| P3.1 `RecoverySession` doorway coordinator | (b) tracked-deferred | Zero hits for `RecoverySession` in doorway/storage/app src. Deferred live: spec 2026-04-22-recovery-protocol-phase-2-revised-design.md §2.2 "Content shard reassembly — Phase 3". Full design preserved in git via the retired RECOVERY-PROTOCOL.md (arc: 2026-06-11-doorway-recovery-protocol-arc.md). |
| P3.3 shard fetching `GET /api/v1/shards/{hash}/{index}` | (a) substrate-implemented, differently | No such HTTP route (only a path-class comment at doorway-service/src/server/http.rs:3734). Shard fetch/probe/push/inventory is P2P-native: transport-neutral `ShardService` (elohim/elohim-storage/src/shard_service.rs) over libp2p (src/p2p/shard_protocol.rs) and iroh (src/p2p_iroh/shard_backend.rs). |
| P3.4 RS reconstruction | (a) implemented | `reconstruct()` at elohim/elohim-storage/src/sharding.rs:301 + parity-drop tests (:524, :556). |
| P3.2/3.5 NATS recovery signals + `/api/v1/recovery/{session}` endpoints | (b) deferred with P3.1 | No recovery session endpoints exist. Adjacent-but-different machinery exists and is recorded, not blessed: doorway's NATS `DisasterRecoveryCoordinator` (src/orchestrator/disaster_recovery.rs, started via orchestrator/mod.rs:384 when `--orchestrator-enabled`) coordinates node-failure content replication; its heartbeat trigger is stubbed (orchestrator/heartbeat.rs:250-260, zome call commented out). |
| P4 work-while-recovering | (b) documented, blocked-by P3 | Zero code (`work.while.recovering` → only docs). Design preserved in git via the retired RECOVERY-PROTOCOL.md "Phase 4: Work While Recovering" (:552, :862; arc: 2026-06-11-doorway-recovery-protocol-arc.md). Meaningless before reconstruction exists. |
| **P5.1 periodic shard verification** | **(c) untracked — zero-consumer machinery** | See below. |
| P5.2 distribution-health analysis | (a) implemented, different shape | elohim/elohim-storage/src/services/distribution_view.rs composes `DistributionSummary` / `ReplicaHealth` / `FaultDomainDiversity` / `PlacementGapRow` (Category C per 2026-05-01-light-up-the-topology-design.md); custody-presence drift via 2026-05-02-blob-custody-reconciliation-design.md (peer_blob_inventory + placement-gap signals). |
| **P5.3 recovery drills (+ P5.4 per-human readiness indicator)** | **(c) untracked — and falsely cited as existing** | See below. |

Identity/key recovery itself (sprint-plan Phase 2) is NOT residue: superseded and delivered by the
revised Phase 2 spec's M1-M4 arc (RecoveryRequest → IntimateQuorum → KeyRotation, fast-path
revocation, Shamir transport) — substrate-readiness inventory in
genesis/data/timeline/backlog/lift-wip-revocation-self-full-recovery.md; only BDD orchestration
remains open there (already tracked, not re-forked here).

## (c)-1 — Shard verification: coordinator fns with zero callers

Phase 1 of the sprint plan (Done) shipped `update_shard_status` and `update_shard_verified_at` in
the node-registry DNA (elohim/holochain/dna/node-registry/zomes/node_registry_coordinator/src/lib.rs:979,
:991) and a `ShardStatus` enum including `Stale`/`Reconstructing`
(elohim/elohim-storage/src/node_registry_api.rs:19-25). The write path is live —
elohim-storage calls `create_shard_assignment` after RS encoding (elohim/elohim-storage/src/http.rs:1798) —
but **no code anywhere calls the two update fns** (grep across elohim/, doorway/, app/: zero hits
outside the zome). The Phase 5.1 "daily job verifies custodied shards, touches verified_at, marks
Stale, triggers re-replication" was never built; the implemented custody-reconciliation layer covers
custody *presence* (inventory gossip + placement gaps), not shard *integrity* re-verification.
Recorded as zero-consumer machinery: either wire a verifier that calls these fns, or fold integrity
verification into the custody-reconciliation design and retire them.

## (c)-2 — Recovery drills: unimplemented, but a live spec builds UI on them

Repo-wide, no drill exists: `recovery_drill`/`readiness_score`/`drill` → zero hits in
elohim-storage, DNA zomes, doorway, app, steward sources. Yet
genesis/docs/superpowers/specs/2026-05-22-qahal-homepage-ux-design.md (:239-246, :620 item 5)
plans the Qahal homepage Sprint-3 drill UI on the claim "The drill operation itself already exists
(recovery-m4-* specs)" — the recovery-m4 plans are archived out of tree and M4 was fast-path
revocation, not drills. When that sprint is scheduled it will hit a nonexistent operation.
The per-human "recovery readiness indicator" (P5.4) depends on this and is equally absent.

**Priority justification (low):** nothing user-blocking — key recovery substrate is delivered, and
custody-presence health is live in the topology surface. Bump to medium when (a) the qahal-homepage
Sprint 3 is scheduled (drill UI dependency becomes real) or (b) Phase 3 reconstruction work starts
(verification feeds reconstruction triggers). OPEN QUESTION: should Phase-3 reconstruction, when
picked up, live in elohim-storage rather than doorway? The retired RECOVERY-PROTOCOL.md's
doorway-orchestrated shard fan-out predates (and now contradicts) doorway/CLAUDE.md's "No Blob
Fan-Out — single-target dispatch" rule; the substrate owns byte mobility.
