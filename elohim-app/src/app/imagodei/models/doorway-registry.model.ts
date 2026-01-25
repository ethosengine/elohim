/**
 * Doorway Registry Models
 *
 * Manages the list of doorways a user has registered with for disaster recovery.
 * In sovereign mode (Tauri app), the user's elohim-node runs locally, but they
 * can pre-register with doorways as fallback options if their local device is lost.
 *
 * Registry is stored:
 * 1. Locally in Tauri encrypted storage
 * 2. In DHT as RecoveryHint (encrypted, type: 'trusted_doorways')
 *
 * @module imagodei/models/doorway-registry
 */

/**
 * Trust tier for a doorway.
 *
 * Trust levels are earned through uptime history and community reputation:
 * - emerging: New doorway, limited history (<1 week uptime)
 * - established: Proven reliability (1 week - 1 month uptime)
 * - trusted: Strong track record (1-6 months uptime)
 * - anchor: Core infrastructure doorway (6+ months, >99.5% uptime)
 */
export type DoorwayTrustTier = 'emerging' | 'established' | 'trusted' | 'anchor';

/**
 * Capabilities a doorway can offer.
 */
export type DoorwayCapability =
  | 'projection' // Can serve content projection queries
  | 'storage' // Can accept blob uploads
  | 'recovery' // Can process disaster recovery requests
  | 'import' // Can process bulk content imports
  | 'streaming'; // Supports WebSocket streaming

/**
 * A doorway the user has registered with for potential recovery.
 */
export interface RegisteredDoorway {
  /** Unique doorway identifier (e.g., "alpha-elohim-host") */
  doorwayId: string;

  /** Doorway API URL (e.g., "https://alpha.elohim.host") */
  url: string;

  /** Human-readable name (e.g., "Alpha Doorway (US-West)") */
  displayName: string;

  /** Geographic region for latency selection */
  region?: string;

  /** Trust tier based on uptime history */
  trustTier: DoorwayTrustTier;

  /** When this doorway was added to user's registry */
  registeredAt: string;

  /** Last successful connection to this doorway */
  lastConnectedAt?: string;

  /** Capabilities this doorway offers */
  capabilities: DoorwayCapability[];

  /** Whether this doorway is enabled for recovery */
  recoveryEnabled: boolean;

  /** User's notes about this doorway (optional) */
  notes?: string;

  /** Whether connection is currently healthy */
  isHealthy?: boolean;

  /** Last health check timestamp */
  healthCheckedAt?: string;
}

/**
 * User's complete doorway registry.
 */
export interface DoorwayRegistry {
  /** List of registered doorways */
  doorways: RegisteredDoorway[];

  /** Primary doorway ID (preferred for recovery) */
  primaryDoorwayId?: string;

  /** When the registry was last synced to DHT */
  lastSynced: string;

  /** Version for conflict resolution */
  version: number;
}

/**
 * Input for adding a new doorway to the registry.
 */
export interface AddDoorwayInput {
  url: string;
  displayName?: string;
  recoveryEnabled?: boolean;
  notes?: string;
}

/**
 * Result of adding a doorway.
 */
export interface AddDoorwayResult {
  success: boolean;
  doorway?: RegisteredDoorway;
  error?: string;
}

/**
 * Recovery selection from doorway picker UI.
 */
export interface DoorwaySelection {
  doorwayId: string;
  url: string;
  displayName: string;
  trustTier: DoorwayTrustTier;
}

/**
 * Health check result for a doorway.
 */
export interface DoorwayHealthCheck {
  doorwayId: string;
  isHealthy: boolean;
  latencyMs?: number;
  checkedAt: string;
  error?: string;
}

/**
 * Doorway info returned from /doorway/info endpoint.
 */
export interface DoorwayInfo {
  doorwayId: string;
  displayName: string;
  region?: string;
  capabilities: DoorwayCapability[];
  trustTier: DoorwayTrustTier;
  version: string;
  publicKey?: string;
}

/**
 * Default trust tiers with minimum uptime thresholds.
 */
export const TRUST_TIER_THRESHOLDS = {
  emerging: 0, // No minimum
  established: 7 * 24 * 60 * 60, // 7 days in seconds
  trusted: 30 * 24 * 60 * 60, // 30 days in seconds
  anchor: 180 * 24 * 60 * 60, // 180 days in seconds
} as const;

/**
 * Trust tier display names and colors for UI.
 */
export const TRUST_TIER_DISPLAY = {
  emerging: {
    name: 'Emerging',
    color: '#888888',
    description: 'New doorway with limited history',
  },
  established: {
    name: 'Established',
    color: '#4CAF50',
    description: 'Proven reliability over 1 week',
  },
  trusted: {
    name: 'Trusted',
    color: '#2196F3',
    description: 'Strong track record over 1 month',
  },
  anchor: {
    name: 'Anchor',
    color: '#9C27B0',
    description: 'Core infrastructure with 6+ months uptime',
  },
} as const;

/**
 * Create an empty doorway registry.
 */
export function createEmptyRegistry(): DoorwayRegistry {
  return {
    doorways: [],
    primaryDoorwayId: undefined,
    lastSynced: new Date().toISOString(),
    version: 1,
  };
}

/**
 * Sort doorways by trust tier (highest first), then by region.
 */
export function sortDoorwaysByTrust(doorways: RegisteredDoorway[]): RegisteredDoorway[] {
  const tierOrder: Record<DoorwayTrustTier, number> = {
    anchor: 0,
    trusted: 1,
    established: 2,
    emerging: 3,
  };

  return [...doorways].sort((a, b) => {
    const tierDiff = tierOrder[a.trustTier] - tierOrder[b.trustTier];
    if (tierDiff !== 0) return tierDiff;
    return (a.region || '').localeCompare(b.region || '');
  });
}

/**
 * Get doorways that support recovery capability.
 */
export function getRecoveryDoorways(registry: DoorwayRegistry): RegisteredDoorway[] {
  return registry.doorways.filter(d => d.capabilities.includes('recovery') && d.recoveryEnabled);
}

/**
 * Get the primary doorway for recovery, or first available.
 */
export function getPrimaryRecoveryDoorway(
  registry: DoorwayRegistry
): RegisteredDoorway | undefined {
  const recoveryDoorways = getRecoveryDoorways(registry);
  if (recoveryDoorways.length === 0) return undefined;

  // Return primary if it's in the recovery list
  if (registry.primaryDoorwayId) {
    const primary = recoveryDoorways.find(d => d.doorwayId === registry.primaryDoorwayId);
    if (primary) return primary;
  }

  // Otherwise return highest trust doorway
  const sorted = sortDoorwaysByTrust(recoveryDoorways);
  return sorted[0];
}
