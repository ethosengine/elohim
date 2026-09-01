---
status: landed
landed: 2026-06-10
verify_evidence: "elohim-holochain dev #1321 SUCCESS (sweettests incl. mishpat wall validator); elohim dev #1523/#1524 build+unit stages green (deploy-side red = degraded-alpha substrate family, excluded); local: storage lib 1497/0, mishpat native 34/0, both wasm-checks"
---
# Per-Substrate Limitarian Governor v1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the spec's §11 v1 slice — a scale-invariant concentration measure driving continuous limitarian demurrage, a `concentration_snapshot` projection, the DNA wall validator, and the ratify-writeback that closes the dead ratification seam — proven by the convergence + firewall + anti-capture tests on household-nodes.

**Architecture:** Pure-math measure module in elohim-storage (v1 placement; see arc journal decision #2) feeds a continuous decay-rate replacing the 5-rung step in `token_decay_service`; a Phase-A aggregator snapshots per-layer concentration (k≥5, no per-agent identity) over token balances (per spec §11.3 until substrate_signal generalization); ratification rides the existing Mishpat Commitment + content_store governance-action pipelines with a new reject-at-write wall validator; a storage projector finally writes `ratified_by/ratified_at/dht_anchor_hash`.

**Tech Stack:** Rust (elohim-storage diesel/SQLite; mishpat + content_store zomes, plain cargo for DNA), spec: `genesis/docs/superpowers/specs/2026-06-09-per-substrate-limitarian-governor-design.md`.

**Build environments (container quirks — read first):**
- elohim-storage: `RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/cargo-target-elohim-storage cargo test --lib <filter>` (pool slot hits fingerprint-ENOENT; /tmp is the documented recovery). Plain `cargo test`, never nextest here.
- DNA workspaces: plain cargo, NO target redirect for `just pack`; for type-checks use `RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/dna-<name>-check cargo check --target wasm32-unknown-unknown` from the workspace dir (check-only redirect is safe; pack is not needed in this plan).

**Interpretive decisions in force (journaled at `.claude/shifts/2026-06-10-overnight-delivery-stasis.journal.md`):**
1. v1 computes C over token balances (spec §11.3 explicit) — substrate_signal generalization is follow-on.
2. Measure module lives at `elohim/elohim-storage/src/services/measure.rs` (no elohim-core crate exists; DNA needs only wall constants).
3. Rate-function signature follows §11's test (single composite `C` drives shape; S_q folds into C via w_s) — not §3's separate tail term.

---

### Task 1: The measure module (pure math + property tests)

**Files:**
- Create: `elohim/elohim-storage/src/services/measure.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs` (add `pub mod measure;` alongside existing `pub mod` lines)

- [ ] **Step 1: Write the module with tests included** (pure no-I/O; tests in-file `#[cfg(test)]`)

```rust
//! Concentration measures for the per-substrate limitarian governor.
//!
//! Pure, no-I/O math (spec §2, per-substrate-limitarian-governor-design):
//! scale-invariant (z(λD)=z(D)), tail-sensitive, decomposable. Gini ships as a
//! human-readable diagnostic and convergence-test target — it is NOT a friction
//! driver. v1 placement: storage-local (arc decision #2); graduates to a shared
//! crate when a second consumer (WASM measure math) appears.

/// Generalized Entropy index GE(α) over a non-negative distribution.
/// α ∈ [1,2] by DNA wall; α=1 is Theil-T (computed as the α→1 limit),
/// α=2 is maximally tail-sensitive. Returns 0.0 for empty/degenerate input.
pub fn ge_alpha(xs: &[f32], alpha: f32) -> f32 {
    let n = xs.len();
    if n == 0 {
        return 0.0;
    }
    let mu: f64 = xs.iter().map(|&x| x as f64).sum::<f64>() / n as f64;
    if mu <= 0.0 {
        return 0.0;
    }
    let a = alpha as f64;
    if (a - 1.0).abs() < 1e-6 {
        // Theil-T: (1/N) Σ (x/μ)·ln(x/μ); zero terms contribute 0 (lim x·ln x = 0).
        let s: f64 = xs
            .iter()
            .map(|&x| {
                let r = x as f64 / mu;
                if r > 0.0 { r * r.ln() } else { 0.0 }
            })
            .sum();
        (s / n as f64) as f32
    } else {
        let s: f64 = xs.iter().map(|&x| ((x as f64 / mu).powf(a)) - 1.0).sum();
        (s / (n as f64 * a * (a - 1.0))) as f32
    }
}

/// squash(g) = g/(1+g) — the spec's mandated normalizer: fixed, monotone,
/// scale-invariance-preserving (a function of already-scale-invariant GE,
/// introducing NO N- or μ-dependence). Do NOT replace with GE/GE_max(N).
pub fn squash(g: f32) -> f32 {
    if g <= 0.0 { 0.0 } else { g / (1.0 + g) }
}

/// Share of the total held by the top ⌈q·N⌉ holders (q=0.01 default).
/// With small N, the top-1 holder is the tail (⌈q·N⌉ ≥ 1 always).
pub fn top_quantile_share(xs: &[f32], q: f32) -> f32 {
    if xs.is_empty() {
        return 0.0;
    }
    let total: f64 = xs.iter().map(|&x| x as f64).sum();
    if total <= 0.0 {
        return 0.0;
    }
    let mut sorted: Vec<f64> = xs.iter().map(|&x| x as f64).collect();
    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    let k = ((q as f64 * xs.len() as f64).ceil() as usize).max(1).min(xs.len());
    let top: f64 = sorted[..k].iter().sum();
    (top / total) as f32
}

/// Gini coefficient — DIAGNOSTIC ONLY (human-readable; the convergence test's
/// secondary series). Never a friction driver (spec §2).
pub fn gini(xs: &[f32]) -> f32 {
    let n = xs.len();
    if n == 0 {
        return 0.0;
    }
    let mut sorted: Vec<f64> = xs.iter().map(|&x| x as f64).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let total: f64 = sorted.iter().sum();
    if total <= 0.0 {
        return 0.0;
    }
    // G = (2·Σ i·x_(i) / (N·Σx)) − (N+1)/N  with 1-based ranks over ascending sort.
    let weighted: f64 = sorted
        .iter()
        .enumerate()
        .map(|(i, &x)| (i as f64 + 1.0) * x)
        .sum();
    ((2.0 * weighted) / (n as f64 * total) - (n as f64 + 1.0) / n as f64) as f32
}

/// The composite concentration C(D) = w_e·squash(GE(α)) + w_s·S_q (spec §2).
pub fn composite_concentration(xs: &[f32], alpha: f32, q: f32, w_e: f32, w_s: f32) -> f32 {
    w_e * squash(ge_alpha(xs, alpha)) + w_s * top_quantile_share(xs, q)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_invariance_z_lambda_d_equals_z_d() {
        let d = vec![100.0_f32, 250.0, 50.0, 900.0, 75.0, 320.0];
        let scaled: Vec<f32> = d.iter().map(|x| x * 37.5).collect();
        for alpha in [1.0_f32, 1.5, 2.0] {
            let a = ge_alpha(&d, alpha);
            let b = ge_alpha(&scaled, alpha);
            assert!((a - b).abs() < 1e-4, "GE(α={alpha}) not scale-invariant: {a} vs {b}");
        }
        let c1 = composite_concentration(&d, 2.0, 0.01, 0.6, 0.4);
        let c2 = composite_concentration(&scaled, 2.0, 0.01, 0.6, 0.4);
        assert!((c1 - c2).abs() < 1e-4, "composite not scale-invariant");
        assert!((gini(&d) - gini(&scaled)).abs() < 1e-4, "gini not scale-invariant");
    }

    #[test]
    fn equality_zero_on_the_equality_manifold() {
        let equal = vec![500.0_f32; 12];
        for alpha in [1.0_f32, 2.0] {
            assert!(ge_alpha(&equal, alpha).abs() < 1e-6, "GE(equal) must be 0");
        }
        assert!(gini(&equal).abs() < 1e-5, "gini(equal) must be 0");
        // top-share of an equal 12-agent distribution at q=0.01 is exactly 1/12.
        assert!((top_quantile_share(&equal, 0.01) - 1.0 / 12.0).abs() < 1e-5);
    }

    #[test]
    fn tail_sensitivity_one_giant_moves_the_measure() {
        let flat = vec![100.0_f32; 100];
        let mut spiked = flat.clone();
        spiked[0] = 100_000.0;
        assert!(
            ge_alpha(&spiked, 2.0) > ge_alpha(&flat, 2.0) + 1.0,
            "GE(2) must move strongly on a mega-concentrator"
        );
        assert!(
            top_quantile_share(&spiked, 0.01) > 0.9,
            "top-1% share must capture the giant"
        );
    }

    #[test]
    fn squash_is_bounded_monotone_and_fixed() {
        assert_eq!(squash(0.0), 0.0);
        assert!(squash(1.0) - 0.5 < 1e-6);
        assert!(squash(1e9) < 1.0);
        assert!(squash(2.0) > squash(1.0));
    }

    #[test]
    fn degenerate_inputs_do_not_panic() {
        assert_eq!(ge_alpha(&[], 2.0), 0.0);
        assert_eq!(top_quantile_share(&[], 0.01), 0.0);
        assert_eq!(gini(&[], ), 0.0);
        assert_eq!(ge_alpha(&[0.0, 0.0], 2.0), 0.0);
    }
}
```

- [ ] **Step 2: Register the module**

In `elohim/elohim-storage/src/services/mod.rs`, add `pub mod measure;` in the existing alphabetical `pub mod` list.

- [ ] **Step 3: Run the tests**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/cargo-target-elohim-storage cargo test --lib services::measure 2>&1 | tail -5`
Expected: `test result: ok. 5 passed`

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/services/measure.rs elohim/elohim-storage/src/services/mod.rs
git commit -m "feat(governor): scale-invariant concentration measures — GE(α)+squash, top-share, gini diagnostic (spec §2, v1 land 1)"
```

---

### Task 2: Continuous decay rate (replace the 5-rung step)

**Files:**
- Modify: `elohim/elohim-storage/src/services/token_decay_service.rs` (replace `calculate_decay_rate` :57-65 and the five `test_decay_rate_*` tests :217-265; `apply_decay` Steps 4-6 at :160-196 stay byte-identical)

- [ ] **Step 1: Add `GradientConfig` + the continuous rate, replacing the step fn**

Replace the `calculate_decay_rate` function (lines 57-65) with:

```rust
/// The ratifiable gradient parameters (spec §3/§5.1 payload shape; every
/// numeric default is in-wall and marked TBD-operator at the wall layer).
#[derive(Debug, Clone, Copy)]
pub struct GradientConfig {
    pub base_rate: f32,     // default 0.001 — the old Normal rung becomes the curve floor
    pub dignity_floor: f32, // sufficientarian gate; decay OFF below
    pub gamma: f32,         // rank exponent; friction exponent = 1+gamma
    pub k_max: f32,         // per-tick confiscation ceiling (top-side dignity)
    pub c_target: f32,      // governed extinction setpoint (NOT zero — spec §4.4)
    pub k_s: f32,           // shape gain
    pub alpha: f32,         // GE order, wall [1,2]
    pub q: f32,             // top-share quantile
    pub w_e: f32,           // composite weight: shape term
    pub w_s: f32,           // composite weight: tail term
}

impl Default for GradientConfig {
    fn default() -> Self {
        // In-wall, value-laden core defaults (spec §5.1 payload defaults).
        Self { base_rate: 0.001, dignity_floor: 100.0, gamma: 1.0, k_max: 0.05,
               c_target: 0.15, k_s: 0.5, alpha: 1.0, q: 0.01, w_e: 0.6, w_s: 0.4 }
    }
}

/// Continuous limitarian rate (spec §3, signature per the §11 done-when test —
/// arc decision #3: the composite C drives the shape term; S_q is folded into
/// C via w_s, not a separate factor).
///
///   shape(C)   = 1 + k_s · relu(C − C_target)
///   rank(b̂)    = b̂^γ          (b̂ = b_i/μ — the agent's RELATIONAL position)
///   rate       = clamp(base_rate · shape · rank, 0, k_max)
///
/// The sufficientarian floor_factor is applied by the CALLER (decay is skipped
/// entirely below dignity_floor); downstream of the rate, apply_decay Steps 4-6
/// (decay_amount, the .max(dignity_floor) clamp, the audit row) are REUSED
/// VERBATIM (spec §3 honest costing).
pub fn calculate_decay_rate_continuous(b_hat: f32, c: f32, g: &GradientConfig) -> f32 {
    let shape = 1.0 + g.k_s * (c - g.c_target).max(0.0);
    let rank = b_hat.max(0.0).powf(g.gamma);
    (g.base_rate * shape * rank).clamp(0.0, g.k_max)
}
```

**Search check before replacing:** `grep -n "calculate_decay_rate(" elohim/elohim-storage/src` — every existing CALLER of the old step function must be updated in the same task. If a caller derives its argument from `ObligationLevel` (e.g. in `apply_decay` Step ~3 or `api/token.rs`), change that call site to: compute `let cfg = GradientConfig::default();` (Task 6 replaces this with the registry read), `let b_hat = balance / mu_estimate;` where for THIS task `mu_estimate = config.median_estimate` (the existing field — Task 6 swaps it to the snapshot's μ), `let c = 0.0;` placeholder until Task 6 wires the snapshot (rate then equals `base_rate·b_hat^γ` clamped — strictly continuous, backward-leaning). Keep `ObligationLevel` derivation for the audit `label` only (it remains an audit string per spec §3).

- [ ] **Step 2: Replace the five rung tests with curve-sample tests**

Replace `test_decay_rate_supported_zero` … `test_decay_rate_extreme` (:217-265) with:

```rust
    #[test]
    fn continuous_rate_at_target_equals_base_rate_for_mean_agent() {
        let g = GradientConfig::default();
        // C at target, agent at the mean (b̂=1): rate collapses to base_rate.
        let r = calculate_decay_rate_continuous(1.0, g.c_target, &g);
        assert!((r - g.base_rate).abs() < 1e-6, "rate at (b̂=1, C=C_target) must be base_rate, got {r}");
    }

    #[test]
    fn continuous_rate_monotone_in_concentration() {
        let g = GradientConfig::default();
        let r_low = calculate_decay_rate_continuous(2.0, g.c_target, &g);
        let r_mid = calculate_decay_rate_continuous(2.0, g.c_target + 0.2, &g);
        let r_high = calculate_decay_rate_continuous(2.0, g.c_target + 0.5, &g);
        assert!(r_low < r_mid && r_mid < r_high, "rate must rise with C: {r_low} {r_mid} {r_high}");
    }

    #[test]
    fn continuous_rate_monotone_in_relational_position() {
        let g = GradientConfig::default();
        let c = g.c_target + 0.3;
        let r1 = calculate_decay_rate_continuous(1.0, c, &g);
        let r4 = calculate_decay_rate_continuous(4.0, c, &g);
        assert!(r4 > r1, "rate must rise with b̂ (super-linear friction): {r1} vs {r4}");
    }

    #[test]
    fn continuous_rate_clamped_at_k_max() {
        let g = GradientConfig::default();
        let r = calculate_decay_rate_continuous(1_000.0, 0.99, &g);
        assert!((r - g.k_max).abs() < 1e-6, "rate must clamp at k_max, got {r}");
    }

    #[test]
    fn continuous_rate_never_negative_below_target() {
        let g = GradientConfig::default();
        // C below target: relu kills the shape excess; rate = base_rate·b̂^γ ≥ 0.
        let r = calculate_decay_rate_continuous(0.5, 0.0, &g);
        assert!(r >= 0.0 && r <= g.base_rate, "below-target rate bounded by base_rate·b̂^γ, got {r}");
    }
```

- [ ] **Step 3: Run the service tests + the full lib filter for regressions**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/cargo-target-elohim-storage cargo test --lib token_decay 2>&1 | tail -4`
Expected: all token_decay tests pass (5 new curve tests + the existing apply_decay tests untouched).

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/services/token_decay_service.rs elohim/elohim-storage/src/api/token.rs
git commit -m "feat(governor): continuous limitarian decay rate replaces the 5-rung step — apply_decay Steps 4-6 reused verbatim (spec §3, v1 land 2)"
```

---

### Task 3: `concentration_snapshot` — migration + schema + model + db fns

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-06-10-020000_concentration_snapshot/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-06-10-020000_concentration_snapshot/down.sql`
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs` (append table! block)
- Modify: `elohim/elohim-storage/src/db/models.rs` (append structs)
- Create: `elohim/elohim-storage/src/db/concentration_snapshots.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs` (register module)

- [ ] **Step 1: Write the migration** (mold: 2026-06-09-000000_mishpat_commitments)

`up.sql`:
```sql
-- Per-layer concentration snapshot — the governor's measured state (spec §11 land 3).
-- Source of truth: NONE — Classification C (Operational): recomputed-on-read
-- aggregate, rebuildable by event replay. DELIBERATELY NOT DHT-ANCHORED (no
-- dht_anchor_hash by design): anchoring would notarize what must stay
-- operational — spec §4.4's A/C seam invariant ("computed edges/values are
-- never given a dht_anchor_hash"). CARRIES NO PER-AGENT IDENTITY (k>=5
-- firewall: writer refuses n<5).
-- v1 computes over token balances (spec §11.3); generalizes per-substrate once
-- EconomicEvent.substrate_signal coverage widens.
CREATE TABLE concentration_snapshots (
    id TEXT PRIMARY KEY,                -- slug: {substrate_signal}:{governance_layer}:{computed_at}
    h_app_id TEXT NOT NULL DEFAULT 'shefa',
    substrate_signal TEXT NOT NULL DEFAULT 'attention',
    governance_layer TEXT NOT NULL,
    n INTEGER NOT NULL,                 -- population size (>=5 enforced by writer)
    mu REAL NOT NULL,                   -- distribution mean (b_hat denominator)
    ge REAL NOT NULL,                   -- raw GE(alpha)
    ge_squashed REAL NOT NULL,          -- squash(GE) = GE/(1+GE)
    top_share REAL NOT NULL,            -- S_q
    gini REAL NOT NULL,                 -- diagnostic only, never a driver
    c_composite REAL NOT NULL,          -- w_e*ge_squashed + w_s*top_share
    alpha REAL NOT NULL,
    q REAL NOT NULL,
    computed_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_concentration_snapshots_lookup
    ON concentration_snapshots(h_app_id, substrate_signal, governance_layer, computed_at);
```

`down.sql`:
```sql
DROP INDEX IF EXISTS idx_concentration_snapshots_lookup;
DROP TABLE IF EXISTS concentration_snapshots;
```

**Timestamp-collision check (memory `feedback_diesel_migration_timestamp_collision`):** `ls elohim/elohim-storage/migrations/ | grep 2026-06-10` — if `020000` collides with an existing entry, bump to the next free `0N0000`.

- [ ] **Step 2: diesel_schema.rs table! block** (append after the mishpat_commitments block, with the migration comment line):

```rust
// Migration: 2026-06-10-020000_concentration_snapshot
// Source of truth: NONE (Category C operational aggregate — see migration header;
// deliberately no dht_anchor_hash, spec §4.4).
diesel::table! {
    concentration_snapshots (id) {
        id -> Text,
        h_app_id -> Text,
        substrate_signal -> Text,
        governance_layer -> Text,
        n -> Integer,
        mu -> Float,
        ge -> Float,
        ge_squashed -> Float,
        top_share -> Float,
        gini -> Float,
        c_composite -> Float,
        alpha -> Float,
        q -> Float,
        computed_at -> Text,
    }
}
```
Also add `concentration_snapshots` to the `allow_tables_to_appear_in_same_query!` block if the file maintains one (grep for it; follow whatever the mishpat_commitments entry did).

- [ ] **Step 3: models.rs structs** (append near MishpatCommitment, mirroring its derive set):

```rust
/// Per-layer concentration snapshot — Classification C (operational aggregate).
/// Source of truth: NONE (rebuildable by replay; deliberately NOT DHT-anchored —
/// spec §4.4 A/C seam invariant). NO per-agent identity by construction
/// (firewall test enforces).
#[derive(Debug, Clone, Queryable, Serialize)]
pub struct ConcentrationSnapshot {
    pub id: String,
    pub h_app_id: String,
    pub substrate_signal: String,
    pub governance_layer: String,
    pub n: i32,
    pub mu: f32,
    pub ge: f32,
    pub ge_squashed: f32,
    pub top_share: f32,
    pub gini: f32,
    pub c_composite: f32,
    pub alpha: f32,
    pub q: f32,
    pub computed_at: String,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = concentration_snapshots)]
pub struct NewConcentrationSnapshot {
    pub id: String,
    pub h_app_id: String,
    pub substrate_signal: String,
    pub governance_layer: String,
    pub n: i32,
    pub mu: f32,
    pub ge: f32,
    pub ge_squashed: f32,
    pub top_share: f32,
    pub gini: f32,
    pub c_composite: f32,
    pub alpha: f32,
    pub q: f32,
}
```
(Match the file's existing import pattern for `Queryable/Insertable/Serialize` and the `diesel(table_name = …)` path — copy whatever MishpatCommitment uses.)

- [ ] **Step 4: db/concentration_snapshots.rs** (insert + latest-lookup):

```rust
//! concentration_snapshots — insert + latest-effective lookup.
//! Writer-side k>=5 firewall lives in the SERVICE (concentration_service);
//! this layer is mechanical.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use super::diesel_schema::concentration_snapshots as cs;
use super::models::{ConcentrationSnapshot, NewConcentrationSnapshot};

pub fn insert_snapshot(
    conn: &mut SqliteConnection,
    new: NewConcentrationSnapshot,
) -> QueryResult<ConcentrationSnapshot> {
    diesel::insert_into(cs::concentration_snapshots)
        .values(&new)
        .execute(conn)?;
    cs::concentration_snapshots
        .filter(cs::id.eq(&new.id))
        .first(conn)
}

/// Most recent snapshot for (h_app_id, substrate_signal, governance_layer) —
/// the effective C the decay path reads. None = never computed (callers fall
/// back to GradientConfig defaults with C treated as c_target → base-rate-only).
pub fn latest_snapshot(
    conn: &mut SqliteConnection,
    h_app_id: &str,
    substrate_signal: &str,
    governance_layer: &str,
) -> QueryResult<Option<ConcentrationSnapshot>> {
    cs::concentration_snapshots
        .filter(cs::h_app_id.eq(h_app_id))
        .filter(cs::substrate_signal.eq(substrate_signal))
        .filter(cs::governance_layer.eq(governance_layer))
        .order(cs::computed_at.desc())
        .first(conn)
        .optional()
}
```

Register in `db/mod.rs`: `pub mod concentration_snapshots;` (alphabetical).

- [ ] **Step 5: Run a lib build + a quick insert/lookup test** — add to `db/concentration_snapshots.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::test_pool;

    #[test]
    fn insert_then_latest_roundtrip() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let row = NewConcentrationSnapshot {
            id: "attention:community:2026-06-10T03:00:00Z".into(),
            h_app_id: "shefa".into(),
            substrate_signal: "attention".into(),
            governance_layer: "community".into(),
            n: 6, mu: 100.0, ge: 0.2, ge_squashed: 0.1667, top_share: 0.3,
            gini: 0.25, c_composite: 0.22, alpha: 1.0, q: 0.01,
        };
        insert_snapshot(&mut conn, row).expect("insert");
        let got = latest_snapshot(&mut conn, "shefa", "attention", "community")
            .expect("query").expect("row");
        assert_eq!(got.n, 6);
        assert!((got.c_composite - 0.22).abs() < 1e-6);
    }
}
```

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/cargo-target-elohim-storage cargo test --lib concentration_snapshots 2>&1 | tail -3`
Expected: `1 passed`.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-06-10-020000_concentration_snapshot elohim/elohim-storage/src/db/diesel_schema.rs elohim/elohim-storage/src/db/models.rs elohim/elohim-storage/src/db/concentration_snapshots.rs elohim/elohim-storage/src/db/mod.rs
git commit -m "feat(governor): concentration_snapshots table — k>=5 aggregate projection, no per-agent identity (spec §11 land 3a)"
```

---

### Task 4: Phase-A aggregator service (+ k<5 suppress + firewall test)

**Files:**
- Create: `elohim/elohim-storage/src/services/concentration_service.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs` (register)
- Possibly modify: `elohim/elohim-storage/src/db/token_balances.rs` (add a list-balances-for-layer fn if absent — check `grep -n "pub fn" src/db/token_balances.rs` first; reuse any existing list/scan)

- [ ] **Step 1: Write the service**

```rust
//! Phase-A concentration aggregator (spec §11 land 3b) — computes the per-layer
//! distribution snapshot. v1: reflexive-sensing, NO clock — driven by the test
//! harness / an HTTP poke (the aggregate-tick scheduler is explicitly deferred).
//! v1 substrate: token balances (spec §11.3).
//!
//! FIREWALL: refuses to write when n < K_MIN (k>=5, aggregator.rs mold) — a
//! sub-k snapshot would expose a small cohort's distribution.

use diesel::sqlite::SqliteConnection;

use crate::db::concentration_snapshots::{insert_snapshot, latest_snapshot};
use crate::db::models::{ConcentrationSnapshot, NewConcentrationSnapshot};
use crate::services::measure::{composite_concentration, ge_alpha, gini, squash, top_quantile_share};
use crate::services::token_decay_service::GradientConfig;

pub const K_MIN: usize = 5;

pub struct ConcentrationService;

#[derive(Debug, PartialEq)]
pub enum SnapshotOutcome {
    Written(ConcentrationSnapshot),
    SuppressedBelowK { n: usize },
}

impl ConcentrationService {
    /// Compute and persist a snapshot for one (substrate, layer) over the given
    /// distribution. The CALLER supplies the balances vector (Phase-A: from
    /// token_balances for the layer); this keeps the service pure over its input
    /// and trivially testable.
    pub fn compute_snapshot(
        conn: &mut SqliteConnection,
        h_app_id: &str,
        substrate_signal: &str,
        governance_layer: &str,
        balances: &[f32],
        g: &GradientConfig,
        computed_at: &str,
    ) -> Result<SnapshotOutcome, diesel::result::Error> {
        let n = balances.len();
        if n < K_MIN {
            return Ok(SnapshotOutcome::SuppressedBelowK { n });
        }
        let mu = balances.iter().sum::<f32>() / n as f32;
        let ge = ge_alpha(balances, g.alpha);
        let row = NewConcentrationSnapshot {
            id: format!("{substrate_signal}:{governance_layer}:{computed_at}"),
            h_app_id: h_app_id.to_string(),
            substrate_signal: substrate_signal.to_string(),
            governance_layer: governance_layer.to_string(),
            n: n as i32,
            mu,
            ge,
            ge_squashed: squash(ge),
            top_share: top_quantile_share(balances, g.q),
            gini: gini(balances),
            c_composite: composite_concentration(balances, g.alpha, g.q, g.w_e, g.w_s),
            alpha: g.alpha,
            q: g.q,
        };
        Ok(SnapshotOutcome::Written(insert_snapshot(conn, row)?))
    }

    /// Effective C for the decay path: latest snapshot's composite, or None.
    pub fn effective_c(
        conn: &mut SqliteConnection,
        h_app_id: &str,
        substrate_signal: &str,
        governance_layer: &str,
    ) -> Result<Option<f32>, diesel::result::Error> {
        Ok(latest_snapshot(conn, h_app_id, substrate_signal, governance_layer)?
            .map(|s| s.c_composite))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::test_pool;

    #[test]
    fn suppresses_below_k() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let out = ConcentrationService::compute_snapshot(
            &mut conn, "shefa", "attention", "household",
            &[100.0, 200.0, 300.0, 400.0], &GradientConfig::default(),
            "2026-06-10T03:00:00Z",
        ).expect("ok");
        assert_eq!(out, SnapshotOutcome::SuppressedBelowK { n: 4 });
        assert!(ConcentrationService::effective_c(&mut conn, "shefa", "attention", "household")
            .expect("q").is_none(), "no row may exist for a suppressed cohort");
    }

    #[test]
    fn writes_at_k_and_effective_c_reads_it() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let balances = [100.0_f32, 100.0, 100.0, 100.0, 10_000.0];
        let out = ConcentrationService::compute_snapshot(
            &mut conn, "shefa", "attention", "community", &balances,
            &GradientConfig::default(), "2026-06-10T03:00:00Z",
        ).expect("ok");
        let snap = match out { SnapshotOutcome::Written(s) => s, _ => panic!("expected write") };
        assert_eq!(snap.n, 5);
        assert!(snap.c_composite > 0.3, "skewed distribution must read concentrated");
        let c = ConcentrationService::effective_c(&mut conn, "shefa", "attention", "community")
            .expect("q").expect("some");
        assert!((c - snap.c_composite).abs() < 1e-6);
    }

    /// Firewall (spec §8.1, aggregator.rs:491 mold): exhaustive construction +
    /// serialize-absent — the snapshot carries NO per-agent identity and cannot
    /// grow one without breaking this test's compile or asserts.
    #[test]
    fn snapshot_struct_has_no_peer_identity() {
        let row = NewConcentrationSnapshot {
            id: "attention:community:t".into(), h_app_id: "shefa".into(),
            substrate_signal: "attention".into(), governance_layer: "community".into(),
            n: 5, mu: 1.0, ge: 0.0, ge_squashed: 0.0, top_share: 0.2,
            gini: 0.0, c_composite: 0.08, alpha: 1.0, q: 0.01,
        };
        let json = serde_json::to_string(&serde_json::json!({
            "id": row.id, "hAppId": row.h_app_id, "substrateSignal": row.substrate_signal,
            "governanceLayer": row.governance_layer, "n": row.n, "mu": row.mu,
            "ge": row.ge, "geSquashed": row.ge_squashed, "topShare": row.top_share,
            "gini": row.gini, "cComposite": row.c_composite, "alpha": row.alpha, "q": row.q,
        })).expect("serialize");
        for leak in ["agent_id", "agentId", "pubkey", "agentKey", "signer", "human_id", "humanId", "balance"] {
            assert!(!json.contains(leak), "snapshot must not carry per-agent field '{leak}': {json}");
        }
    }
}
```

- [ ] **Step 2: Register + run**

Add `pub mod concentration_service;` to services/mod.rs.
Run: `… cargo test --lib concentration_service 2>&1 | tail -3` → Expected: `3 passed`.

- [ ] **Step 3: Commit**

```bash
git add elohim/elohim-storage/src/services/concentration_service.rs elohim/elohim-storage/src/services/mod.rs
git commit -m "feat(governor): Phase-A concentration aggregator — k>=5 suppress, firewall test, effective_c reader (spec §11 land 3b)"
```

---

### Task 5: LimitGradientRegistry + the constitutional CID-stub fix

**Files:**
- Create: `elohim/elohim-storage/src/services/limit_gradient_registry.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs` (register)
- Modify: `elohim/elohim-storage/src/services/constitutional_ratio_registry.rs:141-144` (CID stub)

- [ ] **Step 1: Find the real CID helper** — run `grep -rn "pub fn compute_cid\|fn cid" elohim/elohim-storage/src/epr/ crates/ elohim/ --include="*.rs" | grep -i cid | head -5`. Use whatever `epr::cid` function the spec's §12 points at (`epr/src/cid.rs:12`). If the function hashes bytes, fix the stub to hash the manifest file bytes:

```rust
fn compute_manifest_cid(path: &str) -> String {
    // Substrate-correct CID: hash the manifest bytes via the EPR cid module
    // (spec per-substrate-limitarian-governor-design §6.2 — the governed EPR
    // must be content-addressed-in-fact, not fingerprinted-by-path).
    match std::fs::read(path) {
        Ok(bytes) => crate::epr::cid::compute_cid(&bytes),
        Err(_) => format!("manifest-missing:{path}"),
    }
}
```
(Adjust the call path to the real module location/signature found by the grep. If no byte-hashing helper exists, keep the stub UNCHANGED and record a wishlist line in the arc journal — do not invent a CID format.)

- [ ] **Step 2: Write the registry** (mold: constitutional_ratio_registry)

```rust
//! LimitGradientRegistry — the effective gradient for (substrate, layer):
//! core value-laden defaults, DNA-wall-clamped (spec §6.2). The registry clamps
//! its OWN DEFAULT OUTPUT only; ratified values are wall-checked at
//! create_commitment by the DNA validator (§5.2 — reject-at-write, never
//! silently clamp a ratified truth). Ratified overrides arrive via the
//! responsibility_demand_configs projection once the writeback lands (Task 8);
//! v1's effective lookup: ratified row if ratified_by is set, else core default.
//!
//! WALL WIDTHS ARE TBD-OPERATOR (spec §Decision 2): the SHAPE of each wall is
//! decided; the numerics below are asserted defaults awaiting derivation.

use crate::services::token_decay_service::GradientConfig;

// DNA-wall mirror (native side). Keep in lockstep with mishpat_integrity wall
// constants (Task 7) — the validator there is authoritative at write time.
pub const ALPHA_WALL: (f32, f32) = (1.0, 2.0);        // cannot blind the tail (α=0 forbidden)
pub const C_TARGET_WALL: (f32, f32) = (0.05, 0.30);   // TBD-operator
pub const K_MAX_WALL: (f32, f32) = (0.01, 0.10);      // TBD-operator
pub const BASE_RATE_WALL: (f32, f32) = (0.0005, 0.005); // TBD-operator
pub const GAMMA_WALL: (f32, f32) = (0.5, 2.0);        // TBD-operator

pub struct LimitGradientRegistry;

impl LimitGradientRegistry {
    /// Core value-laden default for a substrate/layer, wall-clamped.
    /// v1: layer-defaulted alpha (small-N household → 1.0; community+ → 2.0).
    pub fn core_default(_substrate_signal: &str, governance_layer: &str) -> GradientConfig {
        let mut g = GradientConfig::default();
        g.alpha = match governance_layer {
            "individual" | "household" => 1.0,
            _ => 2.0,
        };
        Self::clamp_to_walls(g)
    }

    pub fn clamp_to_walls(mut g: GradientConfig) -> GradientConfig {
        g.alpha = g.alpha.clamp(ALPHA_WALL.0, ALPHA_WALL.1);
        g.c_target = g.c_target.clamp(C_TARGET_WALL.0, C_TARGET_WALL.1);
        g.k_max = g.k_max.clamp(K_MAX_WALL.0, K_MAX_WALL.1);
        g.base_rate = g.base_rate.clamp(BASE_RATE_WALL.0, BASE_RATE_WALL.1);
        g.gamma = g.gamma.clamp(GAMMA_WALL.0, GAMMA_WALL.1);
        g
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_in_wall() {
        for layer in ["household", "community", "bioregional"] {
            let g = LimitGradientRegistry::core_default("attention", layer);
            assert!(g.alpha >= ALPHA_WALL.0 && g.alpha <= ALPHA_WALL.1);
            assert!(g.c_target >= C_TARGET_WALL.0 && g.c_target <= C_TARGET_WALL.1);
            assert!(g.k_max >= K_MAX_WALL.0 && g.k_max <= K_MAX_WALL.1);
        }
    }

    #[test]
    fn layer_defaulted_alpha() {
        assert_eq!(LimitGradientRegistry::core_default("attention", "household").alpha, 1.0);
        assert_eq!(LimitGradientRegistry::core_default("attention", "community").alpha, 2.0);
    }

    #[test]
    fn clamp_pulls_out_of_wall_values_in() {
        let mut wild = GradientConfig::default();
        wild.alpha = 0.0;      // tail-blinding attempt
        wild.k_max = 1.0;      // confiscate-everything attempt
        let clamped = LimitGradientRegistry::clamp_to_walls(wild);
        assert_eq!(clamped.alpha, ALPHA_WALL.0);
        assert_eq!(clamped.k_max, K_MAX_WALL.1);
    }
}
```

- [ ] **Step 3: Run + commit**

Run: `… cargo test --lib limit_gradient_registry 2>&1 | tail -3` → `3 passed`. Also `… cargo test --lib constitutional_ratio 2>&1 | tail -3` (CID change must not break its tests; if a test asserted the `manifest-fingerprint:` prefix, that test is asserting the stub — update it to assert the new content-addressed form ONLY if the test's comment says the stub is temporary, which `:142-143` does).

```bash
git add elohim/elohim-storage/src/services/limit_gradient_registry.rs elohim/elohim-storage/src/services/mod.rs elohim/elohim-storage/src/services/constitutional_ratio_registry.rs
git commit -m "feat(governor): LimitGradientRegistry with DNA-wall mirror + constitutional CID stub fixed to content-addressing (spec §6, v1 land 4)"
```

---

### Task 6: Wire the read path — evaluate_position reads C + b̂

**Files:**
- Modify: `elohim/elohim-storage/src/services/responsibility_demand_service.rs` (evaluate_position + its callers)
- Modify: `elohim/elohim-storage/src/services/token_decay_service.rs` (apply_decay rate derivation: replace Task 2's placeholder)

- [ ] **Step 1: Rewrite the rate derivation in apply_decay's caller path.** Where Task 2 left `let c = 0.0;`, now read:

```rust
let g = crate::services::limit_gradient_registry::LimitGradientRegistry::core_default(
    "attention", governance_layer);
let c = crate::services::concentration_service::ConcentrationService::effective_c(
        conn, &ctx.h_app_id, "attention", governance_layer)?
    .unwrap_or(g.c_target); // no snapshot yet → C at target → base-rate-only (fail-coherent)
let snapshot_mu = crate::db::concentration_snapshots::latest_snapshot(
        conn, &ctx.h_app_id, "attention", governance_layer)?
    .map(|s| s.mu)
    .unwrap_or(config.median_estimate); // pre-snapshot fallback: the legacy estimate
let b_hat = if snapshot_mu > 0.0 { balance / snapshot_mu } else { 1.0 };
let rate = if balance < g.dignity_floor {
    0.0 // sufficientarian floor_factor: decay OFF below the floor (spec §3)
} else {
    calculate_decay_rate_continuous(b_hat, c, &g)
};
```

- [ ] **Step 2: Demote `evaluate_position` to a label derivation.** Keep `ObligationLevel` as the audit label (spec §3: "may survive only as an audit label derived from C-bands"). Replace the median-band body with C-band mapping (preserving the enum's payload fields):

```rust
/// AUDIT LABEL ONLY (spec §3) — derived from the snapshot C and the agent's
/// relational position; no longer a rate driver. Bands: Supported below the
/// dignity floor; then C-relative bands at the gradient's target.
pub fn evaluate_position(balance: f32, b_hat: f32, c: f32, g: &GradientConfig) -> ObligationLevel {
    if balance < g.dignity_floor {
        ObligationLevel::Supported
    } else if c <= g.c_target {
        ObligationLevel::Normal
    } else if b_hat < 2.0 {
        ObligationLevel::Elevated { visibility_required: true }
    } else if b_hat < 5.0 {
        ObligationLevel::High { stewardship_required: true, justification_required: true }
    } else {
        ObligationLevel::Extreme { elohim_review_required: true, constitutional_justification: true }
    }
}
```
Update its callers (grep `evaluate_position(` — api/token.rs and the decay path) to pass `(balance, b_hat, c, &g)` computed as in Step 1. Update this service's existing unit tests to the new signature: Supported below floor unchanged; Normal when `c <= c_target` regardless of balance; Elevated/High/Extreme by b_hat at high C.

- [ ] **Step 3: Run the touched test surfaces + commit**

Run: `… cargo test --lib responsibility_demand 2>&1 | tail -3` and `… cargo test --lib token_decay 2>&1 | tail -3` → all pass.

```bash
git add elohim/elohim-storage/src/services/responsibility_demand_service.rs elohim/elohim-storage/src/services/token_decay_service.rs elohim/elohim-storage/src/api/token.rs
git commit -m "feat(governor): decay path reads snapshot C + relational b-hat; ObligationLevel demoted to audit label (spec §3/§11 land 2b)"
```

---

### Task 7: DNA wall validator (mishpat) — `ratifies-limit-gradient`

**Files:**
- Modify: `elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs` (dispatch arm + validator + wall consts)

- [ ] **Step 1: Add the dispatch arm** in `validate_commitment_payload` (:189-198 match):

```rust
        "ratifies-limit-gradient" => validate_ratifies_limit_gradient(&payload),
```

- [ ] **Step 2: Add the wall constants + validator** (mold: `validate_delegates_compute` :330; walls mirror Task 5's registry — keep numerics in lockstep):

```rust
// DNA walls for the limitarian gradient (spec §5.2 — reject-at-write; a config
// that exists is, by construction, in-wall). Widths are TBD-operator (spec
// §Decision 2); the SHAPE (that a wall exists, that α cannot blind the tail,
// that loosening is witnessed) is decided. Mirror: storage
// limit_gradient_registry.rs — keep in lockstep.
const ALPHA_WALL: (f64, f64) = (1.0, 2.0);
const C_TARGET_WALL: (f64, f64) = (0.05, 0.30);
const K_MAX_WALL: (f64, f64) = (0.01, 0.10);
const BASE_RATE_WALL: (f64, f64) = (0.0005, 0.005);
const GAMMA_WALL: (f64, f64) = (0.5, 2.0);

fn wall_check(payload: &serde_json::Value, path: &[&str], wall: (f64, f64), name: &str) -> Result<(), String> {
    let mut v = payload;
    for key in path {
        v = v.get(key).ok_or_else(|| format!("ratifies-limit-gradient missing field: {}", path.join(".")))?;
    }
    let x = v.as_f64().ok_or_else(|| format!("{name} must be a number"))?;
    if x < wall.0 || x > wall.1 {
        return Err(format!(
            "{name}={x} outside DNA wall [{}, {}] — out-of-wall values cannot be ratified (reject-at-write)",
            wall.0, wall.1
        ));
    }
    Ok(())
}

fn validate_ratifies_limit_gradient(payload: &serde_json::Value) -> Result<(), String> {
    for field in ["substrate_signal", "governance_layer", "measure", "shape",
                  "base_rate", "k_max", "dignity_floor", "valid_from", "valid_until",
                  "ratified_by_governance_action_cid"] {
        if payload.get(field).is_none() {
            return Err(format!("ratifies-limit-gradient missing required field: {field}"));
        }
    }
    wall_check(payload, &["measure", "alpha"], ALPHA_WALL, "measure.alpha")?;
    wall_check(payload, &["shape", "C_target"], C_TARGET_WALL, "shape.C_target")?;
    wall_check(payload, &["shape", "gamma"], GAMMA_WALL, "shape.gamma")?;
    wall_check(payload, &["base_rate"], BASE_RATE_WALL, "base_rate")?;
    wall_check(payload, &["k_max"], K_MAX_WALL, "k_max")?;
    let floor = payload["dignity_floor"].as_f64().unwrap_or(-1.0);
    if floor < 0.0 {
        return Err("dignity_floor must be >= 0".into());
    }
    // Loosening witness (spec §5.4 v1-minimal): any param looser than the core
    // default requires loosening_acknowledged=true. Core defaults inline (the
    // WASM cannot read the storage registry; lockstep with GradientConfig::default).
    let loosens = payload["shape"]["C_target"].as_f64().map(|v| v > 0.15).unwrap_or(false)
        || payload["k_max"].as_f64().map(|v| v < 0.05).unwrap_or(false)
        || floor == 0.0;
    if loosens {
        let acked = payload.get("loosening_acknowledged").and_then(|v| v.as_bool()).unwrap_or(false);
        if !acked {
            return Err("loosening override requires loosening_acknowledged=true (witnessed loosening, spec §5.4)".into());
        }
    }
    Ok(())
}
```

- [ ] **Step 3: Add validator unit tests** in the file's existing `#[cfg(test)]` module (grep for `mod tests` in commitments.rs; if absent, create one — these run native, not WASM):

```rust
    fn lg_payload(c_target: f64, k_max: f64, acked: bool) -> serde_json::Value {
        serde_json::json!({
            "substrate_signal": "attention", "governance_layer": "community",
            "measure": {"alpha": 2.0, "q": 0.01, "w_e": 0.6, "w_s": 0.4},
            "shape": {"C_target": c_target, "k_s": 0.5, "gamma": 1.0},
            "base_rate": 0.001, "k_max": k_max, "dignity_floor": 50.0,
            "valid_from": "2026-06-10T00:00:00Z", "valid_until": "2026-09-10T00:00:00Z",
            "loosening_acknowledged": acked,
            "ratified_by_governance_action_cid": "uhCEk-test"
        })
    }

    #[test]
    fn in_wall_config_validates() {
        assert!(validate_ratifies_limit_gradient(&lg_payload(0.15, 0.05, false)).is_ok());
    }

    #[test]
    fn out_of_wall_c_target_rejected_at_write() {
        let err = validate_ratifies_limit_gradient(&lg_payload(0.9, 0.05, true)).unwrap_err();
        assert!(err.contains("DNA wall"), "must name the wall: {err}");
    }

    #[test]
    fn confiscatory_k_max_rejected() {
        assert!(validate_ratifies_limit_gradient(&lg_payload(0.15, 1.0, true)).is_err());
    }

    #[test]
    fn loosening_requires_acknowledgement() {
        let err = validate_ratifies_limit_gradient(&lg_payload(0.25, 0.05, false)).unwrap_err();
        assert!(err.contains("loosening_acknowledged"), "{err}");
        assert!(validate_ratifies_limit_gradient(&lg_payload(0.25, 0.05, true)).is_ok());
    }
```

- [ ] **Step 4: Native tests + WASM check + commit**

Run: `cd elohim/holochain/dna/mishpat && cargo test -p mishpat 2>&1 | tail -3` (plain cargo, native tests).
Run: `cd elohim/holochain/dna/mishpat && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/dna-mishpat-check cargo check --target wasm32-unknown-unknown 2>&1 | tail -3`
Expected: tests pass; `Finished` on the check.

```bash
git add elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs
git commit -m "feat(dna): ratifies-limit-gradient wall validator — reject-at-write, witnessed loosening (spec §5.2, v1 land 5)"
```

---

### Task 8: Governance kind + ratify-writeback projector (the dead seam)

**Files:**
- Modify: `elohim/holochain/dna/elohim/zomes/content_store/src/governance_action.rs` (kind-map :389 + the GOVERNANCE_ACTION_KINDS whitelist — find it: `grep -rn "GOVERNANCE_ACTION_KINDS" elohim/holochain/dna/elohim/zomes/`)
- Modify: `elohim/elohim-storage/src/signals.rs` (the attestation-threshold writeback arm — mold :415-445)
- Possibly modify: `elohim/elohim-storage/src/db/responsibility_demand_configs.rs` (add the ratify-update fn)

- [ ] **Step 1: DNA — extend both whitelists**

In the kind-map (`child_attestation_kind_for_governance_action` :389), add before the `_ => None`:
```rust
        "governance-action:ratify-limit-gradient" => Some("attestation:limit-gradient-approval"),
```
Add `"governance-action:ratify-limit-gradient"` to `GOVERNANCE_ACTION_KINDS` (wherever the grep finds it — integrity or coordinator const). If an `ATTESTATION_KINDS` whitelist exists and gates `issue_attestation`, add `"attestation:limit-gradient-approval"` there too (grep `attestation:renewal-approval` to find every whitelist the existing kinds appear in, and mirror ALL of them).

- [ ] **Step 2: Storage — the writeback fn** (in `db/responsibility_demand_configs.rs`; mirror the file's existing update pattern):

```rust
/// The dead-seam writeback (spec §1/§11 land 6): a PASSED ratify-limit-gradient
/// tally stamps the projection row. Upserts the (h_app_id, governance_layer)
/// row's ratification columns + the ratified gradient params.
pub fn apply_ratification(
    conn: &mut SqliteConnection,
    h_app_id: &str,
    governance_layer: &str,
    ratified_by: &str,        // the governance-action CID
    dht_anchor_hash: &str,    // the ratifying Commitment's entry hash (CID)
    dignity_floor: f32,
    ratified_at: &str,
) -> QueryResult<usize> {
    use super::diesel_schema::responsibility_demand_configs as rdc;
    diesel::update(
        rdc::responsibility_demand_configs
            .filter(rdc::h_app_id.eq(h_app_id))
            .filter(rdc::governance_layer.eq(governance_layer)),
    )
    .set((
        rdc::ratified_by.eq(ratified_by),
        rdc::ratified_at.eq(ratified_at),
        rdc::dht_anchor_hash.eq(dht_anchor_hash),
        rdc::dignity_floor.eq(dignity_floor),
        rdc::updated_at.eq(ratified_at),
    ))
    .execute(conn)
}
```
(Adjust imports/table alias to the file's existing style; if the file doesn't exist as a module, the fns live wherever `handle_create_config`'s db calls go — follow that path.)

- [ ] **Step 3: Storage — the projector arm.** Find where `attestation` signals with `parent_governance_action_cid` are counted (the `:415` key_revocation mold's enclosing function — grep `update_current_votes` callers and the attestation-kind dispatch above it). Mirror the mold for kind `attestation:limit-gradient-approval`: on threshold (use the same `required_votes`-style source the governance-action row carries — read the parent's metadata for `required_votes`/quorum via the contents projection like the mold does for key_revocations), call `apply_ratification` with: `ratified_by` = parent governance-action CID, `dht_anchor_hash` = the ratifying Commitment CID from the parent metadata's `ratified_by_governance_action_cid` back-reference if present else the parent CID, `dignity_floor` + `governance_layer` parsed from the parent metadata payload. **Keep the arm small and mold-shaped; if the attestation counting infrastructure for governance-kinds other than recovery/revocation does NOT exist** (i.e. the mold is recovery-specific with its own tables), then implement the v1 seam more directly instead: a `handle_ratify_limit_gradient_tally` fn called from wherever `ApprovalTally::tally` results are persisted with `recommendation == "pass"` (grep `recommendation` writers in storage). The DELIVERABLE is: a passed tally → `apply_ratification` row update. Document which path was taken in the commit message.

- [ ] **Step 4: Test the seam** (in the same file as `apply_ratification` or the projector):

```rust
    #[test]
    fn passed_tally_writes_the_dead_columns() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        // Seed a config row the legacy way (ratified_* NULL — the dead seam).
        // Use the existing create fn that api/token.rs handle_create_config calls.
        /* create config row for ("shefa", "community") via the existing db fn */
        apply_ratification(&mut conn, "shefa", "community",
            "uhCEk-ga-cid", "uhCEk-commitment-cid", 75.0, "2026-06-10T04:00:00Z")
            .expect("writeback");
        /* read the row back via the existing get fn */
        // assert ratified_by == Some("uhCEk-ga-cid"), dht_anchor_hash == Some(...),
        // dignity_floor == 75.0
    }
```
(Fill the seed/read calls from the file's existing test or `api/token.rs`'s create path — the exact fn names are in `db/responsibility_demand_configs.rs`; this is a 5-minute adaptation at execute time with the file open.)

- [ ] **Step 5: Run + WASM check + commit**

Run: `… cargo test --lib responsibility_demand_configs 2>&1 | tail -3` → seam test passes.
Run: `cd elohim/holochain/dna/elohim && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/dna-elohim-check cargo check --target wasm32-unknown-unknown 2>&1 | tail -3` → `Finished`.

```bash
git add elohim/holochain/dna/elohim/zomes/content_store/src/governance_action.rs elohim/elohim-storage/src/signals.rs elohim/elohim-storage/src/db/responsibility_demand_configs.rs
git commit -m "feat(governor): ratify-limit-gradient governance kind + the dead-seam writeback — passed tally stamps ratified_by/at/anchor (spec §5.3/§11 land 6)"
```

---

### Task 9: The convergence test + anti-capture property test

**Files:**
- Modify: `elohim/elohim-storage/src/services/concentration_service.rs` (append tests) — pure-math tests live here beside the service.

- [ ] **Step 1: The §11 done-when convergence test** (spec-verbatim, adjusted to the final signatures):

```rust
    /// Spec §11 "the one green convergence test" — the SHIPPED CLAMPED model
    /// under rich-get-richer inflow 4× the k_max ceiling (the regime where the
    /// old 5-rung step provably diverges).
    #[test]
    fn continuous_governor_restores_toward_target_under_rich_get_richer_inflow() {
        use crate::services::measure::composite_concentration;
        use crate::services::token_decay_service::{calculate_decay_rate_continuous, GradientConfig};

        let g = GradientConfig { base_rate: 0.001, dignity_floor: 100.0, gamma: 1.0,
                                 k_max: 0.05, c_target: 0.15, k_s: 0.5, alpha: 1.0,
                                 q: 0.01, w_e: 0.6, w_s: 0.4 };
        let c_inflow = 0.20_f32; // 4x k_max — step-divergent regime
        let mut balances = vec![100_000.0_f32, 100.0, 100.0, 100.0, 100.0];
        let mut series = vec![];
        for _ in 0..2000 {
            let mu = balances.iter().sum::<f32>() / balances.len() as f32;
            let cc = composite_concentration(&balances, g.alpha, g.q, g.w_e, g.w_s);
            series.push(cc);
            for b in balances.iter_mut() {
                let inflow = c_inflow * *b;
                let rate = if *b < g.dignity_floor { 0.0 }
                           else { calculate_decay_rate_continuous(*b / mu, cc, &g) };
                *b = (*b + inflow - rate * *b).max(g.dignity_floor);
            }
        }
        let top = balances.iter().cloned().fold(0.0_f32, f32::max);
        // (a) BOUNDED where the step diverges.
        assert!(top.is_finite() && top < 1.0e9, "runaway: {top}");
        // (b) MONOTONE DESCENT toward target in the tail (restoring force).
        let tail = &series[series.len() / 2..];
        assert!(tail.windows(2).all(|w| w[1] <= w[0] + 1e-4), "C not non-increasing in the tail");
        // (c) RESTORES TO THE TARGET, not just somewhere.
        assert!((series.last().unwrap() - g.c_target).abs() < 0.05,
            "settled at {} away from C_target {}", series.last().unwrap(), g.c_target);
        // (d) SELF-EXTINGUISHING-WHEN-JUST: equal start stays equal, friction at base.
        let equal = vec![500.0_f32; 5];
        let cc_eq = composite_concentration(&equal, g.alpha, g.q, g.w_e, g.w_s);
        let r_eq = calculate_decay_rate_continuous(1.0, cc_eq, &g);
        assert!(r_eq <= g.base_rate * 1.5, "friction at equality must sit at base_rate, got {r_eq}");
    }
```
**Note for the executor:** assertion (c)'s tolerance and the relational dynamics may interact with the small-N top-share term (S_q of 5 agents = top-1 share, floor ≈0.2 at equality → C_eq ≈ w_s·0.2 = 0.08 < c_target ✓). If (c) fails by settling slightly high, the evidence-faithful adjustments are k_s (shape gain) or the tick count — both are in-wall tuning, NOT test-loosening; journal whichever was needed. If it fails by a LARGE margin, stop and re-read spec §4.2 (saturation regime) — that's a design-level signal, bail-worthy.

- [ ] **Step 2: The anti-capture property test** (spec §8.1 dual):

```rust
    /// Anti-capture (spec §8.1): exhaustive over the in-wall param grid — NO
    /// ratifiable config can drive effective friction to zero while the
    /// distribution is concentrated. The convergence test proves the loop CAN
    /// close; this proves it CANNOT be governed open.
    #[test]
    fn no_in_wall_config_extinguishes_friction_under_concentration() {
        use crate::services::limit_gradient_registry::*;
        use crate::services::token_decay_service::{calculate_decay_rate_continuous, GradientConfig};

        let alphas = [ALPHA_WALL.0, 1.5, ALPHA_WALL.1];
        let c_targets = [C_TARGET_WALL.0, 0.15, C_TARGET_WALL.1];
        let k_maxes = [K_MAX_WALL.0, 0.05, K_MAX_WALL.1];
        let base_rates = [BASE_RATE_WALL.0, 0.001, BASE_RATE_WALL.1];
        let gammas = [GAMMA_WALL.0, 1.0, GAMMA_WALL.1];

        for &alpha in &alphas { for &c_target in &c_targets { for &k_max in &k_maxes {
        for &base_rate in &base_rates { for &gamma in &gammas {
            let g = GradientConfig { base_rate, dignity_floor: 100.0, gamma, k_max,
                                     c_target, k_s: 0.5, alpha, q: 0.01, w_e: 0.6, w_s: 0.4 };
            // A concentrated world (C far above every in-wall target), a top agent.
            let rate = calculate_decay_rate_continuous(10.0, 0.9, &g);
            assert!(rate > 0.0,
                "in-wall config governed friction open: α={alpha} C_t={c_target} k_max={k_max} base={base_rate} γ={gamma}");
            assert!(rate >= base_rate,
                "top-agent rate under high C must be at least base_rate (got {rate})");
        }}}}}
    }
```

- [ ] **Step 3: Run + commit**

Run: `… cargo test --lib concentration_service 2>&1 | tail -4` → all (now 5) pass. The convergence test is the slow one (~2000 iterations × 5 agents — still milliseconds).

```bash
git add elohim/elohim-storage/src/services/concentration_service.rs
git commit -m "test(governor): the §11 convergence test (bounded, monotone, restores-to-target, self-extinguishing) + anti-capture property sweep (spec §8.1)"
```

---

### Task 10: Story-first a2o scenario

**Files:**
- Create: `genesis/a2o/features/rms/limitarian-governor.feature`

- [ ] **Step 1: Write the feature** (mold: qahal/collective-governance.feature; API-mode; the live-loop scenario is `@wip` until the HTTP poke surface exists — v1 is harness-driven per spec §11):

```gherkin
@e2e @shefa @governance @limitarian-governor @requires:doorway @requires:seeded-content
Feature: A community ratifies the limit it cannot set for itself
  The attention economy's externality is that every participant benefits locally
  from concentration — so the limit must be supplied by the layer that
  internalizes the harm, carried as a governed EPR (witnessed, immutable,
  renewable), and enforced as smooth relational friction with a dignity floor.
  Spec: per-substrate-limitarian-governor-design (v1 slice).

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  @wip
  Scenario: An out-of-wall gradient cannot be ratified
    # DNA wall validator (spec §5.2): reject-at-write — a config that exists is
    # in-wall by construction. @wip until a zome-call probe step exists; the
    # contract is pinned native-side by validate_ratifies_limit_gradient tests.
    When a steward proposes a limit-gradient with concentration target "0.9"
    Then the commitment is rejected naming the DNA wall

  @wip
  Scenario: A passed ratification writes the governed limit
    # The dead seam (spec §1), closed: propose → M-of-N vote → tally pass →
    # the responsibility-demand config row carries ratified_by/ratified_at/
    # dht_anchor_hash. @wip until the governance-action proposal step for the
    # ratify-limit-gradient kind is wired into the step library.
    Given a community governance action "ratify-limit-gradient" with an in-wall gradient
    When the action passes its approval tally
    Then the responsibility demand config for "community" shows a ratification anchor

  @wip
  Scenario: Concentration friction relaxes only at the governed target
    # The governor extinguishes at C_target (a deliberate setpoint), never at
    # median's drifting attractor; below the dignity floor decay is OFF.
    # Native proof: continuous_governor_restores_toward_target_under_rich_get_richer_inflow.
    Given a community whose attention substrate is concentrated above its target
    When the demurrage tick applies the ratified gradient
    Then holders above the mean experience super-linear friction
    And holders below the dignity floor experience none
```

**Why all-@wip is honest here (not a dump):** the v1 slice's done-when (spec §11) is the native convergence + wall + seam tests; the a2o layer needs zome-call/HTTP probe steps that don't exist yet. The feature pins the STORY now (story-first), the steps land when the HTTP poke surface does (deferred per spec §11 "explicitly deferred: the aggregate-tick scheduler … HTTP poke"). The follow-on is captured in the close as an Objective candidate, which owns the un-@wip.

- [ ] **Step 2: Gherkin-parse check** (the parse-abort trap): `cd genesis/a2o && pnpm exec cucumber-js --dry-run features/rms/limitarian-governor.feature 2>&1 | tail -3` — expect a clean dry-run (undefined steps OK, no parse error).

- [ ] **Step 3: Commit**

```bash
git add genesis/a2o/features/rms/limitarian-governor.feature
git commit -m "story(shefa): limitarian-governor ratification + friction scenarios — story-first pin for the v1 slice (@wip pending zome-probe steps)"
```

---

### Task 11: Full gates

- [ ] **Step 1: Full storage lib suite** — `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/cargo-target-elohim-storage cargo test --lib 2>&1 | tail -3` → expected: ≥1483 + ~20 new, 0 failed.
- [ ] **Step 2: fmt + clippy** — `cargo fmt --check` (fix if needed) and `RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/cargo-target-elohim-storage cargo clippy --lib 2>&1 | grep -cE "^warning|^error"` → 0 new vs baseline (run on dev first if unsure).
- [ ] **Step 3: Both DNA wasm-checks** (commands in Tasks 7/8) → `Finished` both.
- [ ] **Step 4: mishpat native tests** — `cd elohim/holochain/dna/mishpat && cargo test -p mishpat 2>&1 | tail -3` → pass.
- [ ] **Step 5: Final commit of any gate-driven fixups**, message `chore(governor): gate fixups — fmt/clippy/test alignment`.

**Plan-level no-dump ledger:** the deferred items (HTTP poke route, aggregate-tick scheduler, Manifest home, step-defs for the @wip scenarios, substrate_signal-keyed distributions, directionality validator full form) are spec §11's explicit deferrals — the close routes them to a backlog entry `limitarian-governor-v1-followons.md` + the a2o @wip pins. The two origination decisions remain on the operator's ceiling menu (untouched by this plan).
