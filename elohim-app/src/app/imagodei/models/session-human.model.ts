/**
 * SessionHuman - A temporary traveler identity for MVP.
 *
 * Philosophy:
 * - Everyone can explore immediately without friction (open content)
 * - Some content requires Holochain identity (gated content)
 * - Sensitive content requires identity + attestations + path completion
 * - Progress is stored locally in their browser
 * - When ready, they "upgrade" by installing the Holochain app
 * - Installation joins them to the p2p network with persistent identity
 *
 * Access Levels:
 * - 'visitor': Session human, can access open content only
 * - 'member': Holochain identity, can access gated content
 * - 'attested': Member with specific attestations, can access sensitive content
 *
 * Holochain migration path:
 * - Session: localStorage with generated sessionId
 * - Holochain: AgentPubKey from conductor, source chain storage
 *
 * The session human accumulates:
 * - Content affinity (what resonates with them)
 * - Path progress (where they are in learning journeys)
 * - Exploration history (what they've discovered)
 *
 * This data can be "claimed" when they install Holochain,
 * migrating their session progress to their permanent identity.
 */
export interface SessionHuman {
  // Temporary identity (generated UUID for session)
  sessionId: string;

  // Display name (optional, can be set later)
  displayName: string;

  // Session state
  isAnonymous: true;  // Always true for session humans

  // Access level - visitors can only access open content
  accessLevel: 'visitor';

  // When the session was created
  createdAt: string;

  // Last activity timestamp
  lastActiveAt: string;

  // Session statistics (computed)
  stats: SessionStats;

  // =========================================================================
  // Social Graph / Profile Metadata (Open Graph protocol - platform-agnostic)
  // =========================================================================

  /** Profile image/avatar URL for social sharing (og:image) */
  avatarUrl?: string;

  /** Alt text for avatar image (accessibility) */
  avatarAlt?: string;

  /** Bio/description for profile sharing (og:description) */
  bio?: string;

  /** Canonical URL to this human's profile (og:url) */
  profileUrl?: string;

  /** Preferred locale/language (og:locale, e.g., 'en_US', 'es_ES') */
  locale?: string;

  /** Profile keywords for discoverability */
  interests?: string[];
}

/**
 * SessionStats - Aggregated session activity.
 */
export interface SessionStats {
  // Content engagement
  nodesViewed: number;
  nodesWithAffinity: number;  // Nodes where human set affinity > 0

  // Path engagement
  pathsStarted: number;
  pathsCompleted: number;
  stepsCompleted: number;

  // Time tracking
  totalSessionTime: number;  // Milliseconds
  averageSessionLength: number;
  sessionCount: number;
}

/**
 * SessionActivity - Individual activity record for history.
 */
export interface SessionActivity {
  timestamp: string;
  type: 'view' | 'affinity' | 'path-start' | 'step-complete' | 'path-complete' | 'explore';
  resourceId: string;
  resourceType: 'content' | 'path' | 'step';
  metadata?: Record<string, unknown>;
}

/**
 * SessionMigration - Data package for Holochain migration.
 *
 * When a human installs the Holochain app, this package transfers
 * their session progress to their permanent agent identity.
 */
export interface SessionMigration {
  sessionId: string;
  migratedAt: string;

  // Target Holochain identity (set during migration)
  targetAgentId?: string;

  // Data to migrate
  affinity: Record<string, number>;
  pathProgress: SessionPathProgress[];
  activities: SessionActivity[];

  // Migration status
  status: 'pending' | 'in-progress' | 'completed' | 'failed';
  error?: string;
}

/**
 * SessionPathProgress - Path progress in session format.
 */
export interface SessionPathProgress {
  pathId: string;
  currentStepIndex: number;
  completedStepIndices: number[];
  stepAffinity: Record<number, number>;
  stepNotes: Record<number, string>;
  startedAt: string;
  lastActivityAt: string;
  completedAt?: string;
}

/**
 * HolochainUpgradePrompt - Configuration for upgrade prompts.
 *
 * These prompts encourage humans to install Holochain at
 * meaningful moments in their journey.
 */
export interface HolochainUpgradePrompt {
  id: string;
  trigger: UpgradeTrigger;
  title: string;
  message: string;
  benefits: string[];
  dismissed: boolean;
  dismissedAt?: string;
}

/**
 * UpgradeTrigger - Events that trigger upgrade prompts.
 */
export type UpgradeTrigger =
  | 'first-affinity'      // Human marks first content as resonant
  | 'path-started'        // Human begins their first path
  | 'path-completed'      // Human completes a learning path
  | 'notes-saved'         // Human saves personal notes
  | 'return-visit'        // Human returns after session expires
  | 'progress-at-risk'    // localStorage nearing quota
  | 'network-feature';    // Human tries a network-only feature
