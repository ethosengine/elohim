# Dataplane Resilience Proofs + CI P2P-sim — Implementation Plan (P-PROOFS)

> For agentic workers: REQUIRED SUB-SKILL: superpowers:subagent-driven-development (or executing-plans). Steps use checkbox (- [ ]) syntax. TDD throughout: write the failing test, run-expect-FAIL, write minimal code, run-expect-PASS, commit.

**Goal:** Close the five THINNEST named gaps in the Approved resilience-dimensions proof suite. The quilt/RS implementation EXISTS (`sharding.rs` RS(4,7) galois_8) — the gap is **proofs, not implementation**. This plan proves the durability claims the whole dataplane quietly depends on, and revives the dead CI P2P-sim stage.

**Findings closed (review §7):**
- **#7(a)** RS reconstruct-from-**any**-K-of-N — current tests prove only *named* drop-sets (drop-2, drop-all-parity, drop-3-mixed); no exhaustive C(7,3) survivor-set property.
- **#7(b)** placement-diversity invariant — `single_fault_domain_risk` is a *felt SCORE*, not a *placement proof*. `p2p/mod.rs:1498` `selected[i % selected.len()]` concentrates multiple shards onto one peer when peers < shards; no test asserts one object's K-of-N shards survive a single-household loss.
- **#7(c)** arc coverage-floor **multi-node** invariant — `arc_policy::derive` `a_cov = min(1, R/N)` is single-node arithmetic; no test proves the *cluster* property (union of N nodes' arcs covers keyspace [0,1) as arcs shrink).
- **#7(d)** no-overwhelm **SOAK** — none exists (`bench_blob_stress_10k.rs` is a sequential p99 latency harness, not sustained-concurrency-under-backpressure).
- **#7(e)** revive the dead elohim-edge P2P-sim CI stage — `simulate.sh` uses `docker-compose`/`docker network`/`docker exec` against a daemonless nerdctl/buildkit pod → exit-127.

**Tech stack:** Rust (`elohim-storage` integration tests, WASM-flagged crate), bash (`simulate.sh`), Gherkin (a2o). **No new crate deps** — RS property test is hand-rolled exhaustive C(7,3) over the existing `reconstruct` (decision below). Tests are integration tests under `elohim/elohim-storage/tests/`; the soak is `#[ignore]`-gated.

---

## Decisions (self-answered; operator may override)

**D1 — `proptest` dev-dep vs hand-rolled combinatorial RS test? → HAND-ROLL.** C(7,3) = 35 survivor sets is small, deterministic, and zero-dep. Adding `proptest` to a WASM-`getrandom`-flagged crate risks a getrandom-backend link conflict and adds a dep for one test. The exhaustive enumeration is *stronger* than randomized proptest here (it covers every case, not a sample). Recommend hand-roll; revisit proptest only if a future property has an unbounded input space.

**D2 — CI P2P-sim path (3 backlog options) → OPTION 2 (`nerdctl compose` migration).** `nerdctl` is confirmed present in the build container (it builds every image in this Jenkinsfile). Migrate `simulate.sh`'s `docker-compose`→`nerdctl compose`, `docker network`→`nerdctl network`, `docker exec/ps`→`nerdctl`. KEEP the advisory `catchError(buildResult:'SUCCESS', stageResult:'UNSTABLE')` wrapper (Jenkinsfile:1390) until ONE green edge CI run proves it; do NOT drop the stage. The `sh './simulate.sh test'` call stays heredoc-free (Jenkinsfile CPS size-limit gotcha) — all logic stays in the .sh.

**D3 — arc multi-node coverage: test-only invariant vs production function? → TEST-ONLY FIRST.** A test-local pure helper `fn coverage_gap(arcs: &[(f64,f64)]) -> Option<(f64,f64)>` inside the test file avoids a premature production surface. If a runtime caller later needs it, promote to `arc_policy::keyspace_covered` then — not now.

**D4 — placement-diversity test target.** `PeerSelection::select()` is DB-bound (Diesel pool); its greedy diversity passes are INLINE, not a pure function. Two assertable layers without a refactor: **(L1)** the `distribute_shards` shard→peer *mapping* (`selected[i % selected.len()]`) is pure arithmetic — assert directly on a synthetic `Vec<SelectedPeer>` that the resulting shard→household map keeps ≥ K survivors after dropping any one household. **(L2)** the greedy ranking is exercised by extracting a pure `rank_by_diversity(candidates, desired) -> Vec<SelectedPeer>` helper (SEAM-DELTA — see below). This plan does **L1 only** (the actual durability-relevant bug surface) and records L2 as a follow-on refactor seam, to keep the diff test-only and avoid touching the DB selector that other tracks may also touch.

