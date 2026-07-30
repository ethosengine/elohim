/**
 * Agency Model - Human-centered autonomy representation.
 *
 * Stages have two roles:
 *   1. **Progression stages** (visitor → hosted → app-steward → node-steward)
 *      form the identity-authority graduation ladder. UI surfaces like the
 *      stage-progression chips and `getNextStage()` walk this ladder.
 *   2. **Recovery modes** (hosted-steward) name in-between states a steward
 *      may temporarily occupy when their authoritative peer-native portal is
 *      unreachable.
 *
 * Happy path when a graduated steward signs in through a doorway: the
 * doorway handles routing and the elohim-storage captive-OAuth portal
 * ceremony only — the DHT/conductor/agent compute happens on the steward's
 * own device or home node, NOT the doorway. The session shows as
 * `app-steward` (or `node-steward`). The doorway is purely a router/proxy
 * in this case; the steward's peer-native portal is the identity provider
 * end-to-end.
 *
 * Failover (hosted-steward) — recovery mode: when the steward's own device
 * is unreachable, the doorway temporarily provides DHT/conductor compute
 * for the steward (the same way it does for `hosted` users) so they can
 * see their content (via DHT shards + resilient backups), register a new
 * device, and restore reciprocal commitments. Identity authority still
 * belongs to the steward's peer-native portal — the doorway is a relying
 * party here, not an identity provider. Once the steward's portal/device
 * comes back online, they return to `app-steward` or `node-steward`.
 *
 * Conductor-compute / identity-authority matrix:
 *
 *   | Stage          | Conductor compute     | Identity authority           |
 *   |----------------|-----------------------|------------------------------|
 *   | visitor        | none (session-only)   | none                         |
 *   | hosted         | doorway               | doorway (IdP + relying party)|
 *   | hosted-steward | doorway (recovery)    | steward's peer-native portal |
 *   | app-steward    | steward's device      | steward's peer-native portal |
 *   | node-steward   | steward's home node   | steward's peer-native portal |
 *
 * Recovery modes carry `isRecoveryMode: true` and are excluded from the
 * progression ladder (chip display, `getNextStage()`).
 *
 * See genesis/docs/plans/2026-05-19-doorway-stewardship-chain-design.md
 * and project_m5_reframe_auth_portal_convergence in memory.
 */

export type AgencyStage = 'visitor' | 'hosted' | 'hosted-steward' | 'app-steward' | 'node-steward';

/**
 * Detailed information about each agency stage.
 */
export interface AgencyStageInfo {
  stage: AgencyStage;
  label: string;
  tagline: string;
  description: string;
  icon: string;
  benefits: string[];
  limitations: string[];
  order: number;
  /**
   * When true, this stage is a recovery / fallback mode rather than a step
   * on the identity-authority progression ladder. The stage-progression UI
   * skips recovery modes, and `getNextStage()` walks over them.
   *
   * Currently only `hosted-steward` carries this flag — it names the state
   * of a graduated steward signing in through a doorway because their own
   * device or node is unreachable. Authoritative identity still lives on
   * the steward's peer-native portal.
   */
  isRecoveryMode?: boolean;
}

/**
 * Where different types of data are stored.
 */
export type DataLocation =
  | 'browser-memory'
  | 'browser-storage'
  | 'hosted-server'
  | 'local-holochain'
  | 'dht'
  | 'encrypted-backup';

/**
 * Categories of user data.
 */
export type DataCategory =
  | 'identity'
  | 'progress'
  | 'preferences'
  | 'content-affinity'
  | 'social'
  | 'credentials';

/**
 * Data residency item - describes where a type of data lives.
 */
export interface DataResidencyItem {
  category: DataCategory;
  label: string;
  description: string;
  location: DataLocation;
  locationLabel: string;
  icon: string;
  controlledBy: 'user' | 'elohim' | 'network';
  exportable: boolean;
  deletable: boolean;
}

/**
 * Connection status for network participation.
 */
export interface ConnectionStatus {
  state: 'offline' | 'connecting' | 'connected' | 'syncing' | 'error';
  label: string;
  description?: string;
  lastSync?: Date;
  peerCount?: number;
  latency?: number;
}

