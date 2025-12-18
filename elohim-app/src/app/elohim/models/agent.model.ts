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

  /**
   * Content mastery tracking (Bloom's Taxonomy progression).
   *
   * Tracks mastery level per content node (by resourceId) across all paths.
   * This enables:
   * - Cross-path mastery views (if you're "apply" level in path A, shows in path B)
   * - UI indicators: seen → practiced → applied → mastered
   * - Attestation gating (require "apply" level for certain privileges)
   *
   * Storage: Stored as Record<resourceId, MasteryLevel>
   * Special pathId '__global__' is used to store this field for cross-path tracking.
   */
  contentMastery?: Record<string, MasteryLevel>;
}

/**
 * Simplified mastery tier for UI display.
 * Maps Bloom's levels to user-friendly tiers.
 */
export type MasteryTier = 'unseen' | 'seen' | 'practiced' | 'applied' | 'mastered';

/**
 * Map MasteryLevel to simplified MasteryTier for UI.
 */
export function getMasteryTier(level: MasteryLevel): MasteryTier {
  switch (level) {
    case 'not_started':
      return 'unseen';
    case 'seen':
      return 'seen';
    case 'remember':
    case 'understand':
      return 'practiced';
    case 'apply':
      return 'applied';
    case 'analyze':
    case 'evaluate':
    case 'create':
      return 'mastered';
    default:
      return 'unseen';
  }
}

/**
 * Get numeric progress percentage for a mastery level.
 * Useful for progress bars.
 */
export function getMasteryProgress(level: MasteryLevel): number {
  const values: Record<MasteryLevel, number> = {
    not_started: 0,
    seen: 15,
    remember: 30,
    understand: 50,
    apply: 70,
    analyze: 85,
    evaluate: 95,
    create: 100,
  };
  return values[level] ?? 0;
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
 * MasteryLevel - Content mastery based on Bloom's Taxonomy.
 *
 * Progression from passive consumption to active contribution.
 * The 'apply' level is the attestation gate - crossing it unlocks
 * participation privileges in the content's governance.
 *
 * Levels:
 * - not_started (0): No engagement
 * - seen (1): Content viewed
 * - remember (2): Basic recall demonstrated (identify, list, name)
 * - understand (3): Comprehension demonstrated (explain, summarize)
 * - apply (4): Application in novel contexts (ATTESTATION GATE)
 * - analyze (5): Can break down, connect, contribute analysis
 * - evaluate (6): Can assess, critique, peer review
 * - create (7): Can author, derive, synthesize
 *
 * Reference: Anderson & Krathwohl (2001), Bloom's Revised Taxonomy
 */
export type MasteryLevel =
  | 'not_started' // 0 - No engagement
  | 'seen' // 1 - Content viewed
  | 'remember' // 2 - Basic recall demonstrated
  | 'understand' // 3 - Comprehension demonstrated
  | 'apply' // 4 - Application in novel contexts (ATTESTATION GATE)
  | 'analyze' // 5 - Can break down, connect, contribute analysis
  | 'evaluate' // 6 - Can assess, critique, peer review
  | 'create'; // 7 - Can author, derive, synthesize

/**
 * Numeric value for MasteryLevel for comparison and persistence.
 */
export const MASTERY_LEVEL_VALUES: Record<MasteryLevel, number> = {
  not_started: 0,
  seen: 1,
  remember: 2,
  understand: 3,
  apply: 4,
  analyze: 5,
  evaluate: 6,
  create: 7,
};

/**
 * @deprecated Use MASTERY_LEVEL_VALUES instead.
 */
export const BLOOM_LEVEL_VALUES = MASTERY_LEVEL_VALUES;

/**
 * The level at which participation privileges unlock.
 * Below this: passive learning (practice anything, no contribution privileges)
 * At/above this: active participation (comment, review, create)
 */
export const ATTESTATION_GATE_LEVEL: MasteryLevel = 'apply';

/**
 * Check if a mastery level is at or above the attestation gate.
 */
export function isAboveGate(level: MasteryLevel): boolean {
  return MASTERY_LEVEL_VALUES[level] >= MASTERY_LEVEL_VALUES[ATTESTATION_GATE_LEVEL];
}

/**
 * Compare two mastery levels.
 * Returns negative if a < b, zero if equal, positive if a > b.
 */
export function compareMasteryLevels(
  a: MasteryLevel,
  b: MasteryLevel
): number {
  return MASTERY_LEVEL_VALUES[a] - MASTERY_LEVEL_VALUES[b];
}


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
 * AgentAttestation - Clearer alias for attestations about agents.
 * Distinct from ContentAttestation (trust claims about content).
 */
export type AgentAttestation = NewAttestation;

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
