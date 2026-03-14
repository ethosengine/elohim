# ElohimGate Sprint 2: Inference — The Gate Comes Alive

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Wire the ElohimGate to an InferenceEngine so it actually calls an LLM for Light and Full tiers. Implement the InferenceRouter with local/sidecar routing. Wire the gate into all mutation handlers (not just curation event). The elohim starts thinking.

**Architecture:** `InferenceEngine` trait abstracts LLM calls. `SidecarEngine` implements it by calling the existing elohim-agent-sdk (TypeScript Fastify sidecar at port 8095). `InferenceRouter` picks the best available engine. The gate moves from PassThrough to returning real Enriched/Pause results. Mutation handlers return GateEvaluationView alongside their normal response.

**Tech Stack:** Rust, reqwest (HTTP client for sidecar), tokio (async), constitution crate (prompt assembly)

**Depends on:** Sprint 1 complete — TrustContext, InferenceTier, GateResult, gate skeleton, session intent.

---

### Sprint 1 Feedback (incorporated below)

1. **Services dispatch pattern**: `handle_api_request` already receives `services: Option<Arc<Services>>`. Sprint 1 proved this works for steward_affinity. Task 6 follows the same pattern for all handlers.
2. **TrustContext construction**: Sprint 1 used direct struct construction (not `compute()`) for the placeholder. Task 5 should use `compute()` with real signal gathering, or keep direct construction with real values queried from DB.
3. **Session routes vs API routes**: Session endpoints live at `/session/*` (in http.rs), not `/api/v1/session/*`. API endpoints go through `api/mod.rs` dispatch.
4. **Test DB helpers**: Any new schema changes require updating in-memory test DB setup helpers (e.g., `setup_test_db()` in local_sessions.rs).
5. **`evaluate()` is sync**: Sprint 1's `ElohimGate::evaluate()` is synchronous. Sprint 2 must make it async since inference calls are async. This ripples into handlers.
6. **Constitution crate is real**: `PromptAssembler` exists with `build_system_prompt(stack)`, `build_reasoning_prompt(stack, query)`, `build_layer_prompt(layer)`. `ConstitutionalStack::build_defaults(StackContext)` constructs the full constitutional stack.

### Sidecar SDK Interface Reference

The elohim-agent-sdk is a **TypeScript Fastify** server at `localhost:8095`.

**POST /invoke** — `InvokeRequest`:
```json
{
  "requestId": "uuid",
  "elohimId": "string",
  "capability": "kebab-case-capability",
  "params": { "mutation_type": "...", "content": "...", "trust_context": "..." },
  "requesterId": "agent-id",
  "priority": "normal"
}
```

**Response** — `ElohimResponse`:
```json
{
  "requestId": "uuid",
  "elohimId": "string",
  "status": "fulfilled|declined|deferred|escalated",
  "constitutionalReasoning": {
    "primaryPrinciple": "string",
    "interpretation": "string",
    "valuesWeighed": [{ "value": "...", "weight": 0.9, "direction": "for|against" }],
    "confidence": 0.85,
    "precedents": [],
    "newPrecedent": false
  },
  "payload": {},
  "declineReason": "optional",
  "respondedAt": "ISO-8601",
  "cost": { "tokensProcessed": 100, "timeMs": 500, "constitutionalChecks": 1, "precedentLookups": 0 }
}
```

**GET /health** — returns `{ "status": "ok", "budgetRemaining": N, "model": "claude-haiku-4-5-20251001" }`.

---

### Task 1: InferenceEngine trait + MockEngine

**Files:**
- Create: `elohim/elohim-storage/src/services/inference_engine.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

**Dependencies needed in Cargo.toml:**
- `async-trait = "0.1"` (for async trait methods)
- `reqwest = { version = "0.12", features = ["json"], default-features = false, features = ["rustls-tls"] }` (for HTTP client in Task 2)

Add both dependencies now to avoid recompile churn.

**Step 1: Write the trait and supporting types**

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::services::elohim_gate::{
    ElohimReasoning, InferenceTier, MutationType, ObservationDraft, TrustContext,
};

/// Error type for inference operations.
#[derive(Debug, thiserror::Error)]
pub enum InferenceError {
    #[error("Engine unavailable: {0}")]
    Unavailable(String),
    #[error("Request failed: {0}")]
    RequestFailed(String),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Budget exhausted")]
    BudgetExhausted,
}

/// What the inference engine recommends.
#[derive(Debug, Clone)]
pub enum RecommendedAction {
    Proceed,
    ProceedWithReachAdjustment(String),
    PauseForConfirmation(String),
    Settle(String),
}

/// Request sent to an inference engine.
#[derive(Debug, Clone)]
pub struct InferenceRequest {
    pub tier: InferenceTier,
    pub mutation_type: MutationType,
    pub trust_context: TrustContext,
    pub mutation_content: serde_json::Value,
    pub constitutional_prompt: String,
}

/// Result returned from inference.
#[derive(Debug, Clone)]
pub struct InferenceResult {
    pub reasoning: ElohimReasoning,
    pub recommended_action: RecommendedAction,
    pub observations: Vec<ObservationDraft>,
}

/// Trait for any inference provider (sidecar, local, steward-node).
#[async_trait]
pub trait InferenceEngine: Send + Sync {
    /// Evaluate a mutation and return reasoning.
    async fn evaluate(&self, request: InferenceRequest) -> Result<InferenceResult, InferenceError>;

    /// Check if this engine is currently available.
    async fn is_available(&self) -> bool;

    /// Human-readable name for logging.
    fn name(&self) -> &str;
}
```

