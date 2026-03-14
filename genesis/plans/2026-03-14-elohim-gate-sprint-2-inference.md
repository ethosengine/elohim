# ElohimGate Sprint 2: Inference — The Gate Comes Alive

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Wire the ElohimGate to an InferenceEngine so it actually calls an LLM for Light and Full tiers. Implement the InferenceRouter with local/sidecar routing. Wire the gate into all mutation handlers (not just curation event). The elohim starts thinking.

**Architecture:** `InferenceEngine` trait abstracts LLM calls. `SidecarEngine` implements it by calling the existing elohim-agent-sdk. `InferenceRouter` picks the best available engine. The gate moves from PassThrough to returning real Enriched/Pause results. Mutation handlers return GateEvaluationView alongside their normal response.

**Tech Stack:** Rust, hyper (HTTP client for sidecar), tokio (async streaming), constitution crate (prompt assembly)

**Depends on:** Sprint 1 complete — TrustContext, InferenceTier, GateResult, gate skeleton, session intent.

**NOTE:** This plan will be updated with Sprint 1 feedback before execution.

---

### Task 1: InferenceEngine trait

**Files:**
- Create: `elohim/elohim-storage/src/services/inference_engine.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

Define the trait that any inference provider implements:

```rust
#[async_trait::async_trait]
pub trait InferenceEngine: Send + Sync {
    /// Evaluate a mutation and return reasoning.
    async fn evaluate(
        &self,
        request: InferenceRequest,
    ) -> Result<InferenceResult, InferenceError>;

    /// Check if this engine is available.
    async fn is_available(&self) -> bool;
}

pub struct InferenceRequest {
    pub tier: InferenceTier,
    pub mutation_type: MutationType,
    pub trust_context: TrustContext,
    pub mutation_content: serde_json::Value,
    pub constitutional_prompt: String,
}

pub struct InferenceResult {
    pub reasoning: ElohimReasoning,
    pub recommended_action: RecommendedAction,
    pub observations: Vec<ObservationDraft>,
}

pub enum RecommendedAction {
    Proceed,
    ProceedWithReachAdjustment(String),
    PauseForConfirmation(String),
    Settle(String),
}
```

Tests: trait compiles, mock engine returns expected results.

---

### Task 2: SidecarEngine — wraps existing elohim-agent-sdk

**Files:**
- Create: `elohim/elohim-storage/src/services/sidecar_engine.rs`

HTTP client that calls the existing sidecar at `http://localhost:8095/invoke`. Translates InferenceRequest into the SDK's InvokeRequest format. Translates ElohimResponse back into InferenceResult.

This preserves the existing elohim-agent-sdk without changes — just calling it from storage instead of from doorway.

Tests: mock HTTP server, verify request/response translation.

---

### Task 3: InferenceRouter

**Files:**
- Create: `elohim/elohim-storage/src/services/inference_router.rs`

Routing logic:
1. Check local engine availability
2. Check sidecar availability
3. Fall back conservatively (escalate tier) if nothing available

Tests: router picks local when available, falls back to sidecar, escalates tier when both unavailable.

---

### Task 4: Constitutional prompt assembly for gate context

**Files:**
- Modify: `elohim/elohim-storage/src/services/elohim_gate.rs`

Use the constitution crate's `PromptAssembler` to build context-aware prompts for the InferenceEngine. The prompt includes:
- Constitutional principles (weighted by layer)
- Trust context summary
- Mutation type and content
- Session intent (if declared)

Tests: prompt assembly produces expected structure for different tiers.

---

### Task 5: Gate evaluate() calls InferenceRouter

**Files:**
- Modify: `elohim/elohim-storage/src/services/elohim_gate.rs`

Replace the PassThrough skeleton with real routing:
- `None` tier: still PassThrough
- `Light` tier: call router, expect fast response, return Enriched
- `Full` tier: call router, return Enriched or Pause based on RecommendedAction
- `Constitutional` tier: call router with constitutional prompt, return Enriched/Pause/Settlement

Tests: gate returns correct GateResult variants for each tier.

---

### Task 6: Wire gate into all mutation handlers

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs`
- Modify: `elohim/elohim-storage/src/api/*.rs` (mutation handlers)

Pattern for each handler:
1. Parse input
2. Load TrustContext (from cache or compute)
3. Call `gate.evaluate()`
4. Match on GateResult — PassThrough/Enriched proceed, Pause returns prompt, Settlement returns boundary
5. Include GateEvaluationView in response

List of handlers to wire:
- Content create/update (`http.rs`)
- Allocation create/update/delete (`http.rs`)
- Curation event (`api/steward_affinity.rs`) — already wired in Sprint 1, upgrade
- Recognition distribute (`api/recognition.rs`)
- Dispute filing/resolution (`http.rs`)
- Comment/reaction endpoints (if they exist, or stub them)

Tests: integration test — mutation through gate returns GateEvaluationView.

---

### Task 7: TrustContext cache with session-scoped lifecycle

**Files:**
- Modify: `elohim/elohim-storage/src/services/elohim_gate.rs`

In-memory cache (HashMap<session_id, TrustContext>) with TTL and refresh triggers. Invalidated by session intent changes, mastery updates, affinity changes.

Tests: cache hit returns same context, cache miss recomputes, invalidation triggers recompute.

---

### Task 8: Sprint 2 integration verification

Full test suite, clippy, fmt. Verify the gate evaluates mutations end-to-end with the sidecar (if running) or falls back gracefully.

---

## Sprint 2 Deliverables

After this sprint:
- [ ] InferenceEngine trait + SidecarEngine implementation
- [ ] InferenceRouter with local/sidecar/fallback routing
- [ ] Gate calls real inference for Light/Full/Constitutional tiers
- [ ] Constitutional prompt assembly using constitution crate
- [ ] All mutation handlers wired through the gate
- [ ] GateEvaluationView returned alongside mutation responses
- [ ] TrustContext cached per-session with invalidation
- [ ] Graceful fallback when inference unavailable

## What Sprint 3 Builds On

Sprint 3 adds the ImagodeiSubconscious — observations are stored, behavioral trust is computed from history, and the streaming UX delivers the elohim's thinking process to the client.
