---
title: Slice 2a — REA economic-event emit + commitment graduation rails (foundational) — Implementation Plan
id: epr-acquisition-slice2a-rea-rails-plan
status: Draft
class: protocol-canonical
domain: D9
sprint: acquisition-family-slice-2a
requires_env: [household-nodes]
cites:
  - epr-acquisition-pull-queue-design | the spec whose §6.4 split this plan implements — Slice 2a delivers the REA emit/graduation rails Slice 2b composes | sha256:96164edffcbaf94e | path: genesis/docs/superpowers/specs/2026-06-07-epr-acquisition-pull-queue-design.md
  - mutual-storage-replication-dwelling-hub-design | the REA compute-commitment instance-1 design whose floor-check gap + replicates-commons reservation this rail unblocks | sha256:1acbeeec8b7a3956 | path: genesis/docs/superpowers/specs/2026-05-28-mutual-storage-replication-dwelling-hub-design.md
  - genesis/docs/superpowers/plans/2026-05-28-mutual-storage-replication-dwelling-hub-plan.md
  - genesis/docs/architecture/rea-compute-commitment-primitive.md
  - genesis/docs/research/2026-05-28-sprint3-storage-replication-implementation-notes.md
---

# Slice 2a — REA Economic-Event Emit + Commitment Graduation Rails

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the general-purpose REA rails the acquisition Slice 2b (and the dwelling-hub instance-1 stubs) depend on — a **conductor-path EconomicEvent emit** (bounds-validated, projected with DHT anchor + `bounded_by`), a **commitment graduation** primitive (`proposed → active` via the existing `call_update_rea_commitment_state`), and the **commitment→content scorer-data query** that instance-1 left stubbed — proven end-to-end in a two-conductor sweettest.

**Architecture:** The elohim DNA already exposes both coordinators (`create_rea_economic_event` at `content_store/src/lib.rs:12124` emitting `ReaEconomicEventCommitted` at `:10892`; `update_rea_commitment_state` reachable via the existing `conductor_writes::call_update_rea_commitment_state`). What's missing is the **storage-side wiring**: there is no `call_create_rea_economic_event` conductor wrapper (only a diesel-direct `economic_events::record_event` that writes state `recorded` with no anchor), no bounds-validated emit service, no graduation helper, and the commitment→content scorer query is a stub. This slice supplies those four pieces. It mints **no new DNA action** — that is Slice 2b (`replicates-commons`). It builds **no UI** — Slice 2b.

**Tech stack:** Rust (elohim-storage services + diesel + the `HcClient` conductor bridge), Holochain sweettest (two-conductor, in-process), the existing `bounds_validator` (7-check).

**Env:** `household-nodes` (conductor + storage trio; the sweettest is in-process two-conductor). Storage native build:
`export CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev`, ambient `RUSTFLAGS` as the system sets it, plain `cargo test` (NO nextest in this container; never pipe gate exit codes). The DNA itself is unchanged here, so no WASM rebuild — but the sweettest links the packed `.happ` (`just pack` in the relevant DNA dir if a rebuild is ever needed; not expected this slice).

**Compose, don't fork** (both prior-art lenses): this mirrors the dwelling-hub instance-1 plan's commitment-path tasks (T2/T8/T9/T16) for the event path. **Verify, don't assume** (semantic-lens caveat): Task 1 confirms the existing DNA coordinator + conductor bridge actually fire before the emit service is built on top.

---

## File structure (locked)

| File | Responsibility |
|---|---|
| Modify `elohim/elohim-storage/src/services/conductor_writes.rs` | add `call_create_rea_economic_event` wrapper (mirror `call_create_rea_commitment:60-71`) |
| Create `elohim/elohim-storage/src/services/economic_event_emit_service.rs` | conductor-path emit: build `CreateReaEconomicEventInput`, run `bounds_validator::validate`, call the wrapper, return the action hash for projection |
| Modify `elohim/elohim-storage/src/services/mod.rs` | `pub mod economic_event_emit_service;` |
| Modify `elohim/elohim-storage/src/db/rea_commitments.rs` | add `graduate_commitment_state` (projection-side state flip helper) + `commitments_for_content` query |
| Modify `elohim/elohim-storage/src/services/rea_commitment_service.rs` | add `graduate_to_active` (conductor `call_update_rea_commitment_state` + projection reflect) |
| Modify `elohim/elohim-storage/src/services/replication_prioritizer.rs` | finish the commitment→content data source (un-stub `replication_commitments` / `commitment_backed_replication`) |
| Create `elohim/elohim-storage/tests/rea_event_emit_graduation_e2e.rs` | two-conductor sweettest: emit → anchor; graduate → active; revoked → refused |
| Modify `elohim/elohim-storage/tests/bounds_validator_integration.rs` | add the emit-path revocation-refusal assertion if not already covered |

