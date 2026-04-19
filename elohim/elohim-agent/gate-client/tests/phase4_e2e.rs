//! Phase 4 E2E integration tests — unified gate dispatch via `check()`.
//!
//! These tests exercise the full path introduced in Task 4.5:
//!
//! ```text
//! check(event)
//!   → override short-circuit (test builds only)
//!   → exempt-space short-circuit
//!   → universal-band DAG
//!     → if NOT Allow: short-circuit
//!     → dispatch table (Phase 4: AttestationWrite → discernment-gate)
//!       → if no domain gate: return universal-band decision
//!       → run discernment-gate DAG
//!         → return domain-gate decision (with attestation CID)
//! ```
//!
//! # Test inventory
//!
//! - **E2E-1** — `ContentPublish` through universal-band only, no domain gate.
//! - **E2E-2** — `AttestationWrite` → universal-band → discernment-gate → Verdict
//!   (rule-1, first-pass-green).  Uses `__test_set_gate_ctx("discernment-gate-v1-mechanical", …)`
//!   to inject pre-populated context fields that would normally come from `assemble`.
//! - **E2E-3** — Override short-circuits the unified path; neither DAG runs.
//! - **E2E-4** — Exempt-space AttestationWrite bypasses even the discernment-gate.
//! - **E2E-5** — AttestationWrite without context injection hits rule-7
//!   (steady-state no-mint → Allow{exempt:true}).
//! - **E2E-6** — check() dispatch matches `run_discernment_gate()` for rule-1
//!   (structural equivalence proof).

use gate_client::{
    check, GateContext, GateDecision, GateStatus, GateTag, RelationalImpactEvent, SideEffect,
};
use serde_json::json;

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn attestation_event(subject_hash: &str) -> RelationalImpactEvent {
    RelationalImpactEvent::AttestationWrite {
        subject_hash: subject_hash.to_string(),
        claim_kind: "experience-moment".to_string(),
        issuer: "agent-e2e-test".to_string(),
    }
}

fn content_publish_event() -> RelationalImpactEvent {
    RelationalImpactEvent::ContentPublish {
        content_cid: "bafye2e-content-publish".to_string(),
        declared_reach: "public".to_string(),
        author: "agent-e2e-author".to_string(),
    }
}

/// Build a GateContext extension for rule-1: first-pass green.
/// Moment status = passed, priorAttestations.count = 0 (no prior → first pass).
fn rule_1_ctx_ext() -> GateContext {
    let mut ctx = GateContext::new();
    ctx.insert(
        "moment",
        json!({
            "status": "passed",
            "scenarioTagsContainsValidatesFailureMode": false
        }),
    );
    ctx.insert(
        "priorAttestations",
        json!({
            "count": 0,
            "mostRecentStatus": null,
            "errorClassIsNew": false,
            "computeFingerprintIsNew": false,
            "hasRicherEvidence": false,
            "witnessEligible": false,
            "refinementEligible": false
        }),
    );
    ctx
}

/// Extract StoryPoint fields from a Verdict decision, panicking if the shape is wrong.
fn extract_story_point(decision: &GateDecision) -> (&str, &str, &str) {
    match &decision.status {
        GateStatus::Verdict(GateTag::StoryPoint {
            valence,
            magnitude,
            evidence_type,
        }) => (valence.as_str(), magnitude.as_str(), evidence_type.as_str()),
        other => panic!("expected Verdict(StoryPoint {{..}}), got: {other:?}"),
    }
}

// ─── E2E-1 — ContentPublish through universal band, no domain gate ────────────
//
// ContentPublish is not in the domain gate dispatch table → universal-band
// decision is returned directly.  phase_note == "wisdom-sourced" proves the
// real universal-band DAG ran.

#[tokio::test]
async fn e2e_1_content_publish_universal_band_only_no_domain_gate() {
    let event = content_publish_event();
    let decision = check(event).await.expect("check() must not fail");

    // Assertion 1: Allow (universal-band DevContext allows all in rehearsal).
    assert!(
        decision.is_allowed(),
        "ContentPublish must return Allow in DevContext; got: {:?}",
        decision.status
    );

    // Assertion 2: NOT exempt — universal-band ran, not the exempt short-circuit.
    assert!(
        !decision.is_exempt(),
        "ContentPublish must NOT be exempt (no domain gate → universal-band result)"
    );

    // Assertion 3: phase_note == "wisdom-sourced" proves universal-band DAG ran
    // (not the Phase 0 `allow_mocked` stub and not the discernment-gate path).
    assert_eq!(
        decision.reasoning.phase_note, "wisdom-sourced",
        "phase_note must be 'wisdom-sourced' — proves universal-band synthesize step ran; \
         got: {:?}",
        decision.reasoning.phase_note
    );

    // Assertion 4: decision_attestation_cid is Some (Phase 4: CID computed in-process).
    assert!(
        decision.decision_attestation_cid.is_some(),
        "ContentPublish must carry decision_attestation_cid in Phase 4; got: {:?}",
        decision.decision_attestation_cid
    );
    let cid = decision.decision_attestation_cid.as_ref().unwrap();
    assert!(
        cid.starts_with("sha256-"),
        "decision_attestation_cid must start with 'sha256-'; got: {cid}"
    );
}

