---
title: EPR Content Durability Arc — finish peer replication, healing, sync, aggregates, projection, and federation, proven by resiliency scenarios
id: epr-durability-replication-arc-plan
status: open
class: substrate
sprint: unranked — born 2026-06-10 from the genesis #1118 stabilization session (M1 → substrate-validation suite). Mixed plan — most gaps testable on household-nodes NOW; only cross-doorway breadth diverges (tagged @requires:shem inline). No doc-level requires_env by convention.
cites:
  - genesis/scripts/ci/substrate-verify.sh
  - genesis/Jenkinsfile
  - elohim/elohim-storage/src/reconcile/custody.rs
  - elohim/elohim-storage/src/p2p/mod.rs
  - elohim/elohim-storage/src/http.rs
  - genesis/seeder/src/peer-id.ts
  - genesis/seeder/src/seed-commitments.ts
  - genesis/data/timeline/backlog/ci-genesis-conductor-adminws-unreachable.md
  - genesis/data/timeline/backlog/security-ci-substrate-authorization-grant-coherence.md
  - genesis/docs/superpowers/specs/2026-05-02-blob-custody-reconciliation-design.md
  - topology-resilience-qahal-synthesis | 2026-05-19-topology-resilience-qahal-synthesis | sha256:8f294b7d71bc51a6 | path: genesis/docs/plans/2026-05-19-topology-resilience-qahal-synthesis.md
informed-by:
  - genesis/a2o/features/federation/
---

# EPR Content Durability Arc — dispatch prompt

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development or
> superpowers:executing-plans, task-by-task, two-stage review per task. The
> **p2p-design-gate skill is MANDATORY** before designing any new entity (table, route,
> sync message, commitment kind) in this arc. Story-first: every workstream lands its a2o
> scenario WITH the implementation, same commit.

## Mission

A **durable EPR app**: published content survives the loss of multiple peers because
replication, healing, sync, and projection are *finished* — not demo-passable. Concretely:
custody-sweep replication converges without a CI nudge; healing recovers a wiped peer;
peer counts and free-storage/stewarded-commitment aggregates report truthfully; doorway
federation serves content when a doorway's home peer is gone; and a2o scenarios PROVE
resiliency / failover / backup-restore / recovery on every genesis build.

## Ground truth (2026-06-10 — do NOT re-derive; verify only what's marked open)

The four commits `74e907f6c` / `875e6256c` / `c6b6de1ef` / `dfeebe5dd` (this arc's parent
session) established:

- **Heal-on-read works by design now** (`http.rs` `get_blob_or_heal`): inventory-first,
  falls back to racing connected peers (cap 8) when gossip inventory is empty — gossip
  publish dies on every pod restart (`InsufficientPeers`), so this fallback is what makes
  the CI cross-pod fetch pass. INFO logs name the failing leg; `source` ∈
  inventory|connected-fallback|no-peers.
