//! Recognition pipeline service — distributes recognition to content stewards
//!
//! Processes recognition events through a staged pipeline:
//! 1. **Normalize** — weight raw recognition by event type
//! 2. **Resolve** — look up steward allocations for content
//! 3. **Weight** — apply affinity-weighted distribution
//! 4. **Limit** — apply floor/ceiling constraints (v0: passthrough)
//! 5. **Settle** — persist economic events and accumulate recognition
//!
//! Stages 1, 3, and 4 are pure functions with no database dependency.
//! Stages 2 and 5 interact with the database.

use chrono::Utc;
use diesel::SqliteConnection;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::db::economic_events::{record_event, CreateEconomicEventInput};
use crate::db::models::StewardshipAllocation;
use crate::db::steward_affinity;
use crate::db::stewardship_allocations;
use crate::db::AppContext;
use crate::error::StorageError;

// =============================================================================
// Domain Types
// =============================================================================

/// Input trigger that starts the recognition pipeline.
#[derive(Debug, Clone)]
pub struct RecognitionTrigger {
    pub content_id: String,
    pub event_type: String,
    pub raw_amount: f64,
    pub triggered_by: Option<String>,
}

/// Stage 1 output: trigger normalized by event-type weight.
#[derive(Debug, Clone)]
pub struct NormalizedTrigger {
    pub content_id: String,
    pub event_type: String,
    pub raw_amount: f64,
    pub weight: f64,
    pub weighted_amount: f64,
}

/// Stage 2 output: a resolved steward with allocation and affinity data.
#[derive(Debug, Clone)]
pub struct ResolvedSteward {
    pub allocation_id: String,
    pub steward_presence_id: String,
    pub allocation_ratio: f32,
    pub stored_affinity: f64,
    pub derived_affinity: f64,
    pub contribution_type: String,
}

/// Stage 3 output: affinity-weighted share of the recognition amount.
#[derive(Debug, Clone)]
pub struct WeightedShare {
    pub allocation_id: String,
    pub steward_presence_id: String,
    pub effective_ratio: f64,
    pub share_amount: f64,
}

/// Stage 4 output: share after floor/ceiling limits applied.
#[derive(Debug, Clone)]
pub struct LimitedShare {
    pub allocation_id: String,
    pub steward_presence_id: String,
    pub pre_limit_amount: f64,
    pub final_amount: f64,
    pub limit_reasons: Vec<LimitReason>,
}

/// Reason a limit was applied to a steward's share.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum LimitReason {
    None,
    FloorApplied { floor: f64, original: f64 },
    CeilingApplied { ceiling: f64, excess: f64 },
    ExcessRedistributed { from_steward: String, amount: f64 },
}

/// Per-steward trace through all pipeline stages.
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
    pub economic_event_id: Option<String>,
}

/// Final pipeline result with full trace.
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

// =============================================================================
// Stage 1: Normalize Trigger
// =============================================================================

const DEFAULT_EVENT_WEIGHT: f64 = 0.01;

/// Event-type weight for recognition normalization.
fn event_type_weight(event_type: &str) -> f64 {
    match event_type {
        "mastery_completion" => 1.0,
        "content_citation" => 0.5,
        "assessment_attempt" => 0.1,
        "content_access" => 0.01,
        _ => DEFAULT_EVENT_WEIGHT,
    }
}

/// Stage 1: Normalize a recognition trigger by applying event-type weights.
///
/// Looks up the weight for the given event type (defaulting to 0.01 for unknown
/// types) and computes weighted_amount = raw_amount * weight.
pub fn normalize_trigger(content_id: &str, event_type: &str, raw_amount: f64) -> NormalizedTrigger {
    let weight = event_type_weight(event_type);
    NormalizedTrigger {
        content_id: content_id.to_string(),
        event_type: event_type.to_string(),
        raw_amount,
        weight,
        weighted_amount: raw_amount * weight,
    }
}

// =============================================================================
// Stage 2: Resolve Stewards
// =============================================================================

