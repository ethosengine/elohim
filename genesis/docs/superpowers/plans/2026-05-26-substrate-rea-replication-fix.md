---
status: design   # implementation plan — task-by-task fix, not yet verified-stable
---

# Substrate REA Replication Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `POST /api/v1/commitments` (project-epr) and `PATCH /db/content/{id}` (blobHash) round-trip through Holochain coordinator zomes so DHT post-commit gossip propagates the writes to every alpha-cluster peer — closing the substrate replication gap that today produces `/lamad` → 404 and `/` → stale blobHash on `alpha.elohim.host`.

**Architecture:** The substrate is already wired end-to-end EXCEPT the emitter: elohim DNA's `content_store::create_rea_commitment` exists at `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs:11757`, its post-commit emits `ProjectionSignal::ReaCommitmentCommitted` at line 10768, and the receiver `rea_projection::project_signal` at `elohim/elohim-storage/src/rea_projection.rs:148` already projects to local SQL with `dht_anchor_hash`. The bug is that `ReaCommitmentService::create` in `elohim/elohim-storage/src/services/rea_commitment_service.rs` writes diesel directly and emits an in-process EventBus event only — bypassing Holochain entirely, so DHT gossip never fires. This plan replaces the diesel-direct write with an in-process AppWebsocket call to the local conductor's zome function. The same fix applies to `PATCH /db/content/{id}`.

**Scope:** project-epr REA commitments + content blobHash updates ONLY. Other Category-A HTTP writes (relationships, knowledge_maps, agreements, economic_events) have the same anti-pattern but are out of scope for this plan — extending the pattern is a follow-up sprint.

**Tech Stack:** Rust 1.81 (native build, `RUSTFLAGS=""` for storage handler code; `RUSTFLAGS='--cfg getrandom_backend="custom"'` for any zome-side changes), Holochain HDK 0.5, `holochain_client::AppWebsocket`, Diesel + SQLite (projection layer), libp2p / Holochain gossip (substrate transport — already configured for this DNA).

---

## Addendum 1 (2026-05-26 14:10Z) — substrate name corrections discovered during Task 1 pre-flight

**Three corrections to the plan body below — supersede where they conflict:**

1. **Role name is `"lamad"`, not `"elohim"`.** The elohim DNA directory (`elohim/holochain/dna/elohim/`) builds a DNA named `lamad` (per `dna.yaml:20`) which is mounted under the `lamad` role in the elohim hApp (`happ.yaml:24`). The `content_store` zome lives in this role. Use `const ROLE_NAME: &str = "lamad";` in conductor_writes.

2. **Use `HcClient` + `HcClientRegistry`, not raw `AppWebsocket`.** Storage already has:
   - `crate::hc_client::HcClient` (signed call_zome wrapper, takes `app_id + role`)
   - `crate::hc_client_registry::HcClientRegistry` (role-keyed registry; currently has `infrastructure` + `imagodei` fields)
   - `HttpServer.hc_registry: Option<Arc<HcClientRegistry>>` at `http.rs:184` — the conductor connection point is ALREADY in app state

   Task 2 facade calls `registry.lamad.as_ref().ok_or(StorageError::Conductor("lamad bridge offline"))?.call_zome("content_store", "create_rea_commitment", payload)` — strictly through the registry, not AppWebsocket directly.

3. **AppState is `HttpServer` (struct at `http.rs:134`).** Plan's references to "AppState" mean this struct. The conductor pathway is already wired in (Task 5 Step 3 is mostly a no-op — just add `lamad` to HcClientRegistry).

**Revised Task 2 Step 3 (the facade code) — replaces the plan-body version:**

```rust
//! Thin facade for HTTP write handlers to call local conductor zome functions.
//!
//! Per elohim/holochain/dna/CLAUDE.md gospel: "Never write to storage directly
//! for notarized types (legacy code may still do this — migrate toward
//! conductor-first)." This module centralizes the conductor call so the
//! migration happens once. Future Category-A migrations follow the same pattern.

use std::sync::Arc;

use crate::db::rea_commitments::CreateReaCommitmentInput;
use crate::error::StorageError;
use crate::hc_client::HcClient;

const ZOME_NAME: &str = "content_store";

/// Round-trip create_rea_commitment through the local conductor's content_store
/// zome on the `lamad` role. Returns the ActionHash from the conductor's
/// post-commit; caller re-reads the SQL projection (populated by the existing
/// receiver at rea_projection.rs:148) to get the full view.
pub async fn call_create_rea_commitment(
    hc: &Arc<HcClient>,
    input: &CreateReaCommitmentInput,
) -> Result<Vec<u8>, StorageError> {
    let payload = rmp_serde::to_vec_named(input)
        .map_err(|e| StorageError::Internal(format!("encode CreateReaCommitmentInput: {e}")))?;
    hc.call_zome(ZOME_NAME, "create_rea_commitment", &payload).await
}
```