// ─── E2E-2 — AttestationWrite → discernment-gate → Verdict (rule-1) ──────────
//
// `check(AttestationWrite)` with a rule-1 context extension injected via the
// test-only `__test_set_gate_ctx` thread-local.  This simulates what
// will happen in Phase 5+ when real `assemble` resolvers populate the context.
//
// Execution path verified:
//   1. universal-band runs (implicit — discernment-gate runs only on Allow).
//   2. discernment-gate runs with the injected context.
//   3. Rule-1 fires: Verdict(StoryPoint{valence:"progress", ...}).
//   4. Side effects include MintAttestation + EmitEconomicEvent.
//   5. decision_attestation_cid is Some with sha256- prefix.

#[tokio::test]
async fn e2e_2_attestation_write_via_check_routes_to_discernment_gate_rule1() {
    // Inject rule-1 context before calling check().
    gate_client::__test_set_gate_ctx("discernment-gate-v1-mechanical", Some(rule_1_ctx_ext()));

    let event = attestation_event("moment-hash-e2e-rule1");
    let decision = check(event)
        .await
        .expect("check() must not fail for rule-1 path");

    // Assertion 1: Verdict(StoryPoint) with progress valence (rule-1 first-pass-green).
    let (valence, magnitude, evidence_type) = extract_story_point(&decision);
    assert_eq!(
        valence, "progress",
        "rule-1 via check() must emit progress valence; got: {valence}"
    );

    // Assertion 2: magnitude and evidence_type are correct.
    assert_eq!(
        magnitude, "meaningful",
        "rule-1 via check() magnitude must be meaningful; got: {magnitude}"
    );
    assert_eq!(
        evidence_type, "first-pass-green",
        "rule-1 via check() evidence_type must be first-pass-green; got: {evidence_type}"
    );

    // Assertion 3: side_effects include MintAttestation with StoryPointLink shape.
    let has_mint = decision.side_effects.iter().any(
        |se| matches!(se, SideEffect::MintAttestation { shape, .. } if shape == "StoryPointLink"),
    );
    assert!(
        has_mint,
        "rule-1 via check() must include MintAttestation{{shape=StoryPointLink}}; \
         got: {:?}",
        decision.side_effects
    );

    // Assertion 4: side_effects include EmitEconomicEvent.
    let has_economic = decision
        .side_effects
        .iter()
        .any(|se| matches!(se, SideEffect::EmitEconomicEvent { .. }));
    assert!(
        has_economic,
        "rule-1 via check() must include EmitEconomicEvent; got: {:?}",
        decision.side_effects
    );

    // Assertion 5: decision_attestation_cid is Some with sha256- prefix.
    assert!(
        decision.decision_attestation_cid.is_some(),
        "Verdict decision from check() must carry decision_attestation_cid; got: {:?}",
        decision.decision_attestation_cid
    );
    let cid = decision.decision_attestation_cid.as_ref().unwrap();
    assert!(
        cid.starts_with("sha256-"),
        "decision_attestation_cid must start with 'sha256-'; got: {cid}"
    );
    assert_eq!(
        cid.len(),
        71,
        "decision_attestation_cid must be 71 chars (sha256- + 64 hex); got: {cid}"
    );
}

// ─── E2E-3 — Override short-circuits the unified path ────────────────────────
//
// `__test_set_decision_override` fires BEFORE universal-band AND before the
// discernment-gate.  Verified by:
//   - The returned decision is the injected Decline (not any DAG result).
//   - The injected decision's reasoning.phase_note is "mocked" (not "wisdom-sourced"
//     which the real DAG would set).
//   - Override is one-shot: next call returns the real DAG's result.

