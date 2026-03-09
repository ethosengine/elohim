# Coordination Verb Layer Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Define TypeScript interfaces for 11 coordination verbs that compose with existing REA, governance, and trust primitives to enable distributed negotiated execution.

**Architecture:** New `coordination.model.ts` in the elohim pillar (cross-pillar concern) + `mutual-aid.model.ts` in qahal pillar (governance primitive). Pure interfaces — no services, no runtime code beyond type guards. Imports from existing protocol-core, rea-bridge, elohim-agent, governance-deliberation, and content-attestation models.

**Tech Stack:** TypeScript 5.x, Angular 19 (for barrel exports), Vitest (for type compilation tests)

---

### Task 1: Create coordination-envelope.model.ts — Core Envelope + Verb Enum

**Files:**
- Create: `elohim-app/src/app/elohim/models/coordination-envelope.model.ts`

**Step 1: Create the coordination envelope model file**

```typescript
/**
 * Coordination Envelope - The coordination primitive for the Elohim Protocol.
 *
 * This module defines the InvocationEnvelope — a domain-level container that
 * composes existing REA, governance, and trust primitives into a unified
 * coordination layer. It enables three converging use cases:
 *
 * 1. Council coordination — hierarchical sense-and-respond across governance layers
 * 2. Compute routing — ephemeral devices delegating to capable nodes
 * 3. Mutual aid — bootstrap/disaster/seasonal solidarity between regions
 *
 * Design decisions:
 * - Envelope WRAPS existing types (not extends WireMessage)
 * - Scope COMPOSES existing ContentVisibility + ElohimLayer + attestation
 * - 4 of 11 verbs reuse existing REA/governance payloads directly
 * - On-chain verbs: delegate, ratify, recall, provision, federate, escalate
 * - Wire-only verbs: invoke, sense, respond, route, aggregate
 *
 * @see genesis/plans/2026-03-09-coordination-verb-layer-design.md
 */

import type {
  ContentVisibility,
  ReachLevel,
  AffinityScope,
  GovernanceLayer,
  GeographicContext,
} from './protocol-core.model';
import type { ElohimLayer, ElohimCapability } from './elohim-agent.model';
import type { ContentAttestationType } from '@app/lamad/models/content-attestation.model';
import type { Intent, Agreement, Commitment } from './rea-bridge.model';
import type {
  DeliberationProposal,
  GraduatedFeedbackSelector,
} from '@app/qahal/models/governance-deliberation.model';

// ============================================================================
// COORDINATION VERBS
// ============================================================================

/**
 * CoordinationVerb - The eleven coordination primitives.
 *
 * On-chain (Holochain entries, auditable):
 *   delegate, escalate, ratify, recall, provision, federate
 *
 * Wire-only (libp2p messages, ephemeral):
 *   invoke, sense, respond, route, aggregate
 */
export type CoordinationVerb =
  // Wire-only — transient coordination messages
  | 'invoke' // Request action from node/council
  | 'sense' // Gather signals across a scope
  | 'respond' // Return signal/result
  | 'aggregate' // Roll up sub-signals
  | 'route' // Direct request to capable node
  // On-chain — authority changes or economic obligations
  | 'delegate' // Transfer authority (scoped Agreement)
  | 'escalate' // Push up the hierarchy (auditable trail)
  | 'ratify' // Collective approval (DeliberationProposal outcome)
  | 'recall' // Revoke delegation (Commitment cancellation)
  | 'provision' // Reserve capacity (Commitment: deliver-service)
  | 'federate'; // Cross-instance agreement

/**
 * Verbs that produce Holochain entries (auditable, DHT-gossipped).
 */
export const ON_CHAIN_VERBS: readonly CoordinationVerb[] = [
  'delegate',
  'escalate',
  'ratify',
  'recall',
  'provision',
  'federate',
] as const;

/**
 * Verbs that exist only as wire protocol messages (ephemeral).
 */
export const WIRE_ONLY_VERBS: readonly CoordinationVerb[] = [
  'invoke',
  'sense',
  'respond',
  'aggregate',
  'route',
] as const;

/**
 * Check if a verb produces an on-chain entry.
 */
export function isOnChainVerb(verb: CoordinationVerb): boolean {
  return (ON_CHAIN_VERBS as readonly string[]).includes(verb);
}

// ============================================================================
// SCOPE - Who this envelope targets
// ============================================================================

/**
 * CoordinationScope - Composes existing visibility, layer, and attestation primitives.
 *
 * Does NOT reinvent reach/affinity/attestation — references them.
 */
export interface CoordinationScope {
  /** Geographic + affinity + federation visibility */
  visibility: ContentVisibility;

  /** Constitutional authority level for this coordination */
  layer: ElohimLayer;

  /** Access gate — what attestations are required to participate */
  attestationRequired?: ContentAttestationType[];

  /** Who receives this envelope */
  target: ScopeTarget;
}

/**
 * ScopeTarget - Discriminated union for envelope recipients.
 */
export type ScopeTarget =
  | { kind: 'agent'; agentId: string }
  | { kind: 'layer'; layer: ElohimLayer; geography?: GeographicContext }
  | { kind: 'affinity'; scope: AffinityScope; communityIds?: string[] }
  | { kind: 'broadcast'; reach: ReachLevel }
  | { kind: 'council'; councilId: string };

// ============================================================================
// ROUTING - How the envelope reaches its target
// ============================================================================

/**
 * RoutingContext - Capability matching and delivery preferences.
 *
 * Priority comes from ambient MutualAidContext (governance),
 * NOT from flags in the envelope itself.
 */
export interface RoutingContext {
  /** What the target must be able to do */
  capabilities?: ElohimCapability[];

  /** How quickly the response is needed */
  urgency: 'async' | 'near-realtime' | 'realtime';

  /** What happens if no capable target is found */
  fallback: 'escalate' | 'queue' | 'reject';

  /** How far to relay before giving up */
  maxHops?: number;
}

// ============================================================================
// AGENT REF - Thin identity reference
// ============================================================================

/**
 * AgentRef - Minimal agent identity for envelope sender/target.
 *
 * Not the full ProtocolAgent — just enough to identify and trace delegation.
 */
export interface AgentRef {
  /** Agent identifier */
  agentId: string;

  /** What governance layer this agent is acting at */
  layer?: ElohimLayer;

  /** Delegation chain — acting on behalf of another agent */
  onBehalfOf?: string;
}

// ============================================================================
// ENVELOPE - The coordination primitive
// ============================================================================

/**
 * InvocationEnvelope - The unified coordination primitive.
 *
 * Wraps existing REA, governance, and trust types into a single
 * container that can be serialized for wire (libp2p) or stored
 * on-chain (Holochain entry) depending on the verb.
 */
export interface InvocationEnvelope {
  /** Unique envelope identifier */
  id: string;

  /** Links request -> response chains */
  correlationId?: string;

  /** Links escalation chains (child -> parent for memory traversal) */
  parentId?: string;

  /** Which coordination verb */
  verb: CoordinationVerb;

  /** Who this targets and at what scope */
  scope: CoordinationScope;

  /** How to route and deliver */
  routing: RoutingContext;

  /** Verb-specific payload */
  payload: CoordinationPayload;

  /** Who authored this envelope */
  sender: AgentRef;

  /** When this envelope was created (ISO 8601) */
  timestamp: string;

  /** EPR ref to constitutional authority permitting this action */
  constitutionalBasis?: string;

  /** Seconds until expiry — wire-only verbs expire, on-chain verbs don't */
  ttl?: number;
}

// ============================================================================
// PAYLOAD - Discriminated union by verb
// ============================================================================

/**
 * CoordinationPayload - Discriminated union of verb-specific payloads.
 *
 * 6 new types + 4 reused from existing models:
 * - delegate  -> Agreement (classification: 'delegation')
 * - ratify    -> DeliberationProposal
 * - recall    -> Commitment (state: 'cancelled')
 * - provision -> Commitment (action: 'deliver-service')
 */
export type CoordinationPayload =
  | { verb: 'invoke'; data: InvocationRequest }
  | { verb: 'sense'; data: SenseQuery }
  | { verb: 'respond'; data: SenseSignal }
  | { verb: 'aggregate'; data: AggregationResult }
  | { verb: 'route'; data: InvocationRequest }
  | { verb: 'delegate'; data: Agreement }
  | { verb: 'escalate'; data: EscalationRecord }
  | { verb: 'ratify'; data: DeliberationProposal }
  | { verb: 'recall'; data: Commitment }
  | { verb: 'provision'; data: Commitment }
  | { verb: 'federate'; data: FederationAgreement };

// ============================================================================
// NEW PAYLOAD TYPES - Invoke + Route
// ============================================================================

/**
 * InvocationRequest - "Do this thing."
 *
 * Used by both 'invoke' (targeted) and 'route' (mesh-discovered).
 * Always carries enough context for an elohim to hydrate into
 * a full REA story retroactively — making care work visible.
 */
export interface InvocationRequest {
  /** What capabilities are needed */
  capability: ElohimCapability[];

  /** Inputs for the capability */
  params: Record<string, unknown>;

  /** Pre-formed REA intent (if negotiating compute/resources) */
  intent?: Intent;

  /** EPR refs to input data */
  inputRefs?: string[];

  /** Hint for expected result shape */
  expectedOutputType?: string;
}

// ============================================================================
// NEW PAYLOAD TYPES - Sense / Respond / Aggregate
// ============================================================================

/**
 * SenseQuery - "What do you think about X?"
 *
 * Initiates a sensemaking round. Typed signals aggregate mechanically.
 * When thresholds are crossed, elohim author freeform syntheses.
 */
export interface SenseQuery {
  /** Human-readable question */
  question: string;

  /** What kind of answer we want */
  responseShape: ResponseShape;

  /** When aggregation happens (ISO 8601) */
  deadline?: string;

  /** Minimum responses before aggregation is valid */
  quorum?: number;

  /** EPR refs to prior sense rounds on this topic (precedent) */
  precedentRefs?: string[];
}

/**
 * ResponseShape - How a SenseQuery response should be structured.
 *
 * 'graduated' reuses the existing GraduatedFeedbackSelector from qahal.
 */
export type ResponseShape =
  | { kind: 'categorical'; options: string[] }
  | { kind: 'graduated'; selector: GraduatedFeedbackSelector }
  | { kind: 'numeric'; min: number; max: number; unit?: string }
  | { kind: 'ranked'; options: string[] }
  | { kind: 'freeform' };

/**
 * SenseSignal - Response to a SenseQuery.
 *
 * Carries both structured (aggregatable) and synthesized (freeform) signals.
 * Structured signals flow up mechanically. Syntheses are the "ambassador" role —
 * an elohim's situated interpretation of what the signals mean.
 */
export interface SenseSignal {
  /** Which SenseQuery this responds to (envelope ID) */
  queryId: string;

  /** Machine-aggregatable response */
  structured?: TypedSignal;

  /** Freeform interpretation by an elohim agent */
  synthesis?: AgentSynthesis;
}

/**
 * TypedSignal - Machine-aggregatable signal value.
 */
export interface TypedSignal {
  /** Echoes the query's response shape */
  shape: ResponseShape;

  /** The actual value (typed by shape: string, number, string[]) */
  value: unknown;

  /** How sure is the respondent (0-1) */
  confidence?: number;

  /** Governance-determined voting weight */
  weight?: number;
}

/**
 * AgentSynthesis - An elohim's freeform interpretation of observed signals.
 *
 * This is the "ambassador" role: not just passing data up, but making
 * sense of it for the next governance level. Because escalate is on-chain,
 * this interpretation is auditable — "why did you characterize it that way?"
 */
export interface AgentSynthesis {
  /** Which elohim authored this synthesis */
  agent: AgentRef;

  /** Freeform narrative interpretation */
  narrative: string;

  /** Envelope IDs of signals this synthesis draws from */
  referencedSignals: string[];

  /** Named patterns observed (e.g., "generational divide", "capacity shortage") */
  observedPatterns?: string[];

  /** How urgent is the situation */
  urgency: 'informational' | 'concerning' | 'critical';

  /** What the synthesizing agent recommends as next step */
  recommendedAction?: CoordinationVerb;
}

/**
 * AggregationResult - Rolled-up signals from a sense round.
 */
export interface AggregationResult {
  /** Which SenseQuery this aggregates */
  queryId: string;

  /** How many responses were received */
  signalCount: number;

  /** Whether quorum was met */
  quorumMet: boolean;

  /** Rolled-up typed signals */
  structured?: AggregatedSignal;

  /** All agent interpretations at this level */
  syntheses: AgentSynthesis[];

  /** How the aggregation was computed (transparency) */
  methodology: AggregationMethodology;
}

/**
 * AggregatedSignal - Statistical summary of typed signals.
 */
export interface AggregatedSignal {
  /** The response shape that was aggregated */
  shape: ResponseShape;

  /** Distribution: value -> count or value -> total weight */
  distribution: Record<string, number>;

  /** Mean value (for numeric shapes) */
  mean?: number;

  /** Median value (for numeric shapes) */
  median?: number;

  /** Degree of agreement (0-1) */
  consensus?: number;
}

/**
 * AggregationMethodology - How signals were rolled up.
 *
 * Transparency: every aggregation records its methodology so
 * participants can understand how their signals were combined.
 */
export type AggregationMethodology =
  | 'simple-majority'
  | 'weighted-by-intimacy' // closer relationships weight more
  | 'weighted-by-stake' // more affected parties weight more
  | 'bridging-priority' // Polis-style: prioritize cross-cutting agreement
  | 'unanimous'
  | 'custom';

// ============================================================================
// NEW PAYLOAD TYPES - Escalate
// ============================================================================

/**
 * EscalationRecord - "We couldn't resolve this, here's why."
 *
 * On-chain: creates an auditable trail for "revisiting memories" —
 * future councils can search escalation history to see how similar
 * coordination problems were handled.
 */
export interface EscalationRecord {
  /** Which governance layer couldn't resolve it */
  fromLayer: ElohimLayer;

  /** Where it's being escalated to */
  toLayer: ElohimLayer;

  /** Why escalation is happening */
  reason: EscalationReason;

  /** The escalating agent's situated interpretation */
  synthesis: AgentSynthesis;

  /** Full trail of prior coordination attempts (envelope IDs) */
  priorEnvelopeIds: string[];

  /** State of signals at time of escalation */
  aggregationSnapshot?: AggregationResult;
}

/**
 * EscalationReason - Why coordination moved up the hierarchy.
 */
export type EscalationReason =
  | 'quorum-not-met' // not enough participation
  | 'consensus-not-reached' // participation but no agreement
  | 'capacity-exhausted' // can't serve the request at this level
  | 'authority-exceeded' // this level doesn't have jurisdiction
  | 'pattern-detected' // elohim noticed something cross-cutting
  | 'emergency-declared'; // needs immediate higher authority

// ============================================================================
// NEW PAYLOAD TYPES - Federate
// ============================================================================

/**
 * FederationAgreement - Cross-instance coordination terms.
 *
 * Extends Agreement (REA) with protocol-specific fields for
 * doorway-to-doorway federation.
 */
export interface FederationAgreement {
  /** Base REA agreement */
  agreement: Agreement;

  /** Which doorway instances are party to this */
  instances: FederationParty[];

  /** Which EPR version governs this federation */
  protocolVersion: string;

  /** What content flows between instances */
  syncScope: ContentVisibility;

  /** Which governance layer arbitrates disagreements */
  disputeResolution: ElohimLayer;
}

/**
 * FederationParty - A doorway instance in a federation agreement.
 */
export interface FederationParty {
  /** Doorway instance identifier */
  instanceId: string;

  /** Discovery endpoint */
  wellKnownUrl: string;

  /** Who signed for this instance */
  authorizedAgent: AgentRef;
}
```

