# Recognition Pipeline Service — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a Rust-side `RecognitionPipelineService` that closes the recognition distribution loop end-to-end: trigger → weighted distribution → economic events.

**Architecture:** Five composable pipeline stages (normalize, resolve, weight, limit, settle) as pure functions in a new service module, exposed via `POST /api/v1/recognition/distribute`. Each stage is independently testable. Angular is thin API client only.

**Tech Stack:** Rust (Diesel, serde, hyper), SQLite, ts-rs for TypeScript type generation.

**Design doc:** `genesis/plans/2026-03-13-recognition-pipeline-design.md`

---

### Task 1: Pipeline Domain Types

**Files:**
- Create: `elohim/elohim-storage/src/services/recognition_pipeline_service.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

**Step 1: Write the failing test**

Add to the bottom of `recognition_pipeline_service.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_known_event_type() {
        let result = normalize_trigger("content-1", "mastery_completion", 10.0);
        assert_eq!(result.weighted_amount, 10.0); // weight 1.0
        assert_eq!(result.weight, 1.0);
    }

    #[test]
    fn normalize_micro_recognition() {
        let result = normalize_trigger("content-1", "content_access", 1.0);
        assert!((result.weighted_amount - 0.01).abs() < 1e-6);
    }

    #[test]
    fn normalize_unknown_event_type_uses_default() {
        let result = normalize_trigger("content-1", "unknown_type", 5.0);
        assert_eq!(result.weight, 0.01); // default fallback
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS="" cargo test recognition_pipeline --lib -- --nocapture 2>&1 | tail -20`

Expected: FAIL — module doesn't exist yet.

**Step 3: Write minimal implementation**

Create `elohim/elohim-storage/src/services/recognition_pipeline_service.rs`:

```rust
//! Recognition pipeline service — composable stages for recognition distribution
//!
//! Orchestrates: trigger normalization → steward resolution → affinity weighting
//! → constitutional limits → economic event settlement.
//!
//! ## Architecture
//!
//! Controller (api/recognition.rs) → **Service (this file)** → Models (db/stewardship_allocations.rs, db/economic_events.rs)

use std::collections::HashMap;

use chrono::Utc;
use serde::Serialize;

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

/// Raw trigger from the caller
#[derive(Debug, Clone)]
pub struct RecognitionTrigger {
    pub content_id: String,
    pub event_type: String,
    pub raw_amount: f64,
    pub triggered_by: Option<String>,
}

/// Stage 1 output: normalized trigger with weight applied
#[derive(Debug, Clone)]
pub struct NormalizedTrigger {
    pub content_id: String,
    pub event_type: String,
    pub raw_amount: f64,
    pub weight: f64,
    pub weighted_amount: f64,
}

/// Stage 2 output: resolved steward with affinity signals
#[derive(Debug, Clone)]
pub struct ResolvedSteward {
    pub allocation_id: String,
    pub steward_presence_id: String,
    pub allocation_ratio: f32,
    pub stored_affinity: f64,
    pub derived_affinity: f64,
    pub contribution_type: String,
}

/// Stage 3 output: weighted share per steward
#[derive(Debug, Clone)]
pub struct WeightedShare {
    pub allocation_id: String,
    pub steward_presence_id: String,
    pub effective_ratio: f64,
    pub share_amount: f64,
}

/// Stage 4 output: share after constitutional limit checks
#[derive(Debug, Clone)]
pub struct LimitedShare {
    pub allocation_id: String,
    pub steward_presence_id: String,
    pub pre_limit_amount: f64,
    pub final_amount: f64,
    pub limit_reasons: Vec<LimitReason>,
}

/// Why a limit was applied (for explainability)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum LimitReason {
    None,
    FloorApplied { floor: f64, original: f64 },
    CeilingApplied { ceiling: f64, excess: f64 },
    ExcessRedistributed { from_steward: String, amount: f64 },
}

/// Full trace per steward for explainability
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StageTrace {
    pub steward_presence_id: String,
    pub allocation_ratio: f32,
    pub stored_affinity: f64,
    pub derived_affinity: f64,
    pub effective_ratio: f64,
    pub pre_limit_share: f64,
    pub final_share: f64,
    pub limit_reasons: Vec<LimitReason>,
    pub economic_event_id: String,
}

/// Final pipeline result
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecognitionDistributionResult {
    pub content_id: String,
    pub trigger_event_type: String,
    pub raw_amount: f64,
    pub weighted_amount: f64,
    pub distributions: Vec<StageTrace>,
    pub economic_event_ids: Vec<String>,
    pub limits_applied: Vec<LimitReason>,
}

