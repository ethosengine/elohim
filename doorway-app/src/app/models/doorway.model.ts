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
export type NodeStatus = 'discovered' | 'registering' | 'online' | 'degraded' | 'offline' | 'failed';

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
  | { type: 'node_update'; timestamp: string; nodeId: string; status: string; combinedScore: number; changes: string[] }
  | { type: 'cluster_update'; timestamp: string; onlineNodes: number; totalNodes: number; healthRatio: number; avgTrustScore: number; avgImpactScore: number }
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
  const names = ['Private', 'Invited', 'Local', 'Neighborhood', 'Municipal', 'Bioregional', 'Regional', 'Commons'];
  return names[level] ?? `Level ${level}`;
}

/**
 * Get color class for node status
 */
export function statusColor(status: NodeStatus): string {
  switch (status) {
    case 'online': return 'text-green-600';
    case 'degraded': return 'text-yellow-600';
    case 'offline': return 'text-gray-500';
    case 'failed': return 'text-red-600';
    case 'discovered': return 'text-blue-400';
    case 'registering': return 'text-blue-600';
    default: return 'text-gray-400';
  }
}

/**
 * Get color class for steward tier
 */
export function tierColor(tier: StewardTier | null): string {
  switch (tier) {
    case 'pioneer': return 'text-purple-600';
    case 'steward': return 'text-indigo-600';
    case 'guardian': return 'text-blue-600';
    case 'caretaker': return 'text-green-600';
    default: return 'text-gray-500';
  }
}

// ============================================================================
// Pipeline Models
// ============================================================================

/**
 * Agency pipeline stage counts
 */
export interface PipelineResponse {
  registered: number;
  hosted: number;
  graduating: number;
  steward: number;
}

/**
 * Pipeline stage for display
 */
export type PipelineStage = 'registered' | 'hosted' | 'graduating' | 'steward';

// ============================================================================
// Federation Models (Admin)
// ============================================================================

/**
 * Federated doorway details for admin dashboard
 */
export interface FederatedDoorway {
  id: string;
  name: string;
  url: string;
  region: string | null;
  status: string;
  latencyMs: number | null;
  humansServed: number;
  contentAvailable: number;
  capabilities: string[];
  isSelf: boolean;
}

/**
 * P2P peer connection details
 */
export interface P2PPeer {
  peerId: string;
  address: string;
  connectionState: string;
  latencyMs: number | null;
  connectedSince: string | null;
  bytesSent: number;
  bytesReceived: number;
}

/**
 * Federation doorways response (admin)
 */
export interface FederationDoorwaysAdminResponse {
  doorways: FederatedDoorway[];
  total: number;
}

/**
 * P2P peers response
 */
export interface P2PPeersResponse {
  peers: P2PPeer[];
  total: number;
}

// ============================================================================
// Federation Peer Config Models (Admin)
// ============================================================================

/**
 * Configured federation peer with enriched status
 */
export interface FederationPeerConfig {
  url: string;
  reachable: boolean;
  doorwayId: string | null;
  region: string | null;
  capabilities: string[];
}

/**
 * Response from GET /admin/federation/peers
 */
export interface FederationPeersConfigResponse {
  peers: FederationPeerConfig[];
  total: number;
  selfId: string | null;
}

// ============================================================================
// Graduation Models
// ============================================================================

/**
 * User in graduation pipeline
 */
export interface GraduationUser {
  id: string;
  identifier: string;
  hasExportedKey: boolean;
  hasLocalConductor: boolean;
  isSteward: boolean;
  keyExportedAt: string | null;
  graduatedAt: string | null;
  createdAt: string | null;
}

/**
 * Graduation pending response
 */
export interface GraduationPendingResponse {
  users: GraduationUser[];
  total: number;
}

/**
 * Graduation completed response
 */
export interface GraduationCompletedResponse {
  users: GraduationUser[];
  total: number;
}

// ============================================================================
// Capabilities Models
// ============================================================================

/**
 * Server capabilities from GET /admin/capabilities
 */
export interface CapabilitiesResponse {
  orchestrator: boolean;
  federation: boolean;
  conductorPool: boolean;
  nats: boolean;
}

// ============================================================================
// Account Models
// ============================================================================

/**
 * Agency pipeline step for self-service view
 */
export type AgencyStep = 'hosted' | 'key_export' | 'install_app' | 'steward';

/**
 * Account response from GET /auth/account
 */
export interface AccountResponse {
  id: string;
  identifier: string;
  identifierType: string;
  permissionLevel: UserPermissionLevel;
  isActive: boolean;
  isSteward: boolean;
  hasLocalConductor: boolean;
  hasExportedKey: boolean;
  createdAt: string | null;
  lastLoginAt: string | null;
  doorwayName: string;
  doorwayRegion: string | null;
  usage: UserUsage;
  quota: UserQuota;
}

// ============================================================================
// User Admin Models
// ============================================================================

/**
 * Permission levels for users
 */