**Step 2: Verify the file compiles**

Run: `cd /projects/elohim && pnpm exec tsc --noEmit --project elohim-app/tsconfig.json 2>&1 | head -30`
Expected: No errors from coordination-envelope.model.ts (existing errors from other files are OK)

**Step 3: Commit**

```bash
git add elohim-app/src/app/elohim/models/coordination-envelope.model.ts
git commit -m "feat(elohim): add coordination envelope types — 11 verbs, scope, routing

Defines InvocationEnvelope as the unified coordination primitive composing
existing REA, governance, and trust types. Six on-chain verbs (delegate,
escalate, ratify, recall, provision, federate) and five wire-only verbs
(invoke, sense, respond, route, aggregate).

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 2: Create mutual-aid.model.ts — Governance Primitive

**Files:**
- Create: `elohim-app/src/app/qahal/models/mutual-aid.model.ts`

**Step 1: Create the mutual aid context model**

```typescript
/**
 * Mutual Aid Context - Governance primitive for compute/resource solidarity.
 *
 * Lives in qahal alongside DeliberationProposal because emergency priority
 * is a governance act, not a routing flag. An elohim can declare emergency
 * priority, but that declaration is auditable and triggers mandatory
 * after-action review when the context closes.
 *
 * Three modes:
 * - bootstrap: New network coming online, needs sponsor region
 * - emergency: Existing network degraded (disaster, infrastructure failure)
 * - scheduled: Predictable demand spikes (exam periods, governance deliberations)
 *
 * Lifecycle:
 *   declared -> active -> closed -> pending-review -> reviewed -> lessons-captured
 *
 * The after-action review is itself a DeliberationProposal with type 'advice':
 * "Was this declaration justified? What do we learn?"
 *
 * @see genesis/plans/2026-03-09-coordination-verb-layer-design.md
 */

