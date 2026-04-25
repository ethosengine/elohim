/* Generated from protocol schema: views/human-relationship.schema.json -- DO NOT EDIT */

/**
 * Reach level corresponding to intimacy (private → commons)
 */
export type Reach =
  | 'private'
  | 'self'
  | 'intimate'
  | 'trusted'
  | 'familiar'
  | 'community'
  | 'public'
  | 'commons';
/**
 * Content reach/visibility level. Ordered from most restrictive to most open. Source of truth: DNA-notarized CORE_REACH_LEVELS constant in content_store_integrity zome. Category A — enumeration values are part of the protocol vocabulary enforced by gateways without parsing payload.
 */
export type Reach1 =
  | 'private'
  | 'self'
  | 'intimate'
  | 'trusted'
  | 'familiar'
  | 'community'
  | 'public'
  | 'commons';

/**
 * Source of truth: DHT (Notarized, Category A). Projection of the imagodei HumanRelationship DHT entry. Rebuildable via signal replay on RelationshipCreated signals. dhtAnchorHash is null for pre-coherence rows.
 */
export interface HumanRelationshipView {
  /**
   * Coordinator-generated stable relationship identifier
   */
  id: string;
  /**
   * Holochain hApp instance identifier
   */
  hAppId: string;
  /**
   * human_id of party A (initiating side)
   */
  partyAId: string;
  /**
   * human_id of party B
   */
  partyBId: string;
  /**
   * One of the RelationshipType enum values (e.g. family, friendship, stewardship)
   */
  relationshipType: string;
  intimacyLevel: Reach;
  isBidirectional: boolean;
  consentGivenByA: boolean;
  consentGivenByB: boolean;
  custodyEnabledByA: boolean;
  custodyEnabledByB: boolean;
  autoCustodyEnabled: boolean;
  emergencyAccessEnabled: boolean;
  /**
   * human_id of the relationship initiator
   */
  initiatedBy: string;
  verifiedAt?: string | null;
  /**
   * Governance scope label (e.g. household, qahal). None = no governance layer applied.
   */
  governanceLayer?: string | null;
  reach: Reach1;
  /**
   * Parsed context object (was context_json in storage). Schema is relationship-type-specific.
   */
  context?: Record<string, unknown> | null;
  createdAt: string;
  updatedAt: string;
  expiresAt?: string | null;
  /**
   * ActionHash (base64url) of the HumanRelationship entry in imagodei DNA. None for pre-coherence rows.
   */
  dhtAnchorHash?: string | null;
}
