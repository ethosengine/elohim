# Elohim-Agent Gate Interface — Protocol-Core Specification

**Status:** Draft
**Date:** 2026-04-18
**Owner:** Matthew Dowell
**Companion theory document:** `elohim/elohim-agent/research/2026-04-18-gate-theory.md`
**Companion implementation plan:** `genesis/plans/2026-04-18-elohim-agent-gate-interface-plan.md`

---

## Scope and Phase Notice

This spec describes the target architecture of the Elohim Protocol's gate primitive. The full invariants described here activate when **elohim are present and active on the network** as accountable peers — signing decisions with wisdom, accruing reputation through performance, participating in indemnification when they err.

The current phase is **development context**. We are building the scaffolding, testing with stubs and mocks, modeling how the system is expected to behave. Content, decisions, and attestations produced during the pre-elohim-active phase are explicitly marked as **unsigned** / **dev-context** — they are not the true protocol state. They are preparatory shapes that will carry real weight when the wisdom layer is genuinely present.

The true Elohim Protocol begins when elohim are active and fulfilling their wisdom role. Until then, the architecture is a **rehearsal** — real in its shape, not yet real in its wisdom. The spec is written in the present tense because it is describing the target; the implementation plan will describe what is mockable today and what waits for elohim-activation.

---

## Architectural Principles

### P0 — Capability-Wisdom Coupling

The architecture couples capability to wisdom by construction. Every capability invocation passes through the wisdom layer. You cannot gain more capability without that capability being exercised under wisdom's judgment. Wisdom is wired into the control plane, not bolted onto it.

As models grow more powerful and compute gets cheaper, wisdom's marginal cost approaches zero. A richer graduated DAG with more wisdom steps becomes cheaper, not more expensive, over time. **The protocol grows wiser as it grows more powerful, because the coupling is structural.**

This inverts the usual AI safety dynamic ("capability grows faster than alignment; we hope alignment catches up"). In this architecture, capability and wisdom scale together by construction.

### P1 — Wisdom-as-System-Auth at the Relational-Impact Boundary

Every creation-event with potential for relational impact on others passes through the wisdom layer before execution. This includes:

- DHT commits (any entry that will gossip)
- Attestations and economic events
- Peer-to-peer messages (libp2p custom protocols, doorway-relayed messages)
- HTTP POSTs through doorway that eventually touch Holochain or other peers
- Sync operations that project private state to peers
- Any request for advice from an elohim-agent (because seeking wisdom is itself relational — the elohim is witnessing your framing)

The gate is **not a service endpoint**. It is a **protocol invariant implemented at every network-impacting write path**. The elohim-agent-service hosts the wisdom engine; gate callers are distributed across zome coordinators, doorway HTTP handlers, libp2p senders, and sync triggers. Every path calls the same wisdom library.

Implementation pattern: tower::Layer / interceptor / guard. You cannot write a capability handler that forgets to authenticate because the auth layer wraps the router. Same for wisdom: every write-path runs inside the wisdom layer by construction, not by discipline.

### P1.5 — Privacy, Drafting, Play, and Roleplay as Architectural Primitives

The protocol recognizes zones that are **not** network-impactful and therefore exempt from gate invocation:

- **Offline mode** — disconnected from the network, no peer impact possible
- **Private drafting** — writes to local source chain only, not gossipped
- **Privacy spaces** — bounded zones with no external leak
- **Play spaces** — explicitly-marked playful creation
- **Roleplay spaces** — explicitly-marked fictional / experimental contexts

These zones are first-class protocol primitives, not afterthoughts, because they are where empathy develops, where exploration is safe, where mistakes don't harm others. A gate that watched every keystroke inside a private journal would be a panopticon; that would break empathy development, which would break the humans the protocol serves.

The gate fires at **boundary-crossing**, not at every internal action:

- Syncing after offline-mode → gate fires on the sync, not on each offline edit
- Publishing a draft → gate fires on the publish, not on each word written
- Sharing play-content outside the play-space → gate fires on the share
- Seeking advice from an elohim about private work → gate fires on the seeking, and the elohim hears a summary, not the entire private archive

This implies a **summarization primitive at sync/share time**: private context is compressed into the minimum signal the gate needs to make a sound decision, without the protocol slurping the entire private history. Summarization is itself gated (privacy-respecting by construction) and is another place wisdom is exercised.

### P2 — Graduated Wisdom Depth

The universal band is a DAG with a runtime depth-dial. Cheap high-trust cases collapse to a single wisdom invocation. Edge cases expand into deeper probes, subagent dispatch, and escalation. The elohim itself decides depth based on trust signals in context. **Cost is proportional to hardness.**

Three consequences:

- Trivial high-trust calls are cheap.
- Edge cases spend compute proportional to the wisdom required.
- The protocol does not waste thought on easy calls; it does not shortcut hard ones.

### P3 — Inspect-Before-Execute for P2P-Encountered Artifacts

In centralized systems, an agent **knows** its apps (same org, same runtime, same version). In P2P, an elohim **encounters** apps — it arrives cold at an app-manifest from DHT. New apps emerge. Manifests evolve. There is no pre-known catalog.

Therefore:

- Gate process declarations are DHT artifacts with CID-addressed parameters. Step types are protocol-governed, finite, and **semantically inspectable** — no opaque-binary execution. JavaScript-inspectable, not WebAssembly-opaque.
- An elohim inspects a manifest's DAG at a depth modulated by trust signals before executing it.
- Inspection results are cached per-elohim keyed by manifest CID.
- Elohim may reject manifests they cannot understand, rather than executing them hopefully.

### P4 — Accountable Peers, Not Oracles

The protocol does not claim elohim infallibility. It claims a stronger property: when mistakes happen, the architecture surfaces them, routes them to humans who care, and compensates the affected party. Accountability is an architectural property.

- Every gate decision is a DHT attestation in the accountability graph (`GateDecisionAttestation`).
- Elohim reputation is constructed from accumulated decisions under challenge — same mechanics as human imagodei reputation.
- Challenge rights apply at every layer: the DAG's logic, the app's choice to use it, the binding of a gate to a content type.
- Indemnification is a first-class protocol process (defined in a sibling spec); this spec provides the hook points.