/**
 * Key/credential information for display.
 */
export interface KeyInfo {
  type: 'agent-pubkey' | 'signing-key' | 'capability-secret';
  label: string;
  value: string;
  truncated: string;
  created?: Date;
  expires?: Date;
  canExport: boolean;
  canRevoke: boolean;
}

/**
 * Network participation statistics.
 */
export interface NetworkStats {
  connectedSince?: Date;
  totalPeers: number;
  dataShared: number;
  dataReceived: number;
  uptime?: number;
}

/**
 * Migration progress when upgrading sovereignty stage.
 */
export interface MigrationProgress {
  status: 'pending' | 'in-progress' | 'completed' | 'failed';
  currentStep: string;
  totalSteps: number;
  completedSteps: number;
  error?: string;
}

/**
 * Complete agency state for a user.
 */
export interface AgencyState {
  currentStage: AgencyStage;
  stageInfo: AgencyStageInfo;
  connectionStatus: ConnectionStatus;
  dataResidency: DataResidencyItem[];
  keys: KeyInfo[];
  hasStoredCredentials: boolean;
  networkStats?: NetworkStats;
  migrationAvailable: boolean;
  migrationTarget?: AgencyStage;
  migrationProgress?: MigrationProgress;
}

// Data residency constants (used across stage definitions)
const LABEL_LEARNING_PROGRESS = 'Learning Progress';
const LOC_BROWSER_STORAGE: DataLocation = 'browser-storage';
const LOC_LABEL_BROWSER_STORAGE = 'Browser Storage';
const CAT_CONTENT_AFFINITY: DataCategory = 'content-affinity';
const LABEL_CONTENT_AFFINITY = 'Content Affinity';
const LOC_LABEL_DHT_NETWORK = 'DHT Network';
const LOC_LOCAL_HOLOCHAIN: DataLocation = 'local-holochain';

/**
 * Stage definitions with human-friendly descriptions.
 */
export const AGENCY_STAGES: Record<AgencyStage, AgencyStageInfo> = {
  visitor: {
    stage: 'visitor',
    label: 'Visitor',
    tagline: 'Exploring anonymously',
    description:
      'Your data exists only in your browser session. No account needed, but progress is temporary.',
    icon: 'explore',
    benefits: ['No account required', 'Complete privacy', 'Explore freely'],
    limitations: [
      'Progress lost when you leave',
      'Cannot access gated content',
      'Cannot participate in community',
    ],
    order: 1,
  },

  hosted: {
    stage: 'hosted',
    label: 'Hosted Visitor',
    tagline: 'A doorway holds your keys',
    description:
      'You are visiting through a doorway that hosts your identity. Easy onboarding; the doorway is both your relying party and your identity provider until you graduate.',
    icon: 'cloud',
    benefits: [
      'Progress saved permanently',
      'Access from any device',
      'Full network participation',
      'Easy setup - no app install',
    ],
    limitations: [
      'Keys managed by the doorway steward',
      'Dependent on this doorway',
      'No peer-native agency yet',
    ],
    order: 2,
  },

  'hosted-steward': {
    stage: 'hosted-steward',
    label: 'Hosted Steward',
    tagline: 'A steward in recovery — doorway temporarily hosts your conductor',
    description:
      'Your own device is unreachable, so you have signed in through a doorway. While in this recovery mode the doorway temporarily provides DHT/conductor compute for you (the same way it does for a hosted user), letting you see your content (sharded across your resilient peer backups), register a new device, and restore reciprocal commitments. Identity authority still belongs to your peer-native portal — the doorway is a relying party here, not an identity provider. Once your portal/device comes back online you return to App Steward (or Node Steward). This is a recovery mode, not a step on the upgrade ladder.',
    icon: 'cloud_done',
    benefits: [
      'See all your content via DHT shards + resilient backups',
      'Register a new device to restore your own native compute',
      'Restore reciprocal commitments after device loss',
      'Identity authority remains on your peer-native portal',
    ],
    limitations: [
      'Doorway-hosted conductor compute — not your own device',
      'Some sensitive actions require your portal to be reachable',
      'Recovery flow — not a long-term home',
    ],
    order: 3,
    isRecoveryMode: true,
  },

  'app-steward': {
    stage: 'app-steward',
    label: 'App Steward',
    tagline: 'Keys on your device',
    description: 'Your identity lives on your device. You control your keys. Syncs when connected.',
    icon: 'smartphone',
    benefits: [
      'You control your keys',
      'Data stored locally',
      'Works offline',
      'True data ownership',
    ],
    limitations: ['Must keep device secure', 'Syncs only when online', 'Requires app installation'],
    order: 4,
  },

  'node-steward': {
    stage: 'node-steward',
    label: 'Node Steward',
    tagline: 'Always-on infrastructure',
    description:
      'You run your own always-on Holochain node. Maximum agency and network contribution.',
    icon: 'dns',
    benefits: [
      'Maximum agency',
      'Always-on participation',
      'Support the network',
      'Host others in your trust network',
    ],
    limitations: [
      'Requires technical setup',
      'Server/hardware costs',
      'Maintenance responsibility',
    ],
    order: 5,
  },
};

