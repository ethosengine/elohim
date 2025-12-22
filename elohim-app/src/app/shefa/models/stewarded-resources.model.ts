/**
 * Stewarded Resources Model - The Transparency of Stewardship
 *
 * From the Manifesto (Part II: Stewardship):
 * "Every human is a steward of resources. The autonomous network makes this
 * explicit through transparent resource accounting. Capacity, allocation, and
 * usage are visible - not to control, but to enable wise stewardship."
 *
 * Core Concept:
 * This system gives every human a comprehensive view of the resources they
 * steward: energy (time, attention, currency), compute (their social node
 * capacity and load), water, food, shelter, transportation, property,
 * equipment, and inventories.
 *
 * Architecture:
 * - StewardedResource: The resource entity with capacity/allocation/usage
 * - ResourceCategory: Classification (energy, compute, water, food, etc.)
 * - ResourceDimension: How the resource is measured (kWh, GB, liters, etc.)
 * - AllocationBlock: What the resource is allocated to
 * - UsageRecord: EconomicEvent-based tracking of actual usage
 *
 * Integration:
 * - Ties to EconomicEvent for immutable audit trail
 * - Connects to Observer protocol for actual measurement
 * - Governance aware (graduated governance levels)
 * - Supports voluntary allocation and constitutional limits
 *
 * Use Cases:
 * - "How much of my node capacity am I using?" → compute dimension
 * - "Where is my attention/time allocated?" → energy dimension
 * - "What do I own and how is it used?" → property/equipment dimensions
 * - "What are my household's resource flows?" → governance-aggregated view
 */

// =============================================================================
// Resource Categories & Dimensions
// =============================================================================

/**
 * ResourceCategory - The different types of resources humans steward
 *
 * Note on Financial/Asset Categories:
 * These support the transition from legacy economic systems to Elohim stewardship.
 * UBA/UBI (Universal Basic Assets/Income) provides dignity floors during transition.
 * Financial assets are tracked transparently; no mystique around currency.
 */
export type ResourceCategory =
  | 'energy'             // Time, attention, currency
  | 'compute'            // Processing power, storage, bandwidth
  | 'water'              // Fresh water, wastewater
  | 'food'               // Calories, nutrition, ingredients
  | 'shelter'            // Housing, square footage, utilities
  | 'transportation'     // Vehicles, fuel, mobility
  | 'property'           // Personal property, goods
  | 'equipment'          // Tools, instruments, machines
  | 'inventory'          // Raw materials, supplies, stock
  | 'knowledge'          // Expertise, learning, skills
  | 'reputation'         // Trust, credentials, standing
  | 'financial-asset'    // Fiat currency, mutual credit, assets
  | 'uba';               // Universal Basic Assets (transitional)

/**
 * ResourceSubcategory - More specific resource types
 */
export interface ResourceSubcategory {
  category: ResourceCategory;
  name: string;
  description: string;
  examples: string[];
}

/**
 * ResourceDimension - How a resource is measured
 */
export interface ResourceDimension {
  unit: string;                    // kWh, GB, liters, kg, m², etc.
  unitLabel: string;               // "kilowatt-hours", "gigabytes", etc.
  unitAbbreviation: string;        // "kWh", "GB", "L", etc.
  conversionFactor?: number;       // For converting to standard unit
  standardUnit?: string;           // Base unit for this category
}

/**
 * ResourceMeasure - A quantity of resource
 */
export interface ResourceMeasure {
  value: number;
  unit: string;
}

/**
 * AllocationStatus - Where the resource is allocated
 */
export type AllocationStatus =
  | 'unallocated'     // Available but not assigned
  | 'allocated'       // Assigned to a purpose/person
  | 'reserved'        // Held for future use
  | 'available'       // Could be allocated if needed
  | 'fully-used';     // All allocated capacity is being used

/**
 * AllocationBlock - An allocation of resource to a purpose
 */
export interface AllocationBlock {
  id: string;
  resourceId: string;
  label: string;                    // "Learning", "Work", "Community", etc.
  description?: string;
  allocated: ResourceMeasure;       // How much is allocated
  used: ResourceMeasure;            // How much is currently used
  reserved: ResourceMeasure;        // How much is reserved for future use
  governanceLevel?: string;         // individual, household, community, etc.
  governedBy?: string;              // Who controls this allocation
  commitmentId?: string;            // hREA Commitment if governed
  priority: number;                 // 1-10 relative priority
  utilization: number;              // Percentage: used / allocated
  createdAt: string;
  updatedAt: string;
}

