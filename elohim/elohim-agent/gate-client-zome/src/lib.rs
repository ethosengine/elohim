//! gate-client-zome — Synchronous gate invocation for Holochain WASM zomes.
//!
//! Zomes run in wasm32-unknown-unknown; no tokio, no network, no threads.
//! DevContext invariant: check_blocking returns Allow { phase: DevContext }.
//! Phase 8+ activation will bridge to elohim-agent-service via HDK remote_signal.
//!
//! # Why this crate exists
//!
//! `gate-client` depends on `elohim-agent` which pulls in `reqwest` + `tokio` +
//! `mio` — none of which compile for `wasm32-unknown-unknown` (Holochain zomes).
//! This crate provides the same `check_blocking` surface with a pure-sync Allow
//! mock, satisfying the gate invariant (every relational-impact write path calls
//! the gate) without any native-only dependencies.
//!
//! # DevContext contract
//!
//! `check_blocking` always returns `Allow { phase: DevContext }`. This matches
//! the `gate-client` DevContext invariant exactly — zome callers get the same
//! Allow shape as native callers, just without the async DAG interpreter overhead.
//!
//! # Phase 8+ upgrade path
//!
//! When `ElohimActive` is configured, this function will bridge through HDK
//! `remote_signal` to the elohim-agent-service running on the local node. That
//! implementation is Task 7.2b and beyond — today this crate is the DevContext
//! stub that proves the zome integration point compiles.

pub use gate_types::*;

/// Check a relational-impact event synchronously.
///
/// DevContext: always returns Allow mock. No runtime, no network, no async.
/// ElohimActive (Phase 8+): bridges through HDK remote_signal.
///
/// # Example (zome coordinator pattern)
///
/// ```rust
/// use gate_client_zome::{check_blocking, GateStatus, RelationalImpactEvent};
///
/// fn create_content_zome_check(cid: String, reach: String, author: String)
///     -> Result<(), String>
/// {
///     let event = RelationalImpactEvent::ContentPublish {
///         content_cid: cid,
///         declared_reach: reach,
///         author,
///     };
///
///     let decision = check_blocking(event).map_err(|e| format!("gate error: {e}"))?;
///
///     match decision.status {
///         GateStatus::Allow { .. } => Ok(()),
///         GateStatus::Decline { grounds } => Err(format!("declined: {}", grounds.summary)),
///         GateStatus::Escalate { .. } => Err("escalated for review".into()),
///         GateStatus::Verdict(_) => Ok(()),
///     }
/// }
///
/// // DevContext: always returns Allow.
/// assert!(create_content_zome_check("cid".into(), "public".into(), "agent".into()).is_ok());
/// ```
pub fn check_blocking(event: RelationalImpactEvent) -> GateResult<GateDecision> {
    // Phase 8+: event will be serialized and forwarded via HDK remote_signal.
    // DevContext: not inspected — the mock short-circuits before any dispatch.
    // Suppress the unused-variable lint; the parameter is part of the public API.
    let _ = &event;
    // DevContext: short-circuit to Allow mock. Matches the gate-client DevContext
    // invariant exactly — zome callers get the same Allow shape as native callers,
    // just without the async DAG interpreter overhead.
    #[cfg(any(test, feature = "testing"))]
    {
        if let Some(override_decision) = test_override::take_test_override() {
            return Ok(override_decision);
        }
    }
    Ok(GateDecision::allow_mocked(Phase::DevContext))
}

/// Test-only thread-local override for injecting decisions.
#[cfg(any(test, feature = "testing"))]
mod test_override {
    use core::cell::RefCell;
    use gate_types::GateDecision;

    thread_local! {
        static OVERRIDE: RefCell<Option<GateDecision>> = RefCell::new(None);
    }

    pub fn set_test_override(decision: Option<GateDecision>) {
        OVERRIDE.with(|o| *o.borrow_mut() = decision);
    }

    pub(super) fn take_test_override() -> Option<GateDecision> {
        OVERRIDE.with(|o| o.borrow_mut().take())
    }
}

#[cfg(any(test, feature = "testing"))]
pub use test_override::set_test_override as __test_set_decision_override;

#[cfg(test)]
mod tests {
    use super::*;

