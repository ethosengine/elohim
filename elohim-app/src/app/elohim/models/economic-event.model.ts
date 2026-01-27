/**
 * Economic Event Model - The Immutable Record of What Happened
 *
 * From the Economic Epic:
 * "REA doesn't ask 'how much money?' It asks 'what actually happened?'
 * Every event is recorded from what we call the 'independent view'—as
 * a transaction between parties rather than separate ledger entries."
 *
 * Core Concept:
 * Economic events are the ONLY way resources change in ValueFlows.
 * They form an immutable audit trail that can be viewed from any
 * agent's perspective. This is the foundation of transparent,
 * constitutional economics.
 *
 * In Lamad Context:
 * - Human views content → Event: use (attention flows to contributor)
 * - Human marks affinity → Event: appreciate (recognition flows)
 * - Human completes path → Event: produce (capability earned)
 * - Contributor claims presence → Event: accept (recognition transfers)
 * - Elohim synthesizes map → Event: deliver-service (synthesis created)
 *
 * Holochain mapping:
 * - Entry type: "economic_event"
 * - Lives on agent's source chain (provider or receiver)
 * - Countersigned by both parties for transfers
 * - Immutable - corrections create new events, don't modify old ones
 *
 * Unyt Integration:
 * - Mutual credit events flow through this same model
 * - HoloFuel transactions are EconomicEvents with currency resources
 * - The "De Beers insight": we control flow, not supply
 */

import { REAAction, Measure, ResourceClassification } from '@app/elohim/models/rea-bridge.model';

// ============================================================================
// Economic Event
// ============================================================================

/**
 * EconomicEvent - An observed economic occurrence.
 *
 * This is the core building block of REA accounting.
 * Events are immutable records of value flows between agents.
 */
export interface EconomicEvent {
  /** Unique identifier (ActionHash in Holochain) */
  id: string;

  /** What happened */
  action: REAAction;

  /** Who provided/gave */
  provider: string; // Agent ID

  /** Who received */
  receiver: string; // Agent ID

  // ─────────────────────────────────────────────────────────────────
  // Resource Identification
  // ─────────────────────────────────────────────────────────────────

  /** What type of resource (if conforming to spec) */
  resourceConformsTo?: string; // ResourceSpecification.id

  /** Specific resource affected (if inventoried) - provider side */
  resourceInventoriedAs?: string; // EconomicResource.id

  /**
   * Resource on the receiving side of a transfer or move.
   * For transfers between agents, this is the destination resource.
   * Required for transfer and move actions.
   */
  toResourceInventoriedAs?: string; // EconomicResource.id

  /** For new resources: classification */
  resourceClassifiedAs?: ResourceClassification[];

  // ─────────────────────────────────────────────────────────────────
  // Quantities
  // ─────────────────────────────────────────────────────────────────

  /** Resource quantity affected */
  resourceQuantity?: Measure;

  /** Effort quantity (for work/service actions) */
  effortQuantity?: Measure;

  // ─────────────────────────────────────────────────────────────────
  // Timing
  // ─────────────────────────────────────────────────────────────────

  /** When this event occurred */
  hasPointInTime: string;

  /** Duration (if applicable) */
  hasDuration?: string;

  // ─────────────────────────────────────────────────────────────────
  // Process Context
  // ─────────────────────────────────────────────────────────────────

  /** If this is an input to a process */
  inputOf?: string; // Process.id

  /** If this is an output of a process */
  outputOf?: string; // Process.id

  // ─────────────────────────────────────────────────────────────────
  // Commitment/Agreement Context
  // ─────────────────────────────────────────────────────────────────

  /** Commitment(s) this event fulfills */
  fulfills?: string[]; // Commitment.id[]

  /** Agreement this operates under */
  realizationOf?: string; // Agreement.id

  /** Reference to an agreement governing this event (URI) */
  agreedIn?: string;

  /** Intent(s) this event satisfies */
  satisfies?: string[]; // Intent.id[]

  /**
   * Accounting scope(s) this event falls within.
   * Used for grouping, reporting, and access control.
   */
  inScopeOf?: string[];

  // ─────────────────────────────────────────────────────────────────
  // Metadata
  // ─────────────────────────────────────────────────────────────────

