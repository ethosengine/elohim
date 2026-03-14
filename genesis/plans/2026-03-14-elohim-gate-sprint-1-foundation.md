# ElohimGate Sprint 1: Foundation — TrustContext + Gate Skeleton

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Stand up the ElohimGate as a pass-through skeleton in elohim-storage, with TrustContext computation and InferenceTier classification — but no actual inference yet. Every mutation flows through the gate; the gate classifies but always returns PassThrough.

**Architecture:** New `elohim_gate` module in elohim-storage with TrustContext (aggregates mastery, affinity, relationships, governance, behavioral trust into a composite score), InferenceTier enum, MutationType enum, and GateResult. Session intent added to local_sessions. ImagodeiObservation table created (empty for now — populated in Sprint 2). The gate is wired into http.rs mutation handlers but does not call any inference engine.

**Tech Stack:** Rust, Diesel (SQLite), hyper, ts-rs, serde, constitution crate

**Why skeleton first:** This sprint proves the gate can intercept all mutations without breaking anything. Sprint 2 adds inference. Sprint 3 adds the subconscious layer and streaming.

---

### Task 1: Add constitution crate dependency + new module scaffolding

**Files:**
- Modify: `elohim/elohim-storage/Cargo.toml`
- Create: `elohim/elohim-storage/src/services/elohim_gate.rs`
- Create: `elohim/elohim-storage/src/db/imagodei_observations.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs`

**Step 1: Add constitution dependency to Cargo.toml**

Add under `[dependencies]`:
```toml
constitution = { path = "../constitution" }
```

**Step 2: Create empty module files**

```rust
// services/elohim_gate.rs
//! ElohimGate — mutation interceptor for protocol-level agent reasoning.
//!
//! Every mutation passes through the gate. The gate computes a TrustContext,
//! classifies an InferenceTier, and returns a GateResult that determines
//! how the mutation settles.

// db/imagodei_observations.rs
//! ImagodeiObservation — constitutional memory layer.
//!
//! Stores elohim observations about human behavior. Access-controlled
//! by constitutional layer. Feeds behavioral_trust in TrustContext.
```

**Step 3: Register modules**

In `services/mod.rs`, add:
```rust
pub mod elohim_gate;
```

In `db/mod.rs`, add:
```rust
pub mod imagodei_observations;
```

**Step 4: Verify it compiles**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | tail -5`
Expected: compiles with no errors (modules are empty)

**Step 5: Commit**

```bash
git add elohim/elohim-storage/
git commit -m "feat(gate): scaffold elohim_gate module + constitution dependency"
```

---

### Task 2: Database schema — imagodei_observations table + session_intent column

**Files:**
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs`
- Modify: `elohim/elohim-storage/src/db/models.rs`
- Create: SQL migration (or inline schema extension)

**Step 1: Add imagodei_observations table to diesel_schema.rs**

Add after the existing tables:
```rust
diesel::table! {
    imagodei_observations (id) {
        id -> Text,
        app_id -> Text,
        human_id -> Text,
        observed_at -> Text,
        observation_type -> Text,
        content -> Text,
        structured_signals_json -> Nullable<Text>,
        trust_delta -> Float,
        visibility_layer -> Text,
        originating_elohim -> Text,
        relevance_decay -> Float,
        superseded_by -> Nullable<Text>,
        created_at -> Text,
    }
}
```

**Step 2: Add session_intent columns to local_sessions table**

Add two new columns to the `local_sessions` table definition:
```rust
session_intent_json -> Nullable<Text>,
intent_set_at -> Nullable<Text>,
```

**Step 3: Add model structs to models.rs**