// ---------------------------------------------------------------------------
// Stage 1: Normalize
// ---------------------------------------------------------------------------

/// Event type weight table — maps trigger types to recognition multipliers
fn event_type_weights() -> HashMap<&'static str, f64> {
    let mut m = HashMap::new();
    m.insert("mastery_completion", 1.0);
    m.insert("content_access", 0.01);
    m.insert("content_citation", 0.5);
    m.insert("assessment_attempt", 0.1);
    m
}

const DEFAULT_EVENT_WEIGHT: f64 = 0.01;

/// Stage 1: Normalize a raw trigger into a weighted amount
pub fn normalize_trigger(content_id: &str, event_type: &str, raw_amount: f64) -> NormalizedTrigger {
    let weights = event_type_weights();
    let weight = weights.get(event_type).copied().unwrap_or(DEFAULT_EVENT_WEIGHT);

    NormalizedTrigger {
        content_id: content_id.to_string(),
        event_type: event_type.to_string(),
        raw_amount,
        weight,
        weighted_amount: raw_amount * weight,
    }
}

// ---------------------------------------------------------------------------
// Stage 3: Weight
// ---------------------------------------------------------------------------

/// Stage 3: Apply affinity coefficients and re-normalize
pub fn apply_weights(stewards: &[ResolvedSteward], weighted_amount: f64) -> Vec<WeightedShare> {
    if stewards.is_empty() {
        return vec![];
    }

    // Calculate effective ratios
    let effective: Vec<(usize, f64)> = stewards
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let eff = (s.allocation_ratio as f64) * s.stored_affinity * s.derived_affinity;
            (i, eff)
        })
        .collect();

    let total_effective: f64 = effective.iter().map(|(_, e)| e).sum();

    if total_effective <= 0.0 {
        return stewards
            .iter()
            .map(|s| WeightedShare {
                allocation_id: s.allocation_id.clone(),
                steward_presence_id: s.steward_presence_id.clone(),
                effective_ratio: 0.0,
                share_amount: 0.0,
            })
            .collect();
    }

    effective
        .iter()
        .map(|(i, eff)| {
            let normalized_ratio = eff / total_effective;
            WeightedShare {
                allocation_id: stewards[*i].allocation_id.clone(),
                steward_presence_id: stewards[*i].steward_presence_id.clone(),
                effective_ratio: normalized_ratio,
                share_amount: weighted_amount * normalized_ratio,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Stage 4: Limit (v0 passthrough)
// ---------------------------------------------------------------------------

/// Stage 4: Apply constitutional limits (v0: passthrough with structure)
pub fn apply_limits(shares: &[WeightedShare]) -> Vec<LimitedShare> {
    shares
        .iter()
        .map(|s| LimitedShare {
            allocation_id: s.allocation_id.clone(),
            steward_presence_id: s.steward_presence_id.clone(),
            pre_limit_amount: s.share_amount,
            final_amount: s.share_amount,
            limit_reasons: vec![],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Stage 1 tests
    #[test]
    fn normalize_known_event_type() {
        let result = normalize_trigger("content-1", "mastery_completion", 10.0);
        assert_eq!(result.weighted_amount, 10.0);
        assert_eq!(result.weight, 1.0);
    }

    #[test]
    fn normalize_micro_recognition() {
        let result = normalize_trigger("content-1", "content_access", 1.0);
        assert!((result.weighted_amount - 0.01).abs() < 1e-6);
    }

    #[test]
    fn normalize_unknown_event_type_uses_default() {
        let result = normalize_trigger("content-1", "unknown_type", 5.0);
        assert_eq!(result.weight, DEFAULT_EVENT_WEIGHT);
    }

    // Stage 3 tests
    #[test]
    fn weights_single_steward() {
        let stewards = vec![ResolvedSteward {
            allocation_id: "alloc-1".into(),
            steward_presence_id: "steward-1".into(),
            allocation_ratio: 1.0,
            stored_affinity: 1.0,
            derived_affinity: 1.0,
            contribution_type: "author".into(),
        }];
        let shares = apply_weights(&stewards, 10.0);
        assert_eq!(shares.len(), 1);
        assert!((shares[0].share_amount - 10.0).abs() < 1e-6);
    }

    #[test]
    fn weights_two_stewards_equal_ratio_different_affinity() {
        let stewards = vec![
            ResolvedSteward {
                allocation_id: "alloc-1".into(),
                steward_presence_id: "steward-1".into(),
                allocation_ratio: 0.5,
                stored_affinity: 1.0,
                derived_affinity: 0.9,
                contribution_type: "author".into(),
            },
            ResolvedSteward {
                allocation_id: "alloc-2".into(),
                steward_presence_id: "steward-2".into(),
                allocation_ratio: 0.5,
                stored_affinity: 1.0,
                derived_affinity: 0.3,
                contribution_type: "curator".into(),
            },
        ];
        let shares = apply_weights(&stewards, 12.0);
        assert_eq!(shares.len(), 2);
        // steward-1 has 3x the affinity of steward-2, both have 0.5 ratio
        // effective: 0.45 vs 0.15, normalized: 0.75 vs 0.25
        assert!((shares[0].share_amount - 9.0).abs() < 1e-6);
        assert!((shares[1].share_amount - 3.0).abs() < 1e-6);
        // Total distributed equals total available
        let total: f64 = shares.iter().map(|s| s.share_amount).sum();
        assert!((total - 12.0).abs() < 1e-6);
    }

    #[test]
    fn weights_empty_stewards() {
        let shares = apply_weights(&[], 10.0);
        assert!(shares.is_empty());
    }

    // Stage 4 tests (v0 passthrough)
    #[test]
    fn limits_passthrough_preserves_amounts() {
        let shares = vec![WeightedShare {
            allocation_id: "alloc-1".into(),
            steward_presence_id: "steward-1".into(),
            effective_ratio: 1.0,
            share_amount: 10.0,
        }];
        let limited = apply_limits(&shares);
        assert_eq!(limited.len(), 1);
        assert_eq!(limited[0].final_amount, 10.0);
        assert!(limited[0].limit_reasons.is_empty());
    }
}
```

Add to `elohim/elohim-storage/src/services/mod.rs` after line 34 (`pub mod stewardship_service;`):

```rust
pub mod recognition_pipeline_service;
```

**Step 4: Run test to verify it passes**

Run: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS="" cargo test recognition_pipeline --lib -- --nocapture 2>&1 | tail -20`

Expected: 6 tests PASS.

**Step 5: Commit**

```bash
git add elohim/elohim-storage/src/services/recognition_pipeline_service.rs elohim/elohim-storage/src/services/mod.rs
git commit -m "feat(storage): add recognition pipeline domain types and pure stage functions

Stages 1 (normalize), 3 (weight), and 4 (limit) as tested pure functions.
Stage 2 (resolve) and 5 (settle) require DB access — added in next task."
```

---

### Task 2: View Types and ts-rs Exports

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs` (append before schema_version_tests module)

**Step 1: Write the failing test**

Add to `views.rs` test module:

```rust
#[test]
fn recognition_trigger_input_deserializes_camel_case() {
    let json = r#"{"contentId":"c-1","eventType":"mastery_completion","rawAmount":10.0}"#;
    let view: RecognitionTriggerInputView = serde_json::from_str(json).unwrap();
    assert_eq!(view.content_id, "c-1");
    assert_eq!(view.event_type, "mastery_completion");
    assert!((view.raw_amount - 10.0).abs() < 1e-6);
}
```

**Step 2: Run test to verify it fails**

Run: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS="" cargo test recognition_trigger_input --lib 2>&1 | tail -10`

Expected: FAIL — type doesn't exist.

**Step 3: Write minimal implementation**

Add to `views.rs` before the `#[cfg(test)]` schema_version_tests block (~line 4028):

```rust
// ============================================================================
// Recognition Pipeline Views
// ============================================================================

/// API input for triggering recognition distribution
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RecognitionTriggerInputView {
    pub content_id: String,
    pub event_type: String,
    pub raw_amount: f64,
    #[serde(default)]
    pub triggered_by: Option<String>,
}

impl From<RecognitionTriggerInputView> for crate::services::recognition_pipeline_service::RecognitionTrigger {
    fn from(v: RecognitionTriggerInputView) -> Self {
        Self {
            content_id: v.content_id,
            event_type: v.event_type,
            raw_amount: v.raw_amount,
            triggered_by: v.triggered_by,
        }
    }
}

/// Per-steward trace in the distribution result
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct StageTraceView {
    pub steward_presence_id: String,
    pub allocation_ratio: f32,
    pub stored_affinity: f64,
    pub derived_affinity: f64,
    pub effective_ratio: f64,
    pub pre_limit_share: f64,
    pub final_share: f64,
    pub limit_reasons: Vec<serde_json::Value>,
    pub economic_event_id: String,
}

impl From<crate::services::recognition_pipeline_service::StageTrace> for StageTraceView {
    fn from(t: crate::services::recognition_pipeline_service::StageTrace) -> Self {
        Self {
            steward_presence_id: t.steward_presence_id,
            allocation_ratio: t.allocation_ratio,
            stored_affinity: t.stored_affinity,
            derived_affinity: t.derived_affinity,
            effective_ratio: t.effective_ratio,
            pre_limit_share: t.pre_limit_share,
            final_share: t.final_share,
            limit_reasons: t.limit_reasons.iter().map(|r| serde_json::to_value(r).unwrap_or_default()).collect(),
            economic_event_id: t.economic_event_id,
        }
    }
}

/// Full pipeline result
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RecognitionDistributionResultView {
    pub content_id: String,
    pub trigger_event_type: String,
    pub raw_amount: f64,
    pub weighted_amount: f64,
    pub distributions: Vec<StageTraceView>,
    pub economic_event_ids: Vec<String>,
    pub limits_applied: Vec<serde_json::Value>,
}

impl From<crate::services::recognition_pipeline_service::RecognitionDistributionResult> for RecognitionDistributionResultView {
    fn from(r: crate::services::recognition_pipeline_service::RecognitionDistributionResult) -> Self {
        Self {
            content_id: r.content_id,
            trigger_event_type: r.trigger_event_type,
            raw_amount: r.raw_amount,
            weighted_amount: r.weighted_amount,
            distributions: r.distributions.into_iter().map(StageTraceView::from).collect(),
            economic_event_ids: r.economic_event_ids,
            limits_applied: r.limits_applied.iter().map(|l| serde_json::to_value(l).unwrap_or_default()).collect(),
        }
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS="" cargo test recognition_trigger_input --lib 2>&1 | tail -10`

Expected: PASS.

**Step 5: Generate TypeScript types**

Run: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS="" cargo test export_bindings --lib 2>&1 | tail -10`

Verify files exist:
- `elohim/sdk/storage-client-ts/src/generated/RecognitionTriggerInputView.ts`
- `elohim/sdk/storage-client-ts/src/generated/StageTraceView.ts`
- `elohim/sdk/storage-client-ts/src/generated/RecognitionDistributionResultView.ts`

**Step 6: Commit**

```bash
git add elohim/elohim-storage/src/views.rs elohim/sdk/storage-client-ts/src/generated/Recognition*.ts elohim/sdk/storage-client-ts/src/generated/StageTraceView.ts
git commit -m "feat(storage): add recognition pipeline view types with ts-rs exports

Input (RecognitionTriggerInputView) and output (RecognitionDistributionResultView,
StageTraceView) types following the camelCase API boundary convention."
```

---

### Task 3: Stage 2 (Resolve) and Stage 5 (Settle) — DB-Dependent Stages

**Files:**
- Modify: `elohim/elohim-storage/src/services/recognition_pipeline_service.rs`

These stages need DB access. They use existing `stewardship_allocations` and `economic_events` DB functions.

**Step 1: Write the test for resolve_stewards**

```rust
// In the #[cfg(test)] module, add:
// (These are unit tests using constructed DB models, not integration tests)
#[test]
fn resolve_builds_stewards_from_allocations() {
    let allocations = vec![
        StewardshipAllocation {
            id: "alloc-1".into(),
            app_id: "lamad".into(),
            content_id: "content-1".into(),
            steward_presence_id: "steward-1".into(),
            allocation_ratio: 0.6,
            allocation_method: "manual".into(),
            contribution_type: "author".into(),
            contribution_evidence_json: None,
            governance_state: "active".into(),
            dispute_id: None,
            dispute_reason: None,
            disputed_at: None,
            disputed_by: None,
            negotiation_session_id: None,
            elohim_ratified_at: None,
            elohim_ratifier_id: None,
            effective_from: "2026-01-01T00:00:00Z".into(),
            effective_until: None,
            superseded_by: None,
            recognition_accumulated: 0.0,
            last_recognition_at: None,
            note: None,
            metadata_json: None,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        },
    ];
    let resolved = resolve_from_allocations(&allocations);
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].steward_presence_id, "steward-1");
    assert_eq!(resolved[0].allocation_ratio, 0.6);
    // Default affinities when no node stewardship data
    assert_eq!(resolved[0].stored_affinity, 1.0);
    assert_eq!(resolved[0].derived_affinity, 1.0);
}
```

**Step 2: Run test to verify it fails**

Run: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS="" cargo test resolve_builds --lib 2>&1 | tail -10`

Expected: FAIL — function doesn't exist.

**Step 3: Write resolve_from_allocations and settle functions**

Add to `recognition_pipeline_service.rs`:

```rust
use diesel::SqliteConnection;
use sha2::{Digest, Sha256};

use crate::db::economic_events::{record_event, CreateEconomicEventInput};
use crate::db::models::StewardshipAllocation;
use crate::db::stewardship_allocations::{self, accumulate_recognition};
use crate::db::AppContext;
use crate::error::StorageError;

// ---------------------------------------------------------------------------
// Stage 2: Resolve
// ---------------------------------------------------------------------------

/// Stage 2: Build resolved stewards from allocation records.
///
/// v0: stored_affinity and derived_affinity default to 1.0.
/// Future: query node_stewardship for stored_affinity, human profiles for derived.
pub fn resolve_from_allocations(allocations: &[StewardshipAllocation]) -> Vec<ResolvedSteward> {
    allocations
        .iter()
        .filter(|a| a.governance_state == "active")
        .map(|a| ResolvedSteward {
            allocation_id: a.id.clone(),
            steward_presence_id: a.steward_presence_id.clone(),
            allocation_ratio: a.allocation_ratio,
            stored_affinity: 1.0, // v0: default, future: node_stewardship lookup
            derived_affinity: 1.0, // v0: default, future: human profile lookup
            contribution_type: a.contribution_type.clone(),
        })
        .collect()
}

/// Stage 2 with DB: query allocations for content, then resolve
pub fn resolve_stewards(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    content_id: &str,
) -> Result<Vec<ResolvedSteward>, StorageError> {
    let allocations = stewardship_allocations::get_allocations_for_content(
        conn, ctx, content_id,
    )?;
    Ok(resolve_from_allocations(&allocations))
}

// ---------------------------------------------------------------------------
// Stage 5: Settle
// ---------------------------------------------------------------------------

/// Stage 5: Create economic events and accumulate recognition for each share
pub fn settle(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    shares: &[LimitedShare],
    trigger: &RecognitionTrigger,
    weighted_amount: f64,
) -> Result<Vec<StageTrace>, StorageError> {
    let mut traces = Vec::with_capacity(shares.len());

    for share in shares {
        let now = Utc::now();
        let event_id = generate_recognition_event_id(
            &trigger.content_id,
            &share.steward_presence_id,
            now.timestamp_millis(),
        );

        // Create the economic event
        let input = CreateEconomicEventInput {
            id: Some(event_id.clone()),
            action: "produce".to_string(),
            provider: trigger.content_id.clone(),
            receiver: share.steward_presence_id.clone(),
            resource_conforms_to: Some("recognition".to_string()),
            resource_inventoried_as: None,
            resource_classified_as: vec!["recognition-distribution".to_string()],
            resource_quantity_value: Some(share.final_amount as f32),
            resource_quantity_unit: Some("recognition".to_string()),
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_point_in_time: Some(now.to_rfc3339()),
            has_duration: None,
            input_of: None,
            output_of: None,
            lamad_event_type: Some(trigger.event_type.clone()),
            content_id: Some(trigger.content_id.clone()),
            contributor_presence_id: Some(share.steward_presence_id.clone()),
            path_id: None,
            triggered_by: trigger.triggered_by.clone(),
            note: Some(format!(
                "Recognition distribution: {} of {} for {}",
                share.final_amount, weighted_amount, trigger.event_type
            )),
            metadata_json: None,
        };

        record_event(conn, ctx, input)?;

        // Accumulate recognition on the allocation
        accumulate_recognition(
            conn,
            ctx,
            &share.allocation_id,
            share.final_amount as f32,
        )?;

        traces.push(StageTrace {
            steward_presence_id: share.steward_presence_id.clone(),
            allocation_ratio: 0.0,  // filled by pipeline orchestrator
            stored_affinity: 0.0,   // filled by pipeline orchestrator
            derived_affinity: 0.0,  // filled by pipeline orchestrator
            effective_ratio: 0.0,   // filled by pipeline orchestrator
            pre_limit_share: share.pre_limit_amount,
            final_share: share.final_amount,
            limit_reasons: share.limit_reasons.clone(),
            economic_event_id: event_id,
        });
    }

    Ok(traces)
}

fn generate_recognition_event_id(content_id: &str, steward_id: &str, timestamp_ms: i64) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content_id.as_bytes());
    hasher.update(steward_id.as_bytes());
    hasher.update(timestamp_ms.to_le_bytes());
    let hash = hasher.finalize();
    let hex_suffix = hex::encode(&hash[..4]);
    format!("recog-{}-{}", timestamp_ms, hex_suffix)
}
```

**Step 4: Run test to verify it passes**

Run: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS="" cargo test recognition_pipeline --lib 2>&1 | tail -20`

Expected: All tests PASS (including the new resolve test).

**Step 5: Commit**

```bash
git add elohim/elohim-storage/src/services/recognition_pipeline_service.rs
git commit -m "feat(storage): add resolve and settle stages to recognition pipeline

Stage 2 resolves stewards from allocations with affinity defaults (v0).
Stage 5 creates economic events per steward and accumulates recognition."
```

---

### Task 4: Pipeline Orchestrator Function

**Files:**
- Modify: `elohim/elohim-storage/src/services/recognition_pipeline_service.rs`

**Step 1: Write the orchestrator function**

This is the function that chains all 5 stages. Add to the service file:

```rust
// ---------------------------------------------------------------------------
// Pipeline orchestrator
// ---------------------------------------------------------------------------

/// Run the full recognition distribution pipeline.
///
/// Chains: normalize → resolve → weight → limit → settle
/// Returns a complete result with per-steward traces and created event IDs.
pub fn distribute(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    trigger: RecognitionTrigger,
) -> Result<RecognitionDistributionResult, StorageError> {
    // Stage 1: Normalize
    let normalized = normalize_trigger(&trigger.content_id, &trigger.event_type, trigger.raw_amount);

    if normalized.weighted_amount <= 0.0 {
        return Ok(RecognitionDistributionResult {
            content_id: trigger.content_id.clone(),
            trigger_event_type: trigger.event_type.clone(),
            raw_amount: trigger.raw_amount,
            weighted_amount: 0.0,
            distributions: vec![],
            economic_event_ids: vec![],
            limits_applied: vec![],
        });
    }

    // Stage 2: Resolve stewards
    let stewards = resolve_stewards(conn, ctx, &trigger.content_id)?;

    if stewards.is_empty() {
        return Ok(RecognitionDistributionResult {
            content_id: trigger.content_id.clone(),
            trigger_event_type: trigger.event_type.clone(),
            raw_amount: trigger.raw_amount,
            weighted_amount: normalized.weighted_amount,
            distributions: vec![],
            economic_event_ids: vec![],
            limits_applied: vec![],
        });
    }

    // Stage 3: Weight
    let shares = apply_weights(&stewards, normalized.weighted_amount);

    // Stage 4: Limit
    let limited = apply_limits(&shares);

    // Stage 5: Settle
    let mut traces = settle(conn, ctx, &limited, &trigger, normalized.weighted_amount)?;

    // Enrich traces with stage 2/3 data
    for (trace, steward) in traces.iter_mut().zip(stewards.iter()) {
        trace.allocation_ratio = steward.allocation_ratio;
        trace.stored_affinity = steward.stored_affinity;
        trace.derived_affinity = steward.derived_affinity;
    }
    for (trace, share) in traces.iter_mut().zip(shares.iter()) {
        trace.effective_ratio = share.effective_ratio;
    }

    let event_ids: Vec<String> = traces.iter().map(|t| t.economic_event_id.clone()).collect();
    let all_limits: Vec<LimitReason> = limited
        .iter()
        .flat_map(|l| l.limit_reasons.clone())
        .collect();

    Ok(RecognitionDistributionResult {
        content_id: trigger.content_id,
        trigger_event_type: trigger.event_type,
        raw_amount: trigger.raw_amount,
        weighted_amount: normalized.weighted_amount,
        distributions: traces,
        economic_event_ids: event_ids,
        limits_applied: all_limits,
    })
}
```

**Step 2: Verify compilation**

Run: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS="" cargo check 2>&1 | tail -10`

Expected: Compiles with no errors.

**Step 3: Run all pipeline tests**

Run: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS="" cargo test recognition_pipeline --lib 2>&1 | tail -20`

Expected: All tests PASS.

**Step 4: Commit**

```bash
git add elohim/elohim-storage/src/services/recognition_pipeline_service.rs
git commit -m "feat(storage): add pipeline orchestrator chaining all 5 recognition stages

distribute() chains normalize → resolve → weight → limit → settle,
returning a full RecognitionDistributionResult with per-steward traces."
```

---

### Task 5: API Controller and Route Registration

**Files:**
- Create: `elohim/elohim-storage/src/api/recognition.rs`
- Modify: `elohim/elohim-storage/src/api/mod.rs`

**Step 1: Create the controller**

Create `elohim/elohim-storage/src/api/recognition.rs`:

```rust
//! Recognition Pipeline API controller
//!
//! Route: `POST /api/v1/recognition/distribute`
//!
//! Delegates to `RecognitionPipelineService::distribute()` for the full
//! 5-stage pipeline.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::recognition_pipeline_service;
use crate::services::response;
use crate::views::{RecognitionDistributionResultView, RecognitionTriggerInputView};

use super::{get_conn, parse_body};

/// Handle `/api/v1/recognition*` requests
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    match (&method, path) {
        // POST /api/v1/recognition/distribute
        (&Method::POST, "distribute") => handle_distribute(req, pool, ctx).await,

        _ => Ok(response::not_found(&format!(
            "Unknown recognition route: /api/v1/recognition/{}",
            path
        ))),
    }
}

async fn handle_distribute(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let input: RecognitionTriggerInputView = parse_body(req).await?;
    let trigger = input.into();

    let mut conn = get_conn(pool)?;
    let result = recognition_pipeline_service::distribute(&mut conn, ctx, trigger)?;
    let view = RecognitionDistributionResultView::from(result);

    Ok(response::json_ok(&view))
}
```

**Step 2: Register in api/mod.rs**

Add to `elohim/elohim-storage/src/api/mod.rs`:

After line 30 (`pub mod rea_commitments;`):

```rust
pub mod recognition;
```

In the `handle_api_request` function, add a new dispatch branch before the final `else` block (~line 107):

```rust
    } else if sub_path.starts_with("recognition") {
        let resource_path = sub_path.strip_prefix("recognition").unwrap_or("");
        recognition::handle(req, method, resource_path, &pool, &app_ctx).await
    } else {
```

**Step 3: Verify compilation**

Run: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS="" cargo check 2>&1 | tail -10`

Expected: Compiles. May need to add `json_ok` or adjust response helper — check the existing `response` module for the right helper name.

Reference: `crate::services::response` — look for `from_result`, `from_create_result`, `json_ok`, etc. Use whichever pattern the `economic_events.rs` controller uses for POST responses.

**Step 4: Run full test suite**

Run: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS="" cargo test --lib 2>&1 | tail -20`

Expected: All tests PASS.

**Step 5: Commit**

```bash
git add elohim/elohim-storage/src/api/recognition.rs elohim/elohim-storage/src/api/mod.rs
git commit -m "feat(storage): add POST /api/v1/recognition/distribute endpoint

New recognition controller dispatches to the pipeline service.
Accepts RecognitionTriggerInputView, returns RecognitionDistributionResultView."
```

---

### Task 6: Route Documentation

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs` (route declaration section ~line 6247)

**Step 1: Add route declaration**

Find the stewardship routes section (~line 6247) and add after it:

```rust
        // =====================================================================
        // /api/v1/recognition — Recognition distribution pipeline
        // =====================================================================
        .route(
            Route::post("/api/v1/recognition/distribute")
                .handler("distribute_recognition")
                .auth_required()
                .build(),
        )
```

**Step 2: Verify compilation**

Run: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS="" cargo check 2>&1 | tail -10`

**Step 3: Commit**

```bash
git add elohim/elohim-storage/src/http.rs
git commit -m "docs(storage): declare recognition/distribute route in route table"
```

---

### Task 7: Angular Thin Client

**Files:**
- Create: `app/elohim-app/src/app/elohim/services/recognition-api.service.ts`
- Create: `app/elohim-app/src/app/elohim/services/recognition-api.service.spec.ts`

**Step 1: Write the failing test**

Create `recognition-api.service.spec.ts`:

```typescript
import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { RecognitionApiService } from './recognition-api.service';

describe('RecognitionApiService', () => {
  let service: RecognitionApiService;
  let httpMock: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [provideHttpClient(), provideHttpClientTesting(), RecognitionApiService],
    });
    service = TestBed.inject(RecognitionApiService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => httpMock.verify());

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should POST to /api/v1/recognition/distribute', () => {
    const trigger = { contentId: 'c-1', eventType: 'mastery_completion', rawAmount: 10 };
    const mockResult = {
      contentId: 'c-1',
      triggerEventType: 'mastery_completion',
      rawAmount: 10,
      weightedAmount: 10,
      distributions: [],
      economicEventIds: [],
      limitsApplied: [],
    };

    service.distribute(trigger).subscribe(result => {
      expect(result.contentId).toBe('c-1');
      expect(result.weightedAmount).toBe(10);
    });

    const req = httpMock.expectOne('/api/v1/recognition/distribute');
    expect(req.request.method).toBe('POST');
    expect(req.request.body).toEqual(trigger);
    req.flush(mockResult);
  });
});
```

**Step 2: Run test to verify it fails**

Run: `cd /projects/elohim/app/elohim-app && pnpm exec vitest run --config vite.config.ts "recognition-api" 2>&1 | tail -15`

Expected: FAIL — service doesn't exist.

**Step 3: Write the service**

Create `recognition-api.service.ts`:

```typescript
import { inject, Injectable } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { Observable } from 'rxjs';
import type { RecognitionTriggerInputView } from '@elohim/storage-client';
import type { RecognitionDistributionResultView } from '@elohim/storage-client';

