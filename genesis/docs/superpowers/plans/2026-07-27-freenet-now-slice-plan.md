---
title: "Freenet NOW slice — close sync-scale-honesty + the zero-dependency guards"
id: freenet-now-slice
tier: plan
status: Draft
created: 2026-07-27
maintainers: Matthew Dowell + Claude Opus 5
domain: D5
sprint: spine-red (sync-scale-honesty) — not a ranked §1 sprint; scoped to not displace the verify track
requires_env: []
topic: [freenet, sync-scale-honesty, anti-entropy, dead-config, spine-red, d5-dataplane]
cites:
  - freenet-lift-and-shift | Freenet lift-and-shift | sha256:d6a221d95b723d32 | path: genesis/docs/superpowers/plans/2026-07-27-freenet-lift-and-shift-plan.md
  - genesis/research/freenet-peer-confrontation-2026-07-27.md
  - genesis/manifests/habits.yaml
  - iroh-libp2p-complementarity | iroh ↔ libp2p Complementarity | sha256:29235aeb35aff128 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-08-iroh-libp2p-complementarity.md
  - adam-slow-link-write-guard-saturation | History/Finding: adam slow-link melt | sha256:556142ddd510a091 | path: genesis/docs/content/elohim-protocol/history/2026-07-20-adam-slow-link-write-guard-saturation.md
---

# Freenet NOW slice — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the standing-red habit `sync-scale-honesty` by making the sync round opener a function of local corpus state, and land the three zero-dependency guards that stop the same class of defect recurring.

**Architecture:** The cure is a body change to one pure planner function. Everything it needs already exists — `corpus_digest()` (sorted, canonical), the `ListDocumentsSince` request variant, the `InSync` response variant, and digest-match short-circuit handlers on **both** transports. The one genuine gap is client-side: `SyncResponse::InSync` is constructed by both responders but has no explicit client arm, so it currently falls into a catch-all that logs "Unhandled sync response type." We make that arm explicit and counted *before* flipping the opener, so the optimisation is observable rather than silent.

**Tech Stack:** Rust, `elohim-storage` (native build), libp2p + iroh dual transport, Prometheus metrics, `cargo test`.

## Global Constraints

- **Build env:** `elohim-storage` keeps the ambient `RUSTFLAGS='--cfg getrandom_backend="custom"'` (do NOT clear it — that rule is for `doorway-service` and `steward/node` only).
- **Target dir (mandatory):** `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev`. A native build without it is denied by the disk-guard hook.
- **`p2p/sync_round.rs` is the ONLY legal construction site** for the round opener and the announce requests. Its module doc states this; making a planner return requests with no caller that sends them is the exact fake-green the extraction exists to prevent.
- **Both transports must stay in step.** Any wire-shape change lands in `src/p2p/mod.rs` (libp2p) *and* `src/p2p_iroh/sync_backend.rs` (iroh).
- **Telemetry cardinality:** Prometheus label values come from a bounded set. Never label with peer ids, doc ids, or error strings.
- Work lands in `/projects/elohim` directly (no sibling worktrees). Commit only — do not push or merge to `dev`.

---

## Ground truth verified 2026-07-27 (do not re-derive)

`cargo test --test sync_scale_honesty -- --ignored` → **1 passed, 1 failed.**

- `a_local_change_is_announced_to_connected_peers` — **PASSES.** The announce leg is already cured; the send site exists at `src/p2p/mod.rs:3612-3618`.
- `the_round_opener_reflects_what_we_already_have` — **FAILS** at `tests/sync_scale_honesty.rs:99`. This is the whole remaining red.

**The habit's evidence block is stale.** It records `0 passed, 2 failed` (2026-07-25) and asserts *"`announcements_for_local_change` returns nothing."* Both statements are now false. Task 3 corrects it.

