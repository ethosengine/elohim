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

import type { ElohimLayer, ElohimCapability } from './elohim-agent.model';
import type {
  ContentVisibility,
  ReachLevel,
  AffinityScope,
  GeographicContext,
} from './protocol-core.model';
import type { Intent, Agreement, Commitment } from './rea-bridge.model';
import type { ContentAttestationType } from '@app/lamad/models/content-attestation.model';
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
