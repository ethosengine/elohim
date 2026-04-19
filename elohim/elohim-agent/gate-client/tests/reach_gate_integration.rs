//! End-to-end integration tests for the reach-gate v1 through the unified `check()` path.
//!
//! These tests exercise the full reach-gate flow:
//!
//! ```text
//! check(CapabilityInvoke { capability: "reach-negotiation" })
//!   → universal-band DAG (Allow in DevContext)
//!   → reach-gate-v1 (aggregate-attestations → synthesize / wisdom-edge → synthesize)
//!   → Verdict(GateTag::ReachLevel { level })
//! ```
//!
//! # Injection pattern
//!
//! The reach-gate's `assemble` step is skipped in DevContext (entrypoint overridden
//! to `"aggregate"` in `GateRegistry::build()`, matching the discernment-gate's
//! pattern).  Tests inject:
//!
//! 1. `__test_set_gate_ctx("reach-gate-v1", ctx_with_subject)` — sets the
//!    `subject` key that the `aggregate` step reads.
//! 2. `__test_set_attestation_resolver(Some(Arc::new(stub)))` — injects a
//!    `StubAttestationResolver` with fixture attestation records for that subject.
//!
//! # Reach levels (from reach-aggregation-v1 threshold-ladder)
//!
//! | Level       | Condition         |
//! |-------------|-------------------|
//! | `protocol`  | count ≥ 50        |
//! | `public`    | count ≥ 20        |
//! | `community` | count ≥ 5         |
//! | `self-reach`| count < 5 (default, level == null from ladder) |
//!
//! # Test inventory
//!
//! - **E2E-R1** — below lowest tier (2 attestations) → `self-reach`
//! - **E2E-R2** — community tier (10 attestations) → `community`
//! - **E2E-R3** — public tier (30 attestations) → `public`
//! - **E2E-R4** — protocol tier (60 attestations) → `protocol`
//! - **E2E-R5** — exactly at community boundary (5 attestations) → `community` with edge_case
//! - **E2E-R6** — non-reach CapabilityInvoke falls through to universal-band only
//! - **E2E-R7** — override short-circuits before reach-gate runs

use std::sync::Arc;

use gate_client::{
    check, AttestationRecord, GateContext, GateStatus, GateTag, RelationalImpactEvent,
    StubAttestationResolver,
};

// ─── Fixture helpers ──────────────────────────────────────────────────────────

/// The subject CID used across tests. Each test builds attestations keyed on this.
const TEST_SUBJECT: &str = "bafyreach-test-content-cid";

/// Build `count` attestation records for `subject` with the given `kind`.
///
/// All four kinds in the reach-aggregation spec are valid:
/// `gate-decision`, `witness`, `endorsement`, `content-approval`.
fn build_attestations(subject: &str, count: usize, kind: &str) -> Vec<AttestationRecord> {
    (0..count)
        .map(|i| AttestationRecord {
            kind: kind.to_string(),
            subject: subject.to_string(),
            issuer: format!("agent-issuer-{i}"),
            issued_at: "2026-01-01T00:00:00Z".to_string(),
            payload_json: "{}".to_string(),
        })
        .collect()
}

/// Set up the thread-locals needed for a reach-gate E2E test.
///
/// - Inserts `subject` into the gate context under key `"subject"`.
/// - Injects `attestation_records` into a `StubAttestationResolver` keyed on `subject`.
fn setup_reach_gate_test(subject: &str, attestation_records: Vec<AttestationRecord>) {
    // Context: pre-populate subject so the `aggregate` step can read it.
    let mut ctx = GateContext::new();
    ctx.insert("subject", serde_json::json!(subject));
    gate_client::__test_set_gate_ctx("reach-gate-v1", Some(ctx));

    // Attestation resolver: stub returns the fixture records for this subject.
    let resolver = Arc::new(StubAttestationResolver::single(
        subject,
        attestation_records,
    ));
    gate_client::__test_set_attestation_resolver(Some(resolver));
}

/// Build a `CapabilityInvoke` event for reach-negotiation.
fn reach_negotiation_event(requester: &str, request_id: &str) -> RelationalImpactEvent {
    RelationalImpactEvent::CapabilityInvoke {
        capability: "reach-negotiation".to_string(),
        requester: requester.to_string(),
        request_id: request_id.to_string(),
    }
}

/// Extract `level` from a `Verdict(GateTag::ReachLevel { level })` decision.
/// Panics with a descriptive message if the shape is wrong.
fn extract_reach_level(decision: &gate_client::GateDecision) -> &str {
    match &decision.status {
        GateStatus::Verdict(GateTag::ReachLevel { level }) => level.as_str(),
        other => panic!(
            "expected Verdict(ReachLevel {{..}}), got: {other:?}\n\
             Hint: check that __test_set_gate_ctx and __test_set_attestation_resolver were set."
        ),
    }
}

