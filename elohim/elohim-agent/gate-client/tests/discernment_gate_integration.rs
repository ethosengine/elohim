//! End-to-end integration tests for the discernment-gate v1 mechanical DAG.
//!
//! These tests exercise the full seven-valence rule set through the real
//! universal-band + discernment-gate execution path.  Each test:
//!
//! 1. Constructs a realistic `RelationalImpactEvent::AttestationWrite` fixture.
//! 2. Builds a pre-populated `GateContext` extension with `moment` and
//!    `priorAttestations` fields the rule conditions require.
//! 3. Calls `run_discernment_gate()` — which runs universal-band first, then
//!    the discernment-gate DAG.
//! 4. Asserts the returned `GateDecision` carries the expected Verdict valence,
//!    magnitude, and evidence_type, plus the expected side effects.
//!
//! # Phase 3 design choices (documented here per task description)
//!
//! - **Choice A (dispatch)**: `run_discernment_gate()` is a separate entry point.
//!   `check()` is unchanged and still runs universal-band only.  Phase 4 will
//!   fold `AttestationWrite` dispatch into `check()` via manifest-driven routing.
//!
//! - **Choice B (context injection)**: The pre-built `GateContext` extension is
//!   passed directly to `run_discernment_gate()`.  The discernment-gate's
//!   `assemble` step runs but its Phase 3 stubs return `Value::Null` for sources
//!   not yet wired.  The pre-populated keys in the extension OVERRIDE those nulls
//!   because `GateContext::merge()` is called AFTER the assemble step completes
//!   — wait, actually the extension is merged BEFORE the DAG runs so the
//!   `assemble` step can see them.  See `run_discernment_gate` in dag_runner.rs.
//!
//! - **Choice C (experience-moment detection)**: ALL `AttestationWrite` events
//!   route through the discernment-gate in Phase 3.  Phase 4 will restrict this
//!   to `experience-moment` content type via manifest-driven dispatch.
//!
//! # Rule-3 vs Rule-2 overlap nuance (spec §7.3)
//!
//! The deliberate ordering `[2a, 2b, 3]` means: a `@validates-failure-mode`
//! scenario that had a prior-PASSED attestation is classified as rule-2a/2b
//! (discovery/regression), NOT rule-3 (validation).  One test explicitly
//! documents this.

use gate_client::{run_discernment_gate, GateContext, GateStatus, GateTag, RelationalImpactEvent};
use serde_json::json;

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Build an `AttestationWrite` event for use in all discernment-gate tests.
fn attestation_event(subject_hash: &str, claim_kind: &str) -> RelationalImpactEvent {
    RelationalImpactEvent::AttestationWrite {
        subject_hash: subject_hash.to_string(),
        claim_kind: claim_kind.to_string(),
        issuer: "agent-discernment-test".to_string(),
    }
}

/// Build a `GateContext` extension pre-populated with `moment` and
/// `priorAttestations` fields needed by the discernment-gate rules.
fn ctx_with_moment_and_priors(
    moment: serde_json::Value,
    prior_attestations: serde_json::Value,
) -> GateContext {
    let mut ctx = GateContext::new();
    ctx.insert("moment", moment);
    ctx.insert("priorAttestations", prior_attestations);
    ctx
}

/// Extract the `GateTag::StoryPoint` fields from a Verdict decision.
/// Panics with a descriptive message if the status is not a StoryPoint Verdict.
fn extract_story_point(decision: &gate_client::GateDecision) -> (&str, &str, &str) {
    // We need to work with the status by value, so borrow it.
    match &decision.status {
        GateStatus::Verdict(GateTag::StoryPoint {
            valence,
            magnitude,
            evidence_type,
        }) => (valence.as_str(), magnitude.as_str(), evidence_type.as_str()),
        other => panic!("expected Verdict(StoryPoint {{..}}), got: {other:?}"),
    }
}

