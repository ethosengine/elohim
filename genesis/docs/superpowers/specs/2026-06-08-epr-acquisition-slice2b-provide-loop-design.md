# EPR Acquisition Slice 2b — The User-Facing Provide Loop (provide-reconciler design)

_Author: Claude Opus · Date: 2026-06-08 · Status: design (brainstorm-approved shape, adversarially reviewed) · Branch context: composes on the Slice 2a REA compute-bounds rail landed on `dev` (`9e7ba313e`, CI-verified DNA-green via `elohim-holochain/dev #1314`)._

> Companion to `genesis/docs/superpowers/specs/2026-06-07-epr-acquisition-pull-queue-design.md` (the parent acquisition spec — §1.2, §6.1–§6.5, §13, §14) and `HANDOFF.md` (Slice 1 + 2a complete). This spec turns the handoff's "Slice 2b is the clean follow-on" into an implementation-ready design, hardened by an adversarial review against the real code.

---

## 1. Vision & context

The EPR acquisition ladder lets a learner pull content to their device (Slice 1: rungs 2–3, the async pull queue + `DevicePin`). Slice 2b closes the loop: a device that has pulled commons content becomes a **provider** of it — rung 4, "pin as peer." The act of providing is notarized, bounded, and revocable, so the network can witness and rely on it without a central host.

**Commons-only v1.** Pinning is `reach == commons` only. Gated-EPR pinning waits on the capability-by-hash adjudication (parent spec §1.4/§14) and stays quarantined here — enforced defense-in-depth (rejected at mint *and* at emit).

This is the first production instantiation of the **REA compute-commitment primitive** (gospel memory `project_rea_compute_commitment_primitive`) for content provision: `Mishpat::Commitment` with a `replicates-commons` action, bounded reciprocity with on-chain standing + revocation + audit trail.

## 2. What 2a proved, and what 2b composes on

The Slice 2a rail (handoff §"Slice 2a COMPLETE") is **locally green** and **DNA-compiled-green in CI**. 2b builds directly on:

- `economic_event_emit_service::emit<F: CommitmentFetcher, R: RateHistory>` — bounds-validated EconomicEvent emit (the 7-check gate runs before any conductor write).
- `mishpat_commitments` projection table + `ProjectionCommitmentFetcher` — **fail-closed on null `dht_anchor_hash`** (un-notarized rows never clear bounds).
- `mishpat_projection::parse_commitment_payload` — action-discriminated parser (`delegates-compute` / `replicates-dwelling` / `acknowledges-reach-change`).
- `rea_projection` graduation — extracts `bounded_by` from the event, calls `graduate_to_active` (proposed→active), **atomically in the same transaction as the event upsert** (no author-event→graduate latency window).

### 2.1 CI-coverage caveat (discovered during review — a spec input + a backlog item)

The 2a *behavioral* tests are **invisible to CI**. `cargo-build-storage` is a Docker image build (`cargo build --release`); **no CI stage runs `cargo test`/`nextest` on the `elohim-storage` workspace**, so `tests/mishpat_bounds_gate_chain.rs` and the storage unit tests are verified **locally only**. The Mishpat `post_commit` `CommitmentCommitted` signal has **no sweettest** (the sole Mishpat sweettest is `bootstrap_steward_is_configured`).

**Consequence for 2b:** behavioral proof must land partly as **sweettests** (which *do* run in CI, with `--run-ignored all`). See §10. A backlog item is filed: `ci-storage-workspace-tests-uncovered` (the storage lib/integration tests have no CI gate).

## 3. The two-commitment model (canonical — do not conflate)

One `ProvideAnnounce` event references **two orthogonal** commitments (parent spec §6.5):

