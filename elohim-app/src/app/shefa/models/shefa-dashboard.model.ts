/**
 * Shefa Dashboard Models
 *
 * Comprehensive data structures for operator visibility into:
 * - Compute node status and metrics (CPU, RAM, disk, network)
 * - Digital footprint replication (where data is protected)
 * - Family-community protection (who's protecting my data)
 * - Infrastructure-token economics (how much compute I'm earning)
 * - Constitutional limits enforcement (dignity floor/ceiling)
 * - hREA economic events (compute flows)
 *
 * Integration Points:
 * - Cache system (Doorway gateway) for real-time metrics
 * - Holochain DHT for CustodianCommitment + StewardedResource
 * - EconomicEvent ledger for infrastructure-token tracking
 * - NATS heartbeats for uptime/health
 */

import { ResourceMeasure } from './stewarded-resources.model';

// =============================================================================
// CORE DASHBOARD STATE
// =============================================================================

/**
 * SheafaDashboardState - Complete state for operator dashboard
 *
 * Aggregates: compute metrics + family-community protection + economics
 */
export interface SheafaDashboardState {
  // Identity
  operatorId: string; // AgentPubKey
  operatorName: string;
  stewardedResourceId: string; // FK to StewardedResource entry

  // Node identity
  nodeId: string; // Holochain conductor public key
  nodeLocation?: {
    region: string; // e.g., "us-west-2"
    country: string; // e.g., "USA"
    latitude?: number;
    longitude?: number;
  };

  // Status
  status: 'online' | 'offline' | 'degraded' | 'maintenance';
  lastHeartbeat: string; // ISO 8601 timestamp
  uptime: UpTimeMetrics;

  // Compute resources
  computeMetrics: ComputeMetrics;
  allocations: AllocationSnapshot; // What's allocated to family-community

  // Family-community protection
  familyCommunityProtection: FamilyCommunityProtectionStatus;

  // Economic
  infrastructureTokens: InfrastructureTokenBalance;
  economicEvents: RecentEconomicEvent[]; // Last 20 compute events

  // Constitutional enforcement
  constitutionalLimits: ConstitutionalLimitsStatus;

  // Real-time updates
  lastUpdated: string; // ISO 8601 timestamp
  updateFrequency: number; // Milliseconds between updates
}

// =============================================================================
// COMPUTE METRICS
// =============================================================================

/**
 * ComputeMetrics - Real-time node performance and capacity
 *
 * Source: Holochain conductor metrics + Doorway cache metrics
 */
export interface ComputeMetrics {
  // CPU
  cpu: {
    totalCores: number;
    available: number;
    usagePercent: number; // 0-100
    usageHistory: MetricHistory[]; // Last 24 hours, 5-minute intervals
    temperature?: number; // Celsius (if available)
  };

  // Memory
  memory: {
    totalGB: number;
    usedGB: number;
    availableGB: number;
    usagePercent: number; // 0-100
    usageHistory: MetricHistory[];
  };

  // Storage
  storage: {
    totalGB: number;
    usedGB: number;
    availableGB: number;
    usagePercent: number; // 0-100
    usageHistory: MetricHistory[];
    breakdown: {
      holochain: number; // GB used by Holochain DHT
      cache: number; // GB used by cache layer
      custodianData: number; // GB of replicated data
      userApplications: number; // GB used by user apps
    };
  };

  // Network
  network: {
    bandwidth: {
      upstreamMbps: number; // Maximum upstream capacity
      downstreamMbps: number; // Maximum downstream capacity
      usedUpstreamMbps: number; // Current usage
      usedDownstreamMbps: number; // Current usage
    };
    latency: {
      p50: number; // Milliseconds
      p95: number;
      p99: number;
    };
    connections: {
      total: number; // Total active connections
      holochain: number; // DHT peers
      cache: number; // Cache clients
      custodian: number; // Custodian replication streams
    };
  };

  // Process/Application load
  loadAverage: {
    oneMinute: number;
    fiveMinutes: number;
    fifteenMinutes: number;
  };

  // Power (if available)
  power?: {
    consumptionWatts: number;
    thermalOutput: number;
  };
}

