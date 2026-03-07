# Elohim Compute Coordination — Design

**Goal:** Build pillar-enabled compute coordination so elohim agents can negotiate, admit, serve, and account for inference requests — with training wheels that remove cleanly when the mesh is ready.

**Architecture:** Three composing layers (admission, REA accounting, capacity gossip) integrated into elohim-node via the existing `elohim-agent` Rust crate. The TypeScript sidecar remains as a dev bridge until the Rust path is fully wired.

---

## Context

### What exists

- **doorway**: Auth gate — "is this a real person?" JWT/API-key check on `/api/v1/elohim/invoke`. Protects the p2p network from web2 abuse. No compute logic.
- **elohim-agent-sdk** (TypeScript sidecar): Demo bridge wrapping Claude Haiku. Budget enforcement (in-memory, 10-call cap). Constitutional prompt assembly (hardcoded Global layer).
- **elohim-agent** (Rust crate): Full production framework — `LlmBackend` trait with AnthropicBackend, OpenAiBackend, LlamaCppBackend (feature-gated), MockBackend. 28-capability registry. Full 5-layer constitutional stack. AuditLog. Streaming support. **Not yet wired into elohim-node.**
- **elohim-node**: System metrics collection (CPU/mem/disk/net), NodeConditions, pod orchestration (observe → act), gossipsub for existing protocols, operator model (permissions, not resources).
- **shefa**: `ComputeEventService` (maps CPU/storage/bandwidth to token earnings, rates TBD), `EconomicEventFactoryService` (generic REA events, banking focus).
- **Angular**: `ElohimPresenceService` invokes via `ElohimAgentService` → `ElohimBackendCatalog` → `NativeBackend` → doorway → sidecar. Cost tracking per session.

### What's missing

The **bridge** between "what resources are available" and "should this request be accepted." No admission control. No compute accounting per invocation. No capacity discovery between nodes.

---

## Design

### Principle

The sidecar is a **model adapter**, not the intelligence coordinator. The `elohim-agent` Rust crate is the canonical inference interface. It belongs in `elohim-node` where it has access to system metrics, gossipsub, and the pod orchestration layer. No single AI company owns the elohim contract — the `LlmBackend` trait is the extensibility point for anyone to bring a suitable model.

### Component Responsibilities

```
Web2 world                    elohim-node (Rust)
─────────────                 ─────────────────────────────────
Browser/App                   ┌─────────────────────────────────┐
    │                         │  elohim-agent (Rust crate)       │
    ▼                         │  ├── LlmBackend trait            │
  doorway ── auth ──────────► │  │   ├── AnthropicBackend        │
  "real human?"               │  │   ├── OpenAiBackend (vLLM/    │
                              │  │   │   Ollama/local)           │
                              │  │   ├── LlamaCppBackend         │
                              │  │   └── ... anyone can add      │
                              │  ├── CapabilityRegistry          │
                              │  ├── ConstitutionalStack         │
                              │  └── AuditLog                    │
                              │                                  │
                              │  Admission Controller (NEW)      │
                              │  ├── Priority Queue              │
                              │  ├── Budget Enforcement          │
                              │  ├── Capacity Awareness          │
                              │  └── Defer + Mesh Hints          │
                              │                                  │
                              │  REA Compute Accounting (NEW)    │
                              │  ├── Commitment (on accept)      │
                              │  └── Event (on fulfill)          │
                              │                                  │
                              │  Gossipsub (existing + NEW topic)│
                              │  ├── /elohim/compute/capacity    │
                              │  └── Neighbor Capacity Table     │
                              └─────────────────────────────────┘
```

**doorway**: Web2 shield. Auth gate. Proxies to elohim-node. No compute logic.

**elohim-node**: The brain. Owns admission, REA accounting, gossip, model adapter dispatch.

**Model adapters** (via `LlmBackend` trait): Thin wrappers per AI provider. `AnthropicBackend`, `OpenAiBackend`, `LlamaCppBackend` already exist in the crate.

**elohim-agent-sdk** (TypeScript): Temporary dev bridge. Stays alive for dev while Rust path is wired. Both coexist — doorway routes to whichever is available.

**Angular (shefa pillar)**: Receives response with cost metadata, displays transparency. Queries/projects REA events from elohim-node but doesn't create them.

### Request Lifecycle

