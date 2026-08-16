---
id: minutes-quiesce-fixture-trust-swarm-plan
title: Minutes-Scale Quiesce — per-peer fixture trust, shard swarm, and the local-first measure
status: Draft
class: protocol-canonical
topic: [dataplane, quiesce, fixture-trust, simulacra, shard-swarm, blob-durability, hc-mesh, ci-measure]
domain: D5
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/trust-as-efficiency-signal.md
refines:
  - genesis/docs/superpowers/plans/2026-08-08-head-plane-trust-gradient-program-plan.md
cites:
  - "head-plane-trust-gradient-program-plan | The refines-target: this sprint lands its open tail (T5/T8/T9/T10) — Simulacra activation IS the declared-trust-bootstrap | sha256:aee96a34080d4efa | path: genesis/docs/superpowers/plans/2026-08-08-head-plane-trust-gradient-program-plan.md"
  - "trust-as-efficiency-signal | The canonical principle being implemented: trusted content must measurably cost less to propagate | sha256:40b8e3d166c935a7 | path: genesis/docs/content/elohim-protocol/architecture/trust-as-efficiency-signal.md"
  - genesis/docs/superpowers/specs/2026-05-02-blob-custody-reconciliation-design.md
  - "epr-acquisition-pull-queue-design | Owns the striping/acquisition seam the shard-swarm spread (W1.3) extends | sha256:24aad9240361c0a4 | path: genesis/docs/superpowers/specs/2026-06-07-epr-acquisition-pull-queue-design.md"
  - genesis/data/timeline/backlog/chunked-blob-over-16mb-not-durable-mesh-repro.md
  - "substrate-trust-contract-runbook | The invariants (heal fills-never-moves, canonical channels) the ceremony compression must not violate; probes the measure reads | sha256:e47d962ca7259c79 | path: genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md"
---

# Minutes-Scale Quiesce — per-peer fixture trust, shard swarm, and the local-first measure

**Habit served:** `notary-authority` (top red). Its banking run requires a measured
Dataplane Validation window; every edge build since #1349 has exited
`FLEET-CHURNING — DID NOT MEASURE` because fleet quiesce takes hours. This sprint's
deliverable is quiesce **in minutes, definitively, measurable on a Jenkins stage** —
which is what un-wedges the banking run and the whole saga register.

## 1. Why (two measurements + the operator's reframe)

- **Alpha, 2026-08-16 morning:** edge #1359 held the fleet-quiesce gate 45 min and did
  not measure. Live probes at 13:05Z: matthew `projectionReconcile pending 2449→2531
  GROWING`, shem pods stuck at 387–716 `divergent_actionable` for 8h flat, adam
  admission-gated (`{"status":"catching-up"}`). The gate is floor-tuned (~6–7 min of
  own overhead); the fleet is the bottleneck.
- **Local mesh, 2026-08-16 overnight:** 3,439 rows × 3 peers converged fully in
  **~90 min** on `hc-mesh.sh` — the baseline this sprint must beat by ~10×.
- **The operator's reframe (binding):** the reach ceremony is architecturally real —
  CI speed must come from **compressing** it, never skipping it. Two moves:
  1. **Per-peer, not per-EPR.** "Hey peer, what all do you have?" — one trust act
     covers a peer's whole advertised fixture corpus. Ceremony cost goes O(EPRs) →
     O(peers).
  2. **Fixtures declare what they are.** Genesis test-fixtures are
     staging/dev/preproduction artifacts; frontmatter declares the trust needed to
     bootstrap them; deployment mints that grant once per deploy/environment.
- **The swarm curve:** sharded blobs replicating across n peers should compound
  torrent-style — every peer holding a shard is a source, so aggregate byte
  propagation accelerates as replicas spread. Today the curve is broken structurally
  (see §2 blob defect).

## 2. Grounded state (verified 2026-08-16, 28/29 claims confirmed on disk)

**Already landed from the trust-gradient program plan** (refines-target; task IDs
theirs): T1/T3 batch externs + `HeadBatchResolver` + `AdaptiveBatchBudget` (AIMD 8–128,
fanout 2), T4 `head_corpus_digest` (behind default-off `ELOHIM_HEAD_CORPUS_DIGEST`),
T6 inert trust seam (`src/trust/`: `NetworkStage` Simulacra<Bootstrap<Coordinated<
Enforced, `VerificationPricer`, floor property test).

**Open tail this sprint picks up:** T5 (digest requester flip), T8/T9 (signed
`HeadSetSnapshot` + `VerificationMemo` wiring), **T10 (`ManifestStakesResolver` +
`ELOHIM_NETWORK_STAKES` + Simulacra activation on genesis fixtures + seeder per-fixture
reach field)** — T10 *is* the declared-trust-bootstrap, already designed, unlanded
(`trust/stage.rs` marks it out-of-scope-there).