/**
 * MetricHistory - Time-series data point
 */
export interface MetricHistory {
  timestamp: string; // ISO 8601
  value: number; // Percentage or absolute value
}

/**
 * UpTimeMetrics - Node availability tracking
 */
export interface UpTimeMetrics {
  upPercent: number; // 0-100 (last 24 hours)
  downtime: {
    hours24: number; // Hours down in last 24h
    hours7d: number; // Hours down in last 7d
    hours30d: number; // Hours down in last 30d
  };
  lastFailure?: string; // ISO 8601 timestamp
  consecutiveUptime: string; // Human-readable duration
  reliability: 'excellent' | 'good' | 'fair' | 'poor'; // Based on SLA
}

// =============================================================================
// COMPUTE ALLOCATION (to family-community)
// =============================================================================

/**
 * AllocationSnapshot - How much compute is allocated to family-community
 *
 * Source: StewardedResource.allocations
 */
export interface AllocationSnapshot {
  // Total allocations by governance level
  byGovernanceLevel: {
    individual: {
      cpuPercent: number;
      memoryPercent: number;
      storagePercent: number;
      bandwidthPercent: number;
    };
    household: {
      cpuPercent: number;
      memoryPercent: number;
      storagePercent: number;
      bandwidthPercent: number;
    };
    community: {
      cpuPercent: number;
      memoryPercent: number;
      storagePercent: number;
      bandwidthPercent: number;
    };
    network: {
      cpuPercent: number;
      memoryPercent: number;
      storagePercent: number;
      bandwidthPercent: number;
    };
  };

  // Summary
  totalAllocated: {
    cpuPercent: number;
    memoryPercent: number;
    storagePercent: number;
    bandwidthPercent: number;
  };

  // Detailed allocation blocks
  allocationBlocks: AllocationBlock[];
}

/**
 * AllocationBlock - A specific allocation for a purpose
 *
 * E.g., "10% CPU for Lamad family learning"
 */
export interface AllocationBlock {
  id: string;
  label: string; // "Lamad family learning"
  governanceLevel: 'individual' | 'household' | 'community' | 'network';
  priority: number; // 1-10

  // Allocation
  cpu: {
    cores: number;
    percent: number;
  };
  memory: {
    gb: number;
    percent: number;
  };
  storage: {
    gb: number;
    percent: number;
  };
  bandwidth: {
    mbps: number;
    percent: number;
  };

  // Current utilization
  utilized: {
    cpuPercent: number;
    memoryPercent: number;
    storagePercent: number;
    bandwidthPercent: number;
  };

  // Linked commitment
  commitmentId?: string; // FK to Commitment entry
  relatedAgents?: string[]; // Who benefits from this allocation
}

// =============================================================================
// FAMILY-COMMUNITY PROTECTION STATUS
// =============================================================================

/**
 * FamilyCommunityProtectionStatus - Where data is replicated and who protects it
 *
 * Source: CustodianCommitment entries
 */
export interface FamilyCommunityProtectionStatus {
  // Redundancy model
  redundancy: {
    strategy: 'full_replica' | 'threshold_split' | 'erasure_coded';
    redundancyFactor: number; // M for threshold (e.g., 2-of-3 = factor 2)
    recoveryThreshold: number; // Minimum shards needed to recover
  };

  // Custodian network
  custodians: CustodianNode[];
  totalCustodians: number;

  // Geographic distribution
  geographicDistribution: {
    regions: RegionalPresence[];
    riskProfile: 'centralized' | 'distributed' | 'geo-redundant';
  };

  // Trust graph
  trustGraph: TrustRelationship[];

  // Overall protection status
  protectionLevel: 'vulnerable' | 'protected' | 'highly-protected';
  estimatedRecoveryTime: string; // Human-readable (e.g., "< 1 hour")
  lastVerification: string; // ISO 8601 timestamp
  verificationStatus: 'verified' | 'pending' | 'failed';
}

/**
 * CustodianNode - A node protecting my data
 *
 * Represents a CustodianCommitment + node status
 */
export interface CustodianNode {
  id: string; // FK to Agent
  name: string;
  type: 'family' | 'friend' | 'community' | 'professional' | 'institution';