  /** Human-readable note */
  note?: string;

  /** State of the event */
  state: EventState;

  /** Triggered by another event */
  triggeredBy?: string; // EconomicEvent.id

  /** Location where event occurred */
  atLocation?: string;

  /** Image/evidence */
  image?: string;

  /** Cryptographic signatures (in production) */
  signatures?: EventSignature[];

  /** Arbitrary metadata */
  metadata?: Record<string, unknown>;
}

/**
 * EventState - Processing state of an event.
 */
export type EventState =
  | 'pending' // Created but not yet validated
  | 'validated' // Passed validation rules
  | 'countersigned' // Both parties have signed (for transfers)
  | 'disputed' // Under dispute
  | 'corrected'; // Superseded by correction event

/**
 * EventSignature - Cryptographic signature on an event.
 */
export interface EventSignature {
  /** Who signed */
  signerId: string;

  /** Signature role */
  role: 'provider' | 'receiver' | 'witness' | 'validator';

  /** The signature (placeholder for production) */
  signature: string;

  /** When signed */
  signedAt: string;
}

// ============================================================================
// Lamad-Specific Event Types
// ============================================================================

/**
 * LamadEventType - Classification of events in Lamad context.
 *
 * These map to specific action + resource combinations.
 */
export type LamadEventType =
  // Attention Events
  | 'content-view' // Human viewed content (use + attention)
  | 'content-complete' // Human completed content (produce + achievement)
  | 'path-step-complete' // Human completed a step (use + attention)
  | 'session-start' // Human began a session (use + attention)
  | 'session-end' // Human ended a session (use + attention)

  // Assessment Events
  | 'assessment-start' // Human started assessment (use + attention)
  | 'assessment-complete' // Human completed assessment (produce + achievement)
  | 'practice-attempt' // Human attempted practice (use + attention)
  | 'quiz-submit' // Human submitted quiz (produce + achievement)

  // Recognition Events
  | 'affinity-mark' // Human marked affinity (appreciate + recognition)
  | 'endorsement' // Formal endorsement (appreciate + endorsement)
  | 'citation' // Content cited another (cite + recognition)
  | 'recognition-given' // Recognition given to contributor (appreciate + recognition)
  | 'recognition-received' // Recognition received (appreciate + recognition)

  // Achievement Events
  | 'path-complete' // Human completed a path (produce + credential)
  | 'attestation-grant' // Attestation granted (produce + attestation)
  | 'capability-earn' // Capability developed (produce + credential)

  // Creation Events
  | 'content-create' // Content created (produce + content)
  | 'path-create' // Path created (produce + curation)
  | 'extension-create' // Path extension created (produce + curation)

  // Synthesis Events
  | 'map-synthesis' // Elohim synthesized map (deliver-service + synthesis)
  | 'analysis-complete' // Elohim completed analysis (deliver-service)

  // Stewardship Events
  | 'stewardship-begin' // Elohim began stewardship (work + stewardship)
  | 'invitation-send' // Invitation sent (deliver-service)
  | 'presence-claim' // Contributor claimed presence (accept + recognition)
  | 'recognition-transfer' // Recognition transferred (transfer + recognition)

  // Governance Events
  | 'attestation-revoke' // Attestation revoked (modify + attestation)
  | 'content-flag' // Content flagged (modify + content)
  | 'governance-vote' // Governance vote cast (work + governance)

  // Currency Events (Unyt integration)
  | 'credit-issue' // Mutual credit issued (produce + currency)
  | 'credit-transfer' // Credit transferred (transfer + currency)
  | 'credit-retire' // Credit retired (consume + currency)

  // Insurance Mutual Events (Elohim Mutual)
  | 'premium-payment' // Member paid premium (transfer + currency)
  | 'claim-filed' // Member filed claim (deliver-service + adjustment)
  | 'claim-evidence-submitted' // Supporting docs attached (deliver-service + adjustment)
  | 'claim-investigated' // Adjuster gathering evidence (work + adjustment)
  | 'claim-adjusted' // Adjuster made determination (deliver-service + adjustment)
  | 'claim-settled' // Claim paid (transfer + currency)
  | 'claim-denied' // Claim rejected (modify + adjustment)
  | 'claim-appealed' // Member appealed decision (work + adjustment)
  | 'risk-reduction-verified' // Observer verified risk mitigation (raise + recognition)
  | 'preventive-care-completed' // Member completed prevention activity (produce + stewardship)
  | 'prevention-incentive-awarded' // Premium discount/reward for risk mitigation (raise + care-token)
  | 'coverage-decision' // Community decided coverage (work + membership)
  | 'claim-review-initiated' // Governance review of adjuster decision (work + membership)
  | 'reserve-adjustment' // Regulatory reserve change (modify + currency)

  // Consent & Data Economy Events (ImagoDei/Qahal Research)
  | 'consent-grant' // Human granted consent for data use (produce + consent)
  | 'consent-withdraw' // Human withdrew consent (modify + consent)
  | 'data-access' // Researcher accessed data under consent (use + data)
  | 'data-value-transfer' // Value transferred for data access (transfer + token)
  | 'consent-audit-complete' // Periodic consent review completed (work + stewardship)
  | 'research-response-submit'; // Research assessment response submitted (produce + data)

