/**
 * Protocol Core - Shared Primitives for the Elohim Protocol
 *
 * This module defines the foundational types shared across all four pillars:
 * - Imago Dei (Identity) - Who you are
 * - Lamad (Content) - What you're learning/becoming
 * - Qahal (Community) - Who you're with
 * - Shefa (Economy) - How value flows
 *
 * Key Insight: These pillars are not separate systems but different views
 * of the same underlying reality. A person learning (Lamad) does so in
 * community (Qahal), through their identity (Imago Dei), creating value
 * flows (Shefa).
 *
 * "First we build the tools, then they build us." — Marshall McLuhan
 *
 * The tools we build here will shape the communities that form around them.
 * These primitives encode our constitutional commitments:
 * - Graduated intimacy (consent-based relationship depth)
 * - Geographic embodiment (humans are placed beings)
 * - Governance layers (from household to global)
 * - Attestation-based trust (claims verified by community)
 *
 * Theological grounding:
 * - Humans are embodied, relational, placed beings (imago Dei)
 * - Community is the context for flourishing (ecclesia/qahal)
 * - Learning is transformation toward the good (lamad)
 * - Economics serves human flourishing, not extraction (shefa)
 */

// ============================================================================
// REACH LEVELS - Graduated Visibility
// ============================================================================

/**
 * ReachLevel - Geographic/jurisdictional scope of visibility.
 *
 * These represent concentric geographic circles, each containing the previous.
 * Reach answers: "How far can this travel geographically?"
 *
 * Note: This is separate from AffinityScope (interest-based filtering)
 * and federation (whether content crosses instance boundaries).
 *
 * Used by:
 * - Lamad: ContentReach (geographic boundary for content)
 * - Qahal: HumanReach (geographic visibility of profile)
 * - Economic layer: TokenReach (where value can flow geographically)
 * - Imago Dei: IdentityReach (geographic scope of identity disclosure)
 */
export type ReachLevel =
  | 'private' // Only the agent themselves (and their Elohim)
  | 'invited' // Explicitly invited individuals (regardless of location)
  | 'local' // Household/immediate dwelling
  | 'neighborhood' // Block/building/immediate area
  | 'municipal' // City/town
  | 'bioregional' // Watershed/ecosystem boundary (crosses political lines)
  | 'regional' // State/province level
  | 'commons'; // Globally public

/**
 * Numeric values for reach levels for comparison operations.
 */
export const REACH_LEVEL_VALUES: Record<ReachLevel, number> = {
  private: 0,
  invited: 1,
  local: 2,
  neighborhood: 3,
  municipal: 4,
  bioregional: 5,
  regional: 6,
  commons: 7,
};

// ============================================================================
// AFFINITY SCOPE - Interest-Based Community Filtering
// ============================================================================

/**
 * AffinityScope - Interest/community filter for content visibility.
 *
 * Unlike ReachLevel (geographic), AffinityScope filters by community membership
 * regardless of location. A 'denomination' scope might span continents.
 *
 * Affinity answers: "Who is this *for*, regardless of where they are?"
 *
 * Content can combine both:
 * - { reach: 'commons', affinity: 'denomination' } → globally visible, but only to my faith network
 * - { reach: 'bioregional', affinity: 'open' } → everyone in my watershed
 * - { reach: 'municipal', affinity: 'professional' } → nurses in my city
 */
export type AffinityScope =
  | 'personal' // Just me (private affinity)
  | 'household' // My family unit
  | 'congregation' // My local faith community
  | 'denomination' // Broader faith affiliation network
  | 'professional' // Guild/trade/field community
  | 'special_district' // School district, water district, etc.
  | 'interest_group' // Topic-based community (hobby, cause, etc.)
  | 'open'; // No affinity filter - anyone within reach

/**
 * ContentVisibility - Combined visibility specification.
 *
 * Separates three orthogonal concerns:
 * - reach: Geographic boundary (how far)
 * - affinity: Community filter (who within that geography)
 * - federated: Instance boundary crossing (how it travels)
 */
export interface ContentVisibility {
  /** Geographic scope - concentric circles */
  reach: ReachLevel;

  /** Interest-based filter - who this is for */
  affinity: AffinityScope;

  /** Whether content can cross instance boundaries at this reach */
  federated: boolean;
}