  // Location
  location?: {
    region: string;
    country: string;
  };

  // Storage
  dataStored: {
    totalGB: number;
    shardCount: number;
    redundancyLevel: number;
  };

  // Health
  health: {
    upPercent: number;
    lastHeartbeat: string;
    responseTime: number; // Milliseconds
  };

  // Commitment details
  commitment: {
    id: string; // FK to CustodianCommitment
    status: 'active' | 'pending' | 'breached' | 'expired';
    startDate: string;
    expiryDate: string;
    renewalStatus: 'auto-renew' | 'manual' | 'expired';
  };

  // Relationship
  trustLevel: number; // 0-100
  relationship: string; // Type of relationship
}

/**
 * RegionalPresence - Distribution in a geographic region
 */
export interface RegionalPresence {
  region: string; // "us-west-2", "eu-central-1", etc.
  custodianCount: number;
  dataShards: number;
  redundancy: number; // How many independent copies
  riskFactors: string[]; // e.g., ["single-isp", "geographic-clustering"]
}

/**
 * TrustRelationship - Connection in trust graph
 *
 * Shows family-community relationships
 */
export interface TrustRelationship {
  from: string; // My agent ID
  to: string; // Custodian agent ID
  type:
    | 'family-member'
    | 'friend'
    | 'community-peer'
    | 'professional'
    | 'institution';
  trustScore: number; // 0-100
  depth: number; // Hops in trust graph (1 = direct)
  strength: 'weak' | 'moderate' | 'strong';
}

// =============================================================================
// INFRASTRUCTURE TOKEN ECONOMICS
// =============================================================================

/**
 * InfrastructureTokenBalance - Earnings from providing compute
 *
 * Tracks infrastructure-tokens earned and their value
 */
export interface InfrastructureTokenBalance {
  // Current balance
  balance: {
    tokens: number; // Actual token count
    estimatedValue: {
      value: number;
      currency: string; // "USD", "HoloFuel", etc.
    };
  };

  // Earning rate
  earningRate: {
    tokensPerHour: number;
    basedOn: {
      cpuAllocation: number; // % of CPU allocated
      storageAllocation: number; // % of storage
      bandwidthAllocation: number; // % of bandwidth
    };
    estimatedMonthly: number; // Projected earnings if consistent
  };

  // Token decay (demurrage)
  decay: {
    demurrageRate: number; // % per month (0.5 = 0.5% per month)
    lastCalculated: string;
    projectedNextMonth: {
      tokens: number;
      valueUSD: number;
    };
  };

  // History
  transactions: TokenTransaction[];
  tokenHistory: {
    last24Hours: number;
    last7Days: number;
    last30Days: number;
    allTime: number; // Total earned since genesis
  };

  // Cross-swimlane exchange
  exchangeRates: ExchangeRate[];
}

/**
 * TokenTransaction - Earnings or spending event
 */
export interface TokenTransaction {
  id: string;
  timestamp: string;
  type: 'earned' | 'transferred' | 'exchanged' | 'decayed' | 'claimed';
  amount: number;
  relatedAgent?: string;
  description: string;
  economicEventId?: string; // FK to EconomicEvent
}

/**
 * ExchangeRate - Cross-swimlane token conversion
 *
 * E.g., infrastructure-tokens → care-tokens
 */
export interface ExchangeRate {
  from: string; // infrastructure
  to: string; // care | time | learning | steward | creator
  rate: number; // 1 infrastructure-token = X care-tokens
  source: 'market' | 'consensus' | 'algorithm';
  lastUpdated: string;
}

// =============================================================================
// ECONOMIC EVENTS - COMPUTE FLOWS
// =============================================================================

/**
 * RecentEconomicEvent - Compute-related hREA event
 *
 * Shows economic flows: who got what, when
 */
export interface RecentEconomicEvent {
  id: string; // FK to EconomicEvent
  timestamp: string;
  eventType:
    | 'cpu-hours-provided'
    | 'storage-provided'
    | 'bandwidth-provided'
    | 'compute-consumed'
    | 'infrastructure-token-issued'
    | 'token-transferred'
    | 'commitment-fulfilled';