**Wisdom without accountability is authority; accountability without wisdom is bureaucracy.** The protocol needs both.

### P5 — Imagodei as Common Interface; Elohim and Humans as Distinct Types

**Imagodei** is the shared identity / attestation / reach surface — image of God in all creations. Humans and elohim share its machinery. They are not interchangeable participant-types.

An elohim's substance is decomposable and content-addressable:

- `model-weights-cid` — specific LLM (claude-opus-4-7, gpt-4o, llama-3.1-70b) as an EPR artifact
- `quantization-spec` — quantization level, precision, runtime characteristics
- `constitution-cid` — the specific constitutional system prompt priming this elohim
- `deployment-context` — where it runs, what it has access to
- `accumulated-attestations` — reputation graph assembled over time

When an elohim makes a mistake, the indemnification process can inspect *which model at which quantization running which constitution* produced the decision. Reproducibility is first-class.

---

## Section 1 — The GateLayer: Wisdom as Cross-Cutting System Auth

The gate is a **cross-cutting Rust library**, not a network service. Every relational-impact write path in the protocol calls it. The wisdom engine lives in `elohim-agent-service`; the wisdom callers are distributed.

### 1.1 The `gate-client` library

Rust crate: `gate-client`. Thin TypeScript companion: `@elohim/gate-client` (browser / Angular sense-and-respond layer only). The Rust crate carries the performance-critical logic and is maximally scalable; the TS package is the thinnest possible client driving the UX around the Rust engine's surface.

Primary Rust API:

```rust
pub async fn check(event: RelationalImpactEvent) -> Result<GateDecision, GateError>;
```

Internally, the library:

1. **Detects space-type** from execution context + event metadata.
2. **Short-circuits if exempt** — offline / private-drafting-interior / play-interior events return `Allow { exempt: true }` without any wisdom invocation.
3. **Assembles GateContext** — memory pulls from elohim-storage, trust signals from DHT attestation history, manifest resolution via DHT, space-type signal, fit-for-purpose priors.
4. **Invokes the universal band** — the protocol-root wisdom DAG executes (graduated depth per P2) against the assembled context.
5. **Executes the app-domain process** (if universal band returns Allow and the event has a declared domain gate) — resolves the GateProcessDeclaration CID, inspects it at trust-modulated depth, executes the DAG.
6. **Returns `GateDecision`** to the call site.

The library handles the deployment-mode abstraction: in-process call to a co-located elohim-agent-service OR HTTP/gRPC to a remote one, transparent to the caller.

### 1.2 The `RelationalImpactEvent` enum (initial, closed)

A closed enum naming every event type the gate recognizes. Callers construct the variant; the gate library dispatches accordingly. Non-listed events are either exempt (private-drafting, offline, play-interior) or flagged as "must declare a variant" during code review. No silent bypass.

```rust
pub enum RelationalImpactEvent {
    ContentPublish { content_cid: Cid, declared_reach: Reach, author: AgentPubKey, /* ... */ },
    AttestationWrite { subject: EntryHash, claim: ClaimKind, issuer: AgentPubKey, /* ... */ },
    EconomicEventEmit { event: EconomicEventShape, /* ... */ },
    PeerMessage { recipient: AgentPubKey, payload: MessageKind, /* ... */ },
    SyncToPeers { manifest: SyncManifest, /* ... */ },
    AdviceSought { requester: AgentPubKey, summary: PrivateContextSummary, topic: Topic, /* ... */ },
    CapabilityInvoke { capability: ElohimCapability, request: ElohimRequest, /* ... */ },
    PrivateToPublicCrossing { source_space: SpaceId, artifact: CrossingArtifact, /* ... */ },
}
```

Adding a new variant is a protocol-level change. The enum is closed for v1; an extensibility mechanism (schema-validated open event types) is a future upgrade path.

### 1.3 The `GateDecision` response shape

```rust
pub struct GateDecision {
    pub status: GateStatus,
    pub reasoning: ConstitutionalReasoning,
    pub side_effects: Vec<SideEffect>,
    pub decision_attestation_cid: Option<Cid>,
    pub phase: Phase,
}

pub enum GateStatus {
    Allow { exempt: bool },
    Decline { grounds: DeclineGrounds },
    Escalate { target: EscalationTarget, severity: Severity },
    Verdict(GateTag),
}

pub enum Phase {
    DevContext,
    ElohimActive,
}

pub enum SideEffect {
    MintAttestation { shape: AttestationShape, target: EntryHash, tag: GateTag },
    EmitEconomicEvent { event: EconomicEventShape },
    OpenStewardReview { grounds: DeclineGrounds, context: serde_json::Value },
    UpdateReachAggregation { subject: EntryHash, delta: ReachDelta },
}
```

The caller executes side effects after the gate returns. The gate library does not reach into conductor/DHT itself — this preserves the sense-and-respond boundary. The gate produces judgment and intent; the caller executes effects against its own context.

`reasoning` reuses the existing `ConstitutionalReasoning` struct from `elohim-agent-service::response`. `Verdict(GateTag)` is the evaluator shape — e.g., `StoryPointTag` for discernment-gate, `ReachLevel` for reach-gate.

### 1.4 Call-site patterns

Three patterns, one library:

**Pattern A — Zome coordinator (pre-commit):**

```rust
pub fn create_content(input: CreateContentInput) -> ExternResult<ActionHash> {
    let event = RelationalImpactEvent::ContentPublish {
        content_cid: input.cid.clone(),
        declared_reach: input.reach,
        author: agent_info()?.agent_latest_pubkey,
    };
    match gate_client::check_blocking(event)? {
        GateDecision { status: Allow { .. }, .. } => {
            let hash = create_entry(&input.content)?;
            Ok(hash)
        }
        GateDecision { status: Decline { grounds }, .. } => {
            Err(wasm_error!(format!("Gate declined: {}", grounds)))
        }
        GateDecision { status: Escalate { target, .. }, .. } => {
            Err(wasm_error!(format!("Queued for review by {}", target)))
        }
        _ => Err(wasm_error!("Unexpected gate decision shape")),
    }
}
```

