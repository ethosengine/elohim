---
title: "History: The recovery-protocol design arc (Jan – Apr 2026)"
id: doorway-recovery-protocol-arc
type: history-gotcha
status: noted
tier: history
created: 2026-06-11
topic: [recovery, shard-tracking, social-recovery, imagodei, node-registry, doorway, design-arc]
# Provenance breadcrumb: the two retiring island docs this record distills (dates are git first-commit; both last revised 2026-03-11).
derived_from:
  - doorway/doorway-service/RECOVERY-PROTOCOL.md     # retired to git 2026-06-11 (doorway island recompose; authored 2026-01-01)
  - doorway/doorway-service/RECOVERY-SPRINT-PLAN.md  # retired to git 2026-06-11 (doorway island recompose; authored 2026-01-01)
canonical:
  - genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md
  - genesis/docs/content/elohim-protocol/resilience/README.md
cites:
  - elohim/holochain/dna/node-registry/zomes/node_registry_integrity/src/lib.rs
  - elohim/holochain/dna/node-registry/zomes/node_registry_coordinator/src/lib.rs
  - elohim/elohim-storage/src/node_registry_api.rs
  - elohim/elohim-storage/src/http.rs
  - elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs
  - elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs
  - recovery-protocol-phase-2-revised-design | the superseding spec — graduated RecoveryAuthority (five layers, lockout-is-design-failure) replaced this arc Layer-4 Shamir-first gates before implementation | sha256:9d1844484ed64de4 | path: genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md
  - genesis/data/timeline/backlog/lift-wip-revocation-self-full-recovery.md
  - app/elohim-app/src/app/imagodei/services/recovery-coordinator.service.ts
  - doorway/doorway-service/Cargo.toml
  - doorway/doorway-service/src/nats/client.rs
  - doorway/doorway-service/src/orchestrator/nats_provisioning.rs
memory_anchors: []
---

# History: The recovery-protocol design arc (Jan – Apr 2026)

> **Hot-context pointer (the one sentence to remember):**
> Layer 3 (shard tracking) shipped exactly as drawn — rare — while Layer 4
> (N-of-M challenge/response social recovery) was **superseded on paper before a
> line of it was implemented**: the live recovery model is graduated authority
> (five `RecoveryAuthority` layers, "absolute lockout is a design failure"), not
> Shamir-first crypto gates.

## Layer 3 shipped FULLY as drawn (name the rare win)

RECOVERY-PROTOCOL.md's shard-tracking layer is live file-for-file:

- `ShardAssignment` entry + `ShardStatus`/`ShardingStrategy` enums:
  `elohim/holochain/dna/node-registry/zomes/node_registry_integrity/src/lib.rs:184-228`
  (link types `ContentToShardAssignment`/`CustodianToShardAssignment` at :253-254).
- Coordinator fns `create_shard_assignment` / `get_shard_assignments_for_content` /
  `get_shard_assignments_for_custodian`:
  `elohim/holochain/dna/node-registry/zomes/node_registry_coordinator/src/lib.rs:859,922,947`.
- elohim-storage registers assignments per shard after Reed-Solomon encode:
  `elohim/elohim-storage/src/http.rs:1798` via the client at
  `elohim/elohim-storage/src/node_registry_api.rs:82`.

The sprint plan's Phase 1 checkboxes (all `[x]`) verify true against the tree —
its Phases 2-5 checkboxes (all `[ ]`) are honest too: none of that shipped as drawn.

## Layer 4 superseded in design before implementation

The original challenge/response flow (`RecoveryRequest` → `RecoveryChallenge` →
`RecoveryAuthorization`, N-of-M) never reached the DNA. The superseding spec —
`genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md`
("Graduated Authority, Not Shamir-First"; explicitly "Builds on:
doorway/doorway-service/RECOVERY-PROTOCOL.md (Jan 2026) phase boundaries", :13) —
replaces single-path crypto gates with five graduated `RecoveryAuthority` layers
(spec :118-143), commits that absolute lockout is a design failure (:480), and
deleted three M1 entry types shipped in error (`RecoverySeedCommitment`,
`HeldRecoveryShare`, `MyRecoveryAuthorization` — spec §5.5, :380). In the DNA today:
`RecoveryRequest` is demoted from DHT entry type to a plain signal-emission struct
("Removed from EntryTypes in Recovery M4 Task 15",
`elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs:548-556`; the
Stage-G deferral list at :422); `RecoveryChallenge`/`RecoveryAuthorization` were
never created (zero hits in `elohim/holochain/dna/`).

## The original Phases 3-5, distilled

- **Phase 3 — reconstruction coordinator in doorway**: `RecoverySession` state,
  parallel shard fetch from custodian DIDs, RS-decode, progress API. Not built in
  doorway; phase-2-revised defers content-shard reassembly to *its* Phase 3 (:60).
- **Phase 4 — work-while-recovering**: ContentResolver checks the active
  RecoverySession, prioritizes user-requested content, reconstructs on demand.
  Not built; the UX principle (don't block on full recovery) travels with the
  revised spec's graduated model.
- **Phase 5 — verification and drills**: daily custodied-shard verification,
  distribution-health scoring, recovery drills with readiness scores. Not built;
  no current canonical home names drills. OPEN QUESTION: does the drill/verification
  vision get a home under the resilience epic, or is it absorbed by tiered-quilt
  attestation work?

Substantial recovery substrate landed later under the revised spec's umbrella —
M3 intimate-witness quorum (`create_recovery_request` + `submit_intimate_witness` +
`commit_key_rotation`), Shamir transport and custody manifest
(`elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs`), and the Angular
coordinator (`app/elohim-app/src/app/imagodei/services/recovery-coordinator.service.ts`).
The verified-open BDD remainder (full lockout-recovery scenario, @wip) is ALREADY
tracked in `genesis/data/timeline/backlog/lift-wip-revocation-self-full-recovery.md`
— substrate ready, orchestration steps not; the gap lives there, not here.

## NATS aside (mechanism-honest)

Sprint-plan Phase 3 said "wire up NATS for recovery signals". async-nats IS live in
doorway (`doorway/doorway-service/Cargo.toml:56`, `src/nats/client.rs`) — but for
orchestrator provisioning (`src/orchestrator/nats_provisioning.rs`), not recovery
signals.
