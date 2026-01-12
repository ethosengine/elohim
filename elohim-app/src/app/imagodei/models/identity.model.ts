/**
 * Identity Model - Holochain-integrated human identity.
 *
 * Philosophy:
 * - Bridge between session-based and Holochain-based identity
 * - Provide unified identity state regardless of backend
 * - Support graceful migration across stewardship stages
 *
 * Identity Modes reflect stewardship progression:
 * - anonymous: Pure browser, no persistent identity
 * - session: localStorage visitor with temporary identity
 * - hosted: Holochain with custodial keys (edge node holds keys)
 * - steward: Holochain with local keys (device or hardware wallet)
 * - migrating: In process of transitioning between stages
 *
 * NOTE: 'self-sovereign' is deprecated and will be removed in a future version.
 * Use 'steward' instead. The term "steward" better reflects the Elohim Protocol
 * ethos of caring for what is entrusted to us, emphasizing relationship and
 * mutual support rather than isolated self-determination.
 *
 * Key Location reflects where cryptographic material lives:
 * - none: No keys (visitor stage)
 * - browser: Keys in IndexedDB/localStorage (less secure)
 * - custodial: Keys held by edge node operator (hosted stage)
 * - device: Keys in local conductor (steward stage)
 * - hardware: Keys in hardware security module (maximum security)
 */

import type { SovereigntyStage } from './sovereignty.model';

// =============================================================================
// Identity Mode
// =============================================================================

/**
 * The current identity mode - reflects stewardship progression.
 *
 * @deprecated 'self-sovereign' - Use 'steward' instead. Will be removed in v2.0.
 */
export type IdentityMode =
  | 'anonymous'      // Pure browser, no session created
  | 'session'        // localStorage visitor with session
  | 'hosted'         // Holochain with custodial keys on edge node
  | 'steward'        // Holochain with keys on user's device (preferred)
  | 'self-sovereign' // @deprecated - use 'steward' instead
  | 'migrating';     // In transition between stages

/**
 * Check if the identity mode represents a steward (local keys).
 * Handles both 'steward' and deprecated 'self-sovereign' values.
 */
export function isStewardMode(mode: IdentityMode): boolean {
  return mode === 'steward' || mode === 'self-sovereign';
}

/**
 * Normalize identity mode, converting deprecated values to current ones.
 * @param mode - The identity mode (possibly deprecated)
 * @returns The normalized mode ('self-sovereign' → 'steward')
 */
export function normalizeIdentityMode(mode: IdentityMode): IdentityMode {
  return mode === 'self-sovereign' ? 'steward' : mode;
}

/**
 * Check if the identity mode represents a network-authenticated mode.
 * Includes both hosted and steward modes.
 */
export function isNetworkMode(mode: IdentityMode): boolean {
  return mode === 'hosted' || isStewardMode(mode);
}

/**
 * Where the user's cryptographic keys are stored.
 */
export type KeyLocation =
  | 'none'           // No keys (visitor/anonymous)
  | 'browser'        // Browser storage (IndexedDB) - least secure
  | 'custodial'      // Edge node server holds keys
  | 'device'         // Local Holochain conductor
  | 'hardware';      // Hardware security module (Ledger, YubiKey)

/**
 * Key backup status - important for recovery planning.
 */
export interface KeyBackupStatus {
  /** Whether keys have been backed up */
  isBackedUp: boolean;
  /** When backup was last verified */
  lastVerified?: string;
  /** Backup method used */
  method?: 'seed-phrase' | 'encrypted-file' | 'hardware-backup' | 'social-recovery';
  /** Whether recovery has been tested */
  recoveryTested: boolean;
}

/**
 * Profile reach levels - who can see your profile.
 * Matches Holochain Human.profile_reach values.
 */
export const ProfileReachLevels = {
  PRIVATE: 'private',      // Only you
  SELF: 'self',            // You and your agents
  INTIMATE: 'intimate',    // Close trust circle
  TRUSTED: 'trusted',      // Trusted connections
  FAMILIAR: 'familiar',    // Known community members
  COMMUNITY: 'community',  // Elohim community
  PUBLIC: 'public',        // Anyone on the network
  COMMONS: 'commons',      // Published to public commons
} as const;

export type ProfileReach = typeof ProfileReachLevels[keyof typeof ProfileReachLevels];

// =============================================================================
// Human Profile
// =============================================================================

/**
 * Human profile data - view model for Angular components.
 * Maps from Holochain Human entry (snake_case) to camelCase.
 */
export interface HumanProfile {
  id: string;
  displayName: string;
  bio: string | null;
  affinities: string[];
  profileReach: ProfileReach;
  location: string | null;
  avatarUrl?: string;
  createdAt: string;
  updatedAt: string;
}

/**
 * Input for registering a new human.
 *
 * For hosted mode (doorway registration):
 * - email and password are required
 * - Doorway creates identity and credentials atomically
 *
 * For native mode (local conductor):
 * - email/password optional
 * - Use registerHumanNative() instead
 */
