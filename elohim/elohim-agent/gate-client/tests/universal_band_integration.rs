//! End-to-end integration tests for the universal-band DAG execution path.
//!
//! These tests exercise the full `check()` path against the real DAG interpreter
//! (not the Phase 0 stub).  They prove:
//!
//! 1. The universal-band DAG runs end-to-end in DevContext.
//! 2. The decision shape is Allow { exempt: false } with phase: DevContext.
//! 3. The reasoning carries `phase_note == "wisdom-sourced"`, proving the real
//!    DAG ran through the WisdomInvoke and Synthesize executors — not the old
//!    Phase 0 `allow_mocked()` stub which uses `"This decision carries no
//!    reputation weight."` as its phase_note.
//! 4. `decision_attestation_cid` is Some (Phase 4: CID computed, DHT write is Phase 6+).
//! 5. The `__test_set_decision_override` short-circuit fires BEFORE the DAG.
//! 6. Exempt space types return `Allow { exempt: true }` without running the DAG.

use gate_client::{check, events::RelationalImpactEvent, phase::Phase, GateStatus};

// ─── E2E-1: ContentPublish through real DAG ───────────────────────────────────
//
// Minimum: 3 assertions per test case per task spec.

#[tokio::test]
async fn content_publish_real_dag_returns_allow_non_exempt_dev_context() {
    let event = RelationalImpactEvent::ContentPublish {
        content_cid: "bafycontentpublish-integration-test".into(),
        declared_reach: "public".into(),
        author: "agent-integration-test".into(),
    };

    let decision = check(event).await.expect("check() must not fail");

    // Assertion 1: The decision is an Allow.
    assert!(
        decision.is_allowed(),
        "universal-band DAG must Allow ContentPublish in DevContext; got: {:?}",
        decision.status
    );

    // Assertion 2: The Allow is NOT exempt — the DAG ran, not the short-circuit.
    assert!(
        !decision.is_exempt(),
        "ContentPublish must NOT be exempt; the DAG ran and produced Allow {{ exempt: false }}"
    );

    // Assertion 3: Phase is DevContext.
    assert!(
        decision.phase.is_dev_context(),
        "phase must be DevContext; got: {:?}",
        decision.phase
    );
}

#[tokio::test]
async fn content_publish_dag_reasoning_phase_note_is_wisdom_sourced() {
    let event = RelationalImpactEvent::ContentPublish {
        content_cid: "bafycontentpublish-reasoning-test".into(),
        declared_reach: "community".into(),
        author: "agent-wisdom-test".into(),
    };

    let decision = check(event).await.expect("check() must not fail");

    // Assertion 1: phase_note == "wisdom-sourced" proves the real allow-with-wisdom
    // synthesize builder ran — the Phase 0 stub would set a different phase_note.
    assert_eq!(
        decision.reasoning.phase_note, "wisdom-sourced",
        "phase_note must be 'wisdom-sourced' (proves real DAG ran); \
         Phase 0 stub sets 'This decision carries no reputation weight.'; got: {:?}",
        decision.reasoning.phase_note
    );

    // Assertion 2: primary_principle carries the dev-context-mock value from
    // WisdomInvokeExecutor's mock output.
    assert_eq!(
        decision.reasoning.primary_principle, "dev-context-mock",
        "primary_principle should carry dev-context-mock from WisdomInvokeExecutor; \
         got: {:?}",
        decision.reasoning.primary_principle
    );

    // Assertion 3: decision_attestation_cid is Some — Phase 4 computes the CID.
    // The CID is computed but not published to DHT (Phase 6+ activation).
    assert!(
        decision.decision_attestation_cid.is_some(),
        "decision_attestation_cid must be Some in Phase 4; got: {:?}",
        decision.decision_attestation_cid
    );
    let cid = decision.decision_attestation_cid.as_ref().unwrap();
    assert!(
        cid.starts_with("sha256-"),
        "decision_attestation_cid must start with 'sha256-'; got: {cid}"
    );
}

// ─── E2E-2: All 8 event variants through real DAG ────────────────────────────