No new DHT entry types, no new DNA action, no schema files (the EconomicEvent entry + `bounded_by` already exist).

---

### Task 1: Verify the existing DNA coordinator + conductor bridge fire (the "don't assume" gate)

**Files:** Create (temporary, deleted in Step 5): a throwaway probe test, OR extend an existing sweettest. Read-then-prove; no production code yet.

**Context:** Before building the emit service on the `create_rea_economic_event` coordinator, prove it actually works in-process. Sprint-3 close-out flagged `emit_reciprocity_imbalance` as log-only "conductor bridge pending" — confirm the event coordinator isn't similarly inert.

- [ ] **Step 1: Write a probe sweettest** at `elohim/elohim-storage/tests/rea_event_emit_graduation_e2e.rs` (this file is the home for Task 6 too — start it here). Mirror the two-conductor bootstrap of `elohim/elohim-storage/tests/epr_atom_federation_integration.rs` (or the dwelling-hub `tests/harness` two-conductor helper — find the one the `replicates-dwelling` sweettest at the Task-18 commit `3464d8d15` used; grep `tests/` for `two_conductor` / `single_agent_conductor`). The probe: directly call the conductor's `create_rea_economic_event` zome function with a minimal `CreateReaEconomicEventInput` (action `"republish-epr"` — an existing valid action — provider/receiver/has_point_in_time set), and assert it returns a `ReaEconomicEventOutput` with a non-empty `action_hash`.

```rust
// shape — bind names to the real harness:
let input = shefa_types::CreateReaEconomicEventInput {
    id: "probe-event-1".into(),
    action: "republish-epr".into(),
    provider: agent_cid.clone(),
    receiver: agent_cid.clone(),
    has_point_in_time: "2026-06-08T00:00:00Z".into(),
    metadata_json: Some(r#"{"bounded_by":"<some-commitment-cid>"}"#.into()),
    ..Default::default()
};
let out: ReaEconomicEventOutput = conductor.call(&zome, "create_rea_economic_event", input).await;
assert!(!out.action_hash.to_string().is_empty());
```

- [ ] **Step 2: Run it.**
```
cd /projects/elohim/elohim/elohim-storage
export CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev
cargo test --test rea_event_emit_graduation_e2e probe 2>&1 | tail -15
```
Expected: PASS — the coordinator fires and returns an anchor. **If it FAILS** (the coordinator rejects the input, or `bounded_by` metadata validation at `content_store/src/lib.rs:12146` rejects a non-existent commitment CID): that is a real finding — report BLOCKED with the exact rejection, because the emit service can't be built on a broken coordinator. (Likely fix: the probe must first author a real commitment so `bounded_by` resolves; adjust the probe to author a commitment, then emit an event bounded by it.)

- [ ] **Step 3: Commit the probe** (it becomes the seed of the Task-6 e2e; keep it).
```
git add elohim/elohim-storage/tests/rea_event_emit_graduation_e2e.rs
git commit --no-verify -m "test(rea): probe — create_rea_economic_event coordinator fires in two-conductor sweettest (slice-2a T1)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```
Stage ONLY that file (shared worktree; `--no-verify`; never touch CONFESSION.md/THEOLOGY.md/backlog). Do NOT push.

---

### Task 2: `call_create_rea_economic_event` conductor wrapper

**Files:** Modify `elohim/elohim-storage/src/services/conductor_writes.rs`.

- [ ] **Step 1: Add the wrapper** next to `call_create_rea_commitment` (lines 60-71), mirroring it exactly (the `ZOME_NAME = "content_store"` const at line 51 is reused):

```rust
/// Emit a REA EconomicEvent through the conductor (vs the diesel-direct
/// `economic_events::record_event`, which writes no DHT anchor). The
/// post-commit hook emits `ProjectionSignal::ReaEconomicEventCommitted`
/// (content_store/src/lib.rs:10892) → the storage projection upserts with
/// `dht_anchor_hash` (rea_projection.rs:373-434).
pub async fn call_create_rea_economic_event(
    hc: &Arc<HcClient>,
    input: &shefa_types::CreateReaEconomicEventInput,
) -> Result<Vec<u8>, StorageError> {
    let payload = rmp_serde::to_vec_named(input).map_err(|e| {
        StorageError::Internal(format!(
            "conductor_writes: encode CreateReaEconomicEventInput: {e}"
        ))
    })?;
    hc.call_zome(ZOME_NAME, "create_rea_economic_event", payload).await
}
```

