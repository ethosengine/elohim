/**
 * Doorway Admin API Models
 *
 * Types for the doorway operator dashboard, reflecting the
 * compute resources and human-scale metrics exposed by the
 * /admin/* endpoints.
 */

// ============================================================================
// Node Models
// ============================================================================

/**
 * Health status of a node
 */
export type NodeStatus =
  | 'discovered'
  | 'registering'
  | 'online'
  | 'degraded'
  | 'offline'
  | 'failed';

/**
 * Steward tier levels
 */
export type StewardTier = 'caretaker' | 'guardian' | 'steward' | 'pioneer';

/**
 * Detailed node information
 */
export interface NodeDetails {
  nodeId: string;
  status: NodeStatus;
  natsProvisioned: boolean;
  lastHeartbeatSecsAgo: number | null;

  // Technical resource metrics
  cpuCores: number | null;
  memoryGb: number | null;
  storageTb: number | null;
  bandwidthMbps: number | null;
  cpuUsagePercent: number | null;
  memoryUsagePercent: number | null;
  storageUsageTb: number | null;
  activeConnections: number | null;
  custodiedContentGb: number | null;

  // Human-scale metrics
  stewardTier: StewardTier | null;
  maxReachLevel: number | null;
  activeReachLevels: number[] | null;
  trustScore: number | null;
  humansServed: number | null;
  contentCustodied: number | null;
  successfulDeliveries: number | null;
  failedDeliveries: number | null;
  deliverySuccessRate: number | null;
  impactScore: number | null;
  combinedScore: number;

  // Location
  region: string | null;
}

/**
 * Node counts by status
 */
export interface NodeStatusCounts {
  online: number;
  degraded: number;
  offline: number;
  failed: number;
  discovering: number;
  registering: number;
}

/**
 * Response from GET /admin/nodes
 */
export interface NodesResponse {
  total: number;
  byStatus: NodeStatusCounts;
  nodes: NodeDetails[];
}

// ============================================================================
// Cluster Models
// ============================================================================

/**
 * Steward tier distribution
 */
export interface StewardCounts {
  pioneers: number;
  stewards: number;
  guardians: number;
  caretakers: number;
}

/**
 * Reach level coverage
 */
export interface ReachCoverage {
  private: number;
  invited: number;
  local: number;
  neighborhood: number;
  municipal: number;
  bioregional: number;
  regional: number;
  commons: number;
}

/**
 * Cluster-wide metrics
 */
export interface ClusterMetrics {
  region: string;
  totalNodes: number;
  onlineNodes: number;
  healthRatio: number;

  // Aggregate capacity
  totalCpuCores: number;
  totalMemoryGb: number;
  totalStorageTb: number;
  totalBandwidthMbps: number;

  // Aggregate usage
  avgCpuUsagePercent: number;
  avgMemoryUsagePercent: number;
  totalStorageUsedTb: number;
  totalActiveConnections: number;
  totalCustodiedContentGb: number;

  // Human-scale aggregates
  avgTrustScore: number;
  avgImpactScore: number;
  totalHumansServed: number;
  totalContentCustodied: number;
  clusterDeliverySuccessRate: number;

  // Distributions
  stewardCounts: StewardCounts;
  reachCoverage: ReachCoverage;
}

// ============================================================================
// Resource Models
// ============================================================================

/**
 * Resource utilization
 */
export interface ResourceUtilization {
  total: number;
  used: number;
  available: number;
  utilizationPercent: number;
}

/**
 * Storage utilization
 */
export interface StorageUtilization {
  totalTb: number;
  usedTb: number;
  availableTb: number;
  utilizationPercent: number;
  custodiedContentGb: number;
}

/**
 * Bandwidth utilization
 */
export interface BandwidthUtilization {
  totalMbps: number;
  activeConnections: number;
  avgBandwidthPerConnectionMbps: number;
}

/**
 * Cache performance
 */
export interface CachePerformance {
  entries: number;
  hits: number;
  misses: number;
  hitRate: number;
}

/**
 * Resource summary
 */
export interface ResourceSummary {
  cpu: ResourceUtilization;
  memory: ResourceUtilization;
  storage: StorageUtilization;
  bandwidth: BandwidthUtilization;
  cache: CachePerformance;
}

// ============================================================================
// Custodian Models
// ============================================================================

/**
 * Custodian network overview
 */
export interface CustodianNetwork {
  registeredCustodians: number;
  trackedBlobs: number;
  totalCommitments: number;
  totalProbes: number;
  successfulProbes: number;
  probeSuccessRate: number;
  totalSelections: number;
  healthyCustodians: number;
}

// ============================================================================
// WebSocket Models
// ============================================================================

/**
 * Node snapshot for WebSocket
 */
export interface NodeSnapshot {
  nodeId: string;
  status: string;
  combinedScore: number;
  stewardTier: string | null;
  trustScore: number | null;
  lastHeartbeatSecsAgo: number | null;
}

/**
 * Cluster snapshot for WebSocket
 */
export interface ClusterSnapshot {
  onlineNodes: number;
  totalNodes: number;
  healthRatio: number;
  avgTrustScore: number;
  avgImpactScore: number;
}

/**
 * Dashboard WebSocket messages
 */
export type DashboardMessage =
  | { type: 'initial_state'; timestamp: string; nodes: NodeSnapshot[]; cluster: ClusterSnapshot }
  | {
      type: 'node_update';
      timestamp: string;
      nodeId: string;
      status: string;
      combinedScore: number;
      changes: string[];
    }
  | {
      type: 'cluster_update';
      timestamp: string;
      onlineNodes: number;
      totalNodes: number;
      healthRatio: number;
      avgTrustScore: number;
      avgImpactScore: number;
    }
  | { type: 'heartbeat'; timestamp: string; intervalSecs: number }
  | { type: 'pong'; timestamp: string }
  | { type: 'error'; message: string };

/**
 * Client WebSocket messages
 */
export type ClientMessage =
  | { type: 'subscribe'; topics?: string[] }
  | { type: 'unsubscribe'; topics?: string[] }
  | { type: 'ping' };

// ============================================================================
// Helpers
// ============================================================================

/**
 * Get display name for reach level
 */
export function reachLevelName(level: number): string {
  const names = [
    'Private',
    'Invited',
    'Local',
    'Neighborhood',
    'Municipal',
    'Bioregional',
    'Regional',
    'Commons',
  ];
  return names[level] ?? `Level ${level}`;
}

/**
 * Get color class for node status
 */
export function statusColor(status: NodeStatus): string {
  switch (status) {
    case 'online':
      return 'text-green-600';
    case 'degraded':
      return 'text-yellow-600';
    case 'offline':
      return 'text-gray-500';
    case 'failed':
      return 'text-red-600';
    case 'discovered':
      return 'text-blue-400';
    case 'registering':
      return 'text-blue-600';
    default:
      return 'text-gray-400';
  }
}

/**
 * Get color class for steward tier
 */
export function tierColor(tier: StewardTier | null): string {
  switch (tier) {
    case 'pioneer':
      return 'text-purple-600';
    case 'steward':
      return 'text-indigo-600';
    case 'guardian':
      return 'text-blue-600';
    case 'caretaker':
      return 'text-green-600';
    default:
      return 'text-gray-500';
  }
}