import type { ResourceClassification } from '@app/elohim/models/rea-bridge.model';
import type {
  CoordinationScope,
  AgentRef,
} from '@app/elohim/models/coordination-envelope.model';

// ============================================================================
// MUTUAL AID CONTEXT
// ============================================================================

/**
 * MutualAidMode - What triggered the mutual aid context.
 */
export type MutualAidMode = 'bootstrap' | 'emergency' | 'scheduled';

/**
 * MutualAidReviewStatus - Lifecycle of the after-action review.
 */
export type MutualAidReviewStatus =
  | 'active' // Context is live, resources flowing
  | 'pending-review' // Context closed, awaiting review
  | 'reviewed' // Review deliberation completed
  | 'lessons-captured'; // Lessons extracted and applied

/**
 * MutualAidContext - A declared period of resource solidarity.
 *
 * NOT part of the InvocationEnvelope — this is an ambient governance
 * context that affects how envelopes are prioritized and routed.
 *
 * Priority comes from governance decisions, not message flags.
 * This prevents abuse of emergency powers: every declaration is
 * auditable and reviewed.
 */
export interface MutualAidContext {
  /** Unique identifier */
  id: string;

  /** What triggered this context */
  mode: MutualAidMode;

  /** Which region/community is affected */
  scope: CoordinationScope;

