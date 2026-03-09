# Coordination Verb Layer Design

**Date:** 2026-03-09
**Status:** Approved
**Scope:** TypeScript interfaces (IoC contracts) for distributed coordination verbs

## Problem

The Elohim Protocol has rich economic vocabulary (21 REA actions), governance primitives (deliberation, consent, councils), and trust topology (reach levels, attestation gates) — but no unified coordination layer that composes them for distributed execution. Three converging use cases need this:

1. **Council coordination** — hierarchical governance sense-and-respond across family -> neighborhood -> city -> bioregion -> global
2. **Compute routing** — ephemeral devices (phones, watches, PoS) delegating inference to capable nodes
3. **Mutual aid** — bootstrap/disaster/seasonal compute solidarity between regions

All three are the same pattern: an agent expresses a need, the mesh routes it to capable/authorized peers, work happens, and the economic story is recorded.

## Architecture Decisions

### TypeScript interfaces first, Rust later
Define IoC contracts as TypeScript interfaces now. Implement as Rust types in elohim-storage (Holochain entries for auditable verbs) and elohim-node (wire protocol for ephemeral verbs) later.

### Envelope wraps existing types (not extends)
`InvocationEnvelope` is a domain concept, not a transport concept. `WireMessage` serializes it for libp2p. Holochain entries store on-chain verbs. The envelope itself is the coordination primitive.

### Integrity boundary
- **On-chain** (Holochain entries, DHT-gossipped): `delegate`, `ratify`, `recall`, `provision`, `federate`, `escalate` — anything that changes authority or creates economic obligation
- **Wire-only** (libp2p messages, ephemeral): `invoke`, `sense`, `respond`, `route`, `aggregate` — transient coordination messages

### Scope composes existing primitives
`CoordinationScope` references `ContentVisibility` (reach + affinity), `ElohimLayer`, `ContentAttestationType`, and `GeographicContext` — no reinvention.

### REA-native payloads
4 of 11 verbs reuse existing REA/governance types directly (`Agreement`, `Commitment`, `DeliberationProposal`). Even lightweight invocations carry enough context for elohim agents to author REA stories retroactively, making care work visible at scale.

### Mutual aid is governance, not routing
`MutualAidContext` lives in qahal alongside `DeliberationProposal`. Emergency priority is a governance act — auditable, reviewable, with mandatory after-action review when the context closes.

### Signals: typed + synthesized
Sense/respond uses typed signals (aggregatable, machine-readable) that flow up mechanically. When thresholds are crossed or patterns detected, elohim agents author freeform syntheses as "ambassadors" of their observed scope. Both travel together through escalation.

## Type Definitions

### InvocationEnvelope

```typescript
interface InvocationEnvelope {
  id: string;
  correlationId?: string;              // links request -> response chains
  parentId?: string;                   // links escalation chains (child -> parent)
  verb: CoordinationVerb;
  scope: CoordinationScope;
  routing: RoutingContext;
  payload: CoordinationPayload;
  sender: AgentRef;
  timestamp: string;                   // ISO 8601
  constitutionalBasis?: string;        // EPR ref to constitutional authority
  ttl?: number;                        // seconds — wire-only verbs expire
}

type CoordinationVerb =
  | 'invoke' | 'sense' | 'respond' | 'aggregate' | 'route'
  | 'delegate' | 'escalate' | 'ratify' | 'recall'
  | 'provision' | 'federate';
```

### Core Types

```typescript
interface CoordinationScope {
  visibility: ContentVisibility;                  // reach + affinity + federated
  layer: ElohimLayer;                             // constitutional authority level
  attestationRequired?: ContentAttestationType[];  // access gate
  target: ScopeTarget;
}

type ScopeTarget =
  | { kind: 'agent'; agentId: string }
  | { kind: 'layer'; layer: ElohimLayer; geography?: GeographicContext }
  | { kind: 'affinity'; scope: AffinityScope; communityIds?: string[] }
  | { kind: 'broadcast'; reach: ReachLevel }
  | { kind: 'council'; councilId: string };

interface RoutingContext {
  capabilities?: ElohimCapability[];
  urgency: 'async' | 'near-realtime' | 'realtime';
  fallback: 'escalate' | 'queue' | 'reject';
  maxHops?: number;
}

interface AgentRef {
  agentId: string;
  layer?: ElohimLayer;
  onBehalfOf?: string;                // delegation chain
}
```