/**
 * UsageRecord - A single use event (usually from EconomicEvent)
 */
export interface UsageRecord {
  id: string;
  resourceId: string;
  economicEventId?: string;         // Link to EconomicEvent for immutability
  allocationBlockId?: string;       // Which allocation was this from
  action: string;                   // 'use', 'consume', 'produce', 'transfer', etc.
  quantity: ResourceMeasure;
  duration?: string;                // ISO 8601 duration if time-based
  observerAttestationId?: string;   // Observer protocol verification
  timestamp: string;
  note?: string;
}

/**
 * ResourceTrend - Historical trend data for a resource
 */
export interface ResourceTrend {
  period: 'day' | 'week' | 'month' | 'quarter' | 'year';
  used: ResourceMeasure;
  allocated: ResourceMeasure;
  utilization: number;
  trend: 'increasing' | 'stable' | 'decreasing';
  changePercent: number;
}

// =============================================================================
// StewardedResource - Core Entity
// =============================================================================

/**
 * StewardedResource - A resource tracked and managed by a human
 *
 * This is the primary entity for resource stewardship. It tracks:
 * - Capacity: Total amount available
 * - Allocation: How it's divided among purposes
 * - Usage: Actual consumption/production over time
 * - Governance: Who decides how it's used (graduated governance)
 */
export interface StewardedResource {
  // ─────────────────────────────────────────────────────────────────
  // Identity
  // ─────────────────────────────────────────────────────────────────
  id: string;
  resourceNumber: string;           // RES-XXXXXXXXXX
  stewardId: string;                // Human ID who stewards this
  category: ResourceCategory;
  subcategory: string;              // More specific type
  name: string;                     // User-friendly name
  description?: string;

  // ─────────────────────────────────────────────────────────────────
  // Capacity & Measurement
  // ─────────────────────────────────────────────────────────────────
  dimension: ResourceDimension;     // How it's measured
  totalCapacity: ResourceMeasure;   // Maximum available
  permanentReserve?: ResourceMeasure; // Constitutional minimum (cannot allocate)
  allocatableCapacity: ResourceMeasure; // What can actually be allocated

  // ─────────────────────────────────────────────────────────────────
  // Current State
  // ─────────────────────────────────────────────────────────────────
  totalAllocated: ResourceMeasure;  // Sum of all allocations
  totalReserved: ResourceMeasure;   // Sum of all reserves
  totalUsed: ResourceMeasure;       // Sum of actual usage
  available: ResourceMeasure;       // totalCapacity - allocated

  // ─────────────────────────────────────────────────────────────────
  // Allocations
  // ─────────────────────────────────────────────────────────────────
  allocations: AllocationBlock[];   // How resource is divided
  allocationStrategy: 'manual' | 'automatic' | 'hybrid';
  allocationNotes?: string;

  // ─────────────────────────────────────────────────────────────────
  // Governance
  // ─────────────────────────────────────────────────────────────────
  governanceLevel: 'individual' | 'household' | 'community' | 'network' | 'constitutional';
  governedBy?: string;              // ID of entity making decisions
  constitutionalBasis?: string;     // Reference to governance document
  canModifyAllocations: boolean;    // Individual can change allocations?
  requiresApprovalFor?: string[];   // Actions needing governance approval

  // ─────────────────────────────────────────────────────────────────
  // Tracking & Verification
  // ─────────────────────────────────────────────────────────────────
  observerEnabled: boolean;         // Is this measured by Observer protocol?
  observerAgentId?: string;         // Who verifies actual usage
  recentUsage: UsageRecord[];       // Last N usage events
  trends: ResourceTrend[];          // Historical trends

  // ─────────────────────────────────────────────────────────────────
  // Economic Integration
  // ─────────────────────────────────────────────────────────────────
  resourceSpecId?: string;          // hREA ResourceSpecification
  commonsPoolId?: string;           // CommonsPool if shared resource
  allocationEventIds: string[];     // EconomicEvent IDs that set allocations
  usageEventIds: string[];          // EconomicEvent IDs for all usage

  // ─────────────────────────────────────────────────────────────────
  // Metadata
  // ─────────────────────────────────────────────────────────────────
  isShared: boolean;                // Is this shared with others?
  visibility: 'private' | 'household' | 'community' | 'public';
  dataQuality: 'measured' | 'estimated' | 'manual' | 'mixed';
  lastVerifiedAt?: string;          // When Observer last confirmed actual usage
  createdAt: string;
  updatedAt: string;
}