  /** Which elohim declared this context */
  declaredBy: AgentRef;

  /** EPR ref to constitutional authority for this declaration */
  constitutionalBasis: string;

  /** When the context was activated (ISO 8601) */
  activatedAt: string;

  /** When this context auto-closes (ISO 8601) */
  expiresAt?: string;

  /** What resource type is being provided */
  resourceType: ResourceClassification;

  /** Commitment IDs from provisioning nodes */
  commitmentRefs: string[];

  /** When the context was closed (ISO 8601) */
  closedAt?: string;

  /** Who closed the context */
  closedBy?: AgentRef;

  /** Current review lifecycle state */
  reviewStatus: MutualAidReviewStatus;

  /** EPR ref to the after-action review deliberation */
  afterActionRef?: string;
}
```

**Step 2: Verify the file compiles**

Run: `cd /projects/elohim && pnpm exec tsc --noEmit --project elohim-app/tsconfig.json 2>&1 | grep mutual-aid`
Expected: No errors from mutual-aid.model.ts

**Step 3: Commit**

```bash
git add elohim-app/src/app/qahal/models/mutual-aid.model.ts
git commit -m "feat(qahal): add MutualAidContext governance primitive

Emergency priority is a governance act, not a routing flag. Declares
bootstrap/emergency/scheduled solidarity with mandatory after-action
review when the context closes.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 3: Wire barrel exports