// =============================================================================
// Standard Constants - Extracted to avoid duplicate string literals
// =============================================================================

/** Standard unit IDs for economic events */
const UNITS = {
  EACH: 'unit-each',
  VIEW: 'unit-view',
  STEP: 'unit-step',
  SESSION: 'unit-session',
  MINUTE: 'unit-minute',
  AFFINITY: 'unit-affinity',
  ENDORSEMENT: 'unit-endorsement',
  ATTESTATION: 'unit-attestation',
  NODE: 'unit-node',
  PATH: 'unit-path',
  TOKEN: 'unit-token',
} as const;

/** Standard REA action IDs - typed as REAAction to ensure compatibility */
const ACTIONS = {
  DELIVER_SERVICE: 'deliver-service' as const,
} as const;

/**
 * Event type to action+resource mapping.
 */
export const LAMAD_EVENT_MAPPINGS: Record<
  LamadEventType,
  {
    action: REAAction;
    resourceType: ResourceClassification;
    defaultUnit: string;
  }
> = {
  // Attention
  'content-view': { action: 'use', resourceType: 'attention', defaultUnit: UNITS.VIEW },
  'content-complete': { action: 'produce', resourceType: 'credential', defaultUnit: UNITS.EACH },
  'path-step-complete': { action: 'use', resourceType: 'attention', defaultUnit: UNITS.STEP },
  'session-start': { action: 'use', resourceType: 'attention', defaultUnit: UNITS.SESSION },
  'session-end': { action: 'use', resourceType: 'attention', defaultUnit: UNITS.MINUTE },

  // Assessment
  'assessment-start': { action: 'use', resourceType: 'attention', defaultUnit: UNITS.EACH },
  'assessment-complete': {
    action: 'produce',
    resourceType: 'credential',
    defaultUnit: UNITS.EACH,
  },
  'practice-attempt': { action: 'use', resourceType: 'attention', defaultUnit: UNITS.EACH },
  'quiz-submit': { action: 'produce', resourceType: 'credential', defaultUnit: UNITS.EACH },

  // Recognition
  'affinity-mark': { action: 'raise', resourceType: 'recognition', defaultUnit: UNITS.AFFINITY },
  endorsement: { action: 'raise', resourceType: 'recognition', defaultUnit: UNITS.ENDORSEMENT },
  citation: { action: 'cite', resourceType: 'recognition', defaultUnit: UNITS.EACH },
  'recognition-given': { action: 'raise', resourceType: 'recognition', defaultUnit: UNITS.EACH },
  'recognition-received': {
    action: 'raise',
    resourceType: 'recognition',
    defaultUnit: UNITS.EACH,
  },

  // Achievement
  'path-complete': { action: 'produce', resourceType: 'credential', defaultUnit: UNITS.EACH },
  'attestation-grant': {
    action: 'produce',
    resourceType: 'credential',
    defaultUnit: UNITS.ATTESTATION,
  },
  'capability-earn': { action: 'produce', resourceType: 'credential', defaultUnit: UNITS.EACH },

  // Creation
  'content-create': { action: 'produce', resourceType: 'content', defaultUnit: UNITS.NODE },
  'path-create': { action: 'produce', resourceType: 'curation', defaultUnit: UNITS.PATH },
  'extension-create': { action: 'produce', resourceType: 'curation', defaultUnit: UNITS.EACH },

  // Synthesis
  'map-synthesis': {
    action: ACTIONS.DELIVER_SERVICE,
    resourceType: 'synthesis',
    defaultUnit: UNITS.EACH,
  },
  'analysis-complete': {
    action: ACTIONS.DELIVER_SERVICE,
    resourceType: 'synthesis',
    defaultUnit: UNITS.EACH,
  },

  // Stewardship
  'stewardship-begin': { action: 'work', resourceType: 'stewardship', defaultUnit: UNITS.EACH },
  'invitation-send': {
    action: ACTIONS.DELIVER_SERVICE,
    resourceType: 'stewardship',
    defaultUnit: UNITS.EACH,
  },
  'presence-claim': { action: 'accept', resourceType: 'recognition', defaultUnit: UNITS.AFFINITY },
  'recognition-transfer': {
    action: 'transfer',
    resourceType: 'recognition',
    defaultUnit: UNITS.AFFINITY,
  },

  // Governance
  'attestation-revoke': {
    action: 'modify',
    resourceType: 'credential',
    defaultUnit: UNITS.ATTESTATION,
  },
  'content-flag': { action: 'modify', resourceType: 'content', defaultUnit: UNITS.NODE },
  'governance-vote': { action: 'work', resourceType: 'membership', defaultUnit: UNITS.EACH },

  // Currency
  'credit-issue': { action: 'produce', resourceType: 'currency', defaultUnit: UNITS.TOKEN },
  'credit-transfer': { action: 'transfer', resourceType: 'currency', defaultUnit: UNITS.TOKEN },
  'credit-retire': { action: 'consume', resourceType: 'currency', defaultUnit: UNITS.TOKEN },

  // Insurance Mutual Events
  'premium-payment': { action: 'transfer', resourceType: 'currency', defaultUnit: UNITS.TOKEN },
  'claim-filed': {
    action: ACTIONS.DELIVER_SERVICE,
    resourceType: 'stewardship',
    defaultUnit: UNITS.EACH,
  },
  'claim-evidence-submitted': {
    action: ACTIONS.DELIVER_SERVICE,
    resourceType: 'stewardship',
    defaultUnit: UNITS.EACH,
  },
  'claim-investigated': { action: 'work', resourceType: 'stewardship', defaultUnit: UNITS.EACH },
  'claim-adjusted': {
    action: ACTIONS.DELIVER_SERVICE,
    resourceType: 'stewardship',
    defaultUnit: UNITS.EACH,
  },
  'claim-settled': { action: 'transfer', resourceType: 'currency', defaultUnit: UNITS.TOKEN },
  'claim-denied': { action: 'modify', resourceType: 'stewardship', defaultUnit: UNITS.EACH },
  'claim-appealed': { action: 'work', resourceType: 'stewardship', defaultUnit: UNITS.EACH },
  'risk-reduction-verified': {
    action: 'raise',
    resourceType: 'recognition',
    defaultUnit: UNITS.AFFINITY,
  },
  'preventive-care-completed': {
    action: 'produce',
    resourceType: 'stewardship',
    defaultUnit: UNITS.EACH,
  },
  'prevention-incentive-awarded': {
    action: 'raise',
    resourceType: 'care-token',
    defaultUnit: UNITS.TOKEN,
  },
  'coverage-decision': { action: 'work', resourceType: 'membership', defaultUnit: UNITS.EACH },
  'claim-review-initiated': {
    action: 'work',
    resourceType: 'membership',
    defaultUnit: UNITS.EACH,
  },
  'reserve-adjustment': { action: 'modify', resourceType: 'currency', defaultUnit: UNITS.TOKEN },

  // Consent & Data Economy Events
  'consent-grant': { action: 'produce', resourceType: 'consent', defaultUnit: UNITS.EACH },
  'consent-withdraw': { action: 'modify', resourceType: 'consent', defaultUnit: UNITS.EACH },
  'data-access': { action: 'use', resourceType: 'data', defaultUnit: UNITS.EACH },
  'data-value-transfer': {
    action: 'transfer',
    resourceType: 'data-token',
    defaultUnit: UNITS.TOKEN,
  },
  'consent-audit-complete': {
    action: 'work',
    resourceType: 'stewardship',
    defaultUnit: UNITS.EACH,
  },
  'research-response-submit': { action: 'produce', resourceType: 'data', defaultUnit: UNITS.EACH },
};