/// Stage 2 (pure): Build resolved stewards from allocation records.
/// v0: stored_affinity and derived_affinity default to 1.0.
pub fn resolve_from_allocations(allocations: &[StewardshipAllocation]) -> Vec<ResolvedSteward> {
    allocations
        .iter()
        .filter(|a| a.governance_state == "active")
        .map(|a| ResolvedSteward {
            allocation_id: a.id.clone(),
            steward_presence_id: a.steward_presence_id.clone(),
            allocation_ratio: a.allocation_ratio,
            stored_affinity: 1.0,
            derived_affinity: 1.0,
            contribution_type: a.contribution_type.clone(),
        })
        .collect()
}

/// Stage 2 (pure): Build resolved stewards with real affinity data.
/// Falls back to 0.0 for stewards without affinity records (no affinity = no share).
pub fn resolve_from_allocations_with_affinity(
    allocations: &[StewardshipAllocation],
    affinity_map: &std::collections::HashMap<String, f64>,
) -> Vec<ResolvedSteward> {
    allocations
        .iter()
        .filter(|a| a.governance_state == "active")
        .map(|a| {
            let stored_affinity = affinity_map
                .get(&a.steward_presence_id)
                .copied()
                .unwrap_or(0.0);
            ResolvedSteward {
                allocation_id: a.id.clone(),
                steward_presence_id: a.steward_presence_id.clone(),
                allocation_ratio: a.allocation_ratio,
                stored_affinity,
                derived_affinity: 1.0,
                contribution_type: a.contribution_type.clone(),
            }
        })
        .collect()
}

/// Stage 2 (DB): Query allocations for content, then resolve with real affinity.
pub fn resolve_stewards(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    content_id: &str,
) -> Result<Vec<ResolvedSteward>, StorageError> {
    let allocations = stewardship_allocations::get_allocations_for_content(conn, ctx, content_id)?;

    // Build affinity map from DB
    let affinities = steward_affinity::list_affinities(
        conn,
        ctx,
        &steward_affinity::AffinityQuery {
            content_id: Some(content_id.to_string()),
            ..Default::default()
        },
    )?;

    let affinity_map: std::collections::HashMap<String, f64> = affinities
        .into_iter()
        .map(|a| (a.steward_id, a.affinity_score as f64))
        .collect();

    Ok(resolve_from_allocations_with_affinity(&allocations, &affinity_map))
}

// =============================================================================
// Stage 3: Apply Affinity Weights
// =============================================================================

/// Stage 3: Distribute weighted_amount across stewards by effective ratio.
///
/// effective_ratio = allocation_ratio * stored_affinity * derived_affinity
/// Each steward's share = weighted_amount * (their effective_ratio / sum of all).
/// Returns empty vec for empty input; all zeros if total effective ratio is zero.
pub fn apply_weights(stewards: &[ResolvedSteward], weighted_amount: f64) -> Vec<WeightedShare> {
    if stewards.is_empty() {
        return Vec::new();
    }

    let effective_ratios: Vec<f64> = stewards
        .iter()
        .map(|s| (s.allocation_ratio as f64) * s.stored_affinity * s.derived_affinity)
        .collect();

    let total_effective: f64 = effective_ratios.iter().sum();

    stewards
        .iter()
        .zip(effective_ratios.iter())
        .map(|(s, &eff)| {
            let share = if total_effective < f64::EPSILON {
                0.0
            } else {
                weighted_amount * (eff / total_effective)
            };
            WeightedShare {
                allocation_id: s.allocation_id.clone(),
                steward_presence_id: s.steward_presence_id.clone(),
                effective_ratio: eff,
                share_amount: share,
            }
        })
        .collect()
}

// =============================================================================
// Stage 4: Apply Limits (v0: passthrough)
// =============================================================================

/// Stage 4: Apply floor/ceiling limits to weighted shares.
///
/// v0 implementation: passthrough — final_amount equals share_amount with no
/// limit reasons applied. Future versions will enforce min/max constraints and
/// redistribute excess.
pub fn apply_limits(shares: &[WeightedShare]) -> Vec<LimitedShare> {
    shares
        .iter()
        .map(|s| LimitedShare {
            allocation_id: s.allocation_id.clone(),
            steward_presence_id: s.steward_presence_id.clone(),
            pre_limit_amount: s.share_amount,
            final_amount: s.share_amount,
            limit_reasons: Vec::new(),
        })
        .collect()
}

