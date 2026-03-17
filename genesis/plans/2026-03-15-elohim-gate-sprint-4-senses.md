# ElohimGate Sprint 4: Senses — Real Trust Signal Computation

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the four placeholder trust signals (mastery_depth, steward_standing, relationship_density, governance_health) in `evaluate_gate()` with real computations from DB state — giving the gate actual senses.

**Architecture:** Four new pure-function computation modules (same pattern as `behavioral_trust.rs` and `anomaly_detection.rs`) that take DB query results and produce f64 signals in [0.0, 1.0]. The `evaluate_gate()` helper queries the DB and calls each module. No new HTTP endpoints — this sprint is entirely backend signal wiring.

**Tech Stack:** Rust, Diesel (SQLite), existing DB queries, existing test patterns.

---

## Sprint 1–3 Feedback Incorporated

- **Pure functions over services**: Signal computers are pure `fn(data) -> f64`, not stateful services.
- **Query once, compute from results**: Each signal gets its DB query in `evaluate_gate()`, same pattern as observations query.
- **No new endpoints**: These signals are internal to gate evaluation, not client-facing.
- **Fallback to 0.5 on error**: Every query failure returns neutral 0.5, never panics.
- **Governance scoping**: `query_challenges`/`query_proposals` are content-scoped. For human-level governance_health, we use disputed allocation ratio from `get_allocations_for_steward()` (already queried for steward_standing).

---

## Task 1: Mastery Depth Computation Module

**Files:**
- Create: `elohim/elohim-storage/src/services/mastery_depth.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs` (add `pub mod mastery_depth;`)

**Step 1: Write the failing test**

```rust
// elohim/elohim-storage/src/services/mastery_depth.rs
//! Mastery depth computation for trust signals.
//!
//! Converts a human's mastery records into a single depth signal (0.0-1.0)
//! reflecting how deeply they've engaged with content across Bloom's taxonomy.

use crate::db::models::ContentMastery;

/// Compute mastery depth from a human's mastery records.
///
/// Algorithm:
/// - Each mastery level maps to a depth value (not_started=0, create=1.0)
/// - Weight by freshness_score (stale mastery counts less)
/// - Average across all records
/// - Empty records → 0.5 (neutral baseline)
pub fn compute(records: &[ContentMastery]) -> f64 {
    todo!()
}

fn level_to_depth(level: &str) -> f64 {
    match level {
        "not_started" => 0.0,
        "aware" => 0.14,
        "remember" => 0.29,
        "understand" => 0.43,
        "apply" => 0.57,
        "analyze" => 0.71,
        "evaluate" => 0.86,
        "create" => 1.0,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_mastery(level: &str, freshness: f32) -> ContentMastery {
        ContentMastery {
            id: "test".to_string(),
            app_id: "lamad".to_string(),
            human_id: "human-1".to_string(),
            content_id: "content-1".to_string(),
            mastery_level: level.to_string(),
            mastery_level_index: 0,
            freshness_score: freshness,
            needs_refresh: 0,
            engagement_count: 1,
            last_engagement_type: None,
            last_engagement_at: None,
            level_achieved_at: None,
            content_version_at_mastery: None,
            assessment_evidence_json: None,
            privileges_json: None,
            created_at: "2026-03-15 00:00:00".to_string(),
            updated_at: "2026-03-15 00:00:00".to_string(),
        }
    }

    #[test]
    fn empty_records_returns_neutral() {
        assert_eq!(compute(&[]), 0.5);
    }

    #[test]
    fn single_create_level_full_freshness() {
        let records = vec![make_mastery("create", 1.0)];
        assert!((compute(&records) - 1.0).abs() < 0.01);
    }

    #[test]
    fn single_not_started_returns_zero() {
        let records = vec![make_mastery("not_started", 1.0)];
        assert!((compute(&records) - 0.0).abs() < 0.01);
    }

    #[test]
    fn freshness_decay_reduces_depth() {
        let fresh = vec![make_mastery("create", 1.0)];
        let stale = vec![make_mastery("create", 0.3)];
        assert!(compute(&fresh) > compute(&stale));
    }

    #[test]
    fn mixed_levels_averaged() {
        let records = vec![
            make_mastery("understand", 1.0), // 0.43
            make_mastery("evaluate", 1.0),   // 0.86
        ];
        let result = compute(&records);
        // Average of 0.43 and 0.86 ≈ 0.645
        assert!(result > 0.6 && result < 0.7);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test mastery_depth -- --nocapture`