@Injectable({ providedIn: 'root' })
export class RecognitionApiService {
  private readonly http = inject(HttpClient);

  distribute(trigger: RecognitionTriggerInputView): Observable<RecognitionDistributionResultView> {
    return this.http.post<RecognitionDistributionResultView>(
      '/api/v1/recognition/distribute',
      trigger,
    );
  }
}
```

Note: If the generated types aren't yet re-exported from `@elohim/storage-client`, use inline type definitions temporarily:

```typescript
export interface RecognitionTrigger {
  contentId: string;
  eventType: string;
  rawAmount: number;
  triggeredBy?: string;
}

export interface RecognitionDistributionResult {
  contentId: string;
  triggerEventType: string;
  rawAmount: number;
  weightedAmount: number;
  distributions: StageTrace[];
  economicEventIds: string[];
  limitsApplied: unknown[];
}

export interface StageTrace {
  stewardPresenceId: string;
  allocationRatio: number;
  storedAffinity: number;
  derivedAffinity: number;
  effectiveRatio: number;
  preLimitShare: number;
  finalShare: number;
  limitReasons: unknown[];
  economicEventId: string;
}
```

**Step 4: Run test to verify it passes**

Run: `cd /projects/elohim/app/elohim-app && pnpm exec vitest run --config vite.config.ts "recognition-api" 2>&1 | tail -15`

Expected: 2 tests PASS.

**Step 5: Commit**

```bash
git add app/elohim-app/src/app/elohim/services/recognition-api.service.ts app/elohim-app/src/app/elohim/services/recognition-api.service.spec.ts
git commit -m "feat(app): add RecognitionApiService thin client in elohim pillar

