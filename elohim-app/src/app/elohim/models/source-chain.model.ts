/**
 * Source Chain Model - Agent-centric data structures for Holochain compatibility.
 *
 * Philosophy:
 * - Model data as if it's already on Holochain, even when using localStorage
 * - Each entry is authored by an agent and immutable once created
 * - Relationships are explicit links, not embedded foreign keys
 * - Corrections create new entries, they don't update existing ones
 *
 * This allows seamless migration from localStorage → Holochain:
 * - Data model stays the same
 * - Only the persistence layer changes
 * - Links and entries map directly to Holochain concepts
 *
 * Holochain Concepts Mirrored:
 * - SourceChainEntry → Entry (with header metadata)
 * - EntryLink → Link (base → target with type)
 * - AgentId → AgentPubKey
 * - EntryHash → ActionHash/EntryHash
 */

// =============================================================================
// ENTRY TYPES
// =============================================================================

/**
 * Base entry type - mirrors Holochain's Entry concept.
 *
 * In Holochain:
 * - Each entry has a header (author, timestamp, prev_action)
 * - Entry content is hashed to create EntryHash
 * - Entries are immutable once committed
 *
 * In localStorage MVP:
 * - entryHash is a generated UUID
 * - authorAgent is the sessionId
 * - prevEntryHash maintains chain ordering
 */
export interface SourceChainEntry<T = unknown> {
  /**
   * Unique identifier for this entry.
   * In Holochain: computed hash of entry content
   * In localStorage: generated UUID
   */
  entryHash: string;

  /**
   * The agent who authored this entry.
   * In Holochain: AgentPubKey
   * In localStorage: sessionId
   */
  authorAgent: string;

  /**
   * Entry type discriminator (e.g., 'human-profile', 'mastery-record', 'recognition-event').
   * In Holochain: maps to entry_def_index
   */
  entryType: string;

  /**
   * The actual entry content.
   * Structure depends on entryType.
   */
  content: T;

  /**
   * When this entry was created (ISO 8601).
   * In Holochain: header timestamp
   */
  timestamp: string;

  /**
   * Hash of the previous entry in this agent's chain.
   * Maintains ordering and chain integrity.
   * In Holochain: prev_action in header
   */
  prevEntryHash?: string;

  /**
   * Sequence number in agent's chain (0-indexed).
   * Derived from chain position.
   */
  sequence?: number;
}

/**
 * Entry types used in Lamad.
 * Each type maps to a specific content structure.
 */
export type LamadEntryType =
  // Identity entries
  | 'human-profile' // SessionHuman / HumanProfile data
  | 'presence-claim' // ContributorPresence claim record
  | 'external-identifier' // Link to external identity (Twitter, GitHub, etc.)

  // Learning entries
  | 'mastery-record' // ContentMastery for a specific content node
  | 'affinity-mark' // Affinity value for a content node
  | 'path-progress' // Progress through a learning path
  | 'step-completion' // Completion of a specific step
  | 'assessment-result' // Quiz/assessment result

  // Recognition entries
  | 'recognition-event' // Recognition flowing to a presence
  | 'endorsement' // Endorsement of content or person
  | 'attestation-grant' // Attestation earned

  // Governance entries
  | 'governance-vote' // Vote on a proposal
  | 'challenge-filed' // Challenge to content
  | 'review-submitted' // Review of content

  // Consent & Relationship entries
  | 'human-consent' // Consent relationship between two humans
  | 'consent-state-change' // Record of consent state transition
  | 'path-negotiation' // Love map negotiation session
  | 'negotiation-message'; // Message in a negotiation

// =============================================================================
// LINK TYPES
// =============================================================================

/**
 * Link between entries - mirrors Holochain's Link concept.
 *
 * In Holochain:
 * - Links are stored separately from entries
 * - Links have a base, target, type, and optional tag
 * - Links can be deleted (but deletions are visible)
 *
 * In localStorage MVP:
 * - Links stored in a separate collection
 * - linkHash is generated UUID
 */