**Step 2: Write a MockEngine for testing**

```rust
/// Mock engine that returns configurable results (for testing).
pub struct MockEngine {
    pub available: bool,
    pub default_action: RecommendedAction,
}

impl MockEngine {
    pub fn always_proceed() -> Self {
        Self {
            available: true,
            default_action: RecommendedAction::Proceed,
        }
    }

    pub fn unavailable() -> Self {
        Self {
            available: false,
            default_action: RecommendedAction::Proceed,
        }
    }
}

#[async_trait]
impl InferenceEngine for MockEngine {
    async fn evaluate(&self, request: InferenceRequest) -> Result<InferenceResult, InferenceError> {
        if !self.available {
            return Err(InferenceError::Unavailable("Mock engine disabled".into()));
        }
        Ok(InferenceResult {
            reasoning: ElohimReasoning {
                primary_principle: "Human Dignity".to_string(),
                interpretation: "Mock evaluation — no real inference".to_string(),
                confidence: 1.0,
            },
            recommended_action: self.default_action.clone(),
            observations: vec![],
        })
    }

    async fn is_available(&self) -> bool {
        self.available
    }

    fn name(&self) -> &str {
        "mock"
    }
}
```

**Step 3: Register module and add dependencies**

In `services/mod.rs`:
```rust
pub mod inference_engine;
```

In `Cargo.toml` under `[dependencies]`:
```toml
async-trait = "0.1"
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
```

Note: Check if `thiserror` is already a dependency. If not, add it too.

**Step 4: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_engine_proceed() {
        let engine = MockEngine::always_proceed();
        assert!(engine.is_available().await);

        let request = InferenceRequest {
            tier: InferenceTier::Light,
            mutation_type: MutationType::Comment,
            trust_context: TrustContext::compute(crate::services::elohim_gate::TrustSignals {
                mastery_depth: 0.5, steward_standing: 0.5,
                relationship_density: 0.5, governance_health: 0.5,
                behavioral_trust: 0.5, intent_divergence: 0.0,
            }),
            mutation_content: serde_json::json!({"text": "test comment"}),
            constitutional_prompt: "test prompt".to_string(),
        };

        let result = engine.evaluate(request).await.unwrap();
        assert!(matches!(result.recommended_action, RecommendedAction::Proceed));
        assert_eq!(result.reasoning.confidence, 1.0);
    }

    #[tokio::test]
    async fn mock_engine_unavailable() {
        let engine = MockEngine::unavailable();
        assert!(!engine.is_available().await);

        let request = InferenceRequest {
            tier: InferenceTier::Light,
            mutation_type: MutationType::Comment,
            trust_context: TrustContext::compute(crate::services::elohim_gate::TrustSignals {
                mastery_depth: 0.5, steward_standing: 0.5,
                relationship_density: 0.5, governance_health: 0.5,
                behavioral_trust: 0.5, intent_divergence: 0.0,
            }),
            mutation_content: serde_json::json!({}),
            constitutional_prompt: String::new(),
        };

        let result = engine.evaluate(request).await;
        assert!(matches!(result, Err(InferenceError::Unavailable(_))));
    }
}
```

**Step 5: Verify**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test inference_engine 2>&1 | tail -10`
Expected: 2 tests PASS

**Step 6: Commit**

```bash
git add elohim/elohim-storage/
git commit -m "feat(gate): InferenceEngine trait + MockEngine with async evaluation"
```

---

### Task 2: SidecarEngine — wraps existing elohim-agent-sdk

**Files:**
- Create: `elohim/elohim-storage/src/services/sidecar_engine.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

The sidecar is a TypeScript Fastify server at `localhost:8095`. We call `POST /invoke` with an `InvokeRequest` and get back an `ElohimResponse`.

**Step 1: Define sidecar request/response types**

```rust
use serde::{Deserialize, Serialize};