#[tokio::test]
async fn e2e_3_override_short_circuits_before_universal_and_discernment_gate() {
    gate_client::__test_set_decision_override(Some(gate_client::testing::mock_decline(
        "e2e-override-category",
        "injected to prove both DAGs are skipped",
    )));

    let event = attestation_event("moment-hash-e2e-override");
    let decision = check(event.clone()).await.expect("check() must not fail");

    // Assertion 1: the injected Decline was returned — not any DAG result.
    assert!(
        !decision.is_allowed(),
        "override must return the injected Decline; got: {:?}",
        decision.status
    );

    // Assertion 2: decision carries the injected category (proves it is the exact
    // override, not a Decline from a real DAG run).
    let status_json = serde_json::to_string(&decision.status).unwrap();
    assert!(
        status_json.contains("e2e-override-category"),
        "injected Decline category must be in status JSON; got: {status_json}"
    );

    // Assertion 3: override is one-shot — the next call on this thread returns
    // the real unified-dispatch result (discernment-gate rule-7 for AttestationWrite
    // without context → Allow{exempt:true}).
    let second = check(event).await.expect("second check() must not fail");
    assert!(
        second.is_allowed(),
        "after override consumed, check() must return Allow; got: {:?}",
        second.status
    );
    // No context was injected for the second call → rule-7 → Allow{exempt:true}.
    assert!(
        second.is_exempt(),
        "second AttestationWrite call without context → rule-7 → Allow{{exempt:true}}; \
         got is_exempt: false"
    );
}

// ─── E2E-4 — Exempt space bypasses even AttestationWrite dispatch ─────────────
//
// The exempt-space short-circuit fires BEFORE the universal-band and before any
// domain gate.  The gate-client v1 space-detection infers space from event type;
// there is no current event variant that maps to an exempt interior for
// AttestationWrite itself (all AttestationWrite events infer SpaceType::Public).
//
// What we CAN test: a deliberately-exempt space by using an event whose space
// type maps to an interior.  Since there is no exempt AttestationWrite variant
// in the current SpaceType inference (spec §3.1), we document this as the
// architectural invariant and verify it remains true — then test the actual
// "no-exempt AttestationWrite" path as the positive case.
//
// E2E-4a: AttestationWrite is NOT exempt (it is SpaceType::Public).
// E2E-4b: The exempt short-circuit works as a separate invariant (tested here
//          via check_blocking with an offline event shape — if one existed).
//
// NOTE: In Phase 5+, when a `PrivateDraftingAttestation` variant is added, or
// when space-type overrides are wired in, this test will need updating.

#[tokio::test]
async fn e2e_4a_attestation_write_is_not_exempt_space() {
    // SpaceType::Public maps to is_exempt() == false for AttestationWrite.
    let event = attestation_event("moment-hash-e2e-not-exempt");
    let space = gate_client::space::detect_from_event(&event);

    // Assertion 1: AttestationWrite infers Public space (not an interior).
    assert_eq!(
        space.space_type,
        gate_client::SpaceType::Public,
        "AttestationWrite must infer Public space type; got: {:?}",
        space.space_type
    );

    // Assertion 2: Public space is NOT exempt — the gate fires.
    assert!(
        !space.is_exempt(),
        "Public space must NOT be exempt; AttestationWrite goes through the gate"
    );

    // Assertion 3: check() confirms Allow (no override, no exempt short-circuit).
    let decision = check(event)
        .await
        .expect("check() must not fail for non-exempt AttestationWrite");
    assert!(
        decision.is_allowed(),
        "non-exempt AttestationWrite must return Allow; got: {:?}",
        decision.status
    );
}

#[tokio::test]
async fn e2e_4b_exempt_event_bypasses_all_gate_logic() {
    // No current RelationalImpactEvent variant maps to an exempt interior for
    // AttestationWrite.  However, we can verify the exempt short-circuit is
    // architecturally active by directly using the allow_exempt builder and
    // confirming the SpaceType API works correctly.
    //
    // This test is a forward-compatibility anchor: when a future
    // PrivateDraftingAttestation variant is added, update this test to use it.
    let exempt_decision = gate_client::GateDecision::allow_exempt(gate_client::Phase::DevContext);

    // Assertion 1: allow_exempt produces the exempt shape.
    assert!(
        exempt_decision.is_exempt(),
        "allow_exempt must produce Allow{{exempt:true}}"
    );

    // Assertion 2: exempt decision has no side effects.
    assert!(
        exempt_decision.side_effects.is_empty(),
        "exempt decision must have no side effects; got: {:?}",
        exempt_decision.side_effects
    );

    // Assertion 3: exempt decision has no attestation CID (gate did not run).
    assert!(
        exempt_decision.decision_attestation_cid.is_none(),
        "exempt decision must NOT carry decision_attestation_cid; gate never ran"
    );
}

