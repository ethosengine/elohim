---
title: "SDK-promise substrate — the six-property implementation path (legibility first, then propagation, convergence, and the three unwired reds)"
id: sdk-promise-substrate-program-plan
status: Ready
class: substrate
context-tier: disclosed
steward: rust-architect
graduation-trigger: superseded-by-implementation — graduate once habits `sync-scale-honesty` and `notary-authority` are green with evidence and the three unwired nodes each carry a runnable red
created: 2026-07-25
domain: D5
sprint: charter-serving
topic: [sync-plane, legibility, convergence, attribution, reach-enforcement, operator-surface, charter]
cites:
  - genesis/manifests/habits.yaml
  - elohim/elohim-storage/src/p2p/sync_round.rs
  - elohim/elohim-storage/tests/sync_scale_honesty.rs
  - elohim/elohim-storage/src/p2p/reconcile_rails.rs
  - elohim/elohim-storage/src/p2p/projection_reconcile.rs
  - elohim/elohim-storage/src/p2p/sync_protocol.rs
  - elohim/elohim-storage/src/p2p/binding_cross_signature.rs
  - genesis/data/timeline/backlog/genesis-pair-cross-conductor-fetch-blocks-canonical-convergence.md
  - genesis/data/timeline/backlog/agent-peer-binding-cross-signed-proof.md
  - genesis/data/timeline/backlog/http-reach-enforcement-gap.md
  - operator-surface-observe-act | Task 13 writes the red against THIS surface — read it before inventing operator verbs | sha256:75b92515588d0ff9 | path: genesis/docs/superpowers/specs/2026-07-05-operator-surface-observe-act-design.md
  - weave-epic-arc-design | Sec #4 names the X25519 reader-key block that keeps Task 12 encryption half HELD | sha256:69966fdcc15dd7ba | path: genesis/docs/superpowers/specs/2026-06-20-weave-epic-arc-design.md
  - coherent-transport-identity-resolver-design | Sec 0 lines 41-48 is the live security issue Task 11 red pins — bindings feed attribution today | sha256:63117b359cfa3891 | path: genesis/docs/superpowers/specs/2026-06-15-coherent-transport-identity-resolver-design.md
---

# SDK-Promise Substrate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the substrate able to say what it is doing, push what changed, and converge — in that order — so the four properties an embedded-persistence SDK must promise (legibility, propagation, convergence, attribution) stop being absent at four different layers, and the remaining two (confidentiality, actuation) each acquire a runnable red instead of a prose gap.

**Architecture:** The six gaps are not six parallel workstreams. **Legibility is the measuring instrument for the other five** — while `caughtUp: true` is reported over 1860 divergent anchors and `/health`'s cached copy disagrees with live `/p2p/status` by 12×, no cure to propagation or convergence can be *verified*, and any SLO offered over those fields would be a lie by construction. So Phase 0 lands an honest convergence metric and a staleness-stamped read path; Phase 1 cures propagation (announce-on-change send site + a round opener that is a function of local state); Phase 2 cures convergence (the isolated `rea_commitment_replication` sweettest red); Phase 3 writes the first red for each of the three `unwired` habits, because per the habits register covenant an unwired node's only legal first move is a runnable check — prose specs do not advance it.

**Tech Stack:** Rust (elohim-storage native + doorway-service native), Automerge CRDT sync plane, libp2p 0.54.1 + iroh dual transport, rmp-serde MessagePack wire, Prometheus metrics, Holochain sweettest, Cucumber/a2o BDD, ts-rs + JSON-Schema view contract.

## Global Constraints

- **Storage builds:** `RUSTFLAGS='--cfg getrandom_backend="custom"'`, `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/integ/elohim__elohim-storage/dev`, run from `/projects/elohim/elohim/elohim-storage`. Explicit `cd` per gate — shell state does not persist between Bash calls.
- **Doorway builds:** `RUSTFLAGS=""`, `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/integ/doorway__doorway-service/dev`, run from `/projects/elohim/doorway/doorway-service`.
- **Sweettest builds:** `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/integ/elohim__holochain__tests__sweettest/dev`. DNA/WASM workspaces (`elohim/holochain/dna/*`) use **plain cargo** — never redirect `target/`, `hc dna pack` canonicalizes `./target`.
- **No nextest in this container** — use plain `cargo test`. Long cargo runs go `run_in_background`.
- **Wire evolution is two-phase — but not for the reason this plan first gave.** MEASURED 2026-07-25 (`p2p::sync_protocol` tests, not assumption): `rmp_serde::to_vec` encodes an externally-tagged enum as `{VariantName: [fields…]}` — a fixmap keyed by the variant **NAME**, never an ordinal (`GetHeads` → `[0x81, 0xA8, "GetHeads", 0x92, …]`). So:
  - **Variant ORDER is NOT a hazard.** An earlier draft of this plan claimed inserting a variant anywhere but the end would renumber its successors. That is false — there are no ordinals. Append or insert freely.
  - **A variant's FIELDS are a positional fixarray**, so adding a field to an existing variant *is* wire-breaking. That is the real reason `ListDocumentsSince` is a new variant rather than an extra field on `ListDocuments`.
  - An older peer decoding an unknown variant **errors** rather than landing on a neighbour — verified by test, and the premise the rollout rests on. Therefore: **ship the handler in one release, ship the sender in the next.** Never both in one deploy.
  - **This gate applies ONLY to new variants.** `AnnounceChange` (Task 9) is an *existing* variant every peer already decodes, so its sender ships without waiting for anything.
- **The charter's fake-green guard is binding** (`charter.yaml`, node `sync-scale-honesty`): `sync_round::round_opener` and `sync_round::announcements_for_local_change` must stay the **only** constructors of the round opener and the announce requests. Returning announce requests without a caller that sends them turns the test green while nothing propagates.
- **The WIP fence is binding** (charter covenant rule 3): max 2 nodes `active: true`. `notary-authority` and `sync-scale-honesty` hold both slots today. Phase 3 does not activate its nodes — it writes their reds, which is legal for an `unwired` node and does not consume a slot.
- **Status flips require evidence** (charter covenant rule 4): a build number, a live probe, or a test run. Never edit `charter.yaml` status from intention.
- **Changing what a CI-gated measure counts is an operator call**, per the blob-durability precedent recorded in `charter.yaml`. Task 5 is explicitly gated on that.
- View-struct changes follow the schema contract: schema first (`elohim/sdk/schemas/v1/views/`), then the Rust struct, then `cargo test --test schema_contract`, then `pnpm run schema:codegen:ts`.

---

## Where this sits

| # | Gap (session evidence) | Charter article | Node status | This plan |
|---|---|---|---|---|
| 1 | Legibility — the substrate can't report its own failure | (instrument for all) | — | **Phase 0** — Tasks 1–5 |
| 2 | Propagation — poll-only, opener is O(corpus) | `sync-scale-honesty` | **red · active** | **Phase 1** — Tasks 6–8 |
| 3 | Convergence — heal converts nothing; B can't retrieve A's REA | `notary-authority` | **red · active** | **Phase 2** — Tasks 9–10 |
| 4 | Attribution — bindings are self-asserted | `identity-cross-signed` | unwired | **Phase 3** — Task 11 (write the red) |
| 5 | Confidentiality — no private-replica encryption; reach unenforced | `reach-enforced-everywhere` | unwired | **Phase 3** — Task 12 (write the red) |
| 6 | Actuation — operator plane is kubectl-only | `operator-runtime-surface` | unwired | **Phase 3** — Task 13 (write the red) |

## Execution status (2026-07-25, updated in flight)

| Task | State | Commit |
|---|---|---|
| 1 · `converged != caughtUp` in the gap machine | **done** | `1e5e51bc7` (on `dev`) |
| 2 · publish `converged`/`exhausted` on `/p2p/status` | **done** | `ae5cbc319` |
| 3 · reconcile honesty metrics | **done** | `e3ba801b3` |
| 4 · `/health` staleness stamp | **done** (parallel lane) | `02dedf204` |
| 5 · move the a2o gate off `caughtUp` | **blocked — operator decision** | — |
| 6 · deploy + read the instrument | pending runway | — |
| 7 · `ListDocumentsSince` handler (inert) | **done** | `0dda5cb6e` |
| 8 · stateful round opener | **gated on Task 7 being fleet-wide** | — |
| 9 · announce-on-change send site | **done — flips check 2** | `b3a7f45b5` |
| 10 · convergence RCA | pending (fleet-free; sweettest reproduces) | — |
| 11–13 · reds for the three unwired nodes | not started | — |

`cargo test --test sync_scale_honesty -- --ignored` is now **1 passed / 1 failed** (was 0/2). The
remaining red is check 1, which is Task 8 by design.

**Three premises in this plan were wrong and are corrected in place** — noted here because a plan
that quietly absorbs its own errors teaches nothing:
1. *Variant ordering was never a wire hazard* (see Global Constraints). Measured, not assumed.
2. *Task 9 was never gated on the Task 7 deploy.* `AnnounceChange` is an existing variant; only
   genuinely-new variants need the two-phase rollout. Treating them as one blocked chain would have
   idled the half that could ship.
