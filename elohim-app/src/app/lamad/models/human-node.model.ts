/**
 * HumanNode - Humans as graph objects for relationship networks.
 *
 * Part of the Qahal (קהל) pillar of the Elohim Protocol.
 * Qahal provides the human relationship layer - who you're with.
 *
 * Evolution of Understanding:
 * Initially, we said "humans don't become nodes in the knowledge graph."
 * This was correct for CONTENT - humans aren't things you learn ABOUT.
 * But humans DO need graph representation for RELATIONSHIPS:
 * - Spouse, family, neighbors, coworkers, congregation
 * - Same infrastructure as content (edges, reach, intimacy levels)
 * - But with agency, consent requirements, bidirectional relationships
 *
 * Two Graph Layers:
 * 1. Content Graph (Lamad): What you learn about (ContentNodes)
 * 2. Human Graph (Qahal): Who you learn with/from (HumanNodes)
 *
 * The same governance layers apply:
 * - Family layer → spouse, children, parents
 * - Neighborhood layer → neighbors, local community
 * - Workplace layer → coworkers, managers, employees
 * - Affinity layer → congregation, interest groups, networks
 *
 * Reach is mediated for humans too - you can discover new connections
 * but intimacy requires explicit consent.
 *
 * Protocol Core Integration:
 * - Uses ReachLevel from protocol-core (mapped to HumanReach)
 * - Uses IntimacyLevel from protocol-core
 * - Uses GovernanceLayer from protocol-core
 * - Uses ConsentState from protocol-core
 * - Uses GeographicContext from protocol-core
 *
 * Holochain mapping:
 * - Entry type: "human_node"
 * - AgentPubKey as primary identifier
 * - Private source chain for sovereign data
 * - DHT for public profile if shared
 */

import {
  type IntimacyLevel,
  type ConsentState,
  type GovernanceLayer,
  type ReachLevel,
  type GeographicContext,
} from '@app/elohim/models/protocol-core.model';

// @coverage: 22.2% (2026-01-31)

// Re-export core types for convenience
export type { IntimacyLevel, ConsentState, GovernanceLayer, ReachLevel, GeographicContext };

// =========================================================================
// Relationship Types
// =========================================================================

/**
 * RelationshipType - Semantic categorization of human-to-human relationships.
 *
 * These map to governance layers and determine default visibility/capabilities.
 * Multiple relationship types can exist between the same two humans.
 */
export type RelationshipType =
  // Family Layer
  | 'spouse'
  | 'parent'
  | 'child'
  | 'sibling'
  | 'grandparent'
  | 'grandchild'
  | 'extended_family'
  | 'guardian'

  // Neighborhood/Community Layer
  | 'neighbor'
  | 'community_member'
  | 'local_friend'

  // Workplace Layer
  | 'coworker'
  | 'manager'
  | 'direct_report'
  | 'mentor'
  | 'mentee'
  | 'business_partner'

  // Affinity/Network Layer
  | 'congregation_member'
  | 'interest_group_member'
  | 'learning_partner'
  | 'network_connection'

  // General
  | 'friend'
  | 'acquaintance'
  | 'other';

/**
 * Relationship type to governance layer mapping.
 * Uses GovernanceLayer from protocol-core.
 */
export const RELATIONSHIP_LAYER_MAP: Record<RelationshipType, GovernanceLayer> = {
  // Family (household/extended_family layers)
  spouse: 'household',
  parent: 'household',
  child: 'household',
  sibling: 'household',
  grandparent: 'extended_family',
  grandchild: 'extended_family',
  extended_family: 'extended_family',
  guardian: 'household',

  // Community (neighborhood/municipality layers)
  neighbor: 'neighborhood',
  community_member: 'municipality',
  local_friend: 'neighborhood',

  // Workplace
  coworker: 'workplace',
  manager: 'workplace',
  direct_report: 'workplace',
  mentor: 'workplace',
  mentee: 'workplace',
  business_partner: 'workplace',

  // Affinity
  congregation_member: 'faith_community',
  interest_group_member: 'affinity_network',
  learning_partner: 'affinity_network',
  network_connection: 'affinity_network',

  // General (defaults to municipality)
  friend: 'affinity_network',
  acquaintance: 'municipality',
  other: 'municipality',
};