```rust
// ============================================================================
// ImagodeiObservation
// ============================================================================

#[derive(Debug, Clone, Queryable, Selectable, Serialize)]
#[diesel(table_name = imagodei_observations)]
pub struct ImagodeiObservation {
    pub id: String,
    pub app_id: String,
    pub human_id: String,
    pub observed_at: String,
    pub observation_type: String,
    pub content: String,
    pub structured_signals_json: Option<String>,
    pub trust_delta: f32,
    pub visibility_layer: String,
    pub originating_elohim: String,
    pub relevance_decay: f32,
    pub superseded_by: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = imagodei_observations)]
pub struct NewImagodeiObservation<'a> {
    pub id: &'a str,
    pub app_id: &'a str,
    pub human_id: &'a str,
    pub observed_at: &'a str,
    pub observation_type: &'a str,
    pub content: &'a str,
    pub structured_signals_json: Option<&'a str>,
    pub trust_delta: f32,
    pub visibility_layer: &'a str,
    pub originating_elohim: &'a str,
    pub relevance_decay: f32,
    pub superseded_by: Option<&'a str>,
}
```

**Step 4: Add allow_tables_to_appear_in_same_query! entry**

In diesel_schema.rs, add `imagodei_observations` to the `allow_tables_to_appear_in_same_query!` macro.

