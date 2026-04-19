//! Wisdom invocation — constitutional LLM gate for relational impact events.
//!
//! # Phase observation rule
//!
//! `WisdomPhase` is **observed from the actual call outcome**, never from a
//! config flag.  If no backend is configured, the backend call fails, or the
//! response cannot be parsed, the response carries `phase: dev-context`.
//! Only a successful, parseable real-LLM call yields `phase: elohim-active`.
//!
//! This makes the `phase` field the authoritative signal for downstream
//! attestation phase-marking — no flag elsewhere can override it.

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

#[cfg(feature = "typescript")]
use ts_rs::TS;

use crate::backend::traits::{CompletionRequest, LlmBackend};

// ─── Wire types ───────────────────────────────────────────────────────────────

/// Input for `invoke_wisdom`. Matches `wisdom-invocation-input.schema.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct WisdomInvocationInput {
    /// CID of the constitution ContentNode that governs this invocation.
    pub constitution_cid: String,
    /// CID of the framing ContentNode that scopes the specific wisdom question.
    pub framing_cid: String,
    /// Which GateContext keys the wisdom call should consider.
    pub context_keys: Vec<String>,
    /// Serialized subset of GateContext for the declared context_keys.
    pub context_json: String,
    /// Short natural-language description of the event being judged.
    pub event_summary: String,
}

/// Response from `invoke_wisdom`. Matches
/// `wisdom-invocation-response.schema.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct WisdomInvocationResponse {
    /// Wisdom's verdict on the relational impact event.
    pub decision: WisdomDecision,
    /// Authoritative phase marker — derived from actual call outcome.
    pub phase: WisdomPhase,
    /// Reasoning audit trail.
    pub reasoning: WisdomReasoning,
    /// Side effects (reserved; always empty until Phase 9+).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub side_effects: Vec<serde_json::Value>,
}

/// Verdict variants. Serializes to kebab-case to match the schema enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
#[serde(rename_all = "kebab-case")]
pub enum WisdomDecision {
    Allow,
    Decline,
    Escalate,
    Verdict,
    NeedDeeper,
}

/// Phase marker. `ElohimActive` ONLY when a real LLM call succeeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
#[serde(rename_all = "kebab-case")]
pub enum WisdomPhase {
    /// Stub response — no API key, backend unavailable, or error fallback.
    DevContext,
    /// Real LLM inference produced this response.
    ElohimActive,
}

/// Reasoning audit trail included with every response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct WisdomReasoning {
    /// Constitutional principle applied as the primary guide.
    pub primary_principle: String,
    /// Confidence score (0.0 = stub / no confidence; 1.0 = fully confident).
    pub confidence: f32,
    /// Human-readable explanation of the reasoning.
    pub summary: String,
    /// Contextual note about the phase.
    pub phase_note: String,
}

// ─── Backend selection ────────────────────────────────────────────────────────

/// Select the appropriate backend based on environment.
///
/// Priority:
/// 1. `ANTHROPIC_API_KEY` → Anthropic (Claude)
/// 2. `OPENAI_API_KEY`    → OpenAI
/// 3. None               → `None` (caller falls back to stub)
pub fn select_env_backend() -> Option<Box<dyn LlmBackend + Send + Sync>> {
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        if !key.trim().is_empty() {
            info!("WisdomInvoke: using Anthropic backend (ANTHROPIC_API_KEY set)");
            return Some(Box::new(crate::backend::AnthropicBackend::claude_sonnet(key)));
        }
    }
    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        if !key.trim().is_empty() {
            info!("WisdomInvoke: using OpenAI backend (OPENAI_API_KEY set)");
            return Some(Box::new(crate::backend::OpenAiBackend::openai(
                "gpt-4o",
                key,
            )));
        }
    }
    info!("WisdomInvoke: no API key configured — will return dev-context stub");
    None
}

// ─── Prompt assembly ──────────────────────────────────────────────────────────

/// Build the system + user prompts from the invocation input.
///
/// Phase 8: simple concatenation.  Real prompt engineering is a future
/// refinement — the call-shape is already correct for Phase 9+.
pub fn build_wisdom_prompt(input: &WisdomInvocationInput) -> CompletionRequest {
    let system = format!(
        "You are a constitutional wisdom agent operating under the Elohim Protocol.\n\
         Constitution: {constitution}\n\
         Framing: {framing}\n\n\
         You evaluate relational impact events and produce structured JSON verdicts.\n\
         Respond ONLY with a JSON object matching this shape:\n\
         {{\n\
           \"decision\": \"allow\" | \"decline\" | \"escalate\" | \"verdict\" | \"need-deeper\",\n\
           \"primary_principle\": \"<constitutional principle>\",\n\
           \"confidence\": <0.0-1.0>,\n\
           \"summary\": \"<reasoning summary>\"\n\
         }}",
        constitution = input.constitution_cid,
        framing = input.framing_cid,
    );

    let user = format!(
        "Event: {event}\n\nContext keys considered: {keys}\n\nContext:\n{ctx}",
        event = input.event_summary,
        keys = input.context_keys.join(", "),
        ctx = input.context_json,
    );

    CompletionRequest::user(user)
        .with_system(system)
        .with_max_tokens(512)
        .with_temperature(0.3)
        .with_json_output()
}