**Blob defect (structural, RCA `chunked-blob-over-16mb-not-durable-mesh-repro.md`):**
`>16MB` blobs shard at 1MB (`sharding.rs` banding none/chunked/rs-4-7); the composite
hash is never stored; the `ShardManifest` lives in an in-process map (`http.rs:218`),
DB-mirrored write-only, never read on GET, never reloaded at boot; `get_blob_or_heal`
(`http.rs:2830-2849`) never consults manifests → `/apps` 404 by construction for any
`encoding != "none"` blob; a replicated shard-set is unservable everywhere except the
minting process. This gates saga ch03/04/05/09/10 and is the live suspect for the
alpha landing-blobHash divergence (matthew `93ec…` vs adam `9263…` observed today).

**Chatty paths (round-trip census):** one `kademlia.put_record` per content id on the
15s drain ticker; one `ContentHeadRecord` RPC per divergent id; replication/acquisition
= two sequential RPCs per item (GetContent → blob pull) at 50/25 inflight on 5s ticks;
discovery inventory is already batched (1 request/peer/table, ≤2000 entries).

**Backoff ladders (prod-tuned, dominate local tail):** contest 3600s, heal-missing
600s, evidence-absent 86400s, MissLedger 12-sweep dormancy, sweep cadence
`PROJECTION_RECONCILE_SECS` default 300s (env-tunable).

**Wired trust levers a scheduler can consume today:** `classify_pre_authorization` /
`node_has_embodied_responsibility` (peer-granular, topic param present-but-unused),
`order_eligible_by_trust_gradient` + `eligible_trust_score` (proven reorder slot),
`Reach::openness()` (8-level ordinal, unused as weight). Standing is `Unknown` until
T19 (owned elsewhere) — no live-gradient claims before it.

## 3. Design — four workstreams

### W1 — Shard-manifest durability + swarm propagation (blob plane)

Composes from `blob-custody-reconciliation-design` (canonical) + the acquisition
striping seam. The manifest is **Ephemeral (C)**: deterministically reconstructable
from the composite bytes + encoding params; its durable projection is the existing
`shard_manifests` DB table (read-back is the fix, not a new plane).

1. **Durability:** GET path falls back to `db::shard_manifests::get_manifest` on
   in-memory miss; boot reloads the map; `get_blob_or_heal` becomes manifest-aware.
   Kills the restart-fatal + cross-process-unservable class.
2. **Propagation:** a fetched blob's manifest travels with the first successful shard
   response (additive optional field on the blob fetch response; `skip_serializing_if`
   compat discipline per the T4 wire rule) — a receiving peer can serve the composite
   as soon as its shards land.
3. **Swarm spread:** shard fetches for one composite spread across the known holder
   set (holder-rotation per shard, not one race per composite); on each completed
   shard, the existing event-driven `BlobInventoryDelta` advertises immediately so the
   holder set — and aggregate bandwidth — compounds. Measure the curve (W4).

### W2 — Per-peer fixture trust (ceremony compression; T5+T10+T8/T9 composed)

- **Fixture declaration (genesis side):** fixture corpora carry frontmatter declaring
  `environment: preproduction` tier + the trust grant needed to bootstrap (grantor
  scope, corpus id) — extending the existing realism-ladder declaration convention
  (qahal-epr-household-lattice: every seed module declares its rung and why). The
  declaration is doc-plane; the seeder consumes it.
- **Deploy-time mint (runtime side):** the seeder/deploy leg writes the stakes field
  on the **standing-policy manifest** (T10's designed shape — no new entry type, no
  new registry) and the environment sets `ELOHIM_NETWORK_STAKES`. Provenance travels
  (`StakesProvenance::Manifest{cid} | OperatorConfig`); absent ⇒ `Bootstrap`
  fail-closed. **Simulacra is never a default and never derived from any DEV_MODE.**
- **Per-peer adoption:** at Simulacra, for the declared fixture corpus, a peer's
  signed `HeadSetSnapshot` (T8: signed transient, NO DHT entry type, carries
  `corpus_digest` + `signer_agent_cid` + `trust_epoch`) prices as
  `AcceptWithProvenance` — the receiver batch-adopts the advertised declared heads
  without per-id `ContentHeadRecord` RPCs or elections. One inventory exchange per
  peer per table IS the ceremony. Floor invariant untouched: Constitutional /
  LocalRelationship / CounterEvidence **never cheapen at any stage** (property-test
  pinned).