**Step 5: Verify it compiles**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | tail -5`
Expected: compiles

**Step 6: Add SQLite migration to ensure_schema()**

Find the `ensure_schema()` function in the DB initialization code. Add:
```sql
CREATE TABLE IF NOT EXISTS imagodei_observations (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL,
    human_id TEXT NOT NULL,
    observed_at TEXT NOT NULL,
    observation_type TEXT NOT NULL,
    content TEXT NOT NULL,
    structured_signals_json TEXT,
    trust_delta REAL NOT NULL DEFAULT 0.0,
    visibility_layer TEXT NOT NULL DEFAULT 'individual',
    originating_elohim TEXT NOT NULL,
    relevance_decay REAL NOT NULL DEFAULT 0.0,
    superseded_by TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_imagodei_obs_human ON imagodei_observations(human_id);
CREATE INDEX IF NOT EXISTS idx_imagodei_obs_type ON imagodei_observations(observation_type);
```

And alter local_sessions:
```sql
ALTER TABLE local_sessions ADD COLUMN session_intent_json TEXT;
ALTER TABLE local_sessions ADD COLUMN intent_set_at TEXT;
```
(Wrap in try/catch — ALTER TABLE fails if column already exists in SQLite.)

**Step 7: Commit**

```bash
git add elohim/elohim-storage/src/db/
git commit -m "feat(gate): imagodei_observations table + session_intent on local_sessions"
```

---

### Task 3: TrustContext computation

**Files:**
- Modify: `elohim/elohim-storage/src/services/elohim_gate.rs`

**Step 1: Write the failing test — TrustContext computation from mock signals**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_context_high_trust() {
        let ctx = TrustContext::compute(TrustSignals {
            mastery_depth: 0.8,
            steward_standing: 0.7,
            relationship_density: 0.9,
            governance_health: 1.0,
            behavioral_trust: 0.85,
            intent_divergence: 0.0,
        });
        assert!(ctx.composite_trust > 0.7);
        assert!(ctx.composite_trust <= 1.0);
    }

    #[test]
    fn trust_context_low_trust() {
        let ctx = TrustContext::compute(TrustSignals {
            mastery_depth: 0.1,
            steward_standing: 0.0,
            relationship_density: 0.05,
            governance_health: 0.5,
            behavioral_trust: 0.3,
            intent_divergence: 0.0,
        });
        assert!(ctx.composite_trust < 0.3);
    }

    #[test]
    fn intent_divergence_lowers_trust() {
        let base = TrustContext::compute(TrustSignals {
            mastery_depth: 0.8,
            steward_standing: 0.7,
            relationship_density: 0.9,
            governance_health: 1.0,
            behavioral_trust: 0.85,
            intent_divergence: 0.0,
        });
        let diverged = TrustContext::compute(TrustSignals {
            mastery_depth: 0.8,
            steward_standing: 0.7,
            relationship_density: 0.9,
            governance_health: 1.0,
            behavioral_trust: 0.85,
            intent_divergence: 0.8,
        });
        assert!(diverged.composite_trust < base.composite_trust);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test elohim_gate 2>&1 | tail -10`
Expected: FAIL — `TrustContext` not defined

**Step 3: Implement TrustContext and TrustSignals**

```rust
use constitution::ConstitutionalLayer;

/// Raw trust signals gathered from DB queries.
#[derive(Debug, Clone)]
pub struct TrustSignals {
    pub mastery_depth: f64,
    pub steward_standing: f64,
    pub relationship_density: f64,
    pub governance_health: f64,
    pub behavioral_trust: f64,
    pub intent_divergence: f64,
}

/// Pre-computed trust context for a session.
#[derive(Debug, Clone)]
pub struct TrustContext {
    pub human_id: String,
    pub session_id: String,
    pub mastery_depth: f64,
    pub steward_standing: f64,
    pub relationship_density: f64,
    pub governance_health: f64,
    pub behavioral_trust: f64,
    pub intent_divergence: f64,
    pub composite_trust: f64,
    pub constitutional_layer: ConstitutionalLayer,
    pub community_id: Option<String>,
    pub family_id: Option<String>,
    pub declared_intent: Option<String>,
    pub computed_at: String,
}

// Signal weights — constitutional-layer-dependent in the future,
// flat weights for Sprint 1.
const W_MASTERY: f64 = 0.20;
const W_STEWARD: f64 = 0.20;
const W_RELATIONSHIP: f64 = 0.25;
const W_GOVERNANCE: f64 = 0.15;
const W_BEHAVIORAL: f64 = 0.20;
const INTENT_DIVERGENCE_PENALTY: f64 = 0.3;

impl TrustContext {
    /// Compute composite trust from raw signals.
    pub fn compute(signals: TrustSignals) -> Self {
        let raw = signals.mastery_depth * W_MASTERY
            + signals.steward_standing * W_STEWARD
            + signals.relationship_density * W_RELATIONSHIP
            + signals.governance_health * W_GOVERNANCE
            + signals.behavioral_trust * W_BEHAVIORAL;

        // Intent divergence penalizes composite trust
        let penalty = signals.intent_divergence * INTENT_DIVERGENCE_PENALTY;
        let composite = (raw - penalty).clamp(0.0, 1.0);

        Self {
            human_id: String::new(),
            session_id: String::new(),
            mastery_depth: signals.mastery_depth,
            steward_standing: signals.steward_standing,
            relationship_density: signals.relationship_density,
            governance_health: signals.governance_health,
            behavioral_trust: signals.behavioral_trust,
            intent_divergence: signals.intent_divergence,
            composite_trust: composite,
            constitutional_layer: ConstitutionalLayer::Individual,
            community_id: None,
            family_id: None,
            declared_intent: None,
            computed_at: String::new(),
        }
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test elohim_gate 2>&1 | tail -10`
Expected: 3 tests PASS

**Step 5: Commit**

```bash
git add elohim/elohim-storage/src/services/elohim_gate.rs
git commit -m "feat(gate): TrustContext computation with weighted signal aggregation"
```

---

### Task 4: MutationType + InferenceTier classification

**Files:**
- Modify: `elohim/elohim-storage/src/services/elohim_gate.rs`

**Step 1: Write failing tests for tier classification**

```rust
#[test]
fn mastery_update_always_none() {
    let high_trust = TrustContext::compute(TrustSignals {
        mastery_depth: 0.9, steward_standing: 0.9,
        relationship_density: 0.9, governance_health: 1.0,
        behavioral_trust: 0.9, intent_divergence: 0.0,
    });
    assert_eq!(
        InferenceTier::classify(MutationType::MasteryUpdate, &high_trust),
        InferenceTier::None
    );
}

#[test]
fn comment_high_trust_is_light() {
    let high_trust = TrustContext::compute(TrustSignals {
        mastery_depth: 0.8, steward_standing: 0.7,
        relationship_density: 0.9, governance_health: 1.0,
        behavioral_trust: 0.85, intent_divergence: 0.0,
    });
    assert_eq!(
        InferenceTier::classify(MutationType::Comment, &high_trust),
        InferenceTier::Light
    );
}

#[test]
fn comment_low_trust_is_full() {
    let low_trust = TrustContext::compute(TrustSignals {
        mastery_depth: 0.1, steward_standing: 0.0,
        relationship_density: 0.05, governance_health: 0.5,
        behavioral_trust: 0.3, intent_divergence: 0.0,
    });
    assert_eq!(
        InferenceTier::classify(MutationType::Comment, &low_trust),
        InferenceTier::Full
    );
}

#[test]
fn governance_vote_is_at_least_full() {
    let high_trust = TrustContext::compute(TrustSignals {
        mastery_depth: 0.9, steward_standing: 0.9,
        relationship_density: 0.9, governance_health: 1.0,
        behavioral_trust: 0.9, intent_divergence: 0.0,
    });
    let tier = InferenceTier::classify(MutationType::GovernanceVote, &high_trust);
    assert!(tier == InferenceTier::Full || tier == InferenceTier::Constitutional);
}

#[test]
fn reach_change_always_constitutional() {
    let high_trust = TrustContext::compute(TrustSignals {
        mastery_depth: 0.9, steward_standing: 0.9,
        relationship_density: 0.9, governance_health: 1.0,
        behavioral_trust: 0.9, intent_divergence: 0.0,
    });
    assert_eq!(
        InferenceTier::classify(MutationType::ReachChange, &high_trust),
        InferenceTier::Constitutional
    );
}

#[test]
fn intent_divergence_escalates_tier() {
    let diverged = TrustContext::compute(TrustSignals {
        mastery_depth: 0.8, steward_standing: 0.7,
        relationship_density: 0.9, governance_health: 1.0,
        behavioral_trust: 0.85, intent_divergence: 0.8,
    });
    // Comment would normally be Light for high trust, but divergence escalates
    let tier = InferenceTier::classify(MutationType::Comment, &diverged);
    assert!(tier == InferenceTier::Full || tier == InferenceTier::Constitutional);
}
```

**Step 2: Run tests — verify they fail**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test elohim_gate 2>&1 | tail -10`
Expected: FAIL — `MutationType` not defined

**Step 3: Implement MutationType, InferenceTier, and classification**

```rust
/// What kind of mutation is being attempted.
/// Extensible — SDK developers register new types here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationType {
    // Internal bookkeeping — never gated
    MasteryUpdate,
    SessionHeartbeat,
    InternalSync,

    // Curation — light gate for trusted stewards
    CurationEvent,
    AllocationUpdate,

    // Human boundary — full gate for untrusted
    Comment,
    Reaction,
    ContentPublish,

    // Recognition pipeline — usually pass-through
    RecognitionTrigger,

    // Governance — always elevated
    DisputeFiling,
    GovernanceVote,
    ReachChange,

    // Catch-all for SDK extensions
    Custom(u32),
}

