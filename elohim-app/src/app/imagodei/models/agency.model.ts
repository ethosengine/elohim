/**
 * Agency Model - Human-centered autonomy representation.
 *
 * Philosophy:
 * - Make human agency visible and understandable
 * - Show users where their data lives and who controls it
 * - Provide clear upgrade paths for more autonomy
 * - Emphasize capacity to act within relational networks
 *
 * Four-Stage Progression:
 * 1. Visitor - Browser session, anonymous, no persistence
 * 2. Hosted User - Custodial keys on server, full DHT participation
 * 3. App User - Local Holochain on device, intermittent connectivity
 * 4. Node Operator - Always-on infrastructure, full network participation
 */

/**
 * The four stages of human agency in data ownership within the Elohim network.
 */
export type AgencyStage = 'visitor' | 'hosted' | 'app-user' | 'node-operator';

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
    label: 'Hosted User',
    tagline: 'Keys held by Elohim',
    description:
      'Your identity and progress are stored securely on Elohim servers. Easy setup, full network participation.',
    icon: 'cloud',
    benefits: [
      'Progress saved permanently',
      'Access from any device',
      'Full network participation',
      'Easy setup - no app install',
    ],
    limitations: [
      'Keys managed by Elohim',
      'Dependent on Elohim servers',
      'Less agency than self-hosting',
    ],
    order: 2,
  },

  'app-user': {
    stage: 'app-user',
    label: 'App User',
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
    order: 3,
  },

  'node-operator': {
    stage: 'node-operator',
    label: 'Node Operator',
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
    order: 4,
  },
};

/**
 * Get the next upgrade stage.
 */
export function getNextStage(current: AgencyStage): AgencyStage | null {
  const order = AGENCY_STAGES[current].order;
  const next = Object.values(AGENCY_STAGES).find(s => s.order === order + 1);
  return next?.stage ?? null;
}

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
      label: 'Learning Progress',
      description: 'Paths started, steps completed',
      location: 'browser-storage',
      locationLabel: 'Browser Storage',
      icon: 'school',
      controlledBy: 'user',
      exportable: true,
      deletable: true,
    },
    {
      category: 'preferences',
      label: 'Preferences',
      description: 'Display name, theme, settings',
      location: 'browser-storage',
      locationLabel: 'Browser Storage',
      icon: 'settings',
      controlledBy: 'user',
      exportable: true,
      deletable: true,
    },
    {
      category: 'content-affinity',
      label: 'Content Affinity',
      description: 'What content resonates with you',
      location: 'browser-storage',
      locationLabel: 'Browser Storage',
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
      label: 'Learning Progress',
      description: 'Paths, steps, attestations',
      location: 'dht',
      locationLabel: 'DHT Network',
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
      locationLabel: 'DHT Network',
      icon: 'settings',
      controlledBy: 'network',
      exportable: true,
      deletable: true,
    },
    {
      category: 'content-affinity',
      label: 'Content Affinity',
      description: 'Engagement patterns',
      location: 'dht',
      locationLabel: 'DHT Network',
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
      locationLabel: 'DHT Network',
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
      locationLabel: 'DHT Network',
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
      location: 'local-holochain',
      locationLabel: 'Your Device',
      icon: 'vpn_key',
      controlledBy: 'user',
      exportable: true,
      deletable: false,
    },
    {
      category: 'progress',
      label: 'Learning Progress',
      description: 'Paths, steps, attestations',
      location: 'dht',
      locationLabel: 'DHT Network',
      icon: 'school',
      controlledBy: 'user',
      exportable: true,
      deletable: false,
    },
    {
      category: 'preferences',
      label: 'Preferences',
      description: 'Profile, settings',
      location: 'local-holochain',
      locationLabel: 'Your Device + DHT',
      icon: 'settings',
      controlledBy: 'user',
      exportable: true,
      deletable: true,
    },
    {
      category: 'content-affinity',
      label: 'Content Affinity',
      description: 'Engagement patterns',
      location: 'local-holochain',
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
      locationLabel: 'DHT Network',
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
      locationLabel: 'DHT Network',
      icon: 'user',
      controlledBy: 'user',
      exportable: true,
      deletable: false,
    },
  ];
}