/// Maps to the SDK's InvokeRequest (TypeScript).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SidecarInvokeRequest {
    request_id: String,
    elohim_id: String,
    capability: String,
    params: serde_json::Value,
    requester_id: String,
    priority: String,
}

/// Maps to the SDK's ElohimResponse (TypeScript).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SidecarResponse {
    request_id: String,
    status: String,  // "fulfilled" | "declined" | "deferred" | "escalated"
    constitutional_reasoning: Option<ConstitutionalReasoningResponse>,
    payload: Option<serde_json::Value>,
    decline_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConstitutionalReasoningResponse {
    primary_principle: String,
    interpretation: String,
    confidence: f64,
}

/// Maps to the SDK's HealthResponse.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SidecarHealthResponse {
    status: String,
    budget_remaining: Option<i32>,
}
```

**Step 2: Implement SidecarEngine**

```rust
use reqwest::Client;
use std::time::Duration;

pub struct SidecarEngine {
    client: Client,
    base_url: String,
    elohim_id: String,
}

impl SidecarEngine {
    pub fn new(base_url: String, elohim_id: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        Self { client, base_url, elohim_id }
    }

    pub fn default_local() -> Self {
        Self::new(
            "http://localhost:8095".to_string(),
            "gate-evaluator".to_string(),
        )
    }

    /// Translate InferenceRequest → SidecarInvokeRequest
    fn build_sidecar_request(&self, request: &InferenceRequest) -> SidecarInvokeRequest {
        SidecarInvokeRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            elohim_id: self.elohim_id.clone(),
            capability: "gate-evaluation".to_string(),
            params: serde_json::json!({
                "tier": format!("{:?}", request.tier),
                "mutationType": format!("{:?}", request.mutation_type),
                "compositeTrust": request.trust_context.composite_trust,
                "mutationContent": request.mutation_content,
                "constitutionalPrompt": request.constitutional_prompt,
                "declaredIntent": request.trust_context.declared_intent,
            }),
            requester_id: request.trust_context.human_id.clone(),
            priority: match request.tier {
                InferenceTier::Constitutional => "urgent",
                InferenceTier::Full => "high",
                InferenceTier::Light => "normal",
                InferenceTier::None => "low",
            }.to_string(),
        }
    }

    /// Translate SidecarResponse → InferenceResult
    fn parse_sidecar_response(&self, resp: SidecarResponse) -> Result<InferenceResult, InferenceError> {
        let reasoning = resp.constitutional_reasoning
            .map(|cr| ElohimReasoning {
                primary_principle: cr.primary_principle,
                interpretation: cr.interpretation,
                confidence: cr.confidence,
            })
            .unwrap_or(ElohimReasoning {
                primary_principle: "Unknown".to_string(),
                interpretation: "No constitutional reasoning provided".to_string(),
                confidence: 0.0,
            });

        let recommended_action = match resp.status.as_str() {
            "fulfilled" => {
                // Check payload for specific recommendations
                if let Some(ref payload) = resp.payload {
                    if let Some(reach) = payload.get("adjustedReach").and_then(|v| v.as_str()) {
                        RecommendedAction::ProceedWithReachAdjustment(reach.to_string())
                    } else if let Some(prompt) = payload.get("pausePrompt").and_then(|v| v.as_str()) {
                        RecommendedAction::PauseForConfirmation(prompt.to_string())
                    } else {
                        RecommendedAction::Proceed
                    }
                } else {
                    RecommendedAction::Proceed
                }
            }
            "declined" => {
                let reason = resp.decline_reason.unwrap_or_else(|| "Constitutional boundary".to_string());
                RecommendedAction::Settle(reason)
            }
            "escalated" => RecommendedAction::PauseForConfirmation(
                "This action requires additional review.".to_string()
            ),
            _ => RecommendedAction::Proceed,
        };

        Ok(InferenceResult {
            reasoning,
            recommended_action,
            observations: vec![], // Sprint 3: populate from response
        })
    }
}
```

**Step 3: Implement InferenceEngine trait**

```rust
#[async_trait]
impl InferenceEngine for SidecarEngine {
    async fn evaluate(&self, request: InferenceRequest) -> Result<InferenceResult, InferenceError> {
        let sidecar_req = self.build_sidecar_request(&request);
        let url = format!("{}/invoke", self.base_url);

        let http_response = self.client.post(&url)
            .json(&sidecar_req)
            .send()
            .await
            .map_err(|e| InferenceError::RequestFailed(e.to_string()))?;

        if http_response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(InferenceError::BudgetExhausted);
        }

        if !http_response.status().is_success() {
            return Err(InferenceError::RequestFailed(
                format!("Sidecar returned {}", http_response.status())
            ));
        }