3. *The reconcile plane was already partly metered.* `elohim_projection_heal_outcomes_total`,
   `…_reconcile_gaps{stream}` and `…_reconcile_local_total{stream}` already existed and are exactly
   what the dataplane shift was reading (`rea local_total:0`, `missing:372`). Task 3 was rescoped to
   add only what those cannot express, and adds no healed-total counter because heals are already
   derivable from the outcomes series.

---

**Prioritization note (staleness guard).** `genesis/data/timeline/roadmap/vision-readiness-sprint-roadmap.md` was last regenerated **2026-06-02** and ranks none of this work; `genesis/manifests/habits.yaml` was updated **2026-07-04** and carries live evidence through 2026-07-25. For dataplane work the habits register is the live prioritization surface and this plan follows it. The roadmap needs a regeneration pass — that is a cartographer job, filed as complementary work below, not folded into this plan.

**Complementary work captured, not absorbed** (one line each to `genesis/data/timeline/backlog/`, so scope stays genuine):
- Roadmap regeneration against the current charter + `--ledger` (cartographer).
- The C2 cross-signature slice decomposition (S2–S6) exists **only** in `.claude/shifts/2026-07-18T03-27-integrate-identity-head-c2-deliver.journal.md` — it needs a real plan file. Task 11 writes the red; the S2–S6 plan is a separate document.
- Bucketed/merkle-range digest refinement of the sync opener (Task 7 lands the flat digest; range-splitting is a follow-on).

**Held, deliberately not planned** (`@requires:` tags mark the divergent legs; this plan declares no doc-level `requires_env` because it is *mixed*):
- **Live encryption substrate** — `KeyEnvelope` DHT entry, `ShardManifest.encryption`/`plaintext_cid` field-add, X25519 reader-key resolver. `weave-epic-arc-design` §#4 marks it substrate-blocked: Holochain agent keys are ed25519, `crypto_box_seal` needs X25519, and nothing sources a per-reader X25519 pubkey from `agent_cid`. Task 12 covers the **enforcement** half of gap 5, which is unblocked; the encryption half stays HELD and is named as such rather than silently dropped.
- **Live alpha verification legs** — tagged `@requires:alpha-cluster-6peer` inline. `cluster-state.yaml` has that capability degraded. Every *cure* in Phases 0–2 is provable on `household-nodes` via cargo test + sweettest; only the fleet-wide confirmations wait.

---

# Phase 0 — Legibility: build the instrument before using it

Two of the three legibility legs were closed by commits already on this branch but **not yet deployed**: the acquisition leg's silence (`486982bb8`) and the sync plane's total absence of metrics. The third — an honest convergence metric — is unfixed, and it is the one the other phases need.

### Task 1: An honest convergence signal in the gap state machine

`GapTracker::update_caught_up` sets `caught_up = pending.is_empty()` (`reconcile_rails.rs:130-132`). `mark_failed` removes an id from `pending` without re-queueing it (`:125-128`), and `enqueue_missing` skips ids whose fail-count has reached `max_retries` (`:85-87`). So an id that exhausted its retries leaves `pending` forever and `caught_up` flips **true** — which is exactly what "caught up while 1860 anchors diverge" means. `ProjectionReconcileStatus.caught_up`'s own doc-comment already admits it: *"True when every discovered gap was healed **or exhausted retries**."*

The cure is **not** to redefine `caught_up` — its wire name is consumed by `/health`, `/p2p/status`, and a live a2o gate, and silently changing what an existing field means is the same dishonesty in the other direction. Add a distinct, strictly-stronger signal alongside it.

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/reconcile_rails.rs` (`GapCounts` ~`:23-29`, `GapTracker` impl ~`:39-46`)
- Test: `elohim/elohim-storage/src/p2p/reconcile_rails.rs` (`#[cfg(test)] mod tests`, in-file — this crate tests the rails in-module)

**Interfaces:**
- Produces: `GapCounts { pending, completed, failed, caught_up, exhausted: usize, converged: bool }` and `GapTracker::exhausted_count(&self) -> usize`. `converged` is `pending.is_empty() && exhausted_count() == 0`. Tasks 2 and 3 consume both new fields.

- [ ] **Step 1: Write the failing test**

Add to the existing `#[cfg(test)] mod tests` in `reconcile_rails.rs`:

```rust
#[test]
fn retry_exhausted_gaps_are_caught_up_but_not_converged() {
    // The live lie: a sweep whose gaps all failed past max_retries drains
    // `pending` and reports caught_up=true while nothing was healed. That is
    // "this sweep is over", not "this peer holds what its peers hold".
    let mut t = GapTracker::new(2);
    t.discover(vec!["a".into(), "b".into(), "c".into()]);
    for _ in 0..2 {
        for id in ["a", "b", "c"] {
            t.mark_failed(id);
        }
        // Retry-on-NEXT-cycle: re-discovery is what re-enqueues (R-E).
        t.discover(vec!["a".into(), "b".into(), "c".into()]);
    }
    t.update_caught_up();

    let c = t.counts();
    assert_eq!(c.completed, 0, "nothing was healed");
    assert!(c.caught_up, "existing semantics preserved: the sweep is over");
    assert_eq!(c.exhausted, 3, "all three exhausted their retry budget");
    assert!(
        !c.converged,
        "converged must be FALSE when gaps were abandoned, not healed"
    );
}

#[test]
fn a_fully_healed_sweep_is_both_caught_up_and_converged() {
    let mut t = GapTracker::new(2);
    t.discover(vec!["a".into(), "b".into()]);
    t.mark_completed("a");
    t.mark_completed("b");
    t.update_caught_up();

    let c = t.counts();
    assert!(c.caught_up);
    assert!(c.converged, "healed gaps converge");
    assert_eq!(c.exhausted, 0);
}
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd /projects/elohim/elohim/elohim-storage && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/integ/elohim__elohim-storage/dev \
  cargo test --lib p2p::reconcile_rails::tests:: -- --nocapture
```

Expected: FAIL — `no field 'exhausted' on type 'GapCounts'` / `no field 'converged'`.

- [ ] **Step 3: Implement**

In `reconcile_rails.rs`, extend `GapCounts` and `GapTracker`:

```rust
/// Snapshot counts for status surfaces.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GapCounts {
    pub pending: usize,
    pub completed: usize,
    pub failed: usize,
    /// This SWEEP is over: every discovered gap was healed OR exhausted its
    /// retry budget. Load-bearing for the existing `/health` + a2o surfaces;
    /// deliberately NOT renamed. It does not mean the peer converged.
    pub caught_up: bool,
    /// Gaps abandoned at `max_retries` — counted, never silently dropped.
    pub exhausted: usize,
    /// This PEER holds what its peers advertised: nothing pending AND nothing
    /// abandoned. Strictly stronger than `caught_up`; this is the field an SLO
    /// may be offered over.
    pub converged: bool,
}

impl GapTracker {
    /// Ids whose retry budget is spent. `enqueue_missing` refuses to re-queue
    /// these (`:85-87`), so they are permanently absent from `pending` — the
    /// exact reason `pending.is_empty()` overstates convergence.
    pub fn exhausted_count(&self) -> usize {
        self.failed
            .values()
            .filter(|&&n| n >= self.max_retries)
            .count()
    }

    pub fn counts(&self) -> GapCounts {
        let exhausted = self.exhausted_count();
        GapCounts {
            pending: self.pending.len(),
            completed: self.completed.len(),
            failed: self.failed.len(),
            caught_up: self.caught_up,
            exhausted,
            converged: self.pending.is_empty() && exhausted == 0,
        }
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cd /projects/elohim/elohim/elohim-storage && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/integ/elohim__elohim-storage/dev \
  cargo test --lib p2p::reconcile_rails
```