(Note: `HcClient::call_zome` returns `Result<Vec<u8>, StorageError>` per the trait at `hc_client.rs:20` — decoding to `ReaCommitmentOutput` is the caller's concern, since we only need ActionHash for the wait loop and the projection arrives via the post-commit receiver.)

**Task 5 also simplified:** thread `Arc<HcClient>` from `state.hc_registry.as_ref().and_then(|r| r.lamad.clone())` rather than adding new AppWebsocket plumbing. Two-line touch in the handler.

**Task 2 must also add `pub lamad: Option<Arc<HcClient>>` field to `HcClientRegistry`** and wire its connect at startup (mirror lines 39-44 of hc_client_registry.rs).

---

## Addendum 2 (2026-05-26 14:30Z) — Task 3 test-scaffold reality

The plan's Task 3 assumed an HTTP-and-conductor integration test scaffold (`TestState::new().await`, `await_projection`, etc.). The codebase **explicitly punts on this** — see `elohim/elohim-storage/tests/api_placement_gaps.rs:1-17`:

> "The codebase pattern (peer_statuses_route.rs, gate_decisions_http.rs, placement_gaps.rs) tests the DB layer and view conversion directly — no live HTTP server. `spawn_test_server` does not exist in `test_util.rs`; adding a full hyper test harness is deferred."

Building a real-conductor integration test scaffold for this one migration would be a multi-day detour. Pivoting:

- **Task 3**: Replace integration test with a **serialization-roundtrip unit test** in `conductor_writes.rs` itself (`#[cfg(test)] mod tests`). Proves `shefa_types::CreateReaCommitmentInput` encodes/decodes cleanly via `rmp_serde::to_vec_named` (the encoding the DNA's MessagePack reader uses). This is the contract that matters at the wire-shape level — the actual zome execution is covered by Task 9's sweettest.

- **Task 9 promoted**: end-to-end coverage that today's plan deferred to a separate task moves up — Task 9's sweettest IS the regression seatbelt that the original Task 3 imagined.

- **Task 4-8 migrations**: rely on `cargo build --release` (compile-time signature enforcement) as the TDD driver. The breaking signature change in Task 4 is the failing test that drives Task 5.

This is a discipline tradeoff: integration coverage moves from per-task to once-at-task-9, in exchange for shipping the substrate fix to alpha sooner (the user's stated priority).

---

## Addendum 4 (2026-05-26 15:30Z) — Tasks 4-6 worked, but only after Task 6.5

Halfway through execution the substrate revealed it's only HALF-wired. The receiver function `rea_projection::handle_rea_signal` exists fully implemented in `elohim/elohim-storage/src/rea_projection.rs:125` and appears to project REA signals correctly — but **nothing actually calls it**. The function is dead code in the codebase; `grep -rn "handle_rea_signal\|try_handle_signal"` returns only the function definition itself and matching docstrings.

The conductor signal stream IS subscribed to elsewhere — `subscribe_infrastructure_signals` (routes to PeerStatus projection) and `subscribe_elohim_content_signals` (routes to AttestationProjector + RecoveryFlowProjector) — but both filter aggressively: any signal that doesn't deserialize as their specific envelope type (`InfrastructureSignal`, `ElohimContentSignal`) is logged at debug and dropped. So `ProjectionSignal::ReaCommitmentCommitted` from the DNA arrives at storage's process and is silently discarded.

Without an explicit `subscribe_rea_projection_signals` subscriber wired in `main.rs`, the conductor-first HTTP write path (Tasks 4-6) would:
1. POST → service → conductor.create_rea_commitment → DHT entry created ✓
2. DNA post-commit emits ProjectionSignal::ReaCommitmentCommitted ✓
3. AppWebsocket on_signal handler receives it ✓
4. Existing subscribers try-and-fail to deserialize as their envelope type ✗
5. Signal dropped; no SQL projection happens
6. Service's bounded 1s poll for the projection times out
7. HTTP 500 with "REA commitment written via conductor but projection did not land"

The fix (commit `fcfc6069c` — Task 6.5) adds:
- `HcClient::subscribe_rea_projection_signals<F>` (mirrors `subscribe_elohim_content_signals` shape, but filters on `ReaProjectionSignal`)
- A new spawned task in `main.rs` (alongside the two existing subscribers) that wires the subscription to `rea_projection::handle_rea_signal` with the shared DB pool

**This pattern recurs for EVERY new signal variant we want projected.** Future migrations (Content, Path, etc.) must add their own subscriber, not assume the dispatcher catches all signal types.

---

## Addendum 5 (2026-05-26 15:50Z) — Content-side architectural asymmetries

Task 7-8 (content blobHash migration) hit substantially deeper substrate asymmetries than the REA side. Pre-flight investigation surfaced four:

1. **Type lives in lamad-types, not shefa-types.** `CreateContentInput`/`Content`/`ContentOutput` are in `elohim/sdk/domains/lamad/types/src/lib.rs`, not shefa. (REA types live in shefa.) So `UpdateContentInput` (which I added to shefa-types during execution as part of working on Task 6 update_rea_commitment_state) needs to move to lamad-types when the content task is picked up. This isn't a bug — it's a clean-up that a fresh context should do as part of Task 7 scaffolding.

2. **Field naming mismatch.** The HTTP wire (and storage diesel column) is `blob_hash: Option<String>`. The DNA's `Content` entry has `blob_cid: Option<String>` (Phase 0 refactor; see `content_store_integrity/src/lib.rs:498-499`). They semantically mean the same thing (the SHA256 content address) but require translation at the service layer. Document this rename in the zome fn docstring.

3. **No bootstrap path for existing rows.** Content rows on alpha today were created via `bulk_create_content` (diesel-direct, no DHT entry). When stageSpaBlobs PATCHes blob_hash, calling `update_content` zome fn would FAIL with "no prior entry" because there's nothing on the DHT to update. The substrate-correct fix needs service-side bootstrap logic:
   - Read existing SQL row
   - If `dht_anchor_hash IS NULL` → call `create_content` (constructing a full CreateContentInput from the existing SQL row's data + patch applied)
   - If `dht_anchor_hash IS NOT NULL` → call `update_content` (just the patch)
   - Then poll SQL for projection (the same pattern as REA)

   This is a LAZY MIGRATION pattern: existing rows stay diesel-only until first PATCH; then they get DHT-migrated and subsequent updates flow through.

4. **No Content receiver subscriber.** Same gap as REA pre-Task 6.5: the DNA emits `ProjectionSignal::ContentCommitted` (lib.rs:10527, dispatched on entry type) but no storage-side subscriber routes it. The fix is to either:
   - Add `ContentCommitted` variant to `rea_projection::ReaProjectionSignal` enum + handler arm in `handle_rea_signal` (reuses Task 6.5 subscriber)
   - OR add a parallel subscriber `subscribe_content_projection_signals` + dedicated dispatcher

   The first approach is simpler and aligned (REA is a misnomer — these are all "DHT-anchored entity commit signals"). Recommend extending ReaProjectionSignal.

5. **content_diesel needs upsert_with_anchor.** The existing `create_content` and `update_content` in `db/content_diesel.rs` don't take `dht_anchor_hash`. The handler for ContentCommitted needs an INSERT-on-missing-PK + UPDATE-on-existing variant that sets the anchor column. Pattern: mirror `rea_commitments::upsert_with_anchor`.

**Implication for plan execution**: Tasks 7-8 are bigger than the plan originally implied. Realistic scope:
- Task 7: failing serde-roundtrip test for UpdateContentInput in lamad-types (~30 min)
- Task 8a: add Content variant to ReaProjectionSignal + handler + content_diesel upsert_with_anchor (~1.5 hr)
- Task 8b: add update_content + lazy-bootstrap zome surface (~1 hr)
- Task 8c: migrate ContentService::update with bootstrap (~1 hr)
- Task 8d: thread hc_lamad to PATCH http.rs handler (~30 min)
- Build + commit cycles add ~30 min

Total: 4-5 hours real-time, ~6 commits.

---

## Current state at handoff (2026-05-26 ~16:00Z)

**Branch:** `dev` (qahal-m1 worktree at `/projects/elohim-worktrees/qahal-m1/`)
**Commits ahead of origin/dev:** 9 (all local, NOT pushed)

| Commit | Subject | Status |
|---|---|---|
| `f90ce1e90` | chore(claude): preserve feature-promise from 2026-05-26 deliver iter-0 | reference artifact |
| `dc6430049` | docs(plan): substrate-rea-replication-fix — 10-task plan | the plan |
| `88d160263` | docs(plan): substrate-rea-replication-fix addendum 1 — name corrections | addendum |
| `d919f1164` | feat(elohim-storage): add conductor_writes facade + wire lamad HcClient | Task 2 |
| `b9edb1b07` | test(elohim-storage): serde roundtrip test for CreateReaCommitmentInput | Task 3 |
| `fff4dfd6a` | feat(elohim-storage): project-epr commitments round-trip through conductor | Tasks 4+5 |
| `58bb9be02` | feat(elohim-dna,elohim-storage): PATCH /api/v1/commitments/{id} round-trips through conductor | Task 6 |
| `fcfc6069c` | feat(elohim-storage): wire REA projection signal subscriber | Task 6.5 |
| *(this commit)* | docs(plan): addenda 4-5 — execution findings + handoff state | this addendum |

**Tasks status:**
- ✅ Tasks 1-6 + 6.5 — REA commitment substrate-correct write path fully wired (project-epr commitments will replicate cross-peer once deployed)
- ⏸️ Task 7-8 — content blobHash migration (not started; see Addendum 5 for revised scope)
- ⏸️ Task 9 — multi-peer sweettest (covers both REA + content once 8 lands)
- ⏸️ Task 10 — deploy + verify on alpha

**Untouched files needing cleanup before content task starts:**
- `elohim/sdk/domains/shefa/types/src/lib.rs` — `UpdateContentInput` was added here during Task 6 work but should be MOVED to `elohim/sdk/domains/lamad/types/src/lib.rs` (the content's home crate). Currently unused (no caller), so the move is risk-free.

**Verification not yet done:**
- No CI run for the REA-substrate changes; pre-push gate not run; alpha not probed
- Sweettest seatbelt (Task 9) absent
- Real-conductor integration test scaffold for storage tests/ still absent (per Addendum 2)

**Outstanding strategic question:** Should the REA-substrate fix push to dev BEFORE the content fix lands? Pushing now:
- Closes /lamad on alpha (the most user-visible failure)
- Leaves the PLACEHOLDER white-screen on / unfixed (needs content substrate)
- Provides 30-60 min of CI cycle time during which content work can proceed
- Operator chose "atomic deploy" in original execution; this addendum revises that recommendation given the depth findings — pushing the working REA fix has independent value, and the content fix can ship as a second deploy when ready

---

## Handoff prompt — pick this up in a fresh context

> Resume execution of `genesis/docs/superpowers/plans/2026-05-26-substrate-rea-replication-fix.md` from Task 7 in worktree `/projects/elohim-worktrees/qahal-m1/` (branch: `dev`, 9 commits ahead of origin/dev).
>
> **Read first (in this order):**
> 1. The plan's main body + all 5 addenda. Addenda 4 + 5 carry the execution discoveries about substrate subscriber wiring and content-side asymmetries — they supersede the plan body where they conflict.
> 2. The "Current state at handoff" section for what's done vs remaining.
> 3. Last night's sprint result at `.claude/shifts/2026-05-26T08-35-shift-epr-app-delivery.sprint-result.md` (Gap C + Gap D context).
>
> **First decision to make (with operator input):** Should the 9 existing commits push to `dev` NOW (closes /lamad alone), or hold until the content fix lands too (atomic deploy)? Addendum 5's revised recommendation is to push now. If pushing: `cd /projects/elohim-worktrees/qahal-m1 && git push origin dev`, then ci-observer the orchestrator + downstream pipelines.
>
> **Then execute Tasks 7-10 per Addendum 5's revised scope:**
> - **Task 7** (failing test): move `UpdateContentInput` from shefa-types to lamad-types, add a serde-roundtrip test mirroring the one at `elohim/elohim-storage/src/services/conductor_writes.rs::tests`.
> - **Task 8a**: add `ContentCommitted` variant to `ReaProjectionSignal` enum (`elohim/elohim-storage/src/rea_projection.rs:45`) + handler arm in `handle_rea_signal` (line 125). Add `upsert_with_anchor` to `db/content_diesel.rs` mirroring the `rea_commitments::upsert_with_anchor` shape.
> - **Task 8b**: add `update_content` coordinator zome fn in `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` (insert near line 2434 after `create_content_unchecked`). The fn assumes prev entry exists; bootstrap logic for existing diesel-only rows lives in the SERVICE layer (Task 8c), not the zome.
> - **Task 8c**: add `ContentService::update_via_conductor` that branches on `dht_anchor_hash IS NULL`: if null, call `call_create_content` (using the existing SQL row's data as the base + patch applied) — this is the lazy-migration bootstrap. Otherwise call `call_update_content`. Poll SQL for projection ≤1s.
> - **Task 8d**: thread `hc_lamad` from `HcClientRegistry` through the PATCH handler at `http.rs:3821`. Branch on `input.blob_hash.is_some()` — only blob_hash patches need the conductor round-trip; other patches stay diesel-direct for now.
> - **Task 9**: sweettest at `elohim/holochain/tests/sweettest/tests/rea_commitment_replication.rs` per plan body. Per memory `_sweettest_cross_agent_consistency`, needs `exchange_peer_infos` + `await_consistency`.
> - **Task 10**: deploy + alpha verification per plan body.
>
> **Key gotchas (caught during execution):**
> - Every new ReaProjectionSignal variant needs a corresponding signal-subscriber wired in `main.rs` (the pattern at `subscribe_rea_projection_signals` is the template). The receiver function being implemented is NOT enough — without an explicit on_signal subscription, the signal is dropped silently.
> - Storage's `CreateReaCommitmentInput` (Option<String> id, Option<String> in_scope_of) and `shefa_types::CreateReaCommitmentInput` (String id, Vec<String> in_scope_of) diverge — conversion lives in the service layer (see `to_shefa_input` in `rea_commitment_service.rs`). Same pattern needed for content.
> - DNA build uses plain cargo (no CARGO_TARGET_DIR override per `elohim/holochain/dna/CLAUDE.md`); storage build uses `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev` and `RUSTFLAGS='--cfg getrandom_backend="custom"'`.
> - The plan's per-task "failing test" Task discipline gave way to compile-time signature enforcement + serde roundtrip tests, since the codebase explicitly punts on full HTTP+conductor integration test scaffold (see Addendum 2). Sweettest (Task 9) is the real seatbelt.
>
> **Stop conditions:** if any new substrate asymmetry takes >1hr to surface, checkpoint with the operator rather than grinding. Addenda 4 + 5 are evidence that this codebase has more half-built primitives than greps suggest.

---

Mid-execution finding: Task 8 (content blobHash) is substantially heavier than the plan body implied. The content side does NOT have a `rea_projection`-style receiver in storage — `ContentCommitted` signals fire from the DNA's post-commit (lib.rs:10527) but no storage subscriber projects them to SQL. So full substrate-correct content fix requires:

1. Add a Content-signal receiver in storage (mirror `rea_projection::project_signal` shape, dispatch on `ContentCommitted`).
2. Wire it into the signal dispatcher in `signals.rs`.
3. Add `update_content` coordinator zome fn (does `update_entry` on the Content entry; emits via existing post-commit dispatch).
4. Migrate ContentService::create + update_blob_hash to round-trip through it.

Operator decision: do the full substrate-correct work and ship everything in one atomic deploy. Not splitting the project-epr fix from the content fix. Plan executes through Task 10 before any push.

Net effect on plan ordering:
- Task 6 stays in place (PATCH commitment state — small, mirrors Task 4-5)
- Task 7 = serde roundtrip test for content update payload
- Task 8 = full substrate fix per the 4 sub-steps above
- Task 9 = sweettest seatbelt (covers both REA + content)
- Task 10 = push + alpha verification

Expected wall-clock at this point: Tasks 6-10 sequential ≈ 1-2 days. Operator's priority is correctness over speed.

---

**Verification:** Pass criteria is end-to-end on alpha — `curl https://alpha.elohim.host/api/v1/commitments?action=project-epr` returns ≥1 row with non-null `dhtAnchorHash`, AND `curl https://alpha.elohim.host/lamad` returns 200 (HTML), AND the same on `alpha.elohim.host/` (no PLACEHOLDER in X-Content-Address).

---

## File Structure

### Files to create

| Path | Responsibility |
|---|---|
| `elohim/elohim-storage/src/services/conductor_writes.rs` | Thin facade module: `call_create_rea_commitment(app_ws, input)` and `call_update_content(app_ws, input)`. Wraps AppWebsocket zome calls in typed helpers with error mapping. Single point of contact for "write to local conductor." |
| `elohim/elohim-storage/tests/conductor_roundtrip_test.rs` | Integration test (single conductor): POST /api/v1/commitments → assert SQL row has non-null `dht_anchor_hash` within 5s. |
| `elohim/holochain/tests/sweettest/tests/rea_commitment_replication.rs` | Two-conductor sweettest: peer A creates commitment via HTTP → exchange_peer_info + await_consistency → peer B's projection has the row. Proves DHT gossip propagates this entry type. |

### Files to modify

| Path | Change |
|---|---|
| `elohim/elohim-storage/src/services/rea_commitment_service.rs:17-32` | `create()` accepts `&AppWebsocket`, calls `conductor_writes::call_create_rea_commitment` instead of `rea_commitments::create_commitment` (diesel). Diesel write happens in the existing receiver path via `rea_projection::project_signal` after post-commit. |
| `elohim/elohim-storage/src/services/rea_commitment_service.rs:61-77` | `update_state()` migration analogous to `create()`. (May require a new `update_rea_commitment` coordinator zome function — see Task 7.) |
| `elohim/elohim-storage/src/services/content_service.rs` | `create()` and `update_blob_hash()` migrate to round-trip through `content_store::create_content` / `content_store::update_content`. |
| `elohim/elohim-storage/src/http.rs:9357,9374,9404,9429` | Handler functions for `create_commitment`, `update_commitment_state`, `create_content`, `update_content` thread `app_ws: &AppWebsocket` from `AppState`. |
| `elohim/elohim-storage/src/lib.rs` | Add `pub mod conductor_writes;` under services. |
| `elohim/elohim-storage/src/services/mod.rs` | Same — register the new module. |

### Files to read (no changes expected)

| Path | Why |
|---|---|
| `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs:11757-11846` | `create_rea_commitment` coordinator — confirms the entry-create + signal-emit shape. |
| `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs:10768` | Post-commit emit point for ReaCommitmentCommitted signal. |
| `elohim/elohim-storage/src/rea_projection.rs:38-244` | Receiver-side projection — confirms `upsert_with_anchor(..., Some(&action_hash))` writes the projection. No change expected. |
| `elohim/elohim-storage/src/signing.rs:98-237` | Existing `AppWebsocket` usage pattern + auth token issuance — Task 2 mirrors this. |
| `elohim/elohim-storage/src/content_server.rs:267,314,349,378` | Existing `call_zome` examples — establishes the calling convention. |
| `elohim/holochain/dna/CLAUDE.md` | Gospel-tier guidance: "Never write to storage directly for notarized types (legacy code may still do this — migrate toward conductor-first)." |

---

## Pre-flight Self-Verification

Before starting Task 1, walk these checks:

- [ ] Confirm `qahal-m1` worktree is at `origin/dev` tip (currently `c204b7394`)
- [ ] Confirm uncommitted `.claude/deliver/feature-promise-epr-app-delivery.json` is preserved or committed first (it's a session artifact; safe to commit on dev as a separate, isolated commit at start of work)
- [ ] Run `grep -n "update_content\|update_rea_commitment\|fn update_content" elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` and document findings — if `update_content` (taking blobHash) does NOT exist as `#[hdk_extern]`, **Task 7 expands** to add it (a new coordinator function with its own validator + post-commit signal). Document the result in this checkbox.
- [ ] Run `grep -rn "pub.*AppState\|struct AppState" elohim/elohim-storage/src/` and locate AppState definition. Note its file:line. The handler functions need to read `app_ws: AppWebsocket` from it.
- [ ] Verify `AppWebsocket` is currently stored in AppState. If not, Task 2 includes adding it (additive change, no migration risk).
- [ ] Confirm we can build elohim-storage today: `cd elohim/elohim-storage && CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release` succeeds.

---

## Task 1: Pre-flight verification + green baseline

**Files:** None modified — investigation + baseline-confirmation only.

- [ ] **Step 1: Switch to qahal-m1 worktree**

```bash
cd /projects/elohim-worktrees/qahal-m1
git status
# Expected: "On branch dev", working tree clean (or only the feature-promise.json carried over)
```

- [ ] **Step 2: Confirm baseline build green (storage)**

```bash
cd elohim/elohim-storage
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev \
RUSTFLAGS='--cfg getrandom_backend="custom"' \
cargo build --release 2>&1 | tail -20
```
Expected: `Finished release [optimized] target(s)`. If not, STOP — baseline must be green before substrate edits.

- [ ] **Step 3: Confirm baseline build green (elohim DNA)**

```bash
cd /projects/elohim-worktrees/qahal-m1/elohim/holochain/dna/elohim
just check 2>&1 | tail -10
```
Expected: `Finished` no errors. (DNA workspaces use plain cargo per CLAUDE.md — do NOT override CARGO_TARGET_DIR.)

- [ ] **Step 4: Verify zome surface**

```bash
grep -n "fn create_rea_commitment\|fn update_content\|fn update_rea_commitment\|fn create_content" \
  /projects/elohim-worktrees/qahal-m1/elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
```
Expected: at minimum `fn create_rea_commitment` (line ~11757) AND `fn create_content` exist. Document result. If `update_content` is missing, flag for Task 7 expansion.

- [ ] **Step 5: Commit baseline note**

```bash
cd /projects/elohim-worktrees/qahal-m1
# Stage the inherited feature-promise as a clean isolated artifact commit if not already there
git add .claude/deliver/feature-promise-epr-app-delivery.json
git commit -m "chore(claude): preserve feature-promise from 2026-05-26 deliver iter-0

Reference artifact from last night's diagnostic shift; informs the
substrate-rea-replication-fix plan. No code impact."
```

---

## Task 2: Add conductor_writes facade module

**Files:**
- Create: `elohim/elohim-storage/src/services/conductor_writes.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

- [ ] **Step 1: Verify AppWebsocket usage pattern (read-only)**

```bash
sed -n '95,120p;185,210p;225,245p' /projects/elohim-worktrees/qahal-m1/elohim/elohim-storage/src/signing.rs
```
Read the `LiveAppWebsocket` shape, the `issue_app_auth_token` + `connect` dance, and an example `call_zome` invocation. This is the template for `conductor_writes.rs`.

- [ ] **Step 2: Inspect existing call_zome shape**

```bash
sed -n '260,330p' /projects/elohim-worktrees/qahal-m1/elohim/elohim-storage/src/content_server.rs
```
Note: zome name is `"content_store"`, the DNA is the elohim DNA, payload is serialized via `ExternIO::encode`.

- [ ] **Step 3: Write the facade**

Create `elohim/elohim-storage/src/services/conductor_writes.rs`:

```rust
//! Thin facade for HTTP write handlers to call local conductor zome functions.
//!
//! Why this exists: per genesis/docs/superpowers/plans/2026-05-26-substrate-rea-replication-fix.md,
//! Category-A entities (notarized via DHT) must round-trip through the
//! Holochain coordinator on HTTP write. Direct diesel writes (the legacy
//! pattern flagged in elohim/holochain/dna/CLAUDE.md) bypass DHT gossip
//! and break cross-peer replication.
//!
//! This module centralizes the in-process AppWebsocket call so the migration
//! happens once. Future Category-A migrations (relationships, agreements,
//! economic_events) follow the same pattern.

use holochain_client::AppWebsocket;
use holochain_zome_types::prelude::ExternIO;

use crate::db::rea_commitments::CreateReaCommitmentInput;
use crate::error::StorageError;
use crate::views::ReaCommitmentView;

const ZOME_NAME: &str = "content_store";
const ROLE_NAME: &str = "elohim";

/// Call the local conductor's `create_rea_commitment` zome function.
///
/// Side effect: post-commit emits `ProjectionSignal::ReaCommitmentCommitted`,
/// which is consumed by `rea_projection::project_signal` and projected into
/// the local SQL `rea_commitments` table with `dht_anchor_hash` populated.
/// Holochain DHT gossip propagates the entry to other peers, whose
/// projection handlers do the same upsert on their local SQL.
pub async fn call_create_rea_commitment(
    app_ws: &AppWebsocket,
    input: CreateReaCommitmentInput,
) -> Result<ReaCommitmentOutputWire, StorageError> {
    let payload = ExternIO::encode(&input)
        .map_err(|e| StorageError::Internal(format!("encode CreateReaCommitmentInput: {e}")))?;

    let result = app_ws
        .call_zome(
            ROLE_NAME.into(),
            ZOME_NAME.into(),
            "create_rea_commitment".into(),
            payload,
        )
        .await
        .map_err(|e| StorageError::Conductor(format!("call_zome create_rea_commitment: {e}")))?;

    let output: ReaCommitmentOutputWire = result
        .decode()
        .map_err(|e| StorageError::Internal(format!("decode ReaCommitmentOutput: {e}")))?;

    Ok(output)
}

/// Mirror of the zome's `ReaCommitmentOutput` (decoded on this side).
/// The full View is reconstructed from the SQL projection — this wire shape
/// is just the action_hash for the post-commit ack window.
#[derive(Debug, serde::Deserialize)]
pub struct ReaCommitmentOutputWire {
    pub action_hash: holo_hash::ActionHash,
    pub entry_hash: holo_hash::EntryHash,
    // commitment field intentionally omitted — caller re-fetches from SQL projection
}
```

- [ ] **Step 4: Register module**

Modify `elohim/elohim-storage/src/services/mod.rs` — add line:

```rust
pub mod conductor_writes;
```

- [ ] **Step 5: Verify it compiles (no callers yet)**

```bash
cd /projects/elohim-worktrees/qahal-m1/elohim/elohim-storage
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev \
RUSTFLAGS='--cfg getrandom_backend="custom"' \
cargo build --release 2>&1 | tail -10
```
Expected: clean build. If `StorageError::Conductor` variant doesn't exist, add it to `error.rs` as a sub-step:

```rust
#[error("conductor call failed: {0}")]
Conductor(String),
```

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/services/conductor_writes.rs \
        elohim/elohim-storage/src/services/mod.rs \
        elohim/elohim-storage/src/error.rs
git commit -m "feat(elohim-storage): add conductor_writes facade for round-trip HTTP→zome writes

Thin module that wraps AppWebsocket call_zome for the migration of
Category-A HTTP writes from diesel-direct to conductor-first. First
consumer is ReaCommitmentService::create (Task 4).

Per elohim/holochain/dna/CLAUDE.md: 'Never write to storage directly
for notarized types (legacy code may still do this — migrate toward
conductor-first).' This is the first step in that migration.

Plan: genesis/docs/superpowers/plans/2026-05-26-substrate-rea-replication-fix.md"
```

---

## Task 3: Failing test — single-peer REA commitment round-trip

**Files:**
- Create: `elohim/elohim-storage/tests/conductor_roundtrip_test.rs`

- [ ] **Step 1: Locate existing integration test scaffolding**

```bash
ls /projects/elohim-worktrees/qahal-m1/elohim/elohim-storage/tests/
grep -l "test_state\|TestState\|build_test_state\|setup_test_conductor" \
  /projects/elohim-worktrees/qahal-m1/elohim/elohim-storage/tests/*.rs
```
Note the helper(s) the existing tests use to spin up a conductor + storage stack. Read one example for the setup pattern.

- [ ] **Step 2: Write the failing test**

Create `elohim/elohim-storage/tests/conductor_roundtrip_test.rs`:

```rust
//! End-to-end test: HTTP POST /api/v1/commitments round-trips through the
//! local conductor's content_store::create_rea_commitment zome and the
//! projection is written to local SQL with non-null dht_anchor_hash.
//!
//! Plan: genesis/docs/superpowers/plans/2026-05-26-substrate-rea-replication-fix.md
//! Task 3 — the failing test that drives Task 4's migration.

use elohim_storage::db::rea_commitments::PROJECT_EPR_ACTION;
use elohim_storage::services::rea_commitment_service::ReaCommitmentService;

// ADAPT to the actual test-state helper found in Step 1.
mod common;
use common::TestState;

#[tokio::test]
async fn http_post_commitment_writes_dht_anchored_projection() {
    let state = TestState::new().await;
    let now = chrono::Utc::now().to_rfc3339();

    let input = elohim_storage::db::rea_commitments::CreateReaCommitmentInput {
        id: "test-projection-commit-001".to_string(),
        action: PROJECT_EPR_ACTION.to_string(),
        provider: "doorway:test-doorway".to_string(),
        receiver: "epr:test-app".to_string(),
        in_scope_of: Some("doorway:test-doorway|epr:test-app".to_string()),
        resource_classified_as: Vec::new(),
        resource_quantity_value: None,
        resource_quantity_unit: None,
        effort_quantity_value: None,
        effort_quantity_unit: None,
        has_beginning: None,
        has_end: None,
        due: None,
        clause_of: None,
        medium_of_exchange_id: None,
        note: None,
        metadata_json: None,
    };

    // Drive through the service layer (proxy for HTTP handler).
    let view = ReaCommitmentService::create(
        &mut state.conn(),
        &state.ctx,
        input,
        Some(&state.event_bus),
        &state.app_ws, // <-- NEW parameter, added in Task 4
    )
    .await
    .expect("create should succeed");

    // Wait up to 5s for post-commit signal → projection to land.
    let projection = state
        .await_projection(&view.id, std::time::Duration::from_secs(5))
        .await
        .expect("projection should arrive within 5s");

    assert!(
        projection.dht_anchor_hash.is_some(),
        "dht_anchor_hash MUST be populated after conductor-first write — \
         current diesel-direct path leaves it NULL, which is the regression \
         this plan closes."
    );
    assert_eq!(projection.action, PROJECT_EPR_ACTION);
}
```

- [ ] **Step 3: Run the test and verify it fails (for the right reason)**

```bash
cd /projects/elohim-worktrees/qahal-m1/elohim/elohim-storage
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev \
RUSTFLAGS='--cfg getrandom_backend="custom"' \
cargo test --release --test conductor_roundtrip_test 2>&1 | tail -20
```
Expected: **compile error** (Service::create signature doesn't take app_ws yet). This is the right failure — drives Task 4. If it compiles and runs to assertion-fail, even better.

- [ ] **Step 4: Commit the failing test**

```bash
git add elohim/elohim-storage/tests/conductor_roundtrip_test.rs
git commit -m "test(elohim-storage): failing test for HTTP→conductor round-trip on REA commitment

Asserts that POST /api/v1/commitments writes a SQL row with non-null
dht_anchor_hash. Currently fails to compile because Service::create
does not yet accept an AppWebsocket — that's Task 4.

Plan: genesis/docs/superpowers/plans/2026-05-26-substrate-rea-replication-fix.md"
```

---

## Task 4: Migrate ReaCommitmentService::create to conductor-first

**Files:**
- Modify: `elohim/elohim-storage/src/services/rea_commitment_service.rs:17-32`

- [ ] **Step 1: Read existing implementation for reference**

```bash
sed -n '1,79p' /projects/elohim-worktrees/qahal-m1/elohim/elohim-storage/src/services/rea_commitment_service.rs
```
Note current signature: synchronous, takes `&mut SqliteConnection`, calls `rea_commitments::create_commitment(conn, ctx, input)` then emits EventBus.

- [ ] **Step 2: Replace `create` with async + conductor round-trip**

In `elohim/elohim-storage/src/services/rea_commitment_service.rs`, replace lines 17-32 with:

```rust
    /// Create an REA commitment by round-trip through the local conductor's
    /// content_store::create_rea_commitment zome function. This is the
    /// substrate-correct write path:
    ///
    ///   HTTP POST
    ///     → ReaCommitmentService::create
    ///     → conductor_writes::call_create_rea_commitment (AppWebsocket)
    ///     → content_store zome create_entry + post_commit signal
    ///     → rea_projection::project_signal (in this process)
    ///     → rea_commitments::upsert_with_anchor (with action_hash)
    ///     → SQL row, dht_anchor_hash populated
    ///   (in parallel: Holochain DHT gossip propagates entry to all peers,
    ///   each peer's signal subscriber projects to its own SQL)
    ///
    /// Caller responsibility: pass the AppWebsocket from AppState. The
    /// EventBus argument is preserved for doorway's local SSE subscriber
    /// — that fires on the in-process bus AFTER projection completes.
    pub async fn create(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        input: CreateReaCommitmentInput,
        events: Option<&EventBus>,
        app_ws: &holochain_client::AppWebsocket,
    ) -> Result<ReaCommitmentView, StorageError> {
        use crate::services::conductor_writes;

        // 1. Round-trip through the conductor. Post-commit signal projects
        //    into the local SQL row with dht_anchor_hash.
        let _output = conductor_writes::call_create_rea_commitment(app_ws, input.clone()).await?;

        // 2. Wait briefly for the projection to land (the post_commit signal
        //    is processed on a separate task; we don't block on it inside the
        //    zome call). 500ms is generous for in-process delivery; this can
        //    be tightened post-soak.
        for _ in 0..10 {
            if let Some(found) = rea_commitments::get_commitment(conn, ctx, &input.id)? {
                if let Some(bus) = events {
                    if found.action == PROJECT_EPR_ACTION {
                        bus.emit(StorageEvent::ProjectionRegistered {
                            commitment_id: found.id.clone(),
                        });
                    }
                }
                return Ok(ReaCommitmentView::from(found));
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        Err(StorageError::Internal(format!(
            "REA commitment {} written via conductor but projection did not \
             land within 500ms — check rea_projection signal pipeline",
            input.id
        )))
    }
```

Note: `create` is now **async** — callers must `.await`. This propagates to the HTTP handler. Mark the breaking signature change in the commit message.

- [ ] **Step 3: Run the failing test from Task 3**

```bash
cd /projects/elohim-worktrees/qahal-m1/elohim/elohim-storage
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev \
RUSTFLAGS='--cfg getrandom_backend="custom"' \
cargo test --release --test conductor_roundtrip_test 2>&1 | tail -30
```
Expected: **compile error in `http.rs` handler** (handler still calls `create` synchronously). That's Task 5. The test crate itself should compile.

- [ ] **Step 4: Stash the unrelated build error temporarily**

The HTTP handler is the next task. To keep this task focused and committable: temporarily mark the handler `#[allow(unused)]` and proceed if it's the only break. Otherwise:

```bash
cd /projects/elohim-worktrees/qahal-m1/elohim/elohim-storage
# This task's commit is the service migration alone — Task 5 wires the handler.
git add elohim/elohim-storage/src/services/rea_commitment_service.rs
git commit -m "feat(elohim-storage): ReaCommitmentService::create round-trips through conductor

BREAKING: create() is now async and takes an AppWebsocket. Caller (HTTP
handler in http.rs:create_commitment) updated in next commit.

Per elohim/holochain/dna/CLAUDE.md: 'Never write to storage directly
for notarized types.' This migration closes the substrate gap where
HTTP POST /api/v1/commitments bypassed Holochain entirely, breaking
cross-peer DHT gossip propagation.

Effect on alpha: project-epr commitments seeded via the Seed Projections
stage will now reach all peers via DHT, not just the one peer
seedProjections.ts happened to POST to.

Plan: genesis/docs/superpowers/plans/2026-05-26-substrate-rea-replication-fix.md"
```

---

## Task 5: Thread AppWebsocket through the HTTP handler

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs` (handler function for "create_commitment")
- Possibly: `elohim/elohim-storage/src/state.rs` (or wherever AppState lives) — add AppWebsocket if not already present

- [ ] **Step 1: Locate the handler function**

The route at `http.rs:9357` binds string `"create_commitment"`. Find where that string dispatches to a function:

```bash
grep -n "create_commitment\b" /projects/elohim-worktrees/qahal-m1/elohim/elohim-storage/src/http.rs
```
The handler is likely a function or match arm taking the AppState + Request and returning a Response. Note the file:line.

- [ ] **Step 2: Locate AppState definition**

```bash
grep -rn "pub struct AppState\|struct AppState\|impl AppState" \
  /projects/elohim-worktrees/qahal-m1/elohim/elohim-storage/src/
```
Note the file. If AppState already holds an AppWebsocket, skip Step 3.

- [ ] **Step 3 (conditional): Add AppWebsocket to AppState**

If AppState does not yet hold an AppWebsocket, follow `signing.rs:98-200` as a template. Add field:

```rust
pub app_ws: Arc<holochain_client::AppWebsocket>,
```

…and initialize it in `AppState::new()` (or wherever AppState is constructed) using the same `issue_app_auth_token` + `connect` pattern in `signing.rs:185-200`. Be explicit about role_name: `"elohim"` (the DNA role hosting content_store).

- [ ] **Step 4: Thread AppWebsocket into the handler**

In the handler for `create_commitment`, change:

```rust
// OLD:
let view = ReaCommitmentService::create(&mut conn, &ctx, input, Some(&state.event_bus))?;
```

to:

```rust
// NEW:
let view = ReaCommitmentService::create(
    &mut conn,
    &ctx,
    input,
    Some(&state.event_bus),
    &state.app_ws,
)
.await?;
```

If the handler was not already async, mark it so. This will cascade — fix call sites one at a time.

- [ ] **Step 5: Run the integration test**

```bash
cd /projects/elohim-worktrees/qahal-m1/elohim/elohim-storage
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev \
RUSTFLAGS='--cfg getrandom_backend="custom"' \
cargo test --release --test conductor_roundtrip_test 2>&1 | tail -30
```
Expected: test **passes** — projection arrives within 5s with non-null dht_anchor_hash.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/http.rs elohim/elohim-storage/src/state.rs
git commit -m "feat(elohim-storage): HTTP create_commitment handler threads AppWebsocket

Completes the round-trip migration started in the previous commit.
POST /api/v1/commitments now writes through the local conductor and
the projection lands via the existing post-commit signal pipeline.

Single-peer test (tests/conductor_roundtrip_test.rs) now passes;
dht_anchor_hash is populated on the SQL row.

Plan: genesis/docs/superpowers/plans/2026-05-26-substrate-rea-replication-fix.md"
```

---

## Task 6: Migrate `update_commitment_state` (PATCH /api/v1/commitments/{id})

**Files:**
- Modify: `elohim/elohim-storage/src/services/rea_commitment_service.rs:61-77`
- Modify: corresponding HTTP handler in `http.rs`
- Conditional: add `update_rea_commitment` coordinator zome function if absent

- [ ] **Step 1: Check zome surface**

```bash
grep -n "fn update_rea_commitment\|fn update_commitment\|fn cancel_commitment\|fn revoke_commitment" \
  /projects/elohim-worktrees/qahal-m1/elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
```

If a coordinator function for state updates EXISTS: skip to Step 3. If absent: Step 2 adds it.

- [ ] **Step 2 (conditional): Add `update_rea_commitment_state` coordinator function**

Add to `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` (alongside `create_rea_commitment` around line 11757):

```rust
#[hdk_extern]
pub fn update_rea_commitment_state(
    input: UpdateReaCommitmentStateInput,
) -> ExternResult<ReaCommitmentOutput> {
    // Locate the latest action for this commitment id via the id_anchor link.
    let id_anchor = StringAnchor::new("commitment_id", &input.id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    let links = get_links(
        GetLinksInputBuilder::try_new(id_anchor_hash, LinkTypes::IdToCommitment)?.build(),
    )?;
    let latest_link = links.last().ok_or(wasm_error!(WasmErrorInner::Guest(format!(
        "no commitment found for id {}",
        input.id
    ))))?;
    let prev_action_hash: ActionHash = latest_link
        .target
        .clone()
        .into_action_hash()
        .ok_or(wasm_error!(WasmErrorInner::Guest("link target not action hash".into())))?;

    // Read the previous entry, mutate state field, write a new entry as update.
    let record = get(prev_action_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("prev entry missing".into())))?;
    let mut commitment: Commitment = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("decode prev: {e}"))))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("no entry on record".into())))?;

    commitment.state = input.state;
    commitment.updated_at = format!("{:?}", sys_time()?);

    let new_action_hash =
        update_entry(prev_action_hash, &EntryTypes::Commitment(commitment.clone()))?;
    let entry_hash = hash_entry(&EntryTypes::Commitment(commitment.clone()))?;

    Ok(ReaCommitmentOutput {
        action_hash: new_action_hash,
        entry_hash,
        commitment: commitment_to_wire(&commitment),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateReaCommitmentStateInput {
    pub id: String,
    pub state: String,
}
```

Add a parallel post-commit emit if not already covered by the existing `post_commit` handler — check the `post_commit(actions)` function around line 10700 for whether it dispatches on UPDATE actions or only CREATE.

Build the DNA: `cd elohim/holochain/dna/elohim && just check && just build`.

- [ ] **Step 3: Add `call_update_rea_commitment_state` to conductor_writes**

In `conductor_writes.rs`, mirror the `call_create_rea_commitment` pattern with function name `update_rea_commitment_state`.

- [ ] **Step 4: Migrate `ReaCommitmentService::update_state`**

In `rea_commitment_service.rs:61-77`, mirror the `create` migration: async, takes AppWebsocket, calls `conductor_writes::call_update_rea_commitment_state`, then polls SQL for state change.

- [ ] **Step 5: Thread through PATCH handler in http.rs**

Find handler bound to `"update_commitment_state"` (route `http.rs:9374`). Thread AppWebsocket; mark async.

- [ ] **Step 6: Extend integration test**

Add a test case to `conductor_roundtrip_test.rs`:

```rust
#[tokio::test]
async fn http_patch_commitment_state_writes_dht_update() {
    // setup: create a commitment first (Task 4 path)
    // then PATCH state=cancelled
    // assert: SQL row state == "cancelled" AND new dht_anchor_hash differs from create's
}
```

Run; expect pass.

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-storage/src/services/conductor_writes.rs \
        elohim/elohim-storage/src/services/rea_commitment_service.rs \
        elohim/elohim-storage/src/http.rs \
        elohim/elohim-storage/tests/conductor_roundtrip_test.rs \
        elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
git commit -m "feat(elohim-storage,elohim-dna): PATCH /api/v1/commitments/{id} round-trips through conductor

Mirrors the create migration. Also adds update_rea_commitment_state
coordinator zome function (was absent — UPDATEs went diesel-direct
even though entry type supports update semantics).

Closes the second half of Gap D: state transitions (e.g. cancellation
of a project-epr commitment) now propagate via DHT to all peers.

Plan: genesis/docs/superpowers/plans/2026-05-26-substrate-rea-replication-fix.md"
```

---

## Task 7: Failing test — content blobHash PATCH round-trip

**Files:**
- Modify: `elohim/elohim-storage/tests/conductor_roundtrip_test.rs` (add test case)

- [ ] **Step 1: Add the failing test**

Append to `conductor_roundtrip_test.rs`:

```rust
#[tokio::test]
async fn http_patch_content_blob_hash_writes_dht_anchored_projection() {
    let state = TestState::new().await;

    // 1. Create a content row first.
    let create_input = elohim_storage::db::content::CreateContentInput {
        id: "test-content-001".to_string(),
        content_type: "html5-app".to_string(),
        title: "Test SPA".to_string(),
        // ... fill in required fields
        blob_hash: None,
        ..Default::default()
    };
    ContentService::create(
        &mut state.conn(),
        &state.ctx,
        create_input,
        Some(&state.event_bus),
        &state.app_ws,
    )
    .await
    .expect("content create should succeed");

    // 2. PATCH the blobHash to a real-shaped value.
    let new_hash = "sha256-deadbeefcafefacef00d";
    ContentService::update_blob_hash(
        &mut state.conn(),
        &state.ctx,
        "test-content-001",
        new_hash,
        Some(&state.event_bus),
        &state.app_ws,
    )
    .await
    .expect("update_blob_hash should succeed");

    // 3. Re-fetch and assert dht_anchor_hash populated, blob_hash matches.
    let projection = state
        .await_content_projection("test-content-001", std::time::Duration::from_secs(5))
        .await
        .expect("projection should arrive");

    assert!(projection.dht_anchor_hash.is_some());
    assert_eq!(projection.blob_hash.as_deref(), Some(new_hash));
}
```

- [ ] **Step 2: Run and confirm compile failure (drives Task 8)**

```bash
cargo test --release --test conductor_roundtrip_test http_patch_content 2>&1 | tail -15
```
Expected: compile error — `ContentService::update_blob_hash` doesn't exist yet, or its signature lacks app_ws.

- [ ] **Step 3: Commit**

```bash
git add elohim/elohim-storage/tests/conductor_roundtrip_test.rs
git commit -m "test(elohim-storage): failing test for content blobHash round-trip through conductor

Drives the ContentService migration in the next commit. Asserts that
PATCH /db/content/{id} with a new blobHash writes a SQL row with
non-null dht_anchor_hash and the updated blob_hash visible.

This is the regression that today leaves alpha's elohim-host-landing
row at sha256-PLACEHOLDER_REPLACED_BY_SEED_SCRIPT.

Plan: genesis/docs/superpowers/plans/2026-05-26-substrate-rea-replication-fix.md"
```

---

## Task 8: Migrate ContentService — create + update_blob_hash

**Files:**
- Modify: `elohim/elohim-storage/src/services/content_service.rs` (whole file)
- Modify: `elohim/elohim-storage/src/services/conductor_writes.rs` (add content helpers)
- Modify: HTTP handlers for `create_content` (http.rs:9404) and `update_content` (http.rs:9429)
- Possibly: add `update_content_blob_hash` coordinator zome function

- [ ] **Step 1: Inspect content_store zome surface**

```bash
grep -n "fn create_content\|fn update_content\|fn update_content_blob" \
  /projects/elohim-worktrees/qahal-m1/elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
```

Document which functions exist. Likely `create_content` exists; `update_content` may or may not. Mirror the Task 6 conditional add pattern if needed.

- [ ] **Step 2: Mirror the REA commitment migration pattern for content**

Apply the same shape: ContentService::create / update_blob_hash become async, take AppWebsocket, route through conductor_writes::call_create_content / call_update_content. HTTP handlers thread AppWebsocket from AppState.

- [ ] **Step 3: Run the test from Task 7**

Expected: pass.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/services/content_service.rs \
        elohim/elohim-storage/src/services/conductor_writes.rs \
        elohim/elohim-storage/src/http.rs \
        elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
git commit -m "feat(elohim-storage,elohim-dna): PATCH /db/content/{id} round-trips through conductor

Closes Gap C (substrate side): content blobHash updates now propagate
via DHT gossip. The stageSpaBlobs PATCH on alpha will now reach all
storage peers, not just whichever one doorway-alpha's stewardUrl
happens to point at on a given pod-restart.

Plan: genesis/docs/superpowers/plans/2026-05-26-substrate-rea-replication-fix.md"
```

---

## Task 9: Multi-peer sweettest — prove DHT gossip works

**Files:**
- Create: `elohim/holochain/tests/sweettest/tests/rea_commitment_replication.rs`

- [ ] **Step 1: Read existing sweettest scaffolding**

```bash
ls /projects/elohim-worktrees/qahal-m1/elohim/holochain/tests/sweettest/tests/
grep -l "two_agent_conductors\|exchange_peer_info\|await_consistency" \
  /projects/elohim-worktrees/qahal-m1/elohim/holochain/tests/sweettest/tests/*.rs
```
Pick the simplest existing two-conductor test as the template.

- [ ] **Step 2: Write the two-conductor test**

Create `rea_commitment_replication.rs`:

```rust
//! Two-conductor sweettest: peer A creates an REA project-epr commitment;
//! peer B's local SQL projection sees the same row after DHT consistency.
//!
//! This validates that the substrate-correct write path (Task 4-6) actually
//! propagates the entry over Holochain DHT gossip, which is the core promise
//! of the substrate-rea-replication-fix plan.

use holochain::prelude::*;
use holochain::sweettest::*;

#[tokio::test(flavor = "multi_thread")]
async fn project_epr_commitment_replicates_to_peer_b() {
    let (dna, _) = load_elohim_dna().await;
    let mut conductors = SweetConductorBatch::from_standard_config(2).await;
    let apps = conductors
        .setup_app("elohim", &[dna])
        .await
        .unwrap();
    let ((alice,), (bob,)) = apps.into_tuples();

    // Crucial — without this, the conductors are isolated.
    conductors.exchange_peer_infos().await;

    // Alice creates a project-epr commitment.
    let input = CreateReaCommitmentInput {
        id: "test-projection-A".to_string(),
        action: "project-epr".to_string(),
        provider: "doorway:test-A".to_string(),
        receiver: "epr:test-app".to_string(),
        // ...
    };
    let alice_zome = alice.zome("content_store");
    let alice_output: ReaCommitmentOutput = conductors[0]
        .call(&alice_zome, "create_rea_commitment", input.clone())
        .await;

    // Await DHT consistency between the two conductors.
    await_consistency(60, [&alice, &bob]).await.unwrap();

    // Bob reads via zome (the projection-handler-on-receive path is the
    // storage layer's responsibility, not the DNA's — for sweettest, we
    // verify at the DNA level: bob can `get_rea_commitment(id)` and see it).
    let bob_zome = bob.zome("content_store");
    let bob_view: Option<ReaCommitmentOutput> = conductors[1]
        .call(&bob_zome, "get_rea_commitment", input.id.clone())
        .await;

    assert!(bob_view.is_some(), "bob's DHT view must contain alice's commitment");
    assert_eq!(bob_view.unwrap().commitment.id, input.id);
}
```

- [ ] **Step 3: Run sweettest**

```bash
cd /projects/elohim-worktrees/qahal-m1/elohim/holochain/tests/sweettest
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__holochain__tests__sweettest/dev \
RUSTFLAGS="" \
cargo test --release rea_commitment_replication 2>&1 | tail -30
```
Expected: pass. (Per memory `feedback_sweettest_cross_agent_consistency`: needs `exchange_peer_infos` + `await_consistency`.)

- [ ] **Step 4: Commit**

```bash
git add elohim/holochain/tests/sweettest/tests/rea_commitment_replication.rs
git commit -m "test(sweettest): two-conductor test proving REA commitment DHT replication

Validates the core substrate promise of the rea-replication-fix plan:
project-epr commitments created on peer A reach peer B via Holochain
DHT gossip. This is the regression seatbelt against future split-peer
divergence on production clusters like alpha.

Plan: genesis/docs/superpowers/plans/2026-05-26-substrate-rea-replication-fix.md"
```

---

## Task 10: Deploy verification on alpha

**Files:** None modified — operational verification only.

- [ ] **Step 1: Push to dev**

```bash
cd /projects/elohim-worktrees/qahal-m1
git push origin dev
```

- [ ] **Step 2: Wait for orchestrator + downstream pipelines**

Use ci-observer (or pipeline-diagnostics) to monitor the orchestrator + DNA + App + Genesis pipelines. Expected: green across all four.

- [ ] **Step 3: Probe alpha**

```bash
# Project-epr commitments should now exist on all peers, with dht_anchor_hash
curl -s 'https://alpha.elohim.host/api/v1/commitments?action=project-epr' | jq '[.[] | {id, dhtAnchorHash}]'

# Expected: ≥6 rows (landing/lamad/imagodei × 2 doorways), all with non-null dhtAnchorHash.

# Content blobHash should be real, not placeholder
curl -sI https://alpha.elohim.host/apps/elohim-host-landing/index.html | grep -i x-content-address

# Expected: x-content-address: sha256-<real hex>, NOT PLACEHOLDER_REPLACED_BY_SEED_SCRIPT.

# /lamad should serve
curl -sI https://alpha.elohim.host/lamad

# Expected: HTTP/2 200, content-type: text/html.

# / should serve with chunks resolving (sample a chunk URL from the HTML)
curl -s https://alpha.elohim.host/ | grep -o 'chunk-[A-Z0-9]*\.js' | head -1
# then curl that chunk:
curl -sI "https://alpha.elohim.host/<chunk-from-above>"

# Expected: 200, content-type: application/javascript.
```

- [ ] **Step 4: Doorway pod robustness check**

The fix removes the dependency on which storage peer doorway happens to read from. Verify by manually `kubectl delete pod` on doorway-alpha (operator does this, agent reports via Jenkins MCP) and re-probing `/lamad` immediately. Expected: still 200, regardless of which storage peer the new pod boots reading from.

- [ ] **Step 5: A2o scenario (regression seatbelt)**

Add a step definition for the @wip scenario in `genesis/a2o/features/delivery/spa-bundle-delivery.feature`:

> SPA routes fall back to index.html so Angular handles routing

Specifically the one asserting `/lamad → 200 + Angular shell markers`. Mark as no longer @wip.

```bash
git add genesis/a2o/features/delivery/spa-bundle-delivery.feature \
        genesis/a2o/src/step_definitions/lamad-spa-load.steps.ts
git commit -m "test(a2o): regression seatbelt — /lamad serves Angular shell on alpha

Removes @wip from the scenario now that the substrate-correct write
path is in place. Future split-peer or replication regressions will
trip this scenario before they trip a human visitor.

Plan: genesis/docs/superpowers/plans/2026-05-26-substrate-rea-replication-fix.md"
git push origin dev
```

- [ ] **Step 6: Update manifest + close the FeaturePromise**

The FeaturePromise at `.claude/deliver/feature-promise-epr-app-delivery.json` becomes deliverable once Step 3 + Step 5 pass. Run `/deliver epr-app-delivery` iter-N to mint the manifest verdict.

---

## Self-Review

### Spec coverage
- ✅ project-epr commitments — Tasks 4-6
- ✅ content blobHash — Tasks 7-8
- ✅ Multi-peer proof — Task 9
- ✅ Alpha verification — Task 10
- ✅ The CLAUDE.md gospel-tier prescription ("Never write to storage directly for notarized types — migrate toward conductor-first") — Tasks 2, 4, 6, 8 collectively implement this
- ⚠️ Broader Category-A sweep (relationships, knowledge_maps, agreements, economic_events) — out of scope, documented in plan header

### Placeholder scan
- No "TBD" / "implement later"
- One conditional sub-task (Task 6 Step 2, Task 8 Step 1) that may or may not need to add a coordinator zome function, depending on what's already there — guarded by an explicit grep + read in Step 1 of each, with both branches (add vs skip) spelled out.

### Type consistency
- `AppWebsocket` is consistently `holochain_client::AppWebsocket` in all signatures
- `CreateReaCommitmentInput` is consistently `elohim_storage::db::rea_commitments::CreateReaCommitmentInput`
- Service methods named `create` / `update_state` / `update_blob_hash` consistently across plan body

### Risk register
- Task 5 (AppWebsocket in AppState) is the highest-uncertainty step — if AppState construction is itself async-from-non-async context, may need a runtime restructure. Mitigated by reading `signing.rs` first (already-working precedent).
- Task 6 Step 2 + Task 8 (adding zome functions) require DNA rebuild + integrity changes. Verify integrity zome doesn't reject UPDATE on Commitment entries before merging.
- The 500ms-poll wait in `ReaCommitmentService::create` is a substitute for proper post-commit handshake. If sweettest reveals it's racy, switch to subscribing to `EventBus::ProjectionRegistered` directly inside `create` and awaiting the matching commitment_id.
- Task 10 Step 4 (pod restart robustness) is the hardest verification — the operator needs to actually trigger the pod swap. Document the expected behavior in advance so the operator can run the check independently.

---

*Plan written 2026-05-26 by Opus 4.7 (1M context) under Matthew's direction. Based on the systematic-debugging Phase 1 + Phase 2 investigation in this session (2026-05-26 morning), the 2026-05-23 spa-blob-deploy-drift plan + dev response, the 2026-05-25 pillar-epr-decomposition plan B11-B15, last night's iter-0 + shift sprint-results, and the p2p-design-gate skill output. The architecture matches the gospel-tier prescription in elohim/holochain/dna/CLAUDE.md.*

---

## Task 10 close-out (2026-05-27) — HALTED on three independent regressions

**Status:** Tasks 1–9 landed and reached alpha as image `1.0.0-dev-c014e896`. Task 10 verification was attempted on orchestrator `elohim-orchestrator/dev #1069` and **halted before probes could pass**. Initial agent diagnosis attributed the halt to cluster-state alone; operator's cluster-side `kubectl describe` / `kubectl logs` pass surfaced three independent regressions — one of them in substrate-rea-adjacent code.

### Three independent regressions

1. **Inventory verifier rejects the canonical wire format.** Every healthy alpha peer is spamming `WARN elohim_storage::inventory: Inventory snapshot failed structural verify — dropped … error=InvalidHashFormat("sha256-1f3ed518a975f0eb55ae72c7cca8ef396c8f73c61ecf730ad54920ea0a24a955")`. The verifier `is_blob_hash_shaped()` at `elohim/elohim-storage/src/p2p/inventory_gossip.rs:132–134` requires exactly 64 lowercase hex chars; the canonical wire format per `elohim-storage/CLAUDE.md` is `sha256-<64-hex>` (71 chars). The verifier landed 2026-05-02 (T13, commit `9169ab99d`) and its tests used `"a".repeat(64)` instead of real-shape strings; the mismatch went latent until substrate-rea exercised inventory gossip end-to-end on a real multi-peer cluster. Fix is small: relax `is_blob_hash_shaped` to accept the `sha256-` prefix.

2. **Genesis seeder polls a workload that doesn't exist.** `elohim-timothy-tutor-alpha` has no Pod/StatefulSet/Deployment/Job anywhere in any namespace — only Services `elohim-timothy-alpha` / `-headless` exist. The "0 completed, 0 pending throughout the 600s window" symptom is what you'd see polling a name with no backing pod. Either the seeder manifest didn't render at all this run, or the polled name (`...-tutor-...`) doesn't match the deployed name (`elohim-timothy-alpha`). Worth a `helm get manifest <release> -n elohim-alpha | grep -i timothy` to compare.

3. **9/14 storage peers fail readiness under CPU contention.** Initial report said "rollout timed out"; the operator's cluster-side check showed all 14 pods are scheduled + Running, but 9 are stuck Ready=0/1 because the embedded conductor's `install_app` is timing out on the admin websocket handshake. Daniel's previous-instance log ends with `Error: install_app failed: Websocket error: Timeout`; pete's readiness probe times out 52× in 16 min while install_app is still running. Root cause: shem at 48% CPU carrying 13 of 14 alpha pods + a doorway replica; each stuck conductor burns 700–1000m spinning up. The 5 healthy peers either landed on `ethosengine` or got onto shem early enough to complete install_app before contention spiked. Fix is operational: anti-affinity or topology-spread on `kubernetes.io/hostname` before re-rolling.

**Separately** — and unrelated to substrate-rea — `elohim-site-alpha-service` has no backing pods. The ingress `elohim-site-alpha-ingress` exists, but no `elohim-site-alpha-*` Deployment was ever applied in this run. So `/apps/elohim-host-landing/index.html`, `/lamad`, and `/` will 404 even after the three substrate regressions clear. That frontend was never deployed; re-running Genesis won't fix it.

### CI-visible facts (unchanged)

**Pipeline outcomes:**
- DNA-Lamad (elohim-holochain/dev #1292): SUCCESS
- Edge (elohim-edge/dev #1012): UNSTABLE — storage image built + pushed, doorway-alpha rolled out clean, but `kubectl rollout status --timeout=600s` reported 9/14 storage peer StatefulSets not Ready inside the window (pete, frank, gertrude, susan, caleb, daniel, emma, eve, nancy). 5 peers succeeded (adam, matthew, jessica, james, terrance). The CI log called this a "rollout timeout"; the cluster-side ground truth (operator's pass) is that all pods are scheduled + Running but 9 fail readiness because embedded conductor `install_app` is timing out under CPU contention on `shem`.
- Genesis (elohim-genesis/dev #1045): FAILED — `Seed Database` cascaded all 14 downstream seed stages. The CI log reported `❌ human-timothy-tutor replication timed out` after polling `0 completed, 0 pending` for the full window. The cluster-side ground truth is that no `elohim-timothy-tutor-alpha` workload exists; the seeder is polling a name that doesn't match any deployed StatefulSet.
- App (root Jenkinsfile): NOT DISPATCHED — the substrate-rea changeset touched only `elohim-storage/*` + `holochain/*` paths.

**Probe outcomes (Step 3):** all four FAILED — `[]` from `/api/v1/commitments?action=project-epr`, 404 on `/apps/elohim-host-landing/index.html`, 404 on `/lamad`, 302 redirect on `/` to a 404 destination. The `[]` is downstream of regression #1 (verifier dropping every inventory snapshot) and would persist even if the seeder ran. The 404s are downstream of the separate `elohim-site-alpha-service`-has-no-backing-pods miss noted above; re-running Genesis won't clear them.

**Step 4 (pod-restart robustness):** deferred — not attempted because Step 3 never passed.

**@wip removal:** reverted before commit. Step-def scaffolding for both scenarios (`genesis/a2o/steps/delivery/substrate-rea-replication.steps.ts` + the `fetchApp` / `responseStore` export from `genesis/a2o/steps/delivery.steps.ts`) is complete, dry-run-clean, lint-clean, format-clean — and left **uncommitted** in the working tree for the operator to bundle with the `@wip` removal once the cluster heals.

**Operator hand-off (revised after operator's cluster pass):**
1. **Verify the seeder manifest naming** — `helm get manifest <release> -n elohim-alpha | grep -i timothy`. Reconcile whichever side is wrong: either the seeder polls the wrong name (`...-tutor-...` vs `...-alpha`) or the manifest never rendered the `tutor` variant.
2. **Fix the `InvalidHashFormat` verifier** at `elohim/elohim-storage/src/p2p/inventory_gossip.rs:132–134` — relax `is_blob_hash_shaped` to accept the canonical `sha256-<64-hex>` wire format (strip the `sha256-` prefix before the length check; update the existing tests at lines 140–225 to use real-shape strings instead of `"a".repeat(64)`).
3. **Reschedule storage peers off `shem`** — anti-affinity by household or topology-spread on `kubernetes.io/hostname` before the next `kubectl rollout restart`. Otherwise install_app will time out again for the same conductor-spin-up reason.
4. **Re-trigger Genesis** (after 1–3 land) — no Edge rerun needed; the substrate-rea image is already on the cluster. The seeder is what exercises the substrate-rea write path end-to-end.
5. **Address the missing `elohim-site-alpha` Deployment separately** — the ingress exists but the service has no backing pods, so the landing/`/lamad` 404s won't clear via Genesis. This is a deploy gap unrelated to substrate-rea.
6. **Re-probe alpha**; if `dhtAnchorHash` is non-null on every project-epr row and `/lamad` returns 200 (assuming #5 is also addressed), the fix is confirmed.
7. **Then drop `@wip`** from the two scenarios + land the scaffolded step defs as `test(a2o): regression seatbelt — substrate-rea replication on alpha`.
8. **Run `/deliver epr-app-delivery`** to mint the FeaturePromise verdict.

**Sprint result:** [genesis/docs/superpowers/sprints/2026-05-27-substrate-rea-task10-result.md](../sprints/2026-05-27-substrate-rea-task10-result.md) carries the full pipeline log quotes and probe outputs.

*Task 10 close-out written 2026-05-27 by Opus 4.7 under Matthew's direction during the shift dispatched against orchestrator #1069.*

---

## Verifier-fix close-out — Sprint 0 of REA-compute-substrate roadmap (2026-05-28)

The substrate-rea sprint's Task 10 close-out captured the symptom: `WARN elohim_storage::inventory: Inventory snapshot failed structural verify — dropped … error=InvalidHashFormat("sha256-…")` on every healthy cluster peer. Root cause was a wire-format mismatch in `elohim/elohim-storage/src/p2p/inventory_gossip.rs` `is_blob_hash_shaped`: the predicate required bare 64-hex while every producer in the crate emitted canonical `sha256-<64-hex>` per `elohim-storage/CLAUDE.md`. Latent ~26 days, exercised end-to-end by substrate-rea's multi-peer alpha cluster.

**Fix landed in six commits on `sprint/cross-pillar-cleanup`:**

| SHA | Layer | What |
|-----|-------|------|
| `c5d6dd827` | tests | Fixtures use canonical wire shape (RED) |
| `d21e0bc0c` | predicate | Verifier accepts `sha256-<hex>` (GREEN) |
| `680a5c0f2` | type system | `BlobAddress` newtype constructor-validates wire shape |
| `a04c07982` | error type | `VerifyError` uses `thiserror::Error` (gets `std::error::Error` for free) |
| `702b0cb05` | producer→consumer | Thread `BlobAddress` through `BlobInventorySnapshot.hashes`, `BlobInventoryDelta.{added,removed}`, `LocalInventory`, `StaticInventory`, `StoreAdapter`, p2p/mod.rs receive arm, 4 integration test files |
| `6f66ffeb5` | regression seatbelt | Producer-↔-verifier round-trip from real `BlobStore` — makes future drift impossible to land green |

**Bug-class closed at two layers:** (1) the predicate now matches the producer's output (immediate fix), and (2) the `BlobAddress` newtype with `#[serde(try_from = "String")]` makes producer-↔-verifier drift unrepresentable at the type level (Stage-2 hardening — survives all subsequent architectural graduations of the REA-compute-substrate roadmap).

**Cluster-verification deferred:** the push-and-re-probe step (Task 0.7 of the roadmap) is rolled up with other work in flight. When the roll-up lands on `dev`, `project_generous_probes_pattern`'s floor-restoration gate becomes unblocked.

**Sibling-bug audit:** `IdentityBindingGossip::verify_structural` has no hash-shape predicate — only non-empty checks on `agent_cid`, `valid_from`, `binding_action_hash`, `signature`. No sibling bugs found in the crate.

**Sprint 0 closes Sprint 0 of the larger roadmap.** See `genesis/docs/superpowers/plans/2026-05-28-rea-compute-substrate-native-roadmap.md` for Sprints 1-8 covering Z.D substrate-correct deploy, bounds validator + standing aggregator, hosting graduation, and the five other rows of the REA compute-commitment gospel-tier generalization table.
