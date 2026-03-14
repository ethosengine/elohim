# ElohimGate Sprint 3: Subconscious — Memory, Streaming, and Behavioral Trust

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Close the loop. Observations are stored in the imagodei subconscious. Behavioral trust is computed from observation history. The streaming UX delivers the elohim's thinking to the client in real-time. The Pause flow works end-to-end with confirmation tokens.

**Architecture:** `ObservationStore` writes to `imagodei_observations` with constitutional access control. `BehavioralTrustComputer` aggregates observation history (with relevance decay) into the behavioral_trust signal in TrustContext. `InferenceStream` delivers SSE events to the client. Pause flow uses confirmation tokens stored in a short-lived cache.

**Tech Stack:** Rust, SSE (Server-Sent Events via hyper), Diesel, tokio

**Depends on:** Sprint 2 complete — InferenceEngine, router, gate wired into handlers.

**NOTE:** This plan will be updated with Sprint 1+2 feedback before execution.

---

### Task 1: ObservationStore — write observations with access control

**Files:**
- Modify: `elohim/elohim-storage/src/db/imagodei_observations.rs`
- Modify: `elohim/elohim-storage/src/services/elohim_gate.rs`

CRUD for imagodei_observations table:
- `create_observation(conn, ctx, &NewImagodeiObservation)` — insert with visibility_layer
- `list_observations_for_human(conn, ctx, human_id, visibility_layer)` — filtered by access level
- `list_observations_by_type(conn, ctx, human_id, observation_type)` — for pattern detection

Constitutional access control: the `visibility_layer` field determines which constitutional layer can read. A query from a community-level context only sees observations with `visibility_layer >= community`.

Tests:
- Create observation, verify it's stored
- List by human_id returns correct observations
- Visibility filter excludes observations from higher layers

---

### Task 2: BehavioralTrustComputer — aggregate observations into trust signal

**Files:**
- Create: `elohim/elohim-storage/src/services/behavioral_trust.rs`

Computes `behavioral_trust` (0.0-1.0) from observation history:
- Start at 0.5 (neutral baseline)
- Each observation adjusts by `trust_delta * (1.0 - relevance_decay * age_days / 365.0)`
- GrowthSignal observations compound positively
- PauseOverride observations compound negatively (but less — the protocol forgives)
- SettlementRecord observations have lasting impact (slower decay)
- Clamp to [0.0, 1.0]

Tests:
- Empty history returns 0.5 (neutral)
- Positive observations increase trust
- Negative observations decrease trust
- Old observations decay toward irrelevance
- Mixed history reaches stable equilibrium

---

### Task 3: Wire BehavioralTrustComputer into TrustContext

**Files:**
- Modify: `elohim/elohim-storage/src/services/elohim_gate.rs`

When computing TrustContext, query behavioral trust from ObservationStore + BehavioralTrustComputer instead of using a placeholder value.

```rust
// In TrustContext computation:
let behavioral_trust = BehavioralTrustComputer::compute(
    &observations_for_human(conn, ctx, human_id, visibility_layer)?
);
```

Tests: TrustContext with real observation history produces correct behavioral_trust.

---

### Task 4: Gate stores observations after evaluation

**Files:**
- Modify: `elohim/elohim-storage/src/services/elohim_gate.rs`

After the InferenceEngine returns observations in the GateResult, the gate writes them to imagodei_observations. This closes the feedback loop — the gate produces observations that feed future TrustContext computation.

```rust
// After evaluate():
if let GateResult::Enriched { observations, .. } = &result {
    for obs in observations {
        observation_store.create(conn, ctx, human_id, obs)?;
    }
}
```

Also store implicit observations:
- `PauseOverride` when human confirms a Pause
- `GrowthSignal` when Light-tier mutations consistently proceed without issues

Tests: gate evaluation produces and stores observations.

---

### Task 5: Pause flow with confirmation tokens

**Files:**
- Modify: `elohim/elohim-storage/src/services/elohim_gate.rs`
- Modify: `elohim/elohim-storage/src/http.rs`