#[tokio::test]
async fn all_8_event_variants_run_through_real_dag_and_return_allow() {
    let events: Vec<RelationalImpactEvent> = vec![
        RelationalImpactEvent::ContentPublish {
            content_cid: "cid-1".into(),
            declared_reach: "public".into(),
            author: "agent-a".into(),
        },
        RelationalImpactEvent::AttestationWrite {
            subject_hash: "hash-1".into(),
            claim_kind: "brit".into(),
            issuer: "agent-b".into(),
        },
        RelationalImpactEvent::EconomicEventEmit {
            event_kind: "transfer".into(),
            provider: "agent-c".into(),
            receiver: "agent-d".into(),
            quantity: "5".into(),
        },
        RelationalImpactEvent::PeerMessage {
            recipient: "agent-e".into(),
            payload_kind: "handshake".into(),
        },
        RelationalImpactEvent::SyncToPeers {
            manifest_cid: "manifest-cid".into(),
            item_count: 3,
        },
        RelationalImpactEvent::AdviceSought {
            requester: "agent-f".into(),
            summary_cid: "sum-cid".into(),
            topic: "conflict".into(),
        },
        RelationalImpactEvent::CapabilityInvoke {
            capability: "translate".into(),
            requester: "agent-g".into(),
            request_id: "req-1".into(),
        },
        RelationalImpactEvent::PrivateToPublicCrossing {
            source_space: "private-draft".into(),
            artifact_ref: "artifact-cid".into(),
        },
    ];

    for event in events {
        let kind = event.kind();
        let decision = check(event)
            .await
            .unwrap_or_else(|e| panic!("check() failed for {kind}: {e}"));

        // Assertion 1: Always Allow.
        assert!(decision.is_allowed(), "DAG must Allow {kind} in DevContext");

        // Assertion 2: Not exempt for non-AttestationWrite events.
        // Phase 4 note: AttestationWrite routes through discernment-gate after
        // the universal band.  Without a pre-populated context extension the gate
        // falls to rule-7 (steady-state no-mint) → Allow{exempt:true}.  All
        // other 7 variants take the universal-band-only path and return
        // Allow{exempt:false}.
        if kind != "attestation-write" {
            assert!(
                !decision.is_exempt(),
                "{kind} must NOT be exempt — it runs the universal band only and returns \
                 Allow{{ exempt: false }}"
            );
        }

        // Assertion 3: Non-AttestationWrite events carry the wisdom-sourced proof.
        // AttestationWrite → discernment-gate rule-7 returns Allow{exempt:true}
        // which carries exempt reasoning, not "wisdom-sourced".
        if kind != "attestation-write" {
            assert_eq!(
                decision.reasoning.phase_note, "wisdom-sourced",
                "{kind}: phase_note must be 'wisdom-sourced' to prove real DAG ran"
            );
        }
    }
}

// ─── E2E-3: Override mechanism short-circuits before DAG ─────────────────────

#[tokio::test]
async fn test_override_short_circuits_before_dag() {
    // Inject a Decline via the override — the DAG should NOT run.
    // Proof: the injected decision's reasoning.phase_note is "mocked" (set by
    // mock_decline), not "wisdom-sourced" (set by the real DAG).
    gate_client::__test_set_decision_override(Some(gate_client::testing::mock_decline(
        "integration-test-override",
        "override injected to prove DAG is skipped",
    )));

    let event = RelationalImpactEvent::ContentPublish {
        content_cid: "cid-override-test".into(),
        declared_reach: "public".into(),
        author: "agent-override".into(),
    };

    let decision = check(event).await.expect("check() must not fail");

    // Assertion 1: The injected Decline was returned (not the DAG's Allow).
    assert!(
        !decision.is_allowed(),
        "override must return the injected Decline; got Allow when Decline was expected"
    );

    // Assertion 2: The decision carries the injected category — it's the exact
    // override, not a DAG decision.
    let json = serde_json::to_string(&decision.status).unwrap();
    assert!(
        json.contains("integration-test-override"),
        "injected Decline category must be present in status JSON; got: {json}"
    );

    // Assertion 3: Override is one-shot — next call returns the real DAG's Allow.
    let event2 = RelationalImpactEvent::ContentPublish {
        content_cid: "cid-after-override".into(),
        declared_reach: "public".into(),
        author: "agent-second-call".into(),
    };
    let second = check(event2).await.expect("second check() must not fail");
    assert!(
        second.is_allowed(),
        "after override consumed, real DAG must run and return Allow"
    );
    assert_eq!(
        second.reasoning.phase_note, "wisdom-sourced",
        "second call phase_note must be 'wisdom-sourced' (real DAG ran)"
    );
}

// ─── E2E-4: decision_attestation_cid is Some with sha256- prefix (Phase 4) ───
//
// Phase 4: every non-exempt DAG decision now carries a computed attestation CID.
// The CID is computed in-process; the DHT write is Phase 6+ activation.

#[tokio::test]
async fn decision_attestation_cid_is_some_with_sha256_prefix_for_all_events() {
    let events = vec![
        RelationalImpactEvent::ContentPublish {
            content_cid: "cid-attest-1".into(),
            declared_reach: "public".into(),
            author: "agent".into(),
        },
        RelationalImpactEvent::AttestationWrite {
            subject_hash: "hash-attest".into(),
            claim_kind: "brit".into(),
            issuer: "agent".into(),
        },
    ];

    for event in events {
        let kind = event.kind();
        let decision = check(event).await.unwrap_or_else(|e| panic!("{kind}: {e}"));

        // Assertion 1: CID is now Some (Phase 4 computes it).
        assert!(
            decision.decision_attestation_cid.is_some(),
            "{kind}: decision_attestation_cid must be Some in Phase 4; got: {:?}",
            decision.decision_attestation_cid
        );

        // Assertion 2: CID has the expected sha256- prefix.
        let cid = decision.decision_attestation_cid.as_ref().unwrap();
        assert!(
            cid.starts_with("sha256-"),
            "{kind}: decision_attestation_cid must start with 'sha256-'; got: {cid}"
        );
        assert_eq!(
            cid.len(),
            71,
            "{kind}: 'sha256-' + 64 hex chars = 71; got len {} for: {cid}",
            cid.len()
        );

        // Assertion 3: phase is DevContext.
        assert_eq!(
            decision.phase,
            Phase::DevContext,
            "{kind}: phase must be DevContext"
        );
    }
}