// ─── Rule 1 — first-pass green → progress/meaningful/first-pass-green ─────────

#[tokio::test]
async fn rule_1_first_pass_green_progress_verdict() {
    let event = attestation_event("moment-hash-r1", "experience-moment");
    let ctx_ext = ctx_with_moment_and_priors(
        json!({
            "status": "passed",
            "scenarioTagsContainsValidatesFailureMode": false
        }),
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

    let decision = run_discernment_gate(&event, ctx_ext)
        .await
        .expect("run_discernment_gate must not fail for rule-1");

    // Assertion 1: Verdict status.
    let (valence, magnitude, evidence_type) = extract_story_point(&decision);
    assert_eq!(valence, "progress", "rule-1 must emit progress valence");

    // Assertion 2: magnitude and evidence type.
    assert_eq!(
        magnitude, "meaningful",
        "rule-1 magnitude must be meaningful"
    );
    assert_eq!(evidence_type, "first-pass-green", "rule-1 evidence type");

    // Assertion 3: side effects include MintAttestation with StoryPointLink shape.
    let has_mint = decision.side_effects.iter().any(|se| {
        matches!(se, gate_client::SideEffect::MintAttestation { shape, .. }
            if shape == "StoryPointLink")
    });
    assert!(
        has_mint,
        "rule-1 decision must include MintAttestation with shape StoryPointLink; \
         got: {:?}",
        decision.side_effects
    );
}

// ─── Rule 2a — novel-failure-class → discovery/meaningful/novel-failure-class ─

#[tokio::test]
async fn rule_2a_novel_failure_class_discovery_verdict() {
    let event = attestation_event("moment-hash-r2a", "experience-moment");
    let ctx_ext = ctx_with_moment_and_priors(
        json!({
            "status": "failed",
            "scenarioTagsContainsValidatesFailureMode": false
        }),
        json!({
            "count": 1,
            "mostRecentStatus": "passed",
            "errorClassIsNew": true,
            "computeFingerprintIsNew": false,
            "hasRicherEvidence": false,
            "witnessEligible": false,
            "refinementEligible": false
        }),
    );

    let decision = run_discernment_gate(&event, ctx_ext)
        .await
        .expect("run_discernment_gate must not fail for rule-2a");

    // Assertion 1: Verdict with discovery valence.
    let (valence, magnitude, evidence_type) = extract_story_point(&decision);
    assert_eq!(valence, "discovery", "rule-2a must emit discovery valence");

    // Assertion 2: magnitude.
    assert_eq!(
        magnitude, "meaningful",
        "rule-2a magnitude must be meaningful"
    );

    // Assertion 3: evidence type.
    assert_eq!(
        evidence_type, "novel-failure-class",
        "rule-2a evidence type must be novel-failure-class"
    );
}

// ─── Rule 2b — known-cause-recurrence → regression/meaningful/known-cause ─────

#[tokio::test]
async fn rule_2b_known_cause_recurrence_regression_verdict() {
    let event = attestation_event("moment-hash-r2b", "experience-moment");
    let ctx_ext = ctx_with_moment_and_priors(
        json!({
            "status": "failed",
            "scenarioTagsContainsValidatesFailureMode": false
        }),
        json!({
            "count": 2,
            "mostRecentStatus": "passed",
            "errorClassIsNew": false,
            "computeFingerprintIsNew": false,
            "hasRicherEvidence": false,
            "witnessEligible": false,
            "refinementEligible": false
        }),
    );

    let decision = run_discernment_gate(&event, ctx_ext)
        .await
        .expect("run_discernment_gate must not fail for rule-2b");

    // Assertion 1: Verdict with regression valence.
    let (valence, magnitude, evidence_type) = extract_story_point(&decision);
    assert_eq!(
        valence, "regression",
        "rule-2b must emit regression valence"
    );

    // Assertion 2: magnitude.
    assert_eq!(
        magnitude, "meaningful",
        "rule-2b magnitude must be meaningful"
    );

    // Assertion 3: evidence type.
    assert_eq!(
        evidence_type, "known-cause-recurrence",
        "rule-2b evidence type must be known-cause-recurrence"
    );
}

// ─── Rule 3 — validates-failure-mode → validation/meaningful/failure-mode ─────
//
// Rule 3 fires when: status == failed AND scenarioTagsContainsValidatesFailureMode == true
// AND rules 2a/2b do NOT match (i.e., priorAttestations.mostRecentStatus != "passed").
// This means the scenario was always-failing — it's an expected negative test.

#[tokio::test]
async fn rule_3_validates_failure_mode_validation_verdict() {
    let event = attestation_event("moment-hash-r3", "experience-moment");
    let ctx_ext = ctx_with_moment_and_priors(
        json!({
            "status": "failed",
            "scenarioTagsContainsValidatesFailureMode": true
        }),
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

    let decision = run_discernment_gate(&event, ctx_ext)
        .await
        .expect("run_discernment_gate must not fail for rule-3");

    // Assertion 1: Verdict with validation valence.
    let (valence, magnitude, evidence_type) = extract_story_point(&decision);
    assert_eq!(valence, "validation", "rule-3 must emit validation valence");

    // Assertion 2: evidence type.
    assert_eq!(
        evidence_type, "failure-mode-confirmed",
        "rule-3 evidence type must be failure-mode-confirmed"
    );

    // Assertion 3: side effects include EmitEconomicEvent.
    let has_economic = decision
        .side_effects
        .iter()
        .any(|se| matches!(se, gate_client::SideEffect::EmitEconomicEvent { .. }));
    assert!(
        has_economic,
        "rule-3 decision must include EmitEconomicEvent side effect; got: {:?}",
        decision.side_effects
    );
}

// ─── Rule-3 vs Rule-2 overlap nuance (spec §7.3) ─────────────────────────────
//
// A @validates-failure-mode scenario that had a PRIOR-PASSED attestation routes
// to rule-2a/2b (discovery/regression), NOT to rule-3 (validation).
// This is deliberate ordering: rules are evaluated in [1, 2a, 2b, 3, ...] order.

#[tokio::test]
async fn rule_3_vs_rule_2_ordering_prior_passed_wins_as_discovery() {
    let event = attestation_event("moment-hash-r3-vs-r2", "experience-moment");
    // Scenario: @validates-failure-mode tag is present, but priorAttestations shows
    // that the story was previously passing (green-to-red flip).
    // Rule 2a fires BEFORE rule 3 and wins because errorClassIsNew is true.
    let ctx_ext = ctx_with_moment_and_priors(
        json!({
            "status": "failed",
            // This tag is set — but rule-3 won't fire because rule-2a fires first
            "scenarioTagsContainsValidatesFailureMode": true
        }),
        json!({
            "count": 1,
            "mostRecentStatus": "passed",  // green-to-red flip → rule-2a/2b territory
            "errorClassIsNew": true,
            "computeFingerprintIsNew": false,
            "hasRicherEvidence": false,
            "witnessEligible": false,
            "refinementEligible": false
        }),
    );

    let decision = run_discernment_gate(&event, ctx_ext)
        .await
        .expect("run_discernment_gate must not fail for rule-3-vs-rule-2 ordering test");

    // Assertion 1: rule-2a wins, not rule-3.
    let (valence, _, evidence_type) = extract_story_point(&decision);
    assert_eq!(
        valence, "discovery",
        "with prior-passed attestation, rule-2a must win over rule-3 \
         (spec §7.3 deliberate ordering); got valence: {valence}"
    );

    // Assertion 2: evidence type confirms rule-2a, not rule-3.
    assert_eq!(
        evidence_type, "novel-failure-class",
        "evidence_type must be novel-failure-class (rule-2a), not failure-mode-confirmed \
         (rule-3); got: {evidence_type}"
    );

    // Assertion 3: not validation valence.
    assert_ne!(
        valence, "validation",
        "validation valence must NOT fire when a prior-passed attestation exists \
         (rule-2/2a takes precedence per spec §7.3)"
    );
}

// ─── Rule 4 — recovery → progress/meaningful/recovery ─────────────────────────

#[tokio::test]
async fn rule_4_recovery_progress_verdict() {
    let event = attestation_event("moment-hash-r4", "experience-moment");
    let ctx_ext = ctx_with_moment_and_priors(
        json!({
            "status": "passed",
            "scenarioTagsContainsValidatesFailureMode": false
        }),
        json!({
            "count": 2,
            "mostRecentStatus": "failed",
            "errorClassIsNew": false,
            "computeFingerprintIsNew": false,
            "hasRicherEvidence": false,
            "witnessEligible": false,
            "refinementEligible": false
        }),
    );

    let decision = run_discernment_gate(&event, ctx_ext)
        .await
        .expect("run_discernment_gate must not fail for rule-4");

    // Assertion 1: Verdict with progress valence (recovery is forward movement).
    let (valence, magnitude, evidence_type) = extract_story_point(&decision);
    assert_eq!(valence, "progress", "rule-4 must emit progress valence");

    // Assertion 2: evidence type.
    assert_eq!(
        evidence_type, "recovery",
        "rule-4 evidence type must be recovery"
    );

    // Assertion 3: magnitude.
    assert_eq!(
        magnitude, "meaningful",
        "rule-4 magnitude must be meaningful"
    );
}

// ─── Rule 5 — witness → witness/meaningful/cross-fingerprint-attestation ───────
//
// witnessEligible = (moment.status == priorAttestations.mostRecentStatus) AND
//                   (priorAttestations.computeFingerprintIsNew == true)
// Precomputed by the context-assemble step; injected via context extension.

#[tokio::test]
async fn rule_5_witness_cross_fingerprint_verdict() {
    let event = attestation_event("moment-hash-r5", "experience-moment");
    let ctx_ext = ctx_with_moment_and_priors(
        json!({
            "status": "passed",
            "scenarioTagsContainsValidatesFailureMode": false
        }),
        json!({
            "count": 1,
            "mostRecentStatus": "passed",
            "errorClassIsNew": false,
            // computeFingerprintIsNew = true → witnessEligible would be true
            "computeFingerprintIsNew": true,
            "hasRicherEvidence": false,
            // Phase 3: this precomputed boolean is what the rule condition reads.
            "witnessEligible": true,
            "refinementEligible": false
        }),
    );

    let decision = run_discernment_gate(&event, ctx_ext)
        .await
        .expect("run_discernment_gate must not fail for rule-5");

    // Assertion 1: Verdict with witness valence.
    let (valence, magnitude, evidence_type) = extract_story_point(&decision);
    assert_eq!(valence, "witness", "rule-5 must emit witness valence");

    // Assertion 2: evidence type.
    assert_eq!(
        evidence_type, "cross-fingerprint-attestation",
        "rule-5 evidence type must be cross-fingerprint-attestation"
    );

    // Assertion 3: magnitude.
    assert_eq!(
        magnitude, "meaningful",
        "rule-5 magnitude must be meaningful"
    );
}

// ─── Rule 6 — refinement → refinement/small/evidence-enriched ─────────────────
//
// refinementEligible = (moment.status == priorAttestations.mostRecentStatus) AND
//                      (priorAttestations.hasRicherEvidence == true) AND
//                      (priorAttestations.computeFingerprintIsNew == false)
// Rule 6 fires only when rule 5 did NOT fire (witnessEligible == false).

#[tokio::test]
async fn rule_6_refinement_evidence_enriched_verdict() {
    let event = attestation_event("moment-hash-r6", "experience-moment");
    let ctx_ext = ctx_with_moment_and_priors(
        json!({
            "status": "passed",
            "scenarioTagsContainsValidatesFailureMode": false
        }),
        json!({
            "count": 2,
            "mostRecentStatus": "passed",
            "errorClassIsNew": false,
            "computeFingerprintIsNew": false,
            "hasRicherEvidence": true,
            // witnessEligible must be false so rule-5 does not fire first.
            "witnessEligible": false,
            "refinementEligible": true
        }),
    );

    let decision = run_discernment_gate(&event, ctx_ext)
        .await
        .expect("run_discernment_gate must not fail for rule-6");

    // Assertion 1: Verdict with refinement valence.
    let (valence, magnitude, evidence_type) = extract_story_point(&decision);
    assert_eq!(valence, "refinement", "rule-6 must emit refinement valence");

    // Assertion 2: evidence type.
    assert_eq!(
        evidence_type, "evidence-enriched",
        "rule-6 evidence type must be evidence-enriched"
    );

    // Assertion 3: magnitude is small (refinement is incremental, not a milestone).
    assert_eq!(
        magnitude, "small",
        "rule-6 magnitude must be small (incremental improvement)"
    );
}

// ─── Rule 7 — steady-state silence → Allow{exempt:true} ─────────────────────
//
// No rule matches → the executor writes ruleDecision = null → the DAG edge
// "ruleDecision == null" routes to terminal-no-mint → Allow{exempt:true}.

#[tokio::test]
async fn rule_7_steady_state_no_rule_matches_allow_exempt() {
    let event = attestation_event("moment-hash-r7", "experience-moment");
    // Status is passed, prior count > 0, prior status is passed, witnessEligible = false,
    // refinementEligible = false → no rule fires.
    let ctx_ext = ctx_with_moment_and_priors(
        json!({
            "status": "passed",
            "scenarioTagsContainsValidatesFailureMode": false
        }),
        json!({
            "count": 5,
            "mostRecentStatus": "passed",
            "errorClassIsNew": false,
            "computeFingerprintIsNew": false,
            "hasRicherEvidence": false,
            "witnessEligible": false,
            "refinementEligible": false
        }),
    );

    let decision = run_discernment_gate(&event, ctx_ext)
        .await
        .expect("run_discernment_gate must not fail for rule-7 steady-state");

    // Assertion 1: Allow with exempt = true (terminal-no-mint path).
    assert!(
        decision.is_allowed(),
        "steady-state must return Allow; got: {:?}",
        decision.status
    );
    assert!(
        decision.is_exempt(),
        "steady-state must return Allow{{exempt: true}} (terminal-no-mint); \
         got is_exempt: false, status: {:?}",
        decision.status
    );

    // Assertion 2: no side effects on the steady-state path.
    assert!(
        decision.side_effects.is_empty(),
        "terminal-no-mint must have no side effects; got: {:?}",
        decision.side_effects
    );

    // Assertion 3: NOT a Verdict (no StoryPoint minted).
    assert!(
        !matches!(decision.status, GateStatus::Verdict(_)),
        "steady-state must NOT be a Verdict; got: {:?}",
        decision.status
    );
}

// ─── Override mechanism short-circuits before both DAGs ───────────────────────
//
// Injecting a `__test_set_decision_override` must prevent the universal-band
// AND discernment-gate from running.  The override fires inside `check()` which
// is called before `run_discernment_gate()` — but `run_discernment_gate` calls
// `run_with_tracing` (not `check()`), so the override does NOT fire there.
//
// However, the `__test_set_decision_override` is wired into `check()` in lib.rs.
// `run_discernment_gate` bypasses it by calling `run_with_tracing` directly.
//
// This test documents that the `check()` override fires as expected, and that
// users who want to skip both layers should use the override with `check()`.

#[tokio::test]
async fn check_override_short_circuits_before_dag() {
    gate_client::__test_set_decision_override(Some(gate_client::testing::mock_decline(
        "discernment-test-override",
        "override injected to prove DAG is skipped",
    )));

    let event = attestation_event("moment-hash-override", "experience-moment");

    let decision = gate_client::check(event)
        .await
        .expect("check() must not fail");

    // Assertion 1: the injected Decline was returned.
    assert!(
        !decision.is_allowed(),
        "override must return the injected Decline; got: {:?}",
        decision.status
    );

    // Assertion 2: the decision carries the injected category.
    let json = serde_json::to_string(&decision.status).unwrap();
    assert!(
        json.contains("discernment-test-override"),
        "injected Decline category must be present in status JSON; got: {json}"
    );

    // Assertion 3: override is one-shot — next check() returns Allow.
    let event2 = attestation_event("moment-hash-after-override", "experience-moment");
    let second = gate_client::check(event2)
        .await
        .expect("second check() must not fail");
    assert!(
        second.is_allowed(),
        "after override consumed, check() must return Allow; got: {:?}",
        second.status
    );
}

// ─── MintAttestation shape is "StoryPointLink" for all Verdict rules ──────────
//
// Every rule that produces a Verdict must emit MintAttestation with shape
// "StoryPointLink" (declared in the discernment-gate YAML).

#[tokio::test]
async fn all_verdict_rules_emit_mint_attestation_with_story_point_link_shape() {
    let cases = vec![
        // (name, moment, prior_attestations)
        (
            "rule-1",
            json!({"status": "passed", "scenarioTagsContainsValidatesFailureMode": false}),
            json!({"count": 0, "mostRecentStatus": null, "errorClassIsNew": false,
                   "computeFingerprintIsNew": false, "hasRicherEvidence": false,
                   "witnessEligible": false, "refinementEligible": false}),
        ),
        (
            "rule-2a",
            json!({"status": "failed", "scenarioTagsContainsValidatesFailureMode": false}),
            json!({"count": 1, "mostRecentStatus": "passed", "errorClassIsNew": true,
                   "computeFingerprintIsNew": false, "hasRicherEvidence": false,
                   "witnessEligible": false, "refinementEligible": false}),
        ),
        (
            "rule-4",
            json!({"status": "passed", "scenarioTagsContainsValidatesFailureMode": false}),
            json!({"count": 1, "mostRecentStatus": "failed", "errorClassIsNew": false,
                   "computeFingerprintIsNew": false, "hasRicherEvidence": false,
                   "witnessEligible": false, "refinementEligible": false}),
        ),
    ];

    for (rule_name, moment, priors) in cases {
        let event = attestation_event(&format!("hash-{rule_name}"), "experience-moment");
        let ctx_ext = ctx_with_moment_and_priors(moment, priors);

        let decision = run_discernment_gate(&event, ctx_ext)
            .await
            .unwrap_or_else(|e| panic!("{rule_name}: run_discernment_gate failed: {e}"));

        // Must be a Verdict.
        assert!(
            matches!(&decision.status, GateStatus::Verdict(_)),
            "{rule_name}: expected Verdict, got: {:?}",
            decision.status
        );

        // MintAttestation with StoryPointLink shape must be present.
        let has_story_point_mint = decision.side_effects.iter().any(|se| {
            matches!(se, gate_client::SideEffect::MintAttestation { shape, .. }
                if shape == "StoryPointLink")
        });
        assert!(
            has_story_point_mint,
            "{rule_name}: MintAttestation with shape StoryPointLink must be present; \
             side effects: {:?}",
            decision.side_effects
        );

        // EmitEconomicEvent must also be present.
        let has_economic = decision
            .side_effects
            .iter()
            .any(|se| matches!(se, gate_client::SideEffect::EmitEconomicEvent { .. }));
        assert!(
            has_economic,
            "{rule_name}: EmitEconomicEvent must be present; side effects: {:?}",
            decision.side_effects
        );
    }
}