When the gate returns `Pause`:
1. Generate a confirmation token (UUID)
2. Store it in a short-lived in-memory cache (TTL 5 minutes)
3. Return the pause prompt + token to the client
4. Client displays the elohim's reasoning and presents a confirm/cancel choice
5. If confirmed: `POST /api/v1/gate/confirm` with token
6. Gate validates token, stores PauseOverride observation, proceeds with original mutation

```rust
// New endpoint
POST /api/v1/gate/confirm
Body: { "confirmToken": "...", "mutationPayload": { ... } }
```

Tests:
- Pause returns valid token
- Confirm with valid token proceeds
- Confirm with expired/invalid token returns error
- PauseOverride observation stored on confirm

---

### Task 6: Streaming inference responses (SSE)

**Files:**
- Modify: `elohim/elohim-storage/src/services/inference_engine.rs`
- Modify: `elohim/elohim-storage/src/http.rs`

Add streaming variant to InferenceEngine:

```rust
#[async_trait::async_trait]
pub trait InferenceEngine: Send + Sync {
    async fn evaluate(&self, request: InferenceRequest) -> Result<InferenceResult, InferenceError>;

    /// Streaming evaluation — sends SSE events as the elohim reasons.
    async fn evaluate_streaming(
        &self,
        request: InferenceRequest,
        sender: tokio::sync::mpsc::Sender<InferenceStreamEvent>,
    ) -> Result<InferenceResult, InferenceError>;

    async fn is_available(&self) -> bool;
}
```

For Full and Constitutional tiers, the gate uses `evaluate_streaming` and pipes events to the HTTP response as SSE:

```
event: thinking
data: {"fragment": "Looking at your comment..."}

event: thinking
data: {"fragment": "This seems thoughtful and constructive."}

event: prompt
data: {"message": "Ready to post?", "confirmToken": "abc-123"}

event: complete
data: {"tier": "light", "action": "proceed"}
```

Tests: SSE events are well-formed, streaming completes with final result.

---

### Task 7: Anomaly detection — behavioral fingerprint divergence

**Files:**
- Create: `elohim/elohim-storage/src/services/anomaly_detection.rs`

Compare current interaction patterns against established behavioral profile:
- Interaction speed (time between mutations)
- Content interest patterns (what types of content accessed)
- Navigation patterns (sequential study vs random browsing)
- Typing/editing patterns (if available from input metadata)

Returns `anomaly_score: f64` (0.0 = consistent, 1.0 = completely different person).

This feeds into TrustContext as an additional signal that can trigger tier escalation. The 8-year-old-on-parent's-device scenario.

Tests:
- Consistent patterns return low anomaly score
- Divergent patterns return high anomaly score
- Score degrades gracefully with limited data

---

### Task 8: Angular client support — GateEvaluationView handling

**Files:**
- Create: `app/elohim-app/src/app/elohim/services/elohim-gate-client.service.ts`

Angular service that:
- Reads GateEvaluationView from mutation responses
- Handles Pause flow (show prompt, collect confirmation, POST confirm)
- Handles SSE streaming (display elohim thinking process)
- Handles Settlement display (show boundary, link to appeal)

This is a thin client — the protocol decides, the client renders.

Tests: Vitest tests for each gate result variant handling.

---

### Task 9: Sprint 3 integration verification

Full test suite across Rust and Angular. Verify end-to-end:
1. Session with intent set
2. Mutation through gate
3. Inference called (or fallback)
4. Observation stored
5. Behavioral trust updated
6. Next mutation reflects updated trust

---

## Sprint 3 Deliverables

After this sprint:
- [ ] Observations stored in imagodei_observations after gate evaluation
- [ ] BehavioralTrustComputer aggregates observation history with decay
- [ ] TrustContext uses real behavioral trust from observations
- [ ] Pause flow works end-to-end with confirmation tokens
- [ ] SSE streaming of elohim reasoning for Full/Constitutional tiers
- [ ] Anomaly detection provides identity divergence signal
- [ ] Angular client handles all GateResult variants
- [ ] Feedback loop closed: gate → observations → behavioral trust → gate

## Post-Sprint 3

The ElohimGate is operational. Future work:
- InferenceRouter steward-node routing (P2P topology integration)
- Multi-elohim coordination (individual + family + community)
- Affinity decay (time-based, separate from observation decay)
- Appeal path integration with qahal governance write path
- SDK documentation for custom MutationType registration