// ─── Response parsing ─────────────────────────────────────────────────────────

/// Attempt to parse a real LLM response into a `WisdomInvocationResponse`.
///
/// Returns `None` if the content cannot be parsed into the expected shape.
pub fn parse_llm_wisdom_response(content: &str) -> Option<WisdomInvocationResponse> {
    let v: serde_json::Value = serde_json::from_str(content).ok()?;

    let decision_str = v.get("decision")?.as_str()?;
    let decision = match decision_str {
        "allow" => WisdomDecision::Allow,
        "decline" => WisdomDecision::Decline,
        "escalate" => WisdomDecision::Escalate,
        "verdict" => WisdomDecision::Verdict,
        "need-deeper" => WisdomDecision::NeedDeeper,
        _ => {
            warn!("WisdomInvoke: unrecognised decision '{}' from LLM", decision_str);
            return None;
        }
    };

    let primary_principle = v
        .get("primary_principle")
        .and_then(|s| s.as_str())
        .unwrap_or("unspecified")
        .to_string();

    let confidence = v
        .get("confidence")
        .and_then(|n| n.as_f64())
        .map(|f| f as f32)
        .unwrap_or(0.5)
        .clamp(0.0, 1.0);

    let summary = v
        .get("summary")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();

    Some(WisdomInvocationResponse {
        decision,
        phase: WisdomPhase::ElohimActive,
        reasoning: WisdomReasoning {
            primary_principle,
            confidence,
            summary,
            phase_note: String::new(),
        },
        side_effects: vec![],
    })
}

/// Return the canonical dev-context stub response.
///
/// Used when: no backend, backend unavailable, call error, or parse failure.
/// Phase is always `DevContext` — honest about the degraded state.
pub fn stub_response(phase_note: impl Into<String>) -> WisdomInvocationResponse {
    WisdomInvocationResponse {
        decision: WisdomDecision::Allow,
        phase: WisdomPhase::DevContext,
        reasoning: WisdomReasoning {
            primary_principle: "dev-context-stub".to_string(),
            confidence: 0.0,
            summary: "No real LLM call was made. Dev-context stub allows by default.".to_string(),
            phase_note: phase_note.into(),
        },
        side_effects: vec![],
    }
}

// ─── Core function ────────────────────────────────────────────────────────────

/// Invoke wisdom with an explicit backend (injectable for testing).
///
/// Phase derivation is **observed**, not flagged:
/// - `backend` is `None` (not configured)   → `DevContext`
/// - `backend` call returns `Err`           → `DevContext` (graceful degradation)
/// - LLM response cannot be parsed          → `DevContext` (honest failure)
/// - LLM call succeeds + parseable          → `ElohimActive`
pub async fn invoke_wisdom_with_backend(
    input: &WisdomInvocationInput,
    backend: Option<&(dyn LlmBackend + Send + Sync)>,
) -> WisdomInvocationResponse {
    let backend = match backend {
        Some(b) => b,
        None => {
            return stub_response("No LLM backend configured (no API key set).");
        }
    };

    if !backend.is_available().await {
        return stub_response(format!(
            "Backend '{}' reported unavailable.",
            backend.id()
        ));
    }

    let request = build_wisdom_prompt(input);
    match backend.complete(request).await {
        Ok(response) => {
            match parse_llm_wisdom_response(&response.content) {
                Some(parsed) => {
                    info!(
                        wisdom.decision = ?parsed.decision,
                        wisdom.confidence = parsed.reasoning.confidence,
                        "WisdomInvoke: elohim-active response received"
                    );
                    parsed
                }
                None => {
                    warn!(
                        content_preview = &response.content[..response.content.len().min(200)],
                        "WisdomInvoke: LLM response could not be parsed — falling back to dev-context"
                    );
                    stub_response("LLM response received but could not be parsed as WisdomDecision JSON.")
                }
            }
        }
        Err(err) => {
            warn!(%err, "WisdomInvoke: backend call failed — falling back to dev-context");
            stub_response(format!("Backend call failed: {err}"))
        }
    }
}

// ─── HTTP handler helper ──────────────────────────────────────────────────────

