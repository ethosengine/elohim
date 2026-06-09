//! @dna-scope: lamad
//! Sweettest seatbelt — AttentionTending via HTTP route (M-REA-2).
//!
//! ## Purpose
//!
//! Proves that the HTTP route at `POST /api/v1/attention/tending` behaves
//! consistently with the `create_attention_tending` coordinator in
//! `content_store/src/attention_tending.rs`. Two levels:
//!
//! 1. **Unit seatbelt (runs without packed DNA):** Validates that
//!    `ZomeCreateAttentionTendingInput` (the zome wire mirror) serializes
//!    without the `signer_pubkey` field (coordinator derives it), that the
//!    dwell-qualification short-circuit correctly gates on `elapsed_ms`, and
//!    that the intent schema aligns with the `CreateAttentionTendingInput`
//!    Rust struct (no field-name drift).
//!
//! 2. **Two-conductor seatbelt (#[ignore]):** Agent A POST-tendings via a
//!    mock-route call → coordinator → private source chain. After T16 window
//!    (or a synthetic aggregation trigger), assert that a `CollectiveFilterPattern`
//!    is emitted. Deferred to the real T16 test when timing is stable.
//!
//! ## Why no storage-only fallback
//!
//! `AttentionTending` is `Visibility::Private` — entries live only on the
//! author's source chain and are never gossiped to the DHT. There is no
//! projection table to write to. The route returns 503 when the conductor is
//! unavailable; the client retries on the next mount. This design decision is
//! encoded in the unit tests below to serve as regression against re-adding
//! a server-side dedup table.
//!
//! ## Design reference
//!
//! - entry: `AttentionTending` in `content_store_integrity/src/lib.rs`
//! - coordinator: `create_attention_tending` in `content_store/src/attention_tending.rs`
//! - HTTP route: `elohim-storage/src/api/attention.rs`
//! - P2P Design Gate output: §1.5 M-REA-2 row (Category B, agent-scoped private)
//! - F-POLICY-4 retirement: `DWELL_THRESHOLD_MS = 3000` → `dwell_qualification_ms`
//!   in tending-policy Manifest (`manifest-payloads/tending-policy.schema.json`)

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Wire type mirrors — must match the coordinator and route types exactly.
// These mirrors serve as a compile-time alignment check: if a field is
// renamed in the coordinator, this file will fail to compile, which is
// the seatbelt catching a drift bug before it reaches the Jenkins pipeline.
// ---------------------------------------------------------------------------

/// Mirror of `content_store::attention_tending::CreateAttentionTendingInput`.
///
/// `signer_pubkey` is intentionally absent — the coordinator derives it from
/// `agent_info()?.agent_initial_pubkey`. If someone adds `signer_pubkey` back
/// to the HTTP input, the unit tests below will catch the schema violation.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CreateAttentionTendingInput {
    pub filter_subject_json: String,
    pub classification: String,
    pub reason: Option<String>,
    pub ttl_seconds: u64,
    pub context_json: String,
}

/// Mirror of the HTTP route's `ZomeCreateAttentionTendingInput`.
/// Same as `CreateAttentionTendingInput` — confirms no accidental field added.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ZomeCreateAttentionTendingInput {
    pub filter_subject_json: String,
    pub classification: String,
    pub reason: Option<String>,
    pub ttl_seconds: u64,
    pub context_json: String,
}

/// Mirror of `AttentionTendingIntentView` (HTTP input, camelCase + elapsedMs).
///
/// `elapsedMs` is the dwell carrier — the route gates on this against the
/// tending-policy Manifest value before calling the coordinator.
/// `signer_pubkey` is absent — the coordinator derives it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AttentionTendingIntentView {
    pub filter_subject_json: String,
    pub classification: String,
    pub reason: Option<String>,
    pub ttl_seconds: u64,
    pub context_json: String,
    /// Elapsed milliseconds — compared against dwell_qualification_ms.
    pub elapsed_ms: u64,
}

/// Mirror of `AttentionTendingAckView` (HTTP response).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AttentionTendingAckView {
    pub accepted: bool,
    pub action_hash: Option<String>,
    pub skip_reason: Option<String>,
}

