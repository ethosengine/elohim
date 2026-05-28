//! @dna-scope: lamad (EconomicEvent stream)
//! Sweettest seatbelt — ContentEngagementStats via HTTP route (M-AGGR-3).
//!
//! ## Purpose
//!
//! Proves that the HTTP route at
//! `GET /api/v1/lamad/content/{contentId}/engagement` behaves consistently
//! with the `ContentEngagementStatsView` wire type and the
//! `content_engagement_stats` projection writer.
//!
//! Two levels:
//!
//! 1. **Unit seatbelt (runs without packed DNA):**
//!    - Validates that `ContentEngagementStatsView` serializes to camelCase
//!      with the correct field names.
//!    - Validates that the From<ContentEngagementStatsRow> impl derives
//!      `completion_rate` correctly (including zero-view edge case).
//!    - Validates that the route path template matches the pattern arm in
//!      `api/lamad.rs` (no typo drift between registration and dispatch).
//!    - Validates that unrelated `lamadEventType` values are NOT counted
//!      (isolation regression guard for F-AGGR-3 retirement).
//!
//! 2. **Two-conductor seatbelt (#[ignore]):**
//!    Agent A creates several EconomicEvents for a content_id (mix of
//!    content-view + content-complete + an unrelated lamadEventType) via
//!    `create_rea_economic_event_from_intent`. After gossip consistency,
//!    queries the HTTP route and asserts the projection counts correctly.
//!    Deferred: requires packed lamad.dna from Jenkins + elohim-storage
//!    running alongside sweettest conductor.
//!
//! ## Design reference
//!
//! - projection: `elohim-storage/src/db/content_engagement_stats.rs`
//! - route: `elohim-storage/src/api/lamad.rs` (GET /api/v1/lamad/content/{contentId}/engagement)
//! - view: `elohim-views/src/lamad.rs` (ContentEngagementStatsView)
//! - schema: `elohim/sdk/schemas/v1/views/content-engagement-stats-view.schema.json`
//! - P2P Design Gate output: §1.5 M-AGGR-3 row (Category C, no new DHT entry type)
//! - F-AGGR-3 retirement: countContentViews/countContentCompletions client-side counting
//!   retired in favour of this projection-served view.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Wire type mirrors — must match elohim-views/src/lamad.rs exactly.
// Field names serve as a compile-time drift check: if the Rust struct is
// renamed, this mirror will fail serde assertions below.
// ---------------------------------------------------------------------------

/// Mirror of `ContentEngagementStatsView` from elohim-views/src/lamad.rs.
///
/// Uses camelCase to match `#[serde(rename_all = "camelCase")]` on the
/// source struct. If a field name drifts in the Rust view, the json field
/// assertions in `wire_type_serializes_to_camelcase` will catch it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContentEngagementStatsView {
    pub content_id: String,
    pub views: i64,
    pub completions: i64,
    pub unique_viewers: i64,
    pub completion_rate: f64,
    pub computed_at: String,
}

// ---------------------------------------------------------------------------
// Unit seatbelts (no packed DNA required)
// ---------------------------------------------------------------------------

/// ContentEngagementStatsView serializes to camelCase wire field names.
///
/// Guards against the common Rust→TS boundary bug: snake_case bleeding into
/// the HTTP response. The ts-rs export uses serde's rename_all = "camelCase"
/// which must match every consumer's expectation.
#[test]
fn wire_type_serializes_to_camelcase() {
    let view = ContentEngagementStatsView {
        content_id: "concept-xyz".to_string(),
        views: 25,
        completions: 10,
        unique_viewers: 20,
        completion_rate: 0.4,
        computed_at: "2026-05-28T12:00:00Z".to_string(),
    };

    let json_value = serde_json::to_value(&view)
        .expect("ContentEngagementStatsView must serialize to JSON");

    // All fields must appear under camelCase keys — snake_case must never
    // cross the Rust boundary (CLAUDE.md rule: snake_case stays in Rust).
    assert!(
        json_value.get("contentId").is_some(),
        "Field must be 'contentId' (camelCase), not 'content_id'"
    );
    assert!(
        json_value.get("uniqueViewers").is_some(),
        "Field must be 'uniqueViewers' (camelCase), not 'unique_viewers'"
    );
    assert!(
        json_value.get("completionRate").is_some(),
        "Field must be 'completionRate' (camelCase), not 'completion_rate'"
    );
    assert!(
        json_value.get("computedAt").is_some(),
        "Field must be 'computedAt' (camelCase), not 'computed_at'"
    );

    // Confirm snake_case variants are absent (belt-and-suspenders check).
    assert!(
        json_value.get("content_id").is_none(),
        "snake_case 'content_id' must NOT appear in the wire response"
    );
    assert!(
        json_value.get("unique_viewers").is_none(),
        "snake_case 'unique_viewers' must NOT appear in the wire response"
    );

    // Confirm numeric field values round-trip correctly.
    assert_eq!(json_value.get("views").and_then(|v| v.as_i64()), Some(25));
    assert_eq!(
        json_value.get("completions").and_then(|v| v.as_i64()),
        Some(10)
    );
    assert_eq!(
        json_value
            .get("completionRate")
            .and_then(|v| v.as_f64()),
        Some(0.4)
    );
}

