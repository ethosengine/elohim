//! Phase 6 Task 6.4 — End-to-end integration tests for the content-safety-gate.
//!
//! These tests exercise the full multi-gate pipeline introduced in Task 6.2:
//!
//! ```text
//! check(event)
//!   → override short-circuit (test builds only)
//!   → universal-band DAG
//!     → content-safety-gate-v1 (all three event kinds)
//!       → discernment-gate-v1-mechanical (AttestationWrite only, after safety)
//! ```
//!
//! # Test inventory
//!
//! - **E2E-CS-1** — `ContentPublish` through content-safety-gate → Allow + CID.
//! - **E2E-CS-2** — `PeerMessage` through content-safety-gate → Allow + CID.
//! - **E2E-CS-3** — Content-safety Decline short-circuits discernment for
//!   `AttestationWrite`.  Uses `__test_set_wisdom_output` (Phase 6 Task 6.4
//!   injection mechanism) to force `safetyOutput: {decision: "decline"}`.
//! - **E2E-CS-4** — `AttestationWrite`: content-safety Allow → discernment Verdict
//!   (both run; discernment Verdict wins as the last-gate decision).
//! - **E2E-CS-5** — `__test_set_decision_override` short-circuits the entire
//!   multi-gate pipeline before any gate runs.
//! - **E2E-CS-6** — `CapabilityInvoke{capability:"reach-negotiation"}` still
//!   routes through reach-gate only (regression: Vec-shaped dispatcher did not
//!   break reach-gate dispatch).
//!
//! # E2E-CS-3: Forced Decline mechanism
//!
//! The `WisdomInvokeExecutor` always writes `{"decision":"allow"}` in DevContext.
//! Pre-injecting `safetyOutput` via `__test_set_gate_ctx` does not help because
//! the wisdom-safety step runs after assemble and OVERWRITES whatever was there.
//!
//! Solution: `__test_set_wisdom_output(constitution_cid, output_key, value)` —
//! a thread-local that the executor reads after writing its mock output, replacing
//! it.  The constitution CID for content-safety-gate is
//! `"epr:constitutions:content-safety-v1"` (from `content_safety_gate_v1.yaml`).
//!
//! This mechanism is Phase 6+ scope only; it is compiled out of production builds
//! (requires `#[cfg(any(test, feature = "testing"))]`).

use std::sync::Arc;

use gate_client::{
    check, AttestationRecord, GateContext, GateStatus, GateTag, RelationalImpactEvent,
    StubAttestationResolver,
};
use serde_json::json;

// ─── Fixture constants ─────────────────────────────────────────────────────────

/// The `constitution_cid` of the `wisdom-safety` step in content-safety-gate-v1.yaml.
/// Used to key the `__test_set_wisdom_output` override.
const CONTENT_SAFETY_CONSTITUTION_CID: &str = "epr:constitutions:content-safety-v1";

/// The `output_key` of the `wisdom-safety` step.
const SAFETY_OUTPUT_KEY: &str = "safetyOutput";

// ─── Event constructors ────────────────────────────────────────────────────────

fn content_publish_event(cid: &str) -> RelationalImpactEvent {
    RelationalImpactEvent::ContentPublish {
        content_cid: cid.to_string(),
        declared_reach: "community".to_string(),
        author: "agent-cs-test".to_string(),
    }
}

fn peer_message_event(recipient: &str) -> RelationalImpactEvent {
    RelationalImpactEvent::PeerMessage {
        recipient: recipient.to_string(),
        payload_kind: "handshake".to_string(),
    }
}

fn attestation_write_event(subject_hash: &str) -> RelationalImpactEvent {
    RelationalImpactEvent::AttestationWrite {
        subject_hash: subject_hash.to_string(),
        claim_kind: "experience-moment".to_string(),
        issuer: "agent-cs-discernment".to_string(),
    }
}