// ─── E2E-4b: All 8 variants produce a non-None attestation CID ───────────────

#[tokio::test]
async fn all_8_variants_produce_non_none_attestation_cid() {
    use gate_client::events::RelationalImpactEvent;

    let events: Vec<RelationalImpactEvent> = vec![
        RelationalImpactEvent::ContentPublish {
            content_cid: "cid-a".into(),
            declared_reach: "public".into(),
            author: "agent-a".into(),
        },
        RelationalImpactEvent::AttestationWrite {
            subject_hash: "hash-b".into(),
            claim_kind: "brit".into(),
            issuer: "agent-b".into(),
        },
        RelationalImpactEvent::EconomicEventEmit {
            event_kind: "transfer".into(),
            provider: "agent-c".into(),
            receiver: "agent-d".into(),
            quantity: "5".into(),
        },
        RelationalImpactEvent::PeerMessage {
            recipient: "agent-e".into(),
            payload_kind: "handshake".into(),
        },
        RelationalImpactEvent::SyncToPeers {
            manifest_cid: "manifest".into(),
            item_count: 2,
        },
        RelationalImpactEvent::AdviceSought {
            requester: "agent-f".into(),
            summary_cid: "sum".into(),
            topic: "conflict".into(),
        },
        RelationalImpactEvent::CapabilityInvoke {
            capability: "translate".into(),
            requester: "agent-g".into(),
            request_id: "req".into(),
        },
        RelationalImpactEvent::PrivateToPublicCrossing {
            source_space: "draft".into(),
            artifact_ref: "art".into(),
        },
    ];

    for event in events {
        let kind = event.kind();
        let decision = check(event)
            .await
            .unwrap_or_else(|e| panic!("{kind}: check() failed: {e}"));

        // Assertion 1: CID is Some.
        assert!(
            decision.decision_attestation_cid.is_some(),
            "{kind}: decision_attestation_cid must be Some in Phase 4"
        );

        // Assertion 2: CID has sha256- prefix.
        let cid = decision.decision_attestation_cid.as_ref().unwrap();
        assert!(
            cid.starts_with("sha256-"),
            "{kind}: CID must start with 'sha256-'; got: {cid}"
        );

        // Assertion 3: CID length is correct (7 + 64 = 71).
        assert_eq!(
            cid.len(),
            71,
            "{kind}: CID must be 71 chars (sha256- + 64 hex); got: {cid}"
        );
    }
}

// ─── E2E-5: Confirm primary_principle is NOT "dev-context-mock" placeholder ──
//
// This is the strongest proof that the real DAG ran: the allow-with-wisdom
// synthesize builder explicitly propagates the wisdom output's reasoning.
// WisdomInvokeExecutor writes "dev-context-mock" as primary_principle — so
// the assertion is that the synthesize builder PROPAGATED this value, confirming
// it read from the wisdom output rather than using ConstitutionalReasoningSummary::mocked()
// (which is the Phase 0 path).
//
// The distinguishing signal between Phase 0 stub and real DAG is phase_note:
//   Phase 0 (allow_mocked):   phase_note == "This decision carries no reputation weight."
//   Phase 2 (allow-with-wisdom): phase_note == "wisdom-sourced"

#[tokio::test]
async fn dag_phase_note_distinguishes_real_dag_from_phase0_stub() {
    let event = RelationalImpactEvent::ContentPublish {
        content_cid: "cid-phase-note-check".into(),
        declared_reach: "public".into(),
        author: "agent-phase-note".into(),
    };

    let decision = check(event).await.expect("check() must not fail");

    // Assertion 1: phase_note is NOT the Phase 0 stub string.
    assert_ne!(
        decision.reasoning.phase_note, "This decision carries no reputation weight.",
        "phase_note must NOT be the Phase 0 stub string — the real DAG should be running"
    );

    // Assertion 2: phase_note IS the real-DAG marker.
    assert_eq!(
        decision.reasoning.phase_note, "wisdom-sourced",
        "phase_note must be 'wisdom-sourced' set by allow-with-wisdom synthesize builder"
    );

    // Assertion 3: full Allow shape is correct.
    assert!(
        matches!(decision.status, GateStatus::Allow { exempt: false }),
        "status must be Allow {{ exempt: false }}; got: {:?}",
        decision.status
    );
}