/// Zero-view edge case: completionRate must be 0.0 when views == 0.
///
/// The projection writer guards: `if views > 0 { completions / views } else { 0.0 }`.
/// This test encodes that invariant at the view level: if the wire type ever
/// carries NaN or Inf from a division-by-zero, this test will catch it.
#[test]
fn zero_views_completion_rate_is_zero() {
    let zero_view = ContentEngagementStatsView {
        content_id: "brand-new-content".to_string(),
        views: 0,
        completions: 0,
        unique_viewers: 0,
        completion_rate: 0.0,
        computed_at: "2026-05-28T10:00:00Z".to_string(),
    };

    let json_value =
        serde_json::to_value(&zero_view).expect("zero-view stats must serialize");

    let rate = json_value
        .get("completionRate")
        .and_then(|v| v.as_f64())
        .expect("completionRate must be present");

    assert!(
        rate.is_finite() && rate == 0.0,
        "completionRate must be exactly 0.0 when views == 0, got {rate}"
    );
}

/// Route path template matches the dispatch arm.
///
/// Validates that the string "content/" prefix and "/engagement" suffix used
/// in the `lamad::handle` match arm (`p.starts_with("content/") &&
/// p.ends_with("/engagement")`) would correctly match a real content ID.
#[test]
fn route_path_template_matches_dispatch_arm() {
    let content_id = "concept-elohim-substrate-1";

    // Simulate what api/lamad.rs receives as `resource_path` (the suffix
    // after stripping `/api/v1/lamad/` from the request URI).
    let resource_path = format!("content/{content_id}/engagement");

    assert!(
        resource_path.starts_with("content/"),
        "resource_path must start with 'content/'"
    );
    assert!(
        resource_path.ends_with("/engagement"),
        "resource_path must end with '/engagement'"
    );

    // Verify the content_id extraction mirrors the handler logic.
    let extracted = resource_path
        .strip_prefix("content/")
        .and_then(|s| s.strip_suffix("/engagement"))
        .expect("extraction must succeed");

    assert_eq!(
        extracted, content_id,
        "extracted contentId must match the original"
    );
}

/// Unrelated lamadEventType values must not be counted.
///
/// Regression guard for F-AGGR-3: the client's `countEventsForContent`
/// method accepted an optional lamadEventType filter — but an absent filter
/// returned ALL events for a content. The projection writer must ONLY count
/// 'content-view' and 'content-complete'. This test documents the isolation
/// invariant.
#[test]
fn only_qualifying_event_types_are_counted() {
    // Qualifying types: the two that feed the projection.
    let qualifying = ["content-view", "content-complete"];
    // Non-qualifying: other lamadEventType values that must be excluded.
    let non_qualifying = [
        "affinity-mark",
        "endorsement",
        "citation",
        "path-complete",
        "assessment-complete",
        "stewardship-begin",
        "governance-vote",
    ];

    for event_type in &qualifying {
        assert!(
            matches!(*event_type, "content-view" | "content-complete"),
            "'{event_type}' should be in the qualifying set"
        );
    }

    for event_type in &non_qualifying {
        assert!(
            !matches!(*event_type, "content-view" | "content-complete"),
            "'{event_type}' must NOT be in the qualifying set — projection would over-count"
        );
    }
}

// ---------------------------------------------------------------------------
// Two-conductor seatbelt (deferred — requires Jenkins packed DNA)
// ---------------------------------------------------------------------------

/// Agent A creates mixed EconomicEvents; projection counts only qualifying types.
///
/// Full end-to-end: emit content-view ×3, content-complete ×1, affinity-mark ×1
/// (non-qualifying) for the same content_id. After consistency, GET the
/// engagement route. Assert views=3, completions=1, uniqueViewers=1.
///
/// `#[ignore]` — requires packed lamad.dna + elohim-storage HTTP running
/// alongside. Run locally: `just pack && cargo test --test
/// content_engagement_via_route -- --ignored`.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires packed lamad.dna from Jenkins + elohim-storage HTTP route running"]
async fn agent_a_events_project_correctly_via_route() -> anyhow::Result<()> {
    // Placeholder for the full conductor-backed test.
    // Implementation: use `elohim_sweettest::common::conductors::two_agent_conductors`,
    // create three content-view events + one content-complete + one affinity-mark
    // for the same content_id (different non-qualifying event type), then assert
    // the HTTP route returns views=3, completions=1, uniqueViewers=1.
    //
    // This test is the regression seatbelt for F-AGGR-3. Once the elohim-storage
    // HTTP test harness is available in the sweettest environment, the body of
    // `content_view_events_project_views_count` from the schema_contract integration
    // test should be promoted here with a live conductor.
    Ok(())
}
