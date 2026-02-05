/**
 * Path Negotiation Model - Elohim-to-Elohim path generation.
 *
 * This model defines the placeholder interface for agent-to-agent negotiation
 * of emergent "love map" paths between two humans.
 *
 * MVP: Simple affinity comparison and manual path creation.
 * Holochain: Full agent-to-agent negotiation protocol.
 *
 * The Love Map Pattern:
 * 1. Alice and Bob each explore the knowledge graph, building affinity patterns
 * 2. With mutual consent (intimate level + attestation), they enable negotiation
 * 3. Their Elohim agents analyze shared and divergent affinities
 * 4. A bridging algorithm generates an emergent path connecting their interests
 * 5. Both humans can follow this path from either direction - meeting in understanding
 *
 * This is the technical foundation for relationships built on shared learning.
 */

import type { IntimacyLevel } from '@app/elohim/models/human-consent.model';

// @coverage: 75.0% (2026-02-05)

// =========================================================================
// Negotiation Status
// =========================================================================

/**
 * NegotiationStatus - Lifecycle states of a path negotiation.
 *
 * - 'proposed': Initiator proposed, awaiting participant response
 * - 'analyzing': Both consented, agents analyzing affinity patterns
 * - 'negotiating': Agents exchanging proposals for path structure
 * - 'accepted': Path generated and accepted by both parties
 * - 'declined': One party declined the negotiation
 * - 'failed': Technical failure during negotiation
 * - 'expired': Negotiation timed out without resolution
 */
export type NegotiationStatus =
  | 'proposed'
  | 'analyzing'
  | 'negotiating'
  | 'accepted'
  | 'declined'
  | 'failed'
  | 'expired';

// =========================================================================
// Bridging Strategy
// =========================================================================

/**
 * BridgingStrategy - Algorithm for generating emergent paths.
 *
 * Different strategies create different learning experiences:
 *
 * - 'shortest_path': Find shortest graph path between affinity clusters
 *   Best for: Quick connection, minimal new content
 *
 * - 'maximum_overlap': Maximize shared concepts in the path
 *   Best for: Building on common ground, comfortable learning
 *
 * - 'complementary': Emphasize concepts each can teach the other
 *   Best for: Mutual growth, balanced exchange
 *
 * - 'exploration': Include novel concepts neither has explored
 *   Best for: Shared adventure, discovering together
 *
 * - 'custom': Future - AI-negotiated custom strategy based on goals
 */
export type BridgingStrategy =
  | 'shortest_path'
  | 'maximum_overlap'
  | 'complementary'
  | 'exploration'
  | 'custom';

/**
 * Strategy metadata for UI display.
 */
export const BRIDGING_STRATEGY_INFO: Record<
  BridgingStrategy,
  { name: string; description: string }
> = {
  shortest_path: {
    name: 'Quick Connection',
    description: 'Find the shortest path between your shared interests',
  },
  maximum_overlap: {
    name: 'Common Ground',
    description: 'Focus on concepts you both already resonate with',
  },
  complementary: {
    name: 'Mutual Teaching',
    description: 'Learn from each other - concepts one knows and the other does not',
  },
  exploration: {
    name: 'Shared Adventure',
    description: 'Discover new concepts together that neither has explored',
  },
  custom: {
    name: 'Custom Strategy',
    description: 'Let your Elohim agents negotiate a personalized approach',
  },
};

// =========================================================================
// Path Negotiation Record
// =========================================================================

/**
 * PathNegotiation - A negotiation session for generating an emergent path.
 *
 * This record tracks the full lifecycle of a love map path creation,
 * from proposal through analysis to acceptance.
 *
 * Stored on both participants' source chains for sovereignty.
 */
export interface PathNegotiation {
  /** Unique identifier */
  id: string;

  /** Human who initiated the negotiation */
  initiatorId: string;

  /** Human invited to negotiate */
  participantId: string;

  /** Current negotiation status */
  status: NegotiationStatus;

  // =========================================================================
  // Consent & Validation
  // =========================================================================

  /**
   * Consent relationship ID that authorizes this negotiation.
   * Must be at intimate level with active attestations.
   */
  consentId: string;

  /**
   * Required intimacy level for this negotiation.
   * Typically 'intimate' for love maps.
   */
  requiredIntimacyLevel: IntimacyLevel;

  /**
   * Attestation IDs that validate this negotiation.
   * Both participants must have active attestations.
   */
  validatingAttestationIds: string[];

  // =========================================================================
  // Affinity Analysis
  // =========================================================================

  /**
   * Shared high-affinity concept IDs found in both humans' patterns.
   * These form the "common ground" for the emergent path.
   */
  sharedAffinityNodes: string[];

  /**
   * Divergent concepts unique to each human.
   * These represent learning opportunities for the other person.
   */
  divergentNodes: {
    /** Concepts initiator has affinity for that participant doesn't */
    initiator: string[];
    /** Concepts participant has affinity for that initiator doesn't */
    participant: string[];
  };

  /**
   * Affinity strength scores for shared concepts.
   * Higher scores = stronger shared resonance.
   */
  sharedAffinityScores?: Record<string, number>;

  // =========================================================================
  // Path Generation
  // =========================================================================

  /** Selected bridging strategy */
  bridgingStrategy?: BridgingStrategy;

  /** Custom strategy parameters (for 'custom' strategy) */
  customStrategyParams?: Record<string, unknown>;

