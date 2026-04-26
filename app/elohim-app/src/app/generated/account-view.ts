/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/account-view.schema.json -- DO NOT EDIT */

/**
 * Visibility of this human's profile
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
 * Reach level corresponding to intimacy (private → commons)
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
 * Content reach/visibility level. Ordered from most restrictive to most open. Source of truth: DNA-notarized CORE_REACH_LEVELS constant in content_store_integrity zome. Category A — enumeration values are part of the protocol vocabulary enforced by gateways without parsing payload.
 */
export type Reach2 =
  | 'private'
  | 'self'
  | 'intimate'
  | 'trusted'
  | 'familiar'
  | 'community'
  | 'public'
  | 'commons';
/**
 * M5 ships only 'trusted'
 */
export type Reach3 =
  | 'private'
  | 'self'
  | 'intimate'
  | 'trusted'
  | 'familiar'
  | 'community'
  | 'public'
  | 'commons';

/**
 * Source of truth: aggregate (DHT-derived projections). Composes Human + KeyRotation + KeyRevocation + RecoveryRequest + HumanRelationship + PortalHost views into one snapshot for the account-management surface.
 */
export interface AccountView {
  human: HumanView;
  activeKeyRotation?: KeyRotationView | null;
  recentRevocations: KeyRevocationView[];
  pendingRecoveryRequests: RecoveryRequestView[];
  emergencyContacts: HumanRelationshipView[];
  portalHosts: PortalHostView[];
  /**
   * Derived from peer_identity_bindings projection
   */
  isSteward: boolean;
  /**
   * Derived
   */
  hasLocalConductor: boolean;
}
/**
 * Source of truth: DHT (Notarized, Category A). Projection of the imagodei Human DHT entry. Rebuildable via signal replay on HumanCreated/HumanUpdated signals. dhtAnchorHash is null for pre-coherence rows (seeded before DHT was live).
 */
export interface HumanView {
  /**
   * Stable human identifier (UUID or agent-derived)
   */
  id: string;
  /**
   * Holochain AgentPubKey (base64url). None before coherence or for custodial-only rows.
   */
  agentPubKey?: string | null;
  displayName: string;
  bio?: string | null;
  /**
   * Tag-style affinity labels (parsed from JSON text in DB)
   */
  affinities: string[];
  profileReach: Reach;
  location?: string | null;
  profilePhotoUrl?: string | null;
  /**
   * Holochain hApp instance identifier
   */
  hAppId: string;
  createdAt: string;
  updatedAt: string;
  /**
   * ActionHash (base64url) of the Human entry in imagodei DNA. None for pre-coherence rows.
   */
  dhtAnchorHash?: string | null;
}
/**
 * Projection of imagodei KeyRotation DHT entry with RecoveryAuthority enum. Source of truth: DHT. The authoritative claim that a human's agent key has rotated. Authority evidence carried as variant_kind + JSON-encoded variant fields.
 */
export interface KeyRotationView {
  dhtAnchorHash: string;
  humanAgentPubkey: string;
  newAgentPubkey: string;
  supersededAgentPubkey: string;
  recoveryRequestHash: string;
  authorityKind:
    | 'intimateQuorum'
    | 'communityConsensus'
    | 'governanceAct'
    | 'networkWitness'
    | 'cryptographicQuorum';
  authorityJson: string;
  rotatedAt: string;
}
/**
 * Read-optimized projection of an imagodei KeyRevocation DHT entry. Source of truth: DHT. Rebuildable via signal replay on RecoveryV2Signal::KeyRevocationRequested/Effective.
 */
export interface KeyRevocationView {
  /**
   * Holochain ActionHash of the KeyRevocation entry (base64).
   */
  dhtAnchorHash: string;
  /**
   * Coordinator-generated ID.
   */
  id: string;
  /**
   * String id of the human whose key is being revoked.
   */
  humanId: string;
  /**
   * The AgentPubKey being revoked (stringified).
   */
  revokedKey: string;
  /**
   * One of REVOCATION_REASONS: compromised, stolen, challenge_upheld, voluntary.
   */
  reason: string;
  /**
   * How the revocation was initiated.
   */
  triggerType: 'voluntary' | 'steward_vote' | 'challenge';
  /**
   * human_id of the initiating agent.
   */
  initiatedBy: string;
  /**
   * Votes required to reach threshold. 1 for voluntary; >=2 for steward_vote/challenge.
   */
  requiredVotes: number;
  /**
   * Approved votes counted so far.
   */
  currentVotes: number;
  /**
   * True when currentVotes >= requiredVotes and the revocation is effective.
   */
  thresholdReached: boolean;
  /**
   * ISO-8601 timestamp when revocation became effective. Null while pending.
   */
  effectiveAt?: string | null;
  /**
   * ISO-8601 timestamp of initial commit.
   */
  createdAt: string;
  /**
   * ISO-8601 timestamp of last update (e.g., when threshold was reached).
   */
  updatedAt: string;
}
/**
 * Projection of imagodei RecoveryRequest DHT entry (modernized). Source of truth: DHT.
 */
export interface RecoveryRequestView {
  dhtAnchorHash: string;
  humanAgentPubkey: string;
  newAgentPubkey: string;
  hostingDoorwayPubkey: string;
  proposedAuthorityKind:
    | 'intimateQuorum'
    | 'communityConsensus'
    | 'governanceAct'
    | 'networkWitness'
    | 'cryptographicQuorum';
  proposedAuthorityJson: string;
  /**
   * @minItems 16
   * @maxItems 16
   */
  requestNonce: [
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
  ];
  /**
   * Coordinator-populated resolution of humanAgentPubkey to the legacy String human_id used by HumanityWitness/HumanRelationship/IdentityFreeze. Null accepted at request commit; required by the time a KeyRotation is attempted for IntimateQuorum rotations or when a freeze may apply.
   */
  humanId?: string | null;
  /**
   * Coordinator-computed threshold for IntimateQuorum authority. Computed as ceil(emergency_contact_count / 2) + 1 at request time, floored at 2. Validator enforces distinct-witness-author count >= this value at KeyRotation commit.
   */
  requiredWitnessCount: number;
  createdAt: string;
}
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
  intimacyLevel: Reach1;
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
  reach: Reach2;
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
/**
 * Source of truth: DHT (Notarized, Category A). PortalHost declares URLs authorized to render this human's auth portal. Anchored on the Human entry's ActionHash so portal hosts survive KeyRotation. M5 ships only reach=trusted.
 */
export interface PortalHostView {
  /**
   * ActionHash (base64url) of the Human entry this PortalHost anchors on
   */
  humanId: string;
  hostUrl: string;
  label?: string | null;
  addedAt: string;
  /**
   * Operational enrichment from libp2p; not part of notarized entry
   */
  lastReachableAt?: string | null;
  reach: Reach3;
  /**
   * ActionHash (base64url) of the PortalHost entry; canonical PK
   */
  dhtAnchorHash: string;
}