**Files:**
- Modify: `elohim-app/src/app/elohim/models/index.ts`
- Modify: `elohim-app/src/app/qahal/models/index.ts`

**Step 1: Add coordination envelope export to elohim barrel**

Add after the `create-context.model` export (line 40 area):

```typescript
// Coordination Envelope - Distributed coordination verbs
export * from './coordination-envelope.model';
```

**Step 2: Add mutual aid export to qahal barrel**

Add after the `collective.model` export (line 13 area):

```typescript
// Mutual Aid - Compute/resource solidarity governance
export * from './mutual-aid.model';
```

**Step 3: Verify barrel exports compile**

Run: `cd /projects/elohim && pnpm exec tsc --noEmit --project elohim-app/tsconfig.json 2>&1 | grep -E "(coordination|mutual-aid)" | head -10`
Expected: No errors

**Step 4: Verify imports work from pillar aliases**

Create a quick check — confirm that `@app/elohim` and `@app/qahal` resolve the new types:

Run: `cd /projects/elohim && pnpm exec tsc --noEmit --project elohim-app/tsconfig.json 2>&1 | tail -5`
Expected: Existing error count unchanged (no new errors introduced)

**Step 5: Commit**

```bash
git add elohim-app/src/app/elohim/models/index.ts elohim-app/src/app/qahal/models/index.ts
git commit -m "chore(elohim): wire coordination and mutual-aid barrel exports

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 4: Type compilation smoke test

**Files:**
- Create: `elohim-app/src/app/elohim/models/coordination-envelope.model.typetest.ts`

This file exists solely to verify the types compose correctly at compile time. It imports the new types and constructs example values that exercise the discriminated unions, reused REA types, and scope composition.

**Step 1: Write the type compilation test**

```typescript
/**
 * Type-level smoke test for coordination envelope types.
 *
 * This file is NOT a runtime test — it verifies that the types
 * compose correctly at compile time. If this file compiles,
 * the type relationships are sound.
 *
 * Excluded from test runner via naming convention (.typetest.ts).
 */