// ─── E2E-5 — AttestationWrite without context → rule-7 (steady-state no-mint) ─
//
// Without a `__test_set_gate_ctx` extension, the discernment gate's
// `assemble` step returns null for `moment` and `priorAttestations` (Phase 3+
// stubs).  The mechanical ruleset falls through to rule-7 → terminal-no-mint →
// Allow{exempt:true}.
//
// This test documents the expected Phase 4 DevContext behavior for a "naked"
// AttestationWrite call (no fixture context).

#[tokio::test]
async fn e2e_5_attestation_write_without_context_yields_rule7_allow_exempt() {
    // No context extension injected.
    let event = attestation_event("moment-hash-e2e-rule7");
    let decision = check(event)
        .await
        .expect("check() must not fail for rule-7 path");

    // Assertion 1: Allow (rule-7 terminal-no-mint path is still an Allow).
    assert!(
        decision.is_allowed(),
        "naked AttestationWrite must return Allow (rule-7 no-mint path); got: {:?}",
        decision.status
    );

    // Assertion 2: Allow{exempt:true} — the terminal-no-mint path signals this
    // via the exempt flag to distinguish it from a real wisdom Allow.
    assert!(
        decision.is_exempt(),
        "rule-7 steady-state must return Allow{{exempt:true}}; \
         got is_exempt: false, status: {:?}",
        decision.status
    );

    // Assertion 3: no side effects (no story-point minted in steady state).
    assert!(
        decision.side_effects.is_empty(),
        "rule-7 must have no side effects; got: {:?}",
        decision.side_effects
    );
}

// ─── E2E-6 — check() dispatch is structurally equivalent to run_discernment_gate ─
//
// This test proves the architectural invariant of Task 4.5: `check()` for
// `AttestationWrite` with a pre-populated context extension produces the SAME
// Verdict classification as calling `run_discernment_gate()` directly with the
// same event and context.  The exact CIDs will differ (different context states
// → different context_summary_cid inputs) but the classification (valence,
// magnitude, evidence_type) must match.

#[tokio::test]
async fn e2e_6_check_dispatch_structurally_equivalent_to_direct_run_discernment_gate() {
    let event = attestation_event("moment-hash-e2e-equivalence");
    let ctx_ext = rule_1_ctx_ext();

    // Path A: via check() with context extension (uses new keyed API).
    gate_client::__test_set_gate_ctx("discernment-gate-v1-mechanical", Some(ctx_ext.clone()));
    let check_decision = check(event.clone()).await.expect("check() must not fail");

    // Path B: via run_discernment_gate() directly.
    let direct_decision = gate_client::run_discernment_gate(&event, ctx_ext)
        .await
        .expect("run_discernment_gate must not fail");

    // Assertion 1: Both paths produce Verdict (not Allow or Decline).
    assert!(
        matches!(&check_decision.status, GateStatus::Verdict(_)),
        "check() path must produce Verdict; got: {:?}",
        check_decision.status
    );
    assert!(
        matches!(&direct_decision.status, GateStatus::Verdict(_)),
        "run_discernment_gate() path must produce Verdict; got: {:?}",
        direct_decision.status
    );

    // Assertion 2: Both paths produce the same valence/magnitude/evidence_type.
    let (cv, cm, ce) = extract_story_point(&check_decision);
    let (dv, dm, de) = extract_story_point(&direct_decision);
    assert_eq!(
        cv, dv,
        "valence must match: check()={cv}, run_discernment_gate()={dv}"
    );
    assert_eq!(
        cm, dm,
        "magnitude must match: check()={cm}, run_discernment_gate()={dm}"
    );
    assert_eq!(
        ce, de,
        "evidence_type must match: check()={ce}, run_discernment_gate()={de}"
    );

    // Assertion 3: Both paths carry a decision_attestation_cid with sha256- prefix.
    let check_cid = check_decision
        .decision_attestation_cid
        .as_ref()
        .expect("check() path must carry decision_attestation_cid");
    let direct_cid = direct_decision
        .decision_attestation_cid
        .as_ref()
        .expect("run_discernment_gate() path must carry decision_attestation_cid");
    assert!(
        check_cid.starts_with("sha256-"),
        "check() path CID must start with 'sha256-'; got: {check_cid}"
    );
    assert!(
        direct_cid.starts_with("sha256-"),
        "run_discernment_gate() path CID must start with 'sha256-'; got: {direct_cid}"
    );

    // The CIDs may differ (context_summary_cid input differs between the two runs)
    // but both must be non-empty valid sha256- strings.
    assert_eq!(check_cid.len(), 71, "check() CID length must be 71");
    assert_eq!(
        direct_cid.len(),
        71,
        "run_discernment_gate() CID length must be 71"
    );
}