/// How much ceremony this mutation receives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferenceTier {
    None,
    Light,
    Full,
    Constitutional,
}

impl InferenceTier {
    /// Escalate one level.
    pub fn escalate(self) -> Self {
        match self {
            Self::None => Self::Light,
            Self::Light => Self::Full,
            Self::Full => Self::Constitutional,
            Self::Constitutional => Self::Constitutional,
        }
    }

    /// Classify the inference tier for a mutation given trust context.
    pub fn classify(mutation: MutationType, ctx: &TrustContext) -> Self {
        let base = Self::base_tier(mutation, ctx.composite_trust);

        // Intent divergence escalates
        if ctx.intent_divergence > 0.5 {
            base.escalate()
        } else {
            base
        }
    }

    fn base_tier(mutation: MutationType, trust: f64) -> Self {
        use MutationType::*;

        match mutation {
            // Never gated
            MasteryUpdate | SessionHeartbeat | InternalSync => Self::None,

            // Curation
            CurationEvent | AllocationUpdate => {
                if trust > 0.6 { Self::Light } else { Self::Full }
            }

            // Human boundary
            Comment | Reaction | ContentPublish => {
                if trust > 0.7 { Self::Light } else { Self::Full }
            }

            // Recognition
            RecognitionTrigger => {
                if trust > 0.4 { Self::None } else { Self::Light }
            }

            // Governance — always elevated
            DisputeFiling => Self::Full,
            GovernanceVote => {
                if trust > 0.8 { Self::Full } else { Self::Constitutional }
            }
            ReachChange => Self::Constitutional,

            // Unknown SDK types — conservative
            Custom(_) => Self::Full,
        }
    }
}
```

**Step 4: Run tests — verify they pass**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test elohim_gate 2>&1 | tail -10`
Expected: all tests PASS