        let sidecar_resp: SidecarResponse = http_response.json()
            .await
            .map_err(|e| InferenceError::InvalidResponse(e.to_string()))?;

        self.parse_sidecar_response(sidecar_resp)
    }

    async fn is_available(&self) -> bool {
        let url = format!("{}/health", self.base_url);
        match self.client.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    fn name(&self) -> &str {
        "sidecar"
    }
}
```

**Step 4: Tests** (unit tests with no real HTTP — test translation logic only)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translate_fulfilled_response() {
        let engine = SidecarEngine::default_local();
        let resp = SidecarResponse {
            request_id: "test".to_string(),
            status: "fulfilled".to_string(),
            constitutional_reasoning: Some(ConstitutionalReasoningResponse {
                primary_principle: "Human Dignity".to_string(),
                interpretation: "Comment is respectful".to_string(),
                confidence: 0.9,
            }),
            payload: None,
            decline_reason: None,
        };
        let result = engine.parse_sidecar_response(resp).unwrap();
        assert!(matches!(result.recommended_action, RecommendedAction::Proceed));
        assert_eq!(result.reasoning.confidence, 0.9);
    }

    #[test]
    fn translate_declined_response() {
        let engine = SidecarEngine::default_local();
        let resp = SidecarResponse {
            request_id: "test".to_string(),
            status: "declined".to_string(),
            constitutional_reasoning: Some(ConstitutionalReasoningResponse {
                primary_principle: "Child Protection".to_string(),
                interpretation: "Content inappropriate".to_string(),
                confidence: 0.95,
            }),
            payload: None,
            decline_reason: Some("Violates child protection boundary".to_string()),
        };
        let result = engine.parse_sidecar_response(resp).unwrap();
        assert!(matches!(result.recommended_action, RecommendedAction::Settle(_)));
    }

    #[test]
    fn translate_reach_adjustment() {
        let engine = SidecarEngine::default_local();
        let resp = SidecarResponse {
            request_id: "test".to_string(),
            status: "fulfilled".to_string(),
            constitutional_reasoning: Some(ConstitutionalReasoningResponse {
                primary_principle: "Subsidiarity".to_string(),
                interpretation: "Limit reach to community".to_string(),
                confidence: 0.8,
            }),
            payload: Some(serde_json::json!({ "adjustedReach": "community" })),
            decline_reason: None,
        };
        let result = engine.parse_sidecar_response(resp).unwrap();
        assert!(matches!(result.recommended_action, RecommendedAction::ProceedWithReachAdjustment(_)));
    }
}
```

**Step 5: Register module**

In `services/mod.rs`:
```rust
pub mod sidecar_engine;
```

**Step 6: Verify and commit**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test sidecar_engine 2>&1 | tail -10`

```bash
git add elohim/elohim-storage/
git commit -m "feat(gate): SidecarEngine — HTTP client wrapping elohim-agent-sdk"
```

**Dependencies note:** Also add `uuid = { version = "1", features = ["v4"] }` to Cargo.toml if not already present.

---

### Task 3: InferenceRouter

**Files:**
- Create: `elohim/elohim-storage/src/services/inference_router.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

**Step 1: Implement the router**

```rust
use std::sync::Arc;

use super::inference_engine::{InferenceEngine, InferenceError, InferenceRequest, InferenceResult};

/// Routes inference requests to the best available engine.
/// Priority: local > sidecar > fallback (escalate tier).
pub struct InferenceRouter {
    engines: Vec<Arc<dyn InferenceEngine>>,
}

impl InferenceRouter {
    pub fn new(engines: Vec<Arc<dyn InferenceEngine>>) -> Self {
        Self { engines }
    }

    /// Route request to first available engine.
    /// Returns InferenceError::Unavailable if no engine is available.
    pub async fn route(&self, request: InferenceRequest) -> Result<InferenceResult, InferenceError> {
        for engine in &self.engines {
            if engine.is_available().await {
                tracing::info!(engine = engine.name(), "Routing inference to engine");
                match engine.evaluate(request.clone()).await {
                    Ok(result) => return Ok(result),
                    Err(InferenceError::BudgetExhausted) => {
                        tracing::warn!(engine = engine.name(), "Budget exhausted, trying next engine");
                        continue;
                    }
                    Err(e) => {
                        tracing::warn!(engine = engine.name(), error = %e, "Engine failed, trying next");
                        continue;
                    }
                }
            }
        }

        Err(InferenceError::Unavailable(
            "No inference engine available".to_string(),
        ))
    }
}
```

