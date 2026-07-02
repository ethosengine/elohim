---
title: Genesis seed/verify stabilization (post-leak-fix) — green the 5 Unstable stages + light the resilience card
id: genesis-seed-stabilization-postleakfix-plan
status: Draft
class: substrate
sprint: composes 2026-06-10-epr-durability-replication-arc-plan (Workstreams A/D/E) — refines it with verified root causes now the conductor leak fix exists
domain: D-substrate-distribution
cites:
  - genesis-seed-stages-unstable-resilience-card-rca | RCA: Genesis seeding/verify stages Unstable + all-zeros resilience card | sha256:a93c5b647f8a9530 | path: genesis/docs/content/elohim-protocol/history/2026-06-18-genesis-seed-stages-unstable-resilience-card-rca.md
  - epr-durability-replication-arc-plan | EPR Content Durability Arc | sha256:f263ed845af2f916 | path: genesis/docs/superpowers/plans/2026-06-10-epr-durability-replication-arc-plan.md
  - coherent-transport-identity-resolver-design | Coherent transport-identity resolver | sha256:63117b359cfa3891 | path: genesis/docs/superpowers/specs/2026-06-15-coherent-transport-identity-resolver-design.md
  - elohim/elohim-storage/src/p2p/acquisition.rs
  - elohim/elohim-storage/src/p2p/reconcile_rails.rs
  - elohim/elohim-storage/src/p2p/blob_fetch.rs
  - elohim/elohim-storage/src/p2p/mod.rs
  - elohim/elohim-storage/src/services/household_resilience.rs
  - genesis/scripts/ci/substrate-verify.sh
  - genesis/scripts/ci/verify-doorway-readiness.sh
informed-by:
  - genesis/a2o/features/federation/
---

# Genesis seed/verify stabilization (post-leak-fix) Implementation Plan