/**
 * Check if source reach encompasses target reach.
 */
export function reachEncompasses(source: ReachLevel, target: ReachLevel): boolean {
  return REACH_LEVEL_VALUES[source] >= REACH_LEVEL_VALUES[target];
}

// ============================================================================
// INTIMACY LEVELS - Graduated Relationship Depth
// ============================================================================

/**
 * IntimacyLevel - Depth of relationship between agents.
 *
 * Each level unlocks different capabilities:
 * - recognition: One-way acknowledgment (no consent needed from target)
 * - connection: Mutual connection (consent required)
 * - trusted: Elevated trust circle (access to private content/paths)
 * - intimate: Full love map capability (attestation-gated, agent negotiation)
 *
 * Used by:
 * - Qahal: HumanRelationship intimacy
 * - Lamad: Path sharing permissions
 * - Shefa: Value flow permissions
 */
export type IntimacyLevel =
  | 'recognition' // One-way acknowledgment
  | 'connection' // Mutual connection
  | 'trusted' // Elevated trust
  | 'intimate'; // Full love map capability

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
 * Check if current intimacy meets minimum required level.
 */
export function hasMinimumIntimacy(current: IntimacyLevel, required: IntimacyLevel): boolean {
  return INTIMACY_LEVEL_VALUES[current] >= INTIMACY_LEVEL_VALUES[required];
}

/**
 * Get the next intimacy level, if any.
 */
export function getNextIntimacyLevel(current: IntimacyLevel): IntimacyLevel | null {
  switch (current) {
    case 'recognition':
      return 'connection';
    case 'connection':
      return 'trusted';
    case 'trusted':
      return 'intimate';
    case 'intimate':
      return null;
  }
}

// ============================================================================
// CONSENT STATE - Permission Lifecycle
// ============================================================================

/**
 * ConsentState - Universal consent lifecycle for all permission types.
 *
 * Used by:
 * - Qahal: HumanConsent for relationships
 * - Lamad: Content access permissions
 * - Shefa: Value transfer authorizations
 * - Imago Dei: Identity disclosure permissions
 */
export type ConsentState =
  | 'not_required' // No consent needed (e.g., public content)
  | 'pending' // Request sent, awaiting response
  | 'accepted' // Consent given, permission active
  | 'declined' // Consent refused (can retry after cooldown)
  | 'revoked' // Previously accepted, now withdrawn
  | 'expired'; // Time-limited consent that has lapsed

/**
 * Check if a consent state allows the permission to be used.
 */
export function isConsentActive(state: ConsentState): boolean {
  return state === 'accepted' || state === 'not_required';
}

// ============================================================================
// GOVERNANCE LAYERS - Nested Subsidiarity
// ============================================================================

/**
 * GovernanceLayer - Levels of governance from intimate to global.
 *
 * Implements subsidiarity: decisions made at the most local level capable.
 * Each layer has its own governance processes and constitutional constraints.
 *
 * Used by:
 * - Qahal: Community governance scope
 * - Lamad: Content governance (who can moderate)
 * - Shefa: Economic governance (token rules by layer)
 * - Place: Geographic governance alignment
 */
export type GovernanceLayer =
  // Intimate layers (high trust, small scale)
  | 'self' // Individual agency
  | 'household' // Family/living unit
  | 'extended_family' // Multi-generational family

  // Local layers (place-based community)
  | 'neighborhood' // Block, street, immediate area
  | 'municipality' // City, town, village
  | 'county_regional' // County, district, region

  // Broader layers (identity/affinity-based)
  | 'affinity_network' // Interest/identity group (not place-based)
  | 'workplace' // Organizational context
  | 'faith_community' // Religious/spiritual community

  // Wide layers (large-scale coordination)
  | 'state_provincial' // State, province
  | 'national' // Nation-state
  | 'bioregional' // Ecological boundaries (watershed, etc.)
  | 'continental' // Continental scope
  | 'global'; // Planetary coordination

/**
 * Governance layer ordering (from most local to most global).
 */
export const GOVERNANCE_LAYER_ORDER: GovernanceLayer[] = [
  'self',
  'household',
  'extended_family',
  'neighborhood',
  'municipality',
  'county_regional',
  'affinity_network',
  'workplace',
  'faith_community',
  'state_provincial',
  'national',
  'bioregional',
  'continental',
  'global',
];