### Payload Types — New Interfaces

```typescript
// invoke + route
interface InvocationRequest {
  capability: ElohimCapability[];
  params: Record<string, unknown>;
  intent?: Intent;                    // pre-formed REA intent (if negotiating)
  inputRefs?: string[];               // EPR refs to input data
  expectedOutputType?: string;
}

// sense
interface SenseQuery {
  question: string;
  responseShape: ResponseShape;
  deadline?: string;
  quorum?: number;
  precedentRefs?: string[];           // EPR refs to prior sense rounds
}

type ResponseShape =
  | { kind: 'categorical'; options: string[] }
  | { kind: 'graduated'; selector: GraduatedFeedbackSelector }
  | { kind: 'numeric'; min: number; max: number; unit?: string }
  | { kind: 'ranked'; options: string[] }
  | { kind: 'freeform' };

// respond
interface SenseSignal {
  queryId: string;
  structured?: TypedSignal;
  synthesis?: AgentSynthesis;
}

interface TypedSignal {
  shape: ResponseShape;
  value: unknown;
  confidence?: number;
  weight?: number;
}

interface AgentSynthesis {
  agent: AgentRef;
  narrative: string;
  referencedSignals: string[];
  observedPatterns?: string[];
  urgency: 'informational' | 'concerning' | 'critical';
  recommendedAction?: CoordinationVerb;
}

// aggregate
interface AggregationResult {
  queryId: string;
  signalCount: number;
  quorumMet: boolean;
  structured?: AggregatedSignal;
  syntheses: AgentSynthesis[];
  methodology: AggregationMethodology;
}

interface AggregatedSignal {
  shape: ResponseShape;
  distribution: Record<string, number>;
  mean?: number;
  median?: number;
  consensus?: number;                  // 0-1, degree of agreement
}

type AggregationMethodology =
  | 'simple-majority'
  | 'weighted-by-intimacy'
  | 'weighted-by-stake'
  | 'bridging-priority'               // Polis-style cross-cutting agreement
  | 'unanimous'
  | 'custom';

// escalate (on-chain)
interface EscalationRecord {
  fromLayer: ElohimLayer;
  toLayer: ElohimLayer;
  reason: EscalationReason;
  synthesis: AgentSynthesis;
  priorEnvelopeIds: string[];
  aggregationSnapshot?: AggregationResult;
}

type EscalationReason =
  | 'quorum-not-met'
  | 'consensus-not-reached'
  | 'capacity-exhausted'
  | 'authority-exceeded'
  | 'pattern-detected'
  | 'emergency-declared';

// federate (on-chain)
interface FederationAgreement {
  agreement: Agreement;                // base REA agreement type
  instances: FederationParty[];
  protocolVersion: string;
  syncScope: ContentVisibility;
  disputeResolution: ElohimLayer;
}

interface FederationParty {
  instanceId: string;
  wellKnownUrl: string;
  authorizedAgent: AgentRef;
}
```

### Payload Types — Reused from Existing Models

| Verb | Payload | Existing Type | How It's Used |
|------|---------|---------------|---------------|
| `delegate` | `Agreement` | `rea-bridge.model.ts` | `classification: 'delegation'` with scoped authority |
| `ratify` | `DeliberationProposal` | `governance-deliberation.model.ts` | outcome of governance process |
| `recall` | `Commitment` | `rea-bridge.model.ts` | `state: 'cancelled'` with reason |
| `provision` | `Commitment` | `rea-bridge.model.ts` | `action: 'deliver-service'`, `resourceConformsTo: 'compute'` |

### MutualAidContext (Governance Primitive)

