# Dataplane — Arc & Conductor-Memory Topology Plan (P-ARC)

> ## ⚠ LEAK-GATE RESOLVED — 2026-06-19 (the {0,1} actuation work + corpus-scaling design STAND)
> The "Hard gate on (iii)" leak-vs-bounded-large discriminator is RESOLVED: the alpha OOM was a native
> glibc-malloc arena leak, arc-INDEPENDENT (arc=0 leaked the same shape), CURED by glibc→jemalloc — arc-shrink
> does NOT "shrink the structure that leaks." The corpus-off-DHT spike should no longer be gated on a
> leak-confirm (the leak is gone); judge (iii) purely as a corpus-scaling decision. The {0,1} REA-grant
> actuation work and the fractional-arc infeasibility finding are unaffected and stand.
> Truth: .claude/data/conductor-leak-jemalloc-cure-verdict-2026-06-19.md · conductor-leak-rca-native-heap-reframe-2026-06-18.md


> Working draft. NOT cite-sealed. For agentic workers: REQUIRED SUB-SKILL: superpowers:subagent-driven-development (or executing-plans). Steps use checkbox (- [ ]) syntax.
>
> Track id in the 2026-06-14 P2P-Dataplane Contract Ledger: **P-ARC** (Wave 2, HARD on P-ACTUATION).

---

## 1. CONTEXT / WHY + FINDINGS CLOSED

The conductor authority-arc is the protocol's per-node corpus-memory governor. At `network.target_arc_factor = 1` (the deployed default, set nowhere) **each node holds a DHT working set ∝ the whole corpus**, so per-node RAM ∝ total corpus and OOM-flaps the lean archetypes (james, 3654 docs, chromebook-edu floor, OOM'd 3Gi every ~9 min). The {0,1} REA-grant-bounded actuation path (option i) is **already built** on the current `feat` tree (`arc_policy.rs` derive + coverage clamp + james-elevate, `arc_actuator.rs` authorize/render/apply-with-restart, `system_metrics::container_memory_limit_bytes`, the `GET/POST /api/v1/status/arc-policy` handlers, the `sets-authority-arc` DNA commitment action). This plan does **not** re-build option (i).

**Findings this plan closes (from the review's 7-finding spine, arc track):**

- **#1 (CONFIRMED) — fractional lever does not exist.** `target_arc_factor` is `u32`, hard-clamped `{0,1}` (`apply_arc_factor`: `factor>1 -> ERROR LOG + forced to 1`, "multi-factor sharding isn't yet implemented"; `arc-factor-feasibility-findings.md:39-44`). `arc_policy::derive` returns `f64 arc_factor` as a **signal only**; `arc_actuator::authorize` REFUSES non-`{0,1}` as `NotActuatable` (`arc_actuator.rs:115-124`). **DECISION FORCED** (see Decision Memo §A). This plan does NOT add a fractional actuator — it is spike-verified infeasible on 0.3.2/0.4.1 without forking `holochain_p2p`.
- **#7c (CONFIRMED thinnest) — arc coverage-floor multi-node invariant.** No proptest / multi-node placement test exists; `resilience-dimensions §D4` is a felt SCORE (`compute_regional_distribution`), not a placement proof. **Per RESOLUTION-H this test is OWNED by P-PROOFS** (`tests/arc_coverage_multinode.rs`); P-ARC DROPS its file and supplies P-PROOFS the production type shapes (`ArcDecision`/`derive` — verify-only). This plan therefore closes #7c only by exporting the shapes, not by authoring the test file.
- **NEW (CONFIRMED) — the `L` term is wired to 0.** `http.rs:2764` passes `local_authored_bytes: 0` with comment "L (own authored share) is not yet split out of the corpus stat … slightly optimistic on a_mem." `a_mem` is therefore optimistic for **exactly the james OOM case** (largest authored `L`). Fixing L is the highest-leverage S item. (Closed by Task 2.)
- **NEW (CONFIRMED) — restart-stagger asserted but unimplemented.** `apply_arc_actuation` restarts ONLY the local conductor (`arc_actuator.rs:345-379`; doc-comment line ~340 explicitly defers staggering to "the caller"). On a multi-node mesh, concurrent leecher-flips can transiently breach the coverage floor during the `DhtArc::Empty`→gossip reconvergence window (`core_space.rs:451`). (Closed by Task 3 — a node-local stagger gate; cross-mesh negotiation is a FOLLOW-ON seam.)
- **NEW (CONFIRMED) — version + leecher skew, surfaced not fixed.** edgenode = kitsune2 0.3.2; steward (tauri) = kitsune2 0.4.1, which hard-sets `target_arc_factor = 0` (`steward .../lib.rs:140`). Mobile is a leecher by default and never appears in the alpha coverage gate. This plan only DOCUMENTS the skew in the spike doc (Task 5); reconciling versions is out of scope.

**The durable answer (option iii) is corpus-off-DHT.** The auto-policy spec's through-line (`2026-06-13-conductor-authority-arc-auto-policy.md:18-26`) is "arc-shrink = tiered-quilt sharding applied to the conductor DHT working set," which genuinely bounds per-node RAM ∝ C/N (spec §4 math, lines 85-87) AND keeps lean devices full participants (they hold quilt shards, not a leecher's nothing). The RS quilt (`sharding.rs`, rs-4-7, galois_8) shards **blob bytes**; the conductor DHT still holds the corpus authority arc independently — **there is no code that diverts corpus off the DHT plane into the quilt byte-plane.** This is a design spike (Task 5), GATED on the operator's leak-vs-bounded confirm, not a sprint.