export interface EntryLink {
  /**
   * Unique identifier for this link.
   */
  linkHash: string;

  /**
   * Entry hash of the source/base entry.
   */
  baseHash: string;

  /**
   * Entry hash of the target entry.
   */
  targetHash: string;

  /**
   * Link type (e.g., 'mastery-for-content', 'recognition-to-presence').
   */
  linkType: string;

  /**
   * Optional tag for additional link metadata.
   * In Holochain: serialized bytes, often used for filtering
   */
  tag?: string;

  /**
   * When this link was created.
   */
  timestamp: string;

  /**
   * The agent who created this link.
   */
  authorAgent: string;

  /**
   * If true, this link has been "deleted" (soft delete).
   * In Holochain, deletions create new actions; the link remains visible.
   */
  deleted?: boolean;

  /**
   * When the link was deleted (if deleted).
   */
  deletedAt?: string;
}

/**
 * Link types used in Lamad.
 */
export type LamadLinkType =
  // Identity links
  | 'profile-for-agent' // Agent → their profile entry
  | 'presence-for-content' // Content → attributed presence(s)
  | 'identity-claim' // Presence → claim verification entry

  // Learning links
  | 'mastery-for-content' // Content → mastery record(s) by this agent
  | 'affinity-for-content' // Content → affinity mark by this agent
  | 'progress-for-path' // Path → progress record by this agent
  | 'completion-for-step' // Step → completion record

  // Recognition links
  | 'recognition-to-presence' // Recognition event → target presence
  | 'recognition-from-agent' // Recognition event → source agent
  | 'endorsement-for-content' // Content → endorsement

  // Correction links
  | 'correction-of' // New entry → corrected entry
  | 'supersedes' // New entry → superseded entry

  // Consent & Relationship links
  | 'consent-with-human' // Human → consent relationship with another human
  | 'consent-from-human' // Human → consent relationship FROM another human
  | 'negotiation-for-consent' // Consent → negotiation session
  | 'path-from-negotiation'; // Negotiation → generated path

// =============================================================================
// QUERY TYPES
// =============================================================================

/**
 * Query parameters for fetching entries.
 */
export interface EntryQuery {
  /** Filter by entry type */
  entryType?: LamadEntryType;

  /** Filter by author agent */
  authorAgent?: string;

  /** Filter by timestamp range */
  after?: string;
  before?: string;

  /** Limit number of results */
  limit?: number;

  /** Skip entries (for pagination) */
  offset?: number;

  /** Sort order */
  order?: 'asc' | 'desc';
}

/**
 * Query parameters for fetching links.
 */
export interface LinkQuery {
  /** Find links from this base entry */
  baseHash?: string;

  /** Find links to this target entry */
  targetHash?: string;

  /** Filter by link type */
  linkType?: LamadLinkType;

  /** Filter by author */
  authorAgent?: string;

  /** Include deleted links */
  includeDeleted?: boolean;

  /** Filter by tag */
  tag?: string;
}

// =============================================================================
// CHAIN METADATA
// =============================================================================

/**
 * Metadata about an agent's source chain.
 */
export interface ChainMetadata {
  /** Agent ID */
  agentId: string;

  /** Hash of the latest entry in the chain */
  headHash: string;

  /** Total number of entries in the chain */
  entryCount: number;

  /** Total number of links created by this agent */
  linkCount: number;

  /** When the chain was created */
  createdAt: string;

  /** When the chain was last updated */
  updatedAt: string;
}

// =============================================================================
// CONTENT TYPES FOR ENTRIES
// =============================================================================

/**
 * Content structure for 'human-profile' entry.
 */
export interface HumanProfileContent {
  displayName: string;
  avatarUrl?: string;
  bio?: string;
  locale?: string;
  interests?: string[];
  isAnonymous: boolean;
  accessLevel: 'visitor' | 'member' | 'attested';
}

/**
 * Content structure for 'mastery-record' entry.
 */