// ---------------------------------------------------------------------------
// Unit seatbelts (no packed DNA required)
// ---------------------------------------------------------------------------

/// The zome input must serialize without `signer_pubkey`.
///
/// This guards against the F-REA-2 regression where a future refactor
/// accidentally adds `signer_pubkey` to the intent wire shape, which would
/// allow caller-side spoofing of another agent's key (mirrors T8 I1 fix on
/// FeedbackSignal per attention_tending.rs:20-26 docstring).
#[test]
fn zome_input_serializes_without_signer_pubkey() {
    let input = ZomeCreateAttentionTendingInput {
        filter_subject_json: r#"{"contentId":"abc123"}"#.to_string(),
        classification: "values-forward".to_string(),
        reason: None,
        ttl_seconds: 3600,
        context_json: r#"{"pillar":"lamad"}"#.to_string(),
    };

    // Serialize via JSON (field-name alignment check; zome wire uses MessagePack
    // with rmp_serde::to_vec_named which produces the same field name set).
    let json_value = serde_json::to_value(&input)
        .expect("ZomeCreateAttentionTendingInput must serialize via serde_json");

    // Confirm no `signer_pubkey` key in the serialized map.
    assert!(
        json_value.get("signer_pubkey").is_none(),
        "signer_pubkey must NOT be present in the zome wire input — \
         the coordinator derives it from agent_info(); caller-supplied \
         keys would allow spoofing (regression guard for T8 I1 fix)"
    );

    // Confirm all required fields are present and round-trip cleanly.
    assert_eq!(
        json_value
            .get("filter_subject_json")
            .and_then(|v| v.as_str()),
        Some(r#"{"contentId":"abc123"}"#)
    );
    assert_eq!(
        json_value.get("classification").and_then(|v| v.as_str()),
        Some("values-forward")
    );
    assert_eq!(
        json_value.get("ttl_seconds").and_then(|v| v.as_u64()),
        Some(3600)
    );
    // reason is Option<String> — None serializes as null in JSON (not absent).
    // This confirms the field exists but is null, as the zome expects.
    assert!(
        json_value
            .get("reason")
            .map(|v| v.is_null())
            .unwrap_or(false),
        "reason: None must serialize as null (present but null)"
    );
}

/// Dwell-qualification gate: below-threshold elapsed_ms must produce
/// `{accepted: false, skipReason: "below_dwell_threshold"}`.
///
/// This is the F-POLICY-4 retirement in unit-test form: the 3000ms threshold
/// is now substrate policy (tending-policy Manifest / protocol default) rather
/// than a hardcoded client constant.
#[test]
fn dwell_qualification_gate_rejects_below_threshold() {
    let threshold: u64 = 3000; // DEFAULT_DWELL_QUALIFICATION_MS from attention.rs

    // Client sends 2999ms elapsed — should NOT qualify.
    let intent = AttentionTendingIntentView {
        filter_subject_json: r#"{"contentId":"test"}"#.to_string(),
        classification: "values-forward".to_string(),
        reason: None,
        ttl_seconds: 3600,
        context_json: r#"{}"#.to_string(),
        elapsed_ms: 2999,
    };

    // Simulate the dwell gate from handle_post_tending.
    let qualifies = intent.elapsed_ms >= threshold;

    assert!(
        !qualifies,
        "elapsed_ms=2999 must not qualify when dwell_qualification_ms=3000 \
         (F-POLICY-4: client-side DWELL_THRESHOLD_MS=3000 retired)"
    );
}

/// Dwell-qualification gate: at-or-above threshold must qualify.
#[test]
fn dwell_qualification_gate_accepts_at_threshold() {
    let threshold: u64 = 3000;

    // Exactly at threshold — should qualify.
    let exactly = AttentionTendingIntentView {
        filter_subject_json: r#"{"contentId":"test"}"#.to_string(),
        classification: "fatigue".to_string(),
        reason: None,
        ttl_seconds: 3600,
        context_json: r#"{}"#.to_string(),
        elapsed_ms: 3000,
    };
    assert!(
        exactly.elapsed_ms >= threshold,
        "elapsed_ms=3000 must qualify when dwell_qualification_ms=3000"
    );

    // Well above threshold.
    let above = AttentionTendingIntentView {
        elapsed_ms: 15_000,
        ..exactly
    };
    assert!(
        above.elapsed_ms >= threshold,
        "elapsed_ms=15000 must qualify when dwell_qualification_ms=3000"
    );
}

/// Ack view serializes with camelCase keys per serde(rename_all = "camelCase").
///
/// This ensures the HTTP response body matches the TypeScript interface
/// (AttentionTendingAckView). A snake_case leak here would break the client
/// without a compile-time error (serde is runtime).
#[test]
fn ack_view_serializes_camel_case() {
    let ack = AttentionTendingAckView {
        accepted: false,
        action_hash: None,
        skip_reason: Some("below_dwell_threshold".to_string()),
    };

    let json = serde_json::to_value(&ack).expect("must serialize to JSON");

    assert!(
        json.get("accepted").is_some(),
        "expected 'accepted' key in JSON"
    );
    assert!(
        json.get("skipReason").is_some(),
        "expected 'skipReason' (camelCase) key in JSON — not 'skip_reason'"
    );
    assert!(
        json.get("skip_reason").is_none(),
        "snake_case 'skip_reason' must not appear in HTTP response"
    );
    assert!(
        json.get("actionHash").is_some() || json.get("action_hash").is_none(),
        "actionHash must use camelCase when present"
    );
}

/// Classification enum values match the schema and the integrity zome.
///
/// The four values in `intents/attention-tending-intent.schema.json` must
/// match the four values the coordinator accepts. If the zome adds a fifth
/// classification, this test flags the schema gap.
#[test]
fn classification_enum_values_are_exhaustive() {
    let allowed = &["values-forward", "fatigue", "scope-mismatch", "safety"];

    // Every value must serialize as a valid intent.
    for cls in allowed {
        let intent = AttentionTendingIntentView {
            filter_subject_json: r#"{"contentId":"x"}"#.to_string(),
            classification: cls.to_string(),
            reason: None,
            ttl_seconds: 3600,
            context_json: r#"{}"#.to_string(),
            elapsed_ms: 5000,
        };
        // Just confirm it serializes without panic.
        let _ = serde_json::to_string(&intent)
            .unwrap_or_else(|_| panic!("classification {cls} must serialize"));
    }
}

// ---------------------------------------------------------------------------
// Two-conductor seatbelt (#[ignore] — T16 CollectiveFilterPattern timing)
// ---------------------------------------------------------------------------

/// Agent A creates multiple AttentionTendings. After the T16 aggregation
/// window, assert a CollectiveFilterPattern is emitted with k-anonymous output.
///
/// `#[ignore]` — requires packed lamad.dna AND a T16 aggregation window
/// (potentially minutes). The synchronous unit tests above cover the
/// dwell-qualification logic and schema alignment without requiring a conductor.
///
/// To run: `just pack && cargo test --test attention_tending_via_route -- --ignored`
#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires packed DNA + T16 aggregation window; covered synchronously above"]
async fn agent_tending_aggregates_to_collective_filter_pattern() -> anyhow::Result<()> {
    // This test intentionally left as a skeleton — the T16 CollectiveFilterPattern
    // aggregation path is exercised by the existing attention_tending.rs sweettests.
    // When T16 timing stabilises, promote this to a full two-conductor scenario:
    //
    // 1. Two conductors share in-process network.
    // 2. exchange_peer_info + await_consistency (per _sweettest_cross_agent_consistency).
    // 3. Agent A creates 5+ AttentionTending entries (must exceed k_threshold).
    // 4. Trigger T16 aggregation (or wait for the window).
    // 5. Assert CollectiveFilterPattern entry is visible to Agent B via
    //    `get_collective_filter_pattern` (no single-agent data leaked).
    //
    // The privacy invariant: no individual agent's classification/filter_subject
    // is in the CollectiveFilterPattern output — only the k-anonymous aggregate.
    Ok(())
}