Already present, needing no new code:
- `sync_round::corpus_digest(&LocalCorpusState) -> String` (`sync_round.rs:122`) — sorts doc-ids and heads before hashing, so two peers holding the same corpus produce the same digest regardless of store enumeration order. The canonical-serialization hazard is already handled.
- `SyncRequest::ListDocumentsSince { h_app_id, prefix, corpus_digest, limit }` (`sync_protocol.rs:115`).
- Digest-match handlers on both transports (`mod.rs:6656`, `p2p_iroh/sync_backend.rs:222`) that answer `InSync` and enumerate nothing, falling back to full enumeration on mismatch.
- `round_opener` already receives `&local_state` at its call site (`mod.rs:7519`); the signature does not change.

---

## File Structure

| File | Responsibility | Change |
|---|---|---|
| `src/p2p/mod.rs` | libp2p client response handling | Add explicit `SyncResponse::InSync` arm (Task 1) |
| `src/p2p_iroh/sync_backend.rs` | iroh client response handling | Same arm, if the iroh client path matches responses (Task 1) |
| `src/metrics.rs` | Prometheus counters | Add `inc_sync_in_sync()` (Task 1) |
| `src/p2p/sync_round.rs` | The pure round planner — sole construction site | Change `round_opener` body (Task 2); add module-doc retention rule (Task 6) |
| `src/reconcile/placement.rs` | Placement strategy | Add module-doc retention rule (Task 6) |
| `tests/sync_scale_honesty.rs` | The spine red | No change — it is the acceptance test |
| `genesis/manifests/habits.yaml` | Delivery habits | Flip node + evidence (Task 3) |
| `scripts/ci/dead-config-lint.sh` | New CI guard | Create (Task 5) |

---

### Task 1: Make `InSync` an explicit, counted client outcome

Today `SyncResponse::InSync` reaches the client and falls into `handle_sync_response`'s catch-all (`src/p2p/mod.rs:6922`), logging "Unhandled sync response type." Behaviour is accidentally correct (in-sync ⇒ nothing to do) but invisible. We make it intentional and countable first, so Task 2's flip is observable on day one.

**Files:**
- Modify: `src/p2p/mod.rs` (the `handle_sync_response` match, before the `_ =>` arm at ~6922)
- Modify: `src/metrics.rs`
- Modify: `src/p2p_iroh/sync_backend.rs` (only if its client path matches `SyncResponse`; verify first)
- Test: `src/p2p/sync_round.rs` `#[cfg(test)] mod tests` (counter is unit-testable there) and manual log verification

**Interfaces:**
- Produces: `crate::metrics::inc_sync_in_sync()` — no args, no labels. Task 4 reads the counter it registers.

- [ ] **Step 1: Confirm the iroh client path before editing**

```bash
cd /projects/elohim/elohim/elohim-storage
grep -n "handle_sync_response\|match response" src/p2p_iroh/sync_backend.rs | head
```

If `sync_backend.rs` has no client-side `SyncResponse` match (it may only serve), skip its edit and note that in the commit message. Do not invent a handler.

- [ ] **Step 2: Add the metric**

In `src/metrics.rs`, following the existing counter pattern in that file (find `inc_sync_round` and mirror it exactly — same registry, same naming convention):

```rust
/// Counts sync rounds that short-circuited on a matching corpus digest —
/// the converged steady state where the opener costs O(1) instead of O(corpus).
/// A flat-zero value after the digest opener lands means the shortcut never
/// fires and the optimisation is inert.
pub fn inc_sync_in_sync() {
    SYNC_IN_SYNC_TOTAL.inc();
}
```

Register `SYNC_IN_SYNC_TOTAL` beside the existing `sync_round` counter, named `elohim_sync_in_sync_total`. No labels.

- [ ] **Step 3: Add the explicit client arm**

In `src/p2p/mod.rs`, immediately **before** the `_ => { debug!(... "Unhandled sync response type") }` arm:

```rust
SyncResponse::InSync {
    h_app_id,
    corpus_digest,
} => {
    // Digest-equal steady state: the peer enumerated nothing and there is
    // nothing to apply. This arm exists so the shortcut is INTENTIONAL and
    // counted — before it, InSync fell into the catch-all and read as
    // "Unhandled sync response type", which is how an inert optimisation
    // hides. The page cursor was already reclaimed at the top of this fn.
    crate::metrics::inc_sync_in_sync();
    debug!(
        peer = %peer,
        h_app_id = %h_app_id,
        digest = %corpus_digest,
        "Sync round InSync — peer holds the same corpus, nothing enumerated"
    );
}
```