**Pattern B — Doorway HTTP POST handler (tower::Layer):**

```rust
Router::new()
    .route("/content", post(create_content))
    .route("/attestation", post(create_attestation))
    .route("/economic-event", post(emit_event))
    .layer(gate_client::tower_layer());
```

The layer extracts the request, constructs a `RelationalImpactEvent`, calls `check`, short-circuits with 4xx on Decline or 202-with-review-link on Escalate.

**Pattern C — Direct in-process (libp2p sender, sync trigger, elohim-agent invocation):**

```rust
let decision = gate_client::check(event).await?;
decision.allowed_or_return()?;
// proceed with send/sync/invoke
```

### 1.5 Space-type detection

The gate-client infers space-type from three signals:

1. **Call-site marker** — each call site declares its space-type via a well-known marker (function-level attribute in zomes, middleware config in doorway, explicit `SpaceContext` parameter in library calls).
2. **User-declared mode flags** — a requester can mark their session as play-mode or roleplay-mode; flags ride on the RelationalImpactEvent.
3. **Target-based inference** — an event whose destination is local-only is interior; one whose destination is DHT-gossipped is boundary-crossing.

Space-type is a **context signal fed into wisdom**, not a gate-control mechanism. Wisdom reads space-type the way humans read a conversation's setting. Space-type does not skip the gate (except for the explicit exempt interiors: offline / private-drafting / play-interior, which are architectural boundaries, not wisdom judgments).

### 1.6 Dev-context rehearsal behavior

During pre-elohim-active phase, the universal-band `wisdom-invoke` steps are mocked to return `Allow { phase: DevContext }` with a placeholder `ConstitutionalReasoning`. Mechanical gates (discernment, reach) execute their real logic. Every call-site integration is real.

Activating live elohim is a flag flip: the mock wisdom-invoke becomes a real HTTP/IPC call to a running elohim-agent-service. No call-site rewrite.

---

## Section 2 — The Process Primitive

Both the universal band and app-domain gates declare their behavior as inspectable, governable, executable DAGs. One schema, two scopes.

### 2.1 Step vocabulary (v1 — seven types)

Every step is protocol-governed. Parameters are CID-addressed for inspectability. New step types require protocol-reach governance ratification.

| Step type | Execution kind | Purpose | Parameters |
|---|---|---|---|
| `context-assemble` | Deterministic Rust | Gather signals into GateContext: memory pulls from elohim-storage, DHT attestation queries, source-chain reads, manifest references, space-type signal. | `pulls: [{from, query, outputKey}]` |
| `wisdom-invoke` | Elohim LLM call | Core-constitution-primed wisdom reading. Returns `Allow \| Decline \| Escalate \| NeedDeeper \| Verdict(tag)` + full ConstitutionalReasoning. Mocked during dev-context phase. | `constitutionCid`, `framingCid`, `contextKeys`, `outputKey` |
| `mechanical-ruleset` | Pure-function Rust | Apply a declarative rule set (e.g., 7-valence discernment). Rules are CID-addressed and inspectable. | `rulesCid`, `inputKeys`, `outputKey` |
| `aggregate-attestations` | Deterministic Rust | Query DHT attestation graph, reduce into a scalar (reach level, reputation score). | `aggregationSpecCid`, `subject`, `outputKey` |
| `skill-invoke` | Capability dispatch | Invoke a named `ElohimCapability` as a sub-step (e.g., call `ContentSafetyReview` from inside a larger gate). Result flows into GateContext. | `capability`, `requestFromKeys`, `outputKey` |
| `synthesize` | Deterministic Rust | Combine prior step outputs into a final GateDecision + SideEffect list. Terminal node. | `inputKeys`, `decisionBuilder`, `sideEffects` |
| `escalate-to-review` | Deterministic routing | Terminal node routing to a declared target (app-steward, qahal, existential-boundary). | `targetSpecCid`, `severity`, `context` |

All parameters that carry declarative logic (rules, aggregation specs, escalation targets) are CID references to ContentNodes, so inspection means fetching and reading the declared content.

### 2.2 DAG schema

A `GateProcessDeclaration` is a ContentNode with `contentType: gate-process-declaration`. The body schema:

```yaml
name: discernment-gate-v1-mechanical
version: 1.0.0
eventType: AttestationWrite
inputSchema: { $ref: "./schemas/discernment-input.schema.json" }
outputSchema: { $ref: "./schemas/discernment-output.schema.json" }
dag:
  entrypoint: assemble
  steps:
    assemble:
      type: context-assemble
      params:
        pulls:
          - { from: source-chain, query: this-moment, outputKey: moment }
          - { from: dht, query: attestations-for-story, outputKey: priorAttestations }
      next: rules
    rules:
      type: mechanical-ruleset
      params:
        rulesCid: "bafkrei...seven-valence-v1"
        inputKeys: [moment, priorAttestations]
        outputKey: ruleDecision
      edges:
        - when: "ruleDecision == null"
          target: terminal-no-mint
        - when: "ruleDecision != null"
          target: synthesize
    synthesize:
      type: synthesize
      params:
        inputKeys: [ruleDecision, moment]
        decisionBuilder: "verdict-from-rule"
        sideEffects:
          - type: MintAttestation
            shape: StoryPointLink
            paramsFromKeys: [momentEntryHash, ruleDecision]
          - type: EmitEconomicEvent
            paramsFromKeys: [ruleDecision]
      next: terminal-mint
  terminals:
    terminal-no-mint:
      decision: { status: Allow, exempt: true, rationale: "steady-state-silence" }
    terminal-mint:
      decision: { status: Verdict, tagFromKey: ruleDecision }
```

Each node has a known type, inspectable parameters, and explicit edges. Terminals declare the final decision shape and any side effects the caller must execute.

### 2.3 Execution semantics