/**
 * Default intimacy level for relationship types.
 */
export const RELATIONSHIP_DEFAULT_INTIMACY: Record<RelationshipType, IntimacyLevel> = {
  // Family - typically intimate or trusted
  spouse: 'intimate',
  parent: 'intimate',
  child: 'intimate',
  sibling: 'trusted',
  grandparent: 'trusted',
  grandchild: 'trusted',
  extended_family: 'connection',
  guardian: 'intimate',

  // Community - typically connection
  neighbor: 'connection',
  community_member: 'connection',
  local_friend: 'trusted',

  // Workplace - typically connection
  coworker: 'connection',
  manager: 'connection',
  direct_report: 'connection',
  mentor: 'trusted',
  mentee: 'trusted',
  business_partner: 'trusted',

  // Affinity - varies
  congregation_member: 'connection',
  interest_group_member: 'connection',
  learning_partner: 'trusted',
  network_connection: 'recognition',

  // General
  friend: 'trusted',
  acquaintance: 'recognition',
  other: 'recognition',
};

// =========================================================================
// Human Node
// =========================================================================

/**
 * HumanNode - A human in the relationship graph.
 *
 * This represents the "public profile" aspect of a human that can be
 * discovered by others based on reach. The full source chain data
 * remains sovereign to the human.
 */
export interface HumanNode {
  /** Unique identifier (AgentPubKey in Holochain) */
  id: string;

  /** Display name (can be pseudonymous) */
  displayName: string;

  /** Short bio/description */
  bio?: string;

  /** Avatar URL or data URI */
  avatarUrl?: string;

  /** Whether this profile is pseudonymous */
  isPseudonymous: boolean;

  /** Profile creation timestamp */
  createdAt: string;

  /** Last profile update */
  updatedAt: string;

  // =========================================================================
  // Reach & Discoverability
  // =========================================================================

  /**
   * Profile visibility - who can discover this human.
   * Similar to ContentReach but for humans.
   */
  profileReach: HumanReach;

  /**
   * Whether this human can receive connection requests.
   */
  acceptingConnections: boolean;

  /**
   * Connection request message (optional custom welcome).
   */
  connectionMessage?: string;

  // =========================================================================
  // Geographic Context
  // =========================================================================

  /**
   * Primary geographic context (where this human is based).
   * Used for neighborhood/community layer relationships.
   */
  primaryLocation?: GeographicContext;

  /**
   * Additional locations (work, extended family, etc.).
   */
  additionalLocations?: GeographicContext[];

  // =========================================================================
  // Group Memberships
  // =========================================================================

  /**
   * Organizations this human belongs to.
   * These become edges in the graph.
   */
  organizationIds?: string[];

  /**
   * Communities this human is part of.
   */
  communityIds?: string[];

  /**
   * Interest groups / affinity networks.
   */
  affinityGroupIds?: string[];

  // =========================================================================
  // Graph Connections
  // =========================================================================

  /**
   * Direct relationship IDs (edges to other humans).
   * Full relationship details fetched separately.
   */
  relationshipIds?: string[];

  /**
   * Count of relationships by type (for quick stats).
   */
  relationshipCounts?: Partial<Record<RelationshipType, number>>;

  /**
   * Total trusted connections (for reach calculation).
   */
  trustedConnectionCount?: number;

  // =========================================================================
  // Learning Context (Bridge to Content Graph)
  // =========================================================================

  /**
   * High-affinity content node IDs (top interests).
   * Shared for love map generation with consent.
   */
  publicAffinityNodeIds?: string[];