export interface RegisterHumanRequest {
  displayName: string;
  bio?: string;
  affinities: string[];
  profileReach: ProfileReach;
  location?: string;
  /** Email for authentication (required for hosted mode) */
  email?: string;
  /** Password for authentication (required for hosted mode) */
  password?: string;
  /** If true, migrate session data to new Holochain identity */
  migrateFromSession?: boolean;
}

/**
 * Input for updating profile.
 * All fields optional - only provided fields are updated.
 */
export interface UpdateProfileRequest {
  displayName?: string;
  bio?: string;
  affinities?: string[];
  profileReach?: ProfileReach;
  location?: string;
}

// =============================================================================
// Identity State
// =============================================================================

/**
 * Complete identity state - used by IdentityService signal.
 */
export interface IdentityState {
  /** Current identity mode */
  mode: IdentityMode;
  /** Whether user is authenticated (session or Holochain) */
  isAuthenticated: boolean;
  /** Human ID (Holochain entry ID, or session ID) */
  humanId: string | null;
  /** Display name for UI */
  displayName: string;
  /** Holochain agent public key (base64), if connected */
  agentPubKey: string | null;
  /**
   * W3C Decentralized Identifier for this identity.
   * Generated based on identity mode:
   * - session: did:web:gateway.elohim.host:session:{sessionId}
   * - hosted: did:web:hosted.elohim.host:humans:{humanId}
   * - steward: did:key:{multibase-encoded-pubkey}
   */
  did: string | null;
  /** Full profile data, if loaded */
  profile: HumanProfile | null;
  /** Attestations earned by this human */
  attestations: string[];
  /** Current sovereignty stage */
  sovereigntyStage: SovereigntyStage;

  // =========================================================================
  // Key Management (for sovereignty progression)
  // =========================================================================

  /** Where cryptographic keys are stored */
  keyLocation: KeyLocation;
  /** Can keys be exported from current location? */
  canExportKeys: boolean;
  /** Key backup status */
  keyBackup: KeyBackupStatus | null;

  // =========================================================================
  // Conductor Information (for app-user/node-operator detection)
  // =========================================================================

  /** Whether connected to a local conductor (vs remote edge node) */
  isLocalConductor: boolean;
  /** Conductor URL if connected */
  conductorUrl: string | null;

  // =========================================================================
  // Linked Identities (for hybrid states)
  // =========================================================================

  /** Session ID if visitor session exists alongside Holochain identity */
  linkedSessionId: string | null;
  /** Whether session data is pending migration */
  hasPendingMigration: boolean;

  // =========================================================================
  // Hosting Economics (Shefa integration)
  // =========================================================================

  /** Hosting cost status for hosted humans (costs incurred) */
  hostingCost: HostingCostSummary | null;

  /** Node operator income for those who host others (income received) */
  nodeOperatorIncome: NodeOperatorHostingIncome | null;

  // =========================================================================
  // UI State
  // =========================================================================

  /** Loading state */
  isLoading: boolean;
  /** Error message if any */
  error: string | null;
}

/**
 * Summary of hosting costs for identity display.
 * Full details in session-human.model.ts HostingCostStatus.
 */
export interface HostingCostSummary {
  /** Who covers costs */
  coverageSource: 'commons' | 'steward' | 'sponsor' | 'self' | 'migrated';
  /** Covered by name (if steward/sponsor) */
  coveredByName?: string;
  /** Monthly cost in display currency */
  monthlyCostDisplay: string;
  /** Storage used display */
  storageUsedDisplay: string;
  /** Whether user should consider migrating (cost threshold) */
  migrationRecommended: boolean;
}

/**
 * Node operator hosting income (for node-operators who host others).
 *
 * Philosophy:
 * - Node operators provide infrastructure for hosted humans
 * - They receive value flows to compensate for network support
 * - This creates sustainable economics for decentralized hosting
 * - Connects to hREA ValueFlows for proper accounting
 */
export interface NodeOperatorHostingIncome {
  /** Number of humans currently hosted on this node */
  hostedHumanCount: number;
  /** Total storage provided in bytes */
  totalStorageProvidedBytes: number;
  /** Current month's hosting income in cost units */
  currentMonthIncome: number;
  /** Total lifetime hosting income */
  lifetimeIncome: number;
  /** Currency/unit for income (e.g., 'USD-microcents', 'HOLO-fuel') */
  incomeUnit: string;
  /** When node started hosting others */
  hostingOthersSince: string;
  /** Average uptime percentage */
  uptimePercentage: number;
  /** Node operator tier/reputation score */
  reputationScore: number;
}

/**
 * Initial identity state (anonymous, not authenticated).
 */
export const INITIAL_IDENTITY_STATE: IdentityState = {
  mode: 'anonymous',
  isAuthenticated: false,
  humanId: null,
  displayName: 'Traveler',
  agentPubKey: null,
  did: null,
  profile: null,
  attestations: [],
  sovereigntyStage: 'visitor',

  // Key management - no keys for anonymous
  keyLocation: 'none',
  canExportKeys: false,
  keyBackup: null,

  // Conductor - not connected
  isLocalConductor: false,
  conductorUrl: null,

  // Linked identities - none
  linkedSessionId: null,
  hasPendingMigration: false,

  // Hosting economics - no costs for visitor
  hostingCost: null,
  nodeOperatorIncome: null,

  // UI state
  isLoading: false,
  error: null,
};