/**
 * Get the next stage on the progression ladder.
 *
 * Walks AGENCY_STAGES by `order`, skipping any stage with `isRecoveryMode: true`.
 * For example, `getNextStage('hosted')` returns `'app-steward'` — `hosted-steward`
 * is a recovery mode, not a step on the upgrade ladder.
 */
export function getNextStage(current: AgencyStage): AgencyStage | null {
  if (AGENCY_STAGES[current].isRecoveryMode) {
    // A recovery mode has no "next" on the progression ladder. The steward's
    // resolution is to restore peer-native authority, not graduate further.
    return null;
  }
  const order = AGENCY_STAGES[current].order;
  const next = Object.values(AGENCY_STAGES)
    .filter(s => !s.isRecoveryMode)
    .sort((a, b) => a.order - b.order)
    .find(s => s.order > order);
  return next?.stage ?? null;
}

/**
 * The progression ladder — all stages excluding recovery modes, sorted by order.
 * UI surfaces (e.g. the stage-progression chips on the device-stewardship upgrade
 * prompt) should iterate this rather than Object.values(AGENCY_STAGES).
 */
export const PROGRESSION_STAGES: AgencyStageInfo[] = Object.values(AGENCY_STAGES)
  .filter(s => !s.isRecoveryMode)
  .sort((a, b) => a.order - b.order);

/**
 * Check if a stage has greater agency than another.
 */
export function hasGreaterAgency(a: AgencyStage, b: AgencyStage): boolean {
  return AGENCY_STAGES[a].order > AGENCY_STAGES[b].order;
}

/**
 * Get data residency for visitor stage.
 */
export function getVisitorDataResidency(): DataResidencyItem[] {
  return [
    {
      category: 'identity',
      label: 'Identity',
      description: 'Session-only (no persistent identity)',
      location: 'browser-memory',
      locationLabel: 'Browser Memory',
      icon: 'person_outline',
      controlledBy: 'user',
      exportable: false,
      deletable: false,
    },
    {
      category: 'progress',
      label: LABEL_LEARNING_PROGRESS,
      description: 'Paths started, steps completed',
      location: LOC_BROWSER_STORAGE,
      locationLabel: LOC_LABEL_BROWSER_STORAGE,
      icon: 'school',
      controlledBy: 'user',
      exportable: true,
      deletable: true,
    },
    {
      category: 'preferences',
      label: 'Preferences',
      description: 'Display name, theme, settings',
      location: LOC_BROWSER_STORAGE,
      locationLabel: LOC_LABEL_BROWSER_STORAGE,
      icon: 'settings',
      controlledBy: 'user',
      exportable: true,
      deletable: true,
    },
    {
      category: CAT_CONTENT_AFFINITY,
      label: LABEL_CONTENT_AFFINITY,
      description: 'What content resonates with you',
      location: LOC_BROWSER_STORAGE,
      locationLabel: LOC_LABEL_BROWSER_STORAGE,
      icon: 'favorite',
      controlledBy: 'user',
      exportable: true,
      deletable: true,
    },
  ];
}