| | `Mishpat::Commitment` (policy envelope) | `content_store::Commitment` / `EconomicEvent` (REA fact) |
|---|---|---|
| Role | compute-bounds substrate primitive | ValueFlows economic accounting |
| Referenced by | event `bounded_by` (metadata + checked by `bounds_validator`) | event `fulfills` (DHT link, accounting) |
| For a **pure provide** | **required** (the `replicates-commons` commitment) | **optional / empty** (no counterparty) |

The historical failure to design around (CoordinationEnvelope ghost): a `ProvideAnnounce` must `bounded_by` a **real notarized** commitment, never a projection-only ghost. The 2a fail-closed `dht_anchor_hash` guard enforces this.

## 4. P2P Design Gate result (clean pass)

| Entity | Class | Address | Source of truth | Entry type | HTTP |
|---|---|---|---|---|---|
| `replicates-commons` Commitment | **A** (notarized) | content-derived (`entry_hash`) | Holochain DHT (Mishpat) | **existing** Mishpat `Commitment` (action discriminates) | none |
| `revokes-commitment` Commitment | **A** (notarized) | content-derived (`entry_hash`) | Holochain DHT (Mishpat) | **existing** Mishpat `Commitment` (action discriminates) | none |
| `ProvideAnnounce` EconomicEvent | **A** (notarized) | content-derived (event `action_hash`) | Holochain DHT (content_store) | **existing** EconomicEvent (action discriminates) | none |
| `mishpat_commitments` / `economic_events` rows | **C** (operational projecting A) | mirrors source | DHT (read-cache) | n/a | internal |
| `EprPullStatusView` | **C** (operational) | n/a | local `AcquisitionState` (reconstructable) | n/a | `GET /api/v1/pins/{eprId}/pull` (own-node-only) |

**Zero new DHT entry types, zero new commitment/event tables, one new operational view.** 2b is action-discrimination + parser arms + a scorer arm + a reconciler + a thin view — composing on existing substrate.

## 5. Architecture — the provide-reconciler (Approach 1)

The provide loop is a **reconciliation controller** (gospel `project_principle_p1_reconciliation_controller`: DHT = manifest, the loop eagerly reconciles). It converges to one invariant each tick:

> For every caught-up `reach == commons` `DevicePin`, there exists an `active` `replicates-commons` commitment whose first `ProvideAnnounce` has fired; for every removed pin, its commitment is revoked.

A new `provide_reconcile` tick (sibling of the 60s acquisition reconcile / 5s dispatch) runs the per-gap state machine. **The synchronous byte-arrival hook (`p2p/mod.rs:~3798 mark_completed`) is unchanged** — it never blocks on Holochain; all authoring is the reconciler's async work.

### 5.1 State machine (per logical provide-gap, keyed by `(provider, head_ref)`)

```
NeedsCommitment ──author (call_create_commitment, conductor)──▶ Authoring
Authoring ──post_commit signal projects row w/ dht_anchor_hash──▶ Projected
Projected ──emit ProvideAnnounce (bounded_by the CID)──▶ Announcing
Announcing ──rea_projection graduate_to_active + author CommitmentByState link (atomic; §6.4)──▶ Active
[pin removed] + Active ──author revokes-commitment──▶ Revoked
```

Transitions are driven by **observed durable state** (the `mishpat_commitments` projection + the pin table), not only an in-memory guard. The in-memory `HashMap<(provider,head_ref), ProvideStage>` is a **within-tick anti-double-author latch**, not the source of truth — on restart it is empty and the machine re-derives stage from the projection (see §6.1 idempotency).

## 6. Design decisions resolved by the adversarial review

These corrections harden Approach 1; the reconciler shape is unchanged.

### 6.1 Commitment idempotency — *logical-key dedup + parameterized `signed_at`* (was a blocker)

`create_commitment` currently calls `sys_time()` internally, so `signed_at` rides in the entry and **every call yields a different `entry_hash`** — re-authoring mints a duplicate, not an upsert (projection dedups on `cid = entry_hash`). Two-part fix:

1. **Reconciler dedup (authoritative):** before authoring, query `mishpat_commitments` for an existing row with logical key `(provider, action='replicates-commons', recipient=head_ref)` in **any non-revoked state** (`proposed`/`active`/`authoring`). Author **only if absent**. This makes re-author impossible across restarts regardless of timestamp.
2. **Byte-identical retry (defense-in-depth):** `create_commitment` (and the new `call_create_commitment` wrapper) take `signed_at` as a **parameter**, supplied by the reconciler, so a within-window retry of the *same* logical commitment is byte-identical → same `entry_hash` → true upsert. (The Mishpat coordinator signature changes from `sys_time()`-internal to param; backfill `delegates-compute`/`replicates-dwelling` callers to pass `sys_time()` explicitly — no behavior change for them.)

### 6.2 Revocation mechanism — *`revokes-commitment` Mishpat action + `pin.commitment_cid`* (was a blocker)

`Commitment` is immutable; `set_revoked_at` exists in the projection but **nothing calls it from un-pin**, and there is **no pin→commitment link**. Mechanism:

- New `pin.commitment_cid` column on `acquisition_pins` (nullable; set when the reconciler graduates a pin's commitment to `active`).
- On un-pin (`handle_remove_pin` → `set_pin_status('removed')`), the reconciler's **revocation arm** detects `removed` pins with a non-revoked `commitment_cid` and authors a `revokes-commitment` Mishpat action (immutability-respecting supersede entry referencing the original CID + a `signed_at`).
- The `post_commit` signal projects `revokes-commitment` → `mishpat_projection` sets `revoked_at` on the original row. `bounds_validator` (already checks `revoked_at`) then refuses subsequent `ProvideAnnounce`s. **Prior accepted events stand** (no retroactive invalidation, parent §6.3); local bytes MAY be GC'd per device policy.

This is DHT-native (the revocation is notarized, not projection-only) and sets the column the bounds-gate reads.

### 6.3 Two-commitment orthogonality — *split the emit input* (was a major)

`economic_event_emit_service::build_event_input` sets **both** `fulfills=[commitment_cid]` and `bounded_by=commitment_cid` to the **same Mishpat CID** — wrong for a pure provide. Fix: `EmitEconomicEventInput` gains an optional `content_store_commitment_cid`; `build_event_input` sets `fulfills = content_store_commitment_cid.map(|c| vec![c]).unwrap_or_default()` and keeps `bounded_by` = the Mishpat `commitment_cid`. A pure provide passes `fulfills: []`, `bounded_by: <mishpat-cid>`. Module doc updated; a test proves the empty-`fulfills` pure-provide path validates and emits.

### 6.4 Graduation truth — *author the `CommitmentByState` link* (was a major; **descope fork**)

2a's `graduate_to_active` only flips a **SQL column** — no DHT `CommitmentByState` link is authored, so "link is truth, SQL is projection" (parent §6.5) is currently aspirational and the SQL column is the de-facto truth (loss-of-state-on-replay risk). **Design intent (comprehensive scope chosen by operator):** on graduation, `rea_projection` calls a new `call_create_commitment_state_link` wrapper → a Mishpat coordinator fn authoring an immutable `CommitmentByState` link (anchor = commitment CID; payload = new state + timestamp + the `ProvideAnnounce` event hash). The SQL `state` becomes a write-through cache rebuilt from the link on restart.

> **Operator descope option:** if 2b should stay tighter, this single item may be **deferred** — keep projection-only `state` for v1 with a documented replay-risk comment and a follow-on ticket. Everything else in 2b is independent of this choice. (Flagged for spec-review.)

### 6.5 Commons content-identity matching — *reconciler-context-fed* (was a blocker; simplified)

`AdvertisedBlob` carries `blob_cid`/`epr_kind_hint`, **not** `content_id`/`head_ref`; inventory `BlobHint` likewise. Rather than widen the gossip wire, **the commons match is fed by the acquisition reconcile loop's `head_ref` context** (the caller knows which EPR it is fetching for). `score_advertised_blob` gains an optional `content_id` context parameter: the acquisition path passes it (commons is acquisition-only), the passive replication path passes `None`. The commons arm matches `content_id == commitment.head_ref` (v1 direct; closure-membership traversal is the Slice-3 closure resolver). No `BlobHint` wire change in v1.

### 6.6 Medium-tier enqueue — *make it an ordered tier* (was a blocker)

`FetchPriority::Medium` is `#[allow(dead_code)]` and the enqueue gate is exact-match `!= High { continue }`, so Medium would never enqueue even once produced. Fix: derive `PartialOrd, Ord` on `FetchPriority` (order `High > Medium > Skip`); change the gate at `score_and_enqueue_snapshot` to `if score == FetchPriority::Skip { continue }` (enqueue High **and** Medium). `score_advertised_blob` returns `Medium` on a commons match.

### 6.7 Per-EPR view — *group by `head_ref`* (was a major)

`AcquisitionState::per_pin()` keys by DB `pin_id`, and shared content is counted independently per pin (two pins on one EPR double-count). `EprPullStatusView` groups by `head_ref` (the EPR identity), rolling up shared content once. For v1 (item pins, `head_ref` IS id) this is usually one pin per EPR, but grouping is the correct contract and forward-compatible with Slice-3 multi-pin closures.

## 7. Layer-by-layer design

### 7.1 DNA (Mishpat zome — sweettest-only; `just pack`; DNA-hash changes)

- **Schema** `elohim/sdk/schemas/v1/commitments/replicates-commons.schema.json` — `oneOf` discriminated on `variant`:
  - `content`: `{ action:'replicates-commons', variant:'content', head_ref, closure_rule?, reach:'commons', bounds:{ rate_per_minute, reach_ceiling:'commons' } }` — **no donut**.
  - `capacity`: `{ action:'replicates-commons', variant:'capacity', commons_bytes>0, bounds, ratio_attestation:{ commons_pct, dwelling_pct, collective_pct, free_pct (sum=100), effective_ratio_cid } }` — carries the donut.
- **Schema** `elohim/sdk/schemas/v1/commitments/revokes-commitment.schema.json` — `{ action:'revokes-commitment', target_cid, reason?, signed_at }`.
- **Coordinator** `mishpat/zomes/mishpat/src/commitments.rs` — add `replicates-commons` and `revokes-commitment` arms to `validate_commitment_payload` (the dispatcher currently handles only 3 actions and rejects others). `validate_replicates_commons` mirrors `validate_replicates_dwelling`: variant-dispatch, `reach=='commons'` enforced, capacity variant runs sum-to-100 + `effective_ratio_cid`. `create_commitment` signature gains a `signed_at` param (§6.1).
- **Integrity** `mishpat_integrity/src/lib.rs` — defense-in-depth substring arms (action match, `variant` present, `reach` non-empty). No `serde_json::Value` across the WASM boundary (keep `payload_json: String`; deserialize *after* the boundary — add the documenting comment the review flagged).
- **Typed payload** `elohim-views/src/replicates_commons.rs` — `ReplicatesCommonsPayload` variant-tagged enum (ts-rs exported).
- **Immutability sweettest** — new `tests/sweettest/.../mishpat_commitment_immutability.rs`: create a `Commitment`, attempt update, assert rejection (closes the untested-immutability gap; also gives the `post_commit` path its first real sweettest).

### 7.2 Storage — author / project / reconcile

- **Conductor wrappers** (new) `call_create_commitment(hc, {action, payload_json, signed_at})` and `get_commitment(hc, cid)` — the Mishpat policy-envelope creation path that **does not exist yet** (only `call_create_rea_economic_event` for content_store events exists from 2a). `ConductorCommitmentFetcher` is still a `ConductorUnreachable` stub — T1 wires it (§11).
- **Projection parsers** `mishpat_projection::parse_replicates_commons` (variant-aware) + `parse_revokes_commitment` (sets `revoked_at` on the target row). Fail-closed on missing fields (warn+skip, dwelling precedent).
- **Validator** `replicates_commons_validator.rs` — three-stage (schema · donut[capacity only] · bounds-delegate); content-scoped skips donut.
- **The reconciler** `services/provide_reconcile.rs` — the §5.1 state machine; logical-key dedup (§6.1); revocation arm (§6.2); idempotent; bounded-backoff retry on conductor-unreachable. Wired into the storage event loop alongside the acquisition reconcile.
- **Emit split** (§6.3) in `economic_event_emit_service`.
- **Graduation link** (§6.4, descope-fork) `call_create_commitment_state_link` + the `rea_projection` call site.
- **Migration** add `acquisition_pins.commitment_cid` (nullable). No new commitment/event tables. Confirm `mishpat_commitments` migration carries `-- Source of truth: DHT`.

### 7.3 Scorer arm

- `ActiveCommitment` gains `action: String` and `head_ref: Option<String>`.
- `active_commitments_for_provider` — new arm loading `action='replicates-commons'` from **`mishpat_commitments`** (note: dwelling reads `rea_commitments` with a `row.id` fallback when anchor is NULL — *intentional* for replication; the commons arm instead **requires `dht_anchor_hash NOT NULL`** because 2b is conductor-path). Parse the content variant's `head_ref`.
- `score_advertised_blob(advertised, commitments, content_id_ctx)` — branch on `action`: dwelling → recipient-hub → `High`; commons → `content_id_ctx == commitment.head_ref` → `Medium`.
- Medium-tier enqueue ordering (§6.6). Fix `test_util.rs` `action='provide'` shorthand → `action='replicates-commons'` with a content-scoped payload (closes the Epic-B test-only gap properly).

### 7.4 View + API (Operational / C)

- **View** `elohim/sdk/schemas/v1/views/epr-pull-status.schema.json` → Rust `EprPullStatusView` in `views.rs` (camelCase) → `schema_contract.rs` test → TS codegen (add to `INTERFACE_FILES`). Fields `{ eprId, total, fetched, pending, failed, caughtUp }`, **grouped by `head_ref`** (§6.7); `total`/`caughtUp` nullable (null ≠ complete).
- **Route** `GET /api/v1/pins/{eprId}/pull` — **own-node-only**, deliberately absent from doorway `build_manifest()`. Source-of-truth comment `local (operational)`.

### 7.5 Angular UI (rung 4)

- `EprLinkComponent.fullActionList` += `{ id:'pin-as-peer', label:'Pin as peer' }`, **gated visible** by `capability()==='peer'` (`connectionMode==='direct'`) AND commons-reach; hidden / "not available in browser" otherwise.
- `AcquisitionService.pinAsPeer(epr)` (peer-path POST) + `pullStatus$(epr)` — **polls** `GET …/pull` ~3–5s while subscribed, maps to per-EPR `PullStatusInfo`, null-guard → neutral placeholder (never false-complete). Polling (no new transport; matches the resilience lazy-fetch pattern).
- New stateless `PinProgressComponent` (mirrors `CommitmentBarComponent`): `@Input` signals `{ total, fetched, pending, failed, caughtUp }` → bar + label (`"5.2 GB of 10 GB"`) + pending/failed badges; `@Output` cancel/retry. The Angular host owns the subscription lifecycle; the element stays blank-slate.

## 8. End-to-end data flow

`pin (DevicePin)` → pull queue fetches → **byte-arrival `mark_completed`** (unchanged) → **`provide_reconcile` tick**: caught-up commons pin, no non-revoked commitment for `(provider,head_ref)` → `call_create_commitment` (conductor, param `signed_at`) → `post_commit` signal → `mishpat_projection` upserts row w/ `dht_anchor_hash` (`proposed`) → **next tick**: projected → `economic_event_emit_service::emit` ProvideAnnounce (`bounded_by` the CID, `fulfills: []`) → `bounds_validator` passes (notarized ✓ reach=commons ✓ in-bounds ✓) → conductor creates event → `rea_projection` extracts `bounded_by` → `graduate_to_active` **+ author `CommitmentByState` link** (atomic) → set `pin.commitment_cid` → **scorer** loads it → advertised blobs matching `head_ref` score `Medium` → node serves as peer. **UI:** `pin-as-peer` → `pullStatus$` polls → `PinProgressComponent` renders. **Un-pin** → reconciler authors `revokes-commitment` → projection sets `revoked_at` → subsequent announces refused (prior stand).

## 9. Error handling & edge cases

- **Projection latency** (author→project): reconciler re-checks next tick — never inline-await. Event→graduate is **atomic** (no window).
- **Conductor unreachable:** pin stays `NeedsCommitment`/`Authoring`, bounded-backoff retry; logical-key dedup prevents duplicate authors when it recovers.
- **`reach != commons`:** rejected at mint (coordinator) AND at emit (bounds_validator).
- **RS-band > 64 MB:** pin fails honest `transport-unavailable`, never reaches caught-up, never authors.
- **Restart mid-flight:** in-memory latch lost; state re-derived from `mishpat_commitments` projection; logical-key dedup ⇒ no duplicate mint.
- **Idempotency everywhere:** logical-key dedup + parameterized `signed_at` + guarded `graduate_to_active` (proposed→active only) + `revoked_at` monotonic.

## 10. Testing strategy (CI-aware — see §2.1)

- **DNA sweettests (CI-covered, the primary behavioral proof):** `replicates-commons` commit (both variants); `reach != commons` rejected; `revokes-commitment` sets revocation; **Commitment immutability** end-to-end; and a sweettest exercising `create_commitment → post_commit → CommitmentCommitted` (the signal currently has zero coverage).
- **Storage unit (local + a NEW CI stage — see backlog item):** `parse_replicates_commons`/`parse_revokes_commitment` (fail-closed); validator (donut on capacity, skipped on content); scorer arm (Medium, content-id match, Medium enqueued); reconciler state-machine transitions + logical-key dedup + restart re-derivation.
- **Composition e2e** (mirror `mishpat_bounds_gate_chain`): caught-up pin → reconciler authors → projects → announces → graduates (+link) → scorer `Medium`; + un-pin revokes-refuses; pure-provide `fulfills:[]` path.
- **View contract:** `EprPullStatusView` schema_contract test; per-EPR grouping (shared content counted once).
- **Angular:** `pullStatus$` poll + null-guard; `PinProgressComponent` render; `pin-as-peer` visibility gating.
- **a2o:** extend `genesis/a2o/features/delivery/acquisition-pins.feature` with a provide scenario (cross-node serve leg `@requires:household-nodes`; trust-weighted/WAN `@requires:alpha-cluster-6peer`, HELD).
- **CI gap (backlog):** recommend a `cargo nextest run -p elohim-storage` CI stage so the composition e2e + unit tests gate (today they're local-only).

## 11. Preconditions & sequencing

**Hard precondition:** 2a green (DNA `#1314` ✓; storage rail locally green; CI behavioral coverage is the §2.1 gap).

**T1 is BUILD-and-verify, not just a probe (the foundational gate — the conductor path does not exist yet):**
1. Implement `call_create_commitment` + `get_commitment` (Mishpat); wire `ConductorCommitmentFetcher` (replace the `ConductorUnreachable` stub).
2. Author a `replicates-commons` commitment through the conductor; assert the `post_commit` signal projects it into `mishpat_commitments` with **non-null `dht_anchor_hash`**.
3. Assert `bounds_validator` accepts the notarized commitment (all checks pass).

**Do not build `provide_reconcile` (T-state-machine) until T1 is green.** Everything in §7 sits on the proven round-trip.

## 12. Task decomposition preview (~14 TDD tasks — `writing-plans` will detail)

1. **T1** conductor round-trip: `call_create_commitment`+`get_commitment`+`ConductorCommitmentFetcher` wired; non-null-anchor projection proven (sweettest + storage e2e). **GATE.**
2. `replicates-commons` schema (union) + `revokes-commitment` schema; codegen.
3. Mishpat coordinator+integrity validators (`validate_replicates_commons`, `validate_revokes_commitment`); `create_commitment` `signed_at` param + caller backfill; DNA sweettests (incl. immutability + post_commit).
4. `ReplicatesCommonsPayload` typed view (ts-rs).
5. `parse_replicates_commons` + `parse_revokes_commitment` projection arms (fail-closed).
6. `replicates_commons_validator` (schema · donut[capacity] · bounds).
7. Emit-input split (`content_store_commitment_cid`; pure-provide `fulfills:[]`).
8. `acquisition_pins.commitment_cid` migration.
9. **`provide_reconcile`** state machine + logical-key dedup + restart re-derivation.
10. Revocation arm (un-pin → `revokes-commitment` → `revoked_at`).
11. Graduation `CommitmentByState` link (`call_create_commitment_state_link`) — *or descope-defer per §6.4*.
12. Scorer arm (`ActiveCommitment.head_ref`, commons branch, `FetchPriority: Ord`, Medium enqueue, `content_id` ctx; fix `test_util` shorthand).
13. `EprPullStatusView` + `GET /api/v1/pins/{eprId}/pull` (group-by-head_ref; schema contract; codegen).
14. Angular rung-4: `pin-as-peer` action + `pullStatus$` + `PinProgressComponent`; a2o provide scenario.

## 13. Out of scope / deferred

- **Closure resolver** (multi-item cluster pins; graph-walk membership) — Slice 3.
- **Capacity-pledge auto-caller** — the capacity variant is *built and validated* but has no automatic minting path in 2b (the reconciler only mints content-scoped). A future donut-contribution flow drives it.
- **Dwelling-tier escalation** (co-stewardship, household scope, two-party consent) — distinct action `replicates-dwelling`, designed in parent §1.3, implemented when the consent surface exists.
- **Capability-by-hash / gated pinning** — quarantined (parent §1.4/§14); commons-only v1.
- **RS-band striping** (>64 MB) — separate transport spec.

## 14. Open decisions for the operator (at spec-review)

1. **Graduation truth (§6.4):** author the `CommitmentByState` DHT link in 2b (comprehensive, substrate-correct, partly retrofits 2a) — **default** — or **defer** with documented projection-only state for v1? (The only item where descope is clean.)
2. **CI storage-test stage (§2.1/§10):** add `cargo nextest run -p elohim-storage` to CI as part of 2b, or file-and-defer the coverage gap? (Default: file the backlog item; add the stage if you want 2b's e2e CI-gated.)

## 15. References & spawned backlog

- Parent: `genesis/docs/superpowers/specs/2026-06-07-epr-acquisition-pull-queue-design.md` (§1.2, §6.1–§6.5, §13, §14).
- `HANDOFF.md` (Slice 1 + 2a complete).
- Substrate floor: `compute-commitment-substrate-floor-design`, `rea-compute-substrate-native-roadmap`.
- Gospel memory: `project_rea_compute_commitment_primitive`, `project_principle_p1_reconciliation_controller`, `project_inventory_exchange_not_byte_replication`.
- **Spawned backlog items:**
  - `ci-storage-workspace-tests-uncovered` — no CI stage runs `cargo test`/`nextest` on `elohim-storage`; `mishpat_bounds_gate_chain` + storage units are local-only.
  - History-record-worthy when 2b lands: "why both commitment writers exist" (note in `epr-routing-complementary-captures.md`, per handoff).