> ## ⚠ CURE COMMIT CORRECTED — 2026-06-19 (Tasks 1–2 leak-independent repo bugs STAND)
> The operator-runbook step 1 names `2af2607e7` ("embed the patched conductor by default") as "the conductor
> leak fix … already on origin/dev." That commit is the tx5 #194/#199 connection-patch conductor, which was
> deployed fleet-wide and DID NOT cure the leak. The actual cure was glibc→jemalloc (temp-prof `b8481f090`;
> prod che-dw build #13 / Part C `ed111a5cc`). The plan's "leak fix deployed → healthy mesh" precondition IS
> now met — by jemalloc, not by `2af2607e7`. Tasks 1 (acquisition rollup) and 2 (quilt-draw serve-blob event)
> are leak-INDEPENDENT repo bugs and stand unchanged.
> Truth: genesis/docs/content/elohim-protocol/history/2026-06-19-conductor-leak-jemalloc-cure-verdict.md · genesis/docs/content/elohim-protocol/history/2026-06-19-conductor-leak-jemalloc-prod-changeset.md


> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development or superpowers:executing-plans, task-by-task. The **p2p-design-gate skill is MANDATORY** before implementing Task 2 (a new economic-event emission point) and Task 3-as-design (a new session-write path / identity surface). Story-first: land the a2o scenario with the implementation, same commit.

**Goal:** Drive the five genesis stages (Seed Substrate, Seed Custody Commitments, Seed REA Commitments, Verify Delivery Events, Verify Projection Sync) from Unstable→Success and light the `elohim-host-landing` resilience card, by fixing the genuinely leak-independent repo bugs now and handing the operator a precise runbook for the deploy-gated remainder.

**Architecture:** Root-cause triage (see the cited RCA) split the 8 causes into three buckets: **leak-dependent** (resolve on deploying the conductor fix + redeploy), **genuine repo bugs** (fixable now, leak-independent), and **deploy/cluster-state** (operator). This plan implements ONLY the genuine repo bugs that are unit-testable on `household-nodes`, captures the security-sensitive identity gap as a design task, and documents the keystone operator action.

**Tech Stack:** Rust (elohim-storage, libp2p data plane), bash CI verify suite, Holochain conductor (embedded child of the storage image).

## Global Constraints (verbatim, every task inherits)

- elohim-storage builds keep ambient `RUSTFLAGS=--cfg getrandom_backend="custom"`; set `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/frontend/elohim__elohim-storage/dev` (fall back to a `/tmp` target dir on a fingerprint ENOENT). Plain `cargo test` (NO nextest in this container).
- **Commit-only; the integrator pushes/merges/deploys.** Autonomous mode never `git push`, never `kubectl`. The repo manifests are the cleanup surface.
- **Do NOT relax any verify assertion to force green** — `serve-blob=0` and `pull=false` are real conditions to fix or explain, never gates to loosen.
- genesis/Jenkinsfile CPS method is at ~63.8KB of a 65KB cap — NO new inline heredocs; bash bodies live in `genesis/scripts/ci/*.sh`.
- Shared worktree — selective staging; never bulk-revert ambient mods.

## Honest framing (set with the operator — do not over-promise)

- **None of the 5 stages are verifiable-green by this session.** They seed/verify the LIVE alpha cluster; `feat/*` is not orchestrator-indexed and I cannot trigger CI or run kubectl. CI for this work is the operator dev-merge + redeploy.
- **The resilience card was never lit on real alpha** — it is unfinished-feature, not a regression. BOTH columns are multi-step gated:
  - `commitment-backed`: needs `humans.agent_pub_key` populated (Task 3 — security-gated design) AND `content:<reach>` provide rows landing (blocked today by the catching-up shed = leak).
  - `stewarding`: needs `shard_locations`, written ONLY by runtime `distribute_shards` over a healthy multi-peer mesh (no REST seed path; cause #6). Needs the leak fix deployed so the mesh isn't collapsed to 1 peer.
- So the card is a **multi-step arc**: (leak fix deployed → healthy mesh) + Task 3 identity design + Epic-B committed-accounting. This plan moves it forward; it does not finish it in one pass.

---

### Task 1: Fix the acquisition `pull` rollup so already-local pins report caught-up (cause #4, Verify Projection Sync)

**Why:** `rollup()` counts every wanted item in `total` but only byte-arrivals (`mark_completed`) ever increment `fetched`. A node that already holds its pinned content (matthew holds the seeded corpus) has wants that are skipped by `enqueue_missing` (already-local) — counted in `total`, never in `fetched` → `caught_up = total>0 && fetched==total` is false forever. The sibling `per_epr` rollup already excludes already-local items, so the two surfaces disagree on the identical state (the defect tell). An already-local want's bytes ARE present locally — counting it satisfied does NOT violate the "never false-complete" (R-A) invariant, which guards against claiming complete before bytes *arrive*.

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/reconcile_rails.rs` (add `GapTracker::mark_local_wants_satisfied`)
- Modify: `elohim/elohim-storage/src/p2p/acquisition.rs:88-106` (call it in `reconcile`; update existing test at ~277-294)
- Test: `elohim/elohim-storage/src/p2p/acquisition.rs` (`#[cfg(test)]`)

**Interfaces:**
- Produces: `GapTracker::mark_local_wants_satisfied(&mut self, want_ids: &[String])` — for each want id already in `self.local_ids`, insert it into `self.completed` (idempotent; does not touch `pending`/`failed`). Distinct from `mark_completed` (which is the byte-arrival done-signal).

- [ ] **Step 1: Write the failing test** (add to `acquisition.rs` tests, beside `reconcile_diffs_wants_and_rolls_up`)

```rust
#[tokio::test]
async fn all_pinned_items_already_local_is_caught_up() {
    let acq = AcquisitionState::new();
    let local: std::collections::HashSet<String> = ["have-1".into()].into_iter().collect();
    // a single-item pin whose content is already present locally (the matthew case)
    acq.reconcile(vec![(1, vec!["have-1".into()])], &local).await;
    let r = acq.rollup().await;
    assert_eq!((r.total, r.fetched, r.pending), (1, 1, 0));
    assert!(r.caught_up, "a pin whose every item is already local must be caught_up");
}
```

- [ ] **Step 2: Run it, verify it FAILS**

Run: `cd elohim/elohim-storage && cargo test --lib p2p::acquisition::tests::all_pinned_items_already_local_is_caught_up -- --nocapture`
Expected: FAIL — today `r.fetched == 0`, `r.caught_up == false`.

- [ ] **Step 3: Add the GapTracker helper** in `reconcile_rails.rs` (after `mark_completed`, ~line 102)

```rust
/// Acquisition-only: count wanted ids already present locally as completed,
/// so the rollup's `total == fetched` holds for content the node already
/// holds. The want is *satisfied* (bytes are local), not pending — this is
/// NOT a false-complete (R-A guards against claiming done before bytes
/// ARRIVE; here they are already present). Idempotent; leaves `failed`
/// untouched. Distinct from `mark_completed` (byte-arrival, cross-stream).
pub fn mark_local_wants_satisfied(&mut self, want_ids: &[String]) {
    for id in want_ids {
        if self.local_ids.contains(id) {
            self.completed.insert(id.clone());
        }
    }
}
```

- [ ] **Step 4: Call it in `acquisition.rs::reconcile`** (the per-pin loop, ~line 96-105). `want_ids` is moved into `reconcile_desired`, so clone the small item-pin set first (the loop already clones `local_has` per pin per the existing TODO):

```rust
let tracker = inner
    .trackers
    .entry(pin_id)
    .or_insert_with(|| GapTracker::new(MAX_RETRIES));
tracker.set_local_ids(local_has.clone());
let want_ids_for_local = want_ids.clone();
let gaps = tracker.reconcile_desired(want_ids);
tracker.mark_local_wants_satisfied(&want_ids_for_local);
to_dispatch.extend(gaps);
```

- [ ] **Step 5: Update the existing rollup test** `reconcile_diffs_wants_and_rolls_up` (~line 292): the already-local item now counts as fetched, so the assertion `(total, fetched, pending) == (3, 0, 2)` becomes `(3, 1, 2)`. Update it and its comment.

- [ ] **Step 6: Run the acquisition test module, verify all PASS**

Run: `cd elohim/elohim-storage && cargo test --lib p2p::acquisition -- --nocapture`
Expected: PASS (new test + updated `reconcile_diffs_wants_and_rolls_up`).

- [ ] **Step 7: Add an a2o regression note + commit** (story-first). Append the constraint to the durability-arc Workstream E scenario coverage (projection caught-up across the fleet includes content-holder nodes). Then:

```bash
git add elohim/elohim-storage/src/p2p/reconcile_rails.rs elohim/elohim-storage/src/p2p/acquisition.rs
git commit -m "fix(storage): acquisition rollup counts already-local pins as caught-up

A content-holder node's pinned items are skipped by enqueue_missing
(already local) — counted in total, never in fetched — so pull.caughtUp
was false forever, failing Verify Projection Sync. An already-local want
is satisfied (bytes present), not a false-complete. Aligns rollup() with
per_epr (which already excludes already-local).

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 2: Emit a delivery economic-event on the proactive replication (quilt-draw) path (cause #3, Verify Delivery Events)

**Why:** `serve-blob` REA EconomicEvents are written ONLY by on-demand pulls (`blob_fetch.rs:218-254 finalize_fetch_success`). The proactive quilt-draw path that actually moves bytes on alpha (`p2p/mod.rs:3756-3777`) calls bare `blob_store.store(&data)` and books no event — so Verify Delivery Events sees 0 events even though Verify Substrate Propagation passes (bytes on disk). Leak-independent (`blob_fetch.rs` byte-identical dev↔branch).

**⚠ BLOCKING design gate — resolve BEFORE implementing (p2p-design-gate):** A proactive draw is not an on-demand *serve*. The gospel notes `peer_blob_inventory` projects two distinct observation kinds: `infrastructure:blob-served` AND `infrastructure:blob-hosted`. Decide deliberately: does the quilt-draw responder book `serve-blob` (treating the responder as provider) or a distinct `blob-hosted`/`replicates-content` action — and does `substrate-verify.sh` `cmd_delivery` (currently `action=serve-blob` only, line 418) widen to the full delivery vocabulary? Reconcile with the REA-action-taxonomy owner; do NOT silently force `serve-blob` onto the proactive path (the verify and the emitter will re-diverge). **Record the decision in this task before writing code.**

**✅ GATE RESOLVED (2026-06-18, advisor-confirmed) — book `serve-blob`; do NOT widen the verify; reuse `finalize_fetch_success`.** Self-answered from primary-source evidence (per `feedback_skip_brainstorm_gates_self_answer`):
- **A serve *did* occur.** The proactive draw is physically and economically identical to the on-demand race-fetch — a remote peer served us bytes in response to our `ShardRequest::Get`; we received them. The REA direction is the same (provider = serving peer, receiver = `self_cid`). On-demand-vs-proactive is a *trigger* difference, not a transfer-direction or attribution difference, and the EconomicEvent records the transfer, not its trigger.
- **`blob-hosted` is the wrong layer and has zero consumers.** In the observation taxonomy, `blob-served`/`blob-hosted` live in the `peer_blob_inventory`/observation plane (the *hosting* fact = the propagation surface, already green via bytes-on-disk + the inventory row this fix also writes). The *delivery* surface wants the transfer trail, which is a serve. A `blob-hosted` REA action would duplicate propagation **and** has no reader — `rea_projection` ignores `serve-blob`/`custody-blob` (`rea_projection.rs:555`), the resilience card's stewarding comes from `shard_locations` not `economic_events`, so the ONLY consumer of these events is `substrate-verify.sh`'s delivery count.
- **No verify-script change** keeps emitter + verify on one vocabulary (kills the re-divergence risk) and honors "don't relax assertions" — the proactive path now emits `serve-blob`, so the unchanged `action=serve-blob` query passes honestly.
- **Why the gospel economic-attribution gate does NOT apply here:** `finalize_fetch_success` sets `dht_anchor_hash: None` — these are **local SQLite accounting rows, not DHT-notarized claims**, and `rea_projection` drops them. The "don't consume self-asserted agent↔transport bindings for economic attribution" concern (load-bearing for Task 3's *commitment-backed* column) does not reach `serve-blob`. The author's original hedge imported Task-3 caution into a place it doesn't go.
- **Implementation refinement (supersedes the `finalize_quilt_draw`-that-mirrors-`finalize_fetch_success` suggestion below):** a thin `finalize_quilt_draw` wrapper that **verifies the pulled bytes then DELEGATES to `finalize_fetch_success`** — NOT a forked copy. Reuse preserves the T18 parity contract (inventory row ⟺ serve-blob event, atomic) and avoids drift. The verify guard is essential and new: the quilt-draw arm (unlike race-fetch) does not pre-verify, so a mismatched reply would otherwise write an inventory row + event claiming we host `blob_hash` while `BlobStore::store` filed the bytes under their real (different) hash.
- **`provider` namespace:** the quilt-draw passes `peer.to_string()` (the libp2p transport id) — the same namespace the on-demand path already records (`peer_blob_inventory.peer_id` / race-fetch `source_peer` is matched against the libp2p connected set), so the two paths are consistent. No cross-namespace join (these rows are never joined on `provider`).
- **⚠ Live-greening caveat (operator framing, non-blocking):** the trigger is guarded by `!self.blob_store.exists(hash)` — a draw only fires when the node does NOT already hold the blob. On a post-deploy re-run where peers already hold the corpus, **no draw fires → no event → Verify Delivery Events can still read 0 *with the fix in place*.** That is "no transfer happened," not a regression. The unit test (`quilt_draw_books_serve_blob_event`) is the real correctness proof; live greening is conditional on an actual draw during the build window.

**Files (after the gate resolves):**
- Modify: `elohim/elohim-storage/src/p2p/blob_fetch.rs` (extract a `finalize_quilt_draw` helper mirroring `finalize_fetch_success`, with the chosen action)
- Modify: `elohim/elohim-storage/src/p2p/mod.rs:3756-3777` (route the `ShardResponse::Data` quilt-draw arm through the helper instead of bare `store()`)
- Modify (if the gate widens it): `genesis/scripts/ci/substrate-verify.sh:418` (delivery query vocabulary)
- Test: `elohim/elohim-storage/src/p2p/blob_fetch.rs` `#[cfg(test)]` (beside `finalize_persists_bytes_then_writes_sql`)

- [ ] **Step 1: Record the p2p-design-gate decision** (action string + whether the verify query widens) as a comment block at the head of the helper and in this task.
- [ ] **Step 2: Write the failing test** — drive a simulated `ShardResponse::Data(bytes)` through `finalize_quilt_draw` against `test_pool()` + `BlobStore::new_memory()`; assert `economic_events` has exactly one row with the chosen `action` and `resource_inventoried_as == <hash>`. (RED today: the production arm calls raw `store()`, writes zero rows.)
- [ ] **Step 3: Run it, verify it FAILS.** Run: `cd elohim/elohim-storage && cargo test --lib p2p::blob_fetch -- --nocapture`
- [ ] **Step 4: Implement** the `finalize_quilt_draw` helper (persist + `record_fetch_success` + the economic event in one transaction) and call it from the `mod.rs:3756-3777` arm (`self.db_pool`, `self.config.self_cid`, and `peer` are in scope in `handle_behaviour_event`).
- [ ] **Step 5: Run blob_fetch tests + verify they PASS.**
- [ ] **Step 6: Story + commit** (extend the durability-arc delivery scenario to assert delivery events ≥1 after a content-replication-driven draw).

> Sequencing note (per advisor): Task 1 is the cleaner, lower-risk landing and should go first; Task 2 lands only after its design gate is resolved. Both are leak-independent and unit-testable on `household-nodes`.

---

### Task 3 (CAPTURE AS DESIGN — do NOT implement this session): populate `humans.agent_pub_key` without a self-asserted session (cause #5)

**Why captured, not built:** `humans.agent_pub_key` is never populated — the only real-key writer (`heal`, `api/identity.rs:149`) is gated on `/auth/me`, which needs a `LocalSession` that nothing creates on a server pod (no `POST /session` in seeding, no boot self-session, no portal handoff; `on_membership_projected` writes only `household_id`). So the commitment-backed column stays dark regardless of conductor health.

**But the obvious fix is security-sensitive, not a repo bug.** A boot-time `LocalSession` minted from the conductor cell key asserts "this pod IS human-X" **bypassing** the TOFU/portal-handoff trust check `session/exchange` implements. The storage gospel is explicit: the agent↔transport binding is self-asserted/unsigned today and must NOT be consumed for **economic attribution** until a cross-signed control proof lands (open security item). The commitment-backed column *is* economic attribution. Lighting it via a self-asserted key is plausibly the exact thing that gate forbids. This connects to the **blocked** `2026-06-15-coherent-transport-identity-resolver-design.md`.

- [x] **Action (done this session):** this concern ALREADY has a home —
  `genesis/data/timeline/backlog/resilience-card-self-cid-provide-loop-gate.md`. Do NOT open a
  duplicate. A dated **2026-06-18 reconciliation note** was appended there flagging the
  contradiction (this triage's "`agent_pub_key` never populated → deploy insufficient" vs that
  doc's "deploy+reseed suffices") and the single discriminating operator test (probe `/auth/me`
  on a healthy reseeded pod: 200+key → leak-dependent/that doc right; 401 → structural-gap/this
  triage right). It carries the p2p-design-gate (where the key lives, who may assert it,
  one-human-per-pod, cross-signed proof) and is operator/security-owned. Do not implement until
  the discriminating probe resolves it AND the cross-signed binding lands.

---

## Operator runbook (keystone — not code; the highest-leverage action)

These are the leak-dependent + deploy/cluster-state causes (#1 CellDisabled, #2 catching-up shed, #6 mesh-degraded stewarding, #7 fix landing, #8 apex 502). I commit; the integrator deploys.

1. **Land the conductor leak fix — ✅ ALREADY ON `origin/dev` (git-verified 2026-06-18, later session).** `origin/dev` carries `2af2607e7` ("embed the patched conductor by default (fleet-wide)"), **byte-identical to the branch's `7747f3ec8`** (same `git patch-id` `6789a9f2cd1e289f…`), atop the canary plumbing (`5a73e400d`/`b33ff524a`). The "diverged dev refs" framing was stale: local `dev` (`ebbe201f7`) is **fast-forward-behind** `origin/dev` by 260 commits (`git merge-base --is-ancestor ebbe201f7 origin/dev` ⇒ yes), **not diverged**. So the storage Dockerfile default already points at patched `harbor.ethosengine.com/ethosengine/elohim-edgenode:latest` on dev. **Nothing to land here** — proceed to step 2. (The separate repo Fix #4 `7afa03337` is still branch-only and is the one outstanding dev merge; it is leak-independent and does not gate the redeploy.)
2. **Verify the patched image exists.** Confirm the che-devworkspaces `elohim-edgenode` job built+pushed the PATCHED harbor `:latest` — recipe gates still open: go-pion feature set vs holo-host's recipe, and kitsune2 wire-compat (our binary links 0.3.2; live mesh runs 0.3.0-dev.3).
3. **Canary first** (per the deploy recipe): deploy the patched conductor to ONE non-genesis leecher's `elohim-node` container, watch `elohim_node_conductor_smaps_anon_bytes{class="other"}` flatten over a multi-hour window, confirm it stays in-mesh. Roll wider on flatten+healthy; the genesis pair (matthew/adam) LAST. Image rebuild + rolling restart only — NOT `ALLOW_DNA_REINSTALL` (binary swap, DNA hash unchanged, no re-key).
4. **Re-measure** after the patched conductor is fleet-wide: re-run the genesis pipeline. Expect Seed Conductor Identities/Household Formation to complete (no CellDisabled), the catching-up shed to clear (conductor calls stop failing → no breaker cascade), `distribute_shards` to write `shard_locations` and clear the stale 2026-06-13 placement gap. Stages 1,2,4 + Verify Projection Sync (with Task 1) should green.
5. **Apex 502:** `doorway.elohim.host` returns 502 (the elohim.host half of the card can't render). Operator-owned — repo manifests are the surface; never touch the live ingress from a dev session.

**Optional repo hardening (does NOT green a stage; reduces blast-radius / improves diagnosis):**
- Make `verify-doorway-readiness.sh` leak-aware: add a canary anchored write (or a cell-status assert) so a connected-but-CellDisabled cell converts the downstream Seed Custody/REA stages from hard-UNSTABLE to **SKIP** (honest skip, not false green).
- `StorageError::Conductor` 503 should carry a short `Retry-After` (mirror the 2s admission value) so the doorway doesn't default to 30 and trip the breaker after 3.
- Build-provenance guard: a `scripts/ci/` test asserting the resolved storage build's conductor image is the patched harbor image on dev (fails today; passes once `7747f3ec8` lands).

---

## Self-review

- **Coverage:** all 8 RCA causes are placed — #3,#4 → Tasks 2,1 (implement); #5 → Task 3 (design capture); #1,#2,#6,#7,#8 → operator runbook. The 5 named stages each map to a cause + bucket.
- **No placeholders:** Task 1 carries complete code + exact commands; Task 2 is gated on a named design decision (intentional — not a placeholder); Task 3 is explicitly a capture, not an implementation.
- **Type consistency:** `mark_local_wants_satisfied(&[String])` (Task 1) is used exactly as defined; `finalize_quilt_draw` (Task 2) mirrors the existing `finalize_fetch_success` signature.

## Done (stability-gated, not single-green)

- Task 1 merged; on the next dev redeploy, `Verify Projection Sync` no longer fails on the content-holder's `pull=false`.
- Task 2 merged (post-gate); `Verify Delivery Events` reports ≥1 event after a replication-driven draw.
- Task 3 captured as an operator/security backlog concern with a p2p-design-gate.
- Operator runbook executed: patched conductor fleet-wide, genesis re-run shows stages 1/2/4 green and the card's columns lighting as their (deploy + Epic-B + identity) gates clear.