**Step 2: Tests using MockEngine**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::inference_engine::{MockEngine, RecommendedAction};

    #[tokio::test]
    async fn routes_to_first_available() {
        let router = InferenceRouter::new(vec![
            Arc::new(MockEngine::always_proceed()),
        ]);
        let request = test_request();
        let result = router.route(request).await.unwrap();
        assert!(matches!(result.recommended_action, RecommendedAction::Proceed));
    }

    #[tokio::test]
    async fn skips_unavailable_engines() {
        let router = InferenceRouter::new(vec![
            Arc::new(MockEngine::unavailable()),
            Arc::new(MockEngine::always_proceed()),
        ]);
        let request = test_request();
        let result = router.route(request).await.unwrap();
        assert!(matches!(result.recommended_action, RecommendedAction::Proceed));
    }

    #[tokio::test]
    async fn all_unavailable_returns_error() {
        let router = InferenceRouter::new(vec![
            Arc::new(MockEngine::unavailable()),
        ]);
        let request = test_request();
        let result = router.route(request).await;
        assert!(matches!(result, Err(InferenceError::Unavailable(_))));
    }

    fn test_request() -> InferenceRequest {
        InferenceRequest {
            tier: crate::services::elohim_gate::InferenceTier::Light,
            mutation_type: crate::services::elohim_gate::MutationType::Comment,
            trust_context: crate::services::elohim_gate::TrustContext::compute(
                crate::services::elohim_gate::TrustSignals {
                    mastery_depth: 0.5, steward_standing: 0.5,
                    relationship_density: 0.5, governance_health: 0.5,
                    behavioral_trust: 0.5, intent_divergence: 0.0,
                }
            ),
            mutation_content: serde_json::json!({}),
            constitutional_prompt: String::new(),
        }
    }
}
```

**Step 3: Register and verify**

```bash
git add elohim/elohim-storage/
git commit -m "feat(gate): InferenceRouter — priority-based engine routing with fallback"
```

---

### Task 4: Constitutional prompt assembly for gate context

**Files:**
- Modify: `elohim/elohim-storage/src/services/elohim_gate.rs`

Build context-aware prompts using the `constitution` crate. The crate provides:
- `ConstitutionalStack::build_defaults(StackContext)` — builds the full constitutional stack
- `PromptAssembler::build_system_prompt(&stack)` — builds a system prompt
- `PromptAssembler::build_reasoning_prompt(&stack, query)` — builds a reasoning prompt
- `StackContext::agent_only(id).with_family(id).with_community(id)` — context builder

**Step 1: Add prompt building to ElohimGate**

```rust
use constitution::{ConstitutionalStack, PromptAssembler, StackContext};

impl ElohimGate {
    /// Build constitutional prompt for a given trust context and mutation.
    pub fn build_constitutional_prompt(
        &self,
        ctx: &TrustContext,
        mutation: MutationType,
        tier: InferenceTier,
    ) -> String {
        let mut stack_ctx = StackContext::agent_only(&ctx.human_id);
        if let Some(ref family_id) = ctx.family_id {
            stack_ctx = stack_ctx.with_family(family_id);
        }
        if let Some(ref community_id) = ctx.community_id {
            stack_ctx = stack_ctx.with_community(community_id);
        }

        let stack = ConstitutionalStack::build_defaults(stack_ctx);

        let query = format!(
            "Evaluate {:?} mutation (tier: {:?}, composite_trust: {:.2}). {}",
            mutation,
            tier,
            ctx.composite_trust,
            ctx.declared_intent.as_deref().unwrap_or("No declared intent."),
        );

        match tier {
            InferenceTier::Constitutional => PromptAssembler::build_reasoning_prompt(&stack, &query),
            _ => PromptAssembler::build_system_prompt(&stack),
        }
    }
}
```

**Step 2: Tests**

```rust
#[test]
fn prompt_assembly_light_tier() {
    let gate = ElohimGate::new_skeleton();
    let ctx = TrustContext::compute(TrustSignals {
        mastery_depth: 0.8, steward_standing: 0.7,
        relationship_density: 0.9, governance_health: 1.0,
        behavioral_trust: 0.85, intent_divergence: 0.0,
    });
    let prompt = gate.build_constitutional_prompt(&ctx, MutationType::Comment, InferenceTier::Light);
    assert!(!prompt.is_empty());
    // Light tier gets system prompt (principles only)
}

#[test]
fn prompt_assembly_constitutional_tier() {
    let gate = ElohimGate::new_skeleton();
    let ctx = TrustContext::compute(TrustSignals {
        mastery_depth: 0.5, steward_standing: 0.5,
        relationship_density: 0.5, governance_health: 0.5,
        behavioral_trust: 0.5, intent_divergence: 0.0,
    });
    let prompt = gate.build_constitutional_prompt(
        &ctx, MutationType::ReachChange, InferenceTier::Constitutional
    );
    assert!(!prompt.is_empty());
    // Constitutional tier gets reasoning prompt (includes the query)
    assert!(prompt.contains("Evaluate"));
}
```

**Step 3: Commit**

```bash
git commit -m "feat(gate): constitutional prompt assembly using constitution crate"
```

---

### Task 5: Gate evaluate() calls InferenceRouter

**Files:**
- Modify: `elohim/elohim-storage/src/services/elohim_gate.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