export type UserPermissionLevel = 'PUBLIC' | 'AUTHENTICATED' | 'ADMIN';

/**
 * Usage tracking for hosted users
 */
export interface UserUsage {
  storageBytes: number;
  storageMb: number;
  projectionQueries: number;
  bandwidthBytes: number;
  bandwidthMb: number;
  periodStart: string | null;
  lastUpdated: string | null;
}

/**
 * Quota limits and status
 */
export interface UserQuota {
  storageLimitBytes: number;
  storageLimitMb: number;
  storagePercentUsed: number;
  dailyQueryLimit: number;
  queriesPercentUsed: number;
  dailyBandwidthLimitBytes: number;
  dailyBandwidthLimitMb: number;
  bandwidthPercentUsed: number;
  enforceHardLimit: boolean;
  isOverQuota: boolean;
}

/**
 * User summary for list view
 */
export interface UserSummary {
  id: string;
  identifier: string;
  identifierType: string;
  permissionLevel: UserPermissionLevel;
  isActive: boolean;
  createdAt: string | null;
  lastLoginAt: string | null;
  storageUsedMb: number;
  storageLimitMb: number;
  storagePercent: number;
  isOverQuota: boolean;
}

/**
 * Full user details with usage and quota
 */
export interface UserDetails {
  id: string;
  identifier: string;
  identifierType: string;
  humanId: string;
  agentPubKey: string;
  permissionLevel: UserPermissionLevel;
  isActive: boolean;
  tokenVersion: number;
  createdAt: string | null;
  updatedAt: string | null;
  lastLoginAt: string | null;
  usage: UserUsage;
  quota: UserQuota;
}

/**
 * Paginated users response
 */
export interface UsersResponse {
  users: UserSummary[];
  total: number;
  page: number;
  limit: number;
  totalPages: number;
}

/**
 * Quota status from enforcement check
 */
export interface QuotaStatus {
  allowed: boolean;
  storageExceeded: boolean;
  queriesExceeded: boolean;
  bandwidthExceeded: boolean;
  message: string | null;
}

/**
 * Parameters for listing users
 */
export interface ListUsersParams {
  page?: number;
  limit?: number;
  search?: string;
  permissionLevel?: UserPermissionLevel;
  isActive?: boolean;
  overQuota?: boolean;
  sortBy?: string;
  sortDir?: 'asc' | 'desc';
}

/**
 * Update quota request
 */
export interface UpdateQuotaRequest {
  storageLimitMb?: number;
  dailyQueryLimit?: number;
  dailyBandwidthLimitMb?: number;
  enforceHardLimit?: boolean;
}

/**
 * Success response for mutations
 */
export interface UserMutationResponse {
  success: boolean;
  message: string;
}

// ============================================================================
// User Admin Helpers
// ============================================================================

/**
 * Get color class for permission level
 */
export function permissionLevelColor(level: UserPermissionLevel): string {
  switch (level) {
    case 'ADMIN': return 'text-purple-600';
    case 'AUTHENTICATED': return 'text-blue-600';
    case 'PUBLIC': return 'text-gray-500';
    default: return 'text-gray-400';
  }
}

/**
 * Get display name for permission level
 */
export function permissionLevelName(level: UserPermissionLevel): string {
  switch (level) {
    case 'ADMIN': return 'Admin';
    case 'AUTHENTICATED': return 'Authenticated';
    case 'PUBLIC': return 'Public';
    default: return level;
  }
}

/**
 * Get color class for quota status (usage percentage)
 */
export function quotaStatusColor(percent: number): string {
  if (percent >= 100) return 'text-red-600';
  if (percent >= 80) return 'text-yellow-600';
  if (percent >= 50) return 'text-blue-600';
  return 'text-green-600';
}

/**
 * Format bytes as human-readable string
 */
export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

/**
 * Get color for pipeline stage
 */
export function pipelineStageColor(stage: PipelineStage): string {
  switch (stage) {
    case 'registered': return '#6b7280';
    case 'hosted': return '#3b82f6';
    case 'graduating': return '#f59e0b';
    case 'steward': return '#10b981';
    default: return '#9ca3af';
  }
}

/**
 * Get display name for pipeline stage
 */
export function pipelineStageName(stage: PipelineStage): string {
  switch (stage) {
    case 'registered': return 'Registered';
    case 'hosted': return 'Hosted';
    case 'graduating': return 'Graduating';
    case 'steward': return 'Steward';
    default: return stage;
  }
}

/**
 * Get color for connection state
 */
export function connectionStateColor(state: string): string {
  switch (state) {
    case 'connected': return 'text-green-600';
    case 'connecting': return 'text-yellow-600';
    case 'disconnected': return 'text-gray-500';
    default: return 'text-gray-400';
  }
}

/**
 * Get quota gauge color based on usage percentage
 */
export function quotaGaugeColor(percent: number): string {
  if (percent >= 90) return '#ef4444';
  if (percent >= 70) return '#f59e0b';
  return '#10b981';
}