/**
 * Get the index of a governance layer (for ordering).
 */
export function governanceLayerIndex(layer: GovernanceLayer): number {
  return GOVERNANCE_LAYER_ORDER.indexOf(layer);
}

/**
 * Check if layer A is more local than layer B.
 */
export function isMoreLocal(a: GovernanceLayer, b: GovernanceLayer): boolean {
  return governanceLayerIndex(a) < governanceLayerIndex(b);
}

// ============================================================================
// GEOGRAPHIC CONTEXT - Embodied Place
// ============================================================================

/**
 * GeographicLayer - Classification of place types.
 *
 * Parallel to GovernanceLayer but specifically for geographic scope.
 * Some governance layers are place-based, others are not.
 */
export type GeographicLayer =
  | 'point' // Specific location (lat/lng)
  | 'building' // Single structure
  | 'block' // City block
  | 'neighborhood' // Neighborhood area
  | 'municipality' // City/town boundaries
  | 'county' // County/district
  | 'state_province' // State/province
  | 'nation' // Country
  | 'continent' // Continental
  | 'global' // Worldwide
  | 'bioregion'; // Ecological boundary (watershed, etc.)

/**
 * GeographicContext - Location information attached to entities.
 *
 * Used by:
 * - Qahal: Human primary location
 * - Lamad: Content geographic relevance
 * - Shefa: Economic activity location
 * - Place: The place model itself
 */
export interface GeographicContext {
  /** What level of geographic specificity */
  layer: GeographicLayer;

  /** Human-readable name for the location */
  displayName: string;

  /** Who can see this location information */
  reach: ReachLevel;

  /** Optional coordinates (only if reach allows) */
  coordinates?: {
    latitude: number;
    longitude: number;
    accuracy?: 'precise' | 'approximate' | 'area';
  };

  /** Reference to a Place node (if one exists) */
  placeId?: string;

  /** Timezone (for human coordination) */
  timezone?: string;
}

// ============================================================================
// PROTOCOL AGENT - Universal Agent Identity
// ============================================================================

/**
 * AgentType - Classification of agents in the protocol.
 *
 * All economic, social, and learning activity involves agents.
 * Agents can be humans, organizations, AI entities, or presences.
 */
export type AgentType =
  | 'human' // Individual person (embodied)
  | 'organization' // Group with shared identity
  | 'community' // Community collective
  | 'household' // Family/living unit
  | 'contributor_presence' // Unclaimed/stewarded external contributor
  | 'elohim' // AI agent (constitutional steward)
  | 'system'; // Protocol infrastructure

/**
 * ProtocolAgent - Minimal agent identity shared across pillars.
 *
 * This is the base identity that appears in all four pillars.
 * Each pillar extends this with domain-specific attributes.
 */
export interface ProtocolAgent {
  /** Unique identifier */
  id: string;

  /** Agent type classification */
  type: AgentType;

  /** Display name */
  displayName: string;

  /** Decentralized identifier (W3C DID) */
  did?: string;

  /** Is this agent currently active? */
  isActive: boolean;

  /** When this agent was created */
  createdAt: string;

  /** Last activity timestamp */
  lastActiveAt?: string;
}

// ============================================================================
// ATTESTATION - Universal Claim Structure
// ============================================================================

/**
 * AttestationType - Categories of attestations.
 *
 * Attestations are claims made by agents about other agents or content.
 * They form the basis of trust in the protocol.
 */
export type AttestationType =
  // Identity attestations (Imago Dei)
  | 'identity_verification' // Attesting someone is who they claim
  | 'relationship_claim' // Attesting a relationship exists

  // Capability attestations (Lamad)
  | 'skill_attestation' // Attesting demonstrated skill
  | 'completion_attestation' // Attesting path completion
  | 'authorship_attestation' // Attesting content authorship

  // Trust attestations (Qahal)
  | 'community_membership' // Attesting community membership
  | 'role_attestation' // Attesting role in organization
  | 'endorsement' // General endorsement

  // Economic attestations (Shefa)
  | 'contribution_attestation' // Attesting value contribution
  | 'stewardship_attestation' // Attesting stewardship commitment

  // Meta attestations
  | 'attestation_revocation' // Revoking a previous attestation
  | 'dispute' // Disputing another attestation
  | 'custom'; // User-defined attestation type