Expected: FAIL with `not yet implemented`

**Step 3: Write minimal implementation**

Replace `todo!()` in `compute`:

```rust
pub fn compute(records: &[ContentMastery]) -> f64 {
    if records.is_empty() {
        return 0.5;
    }

    let total: f64 = records
        .iter()
        .map(|r| level_to_depth(&r.mastery_level) * (r.freshness_score as f64))
        .sum();
    let weight: f64 = records.iter().map(|r| r.freshness_score as f64).sum();

    if weight < f64::EPSILON {
        return 0.5;
    }

    (total / weight).clamp(0.0, 1.0)
}
```

**Step 4: Add module to services/mod.rs**

Add `pub mod mastery_depth;` to `elohim/elohim-storage/src/services/mod.rs`.

**Step 5: Run test to verify it passes**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test mastery_depth -- --nocapture`
Expected: 5 tests PASS

**Step 6: Commit**

```bash
git add elohim/elohim-storage/src/services/mastery_depth.rs elohim/elohim-storage/src/services/mod.rs
git commit -m "feat(gate): add mastery_depth trust signal computation"
```

---

## Task 2: Steward Standing Computation Module

**Files:**
- Create: `elohim/elohim-storage/src/services/steward_standing.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs` (add `pub mod steward_standing;`)

**Step 1: Write the failing test**

```rust
// elohim/elohim-storage/src/services/steward_standing.rs
//! Steward standing computation for trust signals.
//!
//! Converts a steward's allocation portfolio into a standing signal (0.0-1.0)
//! reflecting their stewardship health: active ratio, recognition, dispute-free status.

use crate::db::models::StewardshipAllocation;