**Step 5: Commit**

```bash
git add elohim/elohim-storage/src/services/elohim_gate.rs
git commit -m "feat(gate): MutationType + InferenceTier classification with tier matrix"
```

---

### Task 5: GateResult + ElohimGate service skeleton

**Files:**
- Modify: `elohim/elohim-storage/src/services/elohim_gate.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

**Step 1: Write failing test — gate always returns PassThrough in Sprint 1**

```rust
#[test]
fn gate_evaluate_returns_pass_through_for_now() {
    let gate = ElohimGate::new_skeleton();
    let ctx = TrustContext::compute(TrustSignals {
        mastery_depth: 0.5, steward_standing: 0.5,
        relationship_density: 0.5, governance_health: 1.0,
        behavioral_trust: 0.5, intent_divergence: 0.0,
    });
    let result = gate.evaluate(MutationType::Comment, &ctx);
    // Sprint 1: always PassThrough (no inference engine yet)
    assert!(matches!(result, GateResult::PassThrough { tier, .. }));
    // But tier is correctly classified
    assert_eq!(tier, InferenceTier::Light);
}
```

**Step 2: Run test — verify it fails**

**Step 3: Implement GateResult and ElohimGate skeleton**

```rust
use serde::Serialize;

/// Result of gate evaluation.
#[derive(Debug, Clone)]
pub enum GateResult {
    /// No inference needed, or inference not yet available (Sprint 1).
    /// Includes the classified tier for observability.
    PassThrough {
        tier: InferenceTier,
    },

    /// Elohim evaluated. Mutation proceeds with adjustments.
    Enriched {
        tier: InferenceTier,
        reasoning: ElohimReasoning,
        adjusted_reach: Option<String>,
        observations: Vec<ObservationDraft>,
        session_intent_note: Option<String>,
    },

    /// Friction moment. Human must confirm to proceed.
    Pause {
        tier: InferenceTier,
        reasoning: ElohimReasoning,
        prompt: String,
        confirm_token: String,
    },

    /// Constitutional settlement. Appeal path exists.
    Settlement {
        tier: InferenceTier,
        reasoning: ElohimReasoning,
        boundary: String,
        appeal_path: Option<String>,
    },
}

/// Placeholder for Sprint 2 — will carry LLM reasoning.
#[derive(Debug, Clone, Serialize)]
pub struct ElohimReasoning {
    pub primary_principle: String,
    pub interpretation: String,
    pub confidence: f64,
}

/// Draft observation to be stored in imagodei_observations.
#[derive(Debug, Clone)]
pub struct ObservationDraft {
    pub observation_type: String,
    pub content: String,
    pub structured_signals: Option<serde_json::Value>,
    pub trust_delta: f64,
    pub visibility_layer: String,
}

