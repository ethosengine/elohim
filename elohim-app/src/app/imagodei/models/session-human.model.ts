/**
 * SessionHuman - A temporary traveler identity with upgrade path.
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
 * Session State:
 * - A session can exist independently (pure visitor)
 * - A session can be linked to a Holochain identity (hybrid state)
 * - A session can be in the process of upgrading (migrating state)
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
  // =========================================================================
  // Core Identity
  // =========================================================================

  /** Temporary identity (generated UUID for session) */
  sessionId: string;

  /** Display name (optional, can be set later) */
  displayName: string;

  /** When the session was created */
  createdAt: string;

  /** Last activity timestamp */
  lastActiveAt: string;

  /** Session statistics */
  stats: SessionStats;

  // =========================================================================
  // Session State (flexible for hybrid scenarios)
  // =========================================================================

  /**
   * Whether this is a pure anonymous session.
   * False if linked to a Holochain identity (hybrid mode).
   */
  isAnonymous: boolean;

  /**
   * Current access level.
   * Can upgrade beyond 'visitor' if linked to Holochain identity.
   */
  accessLevel: SessionAccessLevel;

  /**
   * Current session state.
   * - 'active': Normal session operation
   * - 'upgrading': In process of migrating to Holochain
   * - 'linked': Session exists alongside Holochain identity
   * - 'migrated': Session has been migrated (kept for reference)
   */
  sessionState: SessionState;

  // =========================================================================
  // Holochain Link (for hybrid state)
  // =========================================================================

  /**
   * Linked Holochain agent public key.
   * Set when user has both session AND Holochain identity.
   * Enables offline session to sync when connection restored.
   */
  linkedAgentPubKey?: string;

  /**
   * Linked Human ID in Holochain.
   * Allows session to reference the permanent identity.
   */
  linkedHumanId?: string;

  /**
   * When the link was established.
   */
  linkedAt?: string;

  // =========================================================================
  // Upgrade Tracking
  // =========================================================================

  /**
   * Upgrade intent - tracks if user has started upgrade process.
   */
  upgradeIntent?: UpgradeIntent;

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
 * Session state machine.
 */
export type SessionState =
  | 'active' // Normal session operation
  | 'upgrading' // In process of migrating to Holochain
  | 'linked' // Session exists alongside Holochain identity
  | 'migrated'; // Session has been fully migrated

/**
 * Access levels for session humans.
 * Expanded to support hybrid scenarios.
 */
export type SessionAccessLevel =
  | 'visitor' // Pure session, open content only
  | 'pending' // Upgrade in progress
  | 'linked'; // Has linked Holochain identity

/**
 * Tracks upgrade intent for users who start but don't complete upgrade.
 */
export interface UpgradeIntent {
  /** Target agency stage */
  targetStage: 'hosted' | 'app-steward' | 'node-operator';
  /** When upgrade process started */
  startedAt: string;
  /** Current step in upgrade process */
  currentStep: string;
  /** Steps completed */
  completedSteps: string[];
  /** Whether user explicitly paused the upgrade */
  paused: boolean;
  /** Reason for pause/abandonment if any */
  pauseReason?: string;
}

// =============================================================================
// Hosting Economics (Shefa Integration)
// =============================================================================

/**
 * Tracks hosting costs and coverage for hosted humans.
 *
 * Philosophy:
 * - Hosted humans have real infrastructure costs (storage, compute, bandwidth)
 * - Until they migrate to their own device, someone covers these costs
 * - Cost coverage can come from: commons fund, steward, sponsor, or self-pay
 * - Transparent cost tracking incentivizes migration to self-sovereignty
 */
export interface HostingCostStatus {
  /**
   * Current hosting cost coverage source.
   * - 'commons': Covered by the Elohim commons fund (default for new users)
   * - 'steward': Covered by a steward who sponsors this human
   * - 'sponsor': Covered by a specific sponsor (org, grant, etc.)
   * - 'self': Human pays their own hosting (before migration)
   * - 'migrated': No hosting costs (data on user's device)
   */
  coverageSource: HostingCoverageSource;

  /** Steward or sponsor ID if applicable */
  coveredById?: string;

  /** Steward or sponsor display name */
  coveredByName?: string;

  /** Current monthly hosting cost in smallest unit (e.g., microcents) */
  monthlyHostingCost: number;

  /** Total hosting costs accumulated since account creation */
  totalHostingCostAccumulated: number;

  /** Currency/unit for costs (e.g., 'USD-microcents', 'HOLO-fuel') */
  costUnit: string;

  /** When hosting started (for cost calculation) */
  hostingStartedAt: string;

  /** Estimated storage used in bytes */
  storageUsedBytes: number;

  /** Whether user has been notified about hosting costs */
  costTransparencyAcknowledged: boolean;

  /** Grace period end date (if applicable) */
  gracePeriodEndsAt?: string;
}

/**
 * Who covers hosting costs for a hosted human.
 */
export type HostingCoverageSource =
  | 'commons' // Elohim commons fund covers costs
  | 'steward' // A steward sponsors this human
  | 'sponsor' // External sponsor (org, grant)
  | 'self' // Human pays own costs
  | 'migrated'; // No costs - data on user's device

/**
 * Hosting cost tier for transparent pricing.
 */
export interface HostingTier {
  /** Tier identifier */
  id: string;
  /** Human-readable name */
  name: string;
  /** Description of what's included */
  description: string;
  /** Storage limit in bytes */
  storageLimitBytes: number;
  /** Monthly cost in cost units */
  monthlyCost: number;
  /** Features included */
  features: string[];
}

/**
 * Default hosting tiers.
 */
export const HOSTING_TIERS: HostingTier[] = [
  {
    id: 'explorer',
    name: 'Explorer',
    description: 'Basic hosting for new community members',
    storageLimitBytes: 100 * 1024 * 1024, // 100 MB
    monthlyCost: 0, // Covered by commons
    features: ['Learning progress', 'Basic profile', 'Community participation'],
  },
  {
    id: 'contributor',
    name: 'Contributor',
    description: 'Enhanced hosting for active contributors',
    storageLimitBytes: 1024 * 1024 * 1024, // 1 GB
    monthlyCost: 100, // 100 microcents = $0.001/month
    features: ['All Explorer features', 'Content creation', 'Extended history'],
  },
  {
    id: 'steward',
    name: 'Steward',
    description: 'Full hosting for community stewards',
    storageLimitBytes: 10 * 1024 * 1024 * 1024, // 10 GB
    monthlyCost: 500, // 500 microcents = $0.005/month
    features: ['All Contributor features', 'Presence stewardship', 'Attestation issuance'],
  },
];

/**
 * SessionStats - Aggregated session activity.
 */
export interface SessionStats {
  // Content engagement
  nodesViewed: number;
  nodesWithAffinity: number; // Nodes where human set affinity > 0

  // Path engagement
  pathsStarted: number;
  pathsCompleted: number;
  stepsCompleted: number;

  // Time tracking
  totalSessionTime: number; // Milliseconds
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
  | 'first-affinity' // Human marks first content as resonant
  | 'path-started' // Human begins their first path
  | 'path-completed' // Human completes a learning path
  | 'notes-saved' // Human saves personal notes
  | 'return-visit' // Human returns after session expires
  | 'progress-at-risk' // localStorage nearing quota
  | 'network-feature'; // Human tries a network-only feature