- [ ] **Step 2: Build + confirm it compiles** (the `CreateReaEconomicEventInput` type is `shefa_types::CreateReaEconomicEventInput`, lib.rs:172-207):
```
cargo build 2>&1 | tail -3
cargo fmt -- src/services/conductor_writes.rs
```

- [ ] **Step 3: Commit.**
```
git add elohim/elohim-storage/src/services/conductor_writes.rs
git commit --no-verify -m "feat(storage): call_create_rea_economic_event conductor wrapper (slice-2a T2)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 3: `economic_event_emit_service` — bounds-validated conductor-path emit

**Files:** Create `elohim/elohim-storage/src/services/economic_event_emit_service.rs`; modify `services/mod.rs` (`pub mod economic_event_emit_service;`).

**Context:** This is the reusable "emit a notarized EconomicEvent bounded by a commitment" primitive. It runs the 7-check `bounds_validator::validate` FIRST (so a revoked/out-of-bounds event never reaches the conductor), then calls Task 2's wrapper, then projects. `bounded_by` travels in `metadata_json` (the EconomicEvent entry has no struct field for it — content_store validates it from metadata at lib.rs:12146; the `economic_events.bounded_by` column is populated by the projection from the entry).

- [ ] **Step 1: Write the failing test** (unit-level, mocking the conductor + the bounds fetcher — mirror `bounds_validator_integration.rs:17-19`'s `MockCommitmentFetcher`/`MockRateHistory`):

```rust
#[tokio::test]
async fn emit_runs_bounds_validation_before_conductor() {
    // a revoked commitment → emit must return BoundsViolation, conductor never called
    // mirror MockCommitmentFetcher returning a revoked commitment (bounds_validator_integration.rs:64-87)
    // assert the emit service returns Err(BoundsViolation{kind: CommitmentRevoked,...})
    // assert the mock conductor's call count is 0
}
```

- [ ] **Step 2: Implement the service.**

```rust
//! Conductor-path EconomicEvent emit (slice-2a). Every emit is bounds-checked
//! against its `bounded_by` commitment (the 7-check bounds_validator) BEFORE it
//! reaches the conductor — so revoked/out-of-bounds events are refused at the
//! source (un-pin revocation, spec §6.3). Distinct from the diesel-direct
//! `economic_events::record_event` (no anchor, no bounds, legacy).

use std::sync::Arc;
use crate::services::{bounds_validator, conductor_writes};
use crate::services::bounds_validator::{EventForValidation, BoundsViolation};

pub struct EmitEconomicEventInput {
    pub id: String,
    pub action: String,            // e.g. "republish-epr" (2a) / the provide verb (2b)
    pub provider: String,
    pub receiver: String,
    pub has_point_in_time: String,
    pub bounded_by: String,        // Commitment CID
    pub target_epr_id: String,
    pub reach: String,
}

/// Build → validate → emit → return the conductor action hash bytes.
pub async fn emit_bounded_event(
    hc: &Arc<HcClient>,
    fetcher: &impl bounds_validator::CommitmentFetcher,   // mirror the trait used in bounds_validator
    rate: &impl bounds_validator::RateHistory,
    input: EmitEconomicEventInput,
) -> Result<Vec<u8>, EmitError> {
    // 1. bounds (7-check) — refuses revoked/out-of-bounds/rate-exceeded
    let for_val = EventForValidation {
        action: input.action.clone(),
        performer: input.provider.clone(),
        bounded_by: input.bounded_by.clone(),
        target_epr_id: input.target_epr_id.clone(),
        reach: input.reach.clone(),
        signed_at: input.has_point_in_time.clone(),
    };
    bounds_validator::validate(&for_val, fetcher, rate).await
        .map_err(EmitError::Bounds)?;

    // 2. build the conductor input — bounded_by rides metadata_json
    let metadata = serde_json::json!({ "bounded_by": input.bounded_by }).to_string();
    let conductor_input = shefa_types::CreateReaEconomicEventInput {
        id: input.id,
        action: input.action,
        provider: input.provider,
        receiver: input.receiver,
        has_point_in_time: input.has_point_in_time,
        metadata_json: Some(metadata),
        ..Default::default()
    };

    // 3. emit (post-commit signal → projection upserts with dht_anchor + bounded_by)
    conductor_writes::call_create_rea_economic_event(hc, &conductor_input).await
        .map_err(EmitError::Conductor)
}
```

(Bind `CommitmentFetcher`/`RateHistory`/`validate`'s real trait + signature by reading `bounds_validator.rs:100+` — the digest shows `validate` exists; mirror its exact param types and the `BoundsViolation` enum. `EmitError` is a small local enum wrapping `BoundsViolation` + `StorageError`.)

- [ ] **Step 3: Run the unit test** → PASS (revoked commitment short-circuits before the conductor).
```
cargo test --lib economic_event_emit 2>&1 | tail -8
```

- [ ] **Step 4: Commit.**
```
git add elohim/elohim-storage/src/services/economic_event_emit_service.rs elohim/elohim-storage/src/services/mod.rs
git commit --no-verify -m "feat(storage): economic_event_emit_service — bounds-validated conductor-path event emit (slice-2a T3)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 4: Commitment graduation primitive (`proposed → active`)

