/**
 * Agent - A traveler in the system (human, org, AI, or Elohim).
 *
 * Holochain mapping:
 * - Entry type: "agent_profile"
 * - id becomes AgentPubKey
 * - Profile data published to DHT if visibility allows
 *
 * W3C DECENTRALIZED IDENTIFIERS (DID) ALIGNMENT:
 * The `id` field remains human-friendly for routing and file paths.
 * The optional `did` field provides cryptographic identity.
 *
 * Separation rationale:
 * - id: Human-friendly routing ("user123", "alice-smith")
 * - did: Cryptographic identity ("did:web:elohim.host:agents:user123")
 *
 * This separation prevents:
 * - Breaking URLs (/lamad/resource/:id uses clean IDs)
 * - Breaking file paths (content/{id}.json on Windows doesn't allow colons)
 * - Ugly user-facing identifiers in localStorage keys
 *
 * Migration path for `did` field:
 * 1. Current: Optional, not yet populated
 * 2. Phase 2: did:web:elohim.host:agents:{id}
 * 3. Holochain: did:holochain:{AgentPubKey}
 *
 * W3C DID Spec: https://www.w3.org/TR/did-core/
 * DID Format: did:<method>:<method-specific-id>
 *
 * For Elohim agents, additional properties are available.
 */
export interface Agent {
  id: string;  // Future: DID or AgentPubKey in Holochain
  displayName: string;
  type: 'human' | 'organization' | 'ai-agent' | 'elohim';

  bio?: string;
  avatar?: string;

  // Profile visibility
  visibility: 'public' | 'connections' | 'private';

  // Timestamps
  createdAt: string;
  updatedAt: string;

  /**
   * ActivityPub Actor type for federated social web.
   *
   * Maps to ActivityStreams vocabulary:
   * - type='human' → 'Person'
   * - type='organization' → 'Organization'
   * - type='ai-agent' → 'Service'
   * - type='elohim' → 'Service' (with constitutional binding metadata)
   *
   * Reference: https://www.w3.org/TR/activitystreams-vocabulary/#actor-types
   */
  activityPubType?: 'Person' | 'Organization' | 'Service' | 'Application' | 'Group';

  /**
   * Decentralized Identifier (DID) for cryptographic identity.
   *
   * Separate from `id` to maintain human-friendly URLs and filenames.
   * The `id` field remains the primary routing identifier.
   *
   * Example: "did:web:elohim.host:agents:user123"
   *
   * Reference: https://www.w3.org/TR/did-core/
   */
  did?: string;

  // Elohim-specific properties (optional, only present for type: 'elohim')
  layer?: string;
  capabilities?: string[];
  attestations?: string[];
  familyId?: string;
}

/**
 * AgentProgress - Private progress on a specific path.
 *
 * Holochain mapping:
 * - Entry type: "agent_progress"
 * - Lives on agent's private source chain
 * - NOT published to DHT unless explicitly shared
 *
 * Relationship to Mastery (Phase 6):
 * - AgentProgress tracks path-specific navigation and engagement
 * - Mastery (Phase 6) tracks concept-level competency across all paths
 */
export interface AgentProgress {
  agentId: string;
  pathId: string;

  // Progress state
  currentStepIndex: number;
  completedStepIndices: number[];

  // Timing
  startedAt: string;
  lastActivityAt: string;
  completedAt?: string;

  // Affinity tracking - engagement depth with content (0.0 to 1.0)
  stepAffinity: Record<number, number>;

  // Personal artifacts
  stepNotes: Record<number, string>;
  reflectionResponses: Record<number, string[]>;

  // Achievement attestations earned through this path
  attestationsEarned: string[];

  /**
   * Global content completion tracking (Khan Academy-style shared completion)
   *
   * Tracks which content nodes (by resourceId) have been completed across ALL paths.
   * This enables cross-path completion views:
   * - If content X is completed in "1st Grade Math", it shows as completed in "Early Math Review"
   * - Path completion calculated by unique content mastered, not just step indices
   *
   * Storage note: Stored as array in JSON, converted to Set in services for O(1) lookup.
   * Typical learner completes <500 content nodes = <10KB storage, well within localStorage limits.
   *
   * Special pathId '__global__' is used to store this field for cross-path tracking.
   */
  completedContentIds?: string[];
}

/**
 * AttestationCategory - Types of attestations
 */
export type AttestationCategory =
  | 'domain-mastery'    // Earned via sustained concept mastery
  | 'path-completion'   // All concepts at impression+ level
  | 'role-credential'   // Granted by governance process
  | 'achievement';      // One-time participation recognition

/**
 * MasteryLevel - For Phase 6 mastery system
 * Values track learning progression from initial exposure to mastery.
 */
export type MasteryLevel =
  | 'not-started'
  | 'struggling'
  | 'learning'
  | 'practicing'
  | 'mastered';

/**
 * NewAttestation - Refined attestation model for v2
 *
 * Attestations are PERMANENT ACHIEVEMENTS, not competency tracking.
 * Mastery (Phase 6) handles graduated competency.
 */
export interface NewAttestation {
  id: string;
  agentId: string;

  // What kind of attestation is this?
  category: AttestationCategory;
  attestationType: string;        // Specific type within category

  // How it was earned (depends on category)
  earnedVia: {
    // For domain-mastery: which concepts contributed
    conceptIds?: string[];
    requiredLevel?: MasteryLevel;

    // For path-completion: which path
    pathId?: string;

    // For role credentials: who granted it
    grantedBy?: string;
    governanceProcess?: string;

    // For achievements: what triggered it
    achievementTrigger?: string;

    // Legacy/generic
    assessmentId?: string;
    manualGrant?: string;
  };

  // Verification
  issuedAt: string;
  issuedBy: string;               // System, steward, governance, or peer
  expiresAt?: string;             // Optional expiration (for role credentials)
  proof?: string;                 // Cryptographic signature in production

  // Display metadata
  displayName: string;
  description: string;
  iconUrl?: string;
  tier?: 'bronze' | 'silver' | 'gold' | 'platinum';  // Visual distinction
}

/**
 * FrontierItem - An item on the learning frontier (what's next?)
 */
export interface FrontierItem {
  pathId: string;
  pathTitle?: string;
  nextStepIndex: number;
  stepTitle?: string;
  lastActivity: string;
}