// ============================================================================
// Event Creation Helpers
// ============================================================================

/**
 * CreateEventRequest - Request to create a new economic event.
 */
export interface CreateEventRequest {
  /** Lamad event type */
  eventType: LamadEventType;

  /** Provider agent ID */
  providerId: string;

  /** Receiver agent ID */
  receiverId: string;

  /** Resource quantity (uses default unit if not specified) */
  quantity?: number;

  /** Custom unit (overrides default) */
  unit?: string;

  /** Related content (for content events) */
  contentId?: string;

  /** Related path (for path events) */
  pathId?: string;

  /** Related step index (for step events) */
  stepIndex?: number;

  /** Related presence (for stewardship events) */
  presenceId?: string;

  /** Process this is part of */
  processId?: string;

  /** Commitment this fulfills */
  commitmentId?: string;

  /** Note */
  note?: string;

  /** Additional metadata */
  metadata?: Record<string, unknown>;
}

/**
 * Helper to create a typed economic event from a request.
 */
export function createEventFromRequest(
  request: CreateEventRequest,
  id: string,
  timestamp: string
): EconomicEvent {
  const mapping = LAMAD_EVENT_MAPPINGS[request.eventType];

  return {
    id,
    action: mapping.action,
    provider: request.providerId,
    receiver: request.receiverId,
    resourceConformsTo: `spec-${mapping.resourceType}`,
    resourceClassifiedAs: [mapping.resourceType],
    resourceQuantity: {
      hasNumericalValue: request.quantity ?? 1,
      hasUnit: request.unit ?? mapping.defaultUnit,
    },
    hasPointInTime: timestamp,
    inputOf: request.processId,
    fulfills: request.commitmentId ? [request.commitmentId] : undefined,
    note: request.note,
    state: 'pending',
    metadata: {
      lamadEventType: request.eventType,
      contentId: request.contentId,
      pathId: request.pathId,
      stepIndex: request.stepIndex,
      presenceId: request.presenceId,
      ...request.metadata,
    },
  };
}