This is the core change — the gate becomes async and calls real inference.

**Step 1: Make ElohimGate hold an optional InferenceRouter**

```rust
use std::sync::Arc;
use super::inference_router::InferenceRouter;

pub struct ElohimGate {
    router: Option<Arc<InferenceRouter>>,
}

impl ElohimGate {
    /// Create a skeleton gate with no inference capability (Sprint 1 fallback).
    pub fn new_skeleton() -> Self {
        Self { router: None }
    }

    /// Create a gate with an inference router.
    pub fn new(router: Arc<InferenceRouter>) -> Self {
        Self { router: Some(router) }
    }
}
```

**Step 2: Make evaluate() async**

```rust
impl ElohimGate {
    /// Evaluate a mutation against trust context.
    /// Routes to inference for Light/Full/Constitutional tiers.
    /// Falls back to PassThrough if no inference available.
    pub async fn evaluate(
        &self,
        mutation: MutationType,
        ctx: &TrustContext,
        mutation_content: serde_json::Value,
    ) -> GateResult {
        let tier = InferenceTier::classify(mutation, ctx);

        // None tier: always pass through
        if tier == InferenceTier::None {
            return GateResult::PassThrough { tier };
        }

        // No router: fall back to PassThrough
        let router = match &self.router {
            Some(r) => r,
            None => return GateResult::PassThrough { tier },
        };

        // Build prompt and request
        let prompt = self.build_constitutional_prompt(ctx, mutation, tier);
        let request = InferenceRequest {
            tier,
            mutation_type: mutation,
            trust_context: ctx.clone(),
            mutation_content,
            constitutional_prompt: prompt,
        };

        // Route to inference
        match router.route(request).await {
            Ok(result) => self.map_inference_result(tier, result),
            Err(e) => {
                tracing::warn!(error = %e, "Inference failed, falling back to PassThrough");
                GateResult::PassThrough { tier }
            }
        }
    }

    /// Map InferenceResult → GateResult based on RecommendedAction
    fn map_inference_result(&self, tier: InferenceTier, result: InferenceResult) -> GateResult {
        match result.recommended_action {
            RecommendedAction::Proceed => GateResult::Enriched {
                tier,
                reasoning: result.reasoning,
                adjusted_reach: None,
                observations: result.observations,
                session_intent_note: None,
            },
            RecommendedAction::ProceedWithReachAdjustment(reach) => GateResult::Enriched {
                tier,
                reasoning: result.reasoning,
                adjusted_reach: Some(reach),
                observations: result.observations,
                session_intent_note: None,
            },
            RecommendedAction::PauseForConfirmation(prompt) => GateResult::Pause {
                tier,
                reasoning: result.reasoning,
                prompt,
                confirm_token: uuid::Uuid::new_v4().to_string(),
            },
            RecommendedAction::Settle(boundary) => GateResult::Settlement {
                tier,
                reasoning: result.reasoning,
                boundary,
                appeal_path: Some("/api/v1/governance/appeal".to_string()),
            },
        }
    }
}
```

**Step 3: Update the `evaluate()` call signature everywhere**

The Sprint 1 curation handler called `gate.evaluate(MutationType::CurationEvent, &trust_ctx)`. Now it needs:
```rust
let gate_result = svc.gate.evaluate(
    MutationType::CurationEvent,
    &trust_ctx,
    serde_json::json!({ "stewardId": input.steward_id, "contentId": input.content_id }),
).await;
```

Update `steward_affinity.rs` to use the new signature. The handler is already async so adding `.await` is straightforward.

**Step 4: Update Services struct to use InferenceRouter**

```rust
// In Services::new():
// Create the router with a sidecar engine (if configured)
let sidecar_url = std::env::var("ELOHIM_AGENT_URL")
    .unwrap_or_else(|_| "http://localhost:8095".to_string());
let sidecar = Arc::new(SidecarEngine::new(sidecar_url, "gate-evaluator".to_string()));
let router = Arc::new(InferenceRouter::new(vec![sidecar as Arc<dyn InferenceEngine>]));
let gate = Arc::new(ElohimGate::new(router));
```

**Step 5: Tests**

