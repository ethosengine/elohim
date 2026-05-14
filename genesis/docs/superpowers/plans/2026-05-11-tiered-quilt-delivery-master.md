# Tiered Quilt Stewardship — Delivery Master

> **For agentic workers:** This is a **portfolio master plan**, not a single executable task list.
> Each wave below is a delivery slot that will receive its own detailed implementation
> plan (`docs/superpowers/plans/2026-XX-XX-tiered-quilt-wave-N-<name>.md`) authored
> when that wave is dispatched. The master plan governs sequencing, dependency, and
> acceptance gates between waves.
>
> When you are dispatched to **author a wave's implementation plan**, use
> `superpowers:writing-plans` with this delivery master as your scope frame.
> When you are dispatched to **execute** a wave's plan, use
> `superpowers:subagent-driven-development` or `superpowers:executing-plans` against
> that wave's authored plan.

**Goal:** Deliver tiered quilt stewardship (drawn / stocked-warm / stocked / shelved
classes, REA event chain, breach + restitution + accounting attestations, dogfooded
on the live MinIO + sccache substrate) across eight sequenced waves with cross-cutting
parallel tracks.

**Architecture:** Peer-local `TierController` inside `elohim-storage` reads
DHT-notarized `Commitment` floors (Category A) + local heuristic signals,
executes physical moves through per-archetype storage drivers, emits operational
`EconomicEvent` rows for every transition (Category C libp2p), and aggregates
periodic `tier-accounting` Attestations (Category B2 on DHT). Composes on top
of self-healing dataplane Plans 1–5; absorbs Plans 2–5 as sub-systems.

**Tech Stack:**
- Rust 2021 (elohim-storage, DNA zomes via HDK 0.3+, new `quilt-s3-shim` crate)
- Holochain DNA: elohim (protocol core, `Commitment` + `Attestation`)
- Diesel + SQLite (storage projections)
- libp2p 0.54 (operational event gossip via existing `InventoryBroadcaster`)
- iroh-blobs 0.94 (warm-tier transport, already wired)
- TypeScript / Angular 19 (operator dashboard tile)
- Vitest + sweettest harness (testing)
- MinIO (live external-archive destination)

**Source-of-truth spec:** `genesis/docs/superpowers/specs/2026-05-11-tiered-quilt-stewardship-design.md`

---

## Master plan structure

This master contains:

- §1 Decisions-required gate — operator sign-offs needed BEFORE Wave 0 dispatch
- §2 Wave portfolio — eight wave scope cards (each is a future implementation-plan stub)
- §3 Parallel tracks — driver backlog, narrative authoring, doc/memory updates
- §4 Dependency graph — explicit pre-requisites
- §5 Acceptance gates — what each wave proves before unblocking the next
- §6 Risk register — known waves with elevated risk + mitigations
- §7 Memory + docs follow-on — stale anchors to correct, new anchors to create
- §8 P2P design gate reference — entity classifications already in spec §4

---

## §1 Decisions-required gate (operator sign-off before Wave 0 dispatch)

Six decisions from spec §9 must be locked before Wave 0 begins. **No wave is
dispatched while a decision remains open** — the implementation plan author
would otherwise guess parameters that the operator wanted to set.

| # | Decision | Spec ref | Default if not specified |
|---|---|---|---|
| 1 | Wave 0 scope: bundle dedupe + rename, or split 0a/0b? | §1 + §8 wave table | Bundle as single Wave 0 |
| 2 | Archetype default catalog (breach thresholds, scan cadences, driver lineup) | §6 breach-threshold table | Use spec proposed values |
| 3 | Cost-class weight table | §4 accounting + §6 hoarder mitigation | warm:1.0, stocked:0.4, shelved-local:0.1, shelved-external:0.05 |
| 4 | Trust integration depth — tunable or hard switch in Wave 5/6 | §6 capability attestations | Tunable, default OFF in Wave 5; ON in Wave 6 |
| 5 | MinIO bucket lifecycle policy for `sccache-elohim` after Wave 3 introduces shelve-traffic | §8 Wave 3 deliverable | Operator-owned manual TTL; spec recommends 90d retention |
| 6 | elohim DNA entry-type capacity audit | §9 item 6 | Confirm headroom before Wave 1 ships new `action` + `category` discriminators |

**Gate procedure:** Operator answers all six in a single annotation pass on
this delivery master (or via session). Defaults stand unless overridden. Once
locked, Wave 0 author writes its implementation plan against the locked values.

---

## §2 Wave portfolio

Each wave is a scope card. When the wave is dispatched, its full implementation
plan (with task-by-task TDD steps per writing-plans skill) is authored as a
separate file referenced below.

---

### Wave 0 — Substrate cleanup

**Sub-plan:** `2026-XX-XX-tiered-quilt-wave-0-substrate-cleanup.md` (not yet authored)

**Why first:** Tiered-quilt adds 10 new `elohim_event_type` values and 5 new
`Attestation` discriminators on top of preexisting drift. Land on a clean
substrate.

**Scope:**
1. **Dedupe `Attestation` entry type.** Remove `Attestation` from
   `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs:416`
   (the duplicate). Single source of truth becomes the elohim DNA copy at
   `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs:1052`.