/**
 * AttestationStatus - Lifecycle of an attestation.
 */
export type AttestationStatus =
  | 'pending' // Awaiting verification
  | 'active' // Currently valid
  | 'suspended' // Temporarily suspended (under review)
  | 'revoked' // Permanently revoked
  | 'expired' // Time-limited and lapsed
  | 'disputed'; // Under governance dispute

/**
 * Attestation - A verifiable claim in the protocol.
 *
 * This is the universal attestation structure used across all pillars.
 * Inspired by W3C Verifiable Credentials.
 */
export interface Attestation {
  /** Unique identifier */
  id: string;

  /** What kind of attestation */
  type: AttestationType;

  /** Who issued this attestation */
  issuerId: string;

  /** Who/what is this attestation about */
  subjectId: string;

  /** The claim being made (structured or string) */
  claim: string | Record<string, unknown>;

  /** Current status */
  status: AttestationStatus;

  /** When issued */
  issuedAt: string;

  /** When it expires (optional) */
  expiresAt?: string;

  /** When it was revoked (if revoked) */
  revokedAt?: string;

  /** Revocation reason (if revoked) */
  revocationReason?: string;

  /** Evidence supporting this attestation */
  evidenceIds?: string[];

  /** Governance layer this attestation operates in */
  governanceLayer?: GovernanceLayer;

  /** Reach of this attestation (who can see it) */
  reach: ReachLevel;

  /** Cryptographic proof (for verified credentials) */
  proof?: {
    type: string;
    created: string;
    verificationMethod: string;
    proofValue: string;
  };
}

// ============================================================================
// CONSENT RECORD - Universal Permission Structure
// ============================================================================

/**
 * ConsentRecord - A record of consent between agents.
 *
 * This is the base consent structure used across pillars.
 * Qahal extends this for HumanConsent, Shefa for value transfer consent, etc.
 */
export interface ConsentRecord {
  /** Unique identifier */
  id: string;

  /** Who initiated the consent request */
  initiatorId: string;

  /** Who is being asked for consent */
  participantId: string;

  /** Current state */
  state: ConsentState;

  /** What is being consented to (domain-specific) */
  scope: string;

  /** Governance layer this consent operates in */
  governanceLayer?: GovernanceLayer;

  /** When created */
  createdAt: string;

  /** When last updated */
  updatedAt: string;

  /** When consent was given (if accepted) */
  consentedAt?: string;

  /** When consent expires (if time-limited) */
  expiresAt?: string;

  /** Message with the request */
  requestMessage?: string;

  /** Response message */
  responseMessage?: string;

  /** History of state changes */
  stateHistory: ConsentStateChange[];
}

/**
 * ConsentStateChange - Record of a consent state transition.
 */
export interface ConsentStateChange {
  fromState: ConsentState;
  toState: ConsentState;
  timestamp: string;
  initiatedBy: 'initiator' | 'participant' | 'system';
  reason?: string;
}

// ============================================================================
// PILLAR REFERENCES - Cross-Pillar Linking
// ============================================================================

/**
 * Pillar - The four pillars of the Elohim Protocol.
 */
export type Pillar = 'imago_dei' | 'lamad' | 'qahal' | 'shefa';

/**
 * PillarReference - A reference to an entity in another pillar.
 *
 * Used for cross-pillar linking without tight coupling.
 */
export interface PillarReference {
  /** Which pillar this entity lives in */
  pillar: Pillar;

  /** The entity ID within that pillar */
  entityId: string;

  /** The entity type within that pillar */
  entityType: string;

  /** Human-readable label for UI */
  displayLabel?: string;
}

/**
 * CrossPillarLink - A link between entities across pillars.
 *
 * Examples:
 * - Lamad ContentNode → Qahal ContributorPresence (authorship)
 * - Qahal HumanRelationship → Shefa TokenFlow (relationship enables transfer)
 * - Imago Dei Identity → Shefa Claim (identity enables claiming)
 */
export interface CrossPillarLink {
  /** Unique identifier */
  id: string;

  /** Source entity */
  source: PillarReference;