// ─── E2E-R1 — Below lowest tier → self-reach ─────────────────────────────────
//
// 2 attestations < community threshold (5) → level == null from ladder →
// `verdict-from-reach` synthesizer maps null to "self-reach".

#[tokio::test]
async fn e2e_r1_below_lowest_tier_yields_self_reach() {
    setup_reach_gate_test(
        TEST_SUBJECT,
        build_attestations(TEST_SUBJECT, 2, "endorsement"),
    );

    let event = reach_negotiation_event("test-requester", "r-1");
    let decision = check(event)
        .await
        .expect("check() must not fail for E2E-R1");

    // Assertion 1: Verdict with self-reach level.
    let level = extract_reach_level(&decision);
    assert_eq!(
        level, "self-reach",
        "E2E-R1: 2 attestations (< community=5) must yield self-reach; got: {level}"
    );

    // Assertion 2: decision_attestation_cid is Some (Phase 4/5 contract).
    assert!(
        decision.decision_attestation_cid.is_some(),
        "E2E-R1: reach-gate decision must carry decision_attestation_cid; got: {:?}",
        decision.decision_attestation_cid
    );

    // Assertion 3: CID has sha256- prefix.
    let cid = decision.decision_attestation_cid.as_ref().unwrap();
    assert!(
        cid.starts_with("sha256-"),
        "E2E-R1: decision_attestation_cid must start with 'sha256-'; got: {cid}"
    );
}

// ─── E2E-R2 — Community tier (10 attestations) ───────────────────────────────
//
// 10 attestations ≥ community threshold (5), < public (20) → level == "community".

#[tokio::test]
async fn e2e_r2_community_tier_at_10_attestations() {
    setup_reach_gate_test(
        TEST_SUBJECT,
        build_attestations(TEST_SUBJECT, 10, "witness"),
    );

    let event = reach_negotiation_event("test-requester", "r-2");
    let decision = check(event)
        .await
        .expect("check() must not fail for E2E-R2");

    // Assertion 1: community level.
    let level = extract_reach_level(&decision);
    assert_eq!(
        level, "community",
        "E2E-R2: 10 attestations (≥ community=5, < public=20) must yield community; got: {level}"
    );

    // Assertion 2: attestation CID is present.
    assert!(
        decision.decision_attestation_cid.is_some(),
        "E2E-R2: decision must carry decision_attestation_cid"
    );
}

// ─── E2E-R3 — Public tier (30 attestations) ──────────────────────────────────
//
// 30 attestations ≥ public threshold (20), < protocol (50) → level == "public".

#[tokio::test]
async fn e2e_r3_public_tier_at_30_attestations() {
    setup_reach_gate_test(
        TEST_SUBJECT,
        build_attestations(TEST_SUBJECT, 30, "gate-decision"),
    );

    let event = reach_negotiation_event("test-requester", "r-3");
    let decision = check(event)
        .await
        .expect("check() must not fail for E2E-R3");

    // Assertion 1: public level.
    let level = extract_reach_level(&decision);
    assert_eq!(
        level, "public",
        "E2E-R3: 30 attestations (≥ public=20, < protocol=50) must yield public; got: {level}"
    );

    // Assertion 2: attestation CID is present.
    assert!(
        decision.decision_attestation_cid.is_some(),
        "E2E-R3: decision must carry decision_attestation_cid"
    );
}

// ─── E2E-R4 — Protocol tier (60 attestations) ────────────────────────────────
//
// 60 attestations ≥ protocol threshold (50) → level == "protocol".

#[tokio::test]
async fn e2e_r4_protocol_tier_at_60_attestations() {
    setup_reach_gate_test(
        TEST_SUBJECT,
        build_attestations(TEST_SUBJECT, 60, "content-approval"),
    );

    let event = reach_negotiation_event("test-requester", "r-4");
    let decision = check(event)
        .await
        .expect("check() must not fail for E2E-R4");

    // Assertion 1: protocol level.
    let level = extract_reach_level(&decision);
    assert_eq!(
        level, "protocol",
        "E2E-R4: 60 attestations (≥ protocol=50) must yield protocol; got: {level}"
    );

    // Assertion 2: attestation CID is present.
    assert!(
        decision.decision_attestation_cid.is_some(),
        "E2E-R4: decision must carry decision_attestation_cid"
    );
}