// =============================================================================
// Resource Dashboard Aggregations
// =============================================================================

/**
 * CategorySummary - Aggregation of all resources in one category
 */
export interface CategorySummary {
  category: ResourceCategory;
  resources: StewardedResource[];
  totalCapacity: ResourceMeasure;
  totalAllocated: ResourceMeasure;
  totalUsed: ResourceMeasure;
  utilizationPercent: number;
  healthStatus: 'healthy' | 'warning' | 'critical';
}

/**
 * StewaredResourceDashboard - Complete resource overview
 */
export interface StewardedResourceDashboard {
  stewardId: string;
  stewardName: string;
  governanceLevel: string;          // Their level in governance

  // Overview by category
  categories: CategorySummary[];

  // Key metrics
  metrics: {
    totalResourcesTracked: number;
    categoriesCovered: number;
    overallUtilization: number;     // Weighted average
    fullyAllocatedCount: number;    // How many at capacity
    healthStatus: 'healthy' | 'warning' | 'critical';
  };

  // Alerts and insights
  alerts: ResourceAlert[];
  insights: ResourceInsight[];

  // Recent activity
  recentAllocations: AllocationBlock[];
  recentUsage: UsageRecord[];

  lastUpdatedAt: string;
}

/**
 * ResourceAlert - Warning about resource state
 */
export interface ResourceAlert {
  id: string;
  severity: 'info' | 'warning' | 'critical';
  resource: string;                 // Resource ID or category
  title: string;
  message: string;
  recommendedAction?: string;
  createdAt: string;
}

/**
 * ResourceInsight - Observation about resource patterns
 */
export interface ResourceInsight {
  id: string;
  type: 'trend' | 'pattern' | 'opportunity' | 'governance';
  title: string;
  description: string;
  resource?: string;
  basedOnPeriod: string;            // e.g., "last 30 days"
  confidence: number;               // 0-100, how sure we are
}

// =============================================================================
// Compute Resource Specifics (Node Capacity & Load)
// =============================================================================

/**
 * ComputeResource - Social Elohim node capacity and utilization
 * Specialization of StewardedResource for computing/processing
 */
export interface ComputeResource extends StewardedResource {
  category: 'compute';

  // Compute-specific dimensions
  cpuCores: number;
  cpuUtilizationPercent: number;
  memoryGb: number;
  memoryUtilizationPercent: number;
  storageGb: number;
  storageUtilizationPercent: number;
  bandwidthMbps?: number;
  bandwidthUtilizationPercent?: number;

  // Node-specific
  nodeType: 'personal' | 'household' | 'community' | 'network';
  nodeId: string;                   // Identity in the network
  uptime: number;                   // Percentage online
  healthScore: number;              // 0-100 node health

  // Load allocation
  loadAllocation: {
    lamad: number;                  // Learning system percentage
    shefa: number;                  // Economic coordination
    qahal: number;                  // Governance
    holochain: number;              // Holochain runtime
    userApplications: number;        // User's own apps
    overhead: number;               // System overhead
  };

  // Physical constraints
  physicalLimitations: {
    powerConsumptionWatts: number;
    coolingRequiredKw?: number;
    operatingTemperatureC: number;
    upstreamBandwidthGbps?: number;
  };
}

// =============================================================================
// Energy Resource Specifics (Time, Attention, Currency)
// =============================================================================

/**
 * EnergyResource - Time, attention, and currency resources
 * Specialization of StewardedResource for human energy/time/currency
 */
export interface EnergyResource extends StewardedResource {
  category: 'energy';

  // Time-based (human availability)
  hoursPerWeek?: number;            // Available hours per week
  attentionCapacity?: number;       // Max concurrent engagements
  recoveryTime?: number;            // Hours needed for rest

  // Currency/value
  currencyType?: string;            // USD, EUR, mutual-credit, etc.
  accountBalance?: number;          // Current balance
  incomeSources?: string[];         // Where money comes from

  // Social energy
  relationshipCapacity?: number;    // Max people to maintain relationships with
  collaborativeCapacity?: number;   // Max projects simultaneously
}

// =============================================================================
// Financial Asset Tracking (For Transition Economy)
// =============================================================================

