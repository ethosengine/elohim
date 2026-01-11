/**
 * Doorway Models - Gateway to the Elohim Network
 *
 * Doorways are Web 2.0 gateways that bridge users into the fully P2P
 * Holochain network. Similar to Mastodon instances in the Fediverse,
 * users select a doorway at registration which serves as their:
 *
 * - Identity provider (Imagodei SSO)
 * - Hosted Holochain gateway
 * - Recovery fallback if their local node fails
 *
 * Design notes:
 * - Doorways self-register on the DHT (decentralized discovery)
 * - Bootstrap list provides fallback for first-time users
 * - Doorway selection is stored in localStorage
 */

// =============================================================================
// Storage Keys
// =============================================================================

/** localStorage key for selected doorway URL */
export const DOORWAY_URL_KEY = 'elohim-doorway-url';

/** localStorage key for doorway metadata cache */
export const DOORWAY_CACHE_KEY = 'elohim-doorway-cache';

// =============================================================================
// Doorway Info
// =============================================================================

/** Status of a doorway */
export type DoorwayStatus = 'online' | 'degraded' | 'offline' | 'unknown';

/** Geographic region for doorway filtering */
export type DoorwayRegion =
  | 'north-america'
  | 'south-america'
  | 'europe'
  | 'asia-pacific'
  | 'africa'
  | 'middle-east'
  | 'global';

/** Features a doorway may support */
export type DoorwayFeature =
  | 'premium-content'      // Supports gated/premium content
  | 'high-availability'    // Multi-region redundancy
  | 'media-hosting'        // Rich media storage
  | 'recovery-service'     // Social recovery support
  | 'analytics'            // Usage analytics
  | 'custom-domain';       // Custom domain support

/**
 * Information about an Elohim doorway.
 *
 * This is the primary type for representing a doorway in the UI
 * and is what gets stored in the on-chain registry.
 */
export interface DoorwayInfo {
  /** Unique identifier (kebab-case, e.g., "alpha-elohim-host") */
  id: string;

  /** Display name (e.g., "Alpha (US-West)") */
  name: string;

  /** Base URL for the doorway (e.g., "https://alpha.elohim.host") */
  url: string;

  /** Community description of this doorway */
  description: string;

  /** Geographic region */
  region: DoorwayRegion;

  /** Who operates this doorway */
  operator: string;

  /** Supported features */
  features: DoorwayFeature[];

  /** Current operational status */
  status: DoorwayStatus;

  /** Approximate number of users (optional, for guidance) */
  userCount?: number;

  /** Whether new registrations are accepted */
  registrationOpen: boolean;

  /** When this doorway was registered (ISO string) */
  registeredAt?: string;

  /** Number of vouches from other doorways/users */
  vouchCount?: number;
}

/**
 * Doorway with health check results.
 * Extends DoorwayInfo with runtime health data.
 */
export interface DoorwayWithHealth extends DoorwayInfo {
  /** Latency to doorway in ms (null if unreachable) */
  latencyMs: number | null;

  /** Last successful health check (ISO string) */
  lastHealthCheck: string;

  /** Whether doorway is currently reachable */
  isReachable: boolean;
}

// =============================================================================
// Registry Types
// =============================================================================

/**
 * Entry for registering a new doorway on-chain.
 * Used by doorway operators to advertise their service.
 */
export interface DoorwayRegistration {
  name: string;
  url: string;
  description: string;
  region: DoorwayRegion;
  operator: string;
  features: DoorwayFeature[];
  registrationOpen: boolean;
}

/**
 * Vouch for a doorway's reliability.
 * Users and other doorway operators can vouch.
 */
export interface DoorwayVouch {
  doorwayId: string;
  voucherId: string;     // Agent who vouched
  timestamp: string;
  comment?: string;
}

/**
 * Status update from a doorway operator.
 */
export interface DoorwayStatusUpdate {
  doorwayId: string;
  status: DoorwayStatus;
  message?: string;       // Optional status message
  expectedResolution?: string;  // ISO date for degraded/offline
}

// =============================================================================
// Discovery Types
// =============================================================================

/**
 * Result of a doorway health check.
 */
export interface DoorwayHealthCheckResult {
  url: string;
  status: DoorwayStatus;
  latencyMs: number | null;
  version?: string;       // Doorway software version
  checkedAt: string;
  error?: string;
}

/**
 * Cached doorway data for offline access.
 */