  /** Generated path ID (if negotiation succeeded) */
  generatedPathId?: string;

  /**
   * Proposed path structure before final acceptance.
   * Both parties can review before accepting.
   */
  proposedPathStructure?: ProposedPathStructure;

  // =========================================================================
  // Timestamps
  // =========================================================================

  /** When negotiation was initiated */
  createdAt: string; // ISO 8601

  /** When status was last updated */
  updatedAt: string;

  /** When negotiation was resolved (accepted/declined/failed) */
  resolvedAt?: string;

  /** Negotiation timeout - auto-expires if not resolved */
  expiresAt?: string;

  // =========================================================================
  // Communication Log
  // =========================================================================

  /**
   * Messages exchanged during negotiation.
   *
   * MVP: Human messages only
   * Holochain: Includes agent-to-agent protocol messages
   */
  negotiationLog: NegotiationMessage[];
}

/**
 * ProposedPathStructure - Preview of generated path before acceptance.
 */
export interface ProposedPathStructure {
  /** Proposed path title */
  title: string;

  /** Proposed path description */
  description: string;

  /** Step count */
  stepCount: number;

  /** Estimated duration */
  estimatedDuration: string;

  /** Concepts included */
  conceptIds: string[];

  /** How many concepts are shared vs. teaching opportunities */
  stats: {
    sharedConcepts: number;
    initiatorTeaching: number; // Concepts initiator can teach participant
    participantTeaching: number; // Concepts participant can teach initiator
    novelConcepts: number; // New to both
  };
}

// =========================================================================
// Negotiation Messages
// =========================================================================

/**
 * NegotiationMessage - A message in the negotiation log.
 */
export interface NegotiationMessage {
  /** Message author (human ID or 'system' for automated messages) */
  authorId: string;

  /** When message was sent */
  timestamp: string; // ISO 8601

  /** Message type */
  type: NegotiationMessageType;

  /** Message content (human-readable) */
  content: string;

  /** Structured metadata (for agent messages) */
  metadata?: Record<string, unknown>;
}

/**
 * NegotiationMessageType - Types of messages in negotiation log.
 */
export type NegotiationMessageType =
  | 'proposal' // Initial or revised proposal
  | 'counter' // Counter-proposal
  | 'accept' // Acceptance message
  | 'decline' // Decline message
  | 'question' // Clarifying question
  | 'comment' // General comment
  | 'system' // System/automated message
  | 'agent'; // Agent-to-agent protocol message (Holochain)

// =========================================================================
// Negotiation Request/Response DTOs
// =========================================================================

/**
 * NegotiationRequest - Data for initiating a path negotiation.
 */
export interface NegotiationRequest {
  /** Target human ID */
  participantId: string;

  /** Consent ID authorizing this negotiation */
  consentId: string;

  /** Preferred bridging strategy */
  preferredStrategy?: BridgingStrategy;

  /** Optional message to include with request */
  message?: string;

  /** Custom goals/preferences for path generation */
  goals?: string[];
}

/**
 * NegotiationResponse - Data for responding to a negotiation request.
 */
export interface NegotiationResponse {
  /** Negotiation ID being responded to */
  negotiationId: string;

  /** Accept or decline */
  accept: boolean;

  /** Preferred strategy (if accepting) */
  preferredStrategy?: BridgingStrategy;

  /** Response message */
  message?: string;
}

/**
 * PathAcceptance - Data for accepting a generated path.
 */
export interface PathAcceptance {
  /** Negotiation ID */
  negotiationId: string;

  /** Accept or request changes */
  accept: boolean;

  /** Feedback if requesting changes */
  feedback?: string;
}

// =========================================================================
// Affinity Analysis Types
// =========================================================================

/**
 * AffinityAnalysis - Result of analyzing two humans' affinity patterns.
 */
export interface AffinityAnalysis {
  /** Human 1 ID */
  human1Id: string;

  /** Human 2 ID */
  human2Id: string;

  /** When analysis was performed */
  analyzedAt: string;

  /** Concepts both humans have high affinity for */
  sharedHighAffinity: AffinityNode[];

  /** Concepts with divergent affinity */
  divergent: {
    human1Only: AffinityNode[];
    human2Only: AffinityNode[];
  };

  /** Overall compatibility score (0.0 - 1.0) */
  compatibilityScore: number;

  /** Recommended bridging strategies based on analysis */
  recommendedStrategies: BridgingStrategy[];
}

/**
 * AffinityNode - A concept with affinity score.
 */
export interface AffinityNode {
  /** Concept/content node ID */
  nodeId: string;

  /** Affinity score (0.0 - 1.0) */
  affinity: number;

  /** Mastery level if any */
  masteryLevel?: string;
}

// =========================================================================
// Utility Functions
// =========================================================================

/**
 * Check if a negotiation is still active (can receive updates).
 */
export function isNegotiationActive(status: NegotiationStatus): boolean {
  return status === 'proposed' || status === 'analyzing' || status === 'negotiating';
}

/**
 * Check if a negotiation has been resolved (final state).
 */
export function isNegotiationResolved(status: NegotiationStatus): boolean {
  return (
    status === 'accepted' || status === 'declined' || status === 'failed' || status === 'expired'
  );
}

/**
 * Check if a negotiation succeeded.
 */
export function isNegotiationSuccessful(status: NegotiationStatus): boolean {
  return status === 'accepted';
}