```rust
#[tokio::test]
async fn gate_with_mock_returns_enriched() {
    let engine = Arc::new(MockEngine::always_proceed());
    let router = Arc::new(InferenceRouter::new(vec![engine]));
    let gate = ElohimGate::new(router);

    let ctx = TrustContext::compute(TrustSignals {
        mastery_depth: 0.8, steward_standing: 0.7,
        relationship_density: 0.9, governance_health: 1.0,
        behavioral_trust: 0.85, intent_divergence: 0.0,
    });
    let result = gate.evaluate(
        MutationType::Comment, &ctx, serde_json::json!({})
    ).await;
    assert!(matches!(result, GateResult::Enriched { .. }));
}

#[tokio::test]
async fn gate_without_router_returns_passthrough() {
    let gate = ElohimGate::new_skeleton();
    let ctx = TrustContext::compute(TrustSignals {
        mastery_depth: 0.5, steward_standing: 0.5,
        relationship_density: 0.5, governance_health: 0.5,
        behavioral_trust: 0.5, intent_divergence: 0.0,
    });
    let result = gate.evaluate(
        MutationType::Comment, &ctx, serde_json::json!({})
    ).await;
    assert!(matches!(result, GateResult::PassThrough { .. }));
}

#[tokio::test]
async fn gate_none_tier_always_passthrough() {
    let engine = Arc::new(MockEngine::always_proceed());
    let router = Arc::new(InferenceRouter::new(vec![engine]));
    let gate = ElohimGate::new(router);

    let ctx = TrustContext::compute(TrustSignals {
        mastery_depth: 0.5, steward_standing: 0.5,
        relationship_density: 0.5, governance_health: 0.5,
        behavioral_trust: 0.5, intent_divergence: 0.0,
    });
    // MasteryUpdate is always None tier
    let result = gate.evaluate(
        MutationType::MasteryUpdate, &ctx, serde_json::json!({})
    ).await;
    assert!(matches!(result, GateResult::PassThrough { .. }));
}
```

**Step 6: Commit**

```bash
git commit -m "feat(gate): evaluate() calls InferenceRouter — the gate comes alive"
```

---

### Task 6: Wire gate into all mutation handlers

**Files:**
- Modify: `elohim/elohim-storage/src/api/mod.rs`
- Modify: `elohim/elohim-storage/src/api/steward_affinity.rs` (upgrade Sprint 1 wiring)
- Modify: `elohim/elohim-storage/src/api/stewardship.rs`
- Modify: `elohim/elohim-storage/src/api/recognition.rs`
- Modify: `elohim/elohim-storage/src/api/governance.rs`
- Modify: `elohim/elohim-storage/src/http.rs` (content create/update)

**Sprint 1 learned pattern:** Pass `services: Option<Arc<Services>>` to each handler. The `services` parameter is already passed into `handle_api_request` at `api/mod.rs:52`.

**Step 1: Create a gate evaluation helper**

In `api/mod.rs`, add a helper function that all handlers can call:

```rust
use crate::services::elohim_gate::{ElohimGate, GateResult, MutationType, TrustContext, TrustSignals};
use crate::views::GateEvaluationView;

/// Evaluate a mutation through the ElohimGate.
/// Returns (GateResult, Option<GateEvaluationView>) for inclusion in response.
/// If services unavailable, returns PassThrough.
pub async fn evaluate_gate(
    services: &Option<Arc<Services>>,
    mutation: MutationType,
    mutation_content: serde_json::Value,
) -> (GateResult, Option<GateEvaluationView>) {
    let Some(svc) = services else {
        let tier = crate::services::elohim_gate::InferenceTier::None;
        return (GateResult::PassThrough { tier }, None);
    };

    // Sprint 2: placeholder TrustContext — Sprint 3 gathers real signals
    let trust_ctx = TrustContext::compute(TrustSignals {
        mastery_depth: 0.5,
        steward_standing: 0.5,
        relationship_density: 0.5,
        governance_health: 0.5,
        behavioral_trust: 0.5,
        intent_divergence: 0.0,
    });

    let result = svc.gate.evaluate(mutation, &trust_ctx, mutation_content).await;
    let view = build_gate_view(&result, &trust_ctx);
    (result, Some(view))
}
```

**Step 2: Wire into each handler that performs mutations**

For each POST/PUT/DELETE handler, add:
```rust
let (gate_result, gate_view) = evaluate_gate(&services, MutationType::X, content_json).await;
match gate_result {
    GateResult::PassThrough { .. } | GateResult::Enriched { .. } => {
        // Proceed with mutation, include gate_view in response
    }
    GateResult::Pause { prompt, confirm_token, .. } => {
        return Ok(response::ok(&serde_json::json!({
            "gate": gate_view,
            "pausePrompt": prompt,
            "confirmToken": confirm_token,
        })));
    }
    GateResult::Settlement { boundary, appeal_path, .. } => {
        return Ok(response::forbidden(&serde_json::json!({
            "gate": gate_view,
            "boundary": boundary,
            "appealPath": appeal_path,
        })));
    }
}
```

