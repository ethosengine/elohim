---
title: "Resiliency-card + P2P-sync + Operational-Weave sprint — a sequenced execution plan composing the durability/weave/facings arc"
id: resiliency-card-p2p-weave-sprint-plan
status: Draft
class: protocol-canonical
domain: D5
topic: [sprint, sequencing, p2p-sync, sharding, blob-encryption, compute-contracts, resilience-card, operational-weave, epr-heads, tiered-weave, card-lighting]
refines:
  - genesis/docs/superpowers/plans/2026-06-10-epr-durability-replication-arc-plan.md
  - genesis/docs/superpowers/plans/2026-06-19-resilience-card-lighting-plan.md
  - genesis/docs/superpowers/plans/2026-06-20-operational-weave-lens-plan.md
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-06-14-recursive-architecture-design.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md
cites:
  - weave-epic-arc-design | the tiered-weave index this sprint integrates incrementally as bonus; its #4 (private-replica encryption) is Wave 5 | sha256:69966fdcc15dd7ba | path: genesis/docs/superpowers/specs/2026-06-20-weave-epic-arc-design.md
  - lens-complete-epr-resolution-four-leg-coupling-design | the EPR-head resolution law Wave 3 finishes (Slices 2/3 — relationships closure + value leg) | sha256:79f821217c1c8e11 | path: genesis/docs/superpowers/specs/2026-06-07-lens-complete-epr-resolution-four-leg-coupling-design.md
  - rea-economic-facing-lens-design | the compute-contracts facing Wave 4 lands (Mishpat→REA bridge + compute-fulfilled event) | sha256:b83ead21be13bbaa | path: genesis/docs/superpowers/specs/2026-06-19-rea-economic-facing-lens-design.md
  - operational-weave-facing-lens-design | the lens spec priority-#2 Wave 2 finishes (Slices 2-4 + WeaveView + GET /api/v1/weave) | sha256:fc432fea065dca00 | path: genesis/docs/superpowers/specs/2026-06-19-operational-weave-facing-lens-design.md
  - resilience-facings-select-fold-aggregate-design | the §11 select→fold→aggregate framework the card folds descend from; Wave 1.1 proof-gates it | sha256:8f2136ecd8678e6c | path: genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md
  - epr-slice1-lens-complete-resolver-plan | the EPR-head plan Wave 3 drains (the epr-composite renderer keystone, Task 2) | sha256:3dd888dbe730d5b3 | path: genesis/docs/superpowers/plans/2026-06-08-epr-slice1-lens-complete-resolver-plan.md
  - p2p-dataplane-sync-engine-design-arc | the dead/lesson the binding-constraints honor — no 3rd sync dialect; sharding/sync already shipped | sha256:d509030b5f00acd0 | path: genesis/docs/content/elohim-protocol/history/2026-06-11-p2p-dataplane-sync-engine-design-arc.md
  - dht-is-a-notary-not-a-byte-store | the binding constraint: the who-has-what index is gossip+projection, never a DHT entry | sha256:a1d408ef2478b288 | path: genesis/docs/content/elohim-protocol/history/2026-06-01-dht-is-a-notary-not-a-byte-store.md
  - elohim/elohim-storage/src/services/shard_manifest_backfill.rs
  - elohim/elohim-storage/src/services/seed_shard_manifest.rs
  - elohim/elohim-storage/src/services/household_resilience.rs
  - elohim/elohim-storage/src/p2p/mod.rs
  - elohim/elohim-facings/src/folds/operational_weave.rs
# Mixed-env plan (CLAUDE.md scope convention): NO doc-level requires_env so each
# gap inherits household-nodes by default; only cross-doorway/multi-tenant breadth
# gaps carry an inline @requires:shem.
---

# Resiliency-card + P2P-sync + Operational-Weave sprint

> **This is an EXECUTION-SEQUENCING plan, not a new design.** Every wave below
> `refines:`/`cites:` an existing canonical plan or spec — the design lives there and is
> not re-authored here. This doc's whole value-add is the **dependency-ordered sequence**,
> the **honest land-now ↔ held split**, and three **primary-source verification corrections**
> that prevent chasing phantoms. Branch: `feat/frontend-eyes-sprint`; integration target `dev`.

## Verification corrections (carry these into any downstream work)

