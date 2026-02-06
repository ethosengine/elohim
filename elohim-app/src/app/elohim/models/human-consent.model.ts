/**
 * Human Consent Model - Graduated intimacy and consent-based relationships.
 *
 * This model governs human-to-human relationships in the Elohim Protocol,
 * implementing the "graduated intimacy" pattern where relationship depth
 * is explicitly consented to by both parties.
 *
 * Key concepts:
 * - IntimacyLevel: Four levels of relationship depth
 * - ConsentState: Explicit consent lifecycle
 * - Attestation Integration: Relationship attestations gate access
 *
 * Two Graph Layers:
 * Humans ARE graph nodes, but in a different layer than content:
 * - Content Graph: What you learn about (ContentNodes)
 * - Human Graph: Who you learn with/from, who you relate to (HumanNodes)
 *
 * HumanConsent governs the PERMISSION layer - whether relationships can
 * exist and at what depth. HumanRelationship (in human-node.model.ts)
 * captures the SEMANTIC layer - what KIND of relationship it is.
 *
 * See human-node.model.ts for HumanNode and HumanRelationship types.
 *
 * Example: "I attest I am married to X" + "I attest I am married to Y"
 * enables X and Y to have their Elohim agents craft a love map path.
 */

// Import shared types from protocol-core (canonical source)
import {
  IntimacyLevel,
  ConsentState,
  type ConsentStateChange as BaseConsentStateChange,
} from '@app/elohim/models/protocol-core.model';

// @coverage: 66.7% (2026-02-05)

// Re-export for convenience (types only to avoid duplicate values)

// =========================================================================
// Human Consent Record
// =========================================================================

/**
 * HumanConsent - The permission layer for human-to-human relationships.
 *
 * This governs WHETHER a relationship can exist and at what depth:
 * - What visibility levels each human has to the other's content
 * - Whether agent-to-agent negotiation is permitted
 * - What attestation-gated resources they can access together
 *
 * Paired with HumanRelationship which captures WHAT KIND of relationship.
 * Stored on both humans' private source chains for sovereignty.
 */
export interface HumanConsent {
  /** Unique identifier for this consent relationship */
  id: string;

  /** Human who initiated the relationship */
  initiatorId: string;

  /** Human who received the request */
  participantId: string;

  /** Current intimacy level */
  intimacyLevel: IntimacyLevel;

  /** State of consent */
  consentState: ConsentState;

  // =========================================================================
  // Timestamps
  // =========================================================================

  /** When relationship was initiated */
  createdAt: string; // ISO 8601

  /** When consent state was last updated */
  updatedAt: string;

  /** When consent was given (if accepted) */
  consentedAt?: string;

  /** When relationship will expire (if set) - requires renewal */
  expiresAt?: string;

  // =========================================================================
  // Notes & Messages
  // =========================================================================

  /** Public note visible to both parties */
  publicNote?: string;

  /** Private note from initiator (only initiator can see) */
  initiatorPrivateNote?: string;

  /** Private note from participant (only participant can see) */
  participantPrivateNote?: string;

  /** Message sent with the consent request */
  requestMessage?: string;

  /** Response message (accept/decline reason) */
  responseMessage?: string;

  // =========================================================================
  // Attestation Integration
  // =========================================================================

  /**
   * Attestation IDs that validate this relationship.
   *
   * For intimate-level relationships, attestations provide cryptographic
   * verification of the relationship claim (e.g., "married to", "partnered with").
   *
   * The relationship is only valid if attestations are active.
   */
  validatingAttestationIds?: string[];

  /**
   * Required attestation type for this relationship.
   *
   * Example: "relationship:marriage" requires both parties to have
   * active marriage attestations referencing each other.
   */
  requiredAttestationType?: string;

  // =========================================================================
  // History & Audit
  // =========================================================================

  /**
   * History of consent state changes for audit purposes.
   */
  stateHistory: HumanConsentStateChange[];

  /**
   * How many times has elevation been requested/declined?
   * Used for rate limiting elevation requests.
   */
  elevationAttempts?: number;
}

/**
 * HumanConsentStateChange - Record of a consent state transition for human relationships.
 *
 * Extends the base ConsentStateChange with intimacy level tracking.
 */
export interface HumanConsentStateChange extends BaseConsentStateChange {
  /** Previous intimacy level (if changed) */
  fromLevel?: IntimacyLevel;
  /** New intimacy level (if changed) */
  toLevel?: IntimacyLevel;
}

// =========================================================================
// Consent Request/Response DTOs
// =========================================================================

/**
 * ConsentRequest - Data for initiating a consent relationship.
 */
export interface ConsentRequest {
  /** Target human ID */
  participantId: string;

  /** Requested intimacy level */
  requestedLevel: IntimacyLevel;

  /** Message to include with request */
  message?: string;

  /** For intimate level - attestation type required */
  requiredAttestationType?: string;
}

/**
 * ConsentResponse - Data for responding to a consent request.
 */
export interface ConsentResponse {
  /** Consent ID being responded to */
  consentId: string;

  /** Accept or decline */
  accept: boolean;

  /** Response message */
  message?: string;
}

/**
 * ElevationRequest - Data for requesting elevation to higher intimacy level.
 */
export interface ElevationRequest {
  /** Consent ID to elevate */
  consentId: string;

  /** Requested new level */
  newLevel: IntimacyLevel;

  /** Message explaining the request */
  message?: string;

  /** For intimate level - attestation type required */
  requiredAttestationType?: string;
}

// =========================================================================
// Relationship Attestation Types
// =========================================================================

/**
 * Common relationship attestation types for love maps.
 *
 * These are semantic types that can gate intimate-level path access.
 */
export const RELATIONSHIP_ATTESTATION_TYPES = {
  /** Married partners */
  MARRIAGE: 'relationship:marriage',

  /** Domestic partnership */
  PARTNERSHIP: 'relationship:partnership',

  /** Parent-child relationship */
  PARENT_CHILD: 'relationship:parent_child',

  /** Sibling relationship */
  SIBLING: 'relationship:sibling',

  /** Close friendship (requires mutual attestation) */
  CLOSE_FRIEND: 'relationship:close_friend',

  /** Mentor-mentee relationship */
  MENTORSHIP: 'relationship:mentorship',

  /** Business partnership */
  BUSINESS_PARTNER: 'relationship:business_partner',

  /** Custom relationship (freeform) */
  CUSTOM: 'relationship:custom',
} as const;

export type RelationshipAttestationType =
  (typeof RELATIONSHIP_ATTESTATION_TYPES)[keyof typeof RELATIONSHIP_ATTESTATION_TYPES];

// =========================================================================
// Utility Functions
// =========================================================================

/**
 * Check if consent level requires mutual attestation.
 */
export function requiresMutualAttestation(level: IntimacyLevel): boolean {
  return level === 'intimate';
}

/**
 * Check if consent can be elevated (not at max level, is active).
 */
export function canElevate(consent: HumanConsent): boolean {
  return (
    (consent.consentState === 'accepted' || consent.consentState === 'not_required') &&
    consent.intimacyLevel !== 'intimate'
  );
}

// Re-export utility functions from protocol-core for convenience

export {
  INTIMACY_LEVEL_VALUES,
  getNextIntimacyLevel,
  hasMinimumIntimacy,
  type IntimacyLevel,
  isConsentActive,
  type ConsentState,
} from '@app/elohim/models/protocol-core.model';