/// The gate itself. Sprint 1: classify-only skeleton.
pub struct ElohimGate {
    // Sprint 2: inference_router: Option<InferenceRouter>,
    // Sprint 3: observation_store: ObservationStore,
}

impl ElohimGate {
    /// Create a skeleton gate with no inference capability.
    pub fn new_skeleton() -> Self {
        Self {}
    }

    /// Evaluate a mutation against trust context.
    /// Sprint 1: always returns PassThrough with correct tier classification.
    pub fn evaluate(&self, mutation: MutationType, ctx: &TrustContext) -> GateResult {
        let tier = InferenceTier::classify(mutation, ctx);

        // Sprint 1: classify but don't invoke inference
        GateResult::PassThrough { tier }
    }
}
```

**Step 4: Run tests — verify they pass**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test elohim_gate 2>&1 | tail -10`
Expected: all tests PASS

**Step 5: Add ElohimGate to Services struct**

In `services/mod.rs`, add to the struct:
```rust
pub gate: Arc<ElohimGate>,
```

And in `Services::new()`:
```rust
gate: Arc::new(ElohimGate::new_skeleton()),
```

**Step 6: Verify full build**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | tail -5`

**Step 7: Commit**

```bash
git add elohim/elohim-storage/src/services/
git commit -m "feat(gate): GateResult + ElohimGate skeleton on Services struct"
```

---

### Task 6: Views + TypeScript type generation

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs`

**Step 1: Add TrustContext and GateResult views for TypeScript consumption**

```rust
/// Trust context summary for client observability.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct TrustContextView {
    pub composite_trust: f64,
    pub mastery_depth: f64,
    pub steward_standing: f64,
    pub relationship_density: f64,
    pub governance_health: f64,
    pub behavioral_trust: f64,
    pub intent_divergence: f64,
    pub declared_intent: Option<String>,
}

/// Gate evaluation result for client.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct GateEvaluationView {
    pub tier: String,
    pub trust_context: TrustContextView,
    /// Present when gate pauses the mutation
    pub pause_prompt: Option<String>,
    /// Token to confirm a paused mutation
    pub confirm_token: Option<String>,
    /// Present when gate settles (constitutional boundary)
    pub settlement_boundary: Option<String>,
    pub appeal_path: Option<String>,
}

/// Imagodei observation output view.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ImagodeiObservationView {
    pub id: String,
    pub human_id: String,
    pub observed_at: String,
    pub observation_type: String,
    pub content: String,
    pub structured_signals: Option<JsonVal>,
    pub trust_delta: f64,
    pub visibility_layer: String,
    pub relevance_decay: f64,
    pub created_at: String,
}

/// Session intent input view.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct SetSessionIntentInputView {
    pub intent: String,
}
```

**Step 2: Generate TypeScript types**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings 2>&1 | tail -5`
Expected: new `.ts` files appear in `elohim/sdk/storage-client-ts/src/generated/`

**Step 3: Verify generated files**

Run: `ls elohim/sdk/storage-client-ts/src/generated/TrustContextView.ts elohim/sdk/storage-client-ts/src/generated/GateEvaluationView.ts`
Expected: both files exist

**Step 4: Commit**

```bash
git add elohim/elohim-storage/src/views.rs elohim/sdk/storage-client-ts/src/generated/
git commit -m "feat(gate): TrustContext + GateEvaluation + ImagodeiObservation views with TS export"
```

---

### Task 7: Wire gate into one mutation handler (proof of concept)

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs`
- Modify: `elohim/elohim-storage/src/api/steward_affinity.rs` (curation event handler)

**Step 1: Wire gate into the curation event handler as proof of concept**

In `api/steward_affinity.rs`, modify `handle_curation_event`:

```rust
async fn handle_curation_event(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
    gate: &ElohimGate,
    trust_ctx: &TrustContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let input: crate::views::CurationEventInputView = parse_body(req).await?;

    // Gate evaluation
    let gate_result = gate.evaluate(
        crate::services::elohim_gate::MutationType::CurationEvent,
        trust_ctx,
    );

    // Sprint 1: always PassThrough, log the tier for observability
    tracing::info!(
        tier = ?gate_result.tier(),
        human_id = %trust_ctx.human_id,
        "ElohimGate evaluated curation event"
    );

    let mut conn = get_conn(pool)?;
    let result = crate::services::steward_affinity_service::record_curation_activity(
        &mut conn, ctx,
        &input.steward_id, &input.content_id, &input.activity_type,
    )?;

    let view = StewardAffinityView::from(result);
    Ok(response::created(&view))
}
```

**Step 2: Add a tier() accessor to GateResult**

```rust
impl GateResult {
    pub fn tier(&self) -> InferenceTier {
        match self {
            Self::PassThrough { tier } => *tier,
            Self::Enriched { tier, .. } => *tier,
            Self::Pause { tier, .. } => *tier,
            Self::Settlement { tier, .. } => *tier,
        }
    }
}
```

**Step 3: Verify it compiles and existing tests pass**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test 2>&1 | tail -10`
Expected: all existing tests + gate tests pass

**Step 4: Commit**

```bash
git add elohim/elohim-storage/src/
git commit -m "feat(gate): wire ElohimGate into curation event handler (proof of concept)"
```

---

### Task 8: Session intent API endpoint

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs` or relevant API module
- Modify: `elohim/elohim-storage/src/db/local_sessions.rs`

**Step 1: Add set_session_intent DB function**

```rust
pub fn set_session_intent(
    conn: &mut SqliteConnection,
    session_id: &str,
    intent_json: &str,
) -> Result<(), StorageError> {
    use crate::db::diesel_schema::local_sessions::dsl::*;
    diesel::update(local_sessions.filter(id.eq(session_id)))
        .set((
            session_intent_json.eq(Some(intent_json)),
            intent_set_at.eq(Some(current_timestamp())),
        ))
        .execute(conn)?;
    Ok(())
}
```

**Step 2: Add HTTP endpoint**

Route: `POST /api/v1/session/intent`

```rust
// Parse SetSessionIntentInputView
// Call set_session_intent(conn, session_id, &serde_json::to_string(&input)?)
// Invalidate cached TrustContext for this session
// Return 200 OK
```

**Step 3: Verify**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | tail -5`

**Step 4: Commit**

```bash
git add elohim/elohim-storage/src/
git commit -m "feat(gate): POST /api/v1/session/intent endpoint for session set-point"
```

---

### Task 9: Sprint 1 integration verification

**Step 1: Run full test suite**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test 2>&1 | tail -20`
Expected: all tests pass

**Step 2: Run clippy**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings 2>&1 | tail -10`
Expected: no warnings

**Step 3: Run fmt**

Run: `cd elohim/elohim-storage && cargo fmt --check 2>&1`
Expected: no formatting issues

**Step 4: Final commit if any fixes needed**

```bash
git commit -m "chore(gate): Sprint 1 cleanup — clippy + fmt"
```

---

## Sprint 1 Deliverables

After this sprint:
- [x] `imagodei_observations` table exists (empty, ready for Sprint 2)
- [x] `session_intent` column on `local_sessions`
- [x] `TrustContext` computes composite trust from 5 signals + intent divergence
- [x] `InferenceTier` classifies mutations via the tier matrix
- [x] `ElohimGate` skeleton evaluates every mutation (always PassThrough)
- [x] Gate wired into curation event handler as proof of concept
- [x] `POST /api/v1/session/intent` endpoint for session set-point
- [x] TypeScript types generated for TrustContext, GateEvaluation, ImagodeiObservation
- [x] All tests pass, clippy clean, fmt clean

## What Sprint 2 Builds On

Sprint 2 adds the InferenceRouter and wires the gate to actually call inference (Light tier first). The skeleton becomes alive — the elohim starts thinking.