- [ ] **Step 4: Build and verify no warnings**

```bash
cd /projects/elohim/elohim/elohim-storage
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo build 2>&1 | tail -20
```

Expected: builds clean. If the compiler now reports the `_ =>` arm as unreachable, that means `SyncResponse` has no other unhandled variants — leave the catch-all in place anyway (it guards future variants) unless the compiler errors.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim
git add elohim/elohim-storage/src/p2p/mod.rs elohim/elohim-storage/src/metrics.rs
git commit -m "feat(sync): make InSync an explicit counted client outcome

InSync was constructed by both transports but had no client arm, so it fell
into the catch-all logging 'Unhandled sync response type'. Explicit arm +
elohim_sync_in_sync_total so the digest short-circuit is observable before
the opener starts using it.

Serves habit sync-scale-honesty."
```

---

### Task 2: Make the round opener a function of local state (closes the habits register red)

**Files:**
- Modify: `src/p2p/sync_round.rs:150-157` (`round_opener` body only — signature unchanged)
- Test: `tests/sync_scale_honesty.rs::the_round_opener_reflects_what_we_already_have` (already written and red)

**Interfaces:**
- Consumes: `sync_round::corpus_digest(&LocalCorpusState) -> String` (already exists, `sync_round.rs:122`), `crate::metrics::inc_sync_in_sync()` from Task 1.
- Produces: `round_opener` now emits `SyncRequest::ListDocumentsSince`. Both transports already handle it.

- [ ] **Step 1: Run the test to confirm it is red**

```bash
cd /projects/elohim/elohim/elohim-storage
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo test --test sync_scale_honesty -- --ignored 2>&1 | tail -12
```

Expected: `1 passed; 1 failed`, with `the_round_opener_reflects_what_we_already_have` FAILED at `tests/sync_scale_honesty.rs:99`.

- [ ] **Step 2: Change the opener body**

Replace `round_opener` in `src/p2p/sync_round.rs` (keep the signature; rename `_local` → `local`):

```rust
/// The single request that opens a sync round with one peer.
///
/// Carries a digest of what we already hold, so a converged peer answers
/// `InSync` with one hash instead of enumerating its whole corpus. Divergent
/// peers fall through to exactly the previous `ListDocuments` path, so
/// correctness in the divergent case is unchanged — only the CONVERGED steady
/// state gets cheap, which is where a healthy mesh spends nearly all its time.
///
/// `limit` is the page size the responder uses IF the digests differ; it must
/// stay `SYNC_LIST_PAGE_LIMIT` so the fallback paginates exactly as before.
pub fn round_opener(h_app_id: &str, local: &LocalCorpusState) -> SyncRequest {
    SyncRequest::ListDocumentsSince {
        h_app_id: h_app_id.to_string(),
        prefix: None,
        corpus_digest: corpus_digest(local),
        limit: SYNC_LIST_PAGE_LIMIT,
    }
}
```

- [ ] **Step 3: Run the test to verify it passes**

```bash
cd /projects/elohim/elohim/elohim-storage
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo test --test sync_scale_honesty -- --ignored 2>&1 | tail -12
```

Expected: `2 passed; 0 failed`.

- [ ] **Step 4: Run the full crate test suite — the divergent path must not regress**

```bash
cd /projects/elohim/elohim/elohim-storage
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo test 2>&1 | tail -25
```

Expected: no new failures. Pay specific attention to `tests/sync_libp2p_convergence.rs` — it re-implements the round in the test file, so it exercises the mirror rather than the wire and may need its opener updated to match. If it fails, update *that test's* opener to call `sync_round::round_opener`, never to hand-roll a second construction site.

- [ ] **Step 5: Lint and format**

```bash
cd /projects/elohim/elohim/elohim-storage
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo clippy -- -D warnings 2>&1 | tail -15
cargo fmt --check
```

- [ ] **Step 6: Commit**

```bash
cd /projects/elohim
git add elohim/elohim-storage/src/p2p/sync_round.rs
git commit -m "fix(sync): open rounds with a corpus digest, not a stateless enumeration