import type {
  InvocationEnvelope,
  CoordinationPayload,
  CoordinationScope,
  CoordinationVerb,
  ScopeTarget,
  RoutingContext,
  AgentRef,
  InvocationRequest,
  SenseQuery,
  SenseSignal,
  AggregationResult,
  EscalationRecord,
  FederationAgreement,
  ResponseShape,
  TypedSignal,
  AgentSynthesis,
  AggregatedSignal,
  AggregationMethodology,
  EscalationReason,
  FederationParty,
  isOnChainVerb,
} from './coordination-envelope.model';

import type { MutualAidContext } from '@app/qahal/models/mutual-aid.model';

import type { Agreement, Commitment, Intent } from './rea-bridge.model';
import type { DeliberationProposal } from '@app/qahal/models/governance-deliberation.model';

// -- Verify discriminated union exhaustiveness --

function assertPayloadVerb(p: CoordinationPayload): CoordinationVerb {
  return p.verb; // This proves verb field exists on all variants
}

// -- Verify ScopeTarget discriminated union --

function handleTarget(t: ScopeTarget): string {
  switch (t.kind) {
    case 'agent':
      return t.agentId;
    case 'layer':
      return t.layer;
    case 'affinity':
      return t.scope;
    case 'broadcast':
      return t.reach;
    case 'council':
      return t.councilId;
  }
}

// -- Verify ResponseShape discriminated union --

function handleShape(s: ResponseShape): string {
  switch (s.kind) {
    case 'categorical':
      return s.options[0];
    case 'graduated':
      return s.selector.targetId;
    case 'numeric':
      return `${s.min}-${s.max}`;
    case 'ranked':
      return s.options[0];
    case 'freeform':
      return 'freeform';
  }
}

// -- Verify reused REA types compose in payloads --

const _delegatePayload: CoordinationPayload = {
  verb: 'delegate' as const,
  data: {} as Agreement,
};

const _provisionPayload: CoordinationPayload = {
  verb: 'provision' as const,
  data: {} as Commitment,
};

const _ratifyPayload: CoordinationPayload = {
  verb: 'ratify' as const,
  data: {} as DeliberationProposal,
};

const _recallPayload: CoordinationPayload = {
  verb: 'recall' as const,
  data: {} as Commitment,
};

// -- Verify InvocationRequest can carry optional Intent --

const _invokeWithIntent: InvocationRequest = {
  capability: ['knowledge-map-synthesis'],
  params: { contentId: 'epr:123' },
  intent: {} as Intent,
};

const _invokeWithoutIntent: InvocationRequest = {
  capability: ['content-safety-review'],
  params: { text: 'hello' },
};

// -- Verify MutualAidContext uses CoordinationScope --

const _aidContext: MutualAidContext = {
  id: 'aid-1',
  mode: 'emergency',
  scope: {} as CoordinationScope,
  declaredBy: { agentId: 'elohim-cascadia' },
  constitutionalBasis: 'epr:constitution/4.2',
  activatedAt: '2026-03-09T00:00:00Z',
  resourceType: 'compute',
  commitmentRefs: [],
  reviewStatus: 'active',
};