  // REA structure
  provider?: string; // Who provided (my node)
  receiver?: string; // Who received (family-community)
  quantity: ResourceMeasure; // CPU hours, GB, Mbps, tokens

  // Economic value
  tokensMinted?: number; // Infrastructure-tokens generated
  note: string; // Human-readable description
}

// =============================================================================
// CONSTITUTIONAL LIMITS ENFORCEMENT
// =============================================================================

/**
 * ConstitutionalLimitsStatus - Dignity floor and ceiling enforcement
 *
 * Prevents both starvation and extraction
 */
export interface ConstitutionalLimitsStatus {
  // Dignity floor (minimum entitlement)
  dignityFloor: {
    computeMinCores: number;
    computeMinMemoryGB: number;
    computeMinStorageGB: number;
    computeMinBandwidthMbps: number;

    status: 'met' | 'warning' | 'breached';
    percentOfFloor: number; // How much of minimum we have (should be >100%)
    enforcement: 'voluntary' | 'progressive' | 'hard';
  };

  // Ceiling limit (wise stewardship bound)
  ceilingLimit: {
    computeMaxCores: number;
    computeMaxMemoryGB: number;
    computeMaxStorageGB: number;
    computeMaxBandwidthMbps: number;

    // Token accumulation ceiling
    tokenAccumulationCeiling: number; // Max tokens before forced circulation
    currentAccumulation: number;
    percentOfCeiling: number; // How close to max (should be <100%)

    status: 'safe' | 'warning' | 'breached';
    enforcement: 'voluntary' | 'progressive' | 'hard';
  };

  // Safe operating zone
  safeZone: {
    cpu: number; // % between floor and ceiling
    memory: number;
    storage: number;
    bandwidth: number;
    tokens: number;
  };

  // Alerts
  alerts: ConstitutionalAlert[];
}

/**
 * ConstitutionalAlert - Limit violation warning
 */
export interface ConstitutionalAlert {
  id: string;
  severity: 'info' | 'warning' | 'critical';
  type: 'floor-breach' | 'ceiling-breach' | 'redistribution-required';
  message: string;
  affectedResource: string;
  currentValue: number;
  threshold: number;
  recommendedAction: string;
  timestamp: string;
}

// =============================================================================
// NODE TOPOLOGY - Cluster-wide view of all user's nodes
// =============================================================================

/**
 * NodeTopologyState - Overview of all nodes owned/managed by user
 *
 * This is the "everyday user" view showing:
 * - All nodes in user's cluster
 * - Status of each (online/offline/degraded)
 * - What each node is doing
 * - Overall cluster health
 */
export interface NodeTopologyState {
  // All nodes the user owns/manages
  nodes: OwnedNode[];

  // Summary stats
  totalNodes: number;
  onlineNodes: number;
  offlineNodes: number;
  degradedNodes: number;

  // Primary family node (if designated)
  primaryNode?: {
    nodeId: string;
    status: NodeClusterStatus;
    isOnline: boolean;
  };

  // Overall cluster health
  clusterHealth: 'healthy' | 'degraded' | 'critical' | 'offline';

  // Active alerts
  alerts: OfflineNodeAlert[];

  // Last topology refresh
  lastUpdated: string; // ISO 8601
}

/**
 * OwnedNode - A single node in user's cluster
 */
export interface OwnedNode {
  // Identity
  nodeId: string;
  displayName: string; // User-friendly name, e.g., "Living Room Holoport"
  nodeType: 'holoport' | 'holoport-plus' | 'holoport-nano' | 'self-hosted' | 'cloud';

  // Status
  status: NodeClusterStatus;
  lastHeartbeat: string; // ISO 8601
  consecutiveUptime: string; // Human-readable, e.g., "14 days"

  // Location
  location?: {
    label: string; // "Home Office", "Basement Rack"
    region: string; // Geographic region
    country: string;
  };

  // What this node is doing
  roles: NodeRole[];

  // Resource summary
  resources: {
    cpuPercent: number; // Current usage
    memoryPercent: number;
    storageUsedGB: number;
    storageTotalGB: number;
    bandwidthMbps: number;
  };