// ============================================================================
// Event Queries
// ============================================================================

/**
 * EventQuery - Query parameters for finding events.
 */
export interface EventQuery {
  /** Filter by agent (as provider, receiver, or either) */
  agentId?: string;
  agentRole?: 'provider' | 'receiver' | 'either';

  /** Filter by action type */
  actions?: REAAction[];

  /** Filter by Lamad event type */
  eventTypes?: LamadEventType[];

  /** Filter by resource type */
  resourceTypes?: ResourceClassification[];

  /** Filter by process */
  processId?: string;

  /** Filter by time range */
  from?: string;
  to?: string;

  /** Filter by related content */
  contentId?: string;

  /** Filter by related path */
  pathId?: string;

  /** Filter by related presence */
  presenceId?: string;

  /** Filter by state */
  states?: EventState[];

  /** Sort order */
  sortBy?: 'time' | 'quantity';
  sortOrder?: 'asc' | 'desc';

  /** Pagination */
  limit?: number;
  offset?: number;
}

/**
 * EventQueryResult - Result of an event query.
 */
export interface EventQueryResult {
  /** Matching events */
  events: EconomicEvent[];

  /** Total count (for pagination) */
  totalCount: number;

  /** Aggregations (if requested) */
  aggregations?: EventAggregations;
}

/**
 * EventAggregations - Aggregated statistics from events.
 */
export interface EventAggregations {
  /** Total quantity by resource type */
  byResourceType: Record<string, number>;

  /** Event count by action */
  byAction: Record<string, number>;

  /** Event count by Lamad type */
  byEventType: Record<string, number>;

  /** Total quantity by time period */
  byTimePeriod?: Record<string, number>;

  /** Unique agents involved */
  uniqueAgents: number;
}

