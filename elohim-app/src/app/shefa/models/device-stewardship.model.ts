/**
 * Device Stewardship Model
 *
 * Unified device model covering both app-steward (personal devices running Tauri)
 * and node-steward (infrastructure nodes like HoloPorts) devices.
 *
 * Status mapping from NodeClusterStatus:
 * - online -> connected
 * - degraded / maintenance / provisioning -> seen
 * - offline / unknown -> offline
 */

import type { OwnedNode, NodeClusterStatus, NodeRole } from './shefa-dashboard.model';

// =============================================================================
// Types
// =============================================================================

export type DeviceCategory = 'app-steward' | 'node-steward';
export type DeviceStatus = 'connected' | 'seen' | 'offline';
export type DevicePlatform =
  | 'desktop-macos'
  | 'desktop-windows'
  | 'desktop-linux'
  | 'mobile-ios'
  | 'mobile-android'
  | 'unknown';

// =============================================================================
// Interfaces
// =============================================================================

export interface StewardedDevice {
  deviceId: string; // agentPubKey for app devices, nodeId for nodes
  displayName: string;
  category: DeviceCategory;
  status: DeviceStatus;
  lastSeen: string; // ISO 8601
  isCurrentDevice: boolean;

  // App steward metadata
  platform?: DevicePlatform;
  appVersion?: string;
  doorwayUrl?: string; // Which doorway this device connected through

  // Node steward metadata (from existing OwnedNode)
  nodeType?: OwnedNode['nodeType'];
  location?: OwnedNode['location'];
  roles?: NodeRole[];
  resources?: OwnedNode['resources'];
  isPrimaryNode?: boolean;
}

export interface DeviceStewardshipState {
  devices: StewardedDevice[];
  currentDevice: StewardedDevice | null;
  appStewardDevices: StewardedDevice[];
  nodeStewardDevices: StewardedDevice[];
  totalDevices: number;
  connectedCount: number;
  seenCount: number;
  offlineCount: number;
  isLoading: boolean;
  error: string | null;
  lastUpdated: string;
}

// =============================================================================
// Status Mapping
// =============================================================================

/**
 * Map NodeClusterStatus to DeviceStatus.
 */
export function mapNodeClusterStatusToDeviceStatus(status: NodeClusterStatus): DeviceStatus {
  switch (status) {
    case 'online':
      return 'connected';
    case 'degraded':
    case 'maintenance':
    case 'provisioning':
      return 'seen';
    case 'offline':
    case 'unknown':
    default:
      return 'offline';
  }
}

// =============================================================================
// Display Helpers
// =============================================================================

export function getDeviceStatusDisplay(status: DeviceStatus): {
  label: string;
  color: string;
  icon: string;
} {
  const displays: Record<DeviceStatus, { label: string; color: string; icon: string }> = {
    connected: { label: 'Connected', color: '#22c55e', icon: 'check_circle' },
    seen: { label: 'Seen', color: '#f59e0b', icon: 'schedule' },
    offline: { label: 'Offline', color: '#ef4444', icon: 'cloud_off' },
  };
  return displays[status] ?? displays['offline'];
}

export function getDeviceCategoryDisplay(category: DeviceCategory): {
  label: string;
  icon: string;
} {
  const displays: Record<DeviceCategory, { label: string; icon: string }> = {
    'app-steward': { label: 'Personal Device', icon: 'smartphone' },
    'node-steward': { label: 'Infrastructure Node', icon: 'dns' },
  };
  return displays[category] ?? displays['app-steward'];
}

export function getDevicePlatformDisplay(platform: DevicePlatform): {
  label: string;
  icon: string;
} {
  const displays: Record<DevicePlatform, { label: string; icon: string }> = {
    'desktop-macos': { label: 'macOS', icon: 'laptop_mac' },
    'desktop-windows': { label: 'Windows', icon: 'laptop_windows' },
    'desktop-linux': { label: 'Linux', icon: 'computer' },
    'mobile-ios': { label: 'iOS', icon: 'phone_iphone' },
    'mobile-android': { label: 'Android', icon: 'phone_android' },
    unknown: { label: 'Unknown', icon: 'devices' },
  };
  return displays[platform] ?? displays['unknown'];
}

/**
 * Detect platform from user agent string.
 */
export function detectPlatform(userAgent: string): DevicePlatform {
  const ua = userAgent.toLowerCase();
  if (ua.includes('mac')) return 'desktop-macos';
  if (ua.includes('windows') || ua.includes('win64') || ua.includes('win32'))
    return 'desktop-windows';
  if (ua.includes('linux') && !ua.includes('android')) return 'desktop-linux';
  if (ua.includes('iphone') || ua.includes('ipad')) return 'mobile-ios';
  if (ua.includes('android')) return 'mobile-android';
  return 'unknown';
}

/**
 * Build an initial empty state.
 */
export function createEmptyDeviceStewardshipState(): DeviceStewardshipState {
  return {
    devices: [],
    currentDevice: null,
    appStewardDevices: [],
    nodeStewardDevices: [],
    totalDevices: 0,
    connectedCount: 0,
    seenCount: 0,
    offlineCount: 0,
    isLoading: true,
    error: null,
    lastUpdated: new Date().toISOString(),
  };
}