/**
 * FinancialAsset - Transparent tracking of money and financial instruments
 * Specialization of StewardedResource for fiat, mutual credit, and assets
 *
 * From the Manifesto:
 * "We don't hide from money. Money is information. We make it transparent."
 * Financial assets are tracked in the same stewardship view as all other
 * resources - enabling dignity while transitioning to Elohim economy.
 */
export interface FinancialAsset extends StewardedResource {
  category: 'financial-asset';

  // Asset identification
  assetType: 'fiat-currency' | 'mutual-credit' | 'cryptocurrency' | 'stock' | 'bond' | 'property-equity' | 'debt' | 'other';
  currencyCode?: string;            // USD, EUR, BTC, etc.
  accountNumber?: string;           // Bank/platform account (hashed for privacy)
  accountInstitution?: string;      // Bank, exchange, service name

  // Account state
  accountBalance: number;           // Current balance
  availableBalance: number;         // Balance available to spend
  pendingTransactions: Transaction[];
  accountStatus: 'active' | 'frozen' | 'closed';

  // Income streams (for dignity floors)
  incomeStreams: IncomeStream[];
  monthlyIncome: number;            // Total income per month
  expectedMonthlyIncome: number;    // Projected income

  // Liabilities & Obligations
  obligations: FinancialObligation[];
  totalLiability: number;           // Total amount owed
  monthlyObligations: number;       // Regular monthly obligations

  // Dignity floor (UBI/UBA tracking)
  ubaEligible: boolean;             // Is this person eligible for UBA?
  ubaStatus: 'active' | 'pending' | 'paused' | 'inactive';
  ubaMonthlyAmount?: number;        // Amount allocated monthly
  ubaLastPayment?: string;          // When was last UBA payment
  ubaPaymentSchedule?: string;      // How often paid (weekly, biweekly, monthly)

  // Financial health metrics
  burnRate?: number;                // How fast they spend per day
  runwayDays?: number;              // Days until account depleted (if no income)
  debtToIncomeRatio?: number;       // Monthly obligations / monthly income
  creditScore?: number;             // If tracked by legacy system
  confidenceScore?: number;         // System's confidence in this data (0-100)

  // Transparency & Verification
  dataSource: 'bank-api' | 'manual' | 'blockchain' | 'mixed';
  lastVerifiedAt?: string;          // When was data last confirmed
  verificationMethod?: string;      // How was it verified
}

/**
 * IncomeStream - A source of income
 */
export interface IncomeStream {
  id: string;
  source: string;                   // "Employment", "Gig Work", "UBA", "Inheritance", etc.
  amount: number;
  frequency: 'daily' | 'weekly' | 'biweekly' | 'monthly' | 'quarterly' | 'annual' | 'irregular';
  frequency_value?: number;         // For "every N days/weeks/months"
  status: 'active' | 'paused' | 'ended';
  startDate: string;
  endDate?: string;
  isGuaranteed: boolean;            // Is this guaranteed (job vs gig)?
  confidence: number;               // 0-100, how sure we are this continues
  note?: string;
}

/**
 * Transaction - A single financial transaction
 */
export interface Transaction {
  id: string;
  timestamp: string;
  type: 'debit' | 'credit' | 'transfer' | 'fee';
  amount: number;
  currency: string;
  description: string;
  category?: string;                // 'food', 'housing', 'transport', etc.
  counterparty?: string;            // Who did we transact with
  economicEventId?: string;         // Link to hREA EconomicEvent
  status: 'pending' | 'completed' | 'failed' | 'cancelled';
}

/**
 * FinancialObligation - Money owed
 */
export interface FinancialObligation {
  id: string;
  creditorId: string;               // Who we owe
  creditorName?: string;
  principalAmount: number;          // Original amount borrowed
  remainingAmount: number;          // Still owed
  interest?: number;                // Interest rate (if applicable)
  monthlyPayment: number;           // Regular payment amount
  minimumPayment?: number;          // Minimum required (credit cards)
  paymentDueDate?: string;          // When is next payment due
  obligationType: 'loan' | 'credit-card' | 'rent' | 'mortgage' | 'tax' | 'child-support' | 'other';
  daysOverdue: number;              // If applicable
  status: 'current' | 'overdue' | 'defaulted' | 'paid-off';
  governanceLevel?: string;         // Some obligations may be community-governed
  transparencyLevel: 'private' | 'household' | 'community' | 'public';
}