  /** Target entity */
  target: PillarReference;

  /** What kind of link */
  linkType: CrossPillarLinkType;

  /** Metadata about the link */
  metadata?: Record<string, unknown>;

  /** When established */
  createdAt: string;
}

/**
 * CrossPillarLinkType - Types of cross-pillar relationships.
 */
export type CrossPillarLinkType =
  // Imago Dei ↔ Lamad
  | 'identity_authors_content' // Human authored content
  | 'identity_attests_skill' // Human has skill from learning

  // Imago Dei ↔ Qahal
  | 'identity_participates_community' // Human is community member
  | 'identity_has_relationship' // Human has relationship

  // Imago Dei ↔ Shefa
  | 'identity_enables_claim' // Identity enables value claiming
  | 'identity_receives_value' // Identity receives attribution

  // Lamad ↔ Qahal
  | 'content_shared_in_community' // Content visible in community
  | 'path_negotiated_together' // Humans negotiating learning path

  // Lamad ↔ Shefa
  | 'content_generates_value' // Content creates economic value
  | 'learning_earns_credential' // Learning produces credential

  // Qahal ↔ Shefa
  | 'relationship_enables_flow' // Relationship allows value transfer
  | 'community_stewards_commons' // Community manages commons pool

  // Meta
  | 'attestation_bridges' // Attestation connects pillars
  | 'custom'; // Custom link type

// ============================================================================
// TOKEN TYPE - Shefa Value Types
// ============================================================================

/**
 * TokenType - Categories of value in the Shefa economy.
 *
 * From the Shefa whitepaper - multi-dimensional value tracking.
 */
export type TokenType =
  | 'care' // Generated by caregiving acts
  | 'time' // Hours contributed to community
  | 'learning' // Skills developed and taught
  | 'steward' // Environmental/resource protection
  | 'creator' // Content that helps others
  | 'infrastructure' // Network maintenance contribution
  | 'recognition' // General appreciation/endorsement
  | 'custom'; // Community-defined token type

/**
 * TokenDecayRate - How quickly tokens lose value if not circulated.
 *
 * Implements demurrage to encourage circulation.
 */
export type TokenDecayRate =
  | 'none' // No decay (learning, creator tokens)
  | 'low' // Slow decay (time, steward tokens)
  | 'medium' // Moderate decay (care tokens)
  | 'high'; // Fast decay (infrastructure tokens)

/**
 * TokenSpecification - Definition of a token type.
 */
export interface TokenSpecification {
  /** Token type identifier */
  type: TokenType;

  /** Human-readable name */
  name: string;

  /** Description */
  description: string;

  /** What generates this token */
  generatedBy: string;

  /** What it can be exchanged for */
  circulatesFor: string[];

  /** Decay rate */
  decayRate: TokenDecayRate;

  /** Symbol for display */
  symbol: string;

  /** Icon for UI */
  icon?: string;
}

/**
 * Standard token specifications for multi-dimensional value tracking.
 */
export const TOKEN_SPECS: Record<TokenType, TokenSpecification> = {
  care: {
    type: 'care',
    name: 'Care Token',
    description: 'Generated by witnessed caregiving acts',
    generatedBy: 'Caregiving events witnessed by Elohim',
    circulatesFor: ['services', 'goods', 'recognition'],
    decayRate: 'medium',
    symbol: '♥',
  },
  time: {
    type: 'time',
    name: 'Time Token',
    description: 'Hours contributed to community',
    generatedBy: 'Time spent on community service',
    circulatesFor: ['coordination', 'services'],
    decayRate: 'low',
    symbol: '⏱',
  },
  learning: {
    type: 'learning',
    name: 'Learning Token',
    description: 'Skills developed and taught',
    generatedBy: 'Completing learning paths, teaching others',
    circulatesFor: ['education', 'mentorship'],
    decayRate: 'none',
    symbol: '📚',
  },
  steward: {
    type: 'steward',
    name: 'Steward Token',
    description: 'Environmental/resource protection',
    generatedBy: 'Stewardship of land, resources, commons',
    circulatesFor: ['sustainable goods', 'restoration'],
    decayRate: 'low',
    symbol: '🌱',
  },
  creator: {
    type: 'creator',
    name: 'Creator Token',
    description: 'Content that helps others',
    generatedBy: 'Creating content used by community',
    circulatesFor: ['derivative rights', 'recognition'],
    decayRate: 'none',
    symbol: '✨',
  },
  infrastructure: {
    type: 'infrastructure',
    name: 'Infrastructure Token',
    description: 'Network maintenance contribution',
    generatedBy: 'Running nodes, maintaining protocol',
    circulatesFor: ['protocol services'],
    decayRate: 'high',
    symbol: '🔧',
  },
  recognition: {
    type: 'recognition',
    name: 'Recognition Token',
    description: 'General appreciation/endorsement',
    generatedBy: 'Endorsements, appreciations, thanks',
    circulatesFor: ['reputation', 'social capital'],
    decayRate: 'medium',
    symbol: '⭐',
  },
  custom: {
    type: 'custom',
    name: 'Custom Token',
    description: 'Community-defined token type',
    generatedBy: 'Community-defined rules',
    circulatesFor: ['community-defined'],
    decayRate: 'medium',
    symbol: '◆',
  },
};