/// Top-level entry point — selects backend from environment and invokes.
///
/// This function is the target for `POST /wisdom/invoke`.  The HTTP layer
/// calls it with the deserialised input and serialises the response.
///
/// If elohim-agent-service gains an axum HTTP server in a future task, the
/// handler body is:
/// ```rust,ignore
/// async fn handle_wisdom_invoke(
///     Json(input): Json<WisdomInvocationInput>,
/// ) -> Json<WisdomInvocationResponse> {
///     Json(invoke_wisdom(input).await)
/// }
/// ```
pub async fn invoke_wisdom(input: WisdomInvocationInput) -> WisdomInvocationResponse {
    let backend = select_env_backend();
    // invoke_wisdom_with_backend expects Option<&dyn …>, so we borrow through the Box.
    let backend_ref = backend.as_deref();
    invoke_wisdom_with_backend(&input, backend_ref).await
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::MockBackend;
    use std::sync::Arc;

    // Convenience: build a minimal valid input.
    fn test_input() -> WisdomInvocationInput {
        WisdomInvocationInput {
            constitution_cid: "bafyconstitution123".to_string(),
            framing_cid: "bafyframing456".to_string(),
            context_keys: vec!["contentCid".to_string(), "author".to_string()],
            context_json: r#"{"contentCid":"cid-abc","author":"agent-xyz"}"#.to_string(),
            event_summary: "User published content at reach=public".to_string(),
        }
    }

    // ─── Unit: MockBackend → DevContext ───────────────────────────────────────

    #[tokio::test]
    async fn mock_backend_returns_allow_dev_context() {
        // MockBackend returns its configured string.
        // The response is NOT valid wisdom JSON, so we fall back to dev-context.
        let backend = MockBackend::new("mock-model")
            .with_response("Mock response — not JSON wisdom");

        let response = invoke_wisdom_with_backend(&test_input(), Some(&backend)).await;

        assert_eq!(response.phase, WisdomPhase::DevContext);
        assert_eq!(response.decision, WisdomDecision::Allow);
        assert_eq!(response.reasoning.confidence, 0.0);
    }

    // ─── Unit: Simulated real backend → ElohimActive ──────────────────────────

    #[tokio::test]
    async fn real_backend_response_returns_elohim_active() {
        let canned_wisdom_json = r#"{
            "decision": "allow",
            "primary_principle": "subsidiarity",
            "confidence": 0.92,
            "summary": "Content is within declared reach boundaries."
        }"#;

        let backend = MockBackend::new("mock-anthropic").with_response(canned_wisdom_json);

        let response = invoke_wisdom_with_backend(&test_input(), Some(&backend)).await;

        assert_eq!(response.phase, WisdomPhase::ElohimActive);
        assert_eq!(response.decision, WisdomDecision::Allow);
        assert!((response.reasoning.confidence - 0.92).abs() < 0.001);
        assert_eq!(response.reasoning.primary_principle, "subsidiarity");
    }

    // ─── Unit: Simulated error → graceful DevContext fallback ─────────────────

    #[tokio::test]
    async fn unavailable_backend_falls_back_to_dev_context() {
        let backend = MockBackend::new("broken-model").with_available(false);

        let response = invoke_wisdom_with_backend(&test_input(), Some(&backend)).await;

        assert_eq!(response.phase, WisdomPhase::DevContext);
        assert_eq!(response.decision, WisdomDecision::Allow);
        assert!(
            response.reasoning.phase_note.contains("unavailable"),
            "phase_note should mention unavailability, got: {}",
            response.reasoning.phase_note
        );
    }

    // ─── Unit: No backend → DevContext ───────────────────────────────────────

    #[tokio::test]
    async fn no_backend_returns_dev_context() {
        let response = invoke_wisdom_with_backend(&test_input(), None).await;

        assert_eq!(response.phase, WisdomPhase::DevContext);
        assert_eq!(response.decision, WisdomDecision::Allow);
    }

    // ─── Unit: schema validation — WisdomInvocationResponse serialises cleanly ──

    #[test]
    fn response_serialises_to_valid_json() {
        let response = WisdomInvocationResponse {
            decision: WisdomDecision::Allow,
            phase: WisdomPhase::DevContext,
            reasoning: WisdomReasoning {
                primary_principle: "dev-context-stub".to_string(),
                confidence: 0.0,
                summary: "Test serialisation".to_string(),
                phase_note: "unit test".to_string(),
            },
            side_effects: vec![],
        };

        let json = serde_json::to_string(&response).expect("serialisation must succeed");
        let v: serde_json::Value =
            serde_json::from_str(&json).expect("round-trip must produce valid JSON");

        // Verify camelCase field names at the boundary
        assert_eq!(v.get("decision").and_then(|d| d.as_str()), Some("allow"));
        assert_eq!(v.get("phase").and_then(|p| p.as_str()), Some("dev-context"));
        let reasoning = v.get("reasoning").expect("reasoning must be present");
        assert!(
            reasoning.get("primaryPrinciple").is_some(),
            "reasoning.primaryPrinciple must be camelCase"
        );
        assert!(
            reasoning.get("phaseNote").is_some(),
            "reasoning.phaseNote must be camelCase"
        );
        // sideEffects must be absent when empty (skip_serializing_if)
        assert!(
            v.get("sideEffects").is_none(),
            "sideEffects must be absent when empty"
        );
    }

    // ─── Unit: parse_llm_wisdom_response ─────────────────────────────────────

    #[test]
    fn parse_valid_llm_response() {
        let content = r#"{"decision":"decline","primary_principle":"harm-prevention","confidence":0.88,"summary":"Content poses risk."}"#;
        let parsed = parse_llm_wisdom_response(content).expect("must parse");
        assert_eq!(parsed.decision, WisdomDecision::Decline);
        assert_eq!(parsed.phase, WisdomPhase::ElohimActive);
        assert!((parsed.reasoning.confidence - 0.88).abs() < 0.001);
    }

    #[test]
    fn parse_invalid_decision_returns_none() {
        let content = r#"{"decision":"unknown-verdict","primary_principle":"x","confidence":0.5,"summary":"x"}"#;
        assert!(parse_llm_wisdom_response(content).is_none());
    }

    #[test]
    fn parse_malformed_json_returns_none() {
        assert!(parse_llm_wisdom_response("not json at all").is_none());
    }

    // ─── Unit: WisdomDecision kebab-case serialisation ────────────────────────

    #[test]
    fn decision_serialises_kebab_case() {
        let cases = [
            (WisdomDecision::Allow, "allow"),
            (WisdomDecision::Decline, "decline"),
            (WisdomDecision::Escalate, "escalate"),
            (WisdomDecision::Verdict, "verdict"),
            (WisdomDecision::NeedDeeper, "need-deeper"),
        ];
        for (variant, expected) in cases {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, format!("\"{}\"", expected), "variant {:?}", variant);
        }
    }

    // ─── Unit: WisdomPhase kebab-case serialisation ───────────────────────────

    #[test]
    fn phase_serialises_kebab_case() {
        assert_eq!(
            serde_json::to_string(&WisdomPhase::DevContext).unwrap(),
            "\"dev-context\""
        );
        assert_eq!(
            serde_json::to_string(&WisdomPhase::ElohimActive).unwrap(),
            "\"elohim-active\""
        );
    }

    // ─── Unit: LLM error during complete → DevContext ─────────────────────────

    #[tokio::test]
    async fn llm_error_during_complete_returns_dev_context() {
        // An available but response-less backend can simulate parse failure path.
        // We use a valid available backend that returns a non-JSON response
        // to exercise the parse-failure → dev-context branch.
        let backend = MockBackend::new("error-sim")
            .with_response("I cannot comply with that request.");

        let response = invoke_wisdom_with_backend(&test_input(), Some(&backend)).await;

        // Non-JSON response falls through parse → dev-context
        assert_eq!(response.phase, WisdomPhase::DevContext);
    }

    // ─── Unit: need-deeper decision roundtrip ────────────────────────────────

    #[tokio::test]
    async fn real_backend_need_deeper_decision() {
        let canned = r#"{
            "decision": "need-deeper",
            "primary_principle": "epistemic-humility",
            "confidence": 0.45,
            "summary": "Insufficient context to decide."
        }"#;

        let backend = MockBackend::new("mock-model").with_response(canned);
        let response = invoke_wisdom_with_backend(&test_input(), Some(&backend)).await;

        assert_eq!(response.phase, WisdomPhase::ElohimActive);
        assert_eq!(response.decision, WisdomDecision::NeedDeeper);
    }

    // ─── Arc<dyn LlmBackend> compatibility check ─────────────────────────────
    // (Task 8.2 will want to pass an Arc<dyn LlmBackend> through the transport.
    // This confirms invoke_wisdom_with_backend works through a dyn reference.)
    #[tokio::test]
    async fn works_through_arc_dyn_backend() {
        let canned = r#"{"decision":"escalate","primary_principle":"human-in-the-loop","confidence":0.75,"summary":"Requires human review."}"#;
        let backend: Arc<dyn LlmBackend> =
            Arc::new(MockBackend::new("arc-mock").with_response(canned));

        let response = invoke_wisdom_with_backend(&test_input(), Some(backend.as_ref())).await;

        assert_eq!(response.phase, WisdomPhase::ElohimActive);
        assert_eq!(response.decision, WisdomDecision::Escalate);
    }
}