// =============================================================================
// Universal Basic Assets/Income (Dignity Floor)
// =============================================================================

/**
 * UBAEligibility - Determines if human qualifies for Universal Basic Assets
 *
 * UBA is the transitional mechanism to provide dignity while moving to Elohim.
 * It's not charity - it's recognition that every human has basic entitlements
 * and needs a floor of dignity while building autonomous capacity.
 */
export interface UBAEligibility {
  humanId: string;
  eligible: boolean;
  eligibilityReason?: string;       // Why are they eligible/ineligible

  // Entitlements
  basicAssets: BasicAssetEntitlement[];
  basicIncome?: BasicIncomeEntitlement;

  // Proof of life (from Elohim governance)
  verifiedAt?: string;              // When was eligibility last verified
  verifiedBy?: string;              // Governance entity that verified
  documentationLinks?: string[];    // References to eligibility docs
}

/**
 * BasicAssetEntitlement - What basic assets a human is entitled to
 */
export interface BasicAssetEntitlement {
  assetType: string;                // 'food', 'shelter', 'healthcare', 'internet', etc.
  monthlyAllocation: number;        // Amount per month in currency
  unit?: string;                    // If not currency (kg, m², etc.)
  governanceLevel: string;          // Who funds this (municipal, national, Elohim?)
  provider?: string;                // Where they get it (food bank, co-op, etc.)
  implemented: boolean;             // Is this actually available?
  claimedAt?: string;               // When did they start receiving it
}

/**
 * BasicIncomeEntitlement - Universal Basic Income allocation
 *
 * UBI provides a floor: enough to meet basic needs while transitioning.
 * In Elohim system, this becomes less necessary as people steward more
 * resources and participate in autonomous economic coordination.
 */
export interface BasicIncomeEntitlement {
  currency: string;                 // USD, EUR, etc.
  monthlyAmount: number;            // Amount guaranteed per month
  distributionSchedule: 'weekly' | 'biweekly' | 'monthly';
  distributionDay?: number;         // Day of week/month to distribute
  provider: string;                 // Government, foundation, Elohim community?
  implemented: boolean;             // Is this being paid?
  paidSince?: string;               // When did payments start
  expectedDuration?: string;        // Until when (years, indefinite, etc.)
  requirements?: string[];          // Any requirements to continue receiving
  governanceLevel: string;          // At what level is this governed (city, nation, etc.)
}

/**
 * DignityFloor - Aggregated view of minimum entitlements
 *
 * This shows what every human has access to as a minimum.
 * It's the foundation - not the ceiling. People steward far more.
 */
export interface DignityFloor {
  humanId: string;
  eligible: boolean;

  // Basic survival needs (in local currency equivalent)
  foodDailyAmount: number;          // Minimum food subsidy per day
  shelterMonthlyAmount: number;     // Minimum housing support
  healthcareMonthlyAmount: number;  // Medical coverage
  internetMonthlyAmount: number;    // Digital access
  transportMonthlyAmount?: number;  // Getting around

  // Total dignity floor per month
  totalMonthlyFloor: number;

  // Current status
  floorMet: boolean;                // Is current income meeting minimum?
  monthlyShortfall?: number;        // How much below floor (if applicable)
  gapCovered?: string;              // What's covering the gap (if not income)

  // Constitutional base
  governanceDocument?: string;      // Reference to constitution providing this
  verifiedAt?: string;
}

// =============================================================================
// Stewardship View for Financial Health
// =============================================================================

/**
 * FinancialStewardshipView - Complete financial picture with dignity floor
 *
 * This unifies:
 * - All income streams (guaranteed and uncertain)
 * - All obligations (loans, rent, etc.)
 * - UBA/UBI entitlements (dignity floor)
 * - Financial assets (accounts, investments)
 * - Overall financial health
 *
 * Transparency without judgment. Every human deserves to understand their
 * economic situation in full.
 */
export interface FinancialStewardshipView {
  humanId: string;
  humanName: string;

  // Income picture
  monthlyIncome: number;            // Guaranteed income
  expectedMonthlyIncome: number;    // Including likely/uncertain sources
  incomeStreams: IncomeStream[];
  incomeStability: 'stable' | 'variable' | 'uncertain';

  // Obligations
  monthlyObligations: number;       // Fixed obligations
  totalLiabilities: number;         // Total amount owed
  obligations: FinancialObligation[];

