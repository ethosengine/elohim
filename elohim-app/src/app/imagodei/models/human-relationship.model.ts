/**
 * Human Relationship Model - Social graph relationships with custody capabilities.
 *
 * This model tracks relationships between humans in the social graph,
 * enabling features like:
 * - Intimacy-based access control (who can see what)
 * - Social custody (trusted contacts who can help recover your account)
 * - Auto-custody triggers (automatic verification for trusted relationships)
 *
 * Key concepts:
 * - IntimacyLevel: How close the relationship is (recognition → intimate)
 * - Consent: Both parties must consent to the relationship
 * - Custody: Enabling custody allows the other party to help in recovery
 * - Auto-custody: For highly trusted relationships, auto-verify custody requests
 *
 * This is distinct from ConsentRelationship (data consent) - this is about
 * the social bonds between humans, not about data use permissions.
 */

// =============================================================================
// Relationship Types
// =============================================================================

/**
 * HumanRelationshipType - Categories of human-to-human relationships.
 */
export const HumanRelationshipTypes = {
  FAMILY: 'family',
  FRIEND: 'friend',
  COLLEAGUE: 'colleague',
  MENTOR: 'mentor',
  MENTEE: 'mentee',
  NEIGHBOR: 'neighbor',
  ACQUAINTANCE: 'acquaintance',
  GUARDIAN: 'guardian',
  DEPENDENT: 'dependent',
  PARTNER: 'partner',
  OTHER: 'other',
} as const;

export type HumanRelationshipType = typeof HumanRelationshipTypes[keyof typeof HumanRelationshipTypes];

// =============================================================================
// Intimacy Levels
// =============================================================================

/**
 * IntimacyLevel - How close a relationship is.
 *
 * Levels (from least to most intimate):
 * - recognition: Know of each other
 * - acquaintance: Have met, casual knowledge
 * - connection: Regular contact, some trust
 * - trusted: Significant trust, share personal matters
 * - intimate: Deep trust, family-level closeness
 */
export const IntimacyLevels = {
  RECOGNITION: 'recognition',
  ACQUAINTANCE: 'acquaintance',
  CONNECTION: 'connection',
  TRUSTED: 'trusted',
  INTIMATE: 'intimate',
} as const;

export type IntimacyLevel = typeof IntimacyLevels[keyof typeof IntimacyLevels];

/**
 * Intimacy level ordering for comparison.
 */
export const INTIMACY_LEVEL_ORDER: Record<IntimacyLevel, number> = {
  recognition: 0,
  acquaintance: 1,
  connection: 2,
  trusted: 3,
  intimate: 4,
};

/**
 * Get display label for intimacy level.
 */
export function getIntimacyLabel(level: IntimacyLevel): string {
  const labels: Record<IntimacyLevel, string> = {
    recognition: 'Recognition',
    acquaintance: 'Acquaintance',
    connection: 'Connection',
    trusted: 'Trusted',
    intimate: 'Intimate',
  };
  return labels[level] ?? level;
}

/**
 * Get description for intimacy level.
 */
export function getIntimacyDescription(level: IntimacyLevel): string {
  const descriptions: Record<IntimacyLevel, string> = {
    recognition: 'Know of each other, minimal interaction',
    acquaintance: 'Have met, casual knowledge',
    connection: 'Regular contact, some trust established',
    trusted: 'Significant trust, share personal matters',
    intimate: 'Deep trust, family-level closeness',
  };
  return descriptions[level] ?? '';
}

// =============================================================================
// Human Relationship View
// =============================================================================

/**
 * HumanRelationshipView - View model for human relationships.
 */
export interface HumanRelationshipView {
  /** Unique relationship ID */
  id: string;

  /** First party in the relationship */
  partyAId: string;

  /** Second party in the relationship */
  partyBId: string;

  /** Type of relationship */
  relationshipType: HumanRelationshipType;

  /** Intimacy level */
  intimacyLevel: IntimacyLevel;

  /** Is this relationship bidirectional? */
  isBidirectional: boolean;

  // =========================================================================
  // Consent State
  // =========================================================================

  /** Has party A consented to this relationship? */
  consentGivenByA: boolean;

  /** Has party B consented to this relationship? */
  consentGivenByB: boolean;

  /** Is the relationship fully consented (both parties)? */
  isFullyConsented: boolean;

  // =========================================================================
  // Custody Capabilities
  // =========================================================================

  /** Has party A enabled custody for party B? */
  custodyEnabledByA: boolean;

  /** Has party B enabled custody for party A? */
  custodyEnabledByB: boolean;

  /**
   * Is auto-custody enabled?
   * If true, custody requests from this relationship are auto-verified.
   * Only for intimate/highly trusted relationships.
   */
  autoCustodyEnabled: boolean;

  // =========================================================================
  // Metadata
  // =========================================================================

  /** Who initiated this relationship */
  initiatedBy: string;

  /** When the relationship was verified (both consented) */
  verifiedAt?: string;

  /** Governance layer this relationship is under */
  governanceLayer?: string;

  /** Reach level for this relationship data */
  reach: string;