```typescript
// Lives in qahal alongside DeliberationProposal — NOT in the envelope

interface MutualAidContext {
  id: string;
  mode: 'bootstrap' | 'emergency' | 'scheduled';
  scope: CoordinationScope;
  declaredBy: AgentRef;
  constitutionalBasis: string;
  activatedAt: string;
  expiresAt?: string;

  resourceType: ResourceClassification;
  commitmentRefs: string[];

  // After-action review (populated when context closes)
  closedAt?: string;
  closedBy?: AgentRef;
  reviewStatus: 'active' | 'pending-review' | 'reviewed' | 'lessons-captured';
  afterActionRef?: string;             // EPR ref to the review deliberation
}
```

## Coordination Flows

### Council Sense-and-Respond

```
1. State council sends    sense { question, shape: categorical, target: layer/municipal }
2. Municipal elohim fan   sense to families in their scope
3. Families respond with  respond { structured: TypedSignal }
4. Municipal aggregates   aggregate { distribution, consensus score }
5. If consensus < 0.6     Municipal authors AgentSynthesis (ambassador narrative)
                          escalate { fromLayer: municipal, toLayer: state, reason: consensus-not-reached }
6. State receives both    the aggregate data AND the situated interpretation
```

### Ephemeral Device Inference

```
1. Phone sends            invoke { capability: [knowledge-map-synthesis], params: { contentId } }
                          scope: { target: { kind: 'agent', agentId: homeNodeId } }
                          routing: { urgency: 'near-realtime', fallback: 'escalate' }
2. Home node serves it    (or if busy, re-routes via route to community node)
3. Elohim observes        authors EconomicEvent retroactively (care-token, deliver-service)
```

### Mutual Aid Bootstrap

```
1. New region comes online
2. Sponsor elohim declares MutualAidContext { mode: 'bootstrap', scope: bioregion/west-africa }
3. Nearby nodes respond   provision (Commitment: deliver-service, compute, 30 days)
4. As local capacity grows, commitments taper
5. Context closes         reviewStatus -> 'pending-review'
6. After-action review    DeliberationProposal { type: 'advice' } — was sponsorship effective?
```

### Developer Simulation

```
1. Developer sends        invoke { capability: [compute], intent: Intent { action: 'deliver-service',
                            resourceConformsTo: 'compute', quantity: { value: 48, unit: 'cpu-hour' } } }
                          scope: { target: { kind: 'affinity', scope: 'interest_group' } }
2. Matching nodes respond with provision (Commitment)
3. Workload dispatched    (WASM task or container — execution layer concern, not this design)
4. Settlement             EconomicEvent: deliver-service, mutual-credit settlement
```

## Type Inventory

| Category | Count | Types |
|----------|-------|-------|
| Envelope | 1 | `InvocationEnvelope` |
| Core | 3 | `CoordinationScope`, `RoutingContext`, `AgentRef` |
| Enums | 2 | `CoordinationVerb`, `ScopeTarget` |
| New payloads | 6 | `InvocationRequest`, `SenseQuery`, `SenseSignal`, `AggregationResult`, `EscalationRecord`, `FederationAgreement` |
| Reused payloads | 3 | `Agreement`, `Commitment`, `DeliberationProposal` |
| Signal types | 4 | `TypedSignal`, `AgentSynthesis`, `AggregatedSignal`, `ResponseShape` |
| Governance | 1 | `MutualAidContext` |
| Supporting | 4 | `EscalationReason`, `AggregationMethodology`, `FederationParty`, `EnvelopeMetadata` |
| **Total new** | **~20** | Interfaces + type aliases |

## Implementation Path

Phase C (this plan): TypeScript interfaces in elohim-app, clean IoC boundaries.

Phase B (future): Rust implementations in elohim-storage (on-chain verbs as Holochain entries) and elohim-node (wire-only verbs as libp2p protocol messages). The TypeScript interfaces become the contract that Rust must satisfy through the existing type generation pipeline (Rust views -> `cargo test export_bindings` -> generated TypeScript).

## File Placement

New interfaces go in `elohim-app/src/app/elohim/models/` as the coordination layer is a cross-pillar concern owned by the elohim pillar. `MutualAidContext` goes in `elohim-app/src/app/qahal/models/` as a governance primitive.