// ─── E2E-R5 — At-tier-boundary (exactly 5 attestations → community edge_case) ─
//
// The `aggregate` step's threshold-ladder marks edge_case=true when count
// exactly equals the winning tier's minCount.  At count=5, community tier
// (minCount=5) is the winner and count==minCount → edge_case=true.
//
// The DAG routes: aggregate → (edge_case==true) → wisdom-edge → synthesize.
// In DevContext, wisdom-edge is mocked to Allow, which allows synthesize to run.
// synthesize reads reachData.level == "community" (already set by aggregate).
//
// The test confirms:
// (a) level == "community" (tier was met, wisdom-edge does not change the level)
// (b) decision is a Verdict, not a Decline (wisdom-edge mock did not veto)

#[tokio::test]
async fn e2e_r5_exactly_at_community_boundary_edge_case_yields_community() {
    // Exactly 5 attestations == community minCount → edge_case = true.
    setup_reach_gate_test(TEST_SUBJECT, build_attestations(TEST_SUBJECT, 5, "witness"));

    let event = reach_negotiation_event("test-requester", "r-5");
    let decision = check(event)
        .await
        .expect("check() must not fail for E2E-R5");

    // Assertion 1: level is still "community" — wisdom-edge ran but did not
    // alter reachData.level; synthesize reads the aggregation result.
    let level = extract_reach_level(&decision);
    assert_eq!(
        level, "community",
        "E2E-R5: 5 attestations (== community minCount=5) must yield community after \
         wisdom-edge; got: {level}"
    );

    // Assertion 2: the decision is a Verdict, not an Allow or Decline.
    // This proves wisdom-edge (mocked Allow in DevContext) continued to synthesize
    // rather than short-circuiting with an Allow or Decline.
    assert!(
        matches!(&decision.status, GateStatus::Verdict(_)),
        "E2E-R5: at-boundary result must be Verdict (wisdom-edge allow → synthesize); \
         got: {:?}",
        decision.status
    );

    // Assertion 3: attestation CID is present (gate ran fully).
    assert!(
        decision.decision_attestation_cid.is_some(),
        "E2E-R5: edge-case decision must carry decision_attestation_cid"
    );
}

// ─── E2E-R6 — Non-reach CapabilityInvoke falls through to universal-band only ─
//
// Only `capability == "reach-negotiation"` routes to reach-gate.
// Any other capability value (here: "content-safety-review") must return the
// universal-band Allow directly — no domain gate runs.
//
// Evidence: decision is Allow (not Verdict); phase_note == "wisdom-sourced" proves
// the real universal-band DAG ran (not a domain gate).

#[tokio::test]
async fn e2e_r6_non_reach_capability_invoke_falls_through_to_universal_band_only() {
    // No context or resolver injection needed — no domain gate should fire.
    let event = RelationalImpactEvent::CapabilityInvoke {
        capability: "content-safety-review".to_string(),
        requester: "test-requester".to_string(),
        request_id: "r-6".to_string(),
    };

    let decision = check(event)
        .await
        .expect("check() must not fail for E2E-R6");

    // Assertion 1: Allow (universal-band allows all in DevContext).
    assert!(
        decision.is_allowed(),
        "E2E-R6: non-reach CapabilityInvoke must return Allow (universal-band only); \
         got: {:?}",
        decision.status
    );

    // Assertion 2: NOT a Verdict — reach-gate must not have run.
    assert!(
        !matches!(&decision.status, GateStatus::Verdict(_)),
        "E2E-R6: non-reach-negotiation capability must NOT produce Verdict; \
         reach-gate must not have fired; got: {:?}",
        decision.status
    );

    // Assertion 3: NOT exempt — universal-band DAG ran (not the exempt short-circuit).
    assert!(
        !decision.is_exempt(),
        "E2E-R6: non-reach CapabilityInvoke must NOT be exempt (universal-band ran)"
    );

    // Assertion 4: phase_note == "wisdom-sourced" proves the real universal-band
    // synthesize step ran (not a stub or domain-gate path).
    assert_eq!(
        decision.reasoning.phase_note, "wisdom-sourced",
        "E2E-R6: phase_note must be 'wisdom-sourced' to prove universal-band ran; \
         got: {:?}",
        decision.reasoning.phase_note
    );
}

// ─── E2E-R7 — Override short-circuits before reach-gate runs ─────────────────
//
// `__test_set_decision_override` fires BEFORE universal-band AND before the
// reach-gate.  The returned decision is the injected Decline.