/// Compute steward standing from allocation records.
///
/// Algorithm:
/// - Base: ratio of active allocations (governance_state == "active") to total
/// - Boost: log-scaled recognition_accumulated across active allocations
/// - Penalty: each disputed allocation reduces score by 0.1
/// - Empty allocations → 0.5 (neutral — unknown steward)
pub fn compute(allocations: &[StewardshipAllocation]) -> f64 {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_allocation(governance_state: &str, recognition: f32) -> StewardshipAllocation {
        StewardshipAllocation {
            id: "alloc-1".to_string(),
            app_id: "lamad".to_string(),
            content_id: "content-1".to_string(),
            steward_presence_id: "steward-1".to_string(),
            allocation_ratio: 1.0,
            allocation_method: "manual".to_string(),
            contribution_type: "inherited".to_string(),
            contribution_evidence_json: None,
            governance_state: governance_state.to_string(),
            dispute_id: None,
            dispute_reason: None,
            disputed_at: None,
            disputed_by: None,
            negotiation_session_id: None,
            elohim_ratified_at: None,
            elohim_ratifier_id: None,
            effective_from: "2026-03-15 00:00:00".to_string(),
            effective_until: None,
            superseded_by: None,
            recognition_accumulated: recognition,
            note: None,
            metadata_json: None,
            created_at: "2026-03-15 00:00:00".to_string(),
            updated_at: "2026-03-15 00:00:00".to_string(),
        }
    }

    #[test]
    fn empty_allocations_returns_neutral() {
        assert_eq!(compute(&[]), 0.5);
    }

    #[test]
    fn all_active_returns_high() {
        let allocs = vec![
            make_allocation("active", 10.0),
            make_allocation("active", 5.0),
        ];
        assert!(compute(&allocs) > 0.7);
    }

    #[test]
    fn disputed_allocations_reduce_standing() {
        let clean = vec![make_allocation("active", 10.0)];
        let disputed = vec![
            make_allocation("active", 10.0),
            make_allocation("disputed", 0.0),
        ];
        assert!(compute(&clean) > compute(&disputed));
    }

    #[test]
    fn recognition_boosts_standing() {
        let low = vec![make_allocation("active", 0.0)];
        let high = vec![make_allocation("active", 100.0)];
        assert!(compute(&high) > compute(&low));
    }

    #[test]
    fn result_clamped_to_unit_interval() {
        let allocs: Vec<_> = (0..50)
            .map(|_| make_allocation("active", 1000.0))
            .collect();
        assert!(compute(&allocs) <= 1.0);
        let bad: Vec<_> = (0..50)
            .map(|_| make_allocation("disputed", 0.0))
            .collect();
        assert!(compute(&bad) >= 0.0);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test steward_standing -- --nocapture`
Expected: FAIL with `not yet implemented`

**Step 3: Write minimal implementation**

```rust
pub fn compute(allocations: &[StewardshipAllocation]) -> f64 {
    if allocations.is_empty() {
        return 0.5;
    }

    let total = allocations.len() as f64;
    let active = allocations
        .iter()
        .filter(|a| a.governance_state == "active")
        .count() as f64;
    let disputed = allocations
        .iter()
        .filter(|a| a.governance_state == "disputed")
        .count() as f64;

    // Base: active ratio (0.0-1.0), weighted at 60%
    let active_ratio = active / total;

    // Recognition boost: log-scaled total recognition, weighted at 30%
    let total_recognition: f64 = allocations
        .iter()
        .filter(|a| a.governance_state == "active")
        .map(|a| a.recognition_accumulated as f64)
        .sum();
    let recognition_boost = (1.0 + total_recognition).ln() / (1.0 + 1000.0_f64).ln();

    // Dispute penalty: 10% per disputed allocation, weighted at 10%
    let dispute_penalty = (disputed * 0.1).min(1.0);

    let score = (active_ratio * 0.6) + (recognition_boost.min(1.0) * 0.3) - (dispute_penalty * 0.1);

    score.clamp(0.0, 1.0)
}
```

**Step 4: Add module to services/mod.rs**

Add `pub mod steward_standing;` to `elohim/elohim-storage/src/services/mod.rs`.

**Step 5: Run test to verify it passes**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test steward_standing -- --nocapture`
Expected: 5 tests PASS

**Step 6: Commit**

```bash
git add elohim/elohim-storage/src/services/steward_standing.rs elohim/elohim-storage/src/services/mod.rs
git commit -m "feat(gate): add steward_standing trust signal computation"
```

---

## Task 3: Relationship Density Computation Module

**Files:**
- Create: `elohim/elohim-storage/src/services/relationship_density.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs` (add `pub mod relationship_density;`)

**Step 1: Write the failing test**

```rust
// elohim/elohim-storage/src/services/relationship_density.rs
//! Relationship density computation for trust signals.
//!
//! Converts a human's relationship graph into a density signal (0.0-1.0)
//! reflecting how connected and verified they are in the network.

use crate::db::models::HumanRelationship;

/// Compute relationship density from a human's relationships.
///
/// Algorithm:
/// - Count total relationships
/// - Weight verified relationships higher (verified_at is Some)
/// - Weight bidirectional higher (is_bidirectional == 1)
/// - Weight by consent level (both parties consenting)
/// - Normalize to [0.0, 1.0] using log scaling (diminishing returns after ~20 relationships)
/// - Empty relationships → 0.5 (neutral)
pub fn compute(relationships: &[HumanRelationship]) -> f64 {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_relationship(
        verified: bool,
        bidirectional: bool,
        mutual_consent: bool,
    ) -> HumanRelationship {
        HumanRelationship {
            id: "rel-1".to_string(),
            app_id: "lamad".to_string(),
            party_a_id: "human-1".to_string(),
            party_b_id: "human-2".to_string(),
            relationship_type: "peer".to_string(),
            intimacy_level: "acquaintance".to_string(),
            is_bidirectional: if bidirectional { 1 } else { 0 },
            consent_given_by_a: if mutual_consent { 1 } else { 0 },
            consent_given_by_b: if mutual_consent { 1 } else { 0 },
            custody_enabled_by_a: 0,
            custody_enabled_by_b: 0,
            auto_custody_enabled: 0,
            emergency_access_enabled: 0,
            initiated_by: "human-1".to_string(),
            verified_at: if verified {
                Some("2026-03-15 00:00:00".to_string())
            } else {
                None
            },
            created_at: "2026-03-15 00:00:00".to_string(),
            updated_at: "2026-03-15 00:00:00".to_string(),
        }
    }

    #[test]
    fn empty_relationships_returns_neutral() {
        assert_eq!(compute(&[]), 0.5);
    }

    #[test]
    fn verified_bidirectional_scores_higher() {
        let weak = vec![make_relationship(false, false, false)];
        let strong = vec![make_relationship(true, true, true)];
        assert!(compute(&strong) > compute(&weak));
    }

    #[test]
    fn more_relationships_increases_density() {
        let few = vec![make_relationship(true, true, true)];
        let many: Vec<_> = (0..10)
            .map(|_| make_relationship(true, true, true))
            .collect();
        assert!(compute(&many) > compute(&few));
    }

    #[test]
    fn diminishing_returns_after_threshold() {
        let twenty: Vec<_> = (0..20)
            .map(|_| make_relationship(true, true, true))
            .collect();
        let hundred: Vec<_> = (0..100)
            .map(|_| make_relationship(true, true, true))
            .collect();
        let diff = compute(&hundred) - compute(&twenty);
        // Diminishing returns: 5x more relationships should yield much less than 5x more signal
        assert!(diff < 0.2);
    }

    #[test]
    fn result_clamped_to_unit_interval() {
        let many: Vec<_> = (0..1000)
            .map(|_| make_relationship(true, true, true))
            .collect();
        assert!(compute(&many) <= 1.0);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test relationship_density -- --nocapture`
Expected: FAIL with `not yet implemented`

**Step 3: Write minimal implementation**

```rust
pub fn compute(relationships: &[HumanRelationship]) -> f64 {
    if relationships.is_empty() {
        return 0.5;
    }

    // Score each relationship: 1.0 base + bonuses for quality
    let weighted_count: f64 = relationships
        .iter()
        .map(|r| {
            let mut weight = 1.0;
            if r.verified_at.is_some() {
                weight += 0.5; // Verified relationships count more
            }
            if r.is_bidirectional == 1 {
                weight += 0.3; // Mutual relationships count more
            }
            if r.consent_given_by_a == 1 && r.consent_given_by_b == 1 {
                weight += 0.2; // Full consent counts more
            }
            weight
        })
        .sum();

    // Log-scale normalization: ln(1 + weighted_count) / ln(1 + threshold)
    // Threshold of 40 weighted units ≈ 20 strong relationships
    let density = (1.0 + weighted_count).ln() / (1.0 + 40.0_f64).ln();

    density.clamp(0.0, 1.0)
}
```

**Step 4: Add module to services/mod.rs**

Add `pub mod relationship_density;` to `elohim/elohim-storage/src/services/mod.rs`.

**Step 5: Run test to verify it passes**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test relationship_density -- --nocapture`
Expected: 5 tests PASS

**Step 6: Commit**

```bash
git add elohim/elohim-storage/src/services/relationship_density.rs elohim/elohim-storage/src/services/mod.rs
git commit -m "feat(gate): add relationship_density trust signal computation"
```

---

## Task 4: Governance Health Computation Module

**Files:**
- Create: `elohim/elohim-storage/src/services/governance_health.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs` (add `pub mod governance_health;`)

**Context:** Governance health is computed from the steward's allocation portfolio — specifically the ratio of disputed/pending_review allocations. `query_challenges` and `query_proposals` are content-scoped (not human-scoped), so we reuse the allocations already queried for steward_standing rather than adding new queries.

**Step 1: Write the failing test**

```rust
// elohim/elohim-storage/src/services/governance_health.rs
//! Governance health computation for trust signals.
//!
//! Derives governance health from a steward's allocation governance states.
//! A healthy steward has few disputes, resolved cleanly. A troubled steward
//! has many active disputes or pending reviews.

use crate::db::models::StewardshipAllocation;

/// Compute governance health from allocation governance states.
///
/// Algorithm:
/// - Count allocations in each governance state
/// - Active is healthy, disputed/pending_review is unhealthy
/// - Ratio of healthy to total, with a small bonus for clean history
/// - Empty allocations → 0.5 (neutral)
pub fn compute(allocations: &[StewardshipAllocation]) -> f64 {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_allocation(governance_state: &str) -> StewardshipAllocation {
        StewardshipAllocation {
            id: "alloc-1".to_string(),
            app_id: "lamad".to_string(),
            content_id: "content-1".to_string(),
            steward_presence_id: "steward-1".to_string(),
            allocation_ratio: 1.0,
            allocation_method: "manual".to_string(),
            contribution_type: "inherited".to_string(),
            contribution_evidence_json: None,
            governance_state: governance_state.to_string(),
            dispute_id: None,
            dispute_reason: None,
            disputed_at: None,
            disputed_by: None,
            negotiation_session_id: None,
            elohim_ratified_at: None,
            elohim_ratifier_id: None,
            effective_from: "2026-03-15 00:00:00".to_string(),
            effective_until: None,
            superseded_by: None,
            recognition_accumulated: 0.0,
            note: None,
            metadata_json: None,
            created_at: "2026-03-15 00:00:00".to_string(),
            updated_at: "2026-03-15 00:00:00".to_string(),
        }
    }

    #[test]
    fn empty_allocations_returns_neutral() {
        assert_eq!(compute(&[]), 0.5);
    }

    #[test]
    fn all_active_returns_high() {
        let allocs = vec![
            make_allocation("active"),
            make_allocation("active"),
            make_allocation("active"),
        ];
        assert!(compute(&allocs) > 0.8);
    }

    #[test]
    fn disputed_reduces_health() {
        let clean = vec![make_allocation("active")];
        let messy = vec![make_allocation("active"), make_allocation("disputed")];
        assert!(compute(&clean) > compute(&messy));
    }

    #[test]
    fn all_disputed_returns_low() {
        let allocs = vec![
            make_allocation("disputed"),
            make_allocation("disputed"),
        ];
        assert!(compute(&allocs) < 0.2);
    }

    #[test]
    fn pending_review_is_mildly_unhealthy() {
        let active = vec![make_allocation("active")];
        let pending = vec![make_allocation("pending_review")];
        assert!(compute(&active) > compute(&pending));
        // But pending_review is better than disputed
        let disputed = vec![make_allocation("disputed")];
        assert!(compute(&pending) > compute(&disputed));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test governance_health -- --nocapture`
Expected: FAIL with `not yet implemented`

**Step 3: Write minimal implementation**

```rust
pub fn compute(allocations: &[StewardshipAllocation]) -> f64 {
    if allocations.is_empty() {
        return 0.5;
    }

    let total = allocations.len() as f64;
    let mut health_score: f64 = 0.0;

    for alloc in allocations {
        health_score += match alloc.governance_state.as_str() {
            "active" => 1.0,
            "superseded" => 0.8,     // Normal lifecycle, slightly less than active
            "pending_review" => 0.4, // Mildly concerning
            "disputed" => 0.0,      // Unhealthy
            _ => 0.5,               // Unknown state — neutral
        };
    }

    (health_score / total).clamp(0.0, 1.0)
}
```

**Step 4: Add module to services/mod.rs**

Add `pub mod governance_health;` to `elohim/elohim-storage/src/services/mod.rs`.

**Step 5: Run test to verify it passes**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test governance_health -- --nocapture`
Expected: 5 tests PASS

**Step 6: Commit**

```bash
git add elohim/elohim-storage/src/services/governance_health.rs elohim/elohim-storage/src/services/mod.rs
git commit -m "feat(gate): add governance_health trust signal computation"
```

---

## Task 5: Wire All Four Signals into evaluate_gate

**Files:**
- Modify: `elohim/elohim-storage/src/api/mod.rs` (lines ~173-199)

**Context:** Replace the four `0.5` placeholders with real DB queries + computation. The `evaluate_gate` function already has `human_id: Option<&str>` and a DB pool. We add three new queries alongside the existing observations query, then call each computation module. All queries fail gracefully to 0.5.

**Step 1: Add DB queries before the TrustContext computation**

After line 186 (`None => Vec::new()`) and before line 192 (`let trust_ctx = ...`), add:

```rust
    // Query mastery records for mastery_depth signal
    let mastery_depth = match human_id {
        Some(hid) => match get_conn(pool) {
            Ok(mut conn) => {
                let records = crate::db::content_mastery::get_mastery_for_human(&mut conn, ctx, hid)
                    .unwrap_or_default();
                crate::services::mastery_depth::compute(&records)
            }
            Err(_) => 0.5,
        },
        None => 0.5,
    };

    // Query stewardship allocations for steward_standing and governance_health signals
    // Note: we query once and compute both signals from the same data
    let (steward_standing, governance_health) = match human_id {
        Some(hid) => match get_conn(pool) {
            Ok(mut conn) => {
                let allocations =
                    crate::db::stewardship_allocations::get_allocations_for_steward(&mut conn, ctx, hid)
                        .unwrap_or_default();
                (
                    crate::services::steward_standing::compute(&allocations),
                    crate::services::governance_health::compute(&allocations),
                )
            }
            Err(_) => (0.5, 0.5),
        },
        None => (0.5, 0.5),
    };

    // Query relationships for relationship_density signal
    let relationship_density = match human_id {
        Some(hid) => match get_conn(pool) {
            Ok(mut conn) => {
                let relationships =
                    crate::db::human_relationships::get_relationships_for_human(&mut conn, ctx, hid)
                        .unwrap_or_default();
                crate::services::relationship_density::compute(&relationships)
            }
            Err(_) => 0.5,
        },
        None => 0.5,
    };
```

**Step 2: Replace placeholder signals in TrustContext::compute**

Replace lines 192-199:

```rust
    let trust_ctx = TrustContext::compute(TrustSignals {
        mastery_depth,          // from mastery records
        steward_standing,       // from stewardship allocations
        relationship_density,   // from human relationships
        governance_health,      // from allocation governance states
        behavioral_trust,       // from observation history
        intent_divergence,      // from anomaly detection
    });
```

**Step 3: Run full test suite**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --bins`
Expected: All existing tests PASS (gate tests use mock services, not evaluate_gate)

**Step 4: Run clippy**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings`
Expected: No warnings

**Step 5: Run rustfmt**

Run: `cd elohim/elohim-storage && cargo fmt`

**Step 6: Commit**

```bash
git add elohim/elohim-storage/src/api/mod.rs
git commit -m "feat(gate): wire real trust signals into evaluate_gate — all four senses live"
```

---

## Task 6: Integration Verification

**Files:**
- No file changes — verification only

**Step 1: Run full test suite**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --bins`
Expected: All tests PASS (existing 329+ tests + 20 new signal tests)

**Step 2: Verify clippy clean**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings`
Expected: No warnings

**Step 3: Verify formatting**

Run: `cd elohim/elohim-storage && cargo fmt --check`
Expected: No diffs

**Step 4: Verify no placeholder signals remain**

Run: `grep -n "placeholder" elohim/elohim-storage/src/api/mod.rs`
Expected: No matches

**Step 5: Count test coverage**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --bins 2>&1 | tail -5`
Expected: ~349 tests (329 existing + 20 new)

---

## Verification

### Full Build + Test
```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --bins
```

### Clippy
```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings
```

### Format
```bash
cd elohim/elohim-storage && cargo fmt --check
```

### Placeholder Check
```bash
grep -rn "0\.5.*placeholder\|placeholder.*0\.5" elohim/elohim-storage/src/
```
Target: zero matches after Sprint 4.

---

## What This Sprint Completes

After Sprint 4, the ElohimGate has all six trust signals wired to real data:

| Signal | Source | Sprint |
|--------|--------|--------|
| behavioral_trust | Observation history | Sprint 3 |
| intent_divergence | Anomaly detection | Sprint 3 |
| mastery_depth | Content mastery records | **Sprint 4** |
| steward_standing | Stewardship allocations | **Sprint 4** |
| relationship_density | Human relationships | **Sprint 4** |
| governance_health | Allocation governance states | **Sprint 4** |

## What Remains (Future Sprints)

- **Sprint 5: Angular Gate Client** — Service to handle GateEvaluationView, pause confirm UX, trust context display
- **Sprint 6: SSE Streaming** — Real-time gate evaluation streaming to Angular (no existing SSE patterns in codebase, needs full wire-up)
- **Sprint 7: Inference Sidecar Integration** — Connect elohim-agent-sdk at :8095 for Deep/Constitutional tier inference