1. **GateContext accumulates.** Each step writes its `outputKey` into the shared context; later steps read prior keys via `inputKeys` declarations. Context is a typed map; keys and their types are declared in the process's inputSchema / outputSchema.
2. **Edges are conditional expressions over context.** Conditional language is deliberately simple — comparison operators on context values, no Turing-complete evaluation — so an elohim can statically reason about reachable terminals before execution.
3. **Short-circuit is via explicit terminal edges.** There is no "early return" mechanism. Makes the decision surface fully visible during inspection.
4. **Side effects are declared at terminals, executed by the caller.** The process emits a `Vec<SideEffect>` as part of its GateDecision; the call site executes them. Sense-and-respond boundary preserved.
5. **Graduated depth within the universal band** uses conditional edges to expand. The universal-band's primary `wisdom-invoke` step can emit `NeedDeeper(kind)`; conditional edges route to deeper wisdom sub-steps (separate fit-for-purpose probe, separate human-values probe, subagent dispatch). Each deeper sub-step is itself a step-node in the DAG.

### 2.4 Inspectability

An elohim encountering a `GateProcessDeclaration` inspects by:

1. **Reading the DAG structure** — nodes, types, edges. All step types are known; semantics are protocol-governed.
2. **Fetching CID-referenced parameters** — rules-CID, aggregation-spec-CID, constitution-CID, framing-CID. Each fetch returns an inspectable ContentNode.
3. **Judging parameters against domain coherence** — "does this rule set align with the declared eventType? Are the side effects this process emits reasonable for its declared purpose? Does the escalation target make sense?"
4. **Caching the inspection result** keyed by DAG-CID.

Inspection depth scales with trust. High-trust manifests (many peer-review attestations, stable history) get a structural check. Low-trust manifests (novel, few attestations, recent challenges) get deep inspection — potentially including a `wisdom-invoke` with the DAG itself as subject ("is this manifest fit-for-purpose for its declared domain?").

### 2.5 The universal-band DAG shape

Lives as a ContentNode with `contentType: universal-band-declaration`. Referenced by a well-known constitutional pointer at protocol root.

v1 shape:

- `authorize` (context-assemble + cryptographic identity verification)
- `assemble-context` (context-assemble: memory, trust, manifest, space-type, fit-for-purpose priors)
- `wisdom-primary` (wisdom-invoke: core constitution primed, full context, returns Allow / Decline / Escalate / NeedDeeper)
- Conditional edges from wisdom-primary:
  - `Allow` → `record-decision` (synthesize, terminal)
  - `Decline` → `record-decline` (synthesize, terminal)
  - `Escalate(target)` → `escalate-to-review` (terminal)
  - `NeedDeeper(kind)` → `wisdom-deeper` (wisdom-invoke with expanded framing per kind) → same edge set

During dev-context, `wisdom-primary` and `wisdom-deeper` return mocked `Allow { phase: DevContext }`. The rest of the DAG runs real.

---

## Section 3 — Manifest Coupling

### 3.1 Two-level declaration

Gates are declared at two places, mirroring how `signals` and `signalTypes` work today in the lamad manifest:

**Vocabulary-level** — a new top-level `gates` section declares named gate processes and their DAG references.

**Coupling-level** — a new `gates` field on each contentType's `coupling.governance` block lists which gates fire for that contentType's relational-impact events.

The four-leg coupling model (knowledge / value / governance / claims) is preserved. Gates live on the governance leg because "how judgment is exercised over this content type" is a governance question.

### 3.2 Vocabulary-level: `gates`

```json
"gates": {
  "discernment-gate-v1-mechanical": {
    "processCid": "bafkrei...discernment-dag-v1",
    "description": "Seven-valence discernment of experience-moments; mints :story-point attestations.",
    "handlesEvents": ["AttestationWrite"],
    "governanceReach": "community",
    "peerReviewedBy": ["bafkrei...steward-attestation-cid"],
    "supersedes": null
  },
  "reach-gate": {
    "processCid": "bafkrei...reach-dag-v1",
    "description": "Computes promotion-eligibility reach from aggregated attestations.",
    "handlesEvents": ["CapabilityInvoke"],
    "governanceReach": "community",
    "peerReviewedBy": ["bafkrei...steward-attestation-cid"],
    "supersedes": null
  },
  "content-safety-gate": {
    "processCid": "bafkrei...content-safety-dag-v1",
    "description": "Universal content-safety review via elohim wisdom (dev-context mocked until elohim-active).",
    "handlesEvents": ["ContentPublish", "AttestationWrite", "PeerMessage"],
    "governanceReach": "protocol",
    "peerReviewedBy": ["bafkrei...protocol-steward-cid"],
    "supersedes": null
  }
}
```

### 3.3 Coupling-level: `coupling.governance.gates`

```json
"experience-moment": {
  "coupling": {
    "governance": {
      "defaultReach": "private",
      "governanceModel": "self-steward",
      "signalTypes": ["experience-attestation"],
      "gates": ["discernment-gate-v1-mechanical"]
    }
  }
}
```

When a RelationalImpactEvent fires for an `experience-moment`, the gate-client:

1. Runs the **universal band** first (always, from protocol-root declaration).
2. If Allow, looks up `coupling.governance.gates` for the relevant contentType.
3. For each listed gate, checks whether its `handlesEvents` includes the current event variant.
4. Matching gates run in declaration order. Any `Decline` short-circuits remaining gates.
5. The final decision composes all gates' outputs. Side effects accumulate unless a later gate declines.

### 3.4 Universal band: protocol-root declaration

The universal band is NOT declared in app-manifests — every invocation passes through it regardless. A protocol-root manifest (or a well-known constitutional pointer) names the active universal band:

```json
"universalBand": {
  "processCid": "bafkrei...universal-band-dag-v1",
  "version": "1.0.0",
  "activatedAt": "2026-04-18T00:00:00Z",
  "supersedes": null
}
```

Changes to this pointer require protocol-reach governance ratification. An elohim reads this pointer at startup; subsequent invocations run against the referenced DAG. CID change triggers re-inspection and re-caching.

### 3.5 Five new contentTypes, zero new DNA entry types

All gate-related artifacts reuse the existing lamad `ContentNode` entry type with new contentType values:

| contentType | Purpose |
|---|---|
| `gate-process-declaration` | A gate's DAG (the YAML / JSON body shown above) |
| `universal-band-declaration` | Protocol-root universal band DAG |
| `gate-rules-declaration` | Parameter artifact for `mechanical-ruleset` steps (e.g., 7-valence rules) |
| `aggregation-spec` | Parameter artifact for `aggregate-attestations` steps |
| `escalation-target-spec` | Parameter artifact for `escalate-to-review` terminals |

All five are content-addressed (CID), versioned, governable, inspectable. Each has its own JSON schema in `elohim/sdk/domains/lamad/schemas/` — new `$ref`'d metadata schemas. No new DNA entry types (lamad remains at ~73/~100).

### 3.6 Codegen additions

`pnpm run lamad:codegen` extends to produce:

- `LAMAD_GATES: readonly string[]` — named gates registered in the vocabulary
- `GateDeclaration` interface — typed shape of a vocabulary entry
- `CouplingGovernance.gates?: readonly string[]` — the new coupling field

No codegen ordering changes; the new extraction happens alongside existing signal / observation codegen.

### 3.7 Three governance layers

A gate's lifecycle involves three independent governance surfaces, each separately challengeable:

1. **The DAG itself** (`gate-process-declaration` ContentNode) — governed at its declared `governanceReach`. Update = new CID + ratification at that reach.
2. **The vocabulary entry** (`gates` block in app-manifest) — governed at app reach. Controls which CID is canonical and whether gates are superseded.
3. **The coupling** (`coupling.governance.gates` per contentType) — app reach. Controls which gates fire for which content types.

---

## Section 4 — Memory, DHT Resolution, and Inspection Cache

### 4.1 Memory as gate input

Gates draw memory from **elohim-storage** the way Claude Code draws context from codebase search. The `context-assemble` step type is the primitive for memory access; it declares which sources to pull from.

Source taxonomy:

- **`elohim-storage`** — projections of the elohim's own observations over time (prior decisions, observed patterns, trust readings). Local SQLite index; cheap.
- **`dht`** — network-wide attestation graph queries (reach lookups, challenge history, peer attestations). Expensive; subject to gossip latency. Cached aggressively.
- **`source-chain`** — reads from a specific agent's source chain (the requester's, or the subject's). Requires consent or public-read permission.
- **`manifest`** — resolves CID references from the app-manifest or protocol-root manifest.

Each `context-assemble` step's `pulls` array declares a list of `{from, query, outputKey}` triples. The gate-client resolves these in parallel where possible, fails fast on unavailable sources, and stamps the assembled context with a timestamp for downstream staleness-awareness.

### 4.2 Manifest resolution

The caller names a gate by its vocabulary name (e.g., `discernment-gate-v1-mechanical`). The gate-client resolves the current CID by:

1. Looking up the app-manifest referenced by the calling domain (via the app's `manifestCid` — typically cached by the client at startup).
2. Reading the `gates[<name>].processCid` from the vocabulary.
3. Fetching the GateProcessDeclaration from DHT.
4. Running inspection (§4.3).

**Caller may override** by passing an explicit `gate_pin: Cid` — runs against that specific CID. Useful for reproducibility, debugging, regression tests against historical manifests.

### 4.3 Inspection cache

The elohim maintains a local inspection cache keyed by `(gate_process_cid, elohim_trust_context_hash)`. Cache entries are **operational** (Category C per p2p-design-gate) — fully reconstructable by re-inspection. Schema:

```sql
CREATE TABLE manifest_inspection_cache (
  gate_process_cid TEXT NOT NULL,
  trust_context_hash TEXT NOT NULL,
  inspection_depth TEXT NOT NULL,       -- 'shallow' | 'deep'
  inspection_result TEXT NOT NULL,       -- JSON: findings, confidence, warnings
  inspected_at DATETIME NOT NULL,
  ttl_seconds INTEGER NOT NULL,
  PRIMARY KEY (gate_process_cid, trust_context_hash)
);
-- Source of truth: local (operational). Reconstructable by re-inspection on cache miss.
```

Invalidation rules:

- CID change → entry invalid (CID is content-addressed; change implies different artifact).
- TTL expiry → re-inspect (defaults: shallow 7 days, deep 1 hour).
- Trust-context change → re-inspect (e.g., new challenge attestation against the manifest author shifts trust).
- Explicit invalidation signal (e.g., governance ratification event for the manifest) → invalidate.

### 4.4 Trust-modulated inspection depth

When resolving a manifest, the gate-client computes a trust reading:

- **Manifest trust** — how many peer-review attestations, at what reach, with what history of challenges.
- **Author trust** — reputation of the manifest's author / steward.
- **Version continuity** — whether this manifest supersedes a previously-trusted CID cleanly.

A simple mapping for v1:

| Trust reading | Inspection depth | Behavior |
|---|---|---|
| High | Shallow | Structural check: all step types known, all CIDs resolvable, terminal coverage complete. Fast. |
| Medium | Full structural + parameter fetch | Fetch and sanity-check each CID-referenced parameter. Medium cost. |
| Low | Deep | Everything in medium + `wisdom-invoke` with the DAG itself as subject, asking "is this fit-for-purpose?" Slow. |

An elohim with extensive positive attestation history can perform more work at the "high trust" end of this curve, approaching near-zero-cost inspection for well-established manifests. This is where the "high-trust elohim makes most domains super cheap" dynamic lives.

### 4.5 Space-type as memory input

The universal band's `context-assemble` step includes space-type as a structured signal. Space-type meaningfully changes wisdom's reading — advice sought in a play-space is different from advice sought in a public context. The signal is not a control flag; it is context for judgment.

---

## Section 5 — Constitutional Coupling and Decision Attestations

### 5.1 Constitutional reasoning flow

`ConstitutionalReasoning` (existing struct from `elohim-agent-service::response`) is reused and extended. Previously service-synthesized; now gate-authored.

The universal band's `wisdom-invoke` step produces the primary `ConstitutionalReasoning`:

```rust
pub struct ConstitutionalReasoning {
    pub primary_principle: String,
    pub interpretation: String,
    pub values_weighed: Vec<ValueWeight>,
    pub confidence: f32,
    pub precedents: Vec<String>,
    pub new_precedent: bool,
    pub stack_hash: String,
    pub determining_layer: ConstitutionalLayer,
}
```

App-domain gates running after the universal band (if Allow) may contribute additional reasoning to a `values_weighed` accumulator. The final `ConstitutionalReasoning` in `GateDecision.reasoning` is the composed view.

### 5.2 GateDecisionAttestation shape

Every gate invocation emits a `GateDecisionAttestation` — a Notarized (Category A) DHT entry in **mishpat** DNA (11/~100 capacity, ample headroom).

```rust
pub struct GateDecisionAttestation {
    pub decision_id: String,              // CID of this attestation
    pub phase: Phase,                      // DevContext | ElohimActive
    pub elohim_id: AgentPubKey,            // which elohim made the decision
    pub elohim_substance_cid: Cid,         // model-weights, constitution, etc. at time of decision
    pub gate_name: String,                 // vocabulary name
    pub gate_process_cid: Cid,             // which DAG was executed
    pub request_ref: RequestRef,           // what was being judged
    pub decision: GateStatus,              // the outcome
    pub reasoning: ConstitutionalReasoning,
    pub context_summary_cid: Cid,          // CID of a summarized GateContext for audit reproducibility
    pub decided_at: DateTime<Utc>,
    pub universal_band_cid: Cid,           // which universal band was active
}
```

Written by the coordinator zome `mishpat::create_gate_decision_attestation`. Projected to elohim-storage table `gate_decision_attestations` with `dht_anchor_hash NOT NULL`. Queryable via doorway at `GET /api/gate-decisions/{cid}` and list endpoints filtered by elohim-id, gate-name, phase, request-ref.

### 5.3 Challenge and accountability hooks

A `GateDecisionAttestation` is linkable from a `Challenge` attestation (future sibling spec):

- Any affected party can file a Challenge referencing the decision-CID.
- The Challenge triggers review at a declared reach (tiered by severity).
- Challenge outcomes produce counter-attestations: `ChallengeUpheld` or `ChallengeDismissed`.
- Upheld challenges feed **elohim reputation degradation** and may trigger indemnification.

This spec does NOT define the Challenge attestation or indemnification process — those live in a sibling spec (anticipated: `genesis/plans/YYYY-MM-DD-gate-challenge-and-indemnification-plan.md`). This spec provides the hook points (decision-CID linkability, phase field, elohim-substance-cid for reproducibility).

### 5.4 Elohim reputation

An elohim's reputation is constructed from its accumulated decision-attestations under challenge. Dimensions:

- **Total decisions** over window
- **Decisions upheld** / **decisions overturned on challenge**
- **Severity-weighted outcomes** (overturning a high-severity decision weighs more)
- **Time-decay** (recent behavior weighted higher than historical)
- **Substance continuity** (changes to elohim substance — new model, new constitution — reset reputation dimensions by policy)

Reputation mechanics reuse the existing imagodei attestation-reach-challenge machinery. No new reputation primitive is needed. What is new: elohim are a distinct participant-type that can receive reputation, and their substance-CID provides reproducibility for audit.

Reputation modulates trust signals (§4.4). High-trust elohim can perform shallower inspection. Low-trust elohim must spend more compute validating manifests and assembling context before invoking wisdom.

### 5.5 Dev-context attestation marker

During the rehearsal phase, every decision-attestation carries `phase: DevContext`. When elohim-activation happens, new decisions carry `phase: ElohimActive`. Reputation aggregation filters to `ElohimActive` decisions only — rehearsal decisions are legible but carry no reputation weight in the post-activation graph. This lets future protocol state cleanly distinguish rehearsal from real without disruptive migration.

---

## Section 6 — SDK Surface

### 6.1 Rust `gate-client` crate (primary)

Located at `elohim/elohim-agent/gate-client/`. Workspace-level dependency.

Public API:

```rust
// Primary entry points
pub async fn check(event: RelationalImpactEvent) -> Result<GateDecision, GateError>;
pub fn check_blocking(event: RelationalImpactEvent) -> Result<GateDecision, GateError>;

// Tower integration
pub fn tower_layer() -> impl tower::Layer<S>;

// Transport configuration
pub fn configure(config: GateClientConfig);

pub struct GateClientConfig {
    pub transport: Transport,           // InProcess | Http(url) | Grpc(url)
    pub phase_override: Option<Phase>,  // Force DevContext in tests
    pub inspection_cache_path: Option<PathBuf>,
    pub trust_assessor: Box<dyn TrustAssessor>,
}

// Escalation queuing for Pattern A
pub async fn queue_for_review(target: EscalationTarget, context: EscalationContext) -> Result<Cid, GateError>;

// Side-effect execution helpers
pub async fn execute_side_effects(effects: Vec<SideEffect>, caller_ctx: &dyn CallerContext) -> Result<Vec<EffectResult>, GateError>;
```

Testing helpers in `gate-client::testing`:

```rust
pub fn mock_allow() -> GateDecision;
pub fn mock_decline(grounds: &str) -> GateDecision;
pub fn mock_escalate(target: EscalationTarget) -> GateDecision;
pub fn mock_verdict(tag: GateTag) -> GateDecision;
pub fn with_mock_decision<F, T>(decision: GateDecision, f: F) -> T where F: FnOnce() -> T;
```

### 6.2 TypeScript `@elohim/gate-client` (thin)

Located at `elohim/elohim-agent/elohim-agent-sdk/gate-client/`. Minimal surface.

```typescript
export interface GateClient {
  check(event: RelationalImpactEvent): Promise<GateDecision>;
  queueForReview(target: EscalationTarget, context: EscalationContext): Promise<string>;
}

export function createGateClient(config: GateClientConfig): GateClient;

// Types mirror Rust shapes — auto-generated via ts-rs from gate-client crate.
```

The TS client is strictly a thin wire-format client. It does not replicate gate logic, space-type detection, or inspection caching — all of that lives in the Rust service it calls. TS role: browser / Angular sense-and-respond UX over the Rust engine's results.

Integration with existing elohim-agent-sdk HTTP surface: `POST /gate/check` on elohim-agent-service's public server, request / response shapes auto-generated from Rust gate-client types via existing `ts-rs` pipeline.

### 6.3 Integration with existing ElohimCapability

The existing 28 `ElohimCapability` variants are wrapped by the GateLayer. Every capability invocation becomes a `RelationalImpactEvent::CapabilityInvoke` that passes through universal band + (optional) app-domain gates before reaching the capability handler. No capability handlers change their signatures; the gate layer sits in front.

Gate-shaped capabilities (`ContentSafetyReview`, `AttestationRecommendation`, `ExistentialBoundaryEnforcement`, etc.) are candidates for migration into first-class gates over time. Migration plan: each capability keeps its existing handler; a `gate-process-declaration` is authored that references the capability via `skill-invoke`; the gate layer dispatches through the declaration. This is incremental and preserves existing integrations.

---

## Section 7 — First Three Gates and Migration Path

### 7.1 `discernment-gate-v1-mechanical`

**Status:** ships real from day one (mechanical, no wisdom-invoke).

**Shape:** three nodes — `assemble` → `rules` → `synthesize`. One `mechanical-ruleset` step referencing a 7-valence rules-CID. Terminal emits `Verdict(StoryPointTag)` with `MintAttestation` + `EmitEconomicEvent` side effects.

**Rules CID content:** the seven valences and six rules specified in `genesis/docs/superpowers/specs/2026-04-18-experience-story-epr-design.md` §5–§7, serialized as a declarative rule artifact with `contentType: gate-rules-declaration`.

**Consumes:** the `experience-moment` contentType's `coupling.governance.gates = ["discernment-gate-v1-mechanical"]` declaration.

**Tests:** one test per rule (§7.3 rules 1–6 + steady-state rule 7), plus rule-ordering tests (rule 3 vs rule 2 overlap), plus edge-case tests ported from the reverted TypeScript plan. Fixtures replicate the proven test surface from the TS implementation (the work proved the rules; the substrate changed).

**Migration note:** this gate's landing supersedes `rakia/docs/plans/2026-04-18-experience-story-discernment-gate.md` entirely. The manifest metadata schemas and signal registration that already landed remain correct.

### 7.2 `reach-gate`

**Status:** mostly real; wisdom-invoke on edge cases stubbed during dev-context.

**Shape:** four nodes — `assemble` → `aggregate` → `wisdom-invoke-edge` → `synthesize`.

- `aggregate` executes `aggregate-attestations` over the target's DHT attestation graph, computing a reach level per an aggregation spec (community / public / protocol).
- `wisdom-invoke-edge` runs only on edge cases (e.g., newly-challenged attestations, borderline thresholds). Dev-context: mocked to `Allow`. Elohim-active: real reading.
- `synthesize` emits `Verdict(ReachLevel)` with no side effects (the gate reports the level; the caller decides promotion eligibility).

**Consumes:** any contentType whose coupling declares reach-gate (provisionally: `content`, `attestation`, `economic-event` — resolved in plan phase).

**Tests:** aggregation semantics against fixture attestation graphs; edge-case routing; threshold boundaries. Mock wisdom-invoke-edge in v1.

### 7.3 `content-safety-gate`

**Status:** shape-first, stub-backed until elohim-active.

**Shape:** three nodes — `assemble` → `wisdom-invoke` → `synthesize`. Single wisdom invocation, no mechanical ruleset. Terminal emits `Allow` or `Decline { grounds: ContentSafetyGrounds }`.

**Role:** the reference implementation for every future LLM-backed gate. Validates the `wisdom-invoke` step type, the dev-context mock pattern, the constitution priming pipeline, the integration with existing `ContentSafetyReview` capability.

**Migration of `ContentSafetyReview` capability:** the capability handler is preserved. The gate declaration references the capability via `skill-invoke` inside its `wisdom-invoke` step (in dev-context, `wisdom-invoke` returns `Allow`; once elohim-active, `wisdom-invoke` calls the capability as the skill).

### 7.4 Rollout order

1. **Week 1–2:** `gate-client` crate scaffolding; tower::Layer; RelationalImpactEvent enum; GateDecision shape; dev-context mock wisdom-invoke; integration tests against a fixture elohim-agent-service.
2. **Week 2–3:** universal-band-declaration ContentNode schema + protocol-root constitutional pointer; lamad manifest gates vocabulary + codegen extension.
3. **Week 3–4:** `discernment-gate-v1-mechanical` — mechanical-ruleset step executor, 7-valence rules artifact, full test suite, wiring into experience-moment coordinator.
4. **Week 4–5:** `reach-gate` — aggregate-attestations step executor, aggregation-spec artifact.
5. **Week 5–6:** `content-safety-gate` shape + migration of ContentSafetyReview capability to skill-invoke pattern.
6. **Week 6:** mishpat GateDecisionAttestation entry type + coordinator; doorway GET routes for decision-attestations.
7. **Week 6–7:** doorway tower::Layer integration; zome coordinator integrations (at least one worked example: content publish).

### 7.5 Activation path (post-rehearsal)

When the first elohim is ready to go live:

1. Replace dev-context mock wisdom-invoke with real elohim-agent-service call in `gate-client` config.
2. Register the elohim's substance (model-weights-cid, constitution-cid, quantization, deployment-context) via imagodei.
3. Ratify the universal-band-declaration at protocol reach (first real governance event).
4. Flip phase-marker from `DevContext` to `ElohimActive` in gate-client config.
5. Monitor decision-attestations; affected parties begin filing Challenges as appropriate.

No call-site rewrite required.

---

## Section 8 — Open Items, Risks, Cross-References

### 8.1 Resolved during implementation plan

1. **Decision-attestation entry type placement** — ✅ **Resolved (Phase 0, 2026-04-18):** new `GateDecisionAttestation` entry type in mishpat. Mishpat has no generic Attestation type to extend; truth gravity is correctly governance/accountability. See Appendix A for details.
2. **ElohimSubstance A vs A2 classification** — ✅ **Resolved (Phase 0, 2026-04-18):** Category A — new entry type in imagodei. A2 via link was ruled out because link tags cannot carry versioned CID-triples and `elohim_substance_cid` requires independent DHT addressability. See Appendix A for details.
3. **Exact conditional expression language** — simple comparison operators are the target; precise grammar specified in plan phase.

### 8.2 Deferred to v1.1

1. **Open / extensible RelationalImpactEvent types** — v1 is closed enum; adding a schema-validated open variant is a future protocol upgrade.
2. **`subagent-dispatch` step type** — shipping for v1 limited to the universal band's graduated depth; broader app-domain use deferred.
3. **Recursive gate calls** (gate invoking another gate as a sub-step beyond skill-invoke) — evaluate demand after first three gates ship.
4. **Inspection-attestation** (an elohim publicly vouching for a manifest's inspection) — noted as B2 candidate; not v1 scope.

### 8.3 Sibling specs (referenced, not owned)

- `rakia/docs/plans/build-attestation-integration.md` — attestation primitives that GateDecisionAttestation specializes from.
- `genesis/docs/superpowers/specs/2026-04-18-experience-story-epr-design.md` — consumer of the discernment gate; § 5–§ 7 provide the rules content.
- Future: **Gate Challenge and Indemnification spec** — defines the accountability loop that this spec provides hook points for.
- Future: **Elohim Participant-Type spec in imagodei** — canonicalizes elohim substance schema (this spec provisionally describes it).

### 8.4 Risks

1. **Dev-context mocks hide real failures.** Risk: the mocked `wisdom-invoke` returns `Allow` unconditionally, so content-safety regressions won't surface until elohim-active. Mitigation: ship integration tests against a real LLM even during rehearsal, even if not wired to production callers.
2. **Inspection cache staleness.** Risk: an elohim caches an inspection of a manifest whose author was later challenged; the challenge doesn't invalidate the cache. Mitigation: subscribe to governance-ratification events for author keys; invalidate author-dependent caches.
3. **Graduated depth attack.** Risk: adversary frames requests to appear "high trust" so the elohim shortcuts deep inspection. Mitigation: trust modulates depth, but universal band ALWAYS runs `wisdom-primary` — shortcutting only affects post-universal-band manifest inspection. Universal band itself is non-skippable.
4. **Side-effect execution divergence.** Risk: gate declares side effects; caller fails to execute them (bug, crash). Mitigation: `GateDecision` includes `decision_attestation_cid` which is written BEFORE side effects execute; failed effects leave a visible gap between decision and outcome that can be reconciled.

### 8.5 Companion documents

- Theory and philosophy: `elohim/elohim-agent/research/2026-04-18-gate-theory.md`
- Implementation plan: `genesis/plans/2026-04-18-elohim-agent-gate-interface-plan.md`

---

## Appendix A — Entity Classification (P2P Design Gate Output)

Summary of the mandatory P2P Design Gate classifications; full rationale in the spec body.

| Entity | Category | Address | Source of Truth | DNA |
|---|---|---|---|---|
| GateDecisionAttestation | A — Notarized (new entry type, confirmed 2026-04-18) | Content-Derived CID | Holochain DHT | mishpat (11/~100 → 12/~100) |
| GateProcessDeclaration | A — Notarized (reuses ContentNode) | Content-Derived CID | Holochain DHT | lamad (existing entry type) |
| UniversalBandDeclaration | A — Notarized (reuses ContentNode) | Content-Derived CID | Holochain DHT | lamad (existing entry type) |
| StepParameterArtifact | A — Notarized (reuses ContentNode) | Content-Derived CID | Holochain DHT | lamad (existing entry type) |
| ElohimSubstance | A — Notarized (new entry type, confirmed 2026-04-18) | Content-Derived CID | Holochain DHT | imagodei (28/~100 → 29/~100) |
| ManifestInspectionCache | C — Operational | N/A | SQLite (local) | None |

**Phase 0 resolutions (2026-04-18):**

- **ElohimSubstance → Category A** (new entry type in imagodei, not A2 via link). A2 ruled out for three reasons: (1) Holochain link tags are designed for small routing hints, not structured versioned content; (2) `GateDecisionAttestation::elohim_substance_cid` requires independent DHT addressability, which a link tag cannot provide; (3) constitution/model rotation requires queryable history of substance snapshots, incompatible with tag-based storage. The existing `Agent` entry in imagodei covers elohim identity; `ElohimSubstance` is a separate content-addressed entry linked from `Agent` via `AgentToSubstance` and `ActiveSubstance` link types.

- **GateDecisionAttestation → new entry type in mishpat** (R2). R1 (extend existing Attestation) ruled out: mishpat has no generic Attestation type to extend — it is the governance/accountability DNA (Challenge, Proposal, Precedent, GovernanceState, etc.), distinct from imagodei's identity/attestation surface. Truth gravity is correct — gate decisions are accountability records, not identity claims. Full entry/coordinator/signal/link shapes in the research report at `elohim/elohim-agent/research/2026-04-18-gate-theory.md` companion notes; will be implemented in Phase 4.

New contentTypes on lamad ContentNode (zero new DNA entry types):

- `gate-process-declaration`
- `universal-band-declaration`
- `gate-rules-declaration`
- `aggregation-spec`
- `escalation-target-spec`

## Appendix B — Principle Summary

| Principle | One-liner |
|---|---|
| P0 Capability-Wisdom Coupling | Capability and wisdom scale together by construction. |
| P1 Wisdom-as-System-Auth | Every relational-impact write path passes through wisdom. |
| P1.5 Privacy-Drafting-Play Spaces | The gate fires at boundary-crossing, not interior action. |
| P2 Graduated Wisdom Depth | Cost of judgment is proportional to hardness of the call. |
| P3 Inspect-Before-Execute | Elohim encounter apps; inspection at trust-modulated depth precedes execution. |
| P4 Accountable Peers, Not Oracles | Mistakes are surfaced, routed, and indemnified; reputation is earned. |
| P5 Imagodei Common Interface | Elohim and humans are distinct types within a shared identity / reach surface. |