export interface DoorwayCacheEntry {
  doorway: DoorwayInfo;
  fetchedAt: string;
  expiresAt: string;
}

/**
 * Bootstrap doorway list for first-time users.
 * Hardcoded fallback when DHT is unavailable.
 */
export const BOOTSTRAP_DOORWAYS: DoorwayInfo[] = [
  {
    id: 'doorway-dev-elohim-host',
    name: 'Dev Gateway',
    url: 'https://doorway-dev.elohim.host',
    description: 'Development doorway operated by Matthew Dowell @ Ethos Engine. For alpha/dev testing.',
    region: 'north-america',
    operator: 'Matthew Dowell',
    features: ['premium-content', 'high-availability', 'recovery-service', 'media-hosting'],
    status: 'online',
    registrationOpen: true,
    vouchCount: 0,
  },
];

// =============================================================================
// Selection State
// =============================================================================

/**
 * User's doorway selection state.
 */
export interface DoorwaySelection {
  /** Selected doorway info */
  doorway: DoorwayInfo;

  /** When selection was made */
  selectedAt: string;

  /** Whether this was auto-selected or user choice */
  isExplicit: boolean;
}

// =============================================================================
// API Types
// =============================================================================

/** Request to validate a doorway URL */
export interface ValidateDoorwayRequest {
  url: string;
}

/** Response from doorway validation */
export interface ValidateDoorwayResponse {
  isValid: boolean;
  doorway?: DoorwayInfo;
  error?: string;
}

/** Response from doorway health endpoint */
export interface DoorwayHealthResponse {
  status: DoorwayStatus;
  version: string;
  uptime: number;          // Seconds
  userCount?: number;
  registrationOpen: boolean;
}

// =============================================================================
// Helpers
// =============================================================================

/**
 * Get region display name.
 */
export function getRegionDisplayName(region: DoorwayRegion): string {
  const names: Record<DoorwayRegion, string> = {
    'north-america': 'North America',
    'south-america': 'South America',
    'europe': 'Europe',
    'asia-pacific': 'Asia Pacific',
    'africa': 'Africa',
    'middle-east': 'Middle East',
    'global': 'Global',
  };
  return names[region] ?? region;
}

/**
 * Get status display info.
 */
export function getStatusDisplay(status: DoorwayStatus): { label: string; color: string; icon: string } {
  const displays: Record<DoorwayStatus, { label: string; color: string; icon: string }> = {
    'online': { label: 'Online', color: '#22c55e', icon: 'check_circle' },
    'degraded': { label: 'Degraded', color: '#f59e0b', icon: 'warning' },
    'offline': { label: 'Offline', color: '#ef4444', icon: 'error' },
    'unknown': { label: 'Unknown', color: '#6b7280', icon: 'help' },
  };
  return displays[status] ?? displays['unknown'];
}

/**
 * Get feature display info.
 */
export function getFeatureDisplay(feature: DoorwayFeature): { label: string; icon: string } {
  const displays: Record<DoorwayFeature, { label: string; icon: string }> = {
    'premium-content': { label: 'Premium Content', icon: 'star' },
    'high-availability': { label: 'High Availability', icon: 'verified' },
    'media-hosting': { label: 'Media Hosting', icon: 'cloud' },
    'recovery-service': { label: 'Recovery Support', icon: 'restore' },
    'analytics': { label: 'Analytics', icon: 'analytics' },
    'custom-domain': { label: 'Custom Domain', icon: 'language' },
  };
  return displays[feature] ?? { label: feature, icon: 'extension' };
}

/**
 * Sort doorways by relevance.
 * Prioritizes: online status, registration open, vouch count.
 */
export function sortDoorwaysByRelevance(doorways: DoorwayInfo[]): DoorwayInfo[] {
  return [...doorways].sort((a, b) => {
    // Online first
    const statusOrder: Record<DoorwayStatus, number> = {
      'online': 0,
      'degraded': 1,
      'offline': 2,
      'unknown': 3,
    };
    const statusDiff = statusOrder[a.status] - statusOrder[b.status];
    if (statusDiff !== 0) return statusDiff;

    // Registration open first
    if (a.registrationOpen !== b.registrationOpen) {
      return a.registrationOpen ? -1 : 1;
    }

    // Higher vouch count first
    const vouchDiff = (b.vouchCount ?? 0) - (a.vouchCount ?? 0);
    if (vouchDiff !== 0) return vouchDiff;

    // Alphabetical fallback
    return a.name.localeCompare(b.name);
  });
}