// ============================================================================
// CONSTITUTIONAL CONSTRAINTS - Inviolable Rules
// ============================================================================

/**
 * ConstitutionalConstraintType - Categories of constitutional rules.
 *
 * These are the inviolable constraints that govern the protocol.
 * From Shefa's four-layer governance.
 */
export type ConstitutionalConstraintType =
  // Layer 1: Dignity Floor
  | 'dignity_minimum' // Basic needs floor
  | 'care_recognition' // Care labor always recognized

  // Layer 2: Attribution
  | 'contribution_attribution' // Value flows to creators
  | 'attribution_persistence' // Attribution persists offline

  // Layer 3: Circulation
  | 'circulation_requirement' // Tokens must circulate
  | 'accumulation_limit' // Hoarding triggers redistribution
  | 'demurrage_enforcement' // Decay on hoarding

  // Layer 4: Sustainability
  | 'next_community' // Portion to next liberation
  | 'infrastructure_funding' // Protocol maintenance
  | 'ecological_limit'; // Bioregional boundaries

/**
 * EnforcementLevel - What happens when a constraint is violated.
 */
export type EnforcementLevel =
  | 'warning' // Alert but no action
  | 'soft_limit' // Limit with override possible
  | 'require_governance' // Must go through deliberation
  | 'hard_block'; // Constitutional prohibition

/**
 * ConstitutionalConstraint - A rule that cannot be overridden.
 */
export interface ConstitutionalConstraint {
  /** Unique identifier */
  id: string;

  /** Constraint type */
  type: ConstitutionalConstraintType;

  /** Human-readable description */
  description: string;

  /** What governance layer this applies to */
  governanceLayer: GovernanceLayer;

  /** How strictly enforced */
  enforcement: EnforcementLevel;

  /** The rule itself (structured) */
  rule: {
    condition: string;
    action: string;
    parameters?: Record<string, unknown>;
  };

  /** Constitutional basis (reference to founding document) */
  constitutionalBasis: string;

  /** Is this constraint active? */
  isActive: boolean;
}

// ============================================================================
// UTILITY TYPES
// ============================================================================

/**
 * Timestamps - Common timestamp fields.
 */
export interface Timestamps {
  createdAt: string;
  updatedAt: string;
}

/**
 * Identifiable - Common identity fields.
 */
export interface Identifiable {
  id: string;
  did?: string;
}

/**
 * Reachable - Entities with reach settings.
 */
export interface Reachable {
  reach: ReachLevel;
}

/**
 * Governable - Entities within governance scope.
 */
export interface Governable {
  governanceLayer?: GovernanceLayer;
}

/**
 * Attestable - Entities that can have attestations.
 */
export interface Attestable {
  attestationIds?: string[];
}

// ============================================================================
// PROTOCOL VERSION
// ============================================================================

/**
 * Protocol version for compatibility checking.
 */
export const PROTOCOL_VERSION = '0.1.0';

/**
 * Protocol metadata.
 */
export const PROTOCOL_META = {
  version: PROTOCOL_VERSION,
  name: 'Elohim Protocol',
  pillars: ['imago_dei', 'lamad', 'qahal', 'shefa'] as Pillar[],
  description: 'Constitutional governance for human flourishing',
};
