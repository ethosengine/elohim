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