  // Custodian activity on this node
  custodianActivity: {
    contentItemsCustodied: number; // Content I'm storing for others
    contentItemsBeingCustodied: number; // My content others store on this node's behalf
    totalCustodiedGB: number;
  };

  // Is this the primary family node?
  isPrimary: boolean;
}

/**
 * NodeClusterStatus - Health states for nodes in topology
 */
export type NodeClusterStatus =
  | 'online' // Active and healthy
  | 'offline' // Not responding to heartbeats
  | 'degraded' // Online but performance issues
  | 'maintenance' // Planned downtime
  | 'provisioning' // New node being set up
  | 'unknown'; // Haven't received status yet

/**
 * NodeRole - What a node is doing in the cluster
 */
export interface NodeRole {
  role: 'storage' | 'compute' | 'gateway' | 'custodian' | 'archive';
  description: string;
  utilizationPercent: number; // How busy this role is
}

/**
 * OfflineNodeAlert - Alert when a node goes offline
 *
 * This powers the banner/popup in Shefa header
 */
export interface OfflineNodeAlert {
  id: string;
  severity: 'info' | 'warning' | 'critical';
  nodeId: string;
  nodeName: string;
  isPrimaryNode: boolean;

  // What happened
  eventType: 'went-offline' | 'degraded' | 'heartbeat-missed' | 'recovery-needed';
  message: string;

  // When
  detectedAt: string; // ISO 8601
  lastSeenOnline: string; // ISO 8601
  offlineDuration: string; // Human-readable, e.g., "2 hours"

  // Impact assessment
  impact: {
    affectedContent: number; // Content items impacted
    affectedCustodians: number; // Custodian relationships affected
    computeGapPercent: number; // % of compute lost
    storageGapPercent: number; // % of storage lost
  };

  // Recovery actions
  recommendedActions: string[];
  helpFlowUrl?: string; // Link to compute needs help-flow
  dismissedAt?: string; // If user dismissed alert
}

// =============================================================================
// BIDIRECTIONAL CUSTODIAN VIEW
// =============================================================================

/**
 * BidirectionalCustodianView - Who I'm helping vs who's helping me
 *
 * Shows the mutual aid aspect of the custodian network:
 * - Outbound: Others I'm storing content for
 * - Inbound: Others storing my content
 */
export interface BidirectionalCustodianView {
  // Who I'm helping (storing their content)
  helping: CustodianRelationship[];
  helpingCount: number;
  helpingTotalGB: number;

  // Who's helping me (storing my content)
  beingHelpedBy: CustodianRelationship[];
  beingHelpedByCount: number;
  beingHelpedByTotalGB: number;

  // Balance indicator
  mutualAidBalance: {
    ratio: number; // helping / beingHelped (1.0 = balanced)
    status: 'giving-more' | 'balanced' | 'receiving-more';
    message: string; // "You're helping 3 more people than are helping you"
  };

  // Community health
  communityStrength: 'strong' | 'moderate' | 'weak';
}

/**
 * CustodianRelationship - A single custodian relationship
 */
export interface CustodianRelationship {
  // Who
  agentId: string;
  displayName: string;
  relationshipType: 'family' | 'friend' | 'community' | 'professional';
  trustScore: number; // 0-100

  // Direction
  direction: 'i-help-them' | 'they-help-me';

  // What content
  contentSummary: {
    totalItems: number;
    totalGB: number;
    contentTypes: { type: string; count: number; gb: number }[];
  };

  // Health
  status: 'active' | 'pending' | 'at-risk' | 'expired';
  lastActivity: string; // ISO 8601
  reliability: number; // 0-100 uptime percentage
}

// =============================================================================
// STORAGE CONTENT DISTRIBUTION
// =============================================================================

/**
 * StorageContentDistribution - What types of content are stored where
 *
 * Shows breakdown by content type, reach level, and storage location
 */
export interface StorageContentDistribution {
  // By content type
  byContentType: ContentTypeStorage[];

  // By reach level (0-7)
  byReachLevel: ReachLevelStorage[];

  // By storage location (which nodes)
  byNode: NodeStorageBreakdown[];