// ============================================================================
// Event Streams (for real-time updates)
// ============================================================================

/**
 * EventStreamSubscription - Subscribe to event stream.
 */
export interface EventStreamSubscription {
  /** Subscription ID */
  id: string;

  /** Filter criteria */
  filter: EventQuery;

  /** Callback endpoint/channel */
  callback: string;

  /** When subscription was created */
  createdAt: string;

  /** When subscription expires */
  expiresAt?: string;
}

/**
 * EventStreamMessage - Message from event stream.
 */
export interface EventStreamMessage {
  /** Message type */
  type: 'event' | 'heartbeat' | 'error';

  /** The event (if type is 'event') */
  event?: EconomicEvent;

  /** Error message (if type is 'error') */
  error?: string;

  /** Timestamp */
  timestamp: string;

  /** Sequence number */
  sequence: number;
}

// ============================================================================
// Event Correction
// ============================================================================

/**
 * EventCorrection - A correction to a previous event.
 *
 * Events are immutable, so corrections create new events
 * that reference and supersede the original.
 */
export interface EventCorrection {
  /** The correction event ID */
  correctionEventId: string;

  /** The original event being corrected */
  correctsEventId: string;

  /** Reason for correction */
  reason: CorrectionReason;

  /** Detailed explanation */
  explanation: string;

  /** Who authorized the correction */
  authorizedBy: string;

  /** Constitutional basis for correction */
  constitutionalBasis?: string;

  /** When corrected */
  correctedAt: string;
}

/**
 * CorrectionReason - Why an event was corrected.
 */
export type CorrectionReason =
  | 'data-error' // Wrong data entered
  | 'duplicate' // Duplicate event
  | 'fraud' // Fraudulent event
  | 'governance-order' // Governance decision
  | 'agent-request' // Agent requested correction
  | 'system-error'; // System malfunction

// ============================================================================
// Agent Perspective (REA's "Independent View")
// ============================================================================

/**
 * AgentEventLedger - Events from a single agent's perspective.
 *
 * This is how an agent sees their economic activity.
 * Same events, but organized for their view.
 */
export interface AgentEventLedger {
  /** The agent this ledger is for */
  agentId: string;

  /** Events where agent was provider (outflows) */
  outflows: EconomicEvent[];

  /** Events where agent was receiver (inflows) */
  inflows: EconomicEvent[];

  /** Net position by resource type */
  netPosition: Record<string, Measure>;

  /** Current resource holdings */
  currentHoldings: AgentResourceHolding[];

  /** Period covered */
  periodStart: string;
  periodEnd: string;
}

/**
 * AgentResourceHolding - An agent's current holding of a resource.
 */
export interface AgentResourceHolding {
  /** Resource specification */
  resourceSpecId: string;

  /** Resource name */
  resourceName: string;

  /** Current balance */
  balance: Measure;

  /** Committed (promised but not transferred) */
  committed: Measure;

  /** Available (balance - committed) */
  available: Measure;

  /** Last activity */
  lastActivityAt: string;
}

// ============================================================================
// Network-Wide Metrics (for transparency)
// ============================================================================

/**
 * NetworkEconomicMetrics - System-wide economic activity.
 *
 * This provides the transparency needed for constitutional governance.
 */
export interface NetworkEconomicMetrics {
  /** Period covered */
  periodStart: string;
  periodEnd: string;

  /** Total events in period */
  totalEvents: number;

  /** Events by type */
  eventsByType: Record<LamadEventType, number>;

  /** Total value flows by resource type */
  flowsByResourceType: Record<string, Measure>;

  /** Active agents in period */
  activeAgents: number;

  /** New agents in period */
  newAgents: number;

  /** Recognition flows */
  recognition: {
    totalFlowed: number;
    toContributorPresences: number;
    toClaimedContributors: number;
    topRecipients: { agentId: string; amount: number }[];
  };

  /** Stewardship metrics */
  stewardship: {
    activePresences: number;
    newPresences: number;
    claimedPresences: number;
    invitationsSent: number;
  };

  /** Content metrics */
  content: {
    totalViews: number;
    uniqueViewers: number;
    pathCompletions: number;
    attestationsGranted: number;
  };