**Files:** Modify `elohim/elohim-storage/src/services/rea_commitment_service.rs` (add `graduate_to_active`); `elohim/elohim-storage/src/db/rea_commitments.rs` (add the projection-side state flip if needed).

**Context:** The graduation rail. `call_update_rea_commitment_state` ALREADY exists in `conductor_writes.rs` (the 4th wrapper). Graduation = call it with the active state, then the projection reflects it. Per spec §1.2/§6.1 the trigger is the first `ProvideAnnounce` (Slice 2b composes emit + graduate); 2a provides the primitive and verifies it.

- [ ] **Step 1: Write the failing test** — a commitment authored `proposed` (the default at `rea_commitments.rs:308`), after `graduate_to_active`, reads `active` in the projection and is included by `ACTIVE_PROVIDE_STATES` (rea_commitments.rs:215).

- [ ] **Step 2: Implement `graduate_to_active`** mirroring how `create_via_conductor` (rea_commitment_service.rs:218) calls the conductor, but using `conductor_writes::call_update_rea_commitment_state`:

```rust
/// Graduate a commitment proposed → active via the conductor state-transition
/// (the entry is immutable; the transition is a notarized state record the
/// projection reflects). The graduation gate for self-directed commons
/// commitments (spec §6.1): the act of providing is the acceptance — Slice 2b
/// calls this right after the first ProvideAnnounce emit.
pub async fn graduate_to_active(
    hc: &Arc<HcClient>,
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    commitment_cid: &str,
) -> Result<(), StorageError> {
    let out = conductor_writes::call_update_rea_commitment_state(hc, commitment_cid, "active").await?;
    // reflect in projection (eager path, mirror rea_commitment_service.rs:271-276)
    rea_commitments::set_commitment_state(conn, ctx, commitment_cid, "active")?;
    Ok(())
}
```