  /** Optional context/notes */
  context?: Record<string, unknown>;

  /** Creation timestamp */
  createdAt: string;

  /** Last updated timestamp */
  updatedAt: string;
}

// =============================================================================
// Input Types
// =============================================================================

/**
 * Input for creating a human relationship.
 */
export interface CreateHumanRelationshipInput {
  partyAId: string;
  partyBId: string;
  relationshipType: HumanRelationshipType;
  intimacyLevel?: IntimacyLevel;
  isBidirectional?: boolean;
  context?: Record<string, unknown>;
}

/**
 * Input for updating consent on a relationship.
 */
export interface UpdateConsentInput {
  relationshipId: string;
  consentGiven: boolean;
}

/**
 * Input for updating custody settings.
 */
export interface UpdateCustodyInput {
  relationshipId: string;
  custodyEnabled: boolean;
  autoCustodyEnabled?: boolean;
}

// =============================================================================
// Query Types
// =============================================================================

/**
 * Query parameters for listing human relationships.
 */
export interface HumanRelationshipQuery {
  /** Filter by party (as either A or B) */
  partyId?: string;

  /** Filter by specific party A */
  partyAId?: string;

  /** Filter by specific party B */
  partyBId?: string;

  /** Filter by relationship type */
  relationshipType?: HumanRelationshipType;

  /** Filter by minimum intimacy level */
  minIntimacyLevel?: IntimacyLevel;

  /** Only return fully consented relationships */
  fullyConsentedOnly?: boolean;

  /** Only return relationships with custody enabled */
  custodyEnabledOnly?: boolean;

  /** Pagination */
  limit?: number;
  offset?: number;
}

// =============================================================================
// Wire Format (Diesel Backend)
// =============================================================================

/**
 * HumanRelationshipWire - Backend wire format (snake_case).
 * Maps to the human_relationships table in elohim-storage.
 */
export interface HumanRelationshipWire {
  id: string;
  app_id: string;
  party_a_id: string;
  party_b_id: string;
  relationship_type: string;
  intimacy_level: string;
  is_bidirectional: number; // SQLite boolean
  consent_given_by_a: number;
  consent_given_by_b: number;
  custody_enabled_by_a: number;
  custody_enabled_by_b: number;
  auto_custody_enabled: number;
  initiated_by: string;
  verified_at: string | null;
  governance_layer: string | null;
  reach: string;
  context_json: string | null;
  created_at: string;
  updated_at: string;
}

/**
 * Transform wire format to HumanRelationshipView.
 */
export function transformHumanRelationshipFromWire(wire: HumanRelationshipWire): HumanRelationshipView {
  let context: Record<string, unknown> | undefined;
  try {
    context = wire.context_json ? JSON.parse(wire.context_json) : undefined;
  } catch {
    // Ignore parse errors
  }

  const consentA = wire.consent_given_by_a === 1;
  const consentB = wire.consent_given_by_b === 1;

  return {
    id: wire.id,
    partyAId: wire.party_a_id,
    partyBId: wire.party_b_id,
    relationshipType: wire.relationship_type as HumanRelationshipType,
    intimacyLevel: wire.intimacy_level as IntimacyLevel,
    isBidirectional: wire.is_bidirectional === 1,
    consentGivenByA: consentA,
    consentGivenByB: consentB,
    isFullyConsented: consentA && consentB,
    custodyEnabledByA: wire.custody_enabled_by_a === 1,
    custodyEnabledByB: wire.custody_enabled_by_b === 1,
    autoCustodyEnabled: wire.auto_custody_enabled === 1,
    initiatedBy: wire.initiated_by,
    verifiedAt: wire.verified_at ?? undefined,
    governanceLayer: wire.governance_layer ?? undefined,
    reach: wire.reach,
    context,
    createdAt: wire.created_at,
    updatedAt: wire.updated_at,
  };
}

/**
 * Transform HumanRelationshipView to wire format for backend.
 */
export function transformHumanRelationshipToWire(
  view: HumanRelationshipView,
  appId: string = 'imagodei'
): Omit<HumanRelationshipWire, 'created_at' | 'updated_at'> {
  return {
    id: view.id,
    app_id: appId,
    party_a_id: view.partyAId,
    party_b_id: view.partyBId,
    relationship_type: view.relationshipType,
    intimacy_level: view.intimacyLevel,
    is_bidirectional: view.isBidirectional ? 1 : 0,
    consent_given_by_a: view.consentGivenByA ? 1 : 0,
    consent_given_by_b: view.consentGivenByB ? 1 : 0,
    custody_enabled_by_a: view.custodyEnabledByA ? 1 : 0,
    custody_enabled_by_b: view.custodyEnabledByB ? 1 : 0,
    auto_custody_enabled: view.autoCustodyEnabled ? 1 : 0,
    initiated_by: view.initiatedBy,
    verified_at: view.verifiedAt ?? null,
    governance_layer: view.governanceLayer ?? null,
    reach: view.reach,
    context_json: view.context ? JSON.stringify(view.context) : null,
  };
}
