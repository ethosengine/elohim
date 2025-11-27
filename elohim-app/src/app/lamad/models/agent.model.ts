/**
 * Agent - A traveler in the system (human, org, or AI).
 *
 * Holochain mapping:
 * - Entry type: "agent_profile"
 * - id becomes AgentPubKey
 * - Profile data published to DHT if visibility allows
 */
export interface Agent {
  id: string;  // AgentPubKey in Holochain
  displayName: string;
  type: 'human' | 'organization' | 'ai-agent';

  bio?: string;
  avatar?: string;

  // Profile visibility
  visibility: 'public' | 'connections' | 'private';

  // Timestamps
  createdAt: string;
  updatedAt: string;
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
 */
export type MasteryLevel =
  | 'none'
  | 'impression'
  | 'practiced'
  | 'level-1'
  | 'level-2'
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