  // Dignity floor
  dignityFloor: DignityFloor;
  onTrackForDignity: boolean;       // Will they meet minimum needs?

  // Financial health
  monthlyDifference: number;        // Income - obligations
  netWorth?: number;                // Assets - liabilities
  financialHealth: 'healthy' | 'stable' | 'at-risk' | 'critical';
  burnoutDate?: string;             // When will money run out (if no change)

  // Recommendations
  recommendations: FinancialRecommendation[];

  // Assets overview
  assets: FinancialAsset[];
  totalAssets: number;

  lastUpdatedAt: string;
}

/**
 * FinancialRecommendation - System suggestion for financial improvement
 */
export interface FinancialRecommendation {
  id: string;
  priority: 'critical' | 'high' | 'medium' | 'low';
  title: string;
  description: string;
  recommendedAction: string;
  estimatedImpact?: string;         // How much this could help
  relatedTo?: string[];             // Income stream, obligation, etc. IDs
  governanceLevel?: string;         // Can be help at this level
}

// =============================================================================
// Constitutional Economic Limits (Donut Economy & Limitarianism)
// =============================================================================

/**
 * ConstitutionalLimit - Bounds on stewardship within Elohim framework
 *
 * From the Manifesto (Part III: Constitutional Economics):
 * "Limitarianism recognizes that accumulation beyond constitutional bounds
 * enables extraction. We define wise stewardship bounds based on:
 * - Planetary boundaries (ecological ceiling)
 * - Human dignity (social floor)
 * - Community capacity (collective thriving)
 *
 * These are not punitive but generative - they describe the space where
 * stewardship creates flourishing rather than extraction."
 *
 * Implements Donut Economy thinking:
 * - FLOOR: Dignity minimum (what everyone needs)
 * - CEILING: Constitutional maximum (beyond which is excess)
 * - SAFE OPERATING SPACE: The zone between floor and ceiling
 */
export interface ConstitutionalLimit {
  // Identity
  id: string;
  resourceCategory: ResourceCategory;
  name: string;                         // "Wealth Ceiling", "Energy Allocation", etc.
  description: string;

  // Floor (Dignity Minimum)
  floorValue: number;                   // Minimum every human needs
  floorUnit: string;
  floorRationale: string;               // Why this floor (constitutional basis)
  floorEnforced: boolean;               // Is this currently enforced?

  // Ceiling (Constitutional Maximum)
  ceilingValue: number;                 // Maximum wise stewardship allows
  ceilingUnit: string;
  ceilingRationale: string;             // Why this ceiling (constitutional basis)
  ceilingEnforced: boolean;             // Is this currently enforced?

  // Safe Operating Space (between floor and ceiling)
  safeMinValue: number;
  safeMaxValue: number;
  safeZoneDescription: string;          // Optimal stewardship zone

  // Governance
  governanceLevel: string;              // municipal, national, Elohim-network?
  constitutionalBasis: string;          // Reference to governance document
  adoptionDate?: string;
  reviewSchedule?: string;              // How often is this revisited?

  // Enforcement & Transition
  enforcementMethod: 'voluntary' | 'progressive' | 'hard';
  // voluntary: honor system, trust-based
  // progressive: incentive-based (positive rewards for compliance, gradually increasing)
  // hard: mandatory enforcement
  transitionDeadline?: string;          // When must people be within bounds?
  hardStopDate?: string;                // Point of no return

  // Metadata
  createdAt: string;
  updatedAt: string;
}

/**
 * ResourcePosition - Where an asset stands relative to constitutional bounds
 */
export interface ResourcePosition {
  resourceId: string;
  stewardId: string;
  resourceCategory: ResourceCategory;
  currentValue: number;
  unit: string;

  // Relative to Constitutional Limits
  constitutionalLimit: ConstitutionalLimit;
  positionRelativeToFloor: 'below-floor' | 'at-floor' | 'in-safe-zone' | 'above-ceiling' | 'far-above-ceiling';
  distanceFromFloor: number;            // Negative if below, positive if above
  distanceFromCeiling: number;          // Positive if above ceiling

  // Excess/Surplus Calculation
  excessAboveCeiling?: number;          // How much is above constitutional max
  excessPercentage?: number;            // Percentage above ceiling
  surplusAvailableForTransition?: number; // Amount that could be reallocated