---

## OWNED FILES (verbatim from ledger §2)

**Creates:**
- `elohim/elohim-storage/tests/rs_reconstruct_property.rs` — SOLE owner (#7a)
- `elohim/elohim-storage/tests/placement_diversity_invariant.rs` — SOLE owner (#7b)
- `elohim/elohim-storage/tests/arc_coverage_multinode.rs` — SOLE owner (#7c; absorbs P-ARC's dropped file, RESOLUTION-H)
- `elohim/elohim-storage/tests/no_overwhelm_soak.rs` (`#[ignore]`) — SOLE owner (#7d)
- `genesis/a2o/features/resilience/chaos-peer-churn.feature` — already exists; this plan un-`@wip`s the dynamic rows (the ledger lists this as a P-PROOFS create; it exists, so MUTATE)

**Mutates:**
- `steward/node/simulation/simulate.sh` — SOLE owner (`docker-compose`→`nerdctl compose`, `docker network/exec/ps`→`nerdctl`) (#7e)
- `elohim/holochain/Jenkinsfile` (NOT `dna/Jenkinsfile` — ledger says `dna/`; the real home is the EDGE Jenkinsfile, helper `runSimulationTest()` @ line 167, stage @ line 1375) — re-gate verification, SOLE owner, **hand-off note to the edge-Jenkinsfile owner** (#7e)

**Collision statement:** This plan **touches no Rust source file owned by another plan**. All Rust work is NEW integration-test files under `tests/`; it CONSUMES (never redefines) production types from `sharding.rs`, `peer_selection.rs`, `arc_policy.rs`, and `tests/chaos_dataplane.rs`. The only shared-surface touch is the `simulate.sh` + edge `Jenkinsfile` revival (operator hand-off). No `src/` mutation, no `Cargo.toml` dep add, no `elohim-compute` touch.

---

## NEW PRIMITIVES THIS PLAN OWNS

| Primitive | Kind | Home | Notes |
|---|---|---|---|
| `fn coverage_gap(arcs: &[(f64,f64)]) -> Option<(f64,f64)>` | test-local fn | `tests/arc_coverage_multinode.rs` | pure, no export (D3). Returns the first uncovered [0,1) sub-interval, or `None` if the union covers fully. |
| `fn shard_to_household_map(...)` + `fn survivors_after_household_loss(...)` | test-local fns | `tests/placement_diversity_invariant.rs` | pure, no export. Mirror `distribute_shards`' `i % len` mapping (D4-L1). |

**No shared-crate primitive is created.** p2p-class: all new entities are **Cat-C node-local read-models** (test-only assertions over existing operational state — no DHT entry, no table, no coordinator fn). Cited per SHARED GROUNDING; not re-litigated.

## CONSUMED PRIMITIVES (skip-if-present clause applies)

> **Skip-if-present rule (verbatim):** Before using any of these, verify the owner module already exposes it. If present, VERIFY-ONLY (import + use). If absent at your integration point, the owner-plan has slipped — flag it in hand-off notes and stub a local mirror only as a temporary shim, deleting it when the owner lands.

| Primitive | Owner module (single-owner) | Used by |
|---|---|---|
| `ShardEncoder` / `ShardConfig` / `ShardManifest` / `reconstruct` / `create_shards` / `create_manifest` | `elohim_storage::sharding` | rs_reconstruct_property |
| `SelectedPeer` / `SelectionOutcome` (fields only) | `elohim_storage::services::peer_selection` | placement_diversity_invariant |
| `arc_policy::derive` / `ArcInputs` / `ArcDecision` / `CoverageParams` | `elohim_storage::services::arc_policy` (arc track) | arc_coverage_multinode (verify-only, RESOLUTION-H) |
| chaos `Node` / `spawn_node` / `.kill()` / `.fetch()` / `.dial()` | `elohim-storage/tests/chaos_dataplane.rs` | no_overwhelm_soak (consume the churn primitive; do NOT redefine) |
| `connection_limits::Behaviour` (max_established) | `libp2p::connection_limits` via P-TRANSPORT swarm | no_overwhelm_soak (soft — see DAG) |

---

## DEPENDENCY EDGES (from ledger §4 DAG)

- **P-PROOFS → P-TRANSPORT — SOFT.** The no-overwhelm soak (#7d) *proves* the `connection_limits` floor. If P-TRANSPORT has not landed the cap, the soak documents the unbounded behavior as an `#[ignore]` test that asserts the cap when present — it does NOT block on P-TRANSPORT. Gate the cap-assertion behind a `cfg`/runtime probe so the file compiles either way.
- **P-PROOFS → P-ARC — SOFT (RESOLUTION-H).** `arc_coverage_multinode` consumes `ArcDecision`/`derive` *shape* only (verify-only, no redefine). It synthesizes its own `[(f64,f64)]` arcs and checks the union property; it does not depend on P-ARC's L-term changes. P-ARC DROPS its own `arc_coverage_invariant.rs`; this file is the single owner.
- **NO hard edges.** Proofs consume only existing types. The P-PROOFS-core (a, b) is a DAG root and dispatches in WAVE 1; (c, d) and the soak are WAVE 2; the CI revival (e) is independent and dispatches any time.

---

## Build / test commands (per-crate RUSTFLAGS + /tmp target dir + plain cargo test)

`elohim-storage` is the **WASM-flagged** crate (custom getrandom backend). All Rust tasks:
```
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-proofs RUSTC_WRAPPER="" cargo test --test rs_reconstruct_property 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-proofs RUSTC_WRAPPER="" cargo test --test placement_diversity_invariant 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-proofs RUSTC_WRAPPER="" cargo test --test arc_coverage_multinode 2>&1 | tail -40
# soak is #[ignore]; run explicitly:
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-proofs RUSTC_WRAPPER="" cargo test --test no_overwhelm_soak -- --ignored 2>&1 | tail -60
```
Final gate:
```
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-proofs RUSTC_WRAPPER="" cargo test --tests 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-proofs RUSTC_WRAPPER="" cargo clippy --tests -- -D warnings 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-storage && cargo fmt --check
bash -n steward/node/simulation/simulate.sh   # syntax-check the migrated script
```
Rules: `RUSTFLAGS='--cfg getrandom_backend="custom"'` (this crate ONLY), `RUSTC_WRAPPER=""` (sccache spawn-ENOENT), `/tmp` target dir (pool fingerprint-ENOENT), **plain `cargo test`, NEVER nextest**, never `&&`-pipe a gate exit code (use `2>&1 | tail -N`).

---

## TASK 1 — RS reconstruct-from-any-K-of-N exhaustive property (#7a) [S]

Files: `elohim/elohim-storage/tests/rs_reconstruct_property.rs` (create).

Prove: for RS(4,7), reconstruct succeeds from EVERY survivor set of size ≥ 4 (all C(7,k) for k∈{4,5,6,7}), and FAILS from every survivor set of size < 4. Hand-rolled combinatorial enumeration over the existing `reconstruct` (D1).

- [ ] Write the failing test:
```rust
//! #7(a): RS(4,7) reconstruct from ANY K-of-N survivor set (exhaustive).
//! Strengthens sharding.rs's named-drop tests to the full combinatorial property.

use elohim_storage::sharding::{ShardConfig, ShardEncoder};

fn rs_47_encoder() -> ShardEncoder {
    ShardEncoder::new(ShardConfig {
        shard_size: 25,
        rs_data_shards: 4,
        rs_parity_shards: 3,
        rs_threshold: 50,
        single_shard_max: 10, // force RS for the test payload
    })
}

/// All size-k subsets of 0..n as bitmasks.
fn subsets(n: usize, k: usize) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    for mask in 0u32..(1 << n) {
        if (mask.count_ones() as usize) == k {
            out.push((0..n).filter(|i| mask & (1 << i) != 0).collect());
        }
    }
    out
}

#[test]
fn rs47_reconstructs_from_every_survivor_set_of_size_ge_4() {
    let enc = rs_47_encoder();
    let data: Vec<u8> = (0..200).map(|i| (i % 256) as u8).collect();
    let manifest = enc
        .create_manifest(&data, "application/octet-stream", "commons")
        .unwrap();
    assert_eq!(manifest.encoding, "rs-4-7");
    let shards = enc.create_shards(&data, &manifest.encoding).unwrap();
    let n = shards.len();
    assert_eq!(n, 7);

    let mut checked = 0usize;
    for k in 4..=7 {
        for survivors in subsets(n, k) {
            let opts: Vec<Option<Vec<u8>>> = (0..n)
                .map(|i| {
                    if survivors.contains(&i) {
                        Some(shards[i].clone())
                    } else {
                        None
                    }
                })
                .collect();
            let recovered = enc.reconstruct(&manifest, &opts).unwrap_or_else(|e| {
                panic!("survivors {survivors:?} (k={k}) must reconstruct: {e}")
            });
            assert_eq!(recovered, data, "byte mismatch for survivors {survivors:?}");
            checked += 1;
        }
    }
    // C(7,4)+C(7,5)+C(7,6)+C(7,7) = 35+21+7+1 = 64
    assert_eq!(checked, 64, "must exhaustively cover every k>=4 survivor set");
}

#[test]
fn rs47_fails_from_every_survivor_set_below_data_shards() {
    let enc = rs_47_encoder();
    let data: Vec<u8> = (0..200).map(|i| (i % 256) as u8).collect();
    let manifest = enc
        .create_manifest(&data, "application/octet-stream", "commons")
        .unwrap();
    let shards = enc.create_shards(&data, &manifest.encoding).unwrap();
    let n = shards.len();

    for k in 0..4 {
        for survivors in subsets(n, k) {
            let opts: Vec<Option<Vec<u8>>> = (0..n)
                .map(|i| survivors.contains(&i).then(|| shards[i].clone()))
                .collect();
            assert!(
                enc.reconstruct(&manifest, &opts).is_err(),
                "k={k} (<4 data shards) must FAIL, survivors {survivors:?}"
            );
        }
    }
}
```
- [ ] Run, expect FAIL (compile-clean but test absent / or assertion if `reconstruct` mishandles an edge survivor set): use the build command for `rs_reconstruct_property`.
- [ ] If green immediately (durability claim already holds): that is the PROOF landing — `reconstruct` is correct; the test is the new artifact. Confirm the `checked == 64` assertion fired (proves exhaustiveness, not a no-op).
- [ ] Commit:
```
git add elohim/elohim-storage/tests/rs_reconstruct_property.rs
git commit -m "test(storage): RS(4,7) reconstruct-from-any-K-of-N exhaustive property (#7a)

Closes the resilience-dimensions gap: prior tests covered only named
drop-sets. Exhaustively proves reconstruct succeeds from every C(7,k>=4)
survivor set (64 cases) and fails below 4 data shards. Hand-rolled
combinatorial, zero new dep.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 2 — Placement-diversity invariant (#7b) [M]

Files: `elohim/elohim-storage/tests/placement_diversity_invariant.rs` (create).

Prove the durability-relevant property and SURFACE the `i % len` concentration bug: map shards→peers exactly as `distribute_shards` does (`selected[i % selected.len()]`, p2p/mod.rs:1498), then assert that dropping any ONE household leaves ≥ K data shards. The bug: when `peers < shards`, multiple shards land on one peer, so a single-household loss can drop below K. This test makes the invariant explicit; where it FAILS it documents the bug as a known-OPEN with a clear assertion (a RED test that names the fix, per the reviewer-admissibility discipline — it asserts the SAFE config and is `#[ignore]`-marked + commented for the deficient config until the round-robin is diversity-aware).

- [ ] Write the test (pure helpers mirror the production mapping):
```rust
//! #7(b): placement-diversity invariant. One object's K-of-N shards must
//! survive a single-household loss. Mirrors distribute_shards' shard->peer
//! mapping (p2p/mod.rs:1498 `selected[i % selected.len()]`).

use elohim_storage::services::peer_selection::SelectedPeer;

fn peer(id: &str, household: &str) -> SelectedPeer {
    SelectedPeer {
        peer_id: id.into(),
        agent_pub_key: id.into(),
        household_id: Some(household.into()),
        archetype: None,
        node_id: None,
    }
}

/// Production mapping: shard i -> selected[i % len] (distribute_shards).
fn shard_to_peer(selected: &[SelectedPeer], total_shards: usize) -> Vec<usize> {
    (0..total_shards).map(|i| i % selected.len()).collect()
}

/// How many shards survive if every peer in `lost_household` is dropped.
fn survivors_after_household_loss(
    selected: &[SelectedPeer],
    total_shards: usize,
    lost_household: &str,
) -> usize {
    let map = shard_to_peer(selected, total_shards);
    map.iter()
        .filter(|&&p| selected[p].household_id.as_deref() != Some(lost_household))
        .count()
}

#[test]
fn one_household_loss_keeps_k_survivors_with_n_diverse_peers() {
    // RS(4,7): 7 shards, K=4. 7 peers across 7 households = full diversity.
    let selected: Vec<SelectedPeer> = (0..7)
        .map(|i| peer(&format!("p{i}"), &format!("hh{i}")))
        .collect();
    for lost in 0..7 {
        let n = survivors_after_household_loss(&selected, 7, &format!("hh{lost}"));
        assert!(n >= 4, "loss of hh{lost} left {n} shards (< K=4)");
    }
}

#[test]
fn diversity_holds_when_peers_equal_data_shards_across_households() {
    // 4 peers, 4 households, 7 shards -> shards 0..3 on distinct hh, 4..6 wrap.
    // Losing one household drops at most ceil(7/4)=2 shards => 5 survivors >= 4.
    let selected: Vec<SelectedPeer> = (0..4)
        .map(|i| peer(&format!("p{i}"), &format!("hh{i}")))
        .collect();
    for lost in 0..4 {
        let n = survivors_after_household_loss(&selected, 7, &format!("hh{lost}"));
        assert!(n >= 4, "loss of hh{lost} left {n} shards (< K=4)");
    }
}

/// KNOWN-OPEN: when peers < K all on the SAME household, round-robin
/// concentrates and a single-household loss is catastrophic. This documents
/// the i%len bug surfaced by the review. Un-ignore once distribute_shards is
/// diversity-aware (placement should refuse/degrade, not silently concentrate).
#[test]
#[ignore = "OPEN: distribute_shards i%len concentrates when peers<shards; \
            fix = diversity-aware placement (review #7b)"]
fn single_household_two_peers_concentration_is_catastrophic() {
    let selected = vec![peer("p0", "hhA"), peer("p1", "hhA")];
    let n = survivors_after_household_loss(&selected, 7, "hhA");
    assert!(n >= 4, "EXPECTED FAIL TODAY: all shards on hhA, loss => {n}");
}
```
- [ ] Run, expect PASS (the two non-ignored invariants hold for diverse configs; the ignored one documents the bug): build command for `placement_diversity_invariant`. The `#[ignore]` test is the named follow-on, not a CI red.
- [ ] Commit:
```
git add elohim/elohim-storage/tests/placement_diversity_invariant.rs
git commit -m "test(storage): placement-diversity invariant — K survivors after one household loss (#7b)

Asserts D4 as a PLACEMENT PROOF, not a felt score. Surfaces the
distribute_shards i%len concentration bug as a named #[ignore] OPEN
(peers<shards on one household => single-loss catastrophic).

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 3 — Arc coverage-floor multi-node keyspace-gap invariant (#7c) [M]

Files: `elohim/elohim-storage/tests/arc_coverage_multinode.rs` (create). RESOLUTION-H: SOLE owner; P-ARC dropped its file.

Prove the CLUSTER property `arc_policy.derive` never proves: synthesize N nodes' arcs as `[0,1)` sub-intervals; assert their union covers the whole keyspace with no gap as arcs shrink — and that shrinking a node below its coverage floor OPENS a gap (the property `derive`'s per-node floor is meant to prevent). Verify-only on `derive`/`ArcDecision` shape (no redefine).

- [ ] Write the test with the pure `coverage_gap` helper (D3):
```rust
//! #7(c): arc coverage-floor MULTI-NODE keyspace invariant. derive()'s
//! a_cov = min(1,R/N) is single-node arithmetic; this proves the cluster
//! property: the union of N nodes' authority arcs covers [0,1) with no gap.

use elohim_storage::services::arc_policy::{derive, ArcInputs, CoverageParams};

/// Returns the first uncovered [0,1) sub-interval, or None if fully covered.
/// arcs are (start, end) with wrap allowed (end may exceed 1.0 => wraps to 0).
fn coverage_gap(arcs: &[(f64, f64)]) -> Option<(f64, f64)> {
    // Normalize to a set of [start,end) segments on [0,1), splitting wraps.
    let mut segs: Vec<(f64, f64)> = Vec::new();
    for &(s, e) in arcs {
        let s = s.rem_euclid(1.0);
        let len = (e - arcs.iter().map(|a| a.0).next().unwrap_or(s)).abs();
        let _ = len; // length captured via (s,e) directly below
        if e <= 1.0 {
            segs.push((s, e));
        } else {
            segs.push((s, 1.0));
            segs.push((0.0, e - 1.0));
        }
    }
    segs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let mut covered_to = 0.0_f64;
    for (s, e) in segs {
        if s > covered_to + 1e-9 {
            return Some((covered_to, s));
        }
        covered_to = covered_to.max(e);
    }
    if covered_to + 1e-9 < 1.0 {
        Some((covered_to, 1.0))
    } else {
        None
    }
}

/// Place N nodes' arcs evenly around the ring; each node owns `arc_factor`
/// of the keyspace starting at its slot. Union covers [0,1) iff
/// sum(arc_factor) >= 1 AND placement has no gap.
fn ring_arcs(n: usize, arc_factor: f64) -> Vec<(f64, f64)> {
    (0..n)
        .map(|i| {
            let start = i as f64 / n as f64;
            (start, start + arc_factor)
        })
        .collect()
}

#[test]
fn full_arcs_cover_keyspace() {
    let arcs = ring_arcs(7, 1.0);
    assert!(coverage_gap(&arcs).is_none(), "full arcs must cover [0,1)");
}

#[test]
fn evenly_shrunk_arcs_stay_covered_above_floor() {
    // 7 nodes each owning >= 1/7 of the ring => union covers with overlap.
    let arcs = ring_arcs(7, 1.0 / 7.0 + 1e-6);
    assert!(coverage_gap(&arcs).is_none(), "1/N arcs tile the ring");
}

#[test]
fn shrinking_below_one_over_n_opens_a_gap() {
    // Each node owns < 1/N => provable gap. This is the failure derive()'s
    // per-node coverage floor exists to PREVENT (never shrink into a gap).
    let arcs = ring_arcs(7, 1.0 / 7.0 - 0.02);
    assert!(
        coverage_gap(&arcs).is_some(),
        "arcs below 1/N must leave an uncovered keyspace band"
    );
}

#[test]
fn derive_floor_keeps_cluster_covered_at_target_redundancy() {
    // Cross-check: derive()'s a_cov = R/N is the per-node floor. With N nodes
    // each at >= a_cov, the union (with R-fold overlap) covers [0,1).
    let n = 14u32;
    let mut min_arc = 1.0;
    for _ in 0..n {
        let d = derive(ArcInputs {
            mem_ceiling_bytes: Some(2 * 1024 * 1024 * 1024),
            storage_headroom_bytes: 0,
            observed_n: Some(n),
            corpus_bytes: Some(8 * 1024 * 1024 * 1024),
            local_authored_bytes: 0,
            archetype_base_arc: 1.0,
            coverage: CoverageParams::default(),
        });
        min_arc = min_arc.min(d.arc_factor);
        assert!(d.arc_factor >= d.coverage_floor - 1e-9);
    }
    // Every node at >= a_cov(=R/N=0.5 here) => union with overlap covers ring.
    let arcs = ring_arcs(n as usize, min_arc);
    assert!(
        coverage_gap(&arcs).is_none(),
        "at the derive() coverage floor the cluster keyspace stays gap-free"
    );
}
```
- [ ] Run, expect PASS: build command for `arc_coverage_multinode`. (If the `coverage_gap` helper has a fencepost, fix the helper — it is test-local.)
- [ ] Commit:
```
git add elohim/elohim-storage/tests/arc_coverage_multinode.rs
git commit -m "test(storage): arc coverage-floor MULTI-NODE keyspace invariant (#7c)

Proves the cluster property derive() only asserts per-node: union of N
nodes' arcs covers [0,1) with no gap above the 1/N floor, and opens a
gap below it. Verify-only on ArcDecision shape (RESOLUTION-H owner).

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 4 — No-overwhelm soak (#7d) [L]

Files: `elohim/elohim-storage/tests/no_overwhelm_soak.rs` (create, `#[ignore]`). Consumes the chaos `Node`/`spawn_node`/`.kill()` churn primitive — BUT `chaos_dataplane.rs` is a sibling test file, not a crate-exported module, so its helpers are NOT importable across test binaries. **Resolution:** the soak is self-contained — it re-uses the SAME production protocol (`BlobCodec`/`BlobProtocol`/`BlobFetchRequest`) and a local minimal node driver (copied-minimal, not a fork of the whole chaos harness; ~40 lines). Flag in FOLLOW-ON: a shared `tests/common/chaos_node.rs` module is the right home for both — left for the integration pass.

**FIRST verify `chaos_dataplane.rs` compiles on libp2p 0.54.1** (it uses `with_codec` at line 113 — a pre-0.54 idiom per CLAUDE.md). Run the chaos suite before building on its pattern; if `with_codec` no longer compiles, the soak's node driver must use `Behaviour::new([(Proto, ProtocolSupport::Full)], cfg)` (the 0.54.1 idiom) instead.

- [ ] Verify chaos compiles: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-proofs RUSTC_WRAPPER="" cargo test --test chaos_dataplane 2>&1 | tail -40`. Record which codec idiom compiles; use that one in the soak.
- [ ] Write the soak test (`#[ignore]`, sustained concurrency + churn, assert bounded outcome / no-hang):
```rust
//! #7(d): no-overwhelm SOAK. Sustained concurrent fetch load against a peer
//! set under churn; assert every fetch resolves to a bounded outcome (bytes
//! or error) within a deadline -- the system never hangs or unbounds.
//! #[ignore]-gated (long-running); run with `-- --ignored`.
//!
//! SOFT-depends P-TRANSPORT: when connection_limits (max_established) has
//! landed, assert the cap holds under load; until then this documents the
//! unbounded-connection behavior (the floor-hole the soak exists to close).

use std::time::{Duration, Instant};
// ... minimal local node driver over BlobCodec/BlobProtocol (0.54.1 idiom
//     confirmed in the pre-step), serving a fixed holdings map ...

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "soak: long-running sustained-concurrency churn (#7d)"]
async fn sustained_concurrency_under_churn_stays_bounded() {
    // 1. spawn a small provider set holding the same blob.
    // 2. spawn a fetcher; fire SUSTAINED concurrent fetches (e.g. 200 in
    //    flight) for a fixed wall-clock window (e.g. 30s).
    // 3. mid-window, KILL and REBIRTH providers on a churn cadence.
    // 4. ASSERT: every fetch future resolves within a per-fetch deadline
    //    (bounded outcome), total resolved == total issued (no leaks/hangs),
    //    and the run completes under the window + margin (no runtime peg).
    let deadline = Duration::from_secs(60);
    let start = Instant::now();
    // ... drive the load; collect outcomes ...
    assert!(start.elapsed() < deadline, "soak must not exceed bounded window");
    // when connection_limits present, additionally assert observed established
    // connections never exceeded the configured cap.
}
```
- [ ] Run with `--ignored`, expect PASS (bounded): the `no_overwhelm_soak -- --ignored` command. If it HANGS, that is the floor-hole — leave the test as the documented RED with a `// FOLLOW-ON: requires P-TRANSPORT connection_limits` note and do NOT mark the suite red in CI (it is `#[ignore]`).
- [ ] Commit:
```
git add elohim/elohim-storage/tests/no_overwhelm_soak.rs
git commit -m "test(storage): no-overwhelm soak — bounded outcomes under sustained churn (#7d)

First sustained-concurrency-under-backpressure soak (vs the sequential
p99 bench). #[ignore]-gated. Soft-consumes P-TRANSPORT connection_limits.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 5 — Revive the P2P-sim: migrate simulate.sh to nerdctl (#7e) [L]

Files: `steward/node/simulation/simulate.sh` (mutate). The exit-127 is daemonless-docker; `nerdctl` is present in the build container.

- [ ] Verify the compose surface and nerdctl: `cd /projects/elohim/steward/node/simulation && ls docker-compose*.yml 2>/dev/null; command -v nerdctl docker docker-compose 2>&1`. Record which compose file(s) and whether `nerdctl compose` parses them (`nerdctl -n k8s.io compose -f docker-compose.yml config` if present).
- [ ] Migrate (exact substitutions across the script):
  - `docker-compose --profile latency up -d` / `docker-compose up -d` → `nerdctl compose --profile latency up -d` / `nerdctl compose up -d`
  - `docker-compose down` / `down -v --rmi local --remove-orphans` → `nerdctl compose down` (+ same flags)
  - `docker-compose logs -f` → `nerdctl compose logs -f`
  - `docker ps --format '{{.Names}}'` → `nerdctl ps --format '{{.Names}}'`
  - `docker network disconnect/connect simulation_wan-bridge ...` → `nerdctl network disconnect/connect ...`
  - `docker exec -it "$node" /bin/sh` → `nerdctl exec -it "$node" /bin/sh`
  - keep all `curl`/health-poll logic and the advisory return semantics unchanged.
- [ ] If the build container needs the `-n k8s.io` namespace (the Jenkinsfile uses it for every nerdctl call), gate it: `NERDCTL="nerdctl ${NERDCTL_NS:+-n $NERDCTL_NS}"` near the top, default `NERDCTL_NS=k8s.io` only when run in CI (via an env the stage sets), and use `$NERDCTL` everywhere. Local runs (no namespace) still work.
- [ ] Syntax-check: `bash -n /projects/elohim/steward/node/simulation/simulate.sh`.
- [ ] Re-gate the Jenkinsfile stage — **hand-off note to the edge-Jenkinsfile owner** (the stage at `elohim/holochain/Jenkinsfile:1375` already wraps `runSimulationTest()` in `catchError(buildResult:'SUCCESS', stageResult:'UNSTABLE')`; KEEP it advisory until one green run). The only change here is the comment at the helper (line 164-165): replace "Requires Docker Compose" with "Requires nerdctl compose (daemonless containerd)". Do NOT inline any bash into the Jenkinsfile (CPS size-limit gotcha) — `sh './simulate.sh test'` stays as-is.
- [ ] Commit:
```
git add steward/node/simulation/simulate.sh elohim/holochain/Jenkinsfile
git commit -m "fix(sim): migrate P2P simulation harness docker-compose -> nerdctl compose (#7e)

Revives the exit-127 P2P-sim stage: the build pod is daemonless
(containerd/nerdctl, no docker). Keeps the advisory-UNSTABLE wrapper
until one green edge CI run verifies. Stage call stays heredoc-free.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 6 — Map proofs to the D-matrix + un-wip chaos-peer-churn.feature (#7 acceptance) [S]

Files: `genesis/a2o/features/resilience/chaos-peer-churn.feature` (mutate). Tie the new proofs to the Approved spec's acceptance gate; un-`@wip` the dynamic-churn rows the proofs now back.

- [ ] Read the full feature; for each `@wip` scenario whose property is now PROVEN deterministically (flapping-idempotence ↔ chaos `kill`+rebirth; simultaneous-loss ↔ placement-diversity Task 2; mid-read kill ↔ chaos property 4), either remove `@wip` (if the live-cluster drill rail is ratified per the EPR durability arc) OR add a `@requires:shem` / `@needs-ratified-drill` tag and a comment pointing to the backing deterministic test (`elohim-storage/tests/{rs_reconstruct_property,placement_diversity_invariant}.rs`, `tests/chaos_dataplane.rs`). Do NOT un-wip a row whose destructive pod-op is not operator-ratified — tag it and cite the deterministic backstop instead.
- [ ] Add a comment block at the top mapping D1-D9 → backing test file (the acceptance ledger).
- [ ] Commit:
```
git add genesis/a2o/features/resilience/chaos-peer-churn.feature
git commit -m "test(a2o): map resilience proofs to D-matrix; un-wip backed chaos rows (#7)

Ties the new deterministic proofs (rs-any-K, placement-diversity,
arc multi-node coverage) to the Approved resilience-dimensions
acceptance gate. Rows whose destructive drill is unratified stay tagged
with a cite to their deterministic backstop.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## // FOLLOW-ON seams (deliberately left for the integration pass)

- **`tests/common/chaos_node.rs` shared module.** Task 4's soak copies a minimal node driver because `chaos_dataplane.rs` helpers are not importable across test binaries. The right home is a shared `tests/common/` module consumed by BOTH chaos + soak. Left out to keep this plan from mutating `chaos_dataplane.rs` (which was just rewritten, commit 1dd50b5c8, and may be mid-flight on another track).
- **`peer_selection::rank_by_diversity` pure extraction (D4-L2).** The greedy diversity passes are inline in the DB-bound `select()`. Extracting a pure `rank_by_diversity(candidates, desired) -> Vec<SelectedPeer>` would let Task 2 test the RANKING (not just the mapping). Left as a `src/` refactor seam — out of scope for a test-only plan, and it touches a selector other tracks may consume.
- **`distribute_shards` diversity-aware placement fix.** Task 2's `#[ignore]` test names the `i % len` concentration bug. The FIX (refuse/degrade when peers < shards rather than concentrate) is a `p2p/mod.rs` `src/` change — a distinct work item, not a proof.
- **`arc_policy::keyspace_covered` production promotion (D3).** If a runtime caller ever needs the multi-node coverage check, promote `coverage_gap` from test-local to `arc_policy`. Not now.
- **CI green-run verification of the revived sim (Task 5).** The advisory-UNSTABLE wrapper stays until ONE green edge CI run proves the nerdctl migration. Triggering that run (a `[build:edge]` push) is operator-owned (anonymous Jenkins MCP can't trigger builds).

---

## Dispatch note

- **Isolated worktree, subagent-driven, commit-only.** Create a worktree off the shift branch; run each task as a TDD subagent (failing test → minimal impl → green → commit). The integrator pushes; do NOT `git push` or merge.
- **Wave placement (ledger §6):** Tasks 1, 2 are WAVE-1 roots (no inbound edges). Tasks 3, 4 are WAVE-2 (soft-consume arc/transport shapes, verify-only). Tasks 5, 6 are independent (dispatch any time). No file is shared with another plan's mutator except `simulate.sh`/edge-`Jenkinsfile` (operator hand-off, not another plan).
- **Per-task gate:** each task ends with its own build/test command (above) at green before commit; run the final whole-`--tests` + clippy + fmt gate before declaring done. Working draft — NOT cite-sealed.