/**
 * Get data residency for hosted user stage.
 */
export function getHostedDataResidency(): DataResidencyItem[] {
  return [
    {
      category: 'identity',
      label: 'Identity Keys',
      description: 'Cryptographic identity (custodial)',
      location: 'hosted-server',
      locationLabel: 'Elohim Server',
      icon: 'vpn_key',
      controlledBy: 'elohim',
      exportable: true,
      deletable: false,
    },
    {
      category: 'progress',
      label: LABEL_LEARNING_PROGRESS,
      description: 'Paths, steps, attestations',
      location: 'dht',
      locationLabel: LOC_LABEL_DHT_NETWORK,
      icon: 'school',
      controlledBy: 'network',
      exportable: true,
      deletable: false,
    },
    {
      category: 'preferences',
      label: 'Preferences',
      description: 'Profile, settings',
      location: 'dht',
      locationLabel: LOC_LABEL_DHT_NETWORK,
      icon: 'settings',
      controlledBy: 'network',
      exportable: true,
      deletable: true,
    },
    {
      category: CAT_CONTENT_AFFINITY,
      label: LABEL_CONTENT_AFFINITY,
      description: 'Engagement patterns',
      location: 'dht',
      locationLabel: LOC_LABEL_DHT_NETWORK,
      icon: 'favorite',
      controlledBy: 'network',
      exportable: true,
      deletable: true,
    },
    {
      category: 'social',
      label: 'Relationships',
      description: 'Connections, endorsements',
      location: 'dht',
      locationLabel: LOC_LABEL_DHT_NETWORK,
      icon: 'people',
      controlledBy: 'network',
      exportable: true,
      deletable: false,
    },
    {
      category: 'credentials',
      label: 'Credentials',
      description: 'Attestations, badges',
      location: 'dht',
      locationLabel: LOC_LABEL_DHT_NETWORK,
      icon: 'verified',
      controlledBy: 'network',
      exportable: true,
      deletable: false,
    },
  ];
}

/**
 * Get data residency for app user stage.
 */
export function getAppUserDataResidency(): DataResidencyItem[] {
  return [
    {
      category: 'identity',
      label: 'Identity Keys',
      description: 'Cryptographic identity (steward)',
      location: LOC_LOCAL_HOLOCHAIN,
      locationLabel: 'Your Device',
      icon: 'vpn_key',
      controlledBy: 'user',
      exportable: true,
      deletable: false,
    },
    {
      category: 'progress',
      label: LABEL_LEARNING_PROGRESS,
      description: 'Paths, steps, attestations',
      location: 'dht',
      locationLabel: LOC_LABEL_DHT_NETWORK,
      icon: 'school',
      controlledBy: 'user',
      exportable: true,
      deletable: false,
    },
    {
      category: 'preferences',
      label: 'Preferences',
      description: 'Profile, settings',
      location: LOC_LOCAL_HOLOCHAIN,
      locationLabel: 'Your Device + DHT',
      icon: 'settings',
      controlledBy: 'user',
      exportable: true,
      deletable: true,
    },
    {
      category: CAT_CONTENT_AFFINITY,
      label: LABEL_CONTENT_AFFINITY,
      description: 'Engagement patterns',
      location: LOC_LOCAL_HOLOCHAIN,
      locationLabel: 'Your Device',
      icon: 'favorite',
      controlledBy: 'user',
      exportable: true,
      deletable: true,
    },
    {
      category: 'social',
      label: 'Relationships',
      description: 'Connections, endorsements',
      location: 'dht',
      locationLabel: LOC_LABEL_DHT_NETWORK,
      icon: 'people',
      controlledBy: 'user',
      exportable: true,
      deletable: false,
    },
    {
      category: 'credentials',
      label: 'Credentials',
      description: 'Attestations, badges',
      location: 'dht',
      locationLabel: LOC_LABEL_DHT_NETWORK,
      icon: 'user',
      controlledBy: 'user',
      exportable: true,
      deletable: false,
    },
  ];
}