  // Health Status
  complianceStatus: 'compliant' | 'approaching-limit' | 'exceeds-ceiling' | 'far-exceeds-ceiling';
  warningLevel: 'none' | 'yellow' | 'orange' | 'red';

  // Transition Path
  onTransitionPath: boolean;
  transitionStatus?: 'exploring' | 'negotiating' | 'committed' | 'executing' | 'completed';
  estimatedComplianceDate?: string;
}

/**
 * TransitionPath - How an excess asset navigates to community stewardship
 *
 * Example: Someone with $5M in stocks, ceiling is $2M
 * They have $3M excess that needs transition pathway:
 * - Negotiation: What portion to commons? What to community benefit corp?
 * - Execution: Gradual reallocation over time
 * - Legacy: What governance role do they keep in transitioned assets?
 */
export interface TransitionPath {
  id: string;
  resourceId: string;
  stewardId: string;
  assetName: string;

  // Current Position
  currentValue: number;
  constitutionalCeiling: number;
  excess: number;                       // Amount above ceiling

  // Proposed Transition (Negotiation Phase)
  proposedSplits: AssetSplit[];        // How to divide the asset
  totalProposedExcess: number;          // Sum of all proposed transitions

  // Timeline
  status: 'proposal' | 'negotiating' | 'agreed' | 'executing' | 'completed';
  proposedAt: string;
  negotiationDeadline?: string;
  executionStartDate?: string;
  completionDeadline?: string;
  actualCompletionDate?: string;

  // Governance
  governanceLevel: string;              // Who decides this transition
  governingBody?: string;               // Committee, Qahal, etc.
  approvalStatus: 'pending' | 'approved' | 'rejected' | 'negotiating';

  // Tracking
  executionPhases: TransitionPhase[];   // Step-by-step execution
  notes?: string;

  // Constitutional Record
  transitionEventIds: string[];         // EconomicEvent IDs for all transfers
  transparencyLevel: 'private' | 'household' | 'community' | 'public';

  // Metadata
  createdAt: string;
  updatedAt: string;
}

/**
 * AssetSplit - How an asset is split during transition
 */
export interface AssetSplit {
  id: string;
  splitName: string;                    // "Commons Pool", "Community Benefit Corp", etc.
  destinationType: 'commons-pool' | 'community-benefit-corp' | 'cooperative' | 'trust' | 'foundation' | 'other';
  destinationId: string;                // ID of receiving entity
  amount: number;
  percentage: number;                   // Percentage of excess
  governance?: string;                  // Does steward keep role in transitioned asset?
  rationale: string;                    // Why this split
  status: 'proposed' | 'agreed' | 'executing' | 'completed';
}

/**
 * TransitionPhase - Steps in executing a transition
 */
export interface TransitionPhase {
  id: string;
  sequenceNumber: number;
  name: string;                         // "Convert to mutual fund", "Transfer to commons", etc.
  description: string;
  targetDate: string;
  actualDate?: string;
  amount: number;
  actions: TransitionAction[];
  status: 'pending' | 'in-progress' | 'completed' | 'blocked';
  blockReason?: string;
}

/**
 * TransitionAction - Individual action within a phase
 */
export interface TransitionAction {
  id: string;
  actionType: 'sell' | 'transfer' | 'liquidate' | 'convert' | 'register' | 'authorize' | 'other';
  description: string;
  responsible: string;                  // Who does this (steward, advisor, governance?)
  targetDate: string;
  completedDate?: string;
  amount?: number;
  notes?: string;
  economicEventId?: string;             // Link to EconomicEvent for immutability
  status: 'pending' | 'completed' | 'failed';
}

/**
 * ConstitutionalCompliance - Overall assessment of asset positions vs constitutional bounds
 */
export interface ConstitutionalCompliance {
  stewardId: string;
  assessmentDate: string;

  // By Category
  byCategory: {
    category: ResourceCategory;
    totalValue: number;
    constitutionalCeiling: number;
    floorEntitlement: number;
    complianceStatus: 'compliant' | 'at-risk' | 'exceeds-ceiling';
    excess?: number;
    warningLevel: 'none' | 'yellow' | 'orange' | 'red';
  }[];

  // Overall
  overallCompliant: boolean;
  totalExcess: number;                  // Sum across all categories
  categories_at_risk: number;           // How many over ceiling
  estimatedTimeToCompliance?: string;   // "18 months", "3 years", etc.

  // Recommendations
  recommendations: ComplianceRecommendation[];

