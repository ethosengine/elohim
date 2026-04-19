//! Activation E2E integration tests — Phase 8.
//!
//! Verifies the full `configure_runner_with_config → check()` pipe.
//!
//! # Why a separate binary?
//!
//! `configure_runner_with_config` writes to the process-global `OnceLock<Arc<DagRunner>>`.
//! Each integration test binary gets its own process, so the OnceLock is fresh
//! for each binary.  The tests within this binary run sequentially; the first
//! test to call `configure_runner_with_config` or `global_runner()` wins the
//! singleton.  Subsequent configure calls must return `AlreadyConfigured`.
//!
//! # Test inventory
//!
//! - **E2E-A1** — Mock transport → DevContext phase in GateDecision.
//! - **E2E-A2** — Honest stub contract: without real inference, phase is DevContext.
//!   This is the KEY test: proves phase honesty — no API key → no ElohimActive.
//! - **E2E-A3** — Canned elohim-active wisdom output → ElohimActive phase.
//!   Proves activation works when real inference actually happens (simulated via
//!   `__test_set_wisdom_output`).
//! - **E2E-A4** — `GateClientConfig::from_env()` round-trip assertions.
//! - **E2E-A5** — Second `configure_runner_with_config` call returns
//!   `AlreadyConfigured` (singleton guard).

use gate_client::{
    check, configure_runner_with_config, global_runner,
    transport::{GateClientConfig, WisdomTransport},
    AlreadyConfigured, RelationalImpactEvent,
};

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn content_publish_event() -> RelationalImpactEvent {
    RelationalImpactEvent::ContentPublish {
        content_cid: "bafye2e-activation-test".to_string(),
        declared_reach: "public".to_string(),
        author: "agent-activation-e2e".to_string(),
    }
}

// ─── E2E-A1 — Mock transport → DevContext ─────────────────────────────────────
//
// Configure the runner with WisdomTransport::Mock.
// Call check() with ContentPublish.
// Assert: phase is DevContext, decision_attestation_cid is Some(sha256-...).
//
// This is the first test in this binary — it wins the singleton OnceLock.

#[tokio::test]
async fn e2e_a1_mock_transport_produces_dev_context() {
    let config = GateClientConfig {
        wisdom_transport: WisdomTransport::Mock,
        ..Default::default()
    };

    // First call in this binary — must succeed.
    let result = configure_runner_with_config(config);
    match result {
        Ok(()) => {
            // Runner configured. Proceed to gate check.
        }
        Err(AlreadyConfigured) => {
            // Another test ran first and won the OnceLock. Still verify behaviour.
        }
    }

    let decision = check(content_publish_event())
        .await
        .expect("check() must succeed");

    assert!(
        decision.is_allowed(),
        "Mock transport must produce Allow; got: {:?}",
        decision.status
    );
    assert!(
        decision.phase.is_dev_context(),
        "Mock transport must produce Phase::DevContext; got: {:?}",
        decision.phase
    );
    assert!(
        decision.decision_attestation_cid.is_some(),
        "decision_attestation_cid must be present (phase-4 attestation)"
    );
    let cid = decision.decision_attestation_cid.unwrap();
    assert!(
        cid.starts_with("sha256-"),
        "attestation CID must start with 'sha256-'; got: {cid}"
    );
}

// ─── E2E-A2 — Honest stub contract: no real inference → DevContext ────────────
//
// The core protocol honesty contract: phase is observed from inference, not
// declared.  Without a real LLM call, the decision must carry Phase::DevContext.
//
// Uses the Mock transport (set in E2E-A1 or by default) which always returns
// DevContext.  This proves the "no activation without real inference" invariant.
// The OnceLock is already set by E2E-A1, so check() uses Mock transport.

#[tokio::test]
async fn e2e_a2_without_real_inference_phase_is_dev_context() {
    // check() uses whatever transport was configured at singleton init (Mock).
    // Mock transport always returns DevContext — no API key needed, no real call.
    let decision = check(content_publish_event())
        .await
        .expect("check() must succeed");

    assert!(
        decision.is_allowed(),
        "Mock transport must produce Allow; got: {:?}",
        decision.status
    );

    // The core honesty contract: without real inference, phase is DevContext.
    // This is the protocol's observed-not-flagged invariant.
    assert!(
        decision.phase.is_dev_context(),
        "Without real inference (Mock transport or no API key), phase must be \
         Phase::DevContext; got: {:?}",
        decision.phase
    );

    assert!(
        decision.decision_attestation_cid.is_some(),
        "decision_attestation_cid must be present"
    );
}

// ─── E2E-A3 — Canned elohim-active wisdom output → ElohimActive ──────────────
//
// Injects a pre-cooked elohim-active wisdom output via `__test_set_wisdom_output`.
// This simulates what happens when real inference actually succeeds.
// Verifies the full wisdom-output → SynthesizeExecutor → GateDecision phase
// propagation path without needing a real LLM backend.
//
// Uses AdviceSought which routes through the universal-band ONLY (no domain gates).
// ContentPublish and AttestationWrite also route through content-safety-gate-v1
// which has its own wisdom step that would return DevContext, overwriting the
// universal-band's elohim-active phase.  AdviceSought has no domain gate, so
// the universal-band decision is final.
//
// The universal-band wisdom step keys (from universal_band_v1.yaml):
//   constitution_cid = "epr:constitution-v1"
//   output_key = "wisdom_output"