**Handlers to wire** (pass `services` through dispatch chain like Sprint 1 steward_affinity pattern):
- `stewardship.rs` — allocation create/update/delete
- `recognition.rs` — distribute
- `governance.rs` — dispute filing if POST routes exist
- `steward_affinity.rs` — upgrade existing wiring to use new async evaluate + GateEvaluationView
- Content create/update in `http.rs` if they exist as mutations (check `/db/content` POST routes)

**Step 3: Update `api/mod.rs` dispatch to pass services to all controllers**

Follow the Sprint 1 pattern: add `services` parameter to each handler's `handle()` signature, update dispatch.

**Step 4: Verify and commit**

```bash
git commit -m "feat(gate): wire ElohimGate into all mutation handlers with GateEvaluationView"
```

---

### Task 7: TrustContext cache with session-scoped lifecycle

**Files:**
- Modify: `elohim/elohim-storage/src/services/elohim_gate.rs`

**Step 1: Add a TrustContextCache**

```rust
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

struct CacheEntry {
    context: TrustContext,
    cached_at: Instant,
}

pub struct TrustContextCache {
    entries: RwLock<HashMap<String, CacheEntry>>,
    ttl: Duration,
}

impl TrustContextCache {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    pub fn get(&self, session_id: &str) -> Option<TrustContext> {
        let entries = self.entries.read().ok()?;
        let entry = entries.get(session_id)?;
        if entry.cached_at.elapsed() < self.ttl {
            Some(entry.context.clone())
        } else {
            None
        }
    }

    pub fn set(&self, session_id: &str, context: TrustContext) {
        if let Ok(mut entries) = self.entries.write() {
            entries.insert(session_id.to_string(), CacheEntry {
                context,
                cached_at: Instant::now(),
            });
        }
    }

    pub fn invalidate(&self, session_id: &str) {
        if let Ok(mut entries) = self.entries.write() {
            entries.remove(session_id);
        }
    }

    pub fn invalidate_all(&self) {
        if let Ok(mut entries) = self.entries.write() {
            entries.clear();
        }
    }
}
```

**Step 2: Add cache to ElohimGate**

```rust
pub struct ElohimGate {
    router: Option<Arc<InferenceRouter>>,
    trust_cache: TrustContextCache,
}
```

**Step 3: Tests**

```rust
#[test]
fn cache_hit_returns_context() {
    let cache = TrustContextCache::new(300); // 5 min TTL
    let ctx = TrustContext::compute(TrustSignals { /* ... */ });
    cache.set("session-1", ctx.clone());
    let cached = cache.get("session-1");
    assert!(cached.is_some());
    assert_eq!(cached.unwrap().composite_trust, ctx.composite_trust);
}

#[test]
fn cache_miss_returns_none() {
    let cache = TrustContextCache::new(300);
    assert!(cache.get("nonexistent").is_none());
}

#[test]
fn invalidation_removes_entry() {
    let cache = TrustContextCache::new(300);
    let ctx = TrustContext::compute(TrustSignals { /* ... */ });
    cache.set("session-1", ctx);
    cache.invalidate("session-1");
    assert!(cache.get("session-1").is_none());
}
```

**Step 4: Commit**

```bash
git commit -m "feat(gate): TrustContext cache with TTL and session-scoped invalidation"
```

---

### Task 8: Sprint 2 integration verification

**Step 1: Run full test suite**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test 2>&1 | tail -20
```

**Step 2: Run clippy**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings 2>&1 | tail -10
```

**Step 3: Run fmt**

```bash
cd elohim/elohim-storage && cargo fmt --check 2>&1
```

**Step 4: Final commit if fixes needed**

---

## Sprint 2 Deliverables

After this sprint:
- [ ] InferenceEngine trait + MockEngine + SidecarEngine
- [ ] InferenceRouter with priority-based routing and graceful fallback
- [ ] Gate calls real inference for Light/Full/Constitutional tiers
- [ ] Constitutional prompt assembly using constitution crate's PromptAssembler
- [ ] All mutation handlers wired through the gate with GateEvaluationView
- [ ] Pause returns prompt + confirm_token, Settlement returns boundary + appeal_path
- [ ] TrustContext cached per-session with TTL and invalidation
- [ ] Graceful fallback when inference unavailable (PassThrough)

## What Sprint 3 Builds On

Sprint 3 adds the ImagodeiSubconscious — observations are stored, behavioral trust is computed from history, and the streaming UX delivers the elohim's thinking process to the client.