2. **Migrate imagodei coordinator paths** that currently create Attestation
   via imagodei DNA to call the elohim DNA coordinator instead. Audit
   `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs:607` and any
   callers.
3. **Rename `lamad_event_type` → `elohim_event_type`** across:
   - JSON Schema: `elohim/sdk/schemas/v1/EconomicEvent.schema.json`
   - Rust storage: `elohim/elohim-storage/src/db/models.rs:332`+ (`EconomicEvent`,
     `NewEconomicEvent`)
   - Rust DNA: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs:1088`+
     (`EconomicEvent` struct field)
   - Generated TS: `elohim/sdk/storage-client-ts/src/generated/`
   - Angular consumers: search-grep for `lamadEventType` and `lamad_event_type`
     across `app/elohim-app/` and `app/elohim-library/`
   - Seed data: `genesis/seed-data/*` files referencing the field
   - Tests across all of the above
4. **Codegen rerun.** Run `pnpm run schema:codegen:ts` + `pnpm run lamad:codegen`
   to regenerate type artifacts from the (unchanged) authoritative protocol +
   lamad manifest sources of truth. No new source-of-truth introduced by this
   step — it only re-emits derived TypeScript from existing JSON definitions.
5. **Pre-push hook validates** field rename didn't leave dangling references
   (drift test).

**Acceptance:**
- `git grep -E "lamad_event_type|lamadEventType"` returns empty in source files
  (allowed in archived specs and CHANGELOG-style records only)
- Imagodei DNA `dna.yaml` no longer declares Attestation entry type
- All existing event values (`content-view`, `path-step-complete`, etc.)
  retain their string values; only the field name changes
- Full Bands 1+2 of the existing test suite pass (no regressions)
- Pre-push hook green

**Risk: HIGH.** Wide blast radius — touches DNA, storage, TS, Angular, seed
data. Coordinate dispatch with other in-flight branches. **Mitigation:**
single integrating PR rather than incremental merges. Acceptance gate is
"no field references remain by old name."

---

### Wave 1 — Commitment factory + tier-state projection

**Sub-plan:** `2026-XX-XX-tiered-quilt-wave-1-commitment-factory.md`

**Depends on:** Wave 0 complete and merged

**Scope:**
1. **Add 10 `elohim_event_type` values** to the lamad manifest's event-type
   catalog: `quilt-stocked`, `quilt-stocked-warm`, `quilt-shelved`,
   `quilt-promoted`, `quilt-demoted`, `quilt-evicted`, `quilt-drawn`,
   `quilt-restituted`, `quilt-floor-committed`, `quilt-floor-released`.
2. **Add 5 new optional columns** to `economic_events` (Diesel migration):
   `tier_from`, `tier_to`, `shelf_destination_uri`, `driver_class`, `cost_class`.
   - **Source of truth: SQLite extension to existing `economic_events`.** Rows are projected from elohim DNA `EconomicEvent` entries via the existing post-commit signal pipeline; these new columns are operational supplements (Category C) populated by `ReaEventEmitter` at write time. No DHT entry-type change.
3. **Create `quilt_tier_state` table** (Diesel migration):
   - **Source of truth: SQLite (Category C operational).** Rebuildable on boot from `BlobStore` + `BlobMetadata` via `reconstruct_tier_state()` (step 7 of this wave). No DHT entry type — no `dht_anchor_hash` column. Spec entity classification: spec §4 + delivery-master §8.
   ```sql
   CREATE TABLE quilt_tier_state (
     peer_id TEXT NOT NULL,
     content_cid TEXT NOT NULL,
     current_tier TEXT NOT NULL,
     observed_at TEXT NOT NULL,
     transition_seq INTEGER NOT NULL,
     shelf_destination_uri TEXT,
     PRIMARY KEY (peer_id, content_cid)
   );
   CREATE INDEX idx_quilt_tier_state_observed_at ON quilt_tier_state(observed_at);
   ```
4. **Create `CommitmentFactory`** at `elohim/elohim-storage/src/tier/commitment.rs`:
   - Public method: `accept_custody_quilt(cid, tier_floor, shelf_destination, duration) -> Result<CommitmentHandle>`
   - Calls elohim DNA coordinator zome to create `Commitment` with
     `action="custody-quilt"`
   - Resolves `resource_classified_as_json` from `QuiltCustodyClassification` struct
   - Verifies caller's earned capability attestations meet `tier_floor` requirement
     before submitting (the graph-pattern gate from spec §6)
5. **Add `QuiltCustodyClassification` to view-schemas**:
   - `elohim/sdk/schemas/v1/views/quilt_custody_classification.schema.json`
   - Codegen TS via `pnpm run schema:codegen:ts`
   - Schema contract test in `elohim/elohim-storage/tests/schema_contract.rs`
6. **HTTP route** `POST /api/v1/rea-commitments` for app-developer surface
   (admin-only initially; broader exposure in later waves).
   - **Entry type: elohim DNA `Commitment` (existing, Category A).** Route is a proxy that calls the elohim DNA coordinator zome `create_commitment`; the projection row is written via the existing post-commit signal handler. Route follows from DHT design (the `custody-quilt` action discriminator on Commitment), not the reverse.
7. **Tier-state reconstruction on boot**: function `reconstruct_tier_state()`
   that walks `BlobStore` + `BlobMetadata` and infers current_tier for each
   known CID (no commitment yet → tier=`drawn`).

**Acceptance:**
- Diesel migration applies forward and reverses cleanly
- Unit tests: `cargo test --lib --bins -p elohim-storage tier::commitment`
- Integration test: sweettest `tier_commitment_lifecycle.rs` (3 peers: 1 commiter, 1 steward, 1 observer) passes
- Schema contract test green
- App-manifest schema accepts `quilt_policy` block (parsing only — TierController consumes it in Wave 2)

**Risk: MEDIUM.** Touches both DNA and storage; first crossing for tiered work.

---

### Wave 2 — TierController + LocalDeviceDriver + heuristic classifier

**Sub-plan:** `2026-XX-XX-tiered-quilt-wave-2-controller-and-drivers.md`

**Depends on:** Wave 1 complete

**Scope:**
1. **`StorageBackend` trait** at `elohim/elohim-storage/src/tier/drivers/mod.rs`:
   ```rust
   pub trait StorageBackend: Send + Sync {
       fn execute_transition(&self, cid: &str, from: Tier, to: Tier) -> Result<TransitionOutcome>;
       fn driver_class(&self) -> &'static str;
       fn advertise_cost_class(&self, tier: Tier) -> CostClass;
       fn capability(&self, tier: Tier) -> Option<CapabilityWindow>;
   }
   ```
2. **`LocalDeviceDriver`** at `elohim/elohim-storage/src/tier/drivers/local_device.rs`:
   - One-drive archetype (laptop/mobile/browser/wearable)
   - Tier = retention priority, not bytes-move
   - LRU eviction within tier band; first-evicted bands at lower tier
3. **`HeuristicClassifier`** at `elohim/elohim-storage/src/tier/heuristic.rs`:
   - Pure function: `(floor, signals, manifest, archetype) -> Tier`
   - Returns `floor ≤ desired ≤ stocked-warm`
   - Pluggable per archetype
4. **`TierController`** at `elohim/elohim-storage/src/tier/controller.rs`:
   - Async loop with archetype-tunable cadence (default: 30s edge, 5s hub)
   - For each row in `quilt_tier_state`: read floor, compute desired, stagger,
     pre-transition reach probe, execute via `StorageBackend`, emit event,
     update row, gossip
   - Stagger: `blake3(cid || peer_id || epoch) % stagger_window`
5. **`ReaEventEmitter`** at `elohim/elohim-storage/src/tier/events.rs`:
   - Writes `EconomicEvent` rows with full `tier_from/tier_to/driver_class/cost_class` set
   - Routes to `InventoryBroadcaster.delta(...)` for libp2p gossip
6. **Wake hook**: `CommitmentFactory.accept_custody_quilt(...)` calls
   `TierController.wake_for_cid(cid)` so newly-committed quilts get an
   immediate transition rather than waiting for next cadence tick.

**Acceptance:**
- Unit tests: `cargo test --lib -p elohim-storage tier::heuristic` (100% on pure function)
- Unit tests: `cargo test --lib -p elohim-storage tier::stagger` (golden tests)
- Unit tests: `cargo test --lib -p elohim-storage tier::drivers::local_device`
- Integration: sweettest `tier_commitment_lifecycle.rs` extended to assert
  `quilt-stocked` event emission within cadence + 1s
- Manifest hint resolution: precedence (app → archetype default → policy.toml) tested

**Risk: MEDIUM.** First end-to-end loop; mostly contained to elohim-storage.

---

### Wave 3 — ShelfRouter + ExternalArchiveDriver + quilt-s3-shim + sccache repoint

**Sub-plan:** `2026-XX-XX-tiered-quilt-wave-3-shelfrouter-and-shim.md`

**Depends on:** Wave 2 complete

**Scope:**
1. **`ShelfRouter`** at `elohim/elohim-storage/src/tier/shelf/mod.rs`:
   - URI-scheme dispatch: `peer-cellar://`, `external-archive://minio/...`
   - Two-phase commit semantics (begin → copy → ack → retention → cleanup)
2. **`ExternalArchiveDriver`** (MinIO scheme) at
   `elohim/elohim-storage/src/tier/shelf/external_minio.rs`:
   - Uses `aws-sdk-s3` (S3 v4 compatible with MinIO)
   - Endpoint, bucket, credentials from env via existing `sccache-credentials`
     pattern
   - Retention window configurable per commitment (default 24h after destination ack)
3. **New crate** `crates/quilt-s3-shim/` (Rust, axum-based):
   - S3-compatible HTTP front-end (GET, HEAD, PUT, DELETE on `/{bucket}/{key}`)
   - Translates to `storage-client.stock(cid, classification_hint)` and
     `storage-client.draw(cid)`
   - Maintains a (sccache-key ↔ cid) mapping table
   - Manifest hint: `ephemeral-build-cache` (shelve_after=5m)
4. **Deploy quilt-s3-shim** to cluster:
   - Helm chart in `genesis/manifests/quilt-s3-shim/`
   - ClusterIP service in `ethosengine` namespace
   - Replicas: 2 (single bucket today, shim is stateless beyond the key↔cid table)
5. **Repoint sccache** in `devfile.yaml`:
   - `SCCACHE_ENDPOINT` env now points at `http://quilt-s3-shim.ethosengine.svc.cluster.local:9000`
   - `SCCACHE_BUCKET=sccache-elohim` (unchanged downstream)
   - Backward-compatible: a feature flag allows old direct-MinIO mode for rollback

**Acceptance:**
- Unit tests: shim S3-compat suite (PUT/GET/DELETE/HEAD)
- Unit tests: ShelfRouter two-phase commit (partition during phase 1 + retention window)
- Integration test: sweettest `tier_two_phase_commit_partition.rs` (2 peers + test MinIO)
- E2E live test: dogfood — run `cargo build` against repointed shim, observe
  cache populates in `sccache-elohim` bucket via shim, observe corresponding
  `economic_events` rows
- Rollback verified: feature flag flip restores direct-MinIO mode

**Risk: HIGH.** Live substrate change. Existing builds depend on sccache working.
Wave dispatches require a maintenance window or feature-flag staging.
**Mitigation:** feature flag + staged rollout to one Che workspace first;
acceptance includes successful build for at least three independent developers
before flip becomes default.

---

### Wave 4 — BreachScanner (absorbs Plan 2)

**Sub-plan:** `2026-XX-XX-tiered-quilt-wave-4-breach-scanner.md`

**Depends on:** Wave 2 complete (does NOT depend on Wave 3 — can run in parallel with shim work)

**Scope:**
1. **`BreachScanner`** at `elohim/elohim-storage/src/tier/scanner.rs`:
   - Pass 1 (observation gap, cheap): walks active commitments + recent holdings;
     issues `tier-breach` when below floor for > threshold
   - Pass 2 (direct draw probe, sampled): probes a subset; issues `tier-breach`
     with `probe_method="direct-draw-failure"` when latency exceeds SLA
2. **`breach_attestations` projection table** (Diesel migration):
   - **Source of truth: elohim DNA `Attestation` entry (Category B2).** Discriminator: `category="storage-stewardship"`, `attestation_type="tier-breach"`. This SQLite table is a projection of DHT-notarized entries; `dht_anchor_hash NOT NULL`. Rows populated via post-commit signal handler. No new DHT entry type — reuses existing `Attestation`.
   ```sql
   CREATE TABLE breach_attestations (
     dht_anchor_hash TEXT PRIMARY KEY,
     witness_agent TEXT NOT NULL,
     subject_agent TEXT NOT NULL,
     subject_cid TEXT NOT NULL,
     commitment_hash TEXT NOT NULL,
     observed_tier TEXT NOT NULL,
     tier_floor TEXT NOT NULL,
     probe_method TEXT NOT NULL,
     breach_window_start TEXT NOT NULL,
     breach_window_end TEXT NOT NULL,
     earned_via_json TEXT NOT NULL,
     created_at TEXT NOT NULL
   );
   ```
3. **HTTP route** `GET /api/v1/quilts/{cid}/breaches`.
   - **Entry type: elohim DNA `Attestation` (existing, Category B2).** Read-only projection query over `breach_attestations`; returns attestations where `subject_cid = {cid}` and `attestation_type="tier-breach"`. Route follows from the DHT entry-type design (the `tier-breach` discriminator), not the reverse.
4. **Archetype-tunable thresholds** loaded from `policy.toml`:
   ```toml
   [tier_breach_thresholds.edge]
   stocked_warm = "2m"
   stocked = "5m"
   shelved = "4h"
   ```
   Defaults from spec §6 table.
5. **Extend `placement_gaps`** with new `gap_kind` values:
   `tier-below-floor`, `tier-breach-unresolved`, `tier-below-floor-prevented`.

**Acceptance:**
- Unit tests: threshold math (with frozen clock)
- Unit tests: probe-method dispatch
- Integration: sweettest `tier_breach_detection.rs` (4 peers: 1 failing steward,
  1 scanner, 2 witnesses) — breach attestation visible in DHT, projected to
  `breach_attestations`, and propagated to `placement_gaps`
- Trust adjustments NOT yet wired (Wave 5/6) — assert no trust changes occur

**Risk: LOW.** Self-contained inside elohim-storage; no live substrate impact.

---

### Wave 5 — Restitutor (absorbs Plan 3)

**Sub-plan:** `2026-XX-XX-tiered-quilt-wave-5-restitution.md`

**Depends on:** Wave 4 complete (needs breach attestations to react to)

**Scope:**
1. **`Restitutor`** at `elohim/elohim-storage/src/tier/restitution.rs`:
   - Subscribes to BreachWitnessed signal (post_commit of `tier-breach`
     attestation → projection → gossip → subscriber)
   - Deterministic arbitration: `argmin(blake3(breach_hash || peer_id || epoch))`
   - Primary path: discover K-of-N survivors, `rs_decode`, verify BLAKE3,
     stock at floor, issue `tier-restitution` attestation
   - Witness path: wait stagger window; if primary doesn't complete, promote
     self to primary
2. **Cascading-breach escalation**:
   - When `len(sources) < K`: emit `placement-gap` with
     `gap_kind="tier-breach-unrecoverable"`, `severity="critical"`; issue
     `Attestation` `attestation_type="tier-restitution-failed"`
3. **`restitution_attestations` projection table**:
   - **Source of truth: elohim DNA `Attestation` entry (Category B2).** Discriminator: `category="storage-stewardship"`, `attestation_type="tier-restitution"`. Projection of DHT-notarized entries; `dht_anchor_hash NOT NULL`. No new DHT entry type.
   ```sql
   CREATE TABLE restitution_attestations (
     dht_anchor_hash TEXT PRIMARY KEY,
     restitutor_agent TEXT NOT NULL,
     subject_cid TEXT NOT NULL,
     breach_attestation_hash TEXT NOT NULL,
     restituted_at TEXT NOT NULL,
     reach_post_recovery INTEGER NOT NULL,
     reconstruction_source_peers_json TEXT NOT NULL,
     bytes_reconstructed INTEGER NOT NULL,
     created_at TEXT NOT NULL
   );
   ```
4. **`recognition-transfer` event emission** on successful restitution
   (existing event type from prior shefa work; this wave adds tier-driven
   trigger).
5. **Trust integration wiring point** (toggleable via decision-#4): when ON,
   capability attestations are debited for the breached steward and credited
   for the restitutor; routing priorities update.

**Acceptance:**
- Integration: sweettest `tier_restitution_arbitration.rs` (5 peers: 1 breach,
  4 candidates) — exactly one restitution attestation issued; the other 3
  candidates emit no duplicate work
- Integration: sweettest `tier_cascading_breach.rs` (6 peers, only K-1 = 3
  shard-survivors) — `tier-restitution-failed` issued; critical
  placement-gap emitted; no infinite retry
- Recognition transfer event emitted in correct direction

**Risk: MEDIUM.** Coordination with Plan 3's existing arbitration code;
extension rather than replacement.

---

### Wave 6 — HoldingsAttester + AccountingAggregator (absorbs Plan 4)

**Sub-plan:** `2026-XX-XX-tiered-quilt-wave-6-holdings-and-accounting.md`

**Depends on:** Wave 5 complete (full restitution chain needed before accounting reflects it)

**Scope:**
1. **`HoldingsAttester`** at `elohim/elohim-storage/src/tier/holdings.rs`:
   - Periodic cadence (archetype-tuned, default: edge=15m, hub=5m)
   - Reads `quilt_tier_state` + `economic_events` for window
   - Constructs `tier-holdings` Attestation with per-CID payload
   - Submits via elohim DNA coordinator → DHT
2. **`AccountingAggregator`** at `elohim/elohim-storage/src/tier/accounting.rs`:
   - Daily cadence (archetype-tunable)
   - Rolls up `economic_events` for prior period into per-agent summary
   - Cost-class weighted; decision-#3 weights table
   - Issues `tier-accounting` Attestation
3. **Two projection tables**:
   - **Source of truth for both: elohim DNA `Attestation` entry (Category B2).** Discriminators: `attestation_type="tier-holdings"` and `attestation_type="tier-accounting"` respectively, both under `category="storage-stewardship"`. Projections of DHT-notarized entries; both with `dht_anchor_hash NOT NULL`. No new DHT entry types.
   ```sql
   CREATE TABLE holdings_attestations (
     dht_anchor_hash TEXT PRIMARY KEY,
     holder_agent TEXT NOT NULL,
     period_start TEXT NOT NULL,
     period_end TEXT NOT NULL,
     earned_via_json TEXT NOT NULL,
     created_at TEXT NOT NULL
   );
   CREATE TABLE accounting_attestations (
     dht_anchor_hash TEXT PRIMARY KEY,
     agent TEXT NOT NULL,
     period_start TEXT NOT NULL,
     period_end TEXT NOT NULL,
     weighted_total REAL NOT NULL,
     net_position REAL NOT NULL,
     earned_via_json TEXT NOT NULL,
     created_at TEXT NOT NULL
   );
   ```
4. **Shefa signal mappings**:
   - Wire `tier-accounting` attestations into existing `compute-contribution`
     signal stream (extends, doesn't replace)
   - New signal: `storage-contribution`
   - New signal: `unresponsive-steward` (derived from commitments-vs-served-draws
     over rolling window)
   - New signal: `commitment-expiring` (derived from active commitments where
     `has_end - now < threshold`)
5. **Trust adjustment ON** by default in this wave (decision-#4): capability
   attestations decay on repeated breach; commitment acceptance gates harden.
6. **Operator dashboard tile** (Angular):
   - `app/elohim-app/src/app/shefa/components/tier-state/tier-state.component.ts`
   - Reads `GET /api/v1/quilts/tier-state-summary` (per-CID observed tier vs
     commitment floor, breach state, restitution history).
   - **Entry types: elohim DNA `Commitment` + `Attestation` (existing, Categories A + B2); plus operational `quilt_tier_state` (Category C).** Read-only query joins the three projections to render the operator dashboard. Route follows from the DHT design (commitment floor + breach/restitution/holdings attestations) — operator surface is a projection over notarized state, not a new authority.
7. **Ambient grandma tile**:
   - `app/elohim-app/src/app/shefa/components/compute-contribution-tile/`
     reads latest `tier-accounting` attestation for the current human and
     surfaces "compute contribution: +X GB-hours, served Y draws" — read-only,
     no controls

**Acceptance:**
- Integration: sweettest `tier_holdings_attestation_cycle.rs` — three peers
  hold different tier mixes; holdings attestations match observed state
- Integration: sweettest `tier_accounting_period_close.rs` — frozen clock;
  raw events roll up into expected `tier-accounting` payload with correct
  weighted_total math
- Operator dashboard: Cypress E2E hits `/api/v1/quilts/tier-state-summary`
  and renders without console errors
- Ambient grandma tile: renders zero state correctly when no attestations
  exist yet; populates within one accounting period after first events fire

**Risk: MEDIUM.** Trust-integration toggle flip is the highest-impact change
in this wave. Stage with feature flag to allow ON/OFF per environment until
soak-tested.

---

### Wave 7 — Chaos demo + dogfood a2o (absorbs Plan 5)

**Sub-plan:** `2026-XX-XX-tiered-quilt-wave-7-chaos-and-dogfood.md`

**Depends on:** Wave 6 complete

**Scope:**
1. **Live shem deployment** for Bands-3 chaos demo (per
   `project_shem_is_p2p_live_canvas`): household cluster for Matthew/Jessica/Terrance
   + shem peers for others.
2. **Three extended Plan 5 scenarios** with tier assertions:
   - Steward offline → reach breached → restitution → reach restored;
     ASSERT `tier-breach` + `tier-restitution` chain visible
   - Partition splits household; ASSERT no spurious cross-partition restitution
   - Wearable churns; ASSERT no `tier-breach` from wearable lifecycle
3. **Four new tier-specific a2o scenarios** authored as Gherkin in
   `genesis/a2o/features/storage/tiered-quilt/`:
   - `new-developer-first-cargo.feature` (the dogfood)
   - `family-cluster-resilience.feature` (grandma's photos across cities)
   - `hoarder-vs-steward-accounting.feature` (REA-math sanity)
   - `self-degraded-driver-report.feature` (transparent failure)
4. **a2o step definitions** in
   `app/elohim-app/cypress/support/step_definitions/tiered-quilt/`
   (Cypress + Cucumber, per existing patterns)
5. **Story-harvest** per
   `feedback_a2o_narrative_is_opus_work` — Opus-authored narrative tying
   each scenario to the manifesto. NO Haiku for this work.
6. **Memory anchor: sprint-result entry**:
   - Capture non-obvious discoveries (Garage→MinIO substrate correction,
     duplicate Attestation drift discovered + resolved, lamad-vs-elohim-DNA
     naming clarification, the "tier as priority on a laptop vs tier as
     hardware on a steward" Bittorrent analogy as a design pillar)
   - Anchor: `project_tiered_quilt_stewardship_landed_2026_XX_XX.md`

**Acceptance:**
- All four new a2o scenarios pass on shem
- All three extended Plan 5 scenarios still pass with tier assertions added
- Live dogfood: at least one new developer onboarded against the repointed
  quilt-s3-shim from Wave 3 — their first cargo produces the expected
  attestation chain and ambient tile within one accounting period
- Sprint-result memory entry committed
- Cross-references back to spec are accurate

**Risk: LOW.** Chaos and narrative work; mostly orchestration.

---

## §3 Parallel tracks (not waves — can run alongside)

### Track A — Driver backlog

Once Wave 2 lands, the additional drivers can be authored in parallel:

| Driver | Sub-plan | Use case | Earliest start |
|---|---|---|---|
| `LocalHardwareTieredDriver` | `2026-XX-XX-tiered-quilt-driver-local-hardware-tiered.md` | Steward node SSD+HDD | After Wave 2 |
| `PeerCoordinatedDriver` | `2026-XX-XX-tiered-quilt-driver-peer-coordinated.md` | Within-mesh peer routing via elohim-operator | After Wave 4 (needs breach detection for failover) |
| `FederatedDwellingDriver` | `2026-XX-XX-tiered-quilt-driver-federated-dwelling.md` | Cross-WAN family-cluster (grandma's photos) | After Wave 6 (full accounting + capability attestations) |
| `ExternalArchiveDriver` IPFS scheme | `2026-XX-XX-tiered-quilt-driver-ipfs.md` | Public archival option | After Wave 3 |
| `ExternalArchiveDriver` Arweave scheme | `2026-XX-XX-tiered-quilt-driver-arweave.md` | Permanent archival option | After Wave 3 |

### Track B — App-manifest evolution

The `quilt_policy` block (declared in `elohim/sdk/schemas/v1/app-manifest.schema.json` — existing source-of-truth file) gains new optional fields across waves. No new schema files introduced by this track; only field additions to the existing app-manifest schema, each governed by the wave that consumes the field:

| Wave | Adds |
|---|---|
| 1 | `default_tier_floor`, `shelve_after`, `hold_warm_min` |
| 3 | `prefer_destinations` array (URI list) |
| 6 | `cost_class_hint` (informs accounting weight selection) |

### Track C — Doc + memory updates

| When | Action |
|---|---|
| Pre-Wave 0 | Update `MEMORY.md`: correct `project_garage_sccache_substrate_2026_05_09` → `project_minio_sccache_substrate_2026_05_09`; trim MEMORY.md (currently over budget per system warning) |
| Per wave | Story-harvest at wave close (per `feedback_a2o_narrative_is_opus_work`) |
| Wave 7 close | Sprint-result memory entry (anchored under "tiered-quilt-stewardship landed YYYY-MM-DD") |
| Wave 7 close | Cross-reference back-links: spec ↔ this delivery master ↔ self-healing dataplane spec |

### Track D — CI integration

| Wave | CI change |
|---|---|
| 0 | Existing test suite must remain green through rename |
| 1+ | New sweettest scenarios added to `elohim/holochain/dna/Jenkinsfile` per-commit stage |
| 7 | Nightly chaos demo on shem added to Jenkins schedule |

---

## §4 Dependency graph

```
                  ┌─── Track A (drivers backlog, parallel from Wave 2)
                  │
Decisions ──▶ Wave 0 ──▶ Wave 1 ──▶ Wave 2 ──┬──▶ Wave 3 (shim) ─────────────────┐
   gate                                      │                                    │
                                             └──▶ Wave 4 ──▶ Wave 5 ──▶ Wave 6 ──▶ Wave 7
                                                                                  │
                                Track B (manifest), C (doc/memory), D (CI) ─────►┘
```

Wave 3 (shim) and Wave 4 (breach scanner) can run in parallel after Wave 2.

---

## §5 Acceptance gates between waves

Each gate is a binary "may we dispatch the next wave?" decision the operator
runs. Defaults are spec-derived; operator can override.

| Gate | Required signal | Default action on fail |
|---|---|---|
| 0 → 1 | Zero `lamad_event_type` references in source; all Bands-1+2 tests pass | Hold Wave 1; debug rename |
| 1 → 2 | `tier_commitment_lifecycle` sweettest green; schema contracts pass | Hold; fix commitment factory |
| 2 → {3,4} | `quilt-stocked` event emitted within cadence + 1s in test; controller cadence does not regress existing tests | Hold; debug controller |
| 3 → (Wave 6 dogfood readiness) | Live dogfood: at least 3 independent developer cargo builds completed against shim with corresponding events recorded | Continue Wave 4/5/6 in parallel; defer shim dogfood until rollout matures |
| 4 → 5 | `tier_breach_detection` sweettest green; breach attestations flow to projection | Hold Wave 5; debug scanner |
| 5 → 6 | Both restitution scenarios green; no infinite retry on cascading breach | Hold; debug arbitration |
| 6 → 7 | Operator dashboard renders; ambient grandma tile populates within one accounting period | Hold Wave 7; debug shefa wiring |
| 7 → close | All four new a2o scenarios pass on shem; three extended Plan 5 scenarios pass; sprint-result memory entry committed | Address scenario failures; re-run |

---

## §6 Risk register

| Risk | Where | Likelihood | Impact | Mitigation |
|---|---|---|---|---|
| Wave 0 rename leaves dangling references | Wave 0 | Medium | Compile failures in dev branch | Single integrating PR + drift test in pre-push hook |
| sccache live repoint breaks builds | Wave 3 | High (if no feature flag) | All developer builds blocked | Feature flag + staged rollout to one workspace first |
| Trust adjustment over-aggressively decays capability | Wave 5/6 | Medium | Stewards lose tier access; commitment factory rejects too many | Tunable knob (decision-#4); start OFF in Wave 5, ON in Wave 6 with soak monitoring |
| Cascading restitution loop | Wave 5 | Low | Network saturation | Deterministic arbitration + circuit breaker; `tier-restitution-failed` attestation halts further retry |
| MinIO bucket fills | Wave 3 onward | Medium (over time) | Cluster disk pressure | Decision-#5 (lifecycle policy); 90d retention recommended |
| elohim DNA entry-type capacity exceeded | Wave 1 | Low | Cannot add `action=custody-quilt` discriminator | Audit pre-Wave 1 (decision-#6); discriminator reuses existing entry type so risk is more about coordinator zome size than entry slots |
| Coordinated dispatch with other in-flight branches | Wave 0 | Medium | Merge conflicts with EPR Phase 2B, M5 sprint, etc. | Operator schedules Wave 0 dispatch window; communicates to other active branches |

---

## §7 Memory + docs follow-on

### Memory anchors to correct or create

| Action | Anchor | Trigger |
|---|---|---|
| Correct (stale) | `project_garage_sccache_substrate_2026_05_09` → rename to `project_minio_sccache_substrate_2026_05_09`; body says MinIO, single replica, openebs-jiva PVC, bucket sccache-elohim | Wave 0 start (pre-flight discovered drift) |
| Create | `project_tiered_quilt_stewardship_design_2026_05_11.md` | Spec land (now) |
| Create | `project_attestation_dedupe_elohim_dna_canonical.md` | Wave 0 close — captures the single-source-of-truth decision for future reference |
| Create | `project_elohim_event_type_field_rename.md` | Wave 0 close — captures the rename rationale |
| Create | `project_tiered_quilt_stewardship_landed_YYYY_MM_DD.md` | Wave 7 close — sprint-result entry |

### MEMORY.md hygiene

System warning: `MEMORY.md is 34.8KB (limit: 24.4KB)`. Track C work includes a
trim pass — keep one-line index entries, push detail into topic files. Don't
defer past Wave 0.

### Vocabulary cross-link

Add to `genesis/graphos/vocabulary.md`:

- Cross-reference to this spec under the **`quilt`** + **`stock`** + **`shelve`** terms — note that the tier classes (`drawn / stocked-warm / stocked / shelved`) name REA resource states that the verbs produce.

---

## §8 P2P design gate reference

Entity classifications are authoritative in the **spec** at
`2026-05-11-tiered-quilt-stewardship-design.md` §4. Summary for sub-plan authors:

| Entity | Class | DNA | Reuses |
|---|---|---|---|
| `ReaCommitment` (custody-quilt) | A | elohim | existing `Commitment` entry type |
| `BreachAttestation` | B2 | elohim | existing `Attestation` entry type |
| `RestitutionAttestation` | B2 | elohim | existing `Attestation` |
| `HoldingsAttestation` | B2 | elohim | existing `Attestation` |
| `AccountingAttestation` | B2 | elohim | existing `Attestation` |
| `SelfDegradedAttestation` | B2 | elohim | existing `Attestation` |
| `QuiltTierState` | C | — | NEW SQLite table, operational |
| `TierTransitionEvent` (raw) | C | — | extends `economic_events` |
| `PlacementGap` (extended) | C | — | existing table |

**Zero new DHT entry types.** All notarized state composes on existing
`Commitment` and `Attestation` types via `action` / `category` /
`attestation_type` discriminators.

---

## §9 Spec-coverage self-review

This master plan covers each spec section:

| Spec § | Master plan coverage |
|---|---|
| 1 Why this spec exists | Master goal + capability bar restated at top |
| 2 Architecture | Wave 2 (TierController + drivers), Wave 6 (Angular dashboard for operator surface) |
| 3 Components | Each component table row maps to a wave in §2 |
| 4 REA event catalog | Wave 0 (rename field), Wave 1 (10 event types + 5 new columns), Waves 4–6 (5 attestation discriminators) |
| 5 Data flow (4 loops) | Loop 1 = Wave 1; Loop 2 = Wave 2; Loop 3 = Waves 4+5; Loop 4 = Wave 6 |
| 6 Error handling | Wave 4 (breach), Wave 5 (restitution + recognition), Wave 6 (trust integration toggle), Wave 7 (self-degraded a2o) |
| 7 Testing | Band 1 unit per wave; Band 2 sweettest acceptance per wave; Band 3 chaos in Wave 7 |
| 8 Delivery waves | §2 Wave portfolio (this section) |
| 9 Decisions still required | §1 gate (six decisions) |
| 10 Cross-references | §7 memory + doc updates |
| 11 Open questions deferred | Track A backlog drivers + spec §11 remain deferred |

No coverage gaps. Plan internally consistent.

---

## §10 Plan author note

This delivery master is a **portfolio** — it sequences and gates the eight
waves but does not pre-author each wave's task-by-task TDD plan. That is
intentional: each wave's plan must be authored when the wave is dispatched,
using the locked decision-gate values + current substrate state at that
moment.

When operator signs off on §1 decisions, dispatch a **plan-authoring
session** for Wave 0 first: that session invokes `superpowers:writing-plans`
with this master + the spec as input, and produces
`2026-XX-XX-tiered-quilt-wave-0-substrate-cleanup.md`. That wave plan is
then executed via `superpowers:subagent-driven-development` or
`superpowers:executing-plans`.

Subsequent waves follow the same pattern, with acceptance gate from previous
wave required before the next plan-authoring session starts.

---

## §11 Cross-references

- Source-of-truth spec:
  `genesis/docs/superpowers/specs/2026-05-11-tiered-quilt-stewardship-design.md`
- Self-healing dataplane (absorbed Plans 2–5):
  `genesis/docs/superpowers/specs/2026-04-19-self-healing-p2p-dataplane-design.md`
- Sweettest harness:
  `genesis/docs/superpowers/specs/2026-04-22-sweettest-integration-layer-design.md`
- MinIO substrate runbook:
  `genesis/manifests/RUNBOOK-minio-sccache-2026-05-09.md`
- Vocabulary lock: `genesis/graphos/vocabulary.md`
- EPR substrate framing:
  `genesis/docs/superpowers/specs/2026-04-21-elohim-core-graph-substrate-design.md`
