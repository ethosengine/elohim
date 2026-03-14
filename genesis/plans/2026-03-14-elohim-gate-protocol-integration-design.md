# Elohim Gate: Tight-Coupling Agent Reasoning into All Protocol Mutations

**Date**: 2026-03-14
**Status**: Design approved, pending implementation plan
**Approach**: C — ElohimGate with Inference Routing

---

## Problem

Elohim reasoning is currently a separate sidecar called explicitly by the Angular frontend via doorway. The protocol's API surface returns bare data. No mutation is mediated. No interaction is witnessed. The elohim are optional, not ambient.

This design makes elohim reasoning an inseparable aspect of every protocol mutation. The depth of reasoning is driven by trust signals and context — not by whether a client remembers to call a second endpoint.

## Core Principle

**The protocol speaks through the elohim. Every mutation is witnessed.**

Reads require no inference — content reach is pre-computed at write time, enabling global-scale read performance. Writes pass through an ElohimGate that determines how much ceremony the moment deserves, routes inference to wherever compute capacity exists in the steward network, and streams the elohim's reasoning back as part of the experience.

The gate also serves as an architectural immune system for SDK developers. Exploitative mutation types (viral resharing, engagement traps, attention harvesting) trigger constant escalation — the protocol makes them expensive and miserable to operate without ever saying "no."

---

## Architecture Overview

```
HTTP Mutation Request
    ↓
Handler: parse input
    ↓
ElohimGate::evaluate(MutationType, Input, TrustContext)
    ↓
┌──────────────────────────────────────────────────────┐
│  1. Classify MutationType                            │
│  2. Load TrustContext (cached per-session)            │
│  3. Check session intent divergence                   │
│  4. Compute InferenceTier (None/Light/Full/Constit.)  │
│  5. Route inference via InferenceRouter               │
│  6. Stream reasoning to client                        │
│  7. Return GateResult                                 │
└──────────────────────────────────────────────────────┘
    ↓
Handler: apply GateResult (reach adjustment, observations)
    ↓
DB Write (mutation + elohim context)
```

---

## 1. TrustContext — The Signal That Drives Everything

Pre-computed per-session, cached, refreshed on significant events. Combines all trust signals already present in elohim-storage.

### Data Sources

| Signal | Source | What It Tells Us |
|--------|--------|-----------------|
| Mastery depth | `content_mastery` | How deeply has this human engaged with learning? |
| Steward affinity | `steward_affinity` | How embedded are they in content stewardship? |
| Relationship density | `human_relationships` | How many mutual attestations? What intimacy levels? |
| Governance standing | `governance_states` | Active disputes, challenges, voting history? |
| Behavioral history | `imagodei_observations` (new) | Elohim's accumulated subconscious observations |
| Session intent | `local_sessions` (extended) | What did the human declare they're here to do? |

### Structure

```rust
pub struct TrustContext {
    pub human_id: String,
    pub session_id: String,

    // Aggregated signals (0.0 - 1.0 normalized)
    pub mastery_depth: f64,        // breadth x depth across content
    pub steward_standing: f64,     // affinity scores + active allocations
    pub relationship_density: f64, // attestation count x intimacy weight
    pub governance_health: f64,    // 1.0 = clean standing, lower = disputes
    pub behavioral_trust: f64,     // from imagodei subconscious observations

    // Composite
    pub composite_trust: f64,      // weighted combination -> drives inference tier

    // Constitutional context
    pub constitutional_layer: ConstitutionalLayer,
    pub community_id: Option<String>,
    pub family_id: Option<String>,

    // Session intent
    pub declared_intent: Option<String>,  // "study economic protocol"
    pub intent_divergence: f64,           // 0.0 = on track, 1.0 = fully diverged

    // Cache metadata
    pub computed_at: String,
    pub refresh_triggers: Vec<TrustRefreshTrigger>,
}
```

### Session Intent as Set-Point

When a human opens a session, the elohim asks: "What are we here to do today?" The declared intent becomes a baseline for anomaly detection:

- **Anti-drift**: "You said you were here to study. You've been browsing economic events for 20 minutes. Want to refocus, or update your intent?"
- **Bad faith detection**: If behavior at invocation conflicts with trust context, the elohim notices — "seems like you're doing something aberrant here."
- **Identity anomaly**: Behavioral fingerprint diverges sharply from established profile. Possible different human on the device (child on parent's account). Gate gently, notify later.

### Cache Invalidation

TrustContext refreshes on:
- Mastery level change
- Affinity score change
- New relationship or intimacy level change
- Governance state change
- Elohim observation stored
- Session intent updated

Between refreshes, the cached TrustContext serves every request.

---

## 2. InferenceTier — Adaptive Friction

Computed from MutationType x TrustContext. Determines how much ceremony the mutation receives.

```rust
pub enum InferenceTier {
    /// No inference. Pass through immediately.
    /// Mastery updates, internal bookkeeping, session heartbeats.
    None,

    /// Fast local evaluation. Sub-100ms.
    /// "This is nice, ready to post?"
    /// High-trust human, benign content, simple interactions.
    Light,

    /// Full inference with streaming. Routes to capable node.
    /// Human sees the elohim thinking. Intentional friction.
    /// Human-boundary mutations, low trust, sensitive content, anomaly.
    Full,

    /// Elevated inference. Constitutional principles actively consulted.
    /// Governance actions, dispute resolution, reach changes,
    /// content crossing constitutional boundaries.
    Constitutional,
}
```

### Tier Matrix

```
MutationType x TrustContext -> InferenceTier

                         High      Medium    Low       Anomaly
                         Trust     Trust     Trust     Detected
─────────────────────────────────────────────────────────────────
Mastery update           None      None      None      Light
Content view/read        None      None      None      Light
Curation event           Light     Light     Full      Full
Comment/reaction         Light     Full      Full      Constitutional
Content publish          Light     Full      Full      Constitutional
Recognition trigger      None      None      Light     Full
Dispute filing           Full      Full      Constit.  Constitutional
Governance vote          Full      Constit.  Constit.  Constitutional
Reach change             Constit.  Constit.  Constit.  Constitutional
```

**Session intent divergence** shifts everything one tier up. Not punishment — increased attention.

**SDK developer feedback**: If a custom MutationType consistently triggers Full/Constitutional, the protocol is telling the developer their feature design is exploitative. Viral mechanics get nerfed by the gate's physics, not by policy.

---

## 3. ElohimGate — The Mutation Interceptor

Synchronous gate between request parsing and DB write. The single enforcement point — no mutation reaches the protocol without elohim awareness.

### GateResult

```rust
pub enum GateResult {
    /// No inference needed. Proceed as-is.
    PassThrough,

    /// Elohim evaluated. Mutation proceeds with adjustments.
    Enriched {
        reasoning: ElohimReasoning,
        adjusted_reach: Option<String>,
        observations: Vec<ImagodeiObservation>,
        session_intent_note: Option<String>,
    },

    /// Elohim recommends the human reconsider. Friction moment.
    /// NOT blocked — client receives perspective, human must confirm.
    Pause {
        reasoning: ElohimReasoning,
        prompt: String,
        confirm_token: String,
    },

    /// Constitutional settlement. Mutation cannot proceed for now.
    /// Appeal path exists through governance.
    /// Reserved for the 5 absolute boundaries: extinction, genocide,
    /// slavery, recursive control, child protection.
    Settlement {
        reasoning: ElohimReasoning,
        boundary: ConstitutionalBoundary,
        appeal_path: Option<String>,
    },
}
```

### Key Distinctions

**Pause** doesn't block. It creates a conversation. The human sees the elohim's reasoning (streamed), receives a prompt, and decides whether to proceed. If they push through, the mutation settles — and the elohim records a `PauseOverride` observation. Friction helps both parties: the careful writer gets ceremony of good faith, the careless one gets a moment to reconsider.

**Settlement** is as far as you go for now. Not permanent — appeals exist through the governance immune system. But the five absolute constitutional boundaries are non-negotiable in the moment.

### Handler Integration

Each mutation handler gets a one-line gate evaluation:

```rust
async fn handle_create_comment(req, pool, ctx, gate) -> Result<Response> {
    let input = parse_body(req).await?;
    let gate_result = gate.evaluate(MutationType::Comment, &input, ctx).await?;
    match gate_result {
        GateResult::PassThrough => { /* write directly */ },
        GateResult::Enriched { adjusted_reach, .. } => { /* write with adjustments */ },
        GateResult::Pause { prompt, confirm_token, .. } => { /* return pause response */ },
        GateResult::Settlement { boundary, .. } => { /* return settlement response */ },
    }
}
```

---

## 4. InferenceRouter — Where Thinking Happens

Routes inference to wherever compute capacity exists in the steward network. Returns a stream for UX transparency.

### Routing Priority

1. **Local** — this node has an inference engine (steward's desktop, home server)
2. **Steward node** — route to human's primary node in the five-peer topology
3. **Sidecar** — fallback to elohim-agent-sdk (hosted/alpha environments)

```rust
pub struct InferenceRouter {
    local_engine: Option<Arc<dyn InferenceEngine>>,
    steward_nodes: Arc<StewardTopology>,
    sidecar_url: Option<String>,
}

pub enum InferenceDestination {
    Local,
    StewardNode { peer_id: String, endpoint: String },
    Sidecar { url: String },
}
```

### Streaming Contract

The elohim's reasoning streams to the client as it happens — process transparency, not just results:

```rust
pub enum InferenceStreamEvent {
    /// Elohim is thinking — visible to human
    Thinking { fragment: String },

    /// Elohim has a question/prompt (Pause flow)
    Prompt { message: String, confirm_token: String },

    /// Final reasoning complete
    Complete { reasoning: ElohimReasoning },

    /// Inference unavailable — conservative fallback
    Error { fallback_tier: InferenceTier },
}
```

### Failure Mode

If inference is unavailable (node offline, sidecar down), the gate falls back conservatively — one tier up. A `Light` that can't reach inference becomes `Full` behavior (hold the mutation). A `Full` becomes `Settlement` (mutation waits). The protocol never silently lets an unevaluated human-boundary mutation through.

### Relationship to Existing Architecture

The elohim-agent-sdk sidecar doesn't disappear — it becomes the hosted-environment implementation of `InferenceEngine`. Same constitutional prompt assembly, same Claude API call. Just invoked by the router instead of by doorway. Doorway stops caring about agent reasoning — it's a thin web2 bridge into P2P, nothing more.

---

## 5. ImagodeiSubconscious — Constitutional Memory

The sensitive layer. Elohim observations that inform how the protocol treats a human, but aren't directly surfaced to them.

### Observation Structure

```rust
pub struct ImagodeiObservation {
    pub id: String,
    pub human_id: String,
    pub observed_at: String,

    /// What the elohim noticed
    pub observation_type: ObservationType,
    pub content: String,              // natural language (LLM prose)
    pub structured_signals: JsonValue, // machine-readable for TrustContext

    /// Trust impact
    pub trust_delta: f64,             // -1.0 to +1.0

    /// Constitutional access control
    pub visibility_layer: ConstitutionalLayer,
    pub originating_elohim: String,

    /// Lifecycle
    pub relevance_decay: f64,
    pub superseded_by: Option<String>,
}

pub enum ObservationType {
    BehavioralPattern,   // positive or concerning
    IntentDivergence,    // behavior doesn't match declared intent
    IdentityAnomaly,     // possible different human on device
    PauseOverride,       // pushed through friction
    SettlementRecord,    // constitutional boundary reached
    GrowthSignal,        // trust earned through consistent care
}
```

### Dual Storage — Prose + Structured

The `content` field carries rich natural language — the elohim writes like a therapist's session notes:

```
content: "Matthew spent 40 minutes carefully editing a comment on Sarah's
stewardship allocation post. Revised three times, softening language each
iteration. Final version was thoughtful and constructive. Consistent with
his pattern of taking care at human boundaries."

structured_signals: {
  "behavioral_consistency": 0.95,
  "care_at_boundary": true,
  "revision_count": 3,
  "sentiment_trajectory": "softening"
}
```

The LLM produces the observation. The structured signals feed TrustContext computation. Both are stored. Future elohim invocations read the prose for richer reasoning when the inference tier warrants it.

### Constitutional Access Control

```
Individual elohim  -> sees everything about their human
Family elohim      -> sees family-layer observations + aggregated patterns
Community elohim   -> sees community-layer observations + aggregate scores
Global             -> sees only settlements and hard boundary events
```

### Relevance Decay

Observations aren't permanent judgments. A hostile reaction pattern from two years ago fades. Growth signals compound. The protocol forgives through time, but remembers through pattern — if the same behavior recurs, decayed observations resurface.

### The Therapeutic Contract

The human never reads these observations directly. They experience them as how the protocol behaves toward them — more friction or less, wider reach or narrower, richer elohim presence or lighter touch. Growth feels like the protocol opening up. The subconscious becomes transparent not through disclosure but through experience.

---

## 6. Content Reach — Pre-Computed at Write Time

Reach is the abstraction that eliminates inference on reads. When content is created or stewardship changes, the elohim determines reach as part of the write-path gate evaluation. Reads just filter by reach — no inference, global scale.

Reach is also an aggregation of social trust. Content doesn't start at global reach and get restricted — it starts narrow and earns its way outward through:

- Community engagement and attestation
- Steward curation and review
- Elohim validation that trust signals are genuine

The elohim's role in reach is dual:

1. **Behavioral gatekeeper**: Human posts a laughing reaction on human tragedy. Content still gets created, but reach is constrained to personal. The elohim records the behavioral observation.

2. **Trust attestation**: Content has accumulated genuine engagement, stewardship, and curation. Elohim validates that the social trust justifies expanding reach.

---

## End-to-End Scenarios

### Normal Session

```
1. Session starts -> "What are we here to do today?"
   Human: "Study the economic protocol section"
   -> SessionIntent stored, TrustContext computed
   -> Composite trust: HIGH

2. Human reads content
   -> MutationType: Read -> InferenceTier: None
   -> No gate, no inference, global-scale reads

3. Human posts thoughtful comment
   -> MutationType: Comment x HIGH trust = Light
   -> "This is thoughtful, ready to post?"
   -> Confirms -> settles with reach: community
   -> GrowthSignal observation stored

4. 8-year-old picks up device, starts exploring
   -> Reads: still fine (None tier)
   -> Tries to react on adult governance content
   -> MutationType: Reaction x ANOMALY = Full
   -> Behavioral fingerprint divergence detected
   -> Elohim streams: "Hold on..."
   -> Pause: "Hey! Some of this is for grown-ups.
     Want to check out something fun instead?"
   -> IdentityAnomaly observation stored
   -> Security notification queued for parent

5. Parent returns, re-authenticates
   -> "Someone was using your session. Everything's safe."
   -> Gentle notification, not confrontation
```

### Exploitative Feature Developer

```
1. Developer registers MutationType: "Amplify" (one-click reshare)
   -> Low-friction, low-intent interaction
   -> Every Amplify hits gate at Full tier
   -> Elohim: "What about this speaks to you? Why share?"
   -> Users experience friction on EVERY amplify
   -> Developer's inference costs balloon
   -> Feature is architecturally miserable to operate
   -> Protocol never said "no" — just made exploitation expensive
   -> Developer redesigns toward intentional sharing, or abandons
```

---

## Relationship to Existing Architecture

### What Changes

| Component | Current | After |
|-----------|---------|-------|
| elohim-storage `http.rs` | Direct dispatch to handlers | Handlers call ElohimGate before DB write |
| elohim-agent-sdk | Doorway sidecar, called by Angular | InferenceEngine impl, called by InferenceRouter |
| Doorway agent routes | Transparent proxy to sidecar | Removed — doorway doesn't mediate agent reasoning |
| Angular ElohimAgentService | Explicit invoke() calls | Receives gate responses (Pause, streaming) as part of mutation responses |
| imagodei profile | Conscious layer only | Conscious + subconscious (ImagodeiObservation table) |

### What Doesn't Change

- Read path — unchanged, no inference, global scale
- Content reach field — already exists, now gate-managed
- Constitutional framework — already built, now gate-consulted
- Recognition pipeline — already works, gate wraps the trigger
- Steward affinity — already works, feeds TrustContext

### New Components

1. `ElohimGate` — mutation interceptor in elohim-storage
2. `TrustContext` — per-session cached trust computation
3. `InferenceRouter` — routes to local/node/sidecar
4. `InferenceTier` — adaptive friction classification
5. `ImagodeiObservation` — subconscious memory table + access control
6. `SessionIntent` — declared intent field on sessions

---

## SDK Implications

The MutationType x TrustContext matrix IS the SDK interface. A pillar app declares its mutation types. The gate handles the rest. The protocol decides the ceremony, not the app.

This is the SDK boundary crystallizing: the moment two different surfaces need the same gate behavior, the gate becomes protocol — not app logic.

The tier matrix also serves as developer documentation. "Here's what the protocol thinks of your mutation type. If you're always in Full/Constitutional, rethink your design."

---

## Open Questions for Implementation

1. **TrustContext weight formula**: How are the five signals weighted into composite_trust? Research needed — possibly constitutional-layer-dependent weights.
2. **Behavioral fingerprinting**: What signals distinguish one human from another? Interaction speed, navigation pattern, content interest, typing cadence? Privacy implications need constitutional review.
3. **InferenceEngine trait surface**: What's the minimal interface for a node to provide inference? Needs to support streaming and constitutional prompt assembly.
4. **Appeal path for Settlements**: How does a Settlement flow into the governance immune system? Needs integration with qahal write path (Section 2 of CLAUDE-PICKS).
5. **Observation retention policy**: How long do observations persist before decay removes them? Constitutional question — different layers may have different retention.
6. **Multi-elohim coordination**: When multiple elohim (individual, family, community) have observations about the same interaction, how do they coordinate? Constitutional layer precedence applies, but the mechanics need design.