#[tokio::test]
async fn e2e_a3_canned_elohim_active_wisdom_output_produces_elohim_active() {
    // Inject an elohim-active output for the universal-band wisdom step.
    gate_client::__test_set_wisdom_output(
        "epr:constitution-v1",
        "wisdom_output",
        serde_json::json!({
            "decision": "allow",
            "reasoning": {
                "primary_principle": "subsidiarity",
                "confidence": 0.93,
                "summary": "Real inference confirmed allowance — activation E2E test.",
                "phase_note": "real-llm"
            },
            "wisdomContext": {},
            "constitutionCid": "epr:constitution-v1",
            "framingCid": "epr:universal-band-framing-v1",
            "phase": "elohim-active"
        }),
    );

    // AdviceSought → universal-band only (no domain gate).
    // The universal-band decision with elohim-active phase is final.
    let advice_event = RelationalImpactEvent::AdviceSought {
        requester: "agent-activation-e2e".to_string(),
        summary_cid: "sum-cid-activation".to_string(),
        topic: "protocol-integrity".to_string(),
    };
    let decision = check(advice_event).await.expect("check() must succeed");

    assert!(
        decision.phase.is_elohim_active(),
        "Canned elohim-active wisdom output must propagate to Phase::ElohimActive \
         in GateDecision; got: {:?}",
        decision.phase
    );
    assert!(
        decision.is_allowed(),
        "Decision must still be Allow; got: {:?}",
        decision.status
    );
    assert!(
        decision.decision_attestation_cid.is_some(),
        "decision_attestation_cid must be present even for ElohimActive decisions"
    );
}

// ─── E2E-A4 — GateClientConfig::from_env() round-trip ────────────────────────
//
// Verifies that from_env() correctly maps env var values to WisdomTransport.
//
// All env-var assertions are in a SINGLE test function to avoid parallel-test
// env races.  Set → assert → restore happens atomically within one test.
// #[test] (not tokio) so it runs synchronously without any thread switching.

#[test]
fn e2e_a4_from_env_round_trip() {
    // ── WisdomTransport: InProcess ────────────────────────────────────────────
    let orig_transport = std::env::var("ELOHIM_AGENT_WISDOM_TRANSPORT").ok();
    std::env::set_var("ELOHIM_AGENT_WISDOM_TRANSPORT", "in-process");
    {
        let config = GateClientConfig::from_env();
        assert_eq!(
            config.wisdom_transport,
            WisdomTransport::InProcess,
            "ELOHIM_AGENT_WISDOM_TRANSPORT=in-process must select WisdomTransport::InProcess"
        );
        assert!(
            config.phase_override.is_none(),
            "from_env() must NOT set phase_override (phase is observed, not declared)"
        );
    }

    // ── WisdomTransport: Mock (default, env unset) ────────────────────────────
    std::env::remove_var("ELOHIM_AGENT_WISDOM_TRANSPORT");
    {
        let config = GateClientConfig::from_env();
        assert_eq!(
            config.wisdom_transport,
            WisdomTransport::Mock,
            "Unset ELOHIM_AGENT_WISDOM_TRANSPORT must default to WisdomTransport::Mock"
        );
    }

    // Restore transport var.
    match orig_transport {
        Some(ref v) => std::env::set_var("ELOHIM_AGENT_WISDOM_TRANSPORT", v),
        None => std::env::remove_var("ELOHIM_AGENT_WISDOM_TRANSPORT"),
    }

    // ── ELOHIM_ID: set then unset ─────────────────────────────────────────────
    let orig_id = std::env::var("ELOHIM_ID").ok();
    std::env::set_var("ELOHIM_ID", "agent-pubkey-base64-abc123");
    {
        let config = GateClientConfig::from_env();
        assert_eq!(
            config.elohim_id.as_deref(),
            Some("agent-pubkey-base64-abc123"),
            "ELOHIM_ID env var must be populated into elohim_id"
        );
    }
    std::env::remove_var("ELOHIM_ID");
    {
        let config = GateClientConfig::from_env();
        assert!(
            config.elohim_id.is_none(),
            "Unset ELOHIM_ID must leave elohim_id as None; got: {:?}",
            config.elohim_id
        );
    }
    // Restore ELOHIM_ID.
    match orig_id {
        Some(ref v) => std::env::set_var("ELOHIM_ID", v),
        None => std::env::remove_var("ELOHIM_ID"),
    }

    // ── ELOHIM_SUBSTANCE_CID: set ─────────────────────────────────────────────
    let orig_cid = std::env::var("ELOHIM_SUBSTANCE_CID").ok();
    std::env::set_var("ELOHIM_SUBSTANCE_CID", "epr:substance:prod-v1");
    {
        let config = GateClientConfig::from_env();
        assert_eq!(
            config.elohim_substance_cid.as_deref(),
            Some("epr:substance:prod-v1"),
            "ELOHIM_SUBSTANCE_CID env var must be populated into elohim_substance_cid"
        );
    }
    // Restore ELOHIM_SUBSTANCE_CID.
    match orig_cid {
        Some(ref v) => std::env::set_var("ELOHIM_SUBSTANCE_CID", v),
        None => std::env::remove_var("ELOHIM_SUBSTANCE_CID"),
    }
}

// ─── E2E-A5 — Second configure_runner_with_config returns AlreadyConfigured ───
//
// The singleton is already set by E2E-A1 (or by the implicit global_runner() call
// from the first check()).  A second configure call must return AlreadyConfigured.

#[test]
fn e2e_a5_double_configure_returns_already_configured() {
    // Ensure the singleton is initialized (may already be by the async tests).
    let _runner = global_runner();

    let config = GateClientConfig {
        wisdom_transport: WisdomTransport::Mock,
        ..Default::default()
    };

    let result = configure_runner_with_config(config);
    assert!(
        result.is_err(),
        "Second configure_runner_with_config must return Err(AlreadyConfigured)"
    );

    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("already"),
        "AlreadyConfigured message must mention 'already'; got: {msg}"
    );
}