---

## 2. OWNED FILES (verbatim from the Contract Ledger file-ownership map)

This plan creates/mutates EXACTLY:

- **M** `elohim/elohim-storage/src/services/arc_policy.rs` (L-term: consume a real `local_authored_bytes`; tune nothing in `derive`'s shape)
- **M** `elohim/elohim-storage/src/services/system_metrics.rs` (new `local_authored_bytes()` reader)
- **M** `elohim/elohim-storage/src/conductor/process_manager.rs` (restart-stagger gate)
- **C** `genesis/docs/superpowers/specs/2026-06-14-corpus-off-dht-spike.md` (option-iii design spike)

> **Ledger drift correction (SEAM-DELTA, see return):** the ledger lists the reader file as `elohim/elohim-storage/src/system_metrics.rs`. The file is actually at `elohim/elohim-storage/src/services/system_metrics.rs` (verified: `container_memory_limit_bytes` at `services/system_metrics.rs:130`). This plan owns the correct path.

**Explicit non-touch (RESOLUTION-A, RESOLUTION-H):**
- This plan does **NOT** touch `arc_actuator.rs` — P-ACTUATION is its sole mutator (it lifts the decision core into `services/actuation/arc.rs` and reduces `arc_actuator.rs` to a thin shim). P-ARC's "refactor arc to an instance" item is DELEGATED to P-ACTUATION (arc is S2's first instance).
- This plan does **NOT** create `tests/arc_coverage_*.rs` — P-PROOFS owns `tests/arc_coverage_multinode.rs` (RESOLUTION-H). P-ARC's only test deliverable is the verify-only export of `ArcDecision`/`derive` shapes (which already exist and are `pub`).
- This plan does **NOT** touch `http.rs` beyond what flows through the `derive` call site — and even the http.rs `local_authored_bytes: 0` literal is changed by ARC ONLY at the value level via the new reader (no http.rs handler rewrite). **CLARIFY:** the one literal at `http.rs:2764` IS a P-ARC edit (it's the `derive` input wiring, owned by no other plan); flagged here so the integrator sees it. It is additive (swap `0` for `system_metrics::local_authored_bytes(...)`).

**Collision statement:** Beyond the single additive `http.rs:2764` input-wiring line (owned by no other plan — RESOLUTION-F confirms P-DIAGNOSTIC does NOT touch http.rs), this plan touches no file owned by another plan. `config.rs` is NOT touched (arc reads `system_metrics`, not config — RESOLUTION-C). `arc_actuator.rs` and the arc test file are explicitly ceded.

---

## 3. PRIMITIVES — OWNED vs CONSUMED

### OWNED by P-ARC
- **None new in `elohim-compute`.** P-ARC owns no shared single-owner primitive. Its only new symbol is `system_metrics::local_authored_bytes(...)` — a crate-private storage reader, not a shared type.
- It OWNS (already-built, verify-only) the `ArcDecision` struct + `derive` fn shapes that P-PROOFS consumes. No change to their public shape is permitted by this plan (P-PROOFS depends on shape stability — SOFT edge).

### CONSUMED (with the skip-if-present clause)

> **Skip-if-present rule (verbatim):** "Before landing this type, verify `elohim-compute` (or the named owner module) already exposes it. If present, VERIFY-ONLY (import + use). If absent at your integration point, land the owner-plan's verbatim definition only as a temporary local shim, flag it in your plan's hand-off notes, and delete the shim when the owner lands."

- **S1 `ActuationRefusal` + `RefusalCode`** — OWNER P-ACTUATION (`elohim_compute::actuation`, PROMOTED from `arc_actuator.rs`). P-ARC consumes ONLY transitively (via the actuation path P-ACTUATION refactors); P-ARC does NOT import or redefine these. They live in `arc_actuator.rs` TODAY (`arc_actuator.rs:77,83`), which P-ARC does not touch. **No shim needed** — P-ARC never references them directly.
- **S2 `trait Actuation` + `GrantBounds` / S3 `ScopeId` / S4 `MetaCooldown`,`NeverTouch`** — OWNER P-ACTUATION. arc becomes the first `impl Actuation for ArcKnob` — **authored by P-ACTUATION in `services/actuation/arc.rs`, NOT by P-ARC** (RESOLUTION-A). P-ARC supplies the production decision shapes (verify-only) and does not implement the trait itself.
- **S13 `sets-authority-arc` projection arm** — OWNER P-ACTUATION (`mishpat_projection.rs`, new `parse_sets_authority_arc`). **This is the HARD dependency:** arc's shipped actuate path is DEAD until S13 lands (the projection currently falls to `other =>` → empty bounds → never `active` → arc actuate never fires). P-ARC's restart-stagger gate (Task 3) is exercised by the actuation path, so it is only end-to-end-live AFTER S13. P-ARC's unit tests do NOT depend on S13 (they test the gate in isolation), so P-ARC is dispatchable in parallel; only the live-path validation waits.
- **S12 `Mishpat::Commitment` / `sets-authority-arc`** — already shipped (DNA, cad5fb67c). Verify-only.

---

## 4. DEPENDENCY EDGES (from the DAG)

| Edge | Type | Reason |
|------|------|--------|
| **P-ARC → P-ACTUATION** | **HARD** | arc-as-instance needs S2 trait + S1 refusal; arc's shipped actuate path is DEAD until S13 (projection arm); `arc_actuator.rs` is sequenced behind P-ACTUATION (RESOLUTION-A). P-ARC's *unit* work (L-term, stagger gate, spike) is independent and may start in Wave 1; only the live actuate-path validation and the arc-as-instance refactor wait on P-ACTUATION. |
| P-ARC → P-RECONCILE | **SOFT** | restart-stagger COULD consume a `StaggerCoordinator` from the Reconciler track; not required for v1 (node-local gate suffices). |
| P-PROOFS → P-ARC | **SOFT (inbound)** | P-PROOFS' multi-node coverage test consumes `ArcDecision`/`derive` shape, verify-only (RESOLUTION-H). P-ARC must NOT change those public shapes. |

No cycles. P-ARC is a Wave-2 consumer; its independent sub-tasks (2, 3, 5) are dispatchable immediately, gated only on the actuator/projection refactor for end-to-end liveness.

---

## 5. BUILD / TEST COMMANDS (per-crate, verified)

elohim-storage is the **WASM-flagged** crate (NOT `""`): use `RUSTFLAGS='--cfg getrandom_backend="custom"'`. `/tmp` target dir (pool fingerprint ENOENT). `RUSTC_WRAPPER=""` (sccache spawn-ENOENT). **Plain `cargo test`, NEVER nextest. Never `&&`-pipe a gate exit code** (use `2>&1 | tail -N`).

```
# Unit tests (Tasks 2, 3) — module-scoped
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib arc_policy 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib system_metrics 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib process_manager 2>&1 | tail -40

# Final gate (whole lib + clippy + fmt)
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo clippy --lib -- -D warnings 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-storage && cargo fmt --check
```

> Note: `cargo build --release` for elohim-storage routes through wasm-pack and is heavy; `cargo test --lib` (test profile, native) is the dispatchable gate. The pre-push hook owns the WASM build.

---

# ════════════════════════════════════════════════════════════════
# DECISION MEMO — operator-only. The 3 options + defended recommendation.
# ════════════════════════════════════════════════════════════════

The arc track is a **redesign gated on an operator decision**. Below is the memo; the implementation plan (§Tasks) is banner-marked GATED and must NOT be dispatched until an option is chosen.

## §A — The decision: which lever do we invest in?

### Option (i) — Ship the {0,1} REA-grant-bounded actuation (ALREADY BUILT)
- **What it is:** a node is a full anchor (`arc=1`) or an accountable, revocable leecher (`arc=0`), flipped via a `sets-authority-arc` REA commitment with a coverage-floor gate + staggered restart. Mechanized today.
- **Pro:** the ONLY thing actuatable on kitsune2 0.3.2. Bounds RAM for any non-lean node willing to be a full anchor; turns lean devices into *accountable* leechers (vs silent YAML edits).
- **Con:** a leecher contributes **nothing**. Option (i) alone does NOT keep "laptop = full participant" true (`project_hub_optional_floor`). It is a RAM-stopgap-or-leecher binary, not scaling.

### Option (ii) — Fractional arc (`a ∈ (0,1)`) via kitsune2 LocalAgent tgt-arc hint
- **What it is:** the continuous size dial `derive()` already computes as a signal, actually actuated.
- **Spike verdict (P0, verified):** **NOT FEASIBLE** on the deployed 0.3.2 line OR the 0.4.1 upgrade. `apply_arc_factor` clamps `{0,1}`; the only continuous path is the `LocalAgent` tgt-arc hint, reachable only via a kitsune2 `network.advanced` block (the edgenode `network:` block has no `advanced:` key) or a forked/custom kitsune2 module. Restart-only regardless; blocked on upstream.
- **Verdict: REJECT.** High carrying cost (fork `holochain_p2p`), restart-only anyway, no upstream timeline. `SAFE_MIN_ARC > 0` (a minimum *contributing* shard) is not expressible until this lands; that ceiling is honest and accepted.

### Option (iii) — Move corpus OFF the DHT authority plane into the RS quilt byte-plane
- **What it is:** the conductor DHT holds only a lean identity/provenance/anchor arc; corpus **content** lives in the RS quilt (`sharding.rs`, rs-4-7), reconstructable from any K-of-N shards. Per-node RAM becomes ∝ (lean anchor set) + (held shard count), independent of total corpus.
- **Pro:** the real scaling answer; architecturally aligned with what we already own (the quilt shards bytes today). Bounds per-node RAM ∝ C/N AND keeps lean devices full participants (they hold quilt shards, not a leecher's nothing).
- **Con:** large blast radius — it re-homes content truth from the DHT plane to the quilt plane. A design spike, not a sprint. There is **no code today** that diverts corpus off the DHT into the quilt.

## §B — RECOMMENDATION (defended)

**Pursue option (i) NOW (already built — only the L-term + stagger gate + projection-arm liveness remain) + scope option (iii) as the durable arc + explicitly REJECT (ii).**

Defense: (i) is the only actuatable lever on 0.3.2 and is shipped; it deserves the two correctness fixes that make it honest (L-term so `a_mem` isn't optimistic for james; stagger gate so the cure can't cause the partition) and the projection-arm fix (S13, owned by P-ACTUATION) that makes its actuate path live at all. (ii) is dead-end carrying cost — reject and stop computing fractional aims as anything but a gauge signal. (iii) is the only path that satisfies the hub-optional floor *and* bounds RAM; it is aligned with the RS quilt we already run, so it is a re-homing, not a green-field build. Scope it as a spike with a hard gate.

**Hard gate on (iii):** confirm **leak-vs-bounded-large first** (operator-side `ps -o rss,comm` conductor-child vs storage-parent split, or `target_arc_factor: 0` ablation on one loaded node). If the climb is DHT-sync convergence, (iii) lowers the plateau; if it is a genuine leak, (iii) shrinks the structure that leaks — but the discriminator tells us how much (iii) buys before we spend the blast radius. **Recommend gate (iii) on this confirm.**

---

# ════════════════════════════════════════════════════════════════
# ⚠ GATED ON OPERATOR ARC DECISION — DO NOT DISPATCH UNTIL CHOSEN ⚠
# Implementation plan for the RECOMMENDED option only (option i correctness
# fixes + the option iii spike). Dispatch ONLY after the operator confirms
# §B above AND the leak-vs-bounded discriminator for Task 5.
# ════════════════════════════════════════════════════════════════

## TASK 1 — (PRECONDITION, owned by P-ACTUATION) S13 projection arm + arc-as-instance
**Not a P-ARC task.** Listed for sequencing only. P-ARC's live-path validation (the integration check at the end of Task 3) requires P-ACTUATION to have landed `parse_sets_authority_arc` (S13) and the `services/actuation/arc.rs` refactor (RESOLUTION-A). P-ARC's unit tasks below do NOT block on this.

---

## TASK 2 — Wire the real `L` term (`local_authored_bytes`) into `derive`

**p2p-class:** the reader is a Cat-C node-local operational read (no DHT entry, no table, no coordinator fn) — cite the class; do not re-litigate. It samples the node's own always-resident authored share for the pure `derive` input.

Files:
- `elohim/elohim-storage/src/services/system_metrics.rs` — new `pub fn local_authored_bytes(...) -> Option<u64>`.
- `elohim/elohim-storage/src/services/arc_policy.rs` — no shape change; the input field already exists (`ArcInputs.local_authored_bytes`, `arc_policy.rs:79`). Verify-only that `derive` consumes it (it does, `:171`).
- `elohim/elohim-storage/src/http.rs:2764` — swap the literal `0` for the reader (the single additive input-wiring edit).

**Reader strategy (verified gap):** there is NO existing source-chain/authored-byte reader. `system_metrics` has `directory_size(path)` (`:30`) and the conductor health call exposes `storage.bytes_used`/`entry_count` (`http.rs:2749-2751`) but NOT an own-authored split. Two candidate sources, pick the available one at build time:
  1. **Source-chain DB size** — `directory_size` over the conductor's own source-chain DB path (own authored entries are the source chain; foreign arc is the cache/DHT store). Preferred if the path is reachable from `ConductorManager`.
  2. **Authored entry count × mean size** — `entry_count`-derived estimate if (1) is not reachable.
The reader returns `Option<u64>`; `None` keeps the current safe-optimistic `0` (never guess up).

- [ ] Write the failing test — append to `system_metrics.rs` `#[cfg(test)] mod tests`:
```rust
    #[test]
    fn local_authored_bytes_none_when_path_absent() {
        // A non-existent source-chain path yields None (safe: derive keeps L=0).
        let missing = std::path::Path::new("/nonexistent/source-chain");
        assert_eq!(local_authored_bytes(missing), None);
    }

    #[test]
    fn local_authored_bytes_sums_existing_dir() {
        let dir = std::env::temp_dir().join("arc-l-term-test");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("chain.db"), vec![0u8; 4096]).unwrap();
        let v = local_authored_bytes(&dir).expect("existing dir sums");
        assert!(v >= 4096, "must include the 4096-byte chain file, got {v}");
        let _ = std::fs::remove_dir_all(&dir);
    }
```
- [ ] Run, expect FAIL: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib system_metrics 2>&1 | tail -40` — expect `cannot find function local_authored_bytes`.
- [ ] Write minimal implementation — add to `system_metrics.rs`:
```rust
/// Best-effort size of this node's OWN always-resident authored share (the
/// source-chain DB), the `L` term of the arc memory model: RAM ≈ L + a·(C−L)
/// (auto-policy spec §3 line 87). Returns None when the path is unreadable —
/// the caller then keeps L=0 (safe, never guesses an upward shrink).
/// Cat-C node-local operational read.
pub fn local_authored_bytes(source_chain_path: &Path) -> Option<u64> {
    if !source_chain_path.exists() {
        return None;
    }
    directory_size(source_chain_path).ok()
}
```
- [ ] Wire the call site — at `http.rs:2761-2764`, replace `local_authored_bytes: 0,` (and its comment) with a best-effort read off the conductor source-chain path (or the entry-count estimate if the path is not exposed). Keep the `Option` → `unwrap_or(0)` so a missing read preserves today's safe behavior. Update the comment to "L = own authored source-chain bytes (system_metrics::local_authored_bytes); 0 when unreadable (safe)."
- [ ] Run, expect PASS: `... cargo test --lib system_metrics 2>&1 | tail -40` and `... cargo test --lib arc_policy 2>&1 | tail -40` (confirm `derive`'s james-case tests still pass with a non-zero L).
- [ ] Commit:
```
git add elohim/elohim-storage/src/services/system_metrics.rs elohim/elohim-storage/src/services/arc_policy.rs elohim/elohim-storage/src/http.rs
git commit -m "feat(elohim-storage): wire real L term (local_authored_bytes) into arc derive

The own-authored share L was hard-wired to 0, making a_mem optimistic for
exactly the james OOM case (largest authored corpus). Add a Cat-C node-local
source-chain size reader; derive now bounds the foreign-arc term honestly.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## TASK 3 — Restart-stagger gate in `process_manager` (no concurrent shrink)

**p2p-class:** Cat-C node-local operational gate (no DHT entry). It is a node-local guard against concurrent local-conductor restart while a recent restart's reconvergence window is still open; true cross-mesh negotiation is a FOLLOW-ON seam (consumes a `StaggerCoordinator` from P-RECONCILE).

Files:
- `elohim/elohim-storage/src/conductor/process_manager.rs` — add a monotonic `last_arc_restart: Option<Instant>` field on `ConductorManager` + a `may_restart_for_arc(now, min_interval) -> Result<(), StaggerRefusal>` pure-ish gate consulted by the actuation path BEFORE `restart()`.

The gate enforces: a local arc-driven restart may not fire within `min_interval` of the previous one (the gossip reconvergence window from `DhtArc::Empty`, `core_space.rs:451`). On refusal the actuation is DEFERRED, not forced — the cure must not cause the partition.

- [ ] Write the failing test — append to `process_manager.rs` `#[cfg(test)] mod tests`:
```rust
    #[test]
    fn arc_restart_gate_blocks_within_window() {
        use std::time::{Duration, Instant};
        let now = Instant::now();
        // First arc restart always admitted.
        assert!(StaggerGate::new(Duration::from_secs(120))
            .check(None, now)
            .is_ok());
        // A restart 30s after the last is within the 120s window: REFUSED.
        let last = now;
        let later = now + Duration::from_secs(30);
        assert!(StaggerGate::new(Duration::from_secs(120))
            .check(Some(last), later)
            .is_err());
        // A restart 121s later: admitted.
        let much_later = now + Duration::from_secs(121);
        assert!(StaggerGate::new(Duration::from_secs(120))
            .check(Some(last), much_later)
            .is_ok());
    }
```
- [ ] Run, expect FAIL: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib process_manager 2>&1 | tail -40` — expect `cannot find type StaggerGate`.
- [ ] Write minimal implementation — add a pure `StaggerGate` next to `ConductorManager` and a `last_arc_restart: Option<Instant>` field updated on a successful arc restart:
```rust
/// Pure node-local gate: an arc-driven conductor restart may not fire within
/// the gossip reconvergence window of the previous one, so a sequence of
/// leecher-flips cannot transiently breach the coverage floor (auto-policy §4,
/// "stagger restarts; remaining mesh covers the reconvergence window"). This is
/// the LOCAL half; cross-mesh negotiation is a FOLLOW-ON (StaggerCoordinator,
/// P-RECONCILE). Returns Err(StaggerRefusal) to DEFER, never to force.
pub struct StaggerGate {
    min_interval: std::time::Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaggerRefusal {
    pub wait_secs: u64,
}

impl StaggerGate {
    pub fn new(min_interval: std::time::Duration) -> Self {
        Self { min_interval }
    }
    pub fn check(
        &self,
        last_arc_restart: Option<std::time::Instant>,
        now: std::time::Instant,
    ) -> Result<(), StaggerRefusal> {
        match last_arc_restart {
            None => Ok(()),
            Some(last) => {
                let elapsed = now.saturating_duration_since(last);
                if elapsed >= self.min_interval {
                    Ok(())
                } else {
                    Err(StaggerRefusal {
                        wait_secs: (self.min_interval - elapsed).as_secs(),
                    })
                }
            }
        }
    }
}
```
  Add `last_arc_restart: Option<std::time::Instant>` to `ConductorManager` (default `None`); on a successful arc-driven `restart()`, set it. Expose a `pub fn may_arc_restart(&self, now, min_interval) -> Result<(), StaggerRefusal>` thin wrapper the actuation path (P-ACTUATION's `services/actuation/arc.rs`) calls before `restart()`. **NOTE the seam:** P-ARC adds the gate + accessor; P-ACTUATION wires the call into the actuate path (it owns `arc_actuator.rs`/`services/actuation/arc.rs`). This is an explicit hand-off — P-ARC lands the gate; P-ACTUATION consumes it. Flag in hand-off notes.
- [ ] Run, expect PASS: `... cargo test --lib process_manager 2>&1 | tail -40`.
- [ ] Commit:
```
git add elohim/elohim-storage/src/conductor/process_manager.rs
git commit -m "feat(elohim-storage): node-local restart-stagger gate for arc actuation

apply_arc_actuation restarted only the local conductor with no guard against
concurrent leecher-flips breaching the coverage floor during the
DhtArc::Empty->gossip reconvergence window. Add a pure StaggerGate (defer,
never force). Cross-mesh negotiation is a follow-on (StaggerCoordinator).

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## TASK 4 — Verify-only: export `ArcDecision`/`derive` shapes for P-PROOFS (RESOLUTION-H)

No code change expected; this task GUARANTEES the SOFT inbound edge P-PROOFS depends on.

- [ ] Verify `arc_policy::ArcDecision` and `arc_policy::derive` are `pub` and reachable (`arc_policy.rs:109` struct, `:138` fn — confirmed `pub`).
- [ ] Confirm this plan changed NONE of their public field/signature shapes (Task 2 changed only the *value* flowing into `local_authored_bytes`, an existing `pub` field).
- [ ] Record in hand-off notes: "P-PROOFS `tests/arc_coverage_multinode.rs` may consume `arc_policy::{ArcDecision, derive, ArcInputs, CoverageParams}` verbatim; shapes are stable post-P-ARC."
- [ ] No commit (verification task).

---

## TASK 5 — Design spike: corpus-off-DHT into the RS quilt (option iii)

**⚠ GATED twice:** dispatch only after (a) the operator chooses option (iii) per the Decision Memo AND (b) the leak-vs-bounded-large discriminator has been run (operator-side). The spike DOCUMENTS the discriminator result in its Evidence section.

**p2p-class (for the entities the spike will propose):** re-homing corpus content from the DHT authority plane to the RS quilt byte-plane touches Cat-A notarized provenance (the anchor stays DHT) and Cat-C operational shard placement. The spike runs `p2p-design-gate` per proposed entity — it does NOT pre-decide the classes here.

Files:
- `genesis/docs/superpowers/specs/2026-06-14-corpus-off-dht-spike.md` — new design spike.

Contents the spike MUST cover:
- The split: what stays on the conductor DHT (lean identity/provenance/anchor arc) vs what moves to the RS quilt (corpus content bytes, reconstructable any-K-of-N).
- The boundary with `sharding.rs` (rs-4-7, galois_8, 64MB threshold) — the quilt shards bytes today; the spike defines how corpus *entries* become quilt *objects* and how a read reconstructs from K shards.
- The RAM math: per-node RAM ∝ (lean anchor set) + (held shard count), independent of total corpus C; cite the auto-policy §4 derivation (`∝ C/N` bounded).
- The hub-optional-floor proof: a lean device holds shards (full participant), NOT a leecher's nothing.
- The leak-vs-bounded discriminator RESULT (operator confirm) and how much (iii) buys under each branch.
- Version skew note (kitsune2 0.3.2 edgenode vs 0.4.1 steward; steward `target_arc_factor=0` default).
- The blast-radius register: provenance gate, reseed/heal-on-read, the SSR/read path, custody convergence.
- A staged rollout (anchor-only first, then content migration) — never a blind content re-home.

- [ ] Author the spike (no tests; design artifact). Run `p2p-design-gate` per proposed entity inside the spike.
- [ ] Commit:
```
git add genesis/docs/superpowers/specs/2026-06-14-corpus-off-dht-spike.md
git commit -m "spec(arc): design spike — corpus off the DHT authority plane into the RS quilt

The durable arc-scaling answer (option iii): conductor holds a lean
anchor arc; corpus content lives in the RS quilt, reconstructable any-K-of-N.
Gated on the operator leak-vs-bounded confirm. Spike, not a sprint.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## FINAL GATE (run before declaring the track done)
```
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo clippy --lib -- -D warnings 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-storage && cargo fmt --check
```

---

## // FOLLOW-ON SEAMS (deliberately left for the integration pass)

- **arc-as-instance refactor** — moving the arc decision core to `impl Actuation for ArcKnob` in `services/actuation/arc.rs` and reducing `arc_actuator.rs` to a thin shim is OWNED by P-ACTUATION (RESOLUTION-A). P-ARC supplies shapes; integration sequences P-ACTUATION first.
- **StaggerGate call-wiring into the actuate path** — P-ARC lands `StaggerGate` + `may_arc_restart`; P-ACTUATION (sole `arc_actuator.rs`/`services/actuation/arc.rs` mutator) wires the call before `restart()`. Explicit hand-off.
- **Cross-mesh `StaggerCoordinator`** — the node-local gate (Task 3) does NOT negotiate across peers. A true coverage-preserving cross-mesh stagger consumes a `StaggerCoordinator` from P-RECONCILE (SOFT edge); deferred to v2.
- **Live actuate-path validation** — depends on P-ACTUATION's S13 projection arm; P-ARC's unit tests stand alone, but the end-to-end "commitment → projection → derive → gate → restart" check waits on S13.
- **`http.rs:2764` input-wiring line** — the single P-ARC edit to http.rs (swap `0` for the reader). Flagged for the integrator since http.rs is otherwise P-ACTUATION's (RESOLUTION-F confirms no P-DIAGNOSTIC overlap).
- **Fractional arc (option ii)** — REJECTED; left dormant. `derive`'s fractional aim stays a gauge signal only; no actuator. Revisit only if a future kitsune2 exposes a runtime/advanced tgt-arc API.
- **L-term reader source** — Task 2 picks source-chain-dir size OR entry-count estimate at build time; if neither is cleanly reachable from `ConductorManager`, the reader returns `None` (safe) and refining the source is itself a follow-on.

---

## DISPATCH NOTE

- **Isolated worktree**, subagent-driven (superpowers:subagent-driven-development). Do NOT run in the shared `feat` tree — a parallel arc/actuation thread mutates adjacent files.
- **Commit-only on the shift branch; the integrator pushes.** Never `git push`.
- **Per-task `git add` lists name exact files only** (selective-stage) — the worktree may carry ambient mods.
- **GATED:** the whole Tasks section is banner-marked GATED ON OPERATOR ARC DECISION. Do not dispatch Tasks 2-5 until the operator confirms option (i)+(iii)/reject-(ii) per the Decision Memo, and (for Task 5) the leak-vs-bounded discriminator has been run.
- **HARD dependency:** P-ACTUATION must land S13 (projection arm) + the RESOLUTION-A refactor before P-ARC's live actuate-path validation; P-ARC's unit work (Tasks 2, 3, 5) is independently dispatchable in Wave 1/2.
- **Runtime Rust must NEVER write `.claude/data`** (the elevate arm is an external poller) — the L-term reader and stagger gate only read system/conductor state.