  // Transition Activity
  activeTransitionPaths: number;
  transitioningAmount: number;          // Currently being reallocated
  completedTransitions: number;
}

/**
 * ComplianceRecommendation - Guidance on reaching constitutional compliance
 */
export interface ComplianceRecommendation {
  id: string;
  priority: 'critical' | 'high' | 'medium' | 'low';
  resourceCategory: ResourceCategory;
  title: string;
  description: string;
  action: string;
  estimatedImpact: string;              // How much this moves toward compliance
  timeline: string;                     // How long this would take
  governanceRequired?: string;          // Any community approval needed?
}

/**
 * CommonsContribution - When asset transitions to community stewardship
 * Records the historical contribution for governance credit
 */
export interface CommonsContribution {
  id: string;
  contributorId: string;
  originalHolding: string;              // What was this (e.g., "$5M Apple stock")
  contributedAmount: number;
  contributedUnit: string;
  destinationCommonsPool: string;       // Which commons received this
  conversionRate?: number;              // How was it valued during transfer
  legacyRole?: string;                  // Does contributor maintain any governance role?
  governanceCredit?: number;            // Recognition in community (reputation, voting power, etc.)
  contributionDate: string;
  transitionPathId: string;             // Link back to transition

  // Transparency
  publicRecognition: boolean;           // Is this publicly acknowledged?
  visibilityLevel: 'private' | 'household' | 'community' | 'public';

  // Economic Integration
  economicEventId: string;              // EconomicEvent for immutability
  commitmentToCommons?: string;         // hREA Commitment if ongoing role
}

// =============================================================================
// Factory Functions for Defaults
// =============================================================================

/**
 * Get the standard dimension for a resource category
 */
export function getDimensionForCategory(category: ResourceCategory): ResourceDimension {
  const dimensions: Record<ResourceCategory, ResourceDimension> = {
    energy: {
      unit: 'hours',
      unitLabel: 'hours',
      unitAbbreviation: 'h',
      standardUnit: 'hours',
    },
    compute: {
      unit: 'percent',
      unitLabel: 'percent',
      unitAbbreviation: '%',
      standardUnit: 'percent',
    },
    water: {
      unit: 'liters',
      unitLabel: 'liters',
      unitAbbreviation: 'L',
      standardUnit: 'liters',
    },
    food: {
      unit: 'calories',
      unitLabel: 'calories',
      unitAbbreviation: 'kcal',
      standardUnit: 'calories',
    },
    shelter: {
      unit: 'square-meters',
      unitLabel: 'square meters',
      unitAbbreviation: 'm²',
      standardUnit: 'square-meters',
    },
    transportation: {
      unit: 'kilometers',
      unitLabel: 'kilometers',
      unitAbbreviation: 'km',
      standardUnit: 'kilometers',
    },
    property: {
      unit: 'count',
      unitLabel: 'items',
      unitAbbreviation: '#',
      standardUnit: 'count',
    },
    equipment: {
      unit: 'count',
      unitLabel: 'items',
      unitAbbreviation: '#',
      standardUnit: 'count',
    },
    inventory: {
      unit: 'units',
      unitLabel: 'units',
      unitAbbreviation: 'u',
      standardUnit: 'units',
    },
    knowledge: {
      unit: 'concepts',
      unitLabel: 'concepts',
      unitAbbreviation: 'c',
      standardUnit: 'concepts',
    },
    reputation: {
      unit: 'score',
      unitLabel: 'reputation score',
      unitAbbreviation: 'pts',
      standardUnit: 'score',
    },
    'financial-asset': {
      unit: 'currency',
      unitLabel: 'currency units',
      unitAbbreviation: '$',
      standardUnit: 'currency',
    },
    uba: {
      unit: 'currency',
      unitLabel: 'currency units',
      unitAbbreviation: '$',
      standardUnit: 'currency',
    },
  };

  return dimensions[category];
}

/**
 * Calculate utilization percentage for a resource
 */
export function calculateUtilization(
  allocated: ResourceMeasure,
  capacity: ResourceMeasure
): number {
  if (capacity.value === 0) return 0;
  return Math.min(100, (allocated.value / capacity.value) * 100);
}

/**
 * Determine health status based on utilization
 */
export function getHealthStatus(utilization: number): 'healthy' | 'warning' | 'critical' {
  if (utilization > 90) return 'critical';
  if (utilization > 75) return 'warning';
  return 'healthy';
}