// =============================================================================
// Migration Types
// =============================================================================

/**
 * Migration status for sovereignty stage transitions.
 */
export type MigrationStatus =
  | 'idle'
  | 'preparing'
  | 'exporting-keys'
  | 'registering'
  | 'importing-keys'
  | 'transferring'
  | 'verifying'
  | 'completed'
  | 'failed';

/**
 * Migration direction - upgrade increases sovereignty, downgrade decreases.
 */
export type MigrationDirection = 'upgrade' | 'downgrade' | 'lateral';

/**
 * Migration state during sovereignty transitions.
 */
export interface MigrationState {
  status: MigrationStatus;
  /** Direction of migration */
  direction: MigrationDirection;
  /** Source stage */
  fromStage: SovereigntyStage;
  /** Target stage */
  toStage: SovereigntyStage;
  /** Current step description */
  currentStep?: string;
  /** Progress percentage (0-100) */
  progress?: number;
  /** Error message if failed */
  error?: string;
  /** Warnings (non-fatal issues) */
  warnings?: string[];
  /** Whether migration can be cancelled at current step */
  canCancel: boolean;
  /** Whether migration can be rolled back */
  canRollback: boolean;
}

/**
 * Initial migration state.
 */
export const INITIAL_MIGRATION_STATE: MigrationState = {
  status: 'idle',
  direction: 'upgrade',
  fromStage: 'visitor',
  toStage: 'visitor',
  canCancel: true,
  canRollback: false,
};

/**
 * Result of migration operation.
 */
export interface MigrationResult {
  success: boolean;
  /** New human ID if identity changed */
  newHumanId?: string;
  /** New sovereignty stage achieved */
  newStage?: SovereigntyStage;
  /** Error message if failed */
  error?: string;
  /** Warnings during migration */
  warnings?: string[];
  /** Data that was migrated */
  migratedData?: {
    affinityCount: number;
    pathProgressCount: number;
    activityCount: number;
    masteryCount: number;
  };
  /** Key migration details */
  keyMigration?: {
    keyExported: boolean;
    keyImported: boolean;
    sameAgentPubKey: boolean;
  };
}

/**
 * Requirements for a sovereignty transition.
 */
export interface TransitionRequirement {
  /** Type of requirement */
  type: 'app-install' | 'key-backup' | 'node-setup' | 'network-connection' | 'identity-verification';
  /** Human-readable label */
  label: string;
  /** Description of what's needed */
  description: string;
  /** Whether this requirement is satisfied */
  satisfied: boolean;
  /** Action to satisfy this requirement */
  action?: {
    type: 'link' | 'button' | 'guide';
    label: string;
    url?: string;
  };
}

/**
 * Defines a possible sovereignty transition.
 */
export interface SovereigntyTransition {
  /** Source stage */
  from: SovereigntyStage;
  /** Target stage */
  to: SovereigntyStage;
  /** Direction of transition */
  direction: MigrationDirection;
  /** Human-readable title */
  title: string;
  /** Description of what this transition means */
  description: string;
  /** Requirements to complete this transition */
  requirements: TransitionRequirement[];
  /** Data categories that will be migrated */
  dataToMigrate: string[];
  /** Whether this transition preserves the same agent pubkey */
  preservesIdentity: boolean;
  /** Whether this transition is reversible */
  reversible: boolean;
  /** Estimated time to complete */
  estimatedTime?: string;
}

// =============================================================================
// Display Helpers
// =============================================================================

/**
 * Get initials from display name for avatar fallback.
 */
export function getInitials(displayName: string): string {
  if (!displayName) return '?';
  const parts = displayName.trim().split(/\s+/);
  if (parts.length === 1) {
    return parts[0].substring(0, 2).toUpperCase();
  }
  return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
}

/**
 * Get display-friendly reach level label.
 */
export function getReachLabel(reach: ProfileReach): string {
  const labels: Record<ProfileReach, string> = {
    private: 'Private',
    self: 'Self Only',
    intimate: 'Intimate Circle',
    trusted: 'Trusted',
    familiar: 'Familiar',
    community: 'Community',
    public: 'Public',
    commons: 'Commons',
  };
  return labels[reach] ?? reach;
}

/**
 * Get reach level description.
 */
export function getReachDescription(reach: ProfileReach): string {
  const descriptions: Record<ProfileReach, string> = {
    private: 'Only you can see this profile',
    self: 'You and your AI agents',
    intimate: 'Your closest trust circle',
    trusted: 'People you have verified trust with',
    familiar: 'Known community members',
    community: 'Anyone in the Elohim community',
    public: 'Anyone on the network',
    commons: 'Published to the public commons',
  };
  return descriptions[reach] ?? '';
}