/// Build the discernment-gate context extension for rule-1 (first-pass green).
///
/// Injected via `__test_set_gate_ctx("discernment-gate-v1-mechanical", ...)` so
/// the discernment-gate DAG runs the correct rule without a real `assemble` step.
fn rule_1_discernment_ctx() -> GateContext {
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

// ─── E2E-CS-1 — ContentPublish dev-context Allow + CID ────────────────────────
//
// ContentPublish routes through content-safety-gate-v1 only.
// In DevContext, content-safety always returns Allow (wisdom mock).
// The decision must carry:
//   - Allow { exempt: false }
//   - phase_note == "wisdom-sourced" (from allow-or-decline-from-wisdom builder)
//   - decision_attestation_cid.is_some() with sha256- prefix

#[tokio::test]
async fn e2e_cs1_content_publish_allow_with_cid() {
    let event = content_publish_event("bafycs1-content-cid");
    let decision = check(event)
        .await
        .expect("E2E-CS-1: check() must not fail for ContentPublish");

    // Assertion 1: Allow (content-safety mock returns Allow in DevContext).
    assert!(
        decision.is_allowed(),
        "E2E-CS-1: ContentPublish must return Allow through content-safety-gate in DevContext; \
         got: {:?}",
        decision.status
    );

    // Assertion 2: not exempt (content-safety synthesize emits Allow{exempt:false}).
    assert!(
        !decision.is_exempt(),
        "E2E-CS-1: content-safety Allow must NOT be exempt (allow-or-decline-from-wisdom \
         emits Allow{{exempt:false}}); got is_exempt: true, status: {:?}",
        decision.status
    );

    // Assertion 3: phase_note == "wisdom-sourced" proves the real DAG ran.
    assert_eq!(
        decision.reasoning.phase_note, "wisdom-sourced",
        "E2E-CS-1: phase_note must be 'wisdom-sourced' (content-safety-gate allow-or-decline \
         builder propagates wisdom reasoning); got: {:?}",
        decision.reasoning.phase_note
    );

    // Assertion 4: attestation CID is Some.
    assert!(
        decision.decision_attestation_cid.is_some(),
        "E2E-CS-1: ContentPublish decision must carry decision_attestation_cid; got: None"
    );

    // Assertion 5: CID starts with sha256-.
    let cid = decision.decision_attestation_cid.as_ref().unwrap();
    assert!(
        cid.starts_with("sha256-"),
        "E2E-CS-1: decision_attestation_cid must start with 'sha256-'; got: {cid}"
    );
}

// ─── E2E-CS-2 — PeerMessage through content-safety-gate ───────────────────────
//
// PeerMessage routes through content-safety-gate-v1 only (same dispatch as
// ContentPublish).  The path, semantics, and CID contract are identical.

#[tokio::test]
async fn e2e_cs2_peer_message_allow_with_cid() {
    let event = peer_message_event("agent-peer-cs2");
    let decision = check(event)
        .await
        .expect("E2E-CS-2: check() must not fail for PeerMessage");

    // Assertion 1: Allow.
    assert!(
        decision.is_allowed(),
        "E2E-CS-2: PeerMessage must return Allow through content-safety-gate; got: {:?}",
        decision.status
    );

    // Assertion 2: not exempt.
    assert!(
        !decision.is_exempt(),
        "E2E-CS-2: PeerMessage Allow must NOT be exempt; got is_exempt: true"
    );

    // Assertion 3: phase_note == "wisdom-sourced" — real DAG ran.
    assert_eq!(
        decision.reasoning.phase_note, "wisdom-sourced",
        "E2E-CS-2: phase_note must be 'wisdom-sourced'; got: {:?}",
        decision.reasoning.phase_note
    );

    // Assertion 4: CID is Some with sha256- prefix.
    assert!(
        decision.decision_attestation_cid.is_some(),
        "E2E-CS-2: PeerMessage decision must carry decision_attestation_cid"
    );
    let cid = decision.decision_attestation_cid.as_ref().unwrap();
    assert!(
        cid.starts_with("sha256-"),
        "E2E-CS-2: decision_attestation_cid must start with 'sha256-'; got: {cid}"
    );
}

// ─── E2E-CS-3 — Content-safety Decline short-circuits discernment ─────────────
//
// AttestationWrite routes through [content-safety-gate-v1, discernment-gate-v1-mechanical].
// When content-safety Declines, the multi-gate loop MUST short-circuit and the
// discernment-gate must NOT run.
//
// Mechanism: `__test_set_wisdom_output` injects `{"decision":"decline",...}` into
// the `safetyOutput` context key AFTER the `WisdomInvokeExecutor` writes its
// mock Allow, overriding it.  The `allow-or-decline-from-wisdom` builder then sees
// the decline and emits `GateStatus::Decline{category:"content-safety"}`.
//
// The content-safety gate Decline short-circuits `run_unified`, which drops all
// prior side effects and returns the Decline directly — discernment never runs.
//
// Evidence that discernment did NOT run:
// - The decision is a Decline (not a Verdict, which discernment would produce).
// - The category is "content-safety" (not a discernment valence).

#[tokio::test]
async fn e2e_cs3_content_safety_decline_short_circuits_discernment() {
    // Inject decline into content-safety wisdom output.
    // constitution_cid matches the `wisdom-safety` step in content_safety_gate_v1.yaml.
    gate_client::__test_set_wisdom_output(
        CONTENT_SAFETY_CONSTITUTION_CID,
        SAFETY_OUTPUT_KEY,
        json!({
            "decision": "decline",
            "reasoning": {
                "primary_principle": "content-safety-mock-decline",
                "confidence": 0.0,
                "summary": "test-forced-decline: content-safety DAG wired to Decline for E2E-CS-3"
            },
            "phase": "dev-context"
        }),
    );

    let event = attestation_write_event("moment-hash-cs3");
    let decision = check(event)
        .await
        .expect("E2E-CS-3: check() must not fail even when content-safety Declines");

    // Assertion 1: the decision is a Decline — not Allow, not Verdict.
    assert!(
        !decision.is_allowed(),
        "E2E-CS-3: forced content-safety Decline must NOT return Allow; got: {:?}",
        decision.status
    );
    assert!(
        !matches!(&decision.status, GateStatus::Verdict(_)),
        "E2E-CS-3: discernment must NOT have run (would produce Verdict); \
         content-safety Decline must short-circuit; got: {:?}",
        decision.status
    );

    // Assertion 2: the Decline category is "content-safety", not a discernment valence.
    match &decision.status {
        GateStatus::Decline { grounds } => {
            assert_eq!(
                grounds.category, "content-safety",
                "E2E-CS-3: Decline category must be 'content-safety'; got: {:?}",
                grounds.category
            );
            assert!(
                grounds.summary.contains("test-forced-decline"),
                "E2E-CS-3: Decline summary must carry the injected reason; got: {:?}",
                grounds.summary
            );
        }
        other => {
            panic!("E2E-CS-3: expected Decline{{category:\"content-safety\"}}, got: {other:?}")
        }
    }

    // Assertion 3: no side effects — Decline discards any accumulated side effects.
    assert!(
        decision.side_effects.is_empty(),
        "E2E-CS-3: Decline must carry no side effects; got: {:?}",
        decision.side_effects
    );
}

// ─── E2E-CS-4 — AttestationWrite: safety Allow → discernment Verdict ──────────
//
// `AttestationWrite` routes through [content-safety-gate-v1, discernment-gate-v1-mechanical].
// In DevContext, content-safety always returns Allow (mock).
// The discernment-gate runs next and produces a Verdict (rule-1: first-pass green).
//
// The multi-gate loop: safety Allow → continue → discernment Verdict → `last_decision`
// is set to Verdict → loop ends → Verdict returned.
//
// Asserts:
// - Final decision is Verdict(StoryPoint{valence:"progress", ...}).
// - Side effects include MintAttestation + EmitEconomicEvent from discernment.
// - decision_attestation_cid is Some (discernment-gate CID, last gate wins).

#[tokio::test]
async fn e2e_cs4_attestation_write_safety_allow_then_discernment_verdict() {
    // Inject discernment-gate context extension (rule-1: first-pass green).
    gate_client::__test_set_gate_ctx(
        "discernment-gate-v1-mechanical",
        Some(rule_1_discernment_ctx()),
    );

    let event = attestation_write_event("moment-hash-cs4");
    let decision = check(event).await.expect(
        "E2E-CS-4: check() must not fail for AttestationWrite + content-safety + discernment",
    );

    // Assertion 1: the final decision is a Verdict (discernment ran after safety Allow).
    assert!(
        matches!(&decision.status, GateStatus::Verdict(_)),
        "E2E-CS-4: AttestationWrite with rule-1 context must produce Verdict; \
         content-safety Allow + discernment Verdict = final Verdict; got: {:?}",
        decision.status
    );

    // Assertion 2: Verdict is StoryPoint with valence "progress" (rule-1: first-pass-green).
    match &decision.status {
        GateStatus::Verdict(GateTag::StoryPoint {
            valence,
            magnitude,
            evidence_type,
        }) => {
            assert_eq!(
                valence, "progress",
                "E2E-CS-4: rule-1 Verdict must have valence 'progress'; got: {valence}"
            );
            assert_eq!(
                magnitude, "meaningful",
                "E2E-CS-4: rule-1 Verdict must have magnitude 'meaningful'; got: {magnitude}"
            );
            assert_eq!(
                evidence_type, "first-pass-green",
                "E2E-CS-4: rule-1 Verdict evidence_type must be 'first-pass-green'; \
                 got: {evidence_type}"
            );
        }
        other => panic!(
            "E2E-CS-4: expected Verdict(StoryPoint{{valence:\"progress\",...}}), got: {other:?}"
        ),
    }

    // Assertion 3: MintAttestation side effect is present (discernment rule-1).
    let has_mint = decision.side_effects.iter().any(|se| {
        matches!(se, gate_client::SideEffect::MintAttestation { shape, .. }
            if shape == "StoryPointLink")
    });
    assert!(
        has_mint,
        "E2E-CS-4: discernment rule-1 must emit MintAttestation{{shape:StoryPointLink}}; \
         side effects: {:?}",
        decision.side_effects
    );

    // Assertion 4: EmitEconomicEvent side effect is present.
    let has_economic = decision
        .side_effects
        .iter()
        .any(|se| matches!(se, gate_client::SideEffect::EmitEconomicEvent { .. }));
    assert!(
        has_economic,
        "E2E-CS-4: discernment rule-1 must emit EmitEconomicEvent side effect; \
         side effects: {:?}",
        decision.side_effects
    );

    // Assertion 5: CID is Some (discernment-gate attached it as the last-running gate).
    assert!(
        decision.decision_attestation_cid.is_some(),
        "E2E-CS-4: final Verdict decision must carry decision_attestation_cid"
    );
    let cid = decision.decision_attestation_cid.as_ref().unwrap();
    assert!(
        cid.starts_with("sha256-"),
        "E2E-CS-4: decision_attestation_cid must start with 'sha256-'; got: {cid}"
    );
}

// ─── E2E-CS-5 — Override short-circuits entire multi-gate pipeline ─────────────
//
// `__test_set_decision_override` fires before the universal-band AND before any
// domain gate.  Neither content-safety nor discernment should run.
//
// Evidence that NO gate ran:
// - The injected Decline is returned immediately.
// - The category matches the injected value (not "content-safety").

#[tokio::test]
async fn e2e_cs5_decision_override_short_circuits_entire_pipeline() {
    gate_client::__test_set_decision_override(Some(gate_client::testing::mock_decline(
        "e2e-cs5-test-override",
        "injected override to prove both domain gates were short-circuited",
    )));

    let event = attestation_write_event("moment-hash-cs5");
    let decision = check(event)
        .await
        .expect("E2E-CS-5: check() must not fail with decision override");

    // Assertion 1: the injected Decline was returned.
    assert!(
        !decision.is_allowed(),
        "E2E-CS-5: override must return the injected Decline; got: {:?}",
        decision.status
    );

    // Assertion 2: the category is the injected one — NOT "content-safety".
    // This proves neither content-safety NOR discernment ran.
    let status_json = serde_json::to_string(&decision.status).unwrap();
    assert!(
        status_json.contains("e2e-cs5-test-override"),
        "E2E-CS-5: injected Decline category must be in status JSON; got: {status_json}"
    );
    assert!(
        !status_json.contains("content-safety"),
        "E2E-CS-5: content-safety must NOT have run (category must not be 'content-safety'); \
         got: {status_json}"
    );

    // Assertion 3: the override is one-shot — next call on a non-routed event returns Allow.
    let other_event = RelationalImpactEvent::EconomicEventEmit {
        event_kind: "transfer".to_string(),
        provider: "agent-p".to_string(),
        receiver: "agent-r".to_string(),
        quantity: "1".to_string(),
    };
    let second = check(other_event)
        .await
        .expect("E2E-CS-5: second check() must not fail after override cleared");
    assert!(
        second.is_allowed(),
        "E2E-CS-5: after override consumed, next check() must return Allow; got: {:?}",
        second.status
    );
}

// ─── E2E-CS-6 — Reach-gate regression: Vec-shaped dispatcher still routes ──────
//
// Phase 6 Task 6.2 refactored the dispatcher to return a `Vec<&str>` of gate
// names.  This test is a regression guard: `CapabilityInvoke{reach-negotiation}`
// must still route to reach-gate-v1 only (not to content-safety-gate first).
//
// `CapabilityInvoke{capability:"reach-negotiation"}` → `["reach-gate-v1"]` only.
// Not in `["content-safety-gate-v1", ...]`.

#[tokio::test]
async fn e2e_cs6_reach_gate_still_works_after_vec_dispatcher_refactor() {
    let subject = "bafycs6-reach-regression-subject";

    // Inject reach-gate context and attestation resolver (40 attestations → "public").
    let mut ctx = GateContext::new();
    ctx.insert("subject", json!(subject));
    gate_client::__test_set_gate_ctx("reach-gate-v1", Some(ctx));

    let records: Vec<AttestationRecord> = (0..40)
        .map(|i| AttestationRecord {
            kind: "endorsement".to_string(),
            subject: subject.to_string(),
            issuer: format!("agent-issuer-{i}"),
            issued_at: "2026-01-01T00:00:00Z".to_string(),
            payload_json: "{}".to_string(),
        })
        .collect();
    let resolver = Arc::new(StubAttestationResolver::single(subject, records));
    gate_client::__test_set_gate_attestation_resolver("reach-gate-v1", Some(resolver));

    let event = RelationalImpactEvent::CapabilityInvoke {
        capability: "reach-negotiation".to_string(),
        requester: "test-requester".to_string(),
        request_id: "cs6-regression".to_string(),
    };

    let decision = check(event)
        .await
        .expect("E2E-CS-6: check() must not fail for reach-negotiation");

    // Assertion 1: result is Verdict(ReachLevel) — reach-gate ran.
    match &decision.status {
        GateStatus::Verdict(GateTag::ReachLevel { level }) => {
            assert_eq!(
                level, "public",
                "E2E-CS-6: 40 attestations (≥20, <50) must yield 'public'; got: {level}"
            );
        }
        other => panic!(
            "E2E-CS-6: expected Verdict(ReachLevel{{level:\"public\"}}), got: {other:?}\n\
             Hint: reach-gate dispatch may have broken after Vec-refactor"
        ),
    }

    // Assertion 2: CID is Some — reach-gate attached it.
    assert!(
        decision.decision_attestation_cid.is_some(),
        "E2E-CS-6: reach-gate Verdict must carry decision_attestation_cid"
    );

    // Assertion 3: NOT a content-safety Decline — confirms content-safety did NOT
    // run for reach-negotiation (CapabilityInvoke routes only to reach-gate-v1).
    assert!(
        decision.is_allowed() || matches!(&decision.status, GateStatus::Verdict(_)),
        "E2E-CS-6: reach-negotiation must not Decline (content-safety must not have \
         run for this event kind); got: {:?}",
        decision.status
    );
}