```
1. REQUEST ARRIVES
   Browser → doorway (auth gate) → elohim-node HTTP endpoint

2. ADMISSION (elohim-node)
   ├── Constitutional fitness: capability registered + layer check
   ├── Budget check: budget remaining > estimated cost
   ├── Capacity check: active requests < limit, conditions ready
   │
   ├── IF ACCEPTED:
   │   ├── Create REA Commitment
   │   ├── Enqueue with priority (urgent > high > normal > low)
   │   └── Return 202 Accepted { commitment_id, estimated_wait_ms }
   │
   └── IF DEFERRED:
       ├── Query neighbor capacity table (from gossipsub)
       ├── Return 202 { status: "deferred", meshHints, retryAfterMs }
       └── Client can race: wait OR try a hinted neighbor

3. QUEUE PROCESSING
   ├── Dequeue by priority
   ├── Select backend via ElohimAgentService.invoke()
   ├── Build constitutional prompt (full 5-layer stack)
   ├── Call LLM backend
   └── Parse response with constitutional reasoning

4. FULFILLMENT
   ├── Create REA Event (action: 'use', resourceType: 'inference-tokens')
   ├── Update AuditLog
   ├── Decrement budget
   ├── Return ElohimResponse to client
   └── If another node fulfilled first: cancel commitment, no event

5. CAPACITY BROADCAST (every 30s)
   └── Publish to /elohim/compute/capacity/1.0.0:
       { node_id, budget_remaining, active_requests, queue_depth,
         estimated_tokens_per_sec, capabilities, conditions }
```

### Contract Shapes

These types must be correct now — they're the interface that training wheels attach to:

```rust
enum AdmissionDecision {
    Accepted { commitment_id: String, queue_position: u32, estimated_wait_ms: u64 },
    Deferred { reason: DeferReason, mesh_hints: Vec<MeshHint>, retry_after_ms: u64 },
    Declined { reason: String },
}

enum DeferReason { BudgetExhausted, QueueFull, CapabilityUnavailable, SystemPressure }

struct MeshHint {
    node_id: String,
    budget_remaining: u32,
    estimated_wait_ms: u64,
    capabilities: Vec<String>,
}

struct CapacityAnnouncement {
    node_id: String,
    timestamp: u64,
    budget_remaining: u32,
    active_requests: u32,
    queue_depth: u32,
    estimated_tokens_per_sec: f32,
    capabilities: Vec<String>,
    conditions: NodeConditions,
}

struct ComputeCommitment {
    id: String,
    request_id: String,
    node_id: String,
    requester_id: String,
    capability: String,
    estimated_cost: u32,
    status: CommitmentStatus, // Pending | Fulfilled | Cancelled
    created_at: String,
}

struct ComputeEvent {
    id: String,
    commitment_id: String,
    provider_id: String,   // node operator
    receiver_id: String,   // requesting human
    action: String,        // "use"
    resource_type: String, // "inference-tokens"
    tokens_used: u32,
    model: String,
    time_ms: u64,
    capability: String,
    created_at: String,
}
```

### Training Wheels

| Layer | What We Build | Training Wheel | Removes To |
|-------|--------------|----------------|------------|
| Admission | AdmissionController | Single-node, no mesh routing | Mesh-aware routing |
| Admission | Priority queue | In-memory, lost on restart | DHT-persistent queue |
| Admission | Budget enforcement | Static limit from config | Dynamic from shefa governance |
| Admission | Defer with mesh hints | meshHints: [] (empty) | Populated from gossip table |
| REA | Commitment + Event types | Local AuditLog only | Holochain DHT via elohim-storage |
| REA | Per-invocation events | Types + local recording | Full shefa pillar integration |
| Gossip | Capacity topic + announcement type | Broadcasts own, ignores neighbors | Builds neighbor table, feeds hints |
| Backend | Wire elohim-agent into elohim-node | AnthropicBackend with API key from config | Multiple backends, BYOK, local inference |
| Backend | HTTP endpoint on elohim-node | Doorway proxies to new endpoint | Direct p2p invocation |
| Angular | NativeBackend routes to node endpoint | Falls back to sidecar if node unavailable | Sidecar retired |

### Deferred

- Mesh routing (sending requests to hinted neighbors)
- DHT-persisted REA events (needs elohim-storage REA zome)
- Dynamic budget from governance (needs shefa+compute governance)
- Per-user quotas (needs qahal governance)
- Race pattern (try this node + neighbor simultaneously)
- Multiple concurrent backends per node
- Streaming through the full chain

### Testing

**Rust unit tests (elohim-node):**
- AdmissionController: accept/defer/decline decisions, priority ordering, empty mesh hints
- CapacityAnnouncement: serialization, construction from NodeMetrics
- ComputeCommitment: lifecycle (pending → fulfilled/cancelled)
- ComputeEvent: cost capture, commitment linkage
- Integration: full invoke lifecycle with MockBackend

**Angular unit tests:**
- NativeBackend: handles deferred responses, surfaces mesh hints
- ElohimPresenceService: deferred shows waiting state, cost from REA event data

**A2O scenarios:**
- Authenticated learner receives insight within budget
- Request deferred when budget exhausted
- Compute cost recorded as economic event