// =============================================================================
// Stage 5: Settle (persist economic events)
// =============================================================================

/// Stage 5: Create economic events and accumulate recognition for each share.
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

        stewardship_allocations::accumulate_recognition(
            conn,
            ctx,
            &share.allocation_id,
            share.final_amount as f32,
        )?;

        traces.push(StageTrace {
            steward_presence_id: share.steward_presence_id.clone(),
            allocation_ratio: 0.0,
            stored_affinity: 0.0,
            derived_affinity: 0.0,
            effective_ratio: 0.0,
            pre_limit_share: share.pre_limit_amount,
            final_share: share.final_amount,
            limit_reasons: share.limit_reasons.clone(),
            economic_event_id: Some(event_id),
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

// ---------------------------------------------------------------------------
// Pipeline Orchestrator
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
    let normalized =
        normalize_trigger(&trigger.content_id, &trigger.event_type, trigger.raw_amount);

    if normalized.weighted_amount <= 0.0 {
        return Ok(RecognitionDistributionResult {
            content_id: trigger.content_id,
            trigger_event_type: trigger.event_type,
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
            content_id: trigger.content_id,
            trigger_event_type: trigger.event_type,
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

    let event_ids: Vec<String> = traces
        .iter()
        .filter_map(|t| t.economic_event_id.clone())
        .collect();
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

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_known_event_type() {
        let result = normalize_trigger("content-1", "mastery_completion", 10.0);
        assert_eq!(result.weight, 1.0);
        assert_eq!(result.weighted_amount, 10.0);
        assert_eq!(result.content_id, "content-1");
        assert_eq!(result.event_type, "mastery_completion");
        assert_eq!(result.raw_amount, 10.0);
    }

    #[test]
    fn normalize_micro_recognition() {
        let result = normalize_trigger("content-2", "content_access", 1.0);
        assert_eq!(result.weight, 0.01);
        assert!((result.weighted_amount - 0.01).abs() < f64::EPSILON);
    }

    #[test]
    fn normalize_unknown_event_type_uses_default() {
        let result = normalize_trigger("content-3", "some_future_event", 5.0);
        assert_eq!(result.weight, 0.01);
        assert!((result.weighted_amount - 0.05).abs() < f64::EPSILON);
    }

    fn make_steward(
        id: &str,
        presence: &str,
        ratio: f32,
        stored: f64,
        derived: f64,
    ) -> ResolvedSteward {
        ResolvedSteward {
            allocation_id: id.to_string(),
            steward_presence_id: presence.to_string(),
            allocation_ratio: ratio,
            stored_affinity: stored,
            derived_affinity: derived,
            contribution_type: "author".to_string(),
        }
    }

    #[test]
    fn weights_single_steward() {
        let stewards = vec![make_steward("a1", "p1", 1.0, 1.0, 1.0)];
        let shares = apply_weights(&stewards, 10.0);
        assert_eq!(shares.len(), 1);
        assert!((shares[0].share_amount - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn weights_two_stewards_equal_ratio_different_affinity() {
        // Steward A: ratio=0.5, stored=0.9, derived=1.0 -> eff = 0.45
        // Steward B: ratio=0.5, stored=0.3, derived=1.0 -> eff = 0.15
        // Total effective = 0.60
        // A share = 12.0 * 0.45/0.60 = 9.0
        // B share = 12.0 * 0.15/0.60 = 3.0
        let stewards = vec![
            make_steward("a1", "p1", 0.5, 0.9, 1.0),
            make_steward("a2", "p2", 0.5, 0.3, 1.0),
        ];
        let shares = apply_weights(&stewards, 12.0);
        assert_eq!(shares.len(), 2);
        assert!((shares[0].share_amount - 9.0).abs() < 1e-10);
        assert!((shares[1].share_amount - 3.0).abs() < 1e-10);
    }

    #[test]
    fn weights_empty_stewards() {
        let shares = apply_weights(&[], 10.0);
        assert!(shares.is_empty());
    }

    fn make_allocation(
        id: &str,
        steward_presence_id: &str,
        ratio: f32,
        governance_state: &str,
    ) -> StewardshipAllocation {
        StewardshipAllocation {
            id: id.to_string(),
            app_id: "test-app".to_string(),
            content_id: "content-1".to_string(),
            steward_presence_id: steward_presence_id.to_string(),
            allocation_ratio: ratio,
            allocation_method: "manual".to_string(),
            contribution_type: "author".to_string(),
            contribution_evidence_json: None,
            governance_state: governance_state.to_string(),
            dispute_id: None,
            dispute_reason: None,
            disputed_at: None,
            disputed_by: None,
            negotiation_session_id: None,
            elohim_ratified_at: None,
            elohim_ratifier_id: None,
            effective_from: "2026-01-01T00:00:00Z".to_string(),
            effective_until: None,
            superseded_by: None,
            recognition_accumulated: 0.0,
            last_recognition_at: None,
            note: None,
            metadata_json: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn resolve_builds_stewards_from_allocations() {
        let allocations = vec![
            make_allocation("alloc-1", "steward-1", 0.6, "active"),
            make_allocation("alloc-2", "steward-2", 0.4, "active"),
        ];
        let resolved = resolve_from_allocations(&allocations);
        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[0].steward_presence_id, "steward-1");
        assert_eq!(resolved[0].allocation_ratio, 0.6);
        assert_eq!(resolved[0].stored_affinity, 1.0);
        assert_eq!(resolved[0].derived_affinity, 1.0);
    }

    #[test]
    fn resolve_filters_non_active_allocations() {
        let allocations = vec![make_allocation("alloc-1", "steward-1", 0.6, "disputed")];
        let resolved = resolve_from_allocations(&allocations);
        assert!(resolved.is_empty());
    }

    #[test]
    fn resolve_with_affinity_uses_provided_scores() {
        let allocations = vec![
            make_allocation("alloc-1", "steward-1", 0.6, "active"),
            make_allocation("alloc-2", "steward-2", 0.4, "active"),
        ];
        let affinity_map = vec![
            ("steward-1".to_string(), 0.85_f64),
            ("steward-2".to_string(), 0.50_f64),
        ]
        .into_iter()
        .collect::<std::collections::HashMap<_, _>>();

        let resolved = resolve_from_allocations_with_affinity(&allocations, &affinity_map);
        assert_eq!(resolved.len(), 2);
        assert!((resolved[0].stored_affinity - 0.85).abs() < f64::EPSILON);
        assert!((resolved[1].stored_affinity - 0.50).abs() < f64::EPSILON);
    }

    #[test]
    fn resolve_with_affinity_defaults_to_zero_when_missing() {
        let allocations = vec![make_allocation("alloc-1", "steward-1", 0.6, "active")];
        let affinity_map = std::collections::HashMap::new();

        let resolved = resolve_from_allocations_with_affinity(&allocations, &affinity_map);
        assert_eq!(resolved.len(), 1);
        assert!((resolved[0].stored_affinity - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn limits_passthrough_preserves_amounts() {
        let shares = vec![
            WeightedShare {
                allocation_id: "a1".to_string(),
                steward_presence_id: "p1".to_string(),
                effective_ratio: 0.5,
                share_amount: 7.5,
            },
            WeightedShare {
                allocation_id: "a2".to_string(),
                steward_presence_id: "p2".to_string(),
                effective_ratio: 0.5,
                share_amount: 2.5,
            },
        ];
        let limited = apply_limits(&shares);
        assert_eq!(limited.len(), 2);
        assert!((limited[0].final_amount - 7.5).abs() < f64::EPSILON);
        assert!((limited[0].pre_limit_amount - 7.5).abs() < f64::EPSILON);
        assert!(limited[0].limit_reasons.is_empty());
        assert!((limited[1].final_amount - 2.5).abs() < f64::EPSILON);
        assert!(limited[1].limit_reasons.is_empty());
    }
}