1. **The "namespace mismatch" is a misnamed column, not a real bug.** `shard_locations.peer_id`
   stores an `agent_cid` (`uhCAk…`), not a libp2p PeerId — verified `seed_shard_manifest.rs:55-58`,
   `peer_selection.rs:253-255`, and the storage CLAUDE.md identity table. Both sides of the
   resilience join are `agent_cid`. The card is dark because **`shard_locations` is empty** (its
   runtime writer `p2p/mod.rs:1538` never fires — no live `distribute_shards`), NOT because of a
   join-namespace bug. **Keystone = populate the table, not reconcile namespaces.** The usual live
   cause of an empty join is a NULL `humans.agent_pub_key` — fixed by *populating it*, never a resolver.
2. **`CoverageRollup` is built AND consumed** (shefa: `graph_views/shefa/{coverage,distribution,resilience_snapshot}.rs`),
   not "built-but-unconsumed". The weave-epic #1 work is to wire its `descend()` into the *operational-weave*
   hand-written aggregates — a compose, not a build.
3. **The card-lit "honesty boundary" splits the lead slice in two** (the decisive correction):
   - `shard_manifest_backfill.rs` records a manifest **only for blobs the pod genuinely holds**
     (`blob_store.get` → skip-and-warn if absent). It honestly flips `distributionState → measured`,
     but `shard_locations` is written **only by the real `distribute_shards`** — so backfill alone does
     **not** light `stewardingCollectives`.
   - `seed_shard_manifest.rs` **can** light `stewardingCollectives` directly, but it is an explicit
     operator-gated **CLAIM**: off by default (`ALLOW_SEED_SHARD_MANIFEST=1`), rows marked `status="seeded"`
     ("operator asserted" vs. runtime "push-ack observed"). Honest-by-audit-trail, **acceptance/demo only —
     never the production-lit path.**
   The card is a **declared trust surface** (`distributionState: 'measured' | 'unmeasured'`, "never a fake
   at-risk verdict"). Honest production `stewardingCollectives > 0` therefore requires **real
   `distribute_shards`**, which is household-testable on the M/J/J 3-node mesh in short runs but
   **leak-gated for sustained alpha** (`project_storage_metrics_surface_and_leak_verdict`).
4. **The household/dwelling/hub/collective ontology (atlas-grounding `wf9k8i1fl`, 2026-06-22) — and the
   `agent_pub_key` keystone.** `humans.household_id` is **NOT a declared FK** to `collectives.id` — it is a
   nullable TEXT column opportunistically LEFT-JOINed, household-*by convention* (migration DDL has no
   `FOREIGN KEY`; `collective_participations` *does* — the absence is deliberate). There are **two orthogonal
   axes**: a *people* axis (`household_id`, wired) and a *place/dwelling* axis (spec-only, deferred to v2 per
   `2026-06-04-qahal-epr-household-lattice-design`). A household **IS-A** collective (`governance_layer="family"`),
   not has-a; "hub" is the resilience join's *grouping key* (`HolderRow.hub_id`), not an entity supertype.
   **The card's real keystone is `humans.agent_pub_key` (0/36 seeded) + `shard_locations`**: the relation is an
   **INNER JOIN** `agent_pub_key = shard_locations.peer_id` (`household_resilience.rs:370-373`) that drops rows
   *before* `household_id` is ever grouped. So `household_id` backfill is **cosmetic** — see Wave 0.3 (dropped).
   The card hardcodes `kind:"household"` (`:278`) — lifting it to derive from `governance_layer` is captured as
   complementary work (`resilience-tier-content-declared-floor`).

## Binding constraints (the gotcha — violating these re-treads dead paths)

- **DHT is a notary, not a byte-store** (`dht-is-a-notary-not-a-byte-store`). The who-has-what index
  (`shard_locations`/manifests) is gossip + projection (Category C), **never a DHT entry**. The
  reconciliation-controller pattern (custody-sweep + heal-on-read) is LIVE — do not resurrect
  ContentLocation-on-DHT (dead comment survives at `sharding.rs:9-10`).
- **Do not fork a third sync dialect.** steward/node is position-based CRDT (`/elohim/doc-sync/1.0.0`);
  storage is heads-based (`/elohim/storage-sync/1.0.0` + iroh `/elohim/sync/2.0.0`); blob sharding is
  `/elohim/shard/1.0.0`. All shipped. Compose them; don't add a fourth.
- **Encryption-ordering correctness edge.** `content_reach:"commons"` is hardcoded at `p2p/mod.rs:1492`
  (safe today — only public content distributes). If the reach-derivation TODO there resolves **before**
  blob encryption is live, private content plaintext-leaks to every custodian. **Encryption MUST precede
  that TODO's resolution.**
- **p2p-design-gate before ANY new aggregate entity.** genesis/Jenkinsfile is at ~63.8KB of the 65KB CPS
  cap — no new inline heredocs (bash → `genesis/scripts/ci/*.sh`). Commit-only; integrator pushes.

## Status ledger (landed-vs-open, file:line verified 2026-06-21)

| Thread | Owning plan/spec | Landed | Open remainder |
|---|---|---|---|
| P2P spine / sharding | durability-arc-plan | ~70% built, ~0% live | `distribute_shards` (`p2p/mod.rs:1469`) inert in prod; `shard_locations` empty; no a2o exercises it |
| Blob sync (custody/heal/reconcile) | durability-arc A–E | ~80% mechanism | Cross-peer commitment convergence (matthew→jessica) unproven; no `peer-recovery.feature` |
| Blob encryption | weave-epic §#4 | ~5% (seal primitive only) | No `KeyEnvelope`, no manifest `encryption` field; X25519 reader-key substrate specced+**blocked** |
| Compute contracts | rea-economic-facing-lens + Mishpat DNA | ~60% | `delegates-compute` validator landed; no Mishpat→REA bridge, no `MishpatCommitmentView`/route, no `compute-fulfilled` event |
| Card-lighting | resilience-card-lighting-plan | Sprint 1 ✅ (`commitmentBackedCollectives=1` LIVE, `27746ce6e`) | `count=2` operator-blocked (adam cell-enable); `stewardingCollectives>0` deferred here; 22/36 humans NULL `householdId` |
| Operational-weave lens | operational-weave-lens-plan | folds ~70%, route 0% | `WeaveView` + `GET /api/v1/weave` absent; `tier/region_occupancy` folds + gauges unbuilt |
| EPR heads | epr-slice1-resolver-plan + lens-complete-epr-resolution | Slice 0 ✅ (`/epr/{cid}/raw`), Slice 1 Task 1 ✅ | `epr-composite` renderer (keystone) absent; `epr_head.rs:148 relationships: vec![]` |
| Tiered-weave | weave-epic-arc-design | #1 built+consumed, #2/#3 ready, #4 greenfield | wire CoverageRollup into operational-weave; #2 measured-fold + #3 compute-fulfilled unbuilt; #4 = encryption |

---

## SPRINT 1 — the two stated priorities, all land-now (Waves 0–2)

### Wave 0 — close the amber genesis stages + card reads its own pod (land-now, no new substrate)
**Goal (household-felt):** the genesis pipeline stops false-amber on data/test gaps, and every member's pod shows its own provide-commitment — no dark card from a missing identity row.

These are the original-ask quick wins (the unstable `elohim-genesis/dev #1182` stages) folded in so the literal first request stays delivered:

| Slice | Source | PR shape | Verify |
|---|---|---|---|
| 0.1 manifesto blob-ref | sprint findings (Cluster D); `genesis/data/lamad/content/manifesto.json` (verified: 0 blob fields) | add `blobHash`+`blobCid` (CID-first, `bafkrei…`) | a2o `epr-content-addressing` "Blob loads via CID" |
| 0.2 content-insert idempotency | sprint findings (Cluster E); `genesis/a2o/steps/resilience.steps.ts` | GET-then-POST guard | resilience `grandma-photos` 500 gone |
| 0.3 ~~`householdId` backfill~~ **DROPPED** (ontology `wf9k8i1fl`) | — | Do **NOT** backfill the 22 NULLs — cosmetic: the relation INNER-JOINs `agent_pub_key` (0/36) *before* grouping `household_id`, there's no honest household for the 22, affiliations belong in `collective_participations`, and the `kind:"household"` hardcode would make a non-household backfill lie. **Real prereq = seed `agent_pub_key` + `shard_locations` (Wave 1).** Leave the 22 NULL. | n/a (the `d3b` test protects NULL→0) |
| 0.4 household-formation settle-wait | sprint findings (Cluster B); `genesis/seeder/src/seed-household-formation.ts` | `waitForCollectiveProjected()` after affirm loop | qahal `collective_cid stamped` + `triad` pass |
| 0.5 pre-seed readiness gate | sprint findings (Cluster A); `genesis/Jenkinsfile` `seedProjectionsStage`/`seedStewardshipStage` | poll `/health` until `catching-up` clears before the seed stages | Seed REA 503-cascade gone |
| 0.6 delivery-verify honesty | sprint findings (Cluster F); `genesis/scripts/ci/substrate-verify.sh:404-435` | widen `after=` to pod-boot **+ gate on a Loki check first** to rule out a real emit regression | Verify Delivery Events honest |

**Testability:** all household-testable NOW. `commitmentBackedCollectives=2` (adam) stays **operator-gated** (cell-enable, conductor-leak domain). Browser DI cluster (30 occ `LearnerBackend` NullInjector) = **operator redeploy `[build:app]`** — source already correct; flagged, not in-scope here.

### Wave 1 — the dark card lights honestly: `stewardingCollectives > 0` (PRIORITY #1 keystone)
**Goal (household-felt):** the card stops reading zero stewards — a household can *see* its content is held across peers, and the surface stays honest (measured = truly distributed).

| Slice | Source | PR shape |
|---|---|---|
| 1.1 resilience-facings Slice-0 proof-gate | resilience-facings §9 | DB-free: hand-build holder rows → `stewarding_hubs()`/`regional_distribution()` folds → assert non-zero. No substrate. |
| 1.2 backfill → `distributionState: measured` (honest) | durability-arc; `shard_manifest_backfill.rs` | wire the boot backfill into the household seed path so content the pods *hold* records a manifest → card flips to `measured` (NOT a `stewardingCollectives` claim) |
| 1.3 live `distribute_shards` household e2e (honest steward count) | durability-arc A/B; `p2p/mod.rs:1469` | a2o: upload a blob on the M/J/J mesh → shards fan via libp2p → receivers' `shard_locations` populate from the **runtime** path → `stewardingCollectives > 0` is an *observation* |
| 1.4 (acceptance lever only) `seed_shard_manifest` | `seed_shard_manifest.rs` (`ALLOW_SEED_SHARD_MANIFEST=1`, `status="seeded"`) | use ONLY in acceptance/demo to exercise the fold; never the production-lit path |

**Verify:** `stewardingCollectives > 0` via 1.3 (observation) on M/J/J; new a2o `features/federation/peer-stewarding.feature`.
**Testability:** 1.1–1.2 household-now. **1.3 is household-testable on the M/J/J 3-node mesh in short runs** but **leak-gated for sustained alpha** — see Open Decision A.

### Wave 2 — Operational-Weave lens lights (PRIORITY #2) + tiered-weave #1 bonus
**Goal (household-felt):** operators see one cluster-scoped weave view — placement gaps, RS-coverage, capacity, tier/region occupancy — folded from real data.

| Slice | Source | PR shape |
|---|---|---|
| 2.1 Slice-1 proof-gate (`placement_gap_count` → gauge) | operational-weave-lens-plan Slice 1 | DB-free fold → `/metrics` gauge. Zero deps. |
| 2.2 `tier_occupancy` + `region_occupancy` folds | operational-weave-facing-lens Slices 3–4 | pure folds in `folds/operational_weave.rs` (combinators exist) |
| 2.3 `WeaveView` + schema + ts-rs codegen | same; `elohim-views/src/infrastructure.rs` + `sdk/schemas/v1/views/weave-view.schema.json` | new view; `cargo test export_bindings`; serialize after 2.2 (codegen races) |
| 2.4 `GET /api/v1/weave` + `is_service_path` guard + gauges | same Slice 4; `http.rs` | route arm + `is_service_path` (doorway-shadow trap) + routing unit test |
| 🎁 2.5 BONUS tiered-weave #1 — CoverageRollup descent | weave-epic §#1; `recursion.rs` | replace hand-written `aggregate(triptychs)` with `CoverageRollup::descend()` — zero new entry types |

**Verify:** `pnpm look` renders the weave view; `/metrics` exposes the gauges; route unit test proves no EPR-router shadow.
**Testability:** 2.1–2.3 + 2.5 household-now; 2.4 gauges use `observability` (AVAILABLE).

---

## FOLLOW-ON (Waves 3–5 — named, owned by existing plans; not Sprint 1)

- **Wave 3 — EPR-head drill-downs read honest.** `epr-slice1-resolver-plan` Task 2 (the `epr-composite`
  renderer keystone — prevents the post-302-demotion raw-JSON fallback regression), Task 3 (Open-in-pillar),
  Task 4 (a2o 302 inversion), + lens-complete Slice 2/3 (populate `epr_head.rs:148 relationships`, value-leg
  shefa enrich). All household-testable. This is where the card's EPR drill-downs become real destinations.
- **Wave 4 — compute contracts observed + tiered-weave #3.** `rea-economic-facing-lens`: `MishpatCommitmentView`
  + read route; Mishpat→REA bridge projection on `action="delegates-compute"` (no new DHT entry); 🎁 `compute-fulfilled`
  EconomicEvent `bounded_by` the commitment → `mutual_compute` reads observed, not intent. Household-now;
  cross-doorway mutual-compute `@requires:shem`.
- **Wave 5 — private-replica encryption (weave #4).** Slice-0 single-host round-trip proof
  (`services/private_replica.rs`: plaintext → DEK encrypt → RS-encode ciphertext → reconstruct → decrypt →
  assert `plaintext_cid`) is cheap, DB-free, land-now. **Live encryption HELD** behind (a) X25519 reader-key
  substrate (specced, *blocked*, open security item), (b) the conductor-leak fix, (c) the encryption-ordering
  edge above.

---

## Land-now vs Held

- **LAND-NOW (household-testable):** Wave 0 (all), Wave 1.1–1.2, Wave 2 (all), Wave 5.1 (encryption proof),
  Wave 3 (all), Wave 4.1–4.3.
- **HELD — operator-gated:** `commitmentBackedCollectives=2` (adam cell-enable); netpol apply + conductor
  seeding (never run from CI); browser DI redeploy `[build:app]`.
- **HELD — leak-gated:** Wave 1.3 *sustained* live `distribute_shards` on the alpha fleet.
- **HELD — shem/cluster-gated:** cross-doorway commitment convergence; federated EPR closure breadth;
  cross-doorway mutual-compute.
- **HELD — substrate-blocked:** live encryption (X25519 reader-key resolver not built/unsigned).

## The single highest-leverage first slice
**Wave 1.3 — live `distribute_shards` household e2e** is the honest path to priority #1 (the card lighting
`stewardingCollectives` as an *observation*, the only legitimate production-lit state). If the leak-gate on
sustained alpha runs blocks it, the **honest land-now fallback is Wave 1.2** (flip `distributionState → measured`
for content the pods hold) plus a short M/J/J live run for the steward count — with `seed_shard_manifest`
(Wave 1.4) reserved strictly for acceptance/demo. The seeded-claim lever must never present as a production
card-lit state. (Alternative lead if priority #2 first: **Wave 2.1**, zero-dependency, lower learner-visibility.)

## Decisions (resolved 2026-06-22 — operator-confirmed)

- **A (the crux) — RESOLVED: the honest-observation bar.** "Card lights end-to-end" means
  `stewardingCollectives > 0` from a **real `distribute_shards` push-ack** (Wave 1.3) — the only legitimate
  production-lit state for a declared trust surface. Household-testable on the M/J/J mesh in short runs now;
  **sustained alpha is leak-gated** (waits on the conductor-leak fix). Wave 1.4 (`seed_shard_manifest`,
  `status="seeded"`) is **acceptance/demo only — never the production-lit path.**
- **B — CONFIRMED default: EPR-heads are follow-on (Wave 3).** Sprint 1 satisfies both stated priorities;
  pull Wave 3.1 (`epr-composite` renderer) in only if live card drill-downs are wanted this cycle.
- **C — CONFIRMED default: encryption = Wave 5.1 proof only** this sprint; the X25519 reader-key resolver is a
  named follow-on workstream (it gates the `content_reach` TODO — the encryption-ordering edge).
- **D — CONFIRMED default: tiered-weave bonuses (2.5, 4.x) ride their waves inline** (each is compose,
  zero/one new entry types) — the stated "incremental tiered-weave integration" bonus.