    // ─── DevContext invariant ─────────────────────────────────────────────────

    #[test]
    fn check_blocking_returns_allow_mock_in_dev_context() {
        let event = RelationalImpactEvent::ContentPublish {
            content_cid: "bafkreiabc".to_string(),
            declared_reach: "public".to_string(),
            author: "agent-alice".to_string(),
        };
        let decision = check_blocking(event).expect("check_blocking must not fail");
        assert!(decision.is_allowed(), "DevContext must return Allow");
        assert!(!decision.is_exempt(), "DevContext Allow must not be exempt");
        assert!(
            decision.phase.is_dev_context(),
            "DevContext must carry DevContext phase"
        );
        assert_eq!(
            decision.reasoning.primary_principle, "dev-context-mock",
            "DevContext reasoning must be mocked"
        );
    }

    // ─── Override mechanism ───────────────────────────────────────────────────

    #[test]
    fn test_override_returns_injected_decline() {
        // Inject a Decline by constructing one directly.
        let decline = GateDecision {
            status: GateStatus::Decline {
                grounds: DeclineGrounds {
                    category: "test-category".to_string(),
                    summary: "injected for test".to_string(),
                    principle_refs: Vec::new(),
                },
            },
            reasoning: ConstitutionalReasoningSummary::mocked(),
            side_effects: Vec::new(),
            decision_attestation_cid: None,
            phase: Phase::DevContext,
        };

        __test_set_decision_override(Some(decline));

        let event = RelationalImpactEvent::PeerMessage {
            recipient: "agent-bob".to_string(),
            payload_kind: "handshake".to_string(),
        };
        let decision = check_blocking(event).expect("check_blocking must not fail");
        assert!(!decision.is_allowed(), "injected Decline must be returned");
        let json = serde_json::to_string(&decision.status).unwrap();
        assert!(
            json.contains("test-category"),
            "Decline body must carry injected category, got: {json}"
        );
    }

    #[test]
    fn test_override_returns_injected_verdict() {
        let verdict = GateDecision {
            status: GateStatus::Verdict(GateTag::StoryPoint {
                valence: "constructive".to_string(),
                magnitude: "medium".to_string(),
                evidence_type: "direct".to_string(),
            }),
            reasoning: ConstitutionalReasoningSummary::mocked(),
            side_effects: Vec::new(),
            decision_attestation_cid: None,
            phase: Phase::DevContext,
        };

        __test_set_decision_override(Some(verdict));

        let event = RelationalImpactEvent::AttestationWrite {
            subject_hash: "uhCkk-abc".to_string(),
            claim_kind: "brit".to_string(),
            issuer: "agent-carol".to_string(),
        };
        let decision = check_blocking(event).expect("check_blocking must not fail");
        assert!(
            matches!(
                &decision.status,
                GateStatus::Verdict(GateTag::StoryPoint { .. })
            ),
            "injected Verdict must be returned"
        );
    }

    #[test]
    fn test_override_is_one_shot() {
        // Inject a Decline.
        let decline = GateDecision {
            status: GateStatus::Decline {
                grounds: DeclineGrounds {
                    category: "one-shot".to_string(),
                    summary: "should only fire once".to_string(),
                    principle_refs: Vec::new(),
                },
            },
            reasoning: ConstitutionalReasoningSummary::mocked(),
            side_effects: Vec::new(),
            decision_attestation_cid: None,
            phase: Phase::DevContext,
        };

        __test_set_decision_override(Some(decline));

        let event = RelationalImpactEvent::EconomicEventEmit {
            event_kind: "transfer".to_string(),
            provider: "agent-d".to_string(),
            receiver: "agent-e".to_string(),
            quantity: "5".to_string(),
        };

        // First call consumes the override.
        let first = check_blocking(event.clone()).expect("first call must not fail");
        assert!(
            !first.is_allowed(),
            "first call must return the injected Decline"
        );

        // Second call on the same thread must fall back to the DevContext default Allow.
        let second = check_blocking(event).expect("second call must not fail");
        assert!(
            second.is_allowed(),
            "second call must return DevContext Allow (override cleared)"
        );
        assert!(
            second.phase.is_dev_context(),
            "second call must be DevContext phase"
        );
    }
}