// -- Verify full envelope construction --

const _envelope: InvocationEnvelope = {
  id: 'env-1',
  verb: 'sense',
  scope: {
    visibility: { reach: 'municipal', affinity: 'open', federated: false },
    layer: 'municipal',
    target: { kind: 'layer', layer: 'family' },
  },
  routing: { urgency: 'async', fallback: 'escalate' },
  payload: {
    verb: 'sense',
    data: {
      question: 'Should we expand the compute pool?',
      responseShape: { kind: 'categorical', options: ['yes', 'no', 'need-more-info'] },
      quorum: 50,
    },
  },
  sender: { agentId: 'elohim-portland', layer: 'municipal' },
  timestamp: '2026-03-09T00:00:00Z',
  constitutionalBasis: 'epr:constitution/sense-authority',
};

// Suppress unused variable warnings — this file is compile-only
void assertPayloadVerb;
void handleTarget;
void handleShape;
void _delegatePayload;
void _provisionPayload;
void _ratifyPayload;
void _recallPayload;
void _invokeWithIntent;
void _invokeWithoutIntent;
void _aidContext;
void _envelope;
```

**Step 2: Verify the typetest compiles**

Run: `cd /projects/elohim && pnpm exec tsc --noEmit --project elohim-app/tsconfig.json 2>&1 | grep typetest`
Expected: No errors from the typetest file

**Step 3: Commit**

```bash
git add elohim-app/src/app/elohim/models/coordination-envelope.model.typetest.ts
git commit -m "test(elohim): add type compilation smoke test for coordination envelope

Verifies discriminated unions, REA type reuse, scope composition,
and MutualAidContext integration at compile time.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 5: Verify full build and lint

**Step 1: Run TypeScript compilation**

Run: `cd /projects/elohim/elohim-app && pnpm exec tsc --noEmit 2>&1 | tail -20`
Expected: No new errors introduced by coordination types

**Step 2: Run ESLint on new files**

Run: `cd /projects/elohim/elohim-app && pnpm exec eslint src/app/elohim/models/coordination-envelope.model.ts src/app/qahal/models/mutual-aid.model.ts --ext .ts 2>&1`
Expected: No lint errors (or only pre-existing ones)

**Step 3: Run Prettier check**

Run: `cd /projects/elohim/elohim-app && pnpm exec prettier --check src/app/elohim/models/coordination-envelope.model.ts src/app/qahal/models/mutual-aid.model.ts`
Expected: Files formatted correctly

**Step 4: Fix any lint/format issues if needed**

Run: `cd /projects/elohim/elohim-app && pnpm exec prettier --write src/app/elohim/models/coordination-envelope.model.ts src/app/qahal/models/mutual-aid.model.ts && pnpm exec eslint src/app/elohim/models/coordination-envelope.model.ts src/app/qahal/models/mutual-aid.model.ts --fix`

**Step 5: Commit any fixes**

```bash
git add -u
git commit -m "style(elohim): fix lint/format for coordination types

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Summary

| Task | What | Files | Est. |
|------|------|-------|------|
| 1 | Coordination envelope + all verb payloads | `coordination-envelope.model.ts` | 5 min |
| 2 | MutualAidContext governance primitive | `mutual-aid.model.ts` | 3 min |
| 3 | Wire barrel exports | `index.ts` × 2 | 2 min |
| 4 | Type compilation smoke test | `.typetest.ts` | 3 min |
| 5 | Full build + lint verification | — | 2 min |

**Total: ~15 minutes, 5 commits**

## What This Enables (Future Work)

With these interfaces in place, the following can be implemented independently:

1. **Rust types in elohim-node** — `InvocationEnvelope` as a new libp2p request-response codec
2. **Rust types in elohim-storage** — on-chain verbs as Holochain integrity entries with validation
3. **TypeScript services** — `CoordinationService` in elohim pillar dispatching envelopes
4. **Elohim agent integration** — agents authoring `AgentSynthesis` and `EscalationRecord`
5. **Council UI** — sense/respond/aggregate visualizations in qahal components
6. **Compute routing** — `RoutingContext` + `InvocationRequest` powering ephemeral device inference
7. **Mutual aid flows** — `MutualAidContext` lifecycle management with after-action review