Zero logic — calls POST /api/v1/recognition/distribute and returns the result.
All economic intelligence lives in the Rust pipeline."
```

---

### Task 8: Proxy Configuration

**Files:**
- Modify: `app/elohim-app/proxy.conf.mjs` (if recognition routes need proxying)

**Step 1: Check if proxy already covers /api/v1/**

Run: `cd /projects/elohim/app/elohim-app && grep -n "api/v1\|/api" proxy.conf.mjs | head -10`

If `/api/v1/*` is already proxied to the storage service, no changes needed. If not, add:

```javascript
'/api/v1/recognition': {
  target: 'http://localhost:8090',
  secure: false,
},
```

**Step 2: Commit if changed**

```bash
git add app/elohim-app/proxy.conf.mjs
git commit -m "fix(app): add recognition API route to dev proxy config"
```

---

### Task 9: Update CLAUDE-PICKS.md

**Files:**
- Modify: `CLAUDE-PICKS.md`

**Step 1: Update the Steward Economy Services entry**

Update section 3 to reflect what was built:

```markdown
## 3. Steward Economy Services

Recognition pipeline service built in Rust (`elohim-storage/src/services/recognition_pipeline_service.rs`) with 5 composable stages: normalize (event type weights) → resolve (steward allocations + affinity) → weight (proportional with affinity coefficient) → limit (constitutional checks, v0 passthrough) → settle (economic events + recognition accumulation). Exposed via `POST /api/v1/recognition/distribute`. Angular thin client in elohim pillar.

**Remaining**: v0 affinity defaults to 1.0 — wire stored_affinity from node_stewardship and derived_affinity from human profiles. Constitutional limit enforcement (stage 4). Future distribution models documented in `genesis/research/economic/future-distribution-models.md`.

**Impact**: High for M5-M6.
**Effort**: Medium remaining. REA coordination layer wired; deeper economics is research.
```

**Step 2: Commit**

```bash
git add CLAUDE-PICKS.md
git commit -m "docs: update CLAUDE-PICKS with recognition pipeline progress"
```

---

## Summary

| Task | What | Files |
|------|------|-------|
| 1 | Domain types + pure stage functions | `recognition_pipeline_service.rs`, `services/mod.rs` |
| 2 | View types + ts-rs exports | `views.rs` |
| 3 | Resolve + Settle (DB stages) | `recognition_pipeline_service.rs` |
| 4 | Pipeline orchestrator | `recognition_pipeline_service.rs` |
| 5 | API controller + route registration | `api/recognition.rs`, `api/mod.rs` |
| 6 | Route documentation | `http.rs` |
| 7 | Angular thin client | `recognition-api.service.ts` + spec |
| 8 | Proxy config | `proxy.conf.mjs` (if needed) |
| 9 | Update CLAUDE-PICKS | `CLAUDE-PICKS.md` |