- **Custody sweep mechanics** (`reconcile/custody.rs:115-147`, timer 120s): loads ALL
  `action='custody-blob'` rows (no state filter), kicks a race-fetch iff
  `commitment.provider == self_cid` AND blob missing. Seeded commitments now carry REAL
  peer ids (`peer-id.ts` Stage 2 resolves from each pod's `GET /p2p/status .peerId`), so
  `provider == self_cid` CAN match for the first time.
- **The open DHT leg (THE first question of this arc):** custody rows are POSTed via
  doorway → they land in *matthew's* SQLite projection only. Jessica's sweep can only kick
  if the row reaches HER projection — i.e. DHT anchor → conductor gossip → her
  `projection_reconcile` ("REA commitments converge from own conductor", `p2p/mod.rs`
  P2PStatusInfo docs, commit d88bba0a1). Known suspect gaps: the CommitmentCommitted
  signal subscription in storage (memory: "2a gap"), and conductor seeding never having
  run from CI (netpol — see below). **Observable now without log scraping:**
  `/p2p/status` exposes `reconcilePassesTotal`, `kicksFiredTotal`,
  `placementGapsEmittedTotal`. `kicksFiredTotal > 0` on jessica = the whole
  manifest→reality chain is alive. Stuck at 0 with passes>0 = the DHT leg is the break.
- **The substrate-validation suite** (genesis/Jenkinsfile stages → all assertions in
  `genesis/scripts/ci/substrate-verify.sh`, JSON artifact per stage): Verify Peer Mesh /
  Upload Blob-Backed Content (manifesto + BUILD-UNIQUE probe blob — fossil-passes are
  impossible by construction; #1100/#1110 "passed" on a stale replica of the previous
  manifesto hash) / Verify Substrate Propagation (polled GET, never HEAD; bytes-on-disk
  via inventory-parity filesystem delta) / Verify Delivery Events (serve-blob REA) /
  Verify Projection Sync / Verify Federation Layer / Verify Resilience Signals.
- **Netpol gate:** jenkins→conductor pod ports 8444/8445 are committed but the file is
  operator-applied (`kubectl apply -f genesis/orchestrator/manifests/network-policies.yaml`).
  Until applied, the three conductor-seed stages (identities / peer bindings / household
  formation) skip — they have NEVER run from CI — and Verify Resilience Signals
  deliberately gates off `CONDUCTOR_SEEDING_READY`. Scoped bootstrap debt; the consistent
  delegates-compute end-state is canonicalized in
  `security-ci-substrate-authorization-grant-coherence.md`.

## Phase 0 — observe before building (first session move)

After the operator netpol apply + the next edge deploy + genesis run:

1. Pull the suite artifacts from the latest `elohim-genesis/dev` build
   (`substrate-verify-*.json`) and each pod's `/p2p/status` counters
   (CI-reachable: `elohim-{matthew,adam,jessica}-alpha.elohim-alpha.svc:8090` from the
   pipeline; from a dev session use the Jenkins artifacts + Loki).
2. Decision tree on jessica's counters:
   - `kicksFiredTotal > 0` and propagation passes → custody chain ALIVE; skip to
     Workstream B/D.
   - `reconcilePassesTotal > 0`, `kicksFiredTotal == 0`, and her `/api/v1/commitments`
     has NO custody-blob rows → **DHT leg broken**: trace anchor→gossip→projection
     (provenance of rows' `dhtAnchorHash`; whether her storage subscribes
     CommitmentCommitted; whether her conductor holds the entries — DHT partition check
     via Verify Peer Mesh version-parity + conductor peering).
   - Rows present but `kicksFiredTotal == 0` → sweep filter mismatch: compare row
     `provider` to her live `peerId` byte-for-byte (resolver fallback may have fired —
     the seeder warns loudly; check seed-stage console).
3. Open ONE canonicalized concern per confirmed break (timeline-CONVENTIONS,
   status-documented), then implement.

## Workstreams (order by Phase-0 findings; each = scenario + implementation + counters)

**A. Custody-sweep convergence (DHT leg).** Commitment rows reach EVERY party's
projection (provider AND receiver) without CI nudging. Likely work: storage-side
CommitmentCommitted signal subscription; verify conductor-first PATCH heal (6ef1d7987)
covers reseeded anchors; assert in CI via per-pod `/api/v1/commitments` in the
propagation preflight (extend substrate-verify.sh).

**B. Healing & backup-restore drill.** RESET_STORAGE already wipes per-pod `content.db`
(genesis Jenkinsfile Reset Storage stage) — that IS the backup-restore scenario: wipe
jessica (CI has pod-ops powers via ee-jenkins, cf. restart-doorway-epr.sh), then prove
she reconverges from the mesh (blobs via sweep/heal, commitments via DHT leg,
projections via reconcile) within a bounded window. New a2o scenario:
`features/federation/peer-recovery.feature` "A wiped device recovers its stewarded
content from the mesh".

**C. Sync & peer counts.** Mesh assertions exist (Verify Peer Mesh); finish: per-peer
`connectedPeers` floors tuned to the real alpha topology (M/J/J live mesh,
hub-optional floor — one device must still function), adjacency both directions, and a
peer-loss tolerance scenario: kill one storage pod mid-build, assert reads still serve
from surviving peers (failover), pod returns and re-syncs.

**D. Free-storage / stewarded-commitment aggregates.** `/api/v1/network/posture`
(`storagePressure`, `householdsReciprocating`) and the resilience snapshot read through
substrate-owned `humans.agent_pub_key + household_id` — the junction no HTTP surface
sets (memory: commitments-only seeding lights NOTHING; `content:<reach>` provide rows
only in test_util — Epic B gap). Conductor seeding (post-netpol) is the designed
filler: verify PeerStatusRecorded → peer-statuses → posture lights up; then make the
"free storage on this mesh / who stewards what" aggregates truthful and assert them in
Verify Resilience Signals. p2p-design-gate before ANY new aggregate entity.

**E. Projection durability.** Verify Projection Sync currently warns on null streams;
finish: every pod's projector lag bounded post-seed, `projection_reconcile.caughtUp`
true across the fleet, and the doorway EprRouter no longer needs the pod-delete crutch
(restart-doorway-epr.sh exists because SSE projection.registered is flaky — root-cause
or formally adopt; a concern either way).

**F. Doorway federation.** Verify Federation Layer asserts self-membership + bootstrap
surface today. Finish: doorway serves content whose home peer is DOWN (failover through
the pool / peer cache); cross-doorway content resolution scenarios
(`features/federation/cross-doorway-content.feature`) — breadth legs @requires:shem,
single-doorway failover legs run on household-nodes now.

## Scenario slate (story-first — write/extend BEFORE implementing)

`genesis/a2o/features/federation/`: peer-advertisement, shard-tracking,
cross-doorway-content, doorway-pool-degrade already exist (several @wip/@requires:shem —
reuse their vocabulary). New: peer-recovery (B), peer-loss-failover (C),
storage-aggregates-truthful (D). Tag hardware-divergent legs with per-scenario
`@requires:<cap>`; everything provable on household-nodes stays untagged so the suite
runs NOW.

## Constraints & traps (hard-won; violating these wasted prior sessions)

- Museum record first when CI reads weird: NOT_BUILT/superseded ≠ regression; host-green
  ≠ CI-green; `#[ignore]` is a CI no-op.
- genesis/Jenkinsfile CPS: pipeline block is at 63.8KB of a 65KB hard cap — NO new
  inline heredocs; bash → `genesis/scripts/ci/*.sh`; the
  `.claude/hooks/jenkinsfile-method-size.py` hook enforces per-def 8KB too.
- Blob presence checks: GET, never HEAD. Propagation proof: build-unique probe blob +
  filesystem-count delta (streamed ≠ persisted).
- elohim-storage builds keep ambient RUSTFLAGS (getrandom custom); CARGO_TARGET_DIR =
  pool slot (fall back /tmp on fingerprint ENOENT); plain `cargo test` (no nextest in
  container).
- Never kubectl from the dev session (CI pods may, via ee-jenkins pod-ops only); repo
  manifests are the cleanup surface; netpol file is operator-applied.
- Commit-only; integrator pushes. Shared worktree — selective staging.
- Seeder peer-id contract: bindings ('desktop' = live pod) and commitments share the
  resolver; drift empties cluster_view/reciprocity_view joins. The household-formation
  ceremony path (`buildCeremonyCustodyInput`) still writes Stage-1 fake ids — same root,
  fix it in A.

## Done (stability-gated, not single-green)

- Three consecutive genesis builds: propagation passes with `kicksFiredTotal` rising on
  the receiver (sweep, not just heal), delivery events ≥1, projection sync clean,
  resilience signals asserting non-zero posture.
- Peer-recovery and peer-loss-failover scenarios green on household-nodes.
- Aggregates (free storage, stewarded commitments, households reciprocating) non-zero
  and matching seeded reality.
- Every new entity passed p2p-design-gate; every workstream has its scenario in the
  same commit; concerns opened for anything deferred (federation breadth @requires:shem).