  /** Currency metrics (Unyt integration) */
  currency?: {
    totalIssued: Measure;
    totalTransferred: Measure;
    totalRetired: Measure;
    velocity: number; // transfers per token per period
  };
}

// ============================================================================
// Integration Points
// ============================================================================

/**
 * hREAEventAdapter - Adapter for converting to/from hREA format.
 *
 * This will be implemented when integrating with Holochain.
 */
export interface HREAEventAdapter {
  /** Convert Lamad event to hREA GraphQL format */
  toHREA(event: EconomicEvent): unknown;

  /** Convert hREA event to Lamad format */
  fromHREA(hreaEvent: unknown): EconomicEvent;

  /** Batch convert */
  batchToHREA(events: EconomicEvent[]): unknown[];
  batchFromHREA(hreaEvents: unknown[]): EconomicEvent[];
}

/**
 * UnytEventAdapter - Adapter for Unyt/HoloFuel integration.
 *
 * Mutual credit events need special handling for countersigning.
 */
export interface UnytEventAdapter {
  /** Create a mutual credit transaction */
  createCreditTransaction(
    from: string,
    to: string,
    amount: number,
    note?: string
  ): Promise<EconomicEvent>;

  /** Get credit balance for an agent */
  getCreditBalance(agentId: string): Promise<Measure>;

  /** Get credit limit for an agent */
  getCreditLimit(agentId: string): Promise<Measure>;
}

// =============================================================================
// Wire Format (Diesel Backend)
// =============================================================================

/**
 * EconomicEventWire - Backend wire format (camelCase).
 * Maps to the economic_events table in elohim-storage.
 */
export interface EconomicEventWire {
  id: string;
  appId: string;
  action: string;
  provider: string;
  receiver: string;
  resourceConformsTo: string | null;
  resourceQuantityValue: number | null;
  resourceQuantityUnit: string | null;
  hasPointInTime: string;
  lamadEventType: string | null;
  contentId: string | null;
  contributorPresenceId: string | null;
  pathId: string | null;
  state: string;
  metadata: Record<string, unknown> | null;
  createdAt: string;
}

/**
 * Transform wire format to EconomicEvent.
 */
export function transformEventFromWire(wire: EconomicEventWire): EconomicEvent {
  const metadata = wire.metadata ?? undefined;

  // Extract lamad-specific metadata
  if (metadata) {
    metadata['lamadEventType'] = wire.lamadEventType;
    metadata['contentId'] = wire.contentId;
    metadata['presenceId'] = wire.contributorPresenceId;
    metadata['pathId'] = wire.pathId;
  }

  return {
    id: wire.id,
    action: wire.action as REAAction,
    provider: wire.provider,
    receiver: wire.receiver,
    resourceConformsTo: wire.resourceConformsTo ?? undefined,
    resourceQuantity:
      wire.resourceQuantityValue != null
        ? {
            hasNumericalValue: wire.resourceQuantityValue,
            hasUnit: wire.resourceQuantityUnit ?? 'unit-each',
          }
        : undefined,
    hasPointInTime: wire.hasPointInTime,
    state: wire.state as EventState,
    metadata,
  };
}

/**
 * Transform EconomicEvent to wire format for backend.
 */
export function transformEventToWire(
  event: EconomicEvent,
  appId = 'shefa'
): Omit<EconomicEventWire, 'createdAt'> {
  const metadata = event.metadata as Record<string, unknown> | undefined;

  return {
    id: event.id,
    appId: appId,
    action: event.action,
    provider: event.provider,
    receiver: event.receiver,
    resourceConformsTo: event.resourceConformsTo ?? null,
    resourceQuantityValue: event.resourceQuantity?.hasNumericalValue ?? null,
    resourceQuantityUnit: event.resourceQuantity?.hasUnit ?? null,
    hasPointInTime: event.hasPointInTime,
    lamadEventType: (metadata?.['lamadEventType'] as string) ?? null,
    contentId: (metadata?.['contentId'] as string) ?? null,
    contributorPresenceId: (metadata?.['presenceId'] as string) ?? null,
    pathId: (metadata?.['pathId'] as string) ?? null,
    state: event.state,
    metadata: event.metadata ?? null,
  };
}