#[tokio::test]
async fn e2e_r7_override_short_circuits_before_reach_gate() {
    // Inject a Decline override.
    gate_client::__test_set_decision_override(Some(gate_client::testing::mock_decline(
        "reach-gate-override-test",
        "injected to prove reach-gate is skipped",
    )));

    let event = reach_negotiation_event("test-requester", "r-7");
    let decision = check(event.clone())
        .await
        .expect("check() must not fail for E2E-R7 override");

    // Assertion 1: the injected Decline was returned — not a Verdict.
    assert!(
        !decision.is_allowed(),
        "E2E-R7: override must return the injected Decline; got: {:?}",
        decision.status
    );

    // Assertion 2: the decision carries the injected category, proving it is
    // the exact override and not a Decline from a real reach-gate run.
    let status_json = serde_json::to_string(&decision.status).unwrap();
    assert!(
        status_json.contains("reach-gate-override-test"),
        "E2E-R7: injected Decline category must be in status JSON; got: {status_json}"
    );

    // Assertion 3: NOT a Verdict — reach-gate did not run.
    assert!(
        !matches!(&decision.status, GateStatus::Verdict(_)),
        "E2E-R7: override must prevent Verdict from reach-gate; got: {:?}",
        decision.status
    );

    // Assertion 4: override is one-shot — next call returns the real result.
    // Without injected context, reach-gate will run but find null subject →
    // the aggregate step will fail to read the subject key → the gate will error.
    // Instead, verify only that the override is cleared by checking a different event.
    let other_event = RelationalImpactEvent::CapabilityInvoke {
        capability: "translate".to_string(),
        requester: "test-requester".to_string(),
        request_id: "r-7-after".to_string(),
    };
    let second = check(other_event)
        .await
        .expect("second check() must not fail after override cleared");
    assert!(
        second.is_allowed(),
        "E2E-R7: after override consumed, check() must return Allow for non-reach capability; \
         got: {:?}",
        second.status
    );
}

// ─── Reach-level coverage: all four levels reachable ─────────────────────────
//
// Confirms that all four reach level strings are producible through the gate.
// Complementary to the individual per-level tests above — this test runs all
// four in a single parameterized table to catch any regression in the ladder.

#[tokio::test]
async fn all_four_reach_levels_are_reachable_through_check() {
    let cases: &[(&str, usize, &str)] = &[
        ("self-reach", 0, "endorsement"), // 0 < 5 → no tier met → self-reach
        ("community", 7, "witness"),      // 7 ≥ 5, < 20 → community
        ("public", 25, "gate-decision"),  // 25 ≥ 20, < 50 → public
        ("protocol", 55, "content-approval"), // 55 ≥ 50 → protocol
    ];

    for (expected_level, count, kind) in cases {
        let subject = format!("bafyreach-level-test-{expected_level}");
        setup_reach_gate_test(&subject, build_attestations(&subject, *count, kind));

        let event = RelationalImpactEvent::CapabilityInvoke {
            capability: "reach-negotiation".to_string(),
            requester: "test-requester".to_string(),
            request_id: format!("r-all-{expected_level}"),
        };

        let decision = check(event)
            .await
            .unwrap_or_else(|e| panic!("check() failed for {expected_level}: {e}"));

        let level = extract_reach_level(&decision);
        assert_eq!(
            level, *expected_level,
            "count={count} must yield level='{expected_level}'; got: {level}"
        );
    }
}

// ─── Mixed-kind attestations count toward the total ──────────────────────────
//
// The reach-aggregation spec lists four valid kinds. Records of any valid kind
// contribute to the count. This test uses a mix of all four kinds to confirm
// they all count.

#[tokio::test]
async fn mixed_attestation_kinds_all_count_toward_reach_level() {
    let subject = "bafyreach-mixed-kinds-test";
    // 3 × gate-decision + 3 × witness + 3 × endorsement + 3 × content-approval = 12 total
    // 12 ≥ community(5), < public(20) → community
    let mut records = vec![];
    records.extend(build_attestations(subject, 3, "gate-decision"));
    records.extend(build_attestations(subject, 3, "witness"));
    records.extend(build_attestations(subject, 3, "endorsement"));
    records.extend(build_attestations(subject, 3, "content-approval"));

    setup_reach_gate_test(subject, records);

    let event = RelationalImpactEvent::CapabilityInvoke {
        capability: "reach-negotiation".to_string(),
        requester: "test-requester".to_string(),
        request_id: "r-mixed-kinds".to_string(),
    };

    let decision = check(event)
        .await
        .expect("check() must not fail for mixed-kinds test");

    let level = extract_reach_level(&decision);
    assert_eq!(
        level, "community",
        "12 mixed-kind attestations (≥ community=5, < public=20) must yield community; \
         got: {level}"
    );
}