Expected: PASS, including the pre-existing rails tests (the `GapCounts` construction sites in `projection_reconcile.rs:1745,1767` use struct literals — if they fail to compile, add `..Default::default()` there rather than changing the new fields' meaning).

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/p2p/reconcile_rails.rs
git commit -m "feat(storage): converged != caught_up — count retry-exhausted gaps honestly

caught_up is 'this sweep is over'; it flips true when every gap exhausted
its retry budget without healing. That is what 'caughtUp: true over 1860
divergent anchors' was reporting. Adds GapCounts::{exhausted,converged}
alongside it — no existing field changes meaning."
```

---

### Task 2: Surface `converged` on the projection-reconcile status and `/p2p/status`

`ProjectionReconcileStatus` already carries `divergent_anchor` (gaps present locally under a *different* anchor than a peer advertised) and `healed_total`/`sweeps`. Convergence at the status layer is the conjunction of all three signals: nothing pending, nothing abandoned, nothing divergent.

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/projection_reconcile.rs` (`ProjectionReconcileStatus` ~`:335-358`, `publish_sweep` ~`:383-395`)
- Modify: `elohim/sdk/schemas/v1/views/` — the view schema backing this status (locate with `grep -rl "divergentAnchor" elohim/sdk/schemas/v1/views/`)
- Test: `elohim/elohim-storage/src/p2p/projection_reconcile.rs` (in-file tests ~`:1730-1780`), plus `elohim/elohim-storage/tests/schema_contract.rs`

**Interfaces:**
- Consumes: `GapCounts::{exhausted, converged}` from Task 1.
- Produces: `ProjectionReconcileStatus { …, exhausted: usize, converged: bool }`, serialized as `exhausted` / `converged` (camelCase-identical). Task 3 and Task 5 consume `converged`.

- [ ] **Step 1: Write the failing test**

Extend the existing sweep-publishing test in `projection_reconcile.rs`:

```rust
#[tokio::test]
async fn a_sweep_with_divergent_anchors_is_not_converged() {
    let state = ProjectionReconcileState::new();
    // Every gap healed, retry budget untouched — but 1860 rows sit locally
    // under an anchor no peer advertises. This is the live beta shape.
    state
        .publish_sweep(
            crate::p2p::reconcile_rails::GapCounts {
                pending: 0,
                completed: 4,
                failed: 0,
                caught_up: true,
                exhausted: 0,
                converged: true,
            },
            3,    // peers_asked
            1860, // divergent_anchor
        )
        .await;

    let s = state.status().await;
    assert!(s.caught_up, "the sweep did finish");
    assert_eq!(s.divergent_anchor, 1860);
    assert!(
        !s.converged,
        "divergent anchors mean this peer does NOT hold what its peers hold"
    );
}
```

- [ ] **Step 2: Run it to verify it fails**

```bash
cd /projects/elohim/elohim/elohim-storage && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/integ/elohim__elohim-storage/dev \
  cargo test --lib p2p::projection_reconcile
```

Expected: FAIL — `no field 'converged' on type 'ProjectionReconcileStatus'`.

- [ ] **Step 3: Update the view schema FIRST, then the struct**

Per the crate's schema contract (`elohim/elohim-storage/CLAUDE.md`), the JSON Schema is the source of truth. Add to the status view schema, next to `divergentAnchor`:

```json
"exhausted": {
  "type": "integer",
  "minimum": 0,
  "description": "Gaps abandoned at max_retries this sweep — healed nothing, retried no more."
},
"converged": {
  "type": "boolean",
  "description": "This peer holds what its peers advertised: pending==0 AND exhausted==0 AND divergentAnchor==0. Strictly stronger than caughtUp, which means only that the sweep finished."
}
```

Then the struct in `projection_reconcile.rs`:

```rust
    /// Gaps abandoned at MAX_RETRIES this sweep (healed nothing, retried no more).
    #[ts(type = "number")]
    pub exhausted: usize,
    /// True when this peer holds what its peers advertised: nothing pending,
    /// nothing abandoned, nothing divergent. `caught_up` says only that the
    /// sweep ended — an SLO may be offered over THIS field, not that one.
    pub converged: bool,
```

and in `publish_sweep`, after `s.divergent_anchor = divergent_anchor;`:

```rust
        s.exhausted = counts.exhausted;
        s.converged = counts.converged && divergent_anchor == 0;
```

- [ ] **Step 4: Run the tests + the contract + codegen**

```bash
cd /projects/elohim/elohim/elohim-storage && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/integ/elohim__elohim-storage/dev \
  cargo test --lib p2p::projection_reconcile && \
  cargo test --test schema_contract
cd /projects/elohim && pnpm run schema:codegen:ts
```

Expected: PASS on both; codegen rewrites the generated TS. Note `feedback_codegen_prettier_oscillation` — Reach/ContentFormat union line-wraps flip on every codegen run across ~18 generated files; that churn is cosmetic, commit it without chasing it.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/p2p/projection_reconcile.rs elohim/sdk/schemas/v1/views/ elohim/sdk/storage-client-ts/src/generated/
git commit -m "feat(storage): publish converged/exhausted on projection-reconcile status

converged = pending==0 && exhausted==0 && divergentAnchor==0. caughtUp is
unchanged and still means 'the sweep ended'. The 22-sweep/healedTotal:0/
61-pending beta shape now reports converged=false instead of caughtUp=true."
```

---

### Task 3: Emit convergence as a metric, so the sync plane's honesty is graphable

The sync series landed this session (`elohim_sync_rounds_total`, `elohim_sync_requests_total{kind}`, `elohim_sync_docs_enumerated_total`, `elohim_sync_request_outcomes_total{result}`). The reconcile plane still has none, which is why `healedTotal: 0 across 22 sweeps` had to be read out of a status endpoint by hand.

**Files:**
- Modify: `elohim/elohim-storage/src/metrics.rs`
- Modify: `elohim/elohim-storage/src/p2p/projection_reconcile.rs` (call the emitters from `publish_sweep`)
- Test: `elohim/elohim-storage/src/metrics.rs` (in-file tests)

**Interfaces:**
- Produces: `metrics::observe_reconcile_sweep(counts: &GapCounts, divergent_anchor: usize)`, emitting `elohim_reconcile_sweeps_total`, `elohim_reconcile_healed_total`, `elohim_reconcile_gaps{state}` where `state ∈ {pending, exhausted, divergent}`, and `elohim_reconcile_converged` (gauge, 1/0).

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn reconcile_gap_state_labels_are_bounded() {
    // Same discipline as OUTBOUND_FAILURE_LABELS: a Prometheus label value
    // must come from a closed set. Gap ids and peer ids are cardinality bombs.
    assert_eq!(
        RECONCILE_GAP_STATE_LABELS,
        ["pending", "exhausted", "divergent"]
    );
}

#[test]
fn a_sweep_that_healed_nothing_reports_converged_zero() {
    let counts = crate::p2p::reconcile_rails::GapCounts {
        pending: 61,
        completed: 0,
        failed: 61,
        caught_up: false,
        exhausted: 61,
        converged: false,
    };
    assert_eq!(converged_gauge_value(&counts, 0), 0.0);
    assert_eq!(
        converged_gauge_value(
            &crate::p2p::reconcile_rails::GapCounts {
                pending: 0,
                completed: 5,
                failed: 0,
                caught_up: true,
                exhausted: 0,
                converged: true,
            },
            0
        ),
        1.0
    );
    // Divergence alone defeats convergence even with a clean gap ledger.
    assert_eq!(
        converged_gauge_value(
            &crate::p2p::reconcile_rails::GapCounts {
                pending: 0,
                completed: 5,
                failed: 0,
                caught_up: true,
                exhausted: 0,
                converged: true,
            },
            1860
        ),
        0.0
    );
}
```

- [ ] **Step 2: Run it to verify it fails**

```bash
cd /projects/elohim/elohim/elohim-storage && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/integ/elohim__elohim-storage/dev \
  cargo test --lib metrics::
```

Expected: FAIL — `cannot find value RECONCILE_GAP_STATE_LABELS` / `cannot find function converged_gauge_value`.

- [ ] **Step 3: Implement**

Read the existing sync emitters in `metrics.rs` (`inc_sync_round`, `inc_sync_requests`, etc.) and follow their registration pattern exactly — same registry, same naming, same bounded-label discipline. Add:

```rust
/// Closed set of gap-state label values. Unbounded labels (gap ids, peer ids)
/// are a cardinality bomb at mesh scale; the id stays in the log line.
pub const RECONCILE_GAP_STATE_LABELS: [&str; 3] = ["pending", "exhausted", "divergent"];

/// 1.0 only when the peer holds what its peers advertised. Pure so it is
/// testable without a registry.
pub fn converged_gauge_value(
    counts: &crate::p2p::reconcile_rails::GapCounts,
    divergent_anchor: usize,
) -> f64 {
    if counts.converged && divergent_anchor == 0 {
        1.0
    } else {
        0.0
    }
}

pub fn observe_reconcile_sweep(
    counts: &crate::p2p::reconcile_rails::GapCounts,
    divergent_anchor: usize,
) {
    // Register + emit following the sync-series pattern above in this file.
    // Counters: elohim_reconcile_sweeps_total (+1),
    //           elohim_reconcile_healed_total (+counts.completed).
    // Gauges:   elohim_reconcile_gaps{state="pending"}   = counts.pending
    //           elohim_reconcile_gaps{state="exhausted"} = counts.exhausted
    //           elohim_reconcile_gaps{state="divergent"} = divergent_anchor
    //           elohim_reconcile_converged = converged_gauge_value(..)
}
```

Then call it from `publish_sweep` in `projection_reconcile.rs`, immediately after the state mutation:

```rust
        crate::metrics::observe_reconcile_sweep(&counts, divergent_anchor);
```

- [ ] **Step 4: Run the tests**

```bash
cd /projects/elohim/elohim/elohim-storage && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/integ/elohim__elohim-storage/dev \
  cargo test --lib metrics:: p2p::projection_reconcile
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/metrics.rs elohim/elohim-storage/src/p2p/projection_reconcile.rs
git commit -m "feat(storage): reconcile-plane metrics — sweeps, healed, gaps{state}, converged

healedTotal:0 across 22 sweeps had to be read out of a status endpoint by
hand. It is now a series, and elohim_reconcile_converged is the gauge an SLO
can ride. Labels bounded, reusing the sync-series vocabulary."
```

---

### Task 4: Stamp `/health`'s cached P2P copy with its own age

`/health` reads a cached P2P snapshot via `try_read` specifically to avoid stalling the health check (`doorway/doorway-service/src/routes/health.rs:252`) — that non-blocking read is correct and must stay. The defect is that the cached copy is served **undated**, so a 12×-divergent stale snapshot is indistinguishable from a fresh one. Do not make `/health` synchronous; make it honest about what it is serving.

**Files:**
- Modify: `doorway/doorway-service/src/routes/health.rs` (the P2P health struct ~`:85-95`, the read site ~`:252`)
- Modify: the doorway view schema for the health response (locate with `grep -rl "caughtUp" doorway/ --include=*.json`)
- Test: `doorway/doorway-service/src/routes/health.rs` (in-file tests ~`:443`), `doorway/doorway-service/tests/schema_contract.rs`

**Interfaces:**
- Produces: `P2pHealth { …, observed_age_ms: Option<u64>, stale: bool, converged: Option<bool> }`. `stale` is `observed_age_ms > P2P_HEALTH_STALE_AFTER_MS` (const, 120_000 — two sync rounds at the 60s default) or the snapshot is absent. Task 5 consumes `converged` and `stale`.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn a_cached_p2p_snapshot_older_than_two_rounds_is_marked_stale() {
    // The 12x /health-vs-/p2p-status divergence was a stale cache served as
    // if it were live. The cache stays (health must not block); its AGE ships
    // with it so a consumer can refuse an old number.
    assert!(!is_stale(Some(59_000)), "inside one round: fresh");
    assert!(!is_stale(Some(119_999)), "inside two rounds: fresh");
    assert!(is_stale(Some(120_001)), "past two rounds: stale");
    assert!(is_stale(None), "no snapshot at all is stale, never fresh");
}

#[test]
fn p2p_health_carries_converged_alongside_caught_up() {
    let h = P2pHealth {
        caught_up: Some(true),
        converged: Some(false),
        divergent_anchor: Some(1860),
        observed_age_ms: Some(4_000),
        stale: false,
        ..Default::default()
    };
    let v = serde_json::to_value(&h).unwrap();
    assert_eq!(v["caughtUp"], serde_json::json!(true));
    assert_eq!(v["converged"], serde_json::json!(false));
    assert_eq!(v["observedAgeMs"], serde_json::json!(4_000));
    assert_eq!(v["stale"], serde_json::json!(false));
}
```

- [ ] **Step 2: Run it to verify it fails**

```bash
cd /projects/elohim/doorway/doorway-service && \
  RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/integ/doorway__doorway-service/dev \
  cargo test --lib routes::health
```

Expected: FAIL — `cannot find function is_stale`, `no field converged`.

- [ ] **Step 3: Implement**

Read `health.rs:240-270` before editing to match the existing cache-read shape. Add:

```rust
/// Two sync rounds at the 60s default cadence. A snapshot older than this
/// describes a fleet state that has had two full opportunities to change.
pub const P2P_HEALTH_STALE_AFTER_MS: u64 = 120_000;

pub fn is_stale(observed_age_ms: Option<u64>) -> bool {
    match observed_age_ms {
        Some(age) => age > P2P_HEALTH_STALE_AFTER_MS,
        None => true,
    }
}
```

Extend `P2pHealth` with `converged: Option<bool>`, `observed_age_ms: Option<u64>`, `stale: bool`; at the cache-read site, record the snapshot's write instant and compute the age at read time. When `try_read` returns nothing, emit `stale: true` with `observed_age_ms: None` rather than omitting the block.

- [ ] **Step 4: Run the tests + schema contract**

```bash
cd /projects/elohim/doorway/doorway-service && \
  RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/integ/doorway__doorway-service/dev \
  cargo test --lib routes::health && cargo test --test schema_contract && \
  cargo clippy -- -D warnings && cargo fmt --check
```

Expected: PASS on all four.

- [ ] **Step 5: Commit**

```bash
git add doorway/doorway-service/src/routes/health.rs doorway/doorway-service/tests/ elohim/sdk/schemas/
git commit -m "feat(doorway): /health p2p carries observedAgeMs + stale + converged

/health's cached copy disagreed with live /p2p/status by 12x and shipped
undated, so the two were indistinguishable. The cache stays (health must not
block); its age ships with it, and converged rides alongside caughtUp."
```

---

### Task 5: Move the a2o gate onto the trustworthy number — **operator-gated**

`genesis/a2o/features/dataplane/peer-mesh.feature:22,28` asserts `/health p2p.caughtUp is true` for `alpha-A` and `elohim.host`. That gate reads the less trustworthy of the two numbers, and `caughtUp` overstates convergence by design. Changing it changes what a CI-gated measure counts — which `charter.yaml` records as an **operator call** (the blob-durability `@browser-only` precedent).

**Files:**
- Modify: `genesis/a2o/features/dataplane/peer-mesh.feature:18-30`
- Modify: the step definition backing `/health p2p.caughtUp is true` (find with `grep -rn "p2p.caughtUp" genesis/a2o/steps/`)

**Interfaces:**
- Consumes: `converged` and `stale` from Tasks 2 and 4.

- [ ] **Step 1: Present the measure change to the operator, do not land it unilaterally**

State plainly: the scenario currently passes when a peer abandoned every gap; on `converged` it will fail until Phase 1 and 2 land, so this **converts a green scenario to a red one** and the red is honest. Ask whether to (a) switch the assertion now and accept the red, (b) add `converged` as an additional non-gating assertion first, or (c) hold until Phase 2 is green. Record the answer in the commit message.

- [ ] **Step 2: Write the scenario change (assuming (a) or (b))**

```gherkin
    # p2p.caughtUp only says the sweep ENDED — it flips true when every gap
    # exhausted its retry budget. p2p.converged is the honest signal, and
    # p2p.stale refuses an old cached snapshot outright.
    And peer "alpha-A" /health p2p.stale is false
    And peer "alpha-A" /health p2p.converged is true
    And peer "elohim.host" /health p2p.stale is false
    And peer "elohim.host" /health p2p.converged is true
```

- [ ] **Step 3: Add the step definitions**

Extend the existing `/health p2p.<field> is <bool>` step to accept `converged` and `stale`; do not write a bespoke step per field.

- [ ] **Step 4: Verify the scenario parses and the step resolves**

```bash
cd /projects/elohim/genesis/a2o && pnpm exec cucumber-js \
  genesis/a2o/features/dataplane/peer-mesh.feature --dry-run
```

Expected: no undefined steps. `@requires:alpha-cluster-6peer` — the *live* run of this scenario needs the alpha fleet and waits for the next edge Dataplane Validation build; the dry-run proves the wiring on `household-nodes`.

- [ ] **Step 5: Commit**

```bash
git add genesis/a2o/features/dataplane/peer-mesh.feature genesis/a2o/steps/
git commit -m "test(a2o): peer-mesh gates on p2p.converged + p2p.stale, not caughtUp

Operator-approved measure change (<record the decision here>). caughtUp
passes when a peer abandoned every gap; the new assertions cannot."
```

---

### Task 6: Deploy Phase 0 and confirm the instrument reads

Everything above — plus this session's already-committed acquisition-legibility and sync-metrics work — is inert until it reaches the fleet. `@requires:alpha-cluster-6peer`.

- [ ] **Step 1:** Merge the branch to `dev` by local fast-forward (no PR — `dev` is the integration target). **One push per batch**: concurrent dev pushes minutes apart mutually abort each other's builds.
- [ ] **Step 2:** Watch the edge pipeline via `mcp__jenkins__getBuild` / `getBuildLog`. Do not trigger builds through the MCP — it is anonymous and `triggerBuild` is denied; a `[build:edge]` commit tag on a fresh push is the only dispatch path.
- [ ] **Step 3:** Confirm the new series exist and are non-empty:

```
elohim_reconcile_sweeps_total
elohim_reconcile_healed_total
elohim_reconcile_gaps{state="divergent"}
elohim_reconcile_converged
elohim_sync_docs_enumerated_total
elohim_sync_request_outcomes_total{result="timeout"}
```

Query via `mcp__observability__query_prometheus`. `elohim_sync_docs_enumerated_total`'s **rate over rounds** is the scaling read Phase 1 will move.

- [ ] **Step 4:** Re-measure the acquisition leg. It failed 100% on both genesis peers (29/29, 5/5, `fetched=0`) and emitted zero lines in 1.13M logs over 6h. With `486982bb8` deployed it must now emit at or above `info!`. If it is still silent, that is a **new** finding — file it, do not absorb it into this plan.
- [ ] **Step 5:** Record the delta in `charter.yaml` (one line, evidence-backed: build number + the queried values). Do not flip any status yet — Phase 0 builds the instrument, it does not cure a node.

---

# Phase 1 — Propagation: cure `sync-scale-honesty`

Two invariant checks are red in `elohim/elohim-storage/tests/sync_scale_honesty.rs`, both asserting properties rather than mechanisms, so any valid cure flips them. They are `#[ignore]`d deliberately — the storage pre-push gate runs plain `cargo test` across every `tests/` target, so a standing red in the default sweep would wedge the gate for everyone touching the crate. **Keep the `#[ignore]`** until both pass.

Run the red at any time with:

```bash
cd /projects/elohim/elohim/elohim-storage && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/integ/elohim__elohim-storage/dev \
  cargo test --test sync_scale_honesty -- --ignored
```

Baseline (verified 2026-07-25): exit 101, 0 passed, 2 failed.

### Task 7: Ship the `ListDocumentsSince` handler — sender NOT wired

`SyncRequest` is an externally-tagged serde enum over `rmp_serde::to_vec` (compact positional encoding). A peer that predates a new variant fails to decode it; a peer that predates a new *field* on an existing variant mis-parses or errors. Rollout is therefore handler-first, sender-second, across two deploys. This task is deploy #1 and **changes no behavior**.

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/sync_protocol.rs` (add the variant + the response variant)
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` (~`:6423` — the libp2p request handler)
- Modify: `elohim/elohim-storage/src/p2p_iroh/sync_backend.rs` (~`:147` — the iroh handler)
- Test: `elohim/elohim-storage/src/p2p/sync_protocol.rs` (in-file round-trip tests ~`:388`)

**Interfaces:**
- Produces:
  ```rust
  SyncRequest::ListDocumentsSince {
      h_app_id: String,
      prefix: Option<String>,
      /// Fingerprint of the requester's whole corpus for this namespace.
      corpus_digest: String,
      limit: u32,
  }
  SyncResponse::InSync { h_app_id: String, corpus_digest: String }
  ```
  plus `sync_round::corpus_digest(local: &LocalCorpusState) -> String`. Task 8 consumes both.

- [ ] **Step 1: Write the failing test**

In `sync_protocol.rs` tests:

```rust
#[test]
fn list_documents_since_round_trips_over_msgpack() {
    let req = SyncRequest::ListDocumentsSince {
        h_app_id: "elohim".into(),
        prefix: None,
        corpus_digest: "sha256:deadbeef".into(),
        limit: 1000,
    };
    let bytes = rmp_serde::to_vec(&req).unwrap();
    let decoded: SyncRequest = rmp_serde::from_slice(&bytes).unwrap();
    assert!(matches!(decoded, SyncRequest::ListDocumentsSince { .. }));
}

#[test]
fn in_sync_response_round_trips_over_msgpack() {
    let resp = SyncResponse::InSync {
        h_app_id: "elohim".into(),
        corpus_digest: "sha256:deadbeef".into(),
    };
    let bytes = rmp_serde::to_vec(&resp).unwrap();
    let decoded: SyncResponse = rmp_serde::from_slice(&bytes).unwrap();
    assert!(matches!(decoded, SyncResponse::InSync { .. }));
}

#[test]
fn the_new_variants_are_appended_so_existing_variant_indices_do_not_move() {
    // Compact msgpack encodes an externally-tagged enum as [index, payload].
    // Inserting a variant ANYWHERE but the end renumbers its successors and
    // silently reinterprets every in-flight request on the mesh.
    let heads = rmp_serde::to_vec(&SyncRequest::GetHeads {
        h_app_id: "elohim".into(),
        doc_id: "d".into(),
    })
    .unwrap();
    assert_eq!(heads[1], 0, "GetHeads must remain variant index 0");
}
```

- [ ] **Step 2: Run it to verify it fails**

```bash
cd /projects/elohim/elohim/elohim-storage && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/integ/elohim__elohim-storage/dev \
  cargo test --lib p2p::sync_protocol
```

Expected: FAIL — `no variant named ListDocumentsSince`.

- [ ] **Step 3: Implement the variants and both handlers**

Append `ListDocumentsSince` **after** `ListDocuments` in `SyncRequest`, and `InSync` **after** the last existing `SyncResponse` variant. Then, in each handler:

- Compute the responder's own digest over its corpus for `h_app_id` (reuse `sync_round::corpus_digest`).
- If it equals the requester's `corpus_digest`, reply `SyncResponse::InSync` — **O(1), no enumeration**.
- Otherwise, fall through to exactly the existing `ListDocuments` path. Correctness is unchanged in the divergent case; only the converged case gets cheap.

Add `corpus_digest` to `sync_round.rs`:

```rust
/// A stable fingerprint of the whole local corpus for one namespace: the
/// sorted (doc_id, sorted heads) set, hashed. Two peers holding the same
/// documents at the same heads produce the same digest regardless of the
/// order their stores enumerate in.
pub fn corpus_digest(local: &LocalCorpusState) -> String {
    use sha2::{Digest, Sha256};
    let mut entries: Vec<String> = local
        .docs
        .iter()
        .map(|d| {
            let mut heads = d.heads.clone();
            heads.sort();
            format!("{}={}", d.doc_id, heads.join(","))
        })
        .collect();
    entries.sort();
    let mut h = Sha256::new();
    for e in &entries {
        h.update(e.as_bytes());
        h.update(b"\n");
    }
    format!("sha256:{:x}", h.finalize())
}
```

- [ ] **Step 4: Run the tests**

```bash
cd /projects/elohim/elohim/elohim-storage && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/integ/elohim__elohim-storage/dev \
  cargo test --lib p2p::sync_protocol p2p::sync_round && \
  cargo test --test sync_scale_honesty -- --ignored
```

Expected: the protocol tests PASS; `sync_scale_honesty` still **2 failed** — nothing sends the new request yet. That is correct for this task, and is the two-phase rollout working as designed.

- [ ] **Step 5: Commit and deploy before Task 8**

```bash
git add elohim/elohim-storage/src/p2p/sync_protocol.rs elohim/elohim-storage/src/p2p/mod.rs elohim/elohim-storage/src/p2p_iroh/sync_backend.rs elohim/elohim-storage/src/p2p/sync_round.rs
git commit -m "feat(storage): ListDocumentsSince/InSync handler on both transports (sender unwired)

Deploy 1 of 2. rmp compact encoding makes a new variant decode-rejecting on
older peers, so the handler must be fleet-wide BEFORE any sender exists.
Behavior is unchanged: nothing constructs the new request yet."
```

**Gate:** do not start Task 8 until this is live on every peer in the namespace. Confirm via a per-peer version probe, not by assumption.

---

### Task 8: Make the round opener a function of local state

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/sync_round.rs` (`round_opener` ~`:121-129`, and the module doc's "Known-unsatisfied invariant" block ~`:13-28`)
- Test: `elohim/elohim-storage/tests/sync_scale_honesty.rs` (already written — do not weaken it)

**Interfaces:**
- Consumes: `SyncRequest::ListDocumentsSince`, `corpus_digest` (Task 7).
- Produces: `round_opener` returning `ListDocumentsSince` for a non-empty corpus, `ListDocuments` for an empty one.

- [ ] **Step 1: Run the existing red to confirm the exact failure**

```bash
cd /projects/elohim/elohim/elohim-storage && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/integ/elohim__elohim-storage/dev \
  cargo test --test sync_scale_honesty the_round_opener_reflects_what_we_already_have -- --ignored
```

Expected: FAIL — a 2000-doc node and an empty node produce a byte-identical opener.

- [ ] **Step 2: Implement**

```rust
/// The request that opens a round with one peer.
///
/// A peer that holds nothing must enumerate — it has no digest to compare and
/// wants everything. A peer that holds a corpus opens with its fingerprint, so
/// a converged pair exchanges one hash instead of the whole document list
/// (`SyncResponse::InSync`, O(1)); a divergent pair falls back to the full
/// enumeration on the responder side, so correctness is unchanged.
pub fn round_opener(h_app_id: &str, local: &LocalCorpusState) -> SyncRequest {
    if local.docs.is_empty() {
        return SyncRequest::ListDocuments {
            h_app_id: h_app_id.to_string(),
            prefix: None,
            offset: 0,
            limit: SYNC_LIST_PAGE_LIMIT,
        };
    }
    SyncRequest::ListDocumentsSince {
        h_app_id: h_app_id.to_string(),
        prefix: None,
        corpus_digest: corpus_digest(local),
        limit: SYNC_LIST_PAGE_LIMIT,
    }
}
```

Delete the now-false first bullet of the module doc's "Known-unsatisfied invariant" block; leave the second (announce) bullet until Task 9.

- [ ] **Step 3: Run the test to verify it passes**

```bash
cd /projects/elohim/elohim/elohim-storage && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/integ/elohim__elohim-storage/dev \
  cargo test --test sync_scale_honesty -- --ignored
```

Expected: `the_round_opener_reflects_what_we_already_have` PASS; `a_local_change_is_announced_to_connected_peers` still FAIL.

- [ ] **Step 4: Prove the wire actually changed, not just the planner**

```bash
cd /projects/elohim/elohim/elohim-storage && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/integ/elohim__elohim-storage/dev \
  cargo test --test sync_libp2p_convergence
```

Expected: PASS. This suite re-implements the round in the test file, so it is **not** proof the production planner changed — it is proof the cure did not break convergence. Both readings matter; do not treat it as the primary evidence.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/p2p/sync_round.rs
git commit -m "fix(storage): round opener carries the local corpus digest

Deploy 2 of 2. A converged pair now exchanges one hash (SyncResponse::InSync)
instead of the whole document list every tick; a divergent pair falls back to
full enumeration, so correctness is unchanged. Flips sync_scale_honesty check 1.
Watch rate(elohim_sync_docs_enumerated_total) — it should collapse."
```

---

### Task 9: Wire the announce-on-change send site

`SyncRequest::AnnounceChange` is defined (`sync_protocol.rs:77`) and handled by both transports (`p2p/mod.rs:6423`, `p2p_iroh/sync_backend.rs:147`), but **nothing in `src/` constructs one**. `sync::projector::project_content_doc` returns `Ok(true)` exactly when a local change was written — that is the send site the habits register guard names.

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/sync_round.rs` (`announcements_for_local_change` ~`:131-144`)
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` (add a `P2PCommand` variant + its swarm-loop arm; follow the existing `P2PCommand` pattern)
- Modify: the callers of `sync::projector::project_content_doc` (find with `grep -rn "project_content_doc(" elohim/elohim-storage/src/ | grep -v "fn project_content_doc"`)
- Test: `elohim/elohim-storage/tests/sync_scale_honesty.rs` (already written)

**Interfaces:**
- Consumes: `project_content_doc -> Result<bool, StorageError>` (true = a local change was written).
- Produces: `P2PCommand::AnnounceLocalChange { doc_id: String, change_hash: String }`; `announcements_for_local_change(h_app_id, doc_id, change_hash, peers) -> Vec<(PeerId, SyncRequest)>` returning one `AnnounceChange` per connected peer.

- [ ] **Step 1: Run the existing red**

```bash
cd /projects/elohim/elohim/elohim-storage && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/integ/elohim__elohim-storage/dev \
  cargo test --test sync_scale_honesty a_local_change_is_announced_to_connected_peers -- --ignored
```

Expected: FAIL — 0 announcements for 2 connected peers.

- [ ] **Step 2: Implement the planner half**

```rust
/// The push notifications a locally-authored change owes each connected peer.
///
/// One `AnnounceChange` per peer, metadata-only (`change_data: None`) — the
/// receiving peer pulls the bytes through the existing `GetChanges` path, so a
/// large change never fans out N times across the mesh.
pub fn announcements_for_local_change(
    h_app_id: &str,
    doc_id: &str,
    change_hash: &str,
    peers: &[PeerId],
) -> Vec<(PeerId, SyncRequest)> {
    peers
        .iter()
        .map(|p| {
            (
                *p,
                SyncRequest::AnnounceChange {
                    h_app_id: h_app_id.to_string(),
                    doc_id: doc_id.to_string(),
                    change_hash: change_hash.to_string(),
                    change_data: None,
                },
            )
        })
        .collect()
}
```

- [ ] **Step 3: Wire the real send site — this is the half that matters**

The charter's fake-green guard is explicit: returning requests without a caller that sends them turns the test green while nothing propagates. So:

1. Add `P2PCommand::AnnounceLocalChange { doc_id, change_hash }` following the existing `P2PCommand` variants in `p2p/mod.rs`.
2. In the swarm event loop's command arm, collect `swarm.connected_peers()`, call `sync_round::announcements_for_local_change(PROJECTION_NAMESPACE, &doc_id, &change_hash, &peers)`, and send each request through the same request-response behaviour the round uses. Increment `elohim_sync_requests_total{kind="announce_change"}` at the send site.
3. At every `project_content_doc` caller, when it returns `Ok(true)`, send the command. Read the head from the doc after projection — do **not** invent a hash.
4. Do not remove or shorten the 60s round. Announce is the fast path; the round remains the reconciliation backstop that repairs a missed announcement.

- [ ] **Step 4: Run the full check**

```bash
cd /projects/elohim/elohim/elohim-storage && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/integ/elohim__elohim-storage/dev \
  cargo test --test sync_scale_honesty -- --ignored && \
  cargo test --test sync_libp2p_convergence && \
  cargo clippy -- -D warnings && cargo fmt --check
```

Expected: `sync_scale_honesty` **2 passed, 0 failed**; convergence suite PASS; clippy and fmt clean.

- [ ] **Step 5: Remove the `#[ignore]`s and commit**

Both checks now pass, so they belong in the default sweep where they guard against regression. Delete the `#[ignore]` attributes and confirm the plain gate is green:

```bash
cd /projects/elohim/elohim/elohim-storage && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/integ/elohim__elohim-storage/dev \
  cargo test --test sync_scale_honesty
```

```bash
git add elohim/elohim-storage/src/p2p/sync_round.rs elohim/elohim-storage/src/p2p/mod.rs elohim/elohim-storage/src/sync/
git commit -m "feat(storage): announce-on-change send site — the poll is no longer the only path

AnnounceChange was defined and handled by both transports since inception with
no producer. project_content_doc returning Ok(true) is the local-change signal;
it now drives P2PCommand::AnnounceLocalChange to every connected peer,
metadata-only. The 60s round stays as the reconciliation backstop.
sync_scale_honesty: 2 passed, ignores removed."
```

- [ ] **Step 6: Flip the habit with evidence** — `@requires:alpha-cluster-6peer` for the live half

Deploy, then record in `charter.yaml`: the test run (2/2), the build number, and the measured `rate(elohim_sync_docs_enumerated_total)` before vs after. Flip `sync-scale-honesty` to `green` only with those numbers in `evidence:`. Set `active: false`, freeing a WIP slot for Phase 3.

---

# Phase 2 — Convergence: cure the cross-peer retrieval seam

`notary-authority` is red and active. Its REA face is isolated and reproducible without the fleet: sweettest `rea_commitment_replication` (unignored at `837b772c9`) fails with peer B unable to fetch A's commitment inside 60s. Live, adam shows heal outcome `missing:372` with `rea local_total:0` — heal pacing was cured, which exposed that B cannot **retrieve** A-authored REA entries at all. `healedTotal: 0` across 22 sweeps on beta is the same defect seen from the other end: heal converts nothing because retrieval returns nothing.

### Task 10: Diagnose the retrieval seam against the isolated red

This is a **debugging** task, not a code-writing task. It must run under `superpowers:systematic-debugging` — writing a fix before the root cause is identified is how the five-defect convergence arc happened (`history/2026-07-12-substrate-convergence-five-defect-arc.md`, read it first).

**Files:**
- Test (the red): `elohim/holochain/tests/sweettest/` — `rea_commitment_replication`
- Read: `genesis/data/timeline/backlog/genesis-pair-cross-conductor-fetch-blocks-canonical-convergence.md` (REOPENED, carries the full Prometheus/Loki/diagnostics bundle)
- Read: `genesis/docs/content/elohim-protocol/history/2026-07-12-substrate-convergence-five-defect-arc.md`

- [ ] **Step 1: Reproduce**

```bash
cd /projects/elohim/elohim/holochain/tests/sweettest && \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/integ/elohim__holochain__tests__sweettest/dev \
  cargo test rea_commitment_replication -- --ignored --nocapture
```

Expected: 0 passed, 1 failed — B cannot fetch A's commitment in 60s. Run this in the background; sweettest builds are long.

- [ ] **Step 2: Start from the strongest existing lead, not from scratch**

**The fleet is full-arc.** On a full-arc fleet every zome `get`/`get_links` is **local-only** — a link miss means *gossip failed*, not that the data is absent (`project_full_arc_authority_disables_network_get`). This reframes the seam: "B cannot retrieve A's REA entry" is most likely "the entry/link never gossiped to B", not "B's fetch is broken". Test that hypothesis **first**:

- Does B's DHT hold A's `Commitment` entry at all (query B's local DHT directly in the sweettest, bypassing the coordinator fn)?
- If the entry is present but the **link** is not, the seam is link-op integration on A's side or link gossip — not fetch.
- If neither is present, it is gossip/publish timing: `charter.yaml` records that fresh actions need publish time, and restart churn runs ~20min (`substrate-trust-contract-runbook`).

- [ ] **Step 3: Rule in or out the named asymmetry**

`charter.yaml`'s own judgment: content and REA ride the **same Lamad cell**, but content's existing-row admission masks what REA's remote-advertised `IdToCommitment` lookups expose. So compare, in the same sweettest run, a content read and an REA read from B. If content resolves and REA does not on identical timing, the defect is in the `IdToCommitment` link path specifically — a much narrower target than "cross-conductor fetch".

- [ ] **Step 4: Write the fix only after the cause is named**

Record the root cause in the backlog entry before touching code. Then fix, and confirm:

```bash
cd /projects/elohim/elohim/holochain/tests/sweettest && \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/integ/elohim__holochain__tests__sweettest/dev \
  cargo test rea_commitment_replication -- --ignored --nocapture
```

Expected: 1 passed. **Beware the zome class:** if the fix touches only coordinator zomes the DNA hash does not move and the cure lands via `update_coordinators` hot-swap (`ALLOW_COORDINATOR_UPDATE`); if it touches integrity zomes it is a DNA-hash move needing a reinstall, and a partial reinstall across the genesis pair partitions the DHT. Determine which class the diff is in *before* deploying.

- [ ] **Step 5: Confirm heal converts, then flip the node**

With retrieval working, `healedTotal` must become non-zero and `elohim_reconcile_gaps{state="divergent"}` must fall — Phase 0's instrument is what makes this checkable. `@requires:alpha-cluster-6peer` for the live confirmation. Flip `notary-authority` in `charter.yaml` only on the habits register's own strict rule: **×2 fresh edge validations on a settled fleet**, and only with the build numbers in `evidence:`. A single green run measured during post-deploy churn is exactly the false signal that regressed scenario 2 in edge #1188.

- [ ] **Step 6: Invoke `story-harvest`** — per the root CLAUDE.md, after `systematic-debugging` identifies and fixes a root cause, harvest the parameter-bearing discoveries (timing bounds, gossip windows) into a2o regression scenarios before closing.

---

# Phase 3 — Write the reds for the three unwired nodes

Per charter covenant rule 2, an `unwired` node is **not schedulable** and its only legal first move is a runnable check. Prose specs do not advance it. Each task below writes exactly that check — failing, honest, and bounded. None of them activates its node (rule 3's WIP fence); activation is a later decision once Phases 1 and 2 free their slots.

These three are independent of each other and may be dispatched in parallel.

### Task 11: Attribution — the red for `identity-cross-signed`

The signature algebra core already exists: `elohim/elohim-storage/src/p2p/binding_cross_signature.rs` is landed, pure (`ed25519-dalek` only), red-team-reviewed, and declared as slice **C2-S1**. It is `pub mod`-declared in `p2p/mod.rs` and **referenced nowhere else in `src/`**. Meanwhile `reconcile/controller.rs:630` still writes `STAGE1_SIGNATURE_SENTINEL` into every gossiped binding, and `imagodei_integrity`'s `validate_create` checks only that the signature field is non-empty.

Note the slice decomposition (S2–S6) currently lives **only** in `.claude/shifts/2026-07-18T03-27-integrate-identity-head-c2-deliver.journal.md`. Writing the plan for those slices is captured as complementary work; this task writes the red they will turn green.

**Files:**
- Create: `elohim/elohim-storage/tests/binding_attribution_refuses_sentinel.rs`

- [ ] **Step 1: Write the failing test**

```rust
//! Charter red for `identity-cross-signed`.
//!
//! A binding carrying STAGE1_SIGNATURE_SENTINEL is SELF-ASSERTED: the libp2p
//! transport keypair and the Holochain agent key are generated independently
//! and neither signs over the other, so a gossiped spoof (agent_cid = attacker,
//! peer_id = victim) passes validation. Any economic attribution that joins
//! through such a binding credits the wrong peer.
//!
//! Backlog: genesis/data/timeline/backlog/agent-peer-binding-cross-signed-proof.md
//! Algebra core (built, unwired): src/p2p/binding_cross_signature.rs (C2-S1).

use elohim_storage::p2p::identity_binding_gossip::STAGE1_SIGNATURE_SENTINEL;

#[test]
fn a_sentinel_binding_is_refused_for_attribution_outside_dev_mode() {
    // Replace with the real admissibility predicate once it exists. The point
    // of the red is that TODAY there is no such predicate at all — the check
    // is `!signature.is_empty()`, which a sentinel satisfies.
    let admissible = elohim_storage::p2p::binding_admissible_for_attribution(
        STAGE1_SIGNATURE_SENTINEL,
        /* dev_mode */ false,
    );
    assert!(
        !admissible,
        "a self-asserted sentinel binding must never back economic attribution"
    );
}

#[test]
fn a_cross_signed_binding_is_admissible() {
    // Uses the landed C2-S1 algebra core: both Ed25519 halves verify over
    // their domain-separated canonical bytes.
    // Build the (core, proof) pair with binding_cross_signature's test helpers,
    // then assert admissibility is TRUE. This half proves the red is not
    // vacuous — it can be turned green by real cryptography, not by deleting
    // the assertion.
    todo!("construct via binding_cross_signature::{canonical_bytes, verify_binding_signatures}")
}
```

- [ ] **Step 2: Replace the `todo!` before committing**

A `todo!()` in a charter red is a placeholder, not a check — it panics and reads as a failure for the wrong reason. Read `binding_cross_signature.rs`'s test module for the existing keypair/proof construction helpers and write the positive case with them. **A red that cannot be turned green by a correct implementation is not a red.**

- [ ] **Step 3: Run it to verify it fails honestly**

```bash
cd /projects/elohim/elohim/elohim-storage && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/integ/elohim__elohim-storage/dev \
  cargo test --test binding_attribution_refuses_sentinel
```

Expected: FAIL — `cannot find function binding_admissible_for_attribution`. That absence *is* the finding.

- [ ] **Step 4: Mark it `#[ignore]` with the reason inline**

Same discipline as `sync_scale_honesty`: the storage pre-push gate runs every `tests/` target, so a standing red left in the default sweep wedges the gate for everyone. `#[ignore]` keeps it compiled (no silent rot) and leaves execution to the habits register check.

- [ ] **Step 5: Commit and register the check on the habit**

```bash
git add elohim/elohim-storage/tests/binding_attribution_refuses_sentinel.rs genesis/manifests/habits.yaml
git commit -m "test(storage): charter red for identity-cross-signed — sentinel bindings are inadmissible

Writes the runnable check the unwired node needs to become schedulable. The
C2-S1 algebra core is landed and unwired; controller.rs:630 still writes the
sentinel. Node stays unwired->red on evidence, NOT active (WIP fence)."
```

Add to `charter.yaml` under `identity-cross-signed`:
```yaml
    checks:
      - "cargo test --test binding_attribution_refuses_sentinel -- --ignored (elohim/elohim-storage)"
```
and set `status: red` **only after** running it and pasting the result into `evidence:`.

---

### Task 12: Confidentiality — the red for `reach-enforced-everywhere`

Two gaps, both empirically proven, and they are different in kind. The CRDT sync plane's `reach_is_distribution_safe` gate is **broadcast-only fail-closed** — scoped content is *excluded* from the plane rather than *delivered to authorized receivers*, which is not enforcement. And the HTTP path returns 200 to **any** authenticated caller for intimate-reach content, because `check_reach_authorization`'s intimate branch is reachable only from `handle_resolve`, which no HTTP route calls.

Per the node's `first_move`: one scenario per egress plane, each asserting a scoped-tier row does **not** reach an unauthorized peer/client. Start with these two planes.

**Live encryption stays HELD** — `KeyEnvelope`, the `ShardManifest.encryption`/`plaintext_cid` field-add, and the X25519 reader-key resolver are substrate-blocked (agent keys are ed25519; nothing sources a per-reader X25519 pubkey from `agent_cid`). Do not plan or attempt them here.

**Files:**
- Create: `genesis/a2o/features/dataplane/reach-enforcement.feature`
- Create: `elohim/elohim-storage/tests/reach_enforcement_per_plane.rs`

- [ ] **Step 1: Write the HTTP-plane red as an a2o scenario**

```gherkin
@concern:reach-enforcement
Feature: Scoped reach is enforced at every egress plane, not merely excluded

  # Backlog: genesis/data/timeline/backlog/http-reach-enforcement-gap.md
  # Proven 2026-06-04 against the provenanced love-map-adam-eve row: authed
  # James AND authed Jessica both received 200.
  Scenario: an authenticated non-beneficiary cannot read intimate-reach content over HTTP
    Given content "love-map-adam-eve" exists with reach "intimate"
    And "adam" and "eve" are its dual-consented beneficiaries
    When authenticated "james" requests GET /db/content/love-map-adam-eve
    Then the response status is 403
    And the response body does not contain the content body

  Scenario: a beneficiary CAN read the same row
    Given content "love-map-adam-eve" exists with reach "intimate"
    And "adam" and "eve" are its dual-consented beneficiaries
    When authenticated "adam" requests GET /db/content/love-map-adam-eve
    Then the response status is 200
```

The second scenario is what makes the red non-vacuous: it fails if a cure over-corrects into blanket refusal.

- [ ] **Step 2: Write the CRDT-plane red as a Rust test**

```rust
//! Charter red for `reach-enforced-everywhere`, CRDT sync plane.
//!
//! `sync::projector::reach_is_distribution_safe` is broadcast-only fail-closed:
//! scoped content is dropped from the plane entirely (`project_content_doc`
//! returns Ok(false)). That is EXCLUSION — the authorized receiver gets nothing
//! either. Enforcement means the row reaches authorized peers and no others.

#[test]
fn a_scoped_row_reaches_its_authorized_peer_and_no_other() {
    // Assert the PROPERTY, not today's mechanism, so any valid cure flips it:
    //   authorized_peer receives the row  AND  unauthorized_peer does not.
    // Today the first conjunct fails (nobody receives it) and the second
    // passes for the wrong reason. Both must be asserted or a cure that
    // simply keeps excluding everything reads as green.
    todo!("build a two-peer projector fixture; assert both conjuncts")
}
```

- [ ] **Step 3: Replace the `todo!` with a real fixture before committing**

Follow the fixture pattern in `elohim/elohim-storage/tests/sync_libp2p_convergence.rs`. Same rule as Task 11: a red that cannot be turned green by a correct implementation is not a red.

- [ ] **Step 4: Run both and confirm they fail for the stated reasons**

```bash
cd /projects/elohim/genesis/a2o && pnpm exec cucumber-js \
  features/dataplane/reach-enforcement.feature --dry-run
cd /projects/elohim/elohim/elohim-storage && \
  RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/integ/elohim__elohim-storage/dev \
  cargo test --test reach_enforcement_per_plane -- --ignored
```

Expected: the Rust red FAILS on the authorized-peer conjunct. The a2o feature is **not** added to the edge Dataplane Validation tag set in this task — adding a failing scenario to a CI-gated measure changes what that measure counts, which is the operator call already established in Task 5.

- [ ] **Step 5: Commit and register on the habit**

```bash
git add genesis/a2o/features/dataplane/reach-enforcement.feature elohim/elohim-storage/tests/reach_enforcement_per_plane.rs genesis/manifests/habits.yaml
git commit -m "test: charter reds for reach-enforced-everywhere (HTTP + CRDT planes)

HTTP: intimate content returns 200 to any authenticated caller (reach gate is
P2P-resolve-only). CRDT: scoped rows are excluded from the plane entirely,
which is exclusion, not enforcement. Live encryption stays HELD (X25519
reader-key substrate unbuilt). Node stays NOT active (WIP fence)."
```

**Prerequisite flagged, not absorbed:** the node's own `refs:` warn that three reach vocabularies are in drift and unification comes first (`project_reach_enum_drift_reconciliation`; slices 1–3 planned at `2026-07-23-reach-vocab-slice{1,2,3}-*.md`). Writing the red does not depend on that; *curing* it does. Sequence the vocab slices before the enforcement implementation.

---

### Task 13: Actuation — the red for `operator-runtime-surface`

The operator plane is kubectl-only (`restart-doorway-epr.sh`), which also produces the adam-invisibility class: peers are unobservable except through our cluster's Loki. The node's `first_move` names the shape precisely — an operator holding a `delegates-compute` commitment requests a restart/backfill via the doorway; the peer executes and attests; a caller without the commitment is refused. This is the REA compute-commitment primitive's next instance (`project_rea_compute_commitment_primitive`: it displaces X-API-Key grants).

The design spec exists at `Draft` (`2026-07-05-operator-surface-observe-act-design.md`) and there is a built-but-undeployed program at `2026-06-13-self-healing-control-plane-design.md` (also `Draft`; per `project_self_healing_control_plane_vision`, plans A–D are **built** on `shift/self-healing-control-plane` and not deployed). Read both before writing the scenario — the red should target the surface those specs describe, not a fresh invention.

**Files:**
- Create: `genesis/a2o/features/dataplane/operator-commitment-gated-verbs.feature`

- [ ] **Step 1: Read the two specs and the built branch**

```bash
cd /projects/elohim && git log --oneline origin/shift/self-healing-control-plane -15
```

Determine what is already built. If the commitment-gated verb path exists there, the red targets it and the node's real gap is *deployment*, not construction — that is a materially different finding and must be recorded before writing code.

- [ ] **Step 2: Write the scenario**

```gherkin
@concern:operator-runtime-surface
Feature: Operator verbs are commitment-gated protocol actions, not cluster surgery

  # Kills the kubectl-only class (restart-doorway-epr.sh) and the
  # adam-invisibility class (peers observable only via our cluster's Loki).
  # Spec: genesis/docs/superpowers/specs/2026-07-05-operator-surface-observe-act-design.md

  Scenario: a commitment holder can request a reconcile and receives an attestation
    Given "matthew" holds a "delegates-compute" commitment over peer "adam"
    When "matthew" requests a reconcile on "adam" through the doorway
    Then the request is accepted
    And "adam" executes the reconcile
    And "adam" emits an attestation naming the commitment it acted under

  Scenario: a caller without the commitment is refused
    Given "james" holds no "delegates-compute" commitment over peer "adam"
    When "james" requests a reconcile on "adam" through the doorway
    Then the request is refused
    And "adam" performs no reconcile

  Scenario: a peer serves its own telemetry
    # The symmetry gap: adam has zero Loki streams today, so it is legible
    # only through our cluster. A peer must answer for itself.
    When "adam" is asked for its runtime status directly
    Then "adam" answers with its own sync, reconcile, and peer counts
    And the answer does not traverse the operator's observability cluster
```

- [ ] **Step 3: Verify it parses and identify undefined steps**

```bash
cd /projects/elohim/genesis/a2o && pnpm exec cucumber-js \
  features/dataplane/operator-commitment-gated-verbs.feature --dry-run
```

Expected: undefined steps — that list **is** the implementation surface. Record it in the commit message.

- [ ] **Step 4: Do NOT add it to the CI tag set**

Same operator-call discipline as Tasks 5 and 12.

- [ ] **Step 5: Commit and register on the habit**

```bash
git add genesis/a2o/features/dataplane/operator-commitment-gated-verbs.feature genesis/manifests/habits.yaml
git commit -m "test(a2o): charter red for operator-runtime-surface — commitment-gated verbs

Three scenarios per the node's first_move: commitment holder acts and the peer
attests; non-holder is refused; a peer serves its own telemetry (the adam-
invisibility symmetry gap). Node stays NOT active (WIP fence)."
```

---

## Sequencing summary

```
Phase 0 ──────────────────────────────────► the instrument
  T1 converged!=caught_up ─► T2 status/schema ─► T3 metrics
                                    └─► T4 /health staleness ─► T5 a2o gate (operator)
                                                                      └─► T6 deploy + read

Phase 1 (needs T3+T6 to be measurable)
  T7 handler  ──[deploy]──►  T8 opener  ──►  T9 announce send site  ──► sync-scale-honesty GREEN

Phase 2 (independent of Phase 1; needs T1-T3 to confirm the cure)
  T10 systematic-debugging on the isolated sweettest ──► notary-authority GREEN (x2 settled)

Phase 3 (parallel; needs nothing above — writes reds only, activates nothing)
  T11 attribution red   T12 reach red   T13 operator red
```

**What "done" means for this plan:** `sync-scale-honesty` and `notary-authority` are `green` in `charter.yaml` with build numbers and measured values in `evidence:`, and `identity-cross-signed`, `reach-enforced-everywhere`, and `operator-runtime-surface` have each moved `unwired → red` with a runnable check registered. At that point every one of the six properties is either delivered or **schedulable**, which is the precondition for offering any of them as an SDK promise.

## Self-review notes

- **Spec coverage.** All six gaps map to tasks. Gap 1's three legs: acquisition silence (T6 verify — the commit is landed), sync-plane metrics (T3 + T6 verify), honest convergence (T1–T2, the unfixed leg), plus the `/health` 12× divergence (T4) and the a2o gate (T5). Gap 2 → T7–T9. Gap 3 → T10. Gaps 4/5/6 → T11/T12/T13. Gap 5's encryption half is explicitly HELD with the blocking reason named, not silently dropped.
- **Known placeholders, deliberately marked.** T11 Step 1 and T12 Step 2 contain `todo!()` in illustrative test bodies, each with an explicit follow-on step requiring replacement before commit and a stated reason (the real fixture must be built from the existing helpers, which the implementer must read). T3's `observe_reconcile_sweep` body is described rather than written because it must match `metrics.rs`'s existing registration pattern, which is the more reliable instruction than a guessed body.
- **Type consistency.** `GapCounts::{exhausted, converged}` (T1) is consumed unchanged by T2 and T3. `converged` on `ProjectionReconcileStatus` (T2) is consumed by T4's `P2pHealth`. `corpus_digest` and `SyncRequest::ListDocumentsSince` (T7) are consumed by T8. `announcements_for_local_change`'s signature is unchanged from the existing stub, so T9 is a body swap, not an interface change.