- **Local anchor honesty:** this also retires the local-stack DHT-anchor gap
  (bulk seed → provenance-gate 404s → `p2p_published_at` backfill hack): the import
  pipeline gains a legitimate anchor+declare step under the declared grant.

### W3 — Dev-tier pacing profile (declared, not hacked)

A named preproduction pacing profile — env-only, set by `hc-mesh.sh` and the CI mesh
leg, never defaulted: `PROJECTION_RECONCILE_SECS≈30`, `CONTEST_BACKOFF_SECONDS` /
`HEAL_MISSING_BACKOFF_SECONDS` / `ELOHIM_EVIDENCE_ABSENT_BACKOFF_SECS` shortened
proportionally, `ELOHIM_HEAD_CORPUS_DIGEST=1` (T5 flip, local first),
`QUIESCE_SUSTAIN_SECS` scaled to cadence+10% (sustain must exceed one sweep by
construction). The profile is one documented block in `hc-mesh.sh` + the CI stage —
same machinery, declared stakes, no parallel dev path.

**Conductor-side leg (upstream-blessed, from the 2026-08-16 upstream survey):** the
0.6 `ConductorConfig.network.advanced` JSON passes straight through to kitsune2 —
`k2Gossip.{initiateIntervalMs,minInitiateIntervalMs}: 1000` is what sweettest itself
sets for tests (vs multi-second prod defaults; `sweet_conductor_config.rs:82-84` in
the fork), plus `initial_initiate_interval_ms` for the first round (kitsune2 #220).
The dev-tier profile writes this into the hc-mesh sandbox conductor config so local
DHT gossip converges at test cadence, not prod cadence.

### W4 — The definitive measure (local first, then Jenkins)

1. **Local harness:** `time`-wrapped `fleet-quiesce-gate.sh localhost:8888
   localhost:8889 <content-id> localhost:8090 localhost:8091` (all five args already
   printed by `hc-mesh.sh status`), run at baseline and after each workstream lands.
   Record wall-clock-to-PASS in the shift journal.
2. **Jenkins stage (new, definitive):** an hc-mesh-in-CI leg — stand up the 3-peer
   mesh in the build container, seed the fixture corpus under the declared grant, run
   the same gate with the dev-tier profile, **hard verdict in ≤15 min**
   (PASS / FAIL — no DID-NOT-MEASURE class, because the mesh is owned by the stage).
   This is the "definitively in minutes" stage: it measures the protocol's convergence
   machinery per-commit without touching the alpha fleet.
3. **Swarm curve measure:** time-to-N-replicas for an 18MB fixture blob at N=1,2,3
   peers locally; assert aggregate propagation accelerates with holder count
   (regression-pinned as an a2o scenario via story-harvest).
4. **Honesty ledger (binding, from the program plan §6):** local/CI numbers
   characterize reconcile/ceremony/backoff mechanics — they do NOT transfer WAN
   write-guard or arc-fraction effects (local mesh runs full-arc, no cgroup ceiling).
   Alpha numbers are measured only on the alpha gate; the alpha Simulacra activation
   leg stays operator-gated (T10's `@requires:alpha-cluster-6peer` leg).

## 4. Tasks

| ID | Task | Tier | Depends |
|----|------|------|---------|
| Q1 | Local quiesce harness + baseline record (W4.1; no code) | Sonnet | — |
| Q2 | Shard-manifest durability: DB read-back on GET, boot reload, manifest-aware `get_blob_or_heal`, regression tests incl. restart | Opus | — |
| Q3 | Manifest propagation on shard fetch (additive wire field, 3 compat tests) | Sonnet | Q2 |
| Q4 | Shard swarm spread + immediate delta advertise + curve probe | Sonnet | Q2 |
| Q5 | Fixture frontmatter convention + seeder reads it → standing-policy manifest stakes field + anchor/declare import step (T10 local half) | Opus | — |
| Q6 | `ManifestStakesResolver` + `ELOHIM_NETWORK_STAKES` reader (T10 runtime half; fail-closed tests) | Sonnet | — |
| Q7 | Simulacra `AcceptWithProvenance` inventory adoption path (T8 snapshot + pricer join; floor property test extended) | Opus | Q5, Q6 |
| Q8 | Dev-tier pacing profile block in `hc-mesh.sh` + T5 digest flip locally | Sonnet | Q1 |
| Q9 | Jenkins hc-mesh quiesce stage (build-manifest + Jenkinsfile leg; bash in `scripts/ci/`, heredoc-free) | Opus | Q1–Q8 green locally |
| Q10 | Saga re-run local (target: ch03/04/05/09/10 unpinned by Q2; register evidence) + story-harvest the constraints | Sonnet | Q2–Q8 |

Per-task DoD: touched tree's gate clauses green from clean state (fmt, clippy -D
warnings, `cargo test` with echoed `EXIT=$?`); commit-only, integrator pushes; all work
in /projects/elohim.

## 5. Falsifiable targets

- Local: 3.4k-row corpus × 3 peers quiesces (gate PASS, sustained) in **≤10 min**
  (baseline ~90 min).
- Jenkins: the new stage returns a hard verdict in **≤15 min** every run.
- Swarm: 18MB blob time-to-3rd-replica < time-to-1st-replica × 3 (super-linear
  aggregate).
- Alpha (deferred, operator-gated): predicted gate-quiesce falls with `PTxnGuard` rate
  FLAT (round-trips cut, pressure not raised).

## 6. Guards

- Simulacra by explicit declaration only; grant is environment-scoped, declared on the
  artifact, verified at deploy — never inferred from repo origin; preproduction trust
  can never leak into Enforced reach.
- Floor classes never cheapen (property test is the keystone; Opus-reviewed).
- No gradient-behavior claims before T19 standing lands (owned by the SDK-promise
  program).
- Measurement honesty per W4.4; measuring-by-deploy stays forbidden on alpha.

## 7. Captured, not absorbed

- Story/graph-connectivity blended adopt ordering (`order_eligible_by_trust_gradient`
  slot + `Reach::openness()` weight) — consonant stretch; needs no new seam; do after
  Q7 if the sprint has room, else backlog.
- Stale `doorway.elohim.host` ingress answering 503 (nginx, no backend) — manifest
  cleanup for the operator's reconcile surface.
- Shem-side stuck `divergent_actionable` (susan/gertrude/eve 387–716 flat) — likely
  same root family as the blob divergence; re-probe after Q2 reaches alpha.
- Upstream survey (2026-08-16, complete) — findings binding on Q5's shape:
  - The "seed data" pattern = **Init Properties** (Holochain 0.6.2, Dev Pulse 155;
    already in our 0.6.3 fork): an opaque per-role blob delivered ONCE to the zome's
    `init` hook via `get_init_properties`, cleared after init. Migration-grade
    injection, not a fixture framework — complements elohim-import (a "genesis corpus
    baked in at install" option), does not replace the repeated dev-loop re-seed path.
  - `AdminRequest::GraftRecords` (bulk pre-signed records straight onto a source
    chain, `validate: false`) — the only true skip-the-zome bulk path; sharp edges
    (irreversible chain fork risk) make it a snapshot-restore tool, not fixture
    authoring.
  - `call_zome_with_options` per-call timeout (0.6.1) — sanctions the coarse-grained
    "one big zome call commits the batch" seeding shape Q5's anchor/declare step
    should use (batch inside the zome fn; no batch wire API exists).
  - Honest absences verified: no `hc seed` DSL, no batch zome-call API, no sandbox
    fast-boot/snapshot-restore in 0.6.x.

## P2P Design Gate (run 2026-08-16)

- **Entity: fixture trust grant** — NOT a new entity: one stakes field on the existing
  standing-policy manifest (T10's designed shape) + `ELOHIM_NETWORK_STAKES` operator
  config. Manifest is Constitutional-floor (never cheapens; FullChain-verified at
  every stage — one object per deploy, so the cost is O(1)). Provenance enum already
  exists (`StakesProvenance`). No DNA-hash move; no new zome surface.
- **Entity: `HeadSetSnapshot`** — already gated in the program plan: signed transient,
  Path C, NO DHT entry type; CID via `epr_codec` dag-cbor; evidence-not-authority
  (receiver re-derives); C2 trust_epoch regression-refused.
- **Entity: `ShardManifest`** — Ephemeral (C): reconstructable from composite bytes +
  deterministic 1MB banding; durable projection = existing `shard_manifests` table
  (read-back fix); travels as transport metadata on the fetch wire, never a DHT entry;
  addressed by the composite blob hash (legacy `sha256-` form on the existing wire —
  CID-wrapping is the named downstream migration, not this sprint).
- **Entity: fixture frontmatter declaration** — doc-plane (genesis repo), consumed by
  the seeder; runtime artifact is the manifest field above. No runtime entity.
- **Head-plane cost:** zero new heads. The sprint REDUCES per-item head-plane work
  (per-peer adoption replaces per-id RPCs); corpus stays ~3.5k A-class heads
  (Row 16 A→A2 migration remains out of scope, cheaper to defer per the program plan).
- **Anti-pattern check:** no per-host authored trust (grant is deploy-minted through
  the declared manifest, DHT-witnessed); no DEV_MODE-derived stage; no stored score
  (memo carries no numeric field); amber-window honesty preserved (anchor step makes
  fixtures green legitimately, not stamped locally).