  /**
   * Attestation IDs earned by this human.
   * Public attestations visible based on reach.
   */
  publicAttestationIds?: string[];

  /**
   * Role attestations (capability badges).
   */
  roleAttestationIds?: string[];
}

// =========================================================================
// Human Reach
// =========================================================================

/**
 * HumanReach - Who can discover this human's profile.
 *
 * Similar to ContentReach but optimized for human discovery:
 * - 'hidden': Not discoverable (invite only)
 * - 'network': Discoverable by friends-of-friends
 * - 'community': Discoverable by community members
 * - 'public': Globally discoverable
 */
export type HumanReach =
  | 'hidden' // Not discoverable, invite only
  | 'network' // Friends-of-friends can discover
  | 'community' // Community members can discover
  | 'public'; // Anyone can discover

// =========================================================================
// Human Relationship Edge
// =========================================================================

/**
 * HumanRelationship - An edge in the human graph.
 *
 * This represents the relationship between two humans.
 * Unlike HumanConsent (which is about permission), this captures
 * the semantic nature and context of the relationship.
 */
export interface HumanRelationship {
  /** Unique identifier */
  id: string;

  /** Source human ID (who created this edge) */
  sourceHumanId: string;

  /** Target human ID (who this edge points to) */
  targetHumanId: string;

  /** Relationship type (semantic category) */
  type: RelationshipType;

  /** Additional relationship types (e.g., coworker AND friend) */
  additionalTypes?: RelationshipType[];

  /** Consent record ID governing this relationship */
  consentId: string;

  /** Current consent state (denormalized for quick access) */
  consentState: ConsentState;

  /** Current intimacy level (denormalized) */
  intimacyLevel: IntimacyLevel;

  /** When relationship was established */
  establishedAt: string;

  /** Last interaction timestamp */
  lastInteractionAt?: string;

  /** Custom label for this relationship (optional) */
  customLabel?: string;

  /** Notes about this relationship (private to source human) */
  privateNotes?: string;

  // =========================================================================
  // Context
  // =========================================================================

  /**
   * Organization context (if workplace relationship).
   */
  organizationId?: string;

  /**
   * Community context (if community relationship).
   */
  communityId?: string;

  /**
   * Affinity group context (if affinity relationship).
   */
  affinityGroupId?: string;

  /**
   * Geographic context (if location-based relationship).
   */
  geographicContext?: GeographicContext;

  // =========================================================================
  // Reciprocity
  // =========================================================================

  /**
   * Whether the target has reciprocated (created their own edge back).
   */
  isReciprocated: boolean;

  /**
   * The target's relationship edge ID (if reciprocated).
   */
  reciprocalRelationshipId?: string;
}

// =========================================================================
// Helper Functions
// =========================================================================

/**
 * Get the governance layer for a relationship type.
 */
export function getRelationshipLayer(type: RelationshipType): GovernanceLayer {
  return RELATIONSHIP_LAYER_MAP[type];
}

/**
 * Get the default intimacy level for a relationship type.
 */
export function getDefaultIntimacy(type: RelationshipType): IntimacyLevel {
  return RELATIONSHIP_DEFAULT_INTIMACY[type];
}

/**
 * Check if a relationship type is in the family layer (household or extended_family).
 */
export function isFamilyRelationship(type: RelationshipType): boolean {
  const layer = RELATIONSHIP_LAYER_MAP[type];
  return layer === 'household' || layer === 'extended_family';
}

/**
 * Check if a relationship type is in the workplace layer.
 */
export function isWorkplaceRelationship(type: RelationshipType): boolean {
  return RELATIONSHIP_LAYER_MAP[type] === 'workplace';
}

/**
 * Check if a relationship type typically requires high trust.
 */
export function isHighTrustRelationship(type: RelationshipType): boolean {
  const intimacy = RELATIONSHIP_DEFAULT_INTIMACY[type];
  return intimacy === 'intimate' || intimacy === 'trusted';
}