  // Summary
  totalContent: {
    items: number;
    sizeGB: number;
    replicaCount: number; // Total replicas across all custodians
  };
}

/**
 * ContentTypeStorage - Storage breakdown by content type
 */
export interface ContentTypeStorage {
  contentType: 'video' | 'audio' | 'image' | 'document' | 'application' | 'learning' | 'other';
  displayLabel: string; // "Videos", "Learning Materials"
  icon?: string; // Icon class or URL

  // Amounts
  itemCount: number;
  sizeGB: number;
  percentOfTotal: number;

  // Replication status
  fullyReplicated: number; // Items meeting target replica count
  underReplicated: number; // Items below target
  averageReplicas: number;
}

/**
 * ReachLevelStorage - Storage breakdown by reach level
 */
export interface ReachLevelStorage {
  reachLevel: number; // 0-7
  reachLabel: string; // "Private", "Household", ..., "Commons"

  // What's at this reach level
  itemCount: number;
  sizeGB: number;
  targetReplicas: number; // Target replica count for this reach
  currentReplicas: number; // Average current replicas
  replicationStatus: 'met' | 'under' | 'over';
}

/**
 * NodeStorageBreakdown - What's stored on each node
 */
export interface NodeStorageBreakdown {
  nodeId: string;
  nodeName: string;
  nodeStatus: NodeClusterStatus;

  // Storage on this node
  totalGB: number;
  usedGB: number;
  availableGB: number;

  // Content breakdown
  contentBreakdown: {
    myContent: number; // GB of my own content
    custodiedContent: number; // GB of others' content I'm storing
    cacheContent: number; // GB of cached/ephemeral content
  };

  // What types
  contentTypes: { type: string; gb: number }[];
}

// =============================================================================
// COMPUTE NEEDS ASSESSMENT (for help-flow)
// =============================================================================

/**
 * ComputeNeedsAssessment - Evaluation of compute gaps and recommendations
 *
 * Powers the help-flow that guides users to order needed nodes
 */
export interface ComputeNeedsAssessment {
  // Current state
  currentCapacity: {
    totalCPUCores: number;
    totalMemoryGB: number;
    totalStorageGB: number;
    totalBandwidthMbps: number;
  };

  // Gaps
  gaps: ComputeGap[];
  hasGaps: boolean;
  overallGapSeverity: 'none' | 'minor' | 'moderate' | 'critical';

  // Recommendations
  recommendations: NodeRecommendation[];

  // Help flow link
  helpFlowUrl: string;
  helpFlowCTA: string; // "Order a Holoport to restore full protection"
}

/**
 * ComputeGap - A specific compute deficiency
 */
export interface ComputeGap {
  resource: 'cpu' | 'memory' | 'storage' | 'bandwidth' | 'redundancy';
  currentValue: number;
  targetValue: number;
  gapPercent: number;
  severity: 'minor' | 'moderate' | 'critical';
  description: string;
  impact: string; // What this gap means for the user
}

/**
 * NodeRecommendation - Suggested node to address compute gaps
 */
export interface NodeRecommendation {
  nodeType: 'holoport' | 'holoport-plus' | 'holoport-nano' | 'self-hosted' | 'cloud';
  displayName: string;
  description: string;

  // What gaps this would address
  addressesGaps: string[]; // Gap resource types
  improvementPercent: number; // How much better things would be

  // Ordering info
  estimatedCost?: {
    value: number;
    currency: string;
    period?: 'one-time' | 'monthly' | 'yearly';
  };
  orderUrl?: string;
  priority: 'recommended' | 'optional' | 'future';
}

// =============================================================================
// DASHBOARD CONFIGURATION
// =============================================================================

// =============================================================================
// DISPLAY HELPERS
// =============================================================================

/**
 * Get display info for node type.
 */
