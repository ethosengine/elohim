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
 * This is NOT a graph edge - humans don't become nodes. Instead, consent
 * governs visibility and negotiation permissions for shared resources
 * (paths, content, negotiations).
 *
 * Example: "I attest I am married to X" + "I attest I am married to Y"
 * enables X and Y to have their Elohim agents craft a love map path.
 */

// =========================================================================
// Intimacy Levels
// =========================================================================

/**
 * IntimacyLevel - Graduated levels of human-to-human relationship depth.
 *
 * Each level unlocks different capabilities:
 * - 'recognition': One-way acknowledgment (like citations, public endorsement)
 * - 'connection': Mutual connection (like friend requests, enables messaging)
 * - 'trusted': Elevated trust circle (access to private content/paths)
 * - 'intimate': Full love map capability (attestation-gated, agent negotiation)
 *
 * The progression is deliberate - intimacy is earned through explicit consent.
 */
export type IntimacyLevel =
  | 'recognition' // One-way acknowledgment (no consent needed from target)
  | 'connection' // Mutual connection (consent required)
  | 'trusted' // Elevated trust circle (explicit elevation request)
  | 'intimate'; // Full love map (attestation-verified relationship)

/**
 * Numeric values for intimacy levels for comparison operations.
 */
export const INTIMACY_LEVEL_VALUES: Record<IntimacyLevel, number> = {
  recognition: 0,
  connection: 1,
  trusted: 2,
  intimate: 3,
};

/**
 * Check if source level is at least as intimate as target level.
 */
export function hasMinimumIntimacy(
  current: IntimacyLevel,
  required: IntimacyLevel
): boolean {
  return INTIMACY_LEVEL_VALUES[current] >= INTIMACY_LEVEL_VALUES[required];
}

// =========================================================================
// Consent State
// =========================================================================

/**
 * ConsentState - The lifecycle state of a consent request/relationship.
 *
 * - 'not_required': One-way recognition (no consent needed)
 * - 'pending': Request sent, awaiting response
 * - 'accepted': Consent given, relationship active
 * - 'declined': Consent refused (can retry after cooldown)
 * - 'revoked': Previously accepted, now withdrawn
 */
export type ConsentState =
  | 'not_required' // For one-way recognition
  | 'pending' // Request sent, awaiting response
  | 'accepted' // Consent given
  | 'declined' // Consent refused
  | 'revoked'; // Previously accepted, now revoked

// =========================================================================
// Human Consent Record
// =========================================================================

/**
 * HumanConsent - A consent-based relationship between two humans.
 *
 * This is NOT a graph edge - humans don't become nodes in the knowledge graph.
 * Instead, this record governs:
 * - What visibility levels each human has to the other's content
 * - Whether agent-to-agent negotiation is permitted
 * - What attestation-gated resources they can access together
 *
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
  stateHistory: ConsentStateChange[];

  /**
   * How many times has elevation been requested/declined?
   * Used for rate limiting elevation requests.
   */
  elevationAttempts?: number;
}

/**
 * ConsentStateChange - Record of a consent state transition.
 */
export interface ConsentStateChange {
  fromState: ConsentState;
  toState: ConsentState;
  fromLevel?: IntimacyLevel;
  toLevel?: IntimacyLevel;
  timestamp: string;
  initiatedBy: 'initiator' | 'participant' | 'system';
  reason?: string;
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
 * Check if a consent state allows relationship usage.
 */
export function isConsentActive(state: ConsentState): boolean {
  return state === 'accepted' || state === 'not_required';
}

/**
 * Check if consent can be elevated (not at max level, is accepted).
 */
export function canElevate(consent: HumanConsent): boolean {
  return (
    consent.consentState === 'accepted' && consent.intimacyLevel !== 'intimate'
  );
}

/**
 * Get the next intimacy level, if any.
 */
export function getNextIntimacyLevel(
  current: IntimacyLevel
): IntimacyLevel | null {
  switch (current) {
    case 'recognition':
      return 'connection';
    case 'connection':
      return 'trusted';
    case 'trusted':
      return 'intimate';
    case 'intimate':
      return null; // Already at max
  }
}