(Bind `call_update_rea_commitment_state`'s exact signature — read it in `conductor_writes.rs`. Add `rea_commitments::set_commitment_state` if no state-update fn exists; it's a single-column diesel update guarded to only advance `proposed → active`. Confirm the projection signal path doesn't double-write — make the eager update idempotent like the commitment upsert at rea_commitments.rs:271-276.)

- [ ] **Step 3: Run** → PASS. Commit.
```
cargo test --lib graduate 2>&1 | tail -6
git add elohim/elohim-storage/src/services/rea_commitment_service.rs elohim/elohim-storage/src/db/rea_commitments.rs
git commit --no-verify -m "feat(storage): graduate_to_active — commitment proposed→active via conductor state-transition (slice-2a T4)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 5: Finish the commitment→content scorer-data stub

**Files:** Modify `elohim/elohim-storage/src/services/replication_prioritizer.rs` + wherever `DistributionDetails.replication_commitments` / `HouseholdResilienceView.commitment_backed_replication` are computed (grep them — the Sprint-3 notes at `2026-05-28-sprint3-storage-replication-implementation-notes.md:67-75` name them as stubs returning `[]`/zeros).

**Context:** The scorer (`score_advertised_blob`) is action-agnostic, but its commitment data source is stubbed: the rea_commitments-by-content query was deferred in instance-1. Without it, no scorer arm (2b) can see commitment backing. Finish the query.

- [ ] **Step 1: Write the failing test** — seed two `rea_commitments` rows (one active, one proposed) scoping a content id; assert `commitments_for_content(conn, ctx, content_id)` returns only the active one, and that `DistributionDetails.replication_commitments` for that content is non-empty.

- [ ] **Step 2: Implement `commitments_for_content`** in `db/rea_commitments.rs` — a diesel query filtering `state IN ACTIVE_PROVIDE_STATES` (the :215 constant) and the content/scope match (parse the payload scope, mirroring how `active_commitments_for_provider` at `replication_prioritizer.rs:53-96` filters by action + provider). Then replace the stubbed `[]`/zeros in the `DistributionDetails`/`HouseholdResilienceView` computation with calls to it.

- [ ] **Step 3: Run** → PASS. Confirm no instance-1 view test regressed (`cargo test --lib distribution_view reciprocity_view household_resilience 2>&1 | tail`). Commit.
```
git add elohim/elohim-storage/src/services/replication_prioritizer.rs elohim/elohim-storage/src/db/rea_commitments.rs
# + the view-computation file(s) you un-stubbed
git commit --no-verify -m "feat(storage): finish rea_commitments-by-content query — un-stub replication_commitments/commitment_backed_replication (slice-2a T5)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 6: Two-conductor sweettest — the full rail proven

**Files:** Extend `elohim/elohim-storage/tests/rea_event_emit_graduation_e2e.rs` (started in Task 1).

- [ ] **Step 1: Write the e2e** (beyond the Task-1 probe), driving the real services:
  1. Author a commitment (proposed) via the conductor path (`rea_commitment_service::create` → state proposed).
  2. `emit_bounded_event` (Task 3) with `bounded_by` = that commitment → assert it lands with a `dht_anchor_hash` and the `economic_events` projection row has `bounded_by` populated.
  3. `graduate_to_active` (Task 4) → assert the commitment projection reads `active` and is returned by `commitments_for_content` (Task 5).
  4. **Revocation guard**: revoke the commitment (set `revoked_at`), then `emit_bounded_event` again → assert it returns `BoundsViolation::CommitmentRevoked` and emits nothing (the un-pin revocation property, spec §6.3).

- [ ] **Step 2: Run** (generous timeout; mirror the sibling two-conductor test's budget — use `run_in_background` if >10min):
```
cargo test --test rea_event_emit_graduation_e2e 2>&1 | tail -20
```
Expected: all assertions pass.

- [ ] **Step 3: Commit.**
```
git add elohim/elohim-storage/tests/rea_event_emit_graduation_e2e.rs
git commit --no-verify -m "test(rea): two-conductor e2e — emit→anchor, graduate→active, revoked→refused (slice-2a T6)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 7: Gates + ledger close-out

- [ ] **Step 1: Full Rust gates.**
```
cd /projects/elohim/elohim/elohim-storage
export CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev
cargo fmt --check 2>&1 | tail -10
cargo clippy -- -D warnings 2>&1 | tail -10
cargo test --lib 2>&1 | tail -3
cargo test --test rea_event_emit_graduation_e2e --test bounds_validator_integration 2>&1 | tail -6
```
Fix any clippy warning in slice-2a code; fmt clean (only-our-files).

- [ ] **Step 2: ts-rs freshness** (if any view type changed in Task 5): `cargo test export_bindings 2>&1 | tail -3` and commit regenerated TS.

- [ ] **Step 3: Decompose-flip** — this plan's gap-items → CLAIMED (review-verified, awaiting CI). The spec gap-item that 2a partially advances (#9 sync-back / the REA rail) stays OPEN until 2b composes it; note 2a as the rail-prerequisite in the gap-item.
```
python3 .claude/scripts/memory-kit/placement-audit.py --ledger 2>/dev/null | head -12
```

- [ ] **Step 4: Final commit** (only-slice-2a files).
```
git commit --no-verify -m "chore(rea): slice-2a gate pass — REA emit+graduation rails clean (spec §6.4)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Self-review checklist

1. **Spec coverage (§6.4 rail table):** EconomicEvent conductor-emit (T2/T3), bounds-on-emit (T3/T6), commitment graduation (T4), scorer-data un-stub (T5), all proven (T6). NOT in 2a: the `replicates-commons` action mint, the pin sync-back composition, the scorer arm's content-matching, rung-4 UI — all Slice 2b.
2. **Verify-don't-assume:** T1 proves the DNA coordinator fires before T2/T3 build on it; if it's inert, T1 reports BLOCKED rather than building on sand.
3. **No DNA/action/UI scope creep:** this slice mints no action, adds no schema, touches no Angular. Pure storage-side rails + one sweettest.
4. **Type consistency:** `CreateReaEconomicEventInput` (shefa_types:172-207), `EventForValidation` (bounds_validator.rs:68-81), `ACTIVE_PROVIDE_STATES` (rea_commitments.rs:215), `call_update_rea_commitment_state` (existing) — all bound to real signatures, not invented.
5. **Adjust-to-reality seams (deliberate):** the two-conductor harness bootstrap (T1/T6), `bounds_validator::validate`'s exact trait params (T3), `call_update_rea_commitment_state`'s signature (T4), the stubbed view-computation file locations (T5) — each says "bind to the real code," because these are the four places the implementer confirms against the live substrate.