round_opener ignored local state, so a node holding the whole corpus and a
node holding nothing emitted a byte-identical opener and every peer
re-enumerated its whole corpus every 60s tick, converged or not — steady-state
O(peers x corpus) instead of O(changes).

Now emits ListDocumentsSince{corpus_digest}. Both transports already
short-circuit on digest match and fall back to full enumeration on mismatch,
so the divergent path is unchanged.

Closes the head-diff half of habit sync-scale-honesty.
tests/sync_scale_honesty: 2 passed."
```

---

### Task 3: Flip the habit and correct its stale evidence

**Files:**
- Modify: `genesis/manifests/habits.yaml` (the `sync-scale-honesty` node, ~line 277)

- [ ] **Step 1: Verify with a clean run and capture the output verbatim**

```bash
cd /projects/elohim/elohim/elohim-storage
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo test --test sync_scale_honesty -- --ignored 2>&1 | tail -6
```

- [ ] **Step 2: Update the node**

Set `status: green`, `active: false`. Replace the evidence block, and explicitly record the stale-evidence correction so the next reader is not misled:

```yaml
    evidence: >
      GREEN 2026-07-27: `cargo test --test sync_scale_honesty -- --ignored` →
      2 passed, 0 failed. Cure = round_opener now emits
      ListDocumentsSince{corpus_digest} (p2p/sync_round.rs), so a converged
      peer answers InSync with one hash and enumerates nothing; divergent
      peers fall through to the unchanged ListDocuments path. InSync is an
      explicit counted client arm (elohim_sync_in_sync_total) rather than a
      catch-all fallthrough.
      CORRECTION to the prior evidence block (written 2026-07-25, "0 passed,
      2 failed"): the announce leg was already cured before this work — the
      send site exists at p2p/mod.rs:3612 and
      a_local_change_is_announced_to_connected_peers was ALREADY passing on
      2026-07-27. Only the head-diff half was outstanding.
    guard: >
      Regression risk = a second construction site for the opener. p2p/sync_round.rs
      must remain the ONLY constructor; tests/sync_libp2p_convergence.rs must call
      round_opener rather than hand-rolling a mirror, or the test measures the
      mirror instead of the wire. Watch elohim_sync_in_sync_total: a flat zero in
      a converged mesh means the shortcut never fires and the cure is inert.
```

- [ ] **Step 3: Commit**

```bash
cd /projects/elohim
git add genesis/manifests/habits.yaml
git commit -m "spine(sync-scale-honesty): red -> green with evidence; correct stale evidence block"
```

---

### Task 4: Measure the three anti-entropy loops (the one query)

We run three concurrent, unbudgeted anti-entropy loops — kitsune2 (120–300s), inventory gossip (60s), Automerge doc-sync (60s) — where Freenet's *single* 300s loop measures 53.7% of their egress. This is not hypothetical for us: the 2026-07-20 adam melt was a gossip storm transmitting link quality into `holochain_sqlite` write-lock contention. No aggregate bandwidth accounting exists.

**Files:**
- Create: `genesis/data/timeline/backlog/2026-07-27-anti-entropy-egress-baseline.md`

- [ ] **Step 1: Confirm the metrics endpoint and enumerate available sync counters**

```bash
grep -n '"/metrics"' /projects/elohim/elohim/elohim-storage/src/http.rs
grep -n "pub fn inc_\|register_" /projects/elohim/elohim/elohim-storage/src/metrics.rs | head -30
```

- [ ] **Step 2: Query the household nodes**

Use the observability MCP (`mcp__observability__query_prometheus`). Load it first:

```
ToolSearch "select:query_prometheus,list_prometheus_metric_names"
```

Query per-node counter rates for the sync/gossip families over 24h. If a loop has **no counter at all**, that absence is itself the finding — record it rather than inventing a number.

- [ ] **Step 3: Write the baseline with dated numbers, or an honest gap list**

Record: per-loop egress (or "unmeasured — no counter"), the date, the nodes queried, and which of the three loops are instrumented. Cite the adam melt as the incident this baseline exists to prevent recurring. Do **not** extrapolate to a percentage-of-egress figure unless total egress is actually measured — a retry-masked or partial denominator is exactly the metric trap the survey names.

- [ ] **Step 4: Commit**

```bash
cd /projects/elohim
git add genesis/data/timeline/backlog/2026-07-27-anti-entropy-egress-baseline.md
git commit -m "measure(sync): anti-entropy egress baseline across the three loops"
```

---

### Task 5: Dead-config lint — the guard that would have caught this

`ListDocumentsSince` and `AnnounceChange` were both fully implemented on the receive side and never constructed. That is the same class as `enable_eviction: true` with zero readers, `max_storage_bytes` with zero write-path callers, `salvage_capacity_enabled` in zero manifests, and Freenet's own `enable_metering: false` beneath a paper claiming fuel limits. **Dead configuration that reads as shipped capability** is the dominant shared failure mode in both projects.

**Files:**
- Create: `scripts/ci/dead-config-lint.sh`
- Modify: the CI pipeline that runs storage gates (add a call; find it via `grep -rn "cargo clippy" Jenkinsfile elohim/holochain/Jenkinsfile scripts/ci/`)

**Interfaces:**
- Produces: exit 1 with a named list when a tracked symbol has no reader.

- [ ] **Step 1: Write the lint with a seeded watchlist**

Start with an explicit watchlist rather than whole-tree inference — a general "unused constant" analysis is a research project; a named watchlist is shippable today and catches the recurrence.

```bash
#!/usr/bin/env bash
# dead-config-lint.sh — fail when a declared knob has no reader.
#
# WHY: ListDocumentsSince and AnnounceChange were fully handled on the receive
# side and never constructed; the sync plane paid O(peers x corpus) for months
# because a wired-looking mechanism was inert. Same class as enable_eviction:true
# with zero readers. A knob with no reader is not a feature, it is a claim.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fail=0

# symbol : directory to search for a READER (construction / consumption site)
check() {
  local sym="$1" dir="$2" note="$3"
  local hits
  hits=$(grep -rn --include=*.rs "$sym" "$ROOT/$dir" 2>/dev/null \
         | grep -v "^.*sync_protocol.rs" | grep -vc "^$" || true)
  if [ "${hits:-0}" -eq 0 ]; then
    echo "DEAD-CONFIG: $sym has no reader in $dir — $note"
    fail=1
  fi
}

check "SyncRequest::ListDocumentsSince" "elohim/elohim-storage/src/p2p/sync_round.rs" \
      "the round opener must construct it (spine sync-scale-honesty)"
check "inc_sync_in_sync" "elohim/elohim-storage/src/p2p" \
      "the InSync client arm must count the shortcut"

exit $fail
```

- [ ] **Step 2: Run it and verify it passes after Tasks 1–2**

```bash
bash /projects/elohim/scripts/ci/dead-config-lint.sh
echo "exit=$?"
```

Expected: exit 0.

- [ ] **Step 3: Verify it actually fails when it should**

```bash
cd /projects/elohim
git stash push elohim/elohim-storage/src/p2p/sync_round.rs
bash scripts/ci/dead-config-lint.sh; echo "exit=$?"   # expect exit=1, DEAD-CONFIG line
git stash pop
```

A lint that has never been seen to fail is itself unverified.

- [ ] **Step 4: Wire into CI and commit**

```bash
cd /projects/elohim
git add scripts/ci/dead-config-lint.sh
git commit -m "ci: dead-config lint — a knob with no reader is a claim, not a feature"
```

---

### Task 6: Write the two retention rules into the modules they govern

Both are one-paragraph rules that prevent a class of bug we have not written yet. Freenet demoted their own demand estimator out of the eviction sort with a source comment saying "Do NOT re-wire this estimate back into the eviction sort" — the rule lives where the code is, not in a doc nobody opens.

**Files:**
- Modify: `src/reconcile/placement.rs` (module doc)
- Modify: `src/p2p/sync_round.rs` (module doc)

- [ ] **Step 1: Add the placement/retention separation rule**

At the top of `src/reconcile/placement.rs`:

```rust
//! # XOR distance is a PLACEMENT input, never a RETENTION input.
//!
//! Distance decides *where a new copy goes*. It must never enter an eviction or
//! retention ranking. Prior art (Freenet's demand-driven hosting design, surveyed
//! 2026-07-27): distance's causal pull on demand already flows through subscriber
//! count via routing gravity, so ranking on both double-counts it — and locality
//! is delivered by routing, not by the retention decision. They demoted their own
//! distance-derived demand estimator to telemetry-only for exactly this reason.
//!
//! Our arc factor IS a distance input. Keep it on the authority plane; do not let
//! it become the storage plane's retention rule.
```

- [ ] **Step 2: Add the heal-exemption rule**

At the top of `src/p2p/sync_round.rs`, appended to the existing module doc:

```rust
//! # Gate growth, never convergence.
//!
//! When a budget or admission gate lands on the storage plane, the HEAL path
//! must be exempt from it. Freenet #4868: an over-budget peer permanently
//! diverges because the growth UPDATE *and* its ResyncResponse heal hit the same
//! admission gate, so no convergence path exists while over budget. A node that
//! cannot heal is worse than a node that is over budget.
```

- [ ] **Step 3: Verify the crate still builds and commit**

```bash
cd /projects/elohim/elohim/elohim-storage
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo build 2>&1 | tail -5
cd /projects/elohim
git add elohim/elohim-storage/src/reconcile/placement.rs elohim/elohim-storage/src/p2p/sync_round.rs
git commit -m "docs(storage): retention/placement separation + heal-exemption rules

Two rules lifted from the Freenet survey, written where the code is rather than
in a doc nobody opens."
```

---

## Explicitly OUT of scope for this slice

Deferred with reason, so nobody widens this plan:

- **Phase 3 (capability-relative budget + eviction)** — blocked on Task 4's measurement; a budget that binds before allocator behaviour is understood masks rather than fixes (our own jemalloc verdict: flat ~2.7 GB vs glibc's 8–8.5 GB OOM). Sprint-sized.
- **Phase 4 (durability floor / breach detection)** — sprint-sized, and its REA leg overlaps the roadmap's **verify track** (dwelling-hub CLAIMED-ONLY items). It must *compose* with that verification, not fork it.
- **Phase 5 (reach negotiation ceremony)** — design-sized, domain **D8**, and the MAP is explicit that "doorway is OPTIONAL, not architectural." Belongs after the verify track.
- **Root `LICENSE` (Phase 0.1)** — an operator decision, not an implementation task.
- **`alpha-cluster-6peer`** is degraded; nothing here depends on it. All six tasks run on `household-nodes`.

**Roadmap non-displacement:** the roadmap's ranked highest-leverage move is the verify track (dwelling-hub, 77 plan-steps, zero new code), then Sprint 1. This slice is justified *only* because `sync-scale-honesty` is a standing spine red and the session contract is to move reds green with evidence. It is deliberately small enough not to compete.

## Self-review notes

- **Spec coverage:** covers lift-and-shift Phases 2 (Tasks 1–3), 1.1 (Task 4), 0.4 (Task 5), 0.2 (Task 6). Phases 3/4/5 and 0.1 explicitly deferred above.
- **Type consistency:** `corpus_digest(&LocalCorpusState) -> String` is used identically in Task 2 and the existing handlers; `inc_sync_in_sync()` is defined in Task 1 Step 2 and consumed in Task 1 Step 3 and Task 5's watchlist.
- **Phase 2.3/2.4/2.5 of the parent plan** (keep the summary O(1); don't naively fill `bloom_filter`; canonical serialization) need no task: `corpus_digest` is already one sorted hash per namespace, and `bloom_filter` stays `None` and untouched here. The parent plan's warnings stand for whoever revisits them.