export function getNodeTypeDisplay(type: OwnedNode['nodeType']): { label: string; icon: string } {
  const displays: Record<OwnedNode['nodeType'], { label: string; icon: string }> = {
    'holoport': { label: 'HoloPort', icon: 'dns' },
    'holoport-plus': { label: 'HoloPort+', icon: 'hub' },
    'holoport-nano': { label: 'HoloPort Nano', icon: 'memory' },
    'self-hosted': { label: 'Self-Hosted', icon: 'home' },
    'cloud': { label: 'Cloud', icon: 'cloud' },
  };
  return displays[type] ?? { label: type, icon: 'computer' };
}

/**
 * Get display info for node status.
 */
export function getNodeStatusDisplay(status: NodeClusterStatus): { label: string; color: string; icon: string } {
  const displays: Record<NodeClusterStatus, { label: string; color: string; icon: string }> = {
    'online': { label: 'Online', color: '#22c55e', icon: 'check_circle' },
    'offline': { label: 'Offline', color: '#ef4444', icon: 'error' },
    'degraded': { label: 'Degraded', color: '#f59e0b', icon: 'warning' },
    'maintenance': { label: 'Maintenance', color: '#6b7280', icon: 'build' },
    'provisioning': { label: 'Provisioning', color: '#3b82f6', icon: 'sync' },
    'unknown': { label: 'Unknown', color: '#6b7280', icon: 'help' },
  };
  return displays[status] ?? displays['unknown'];
}

/**
 * Get display info for gap severity.
 */
export function getGapSeverityDisplay(severity: ComputeGap['severity'] | 'low' | 'critical'): { label: string; color: string; icon: string } {
  const displays: Record<string, { label: string; color: string; icon: string }> = {
    'none': { label: 'None', color: '#22c55e', icon: 'check_circle' },
    'low': { label: 'Low', color: '#22c55e', icon: 'info' },
    'minor': { label: 'Minor', color: '#22c55e', icon: 'info' },
    'moderate': { label: 'Moderate', color: '#f59e0b', icon: 'warning' },
    'critical': { label: 'Critical', color: '#ef4444', icon: 'dangerous' },
  };
  return displays[severity] ?? displays['moderate'];
}

/**
 * Calculate health score from node statuses.
 */
export function calculateHealthScore(nodes: OwnedNode[]): number {
  if (nodes.length === 0) return 0;

  const statusScores: Record<NodeClusterStatus, number> = {
    'online': 100,
    'provisioning': 80,
    'maintenance': 70,
    'degraded': 50,
    'offline': 0,
    'unknown': 25,
  };

  const totalScore = nodes.reduce((sum, node) => {
    const baseScore = statusScores[node.status];
    // Primary node has more weight
    const weight = node.isPrimary ? 2 : 1;
    return sum + (baseScore * weight);
  }, 0);

  const totalWeight = nodes.reduce((sum, node) => sum + (node.isPrimary ? 2 : 1), 0);
  return Math.round(totalScore / totalWeight);
}

/**
 * Format gap duration for display.
 */
export function formatGapDuration(durationMs: number): string {
  const minutes = Math.floor(durationMs / 60000);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  if (days > 0) {
    return `${days} day${days > 1 ? 's' : ''}`;
  } else if (hours > 0) {
    return `${hours} hour${hours > 1 ? 's' : ''}`;
  } else if (minutes > 0) {
    return `${minutes} minute${minutes > 1 ? 's' : ''}`;
  } else {
    return 'Just now';
  }
}

// =============================================================================
// DASHBOARD CONFIGURATION
// =============================================================================

/**
 * SheafaDashboardConfig - User preferences and display settings
 */
export interface SheafaDashboardConfig {
  // Display preferences
  displayMode: 'compact' | 'detailed' | 'monitoring';
  refreshInterval: number; // Milliseconds
  chartTimescale: '1h' | '24h' | '7d' | '30d';

  // Alerts
  alertThreshold: 'high' | 'medium' | 'low';
  enableNotifications: boolean;

  // Panel visibility
  visiblePanels: {
    computeMetrics: boolean;
    familyProtection: boolean;
    tokenEarnings: boolean;
    economicEvents: boolean;
    constitutionalLimits: boolean;
    trustGraph: boolean;
  };

  // Currency/unit preferences
  preferredCurrency: 'USD' | 'HoloFuel' | 'EUR' | 'other';
  preferredUnits: 'metric' | 'imperial';
}