export interface MasteryRecordContent {
  contentId: string;
  level: string; // MasteryLevel (Bloom's Taxonomy: not_started → create)
  levelAchievedAt: string;
  freshness: number;
  lastEngagementAt: string;
  lastEngagementType: string;
}

/**
 * Content structure for 'affinity-mark' entry.
 */
export interface AffinityMarkContent {
  contentId: string;
  value: number; // 0.0 - 1.0
  previousValue?: number;
}

/**
 * Content structure for 'recognition-event' entry.
 */
export interface RecognitionEventContent {
  presenceId: string;
  recognitionType: 'view' | 'affinity' | 'citation' | 'endorsement' | 'stewardship';
  quantity: number;
  sourceContentId?: string;
}

/**
 * Content structure for 'presence-claim' entry.
 */
export interface PresenceClaimContent {
  presenceId: string;
  verificationMethod: string;
  verificationEvidence?: string;
  claimedAt: string;
}

// =============================================================================
// MIGRATION TYPES
// =============================================================================

/**
 * Package for migrating local chain to Holochain.
 */
export interface ChainMigrationPackage {
  /** Source agent ID (localStorage sessionId) */
  sourceAgentId: string;

  /** Target agent ID (Holochain AgentPubKey) */
  targetAgentId?: string;

  /** All entries to migrate */
  entries: SourceChainEntry[];

  /** All links to migrate */
  links: EntryLink[];

  /** Chain metadata */
  metadata: ChainMetadata;

  /** Migration timestamp */
  preparedAt: string;

  /** Migration status */
  status: 'pending' | 'in-progress' | 'completed' | 'failed';
}

// =============================================================================
// CONSENT & RELATIONSHIP CONTENT TYPES
// =============================================================================

/**
 * Content structure for 'human-consent' entry.
 */
export interface HumanConsentContent {
  /** Consent relationship ID */
  consentId: string;

  /** Human who initiated the relationship */
  initiatorId: string;

  /** Human who received the request */
  participantId: string;

  /** Current intimacy level */
  intimacyLevel: 'recognition' | 'connection' | 'trusted' | 'intimate';

  /** State of consent */
  consentState: 'not_required' | 'pending' | 'accepted' | 'declined' | 'revoked' | 'expired';

  /** When consent state was last updated */
  updatedAt: string;

  /** When consent was given (if accepted) */
  consentedAt?: string;

  /** Request message */
  requestMessage?: string;

  /** Response message */
  responseMessage?: string;

  /** Attestation IDs validating this relationship */
  validatingAttestationIds?: string[];

  /** Required attestation type */
  requiredAttestationType?: string;
}

/**
 * Content structure for 'path-negotiation' entry.
 */
export interface PathNegotiationContent {
  /** Negotiation ID */
  negotiationId: string;

  /** Initiator human ID */
  initiatorId: string;

  /** Participant human ID */
  participantId: string;

  /** Negotiation status */
  status: 'proposed' | 'analyzing' | 'negotiating' | 'accepted' | 'declined' | 'failed' | 'expired';

  /** Consent ID authorizing this negotiation */
  consentId: string;

  /** Shared affinity node IDs */
  sharedAffinityNodes: string[];

  /** Bridging strategy */
  bridgingStrategy?:
    | 'shortest_path'
    | 'maximum_overlap'
    | 'complementary'
    | 'exploration'
    | 'custom';

  /** Generated path ID */
  generatedPathId?: string;

  /** When negotiation was resolved */
  resolvedAt?: string;
}

/**
 * Content structure for 'negotiation-message' entry.
 */
export interface NegotiationMessageContent {
  /** Negotiation ID this message belongs to */
  negotiationId: string;

  /** Message author ID */
  authorId: string;

  /** Message type */
  type: 'proposal' | 'counter' | 'accept' | 'decline' | 'question' | 'comment' | 'system' | 'agent';

  /** Message content */
  content: string;

  /** Structured metadata */
  metadata?: Record<string, unknown>;
}
