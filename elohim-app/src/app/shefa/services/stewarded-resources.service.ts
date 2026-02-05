/**
 * Stewarded Resources Service - Complete Resource Stewardship Management
 *
 * Provides comprehensive management of all resources a human stewards:
 * - Physical resources (water, food, shelter, transportation)
 * - Computing resources (processing, storage, bandwidth)
 * - Human resources (time, attention, relationships)
 * - Economic resources (currency, assets, obligations, UBA/UBI)
 * - Intangible resources (knowledge, reputation)
 *
 * Architecture:
 *   StewardedResourceService → EconomicService → HolochainClientService → Holochain
 *
 * All operations are immutable via EconomicEvent creation.
 * All allocations tracked through hREA Intent/Commitment/Agreement.
 * All usage recorded through Observer protocol attestation.
 *
 * Governance Integration:
 * - Some resources are individually stewarded
 * - Some are household-governed (family decisions)
 * - Some are community-governed (shared capacity)
 * - Constitutional minimums cannot be violated (dignity floors)
 *
 * NOTE: Zome call payloads in this service use snake_case
 * (e.g., resource_id, steward_id) because Holochain zomes are Rust and expect
 * snake_case field names. This cannot be changed without updating the Rust
 * zomes and running a DNA migration.
 */

import { Injectable } from '@angular/core';

// @coverage: 83.7% (2026-02-05)

import { firstValueFrom } from 'rxjs';

import { REAAction } from '@app/elohim/models/rea-bridge.model';
import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import {
  StewardedResource,
  ResourceCategory,
  ResourceMeasure,
  AllocationBlock,
  UsageRecord,
  CategorySummary,
  StewardedResourceDashboard,
  ResourceAlert,
  ResourceInsight,
  FinancialAsset,
  FinancialObligation,
  ConstitutionalLimit,
  IncomeStream,
  DignityFloor,
  FinancialStewardshipView,
  FinancialRecommendation,
  getDimensionForCategory,
  calculateUtilization,
  getHealthStatus,
} from '@app/shefa/models/stewarded-resources.model';

import { EconomicService } from './economic.service';

// =============================================================================
// Constants
// =============================================================================

/**
 * Resource category constants to avoid string duplication
 */
const RESOURCE_CATEGORY = {
  FINANCIAL_ASSET: 'financial-asset' as const,
  ENERGY: 'energy' as const,
  COMPUTE: 'compute' as const,
} as const;

/**
 * Constitutional economics thresholds
 */
const CONSTITUTIONAL_THRESHOLDS = {
  CEILING_APPROACHING_MULTIPLIER: 1.5, // 150% of ceiling triggers warnings
  EXCESS_HIGH_THRESHOLD: 0.5, // 50% excess triggers red warning
} as const;

/**
 * Type aliases for constitutional economics
 */
type PositionRelativeToFloor =
  | 'below-floor'
  | 'at-floor'
  | 'in-safe-zone'
  | 'above-ceiling'
  | 'far-above-ceiling';

type ComplianceStatus =
  | 'compliant'
  | 'approaching-limit'
  | 'exceeds-ceiling'
  | 'far-exceeds-ceiling';

type WarningLevel = 'none' | 'yellow' | 'orange' | 'red';

type CategoryComplianceStatus = 'compliant' | 'at-risk' | 'exceeds-ceiling';

// =============================================================================
// Stewarded Resource Service
// =============================================================================

@Injectable({
  providedIn: 'root',
})
export class StewardedResourceService {
  constructor(
    private readonly holochain: HolochainClientService,
    private readonly economicService: EconomicService
  ) {}

  // =========================================================================
  // Resource Creation & Management
  // =========================================================================

  /**
   * Create a new stewarded resource
   * Initializes tracking, allocations, and economic events
   */
  async createResource(
    stewardId: string,
    category: ResourceCategory,
    subcategory: string,
    name: string,
    totalCapacity: ResourceMeasure,
    options?: {
      description?: string;
      governanceLevel?: string;
      observerEnabled?: boolean;
      permanentReserve?: ResourceMeasure;
    }
  ): Promise<StewardedResource> {
    const now = new Date().toISOString();
    const dimension = getDimensionForCategory(category);

    const resource: StewardedResource = {
      id: `res-${Date.now()}-${(crypto.getRandomValues(new Uint32Array(1))[0] / 2 ** 32).toString(36).substring(2, 11)}`,
      resourceNumber: `RES-${Date.now()}`,
      stewardId,
      category,
      subcategory,
      name,
      description: options?.description,
      dimension,
      totalCapacity,
      permanentReserve: options?.permanentReserve,
      allocatableCapacity: {
        value:
          (options?.permanentReserve?.value ?? 0) > 0
            ? totalCapacity.value - (options?.permanentReserve?.value ?? 0)
            : totalCapacity.value,
        unit: totalCapacity.unit,
      },
      totalAllocated: { value: 0, unit: totalCapacity.unit },
      totalReserved: { value: 0, unit: totalCapacity.unit },
      totalUsed: { value: 0, unit: totalCapacity.unit },
      available: { value: totalCapacity.value, unit: totalCapacity.unit },
      allocations: [],
      allocationStrategy: 'manual',
      governanceLevel: (options?.governanceLevel ?? 'individual') as
        | 'individual'
        | 'household'
        | 'community'
        | 'network'
        | 'constitutional',
      canModifyAllocations: true,
      observerEnabled: options?.observerEnabled ?? false,
      recentUsage: [],
      trends: [],
      allocationEventIds: [],
      usageEventIds: [],
      isShared: false,
      visibility: 'private',
      dataQuality: 'manual',
      createdAt: now,
      updatedAt: now,
    };

    // Persist to Holochain
    await this.persistResource(resource);

    // Create initialization event
    await this.economicService.createEvent({
      action: 'produce',
      providerId: stewardId,
      receiverId: stewardId,
      resourceConformsTo: `${category}/${subcategory}`,
      resourceQuantityValue: totalCapacity.value,
      resourceQuantityUnit: totalCapacity.unit,
      note: `Created stewarded resource: ${name}`,
    });

    return resource;
  }

  /**
   * Get a stewarded resource by ID
   */
  async getResource(resourceId: string): Promise<StewardedResource | null> {
    try {
      const response = await this.holochain.callZome({
        zomeName: 'content_store',
        fnName: 'get_stewarded_resource',
        payload: { resource_id: resourceId },
      });
      return (response.data as StewardedResource) ?? null;
    } catch (err) {
      // Resource not found or error accessing Holochain - this is expected behavior
      // when resource doesn't exist, so we return null instead of throwing
      if (err instanceof Error && !err.message.includes('not found')) {
        console.warn('[StewardedResourceService] Unexpected error fetching resource:', err);
      }
      return null;
    }
  }

  /**
   * Get all resources for a steward
   */
  async getStewardResources(stewardId: string): Promise<StewardedResource[]> {
    try {
      const response = await this.holochain.callZome({
        zomeName: 'content_store',
        fnName: 'get_steward_resources',
        payload: { steward_id: stewardId },
      });
      return (response.data as StewardedResource[]) ?? [];
    } catch (err) {
      // Error accessing Holochain - return empty array for graceful degradation
      if (err instanceof Error) {
        console.warn('[StewardedResourceService] Error fetching steward resources:', err.message);
      }
      return [];
    }
  }

  /**
   * Get resources in a category
   */
  async getResourcesByCategory(
    stewardId: string,
    category: ResourceCategory
  ): Promise<StewardedResource[]> {
    const all = await this.getStewardResources(stewardId);
    return all.filter(r => r.category === category);
  }

  // =========================================================================
  // Allocation Management
  // =========================================================================

  /**
   * Create an allocation block for a resource
   * Links to hREA Commitment if governed
   */
  async createAllocation(
    resourceId: string,
    label: string,
    allocatedAmount: ResourceMeasure,
    options?: {
      description?: string;
      governanceLevel?: string;
      priority?: number;
    }
  ): Promise<AllocationBlock> {
    const now = new Date().toISOString();

    const allocation: AllocationBlock = {
      id: `alloc-${Date.now()}-${(crypto.getRandomValues(new Uint32Array(1))[0] / 2 ** 32).toString(36).substring(2, 11)}`,
      resourceId,
      label,
      description: options?.description,
      allocated: allocatedAmount,
      used: { value: 0, unit: allocatedAmount.unit },
      reserved: { value: 0, unit: allocatedAmount.unit },
      governanceLevel: (options?.governanceLevel ?? 'individual') as
        | 'individual'
        | 'household'
        | 'community'
        | 'network'
        | 'constitutional',
      priority: options?.priority ?? 5,
      utilization: 0,
      createdAt: now,
      updatedAt: now,
    };

    // Persist allocation
    await this.persistAllocation(resourceId, allocation);

    // Create event
    await this.economicService.createEvent({
      action: 'transfer',
      providerId: resourceId,
      receiverId: resourceId,
      resourceQuantityValue: allocatedAmount.value,
      resourceQuantityUnit: allocatedAmount.unit,
      note: `Allocated ${allocatedAmount.value} ${allocatedAmount.unit} to: ${label}`,
    });

    return allocation;
  }

  /**
   * Record usage of an allocated resource
   * Links to Observer attestation if available
   */
  async recordUsage(
    resourceId: string,
    allocationBlockId: string,
    amount: ResourceMeasure,
    options?: {
      action?: string;
      observerAttestationId?: string;
      note?: string;
    }
  ): Promise<UsageRecord> {
    const now = new Date().toISOString();

    const usage: UsageRecord = {
      id: `use-${Date.now()}-${(crypto.getRandomValues(new Uint32Array(1))[0] / 2 ** 32).toString(36).substring(2, 11)}`,
      resourceId,
      allocationBlockId,
      action: options?.action ?? 'use',
      quantity: amount,
      observerAttestationId: options?.observerAttestationId,
      timestamp: now,
      note: options?.note,
    };

    // Persist usage
    await this.persistUsage(resourceId, usage);

    // Create economic event for immutability
    const event = await firstValueFrom(
      this.economicService.createEvent({
        action: (options?.action ?? 'use') as REAAction,
        providerId: resourceId,
        receiverId: resourceId,
        resourceQuantityValue: amount.value,
        resourceQuantityUnit: amount.unit,
        note: options?.note ?? `Resource usage recorded`,
      })
    );

    usage.economicEventId = event.id;

    return usage;
  }

  /**
   * Update utilization for an allocation (aggregate from usage)
   */
  async updateAllocationUtilization(
    resourceId: string,
    allocationBlockId: string,
    newUsed: ResourceMeasure
  ): Promise<AllocationBlock | null> {
    const resource = await this.getResource(resourceId);
    if (!resource) return null;

    const allocation = resource.allocations.find(a => a.id === allocationBlockId);
    if (!allocation) return null;

    allocation.used = newUsed;
    allocation.utilization = calculateUtilization(newUsed, allocation.allocated);
    allocation.updatedAt = new Date().toISOString();

    await this.persistAllocation(resourceId, allocation);
    return allocation;
  }

  // =========================================================================
  // Dashboard & Aggregation
  // =========================================================================

  /**
   * Get category summary - aggregates all resources in a category
   */
  async getCategorySummary(
    stewardId: string,
    category: ResourceCategory
  ): Promise<CategorySummary> {
    const resources = await this.getResourcesByCategory(stewardId, category);

    const totalCapacity = resources.reduce((sum, r) => sum + r.totalCapacity.value, 0);
    const totalAllocated = resources.reduce((sum, r) => sum + r.totalAllocated.value, 0);
    const totalUsed = resources.reduce((sum, r) => sum + r.totalUsed.value, 0);

    const utilizationPercent = totalCapacity > 0 ? (totalUsed / totalCapacity) * 100 : 0;
    const healthStatus = getHealthStatus(utilizationPercent);

    return {
      category,
      resources,
      totalCapacity: { value: totalCapacity, unit: getDimensionForCategory(category).unit },
      totalAllocated: { value: totalAllocated, unit: getDimensionForCategory(category).unit },
      totalUsed: { value: totalUsed, unit: getDimensionForCategory(category).unit },
      utilizationPercent,
      healthStatus,
    };
  }

  /**
   * Build complete stewardship dashboard
   * Shows overview of all resources, health, and alerts
   */
  async buildDashboard(stewardId: string): Promise<StewardedResourceDashboard> {
    const now = new Date().toISOString();
    const allResources = await this.getStewardResources(stewardId);

    // Get category summaries
    const categories: CategorySummary[] = [];
    const categorySet = new Set<ResourceCategory>(allResources.map(r => r.category));

    for (const category of categorySet) {
      const summary = await this.getCategorySummary(stewardId, category);
      categories.push(summary);
    }

    // Calculate metrics
    const totalResourcesTracked = allResources.length;
    const categoriesCovered = categories.length;
    const overallUtilization = this.calculateWeightedUtilization(categories);
    const fullyAllocatedCount = allResources.filter(r => r.available.value <= 0).length;

    // Generate alerts and insights
    const alerts = this.generateAlerts(allResources, categories);
    const insights = this.generateInsights(allResources, categories);

    // Get recent activity
    const recentAllocations = allResources
      .flatMap(r => r.allocations)
      .sort((a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime())
      .slice(0, 5);

    const recentUsage = allResources
      .flatMap(r => r.recentUsage)
      .sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime())
      .slice(0, 10);

    const healthStatus = this.calculateHealthStatus(overallUtilization);

    return {
      stewardId,
      stewardName: '', // Would fetch from Human service
      governanceLevel: 'individual',
      categories,
      metrics: {
        totalResourcesTracked,
        categoriesCovered,
        overallUtilization,
        fullyAllocatedCount,
        healthStatus,
      },
      alerts,
      insights,
      recentAllocations,
      recentUsage,
      lastUpdatedAt: now,
    };
  }

  /**
   * Get constitutional limits for a resource category
   * Defines floor (dignity minimum) and ceiling (wise stewardship maximum)
   */
  // eslint-disable-next-line @typescript-eslint/require-await
  async getConstitutionalLimits(category: ResourceCategory): Promise<ConstitutionalLimit | null> {
    // These would come from Elohim governance documents
    // Here are reasonable defaults implementing donut economy principles
    const limits: Record<string, ConstitutionalLimit> = {
      [RESOURCE_CATEGORY.FINANCIAL_ASSET]: {
        id: 'limit-wealth-ceiling',
        resourceCategory: RESOURCE_CATEGORY.FINANCIAL_ASSET,
        name: 'Wealth Ceiling (Limitarianism)',
        description: 'Constitutional maximum for net worth holding',
        floorValue: 75000, // $75k minimum dignity floor
        floorUnit: 'USD',
        floorRationale: 'Enables basic needs: food, shelter, healthcare, education, dignity',
        floorEnforced: true,
        ceilingValue: 10000000, // $10M reasonable ceiling
        ceilingUnit: 'USD',
        ceilingRationale:
          'Beyond this, accumulation enables extraction. Supports community stewardship.',
        ceilingEnforced: false, // Soft enforcement initially (voluntary → progressive → hard)
        safeMinValue: 75000,
        safeMaxValue: 10000000,
        safeZoneDescription:
          'Flourishing stewardship - adequate for personal/family + community contribution',
        governanceLevel: 'Elohim-network',
        constitutionalBasis: 'Part III: Constitutional Economics',
        enforcementMethod: 'progressive' as const, // Starting with voluntary, moving to incentive-based
        transitionDeadline: '2035-12-31', // 10 year transition period
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      },
      energy: {
        id: 'limit-time-ceiling',
        resourceCategory: 'energy',
        name: 'Time Allocation Ceiling',
        description: "Constitutional maximum for work/extraction of another's time",
        floorValue: 40, // 40 hours/week minimum stewardship of self
        floorUnit: 'hours/week',
        floorRationale: 'Enables human flourishing - rest, relationships, learning',
        floorEnforced: true,
        ceilingValue: 100, // 100 hours max reasonable allocation
        ceilingUnit: 'hours/week',
        ceilingRationale: 'Beyond this is extraction. Community work respects boundaries.',
        ceilingEnforced: false,
        safeMinValue: 40,
        safeMaxValue: 60,
        safeZoneDescription: 'Balanced life - work, rest, relationships, contribution',
        governanceLevel: 'individual/household',
        constitutionalBasis: 'Part II: Stewardship & Human Flourishing',
        enforcementMethod: 'voluntary',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      },
      compute: {
        id: 'limit-node-capacity',
        resourceCategory: 'compute',
        name: 'Node Capacity Ceiling',
        description: 'Constitutional sharing of node resources',
        floorValue: 10, // 10% minimum for personal autonomy
        floorUnit: 'percent',
        floorRationale: 'Minimum capacity for self-sovereign apps',
        floorEnforced: true,
        ceilingValue: 80, // 80% max can be allocated before commonwealth kicks in
        ceilingUnit: 'percent',
        ceilingRationale: 'Beyond this, excess capacity returns to commons for community benefit',
        ceilingEnforced: false,
        safeMinValue: 10,
        safeMaxValue: 80,
        safeZoneDescription: 'Personal autonomy + community contribution balance',
        governanceLevel: 'network',
        constitutionalBasis: 'Part IV: Autonomous Infrastructure',
        enforcementMethod: 'progressive' as const,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      },
    };

    return limits[category] ?? null;
  }

  /**
   * Assess where a resource position stands relative to constitutional bounds
   */
  async assessResourcePosition(
    resourceId: string,
    stewardId: string,
    category: ResourceCategory,
    currentValue: number,
    unit: string
  ): Promise<{
    resourceId: string;
    stewardId: string;
    resourceCategory: ResourceCategory;
    currentValue: number;
    unit: string;
    constitutionalLimit: ConstitutionalLimit | null;
    positionRelativeToFloor: PositionRelativeToFloor;
    distanceFromFloor: number;
    distanceFromCeiling: number;
    excessAboveCeiling?: number;
    excessPercentage?: number;
    surplusAvailableForTransition: number;
    complianceStatus: ComplianceStatus;
    warningLevel: WarningLevel;
    onTransitionPath: boolean;
  }> {
    const limit = await this.getConstitutionalLimits(category);

    if (!limit) {
      throw new Error(`No constitutional limit defined for category: ${category}`);
    }

    const distanceFromFloor = currentValue - limit.floorValue;
    const distanceFromCeiling = currentValue - limit.ceilingValue;

    // Extract position calculation to helper method
    const { positionRelativeToFloor, complianceStatus, warningLevel } =
      this.calculateConstitutionalPosition(currentValue, limit);

    const excessAboveCeiling = Math.max(0, currentValue - limit.ceilingValue);
    const excessPercentage =
      limit.ceilingValue > 0 ? (excessAboveCeiling / limit.ceilingValue) * 100 : 0;

    return {
      resourceId,
      stewardId,
      resourceCategory: category,
      currentValue,
      unit,
      constitutionalLimit: limit,
      positionRelativeToFloor,
      distanceFromFloor,
      distanceFromCeiling,
      excessAboveCeiling: excessAboveCeiling > 0 ? excessAboveCeiling : undefined,
      excessPercentage: excessAboveCeiling > 0 ? excessPercentage : undefined,
      surplusAvailableForTransition: excessAboveCeiling,
      complianceStatus,
      warningLevel,
      onTransitionPath: excessAboveCeiling > 0,
    };
  }

  /**
   * Build constitutional compliance report
   * Shows steward's position relative to all constitutional limits
   */
  async buildComplianceReport(stewardId: string): Promise<{
    stewardId: string;
    assessmentDate: string;
    byCategory: {
      category: ResourceCategory;
      totalValue: number;
      constitutionalCeiling: number;
      floorEntitlement: number;
      complianceStatus: CategoryComplianceStatus;
      excess?: number;
      warningLevel: WarningLevel;
    }[];
    overallCompliant: boolean;
    totalExcess: number;
    categories_at_risk: number;
    estimatedTimeToCompliance?: string;
    recommendations: {
      id: string;
      priority: string;
      resourceCategory: ResourceCategory;
      title: string;
      description: string;
      action: string;
      estimatedImpact: string;
      timeline: string;
      governanceRequired: string;
    }[];
    activeTransitionPaths: number;
    transitioningAmount: number;
    completedTransitions: number;
  }> {
    const allResources = await this.getStewardResources(stewardId);
    const now = new Date().toISOString();

    const byCategory: {
      category: ResourceCategory;
      totalValue: number;
      constitutionalCeiling: number;
      floorEntitlement: number;
      complianceStatus: 'compliant' | 'at-risk' | 'exceeds-ceiling';
      excess?: number;
      warningLevel: 'none' | 'yellow' | 'orange' | 'red';
    }[] = [];
    let totalExcess = 0;
    let categoriesAtRisk = 0;

    // Group resources by category and assess each
    const categorized = new Map<ResourceCategory, StewardedResource[]>();
    for (const resource of allResources) {
      if (!categorized.has(resource.category)) {
        categorized.set(resource.category, []);
      }
      categorized.get(resource.category)!.push(resource);
    }

    for (const [category, resources] of categorized) {
      const limit = await this.getConstitutionalLimits(category);
      if (!limit) continue;

      const totalValue = resources.reduce((sum, r) => sum + r.totalCapacity.value, 0);
      const excess = Math.max(0, totalValue - limit.ceilingValue);

      // Use helper method to calculate compliance status
      const { complianceStatus, warningLevel } = this.calculateCategoryCompliance(
        totalValue,
        limit
      );

      byCategory.push({
        category,
        totalValue,
        constitutionalCeiling: limit.ceilingValue,
        floorEntitlement: limit.floorValue,
        complianceStatus,
        excess: excess > 0 ? excess : undefined,
        warningLevel,
      });

      totalExcess += excess;
      if (excess > 0) {
        categoriesAtRisk++;
      }
    }

    // Generate recommendations
    const recommendations: {
      id: string;
      priority: string;
      resourceCategory: ResourceCategory;
      title: string;
      description: string;
      action: string;
      estimatedImpact: string;
      timeline: string;
      governanceRequired: string;
    }[] = [];
    if (totalExcess > 0) {
      recommendations.push({
        id: 'rec-excess-assets',
        priority: 'high',
        resourceCategory: RESOURCE_CATEGORY.FINANCIAL_ASSET,
        title: 'Constitutional Limit Exceeded',
        description: `Total assets exceed constitutional ceiling by $${totalExcess.toFixed(2)}. This wealth represents capacity to serve the commons.`,
        action: 'Explore transition pathways for excess assets to community stewardship',
        estimatedImpact: `$${totalExcess.toFixed(2)} could be reallocated`,
        timeline: '6-24 months for phased transition',
        governanceRequired: 'Community dialogue on commons beneficiary',
      });
    }

    return {
      stewardId,
      assessmentDate: now,
      byCategory,
      overallCompliant: totalExcess === 0,
      totalExcess,
      categories_at_risk: categoriesAtRisk,
      estimatedTimeToCompliance: totalExcess > 0 ? '18 months' : undefined,
      recommendations,
      activeTransitionPaths: 0, // Would query TransitionPath entries
      transitioningAmount: 0,
      completedTransitions: 0,
    };
  }

  /**
   * Initiate a transition path for excess assets
   * Helps steward navigate from excess to constitutional compliance
   */
  async initiateTransitionPath(
    resourceId: string,
    stewardId: string,
    excessAmount: number,
    proposedSplits: { amount: number; [key: string]: unknown }[],
    governanceLevel: string
  ): Promise<{
    id: string;
    resourceId: string;
    stewardId: string;
    assetName: string;
    currentValue: number;
    constitutionalCeiling: number;
    excess: number;
    proposedSplits: { amount: number; [key: string]: unknown }[];
    totalProposedExcess: number;
    status: string;
    proposedAt: string;
    negotiationDeadline: string;
    governanceLevel: string;
    approvalStatus: string;
    executionPhases: unknown[];
    transitionEventIds: unknown[];
    transparencyLevel: string;
    createdAt: string;
    updatedAt: string;
  }> {
    const now = new Date().toISOString();

    const transitionPath = {
      id: `trans-${Date.now()}-${(crypto.getRandomValues(new Uint32Array(1))[0] / 2 ** 32).toString(36).substring(2, 11)}`,
      resourceId,
      stewardId,
      assetName: '', // Would come from resource
      currentValue: 0, // Would come from resource
      constitutionalCeiling: 0, // Would come from limit
      excess: excessAmount,
      proposedSplits,
      totalProposedExcess: proposedSplits.reduce((sum, s) => sum + s.amount, 0),
      status: 'proposal',
      proposedAt: now,
      negotiationDeadline: new Date(Date.now() + 90 * 24 * 60 * 60 * 1000).toISOString(), // 90 days
      governanceLevel,
      approvalStatus: 'pending',
      executionPhases: [],
      transitionEventIds: [],
      transparencyLevel: 'community', // Transparent process
      createdAt: now,
      updatedAt: now,
    };

    // Persist and create governance proposal
    await this.persistTransitionPath(transitionPath);

    // Create proposal event in governance system (using 'produce' to create the proposal)
    await this.economicService.createEvent({
      action: 'produce',
      providerId: stewardId,
      receiverId: stewardId,
      note: `Proposed constitutional transition: ${excessAmount} units to commons stewardship`,
    });

    return transitionPath;
  }

  /**
   * Execute a transition phase for an asset
   * Records each step toward constitutional compliance
   */
  async executeTransitionPhase(
    transitionPathId: string,
    phaseNumber: number,
    actions: {
      id?: string;
      actionType?: string;
      responsible?: string;
      amount?: number;
      description?: string;
      economicEventId?: string;
      status?: string;
      [key: string]: unknown;
    }[]
  ): Promise<{
    id: string;
    sequenceNumber: number;
    name: string;
    description: string;
    targetDate: string;
    amount: number;
    actions: {
      id?: string;
      actionType?: string;
      responsible?: string;
      amount?: number;
      description?: string;
      economicEventId?: string;
      status?: string;
      [key: string]: unknown;
    }[];
    status: string;
    blockReason?: string;
  }> {
    const phase = {
      id: `phase-${transitionPathId}-${phaseNumber}`,
      sequenceNumber: phaseNumber,
      name: `Phase ${phaseNumber} Execution`,
      description: '',
      targetDate: new Date(Date.now() + 30 * 24 * 60 * 60 * 1000).toISOString(),
      amount: actions.reduce((sum, a) => sum + (a.amount ?? 0), 0),
      actions,
      status: 'in-progress',
    };

    // Execute each action
    for (const action of actions) {
      try {
        // Create immutable economic event for each transfer
        const event = await firstValueFrom(
          this.economicService.createEvent({
            action: (action.actionType ?? 'transfer') as REAAction,
            providerId: action.responsible ?? '',
            receiverId: action.responsible ?? '',
            resourceQuantityValue: action.amount ?? 0,
            note: action.description,
          })
        );

        action.economicEventId = event.id;
        action.status = 'completed';
      } catch (err) {
        action.status = 'failed';
        phase.status = 'blocked';
        // @ts-expect-error - blockReason not in type definition but used at runtime
        phase.blockReason = `Action ${action.id} failed: ${err}`;
      }
    }

    // Persist phase
    await this.persistTransitionPhase(transitionPathId, phase);

    return phase;
  }

  /**
   * Build financial stewardship view
   * Integrates income, obligations, assets, and dignity floor
   */
  async buildFinancialView(stewardId: string): Promise<FinancialStewardshipView> {
    const assets = (await this.getResourcesByCategory(
      stewardId,
      RESOURCE_CATEGORY.FINANCIAL_ASSET
    )) as FinancialAsset[];

    // Aggregate income streams using helper method
    const { allIncomeStreams, guaranteedMonthlyIncome, expectedMonthlyIncome } =
      this.aggregateIncomeStreams(assets);

    // Aggregate obligations using helper method
    const { allObligations, monthlyObligations, totalLiability } =
      this.aggregateObligations(assets);

    // Build dignity floor
    const dignityFloor = this.buildDignityFloor(
      stewardId,
      guaranteedMonthlyIncome,
      monthlyObligations,
      allIncomeStreams
    );

    // Calculate health
    const monthlyDifference = guaranteedMonthlyIncome - monthlyObligations;
    const financialHealth = this.assessFinancialHealth(
      guaranteedMonthlyIncome,
      monthlyObligations,
      totalLiability,
      dignityFloor
    );

    // Generate recommendations
    const recommendations = this.generateFinancialRecommendations(
      guaranteedMonthlyIncome,
      expectedMonthlyIncome,
      monthlyObligations,
      allObligations,
      dignityFloor
    );

    // Get total assets
    const totalAssets = assets.reduce((sum, a) => sum + a.accountBalance, 0);

    return {
      humanId: stewardId,
      humanName: '',
      monthlyIncome: guaranteedMonthlyIncome,
      expectedMonthlyIncome,
      incomeStreams: allIncomeStreams,
      incomeStability: this.assessIncomeStability(allIncomeStreams),
      monthlyObligations,
      totalLiabilities: totalLiability,
      obligations: allObligations,
      dignityFloor,
      onTrackForDignity: monthlyDifference >= dignityFloor.totalMonthlyFloor - monthlyObligations,
      monthlyDifference,
      financialHealth,
      recommendations,
      assets,
      totalAssets,
      lastUpdatedAt: new Date().toISOString(),
    };
  }

  // =========================================================================
  // Dashboard & Analysis Helpers
  // =========================================================================

  /**
   * Calculate health status based on utilization percentage
   * Extracted from buildDashboard to avoid nested ternary
   */
  private calculateHealthStatus(utilization: number): 'healthy' | 'warning' | 'critical' {
    if (utilization > 90) {
      return 'critical';
    }
    if (utilization > 75) {
      return 'warning';
    }
    return 'healthy';
  }

  // =========================================================================
  // Constitutional Economics Helpers
  // =========================================================================

  /**
   * Calculate constitutional position, compliance status, and warning level
   * Extracted from assessResourcePosition to reduce complexity
   */
  private calculateConstitutionalPosition(
    currentValue: number,
    limit: ConstitutionalLimit
  ): {
    positionRelativeToFloor: PositionRelativeToFloor;
    complianceStatus: ComplianceStatus;
    warningLevel: WarningLevel;
  } {
    const ceilingThreshold =
      limit.ceilingValue * CONSTITUTIONAL_THRESHOLDS.CEILING_APPROACHING_MULTIPLIER;

    if (currentValue < limit.floorValue) {
      return {
        positionRelativeToFloor: 'below-floor',
        complianceStatus: 'compliant',
        warningLevel: 'red', // Critical - dignity floor not met
      };
    }

    if (currentValue === limit.floorValue) {
      return {
        positionRelativeToFloor: 'at-floor',
        complianceStatus: 'compliant',
        warningLevel: 'yellow',
      };
    }

    if (currentValue < limit.ceilingValue) {
      return {
        positionRelativeToFloor: 'in-safe-zone',
        complianceStatus: 'compliant',
        warningLevel: 'none',
      };
    }

    if (currentValue < ceilingThreshold) {
      return {
        positionRelativeToFloor: 'above-ceiling',
        complianceStatus: 'approaching-limit',
        warningLevel: 'yellow',
      };
    }

    return {
      positionRelativeToFloor: 'far-above-ceiling',
      complianceStatus: 'far-exceeds-ceiling',
      warningLevel: 'red',
    };
  }

  /**
   * Calculate compliance status for a resource category
   * Extracted from buildComplianceReport to reduce complexity
   */
  private calculateCategoryCompliance(
    totalValue: number,
    limit: ConstitutionalLimit
  ): {
    complianceStatus: CategoryComplianceStatus;
    warningLevel: WarningLevel;
  } {
    const excess = Math.max(0, totalValue - limit.ceilingValue);

    if (excess === 0) {
      const complianceStatus = totalValue >= limit.floorValue ? 'compliant' : 'at-risk';
      return {
        complianceStatus,
        warningLevel: 'none',
      };
    }

    const excessHighThreshold =
      limit.ceilingValue * CONSTITUTIONAL_THRESHOLDS.EXCESS_HIGH_THRESHOLD;
    const warningLevel = excess > excessHighThreshold ? 'red' : 'orange';

    return {
      complianceStatus: 'exceeds-ceiling',
      warningLevel,
    };
  }

  // =========================================================================
  // Financial Health Analysis
  // =========================================================================

  /**
   * Aggregate income streams from financial assets
   * Extracted from buildFinancialView to reduce complexity
   */
  private aggregateIncomeStreams(assets: FinancialAsset[]): {
    allIncomeStreams: IncomeStream[];
    guaranteedMonthlyIncome: number;
    expectedMonthlyIncome: number;
  } {
    const allIncomeStreams: IncomeStream[] = [];
    let guaranteedMonthlyIncome = 0;
    let expectedMonthlyIncome = 0;

    for (const asset of assets) {
      if (asset.incomeStreams) {
        allIncomeStreams.push(...asset.incomeStreams);
        for (const stream of asset.incomeStreams) {
          if (stream.status === 'active') {
            const monthlyAmount = this.normalizeToMonthly(stream.amount, stream.frequency);
            if (stream.isGuaranteed) {
              guaranteedMonthlyIncome += monthlyAmount;
            }
            expectedMonthlyIncome += monthlyAmount * (stream.confidence / 100);
          }
        }
      }
    }

    return { allIncomeStreams, guaranteedMonthlyIncome, expectedMonthlyIncome };
  }

  /**
   * Aggregate obligations from financial assets
   * Extracted from buildFinancialView to reduce complexity
   */
  private aggregateObligations(assets: FinancialAsset[]): {
    allObligations: FinancialObligation[];
    monthlyObligations: number;
    totalLiability: number;
  } {
    const allObligations: FinancialObligation[] = [];
    let monthlyObligations = 0;
    let totalLiability = 0;

    for (const asset of assets) {
      if (asset.obligations) {
        allObligations.push(...asset.obligations);
        for (const obligation of asset.obligations) {
          monthlyObligations += obligation.monthlyPayment;
          totalLiability += obligation.remainingAmount;
        }
      }
    }

    return { allObligations, monthlyObligations, totalLiability };
  }

  /**
   * Build dignity floor - minimum monthly needs
   */
  private buildDignityFloor(
    humanId: string,
    monthlyIncome: number,
    _monthlyObligations: number,
    _incomeStreams: IncomeStream[]
  ): DignityFloor {
    // These would come from governance constitution
    // For now, using reasonable minimums
    const foodDailyAmount = 10; // $10/day for food
    const shelterMonthlyAmount = 800; // $800/month rent minimum
    const healthcareMonthlyAmount = 200; // $200/month healthcare
    const internetMonthlyAmount = 50; // $50/month internet
    const transportMonthlyAmount = 100; // $100/month transport

    const totalMonthlyFloor =
      foodDailyAmount * 30 +
      shelterMonthlyAmount +
      healthcareMonthlyAmount +
      internetMonthlyAmount +
      transportMonthlyAmount;

    const floorMet = monthlyIncome >= totalMonthlyFloor;
    const monthlyShortfall = floorMet ? 0 : totalMonthlyFloor - monthlyIncome;

    return {
      humanId,
      eligible: true,
      foodDailyAmount,
      shelterMonthlyAmount,
      healthcareMonthlyAmount,
      internetMonthlyAmount,
      transportMonthlyAmount,
      totalMonthlyFloor,
      floorMet,
      monthlyShortfall,
      verifiedAt: new Date().toISOString(),
    };
  }

  /**
   * Assess financial health based on multiple factors
   */
  private assessFinancialHealth(
    income: number,
    obligations: number,
    totalLiability: number,
    dignityFloor: DignityFloor
  ): 'healthy' | 'stable' | 'at-risk' | 'critical' {
    if (!dignityFloor.floorMet) {
      return 'critical'; // Below dignity floor
    }
    if (income - obligations < 100) {
      return 'at-risk'; // Little buffer
    }
    if (totalLiability > income * 6) {
      return 'at-risk'; // High debt
    }
    if (income - obligations > 500) {
      return 'healthy'; // Good surplus
    }
    return 'stable'; // Adequate
  }

  /**
   * Generate financial recommendations
   */
  private generateFinancialRecommendations(
    guaranteedIncome: number,
    expectedIncome: number,
    monthlyObligations: number,
    allObligations: FinancialObligation[],
    dignityFloor: DignityFloor
  ): FinancialRecommendation[] {
    const recommendations: FinancialRecommendation[] = [];

    // Check if below dignity floor
    if (!dignityFloor.floorMet) {
      recommendations.push({
        id: 'rec-dignity-floor',
        priority: 'critical',
        title: 'Below Dignity Floor',
        description: `Your current income (${guaranteedIncome}) is below your minimum needs (${dignityFloor.totalMonthlyFloor}). You may be eligible for UBA support.`,
        recommendedAction: 'Check UBA eligibility in your community',
        estimatedImpact: `Could provide $${dignityFloor.monthlyShortfall}/month`,
      });
    }

    // Check for high-interest debt
    const highInterestDebt = allObligations.filter(o => (o.interest ?? 0) > 10);
    if (highInterestDebt.length > 0) {
      recommendations.push({
        id: 'rec-high-interest',
        priority: 'high',
        title: 'High-Interest Debt',
        description: `You have ${highInterestDebt.length} high-interest obligations. Paying these down could save significantly.`,
        recommendedAction: 'Prioritize paying off these obligations',
        relatedTo: highInterestDebt.map(d => d.id),
      });
    }

    // Check income stability
    if (expectedIncome > guaranteedIncome * 1.5) {
      recommendations.push({
        id: 'rec-income-stability',
        priority: 'medium',
        title: 'Income Uncertainty',
        description: `Your expected income varies significantly from guaranteed income. Build a buffer for lean months.`,
        recommendedAction: 'Save 3-6 months of expenses as emergency fund',
      });
    }

    return recommendations;
  }

  // =========================================================================
  // Helper Methods
  // =========================================================================

  /**
   * Persist resource to Holochain
   */
  private async persistResource(resource: StewardedResource): Promise<void> {
    try {
      await this.holochain.callZome({
        zomeName: 'content_store',
        fnName: 'create_stewarded_resource',
        payload: resource,
      });
    } catch (err) {
      // Re-throw persistence errors to caller for upstream handling
      throw new Error(
        `Failed to persist resource: ${err instanceof Error ? err.message : String(err)}`
      );
    }
  }

  /**
   * Persist allocation block
   */
  private async persistAllocation(resourceId: string, allocation: AllocationBlock): Promise<void> {
    try {
      await this.holochain.callZome({
        zomeName: 'content_store',
        fnName: 'create_allocation_block',
        payload: {
          resource_id: resourceId,
          allocation,
        },
      });
    } catch (err) {
      // Re-throw persistence errors to caller for upstream handling
      throw new Error(
        `Failed to persist allocation: ${err instanceof Error ? err.message : String(err)}`
      );
    }
  }

  /**
   * Persist usage record
   */
  private async persistUsage(resourceId: string, usage: UsageRecord): Promise<void> {
    await this.holochain.callZome({
      zomeName: 'content_store',
      fnName: 'record_resource_usage',
      payload: {
        resource_id: resourceId,
        usage,
      },
    });
  }

  /**
   * Calculate weighted utilization across categories
   */
  private calculateWeightedUtilization(categories: CategorySummary[]): number {
    if (categories.length === 0) return 0;

    const weights: Record<ResourceCategory, number> = {
      energy: 0.2, // 20% weight - time/attention critical
      compute: 0.15, // 15% weight - node capacity important
      water: 0.1,
      food: 0.1,
      shelter: 0.1,
      transportation: 0.05,
      property: 0.05,
      equipment: 0.05,
      inventory: 0.05,
      knowledge: 0, // Knowledge is unlimited
      reputation: 0, // Reputation is unlimited
      'financial-asset': 0.15, // 15% weight - money important
      uba: 0, // UBA is safety net
    };

    let totalWeight = 0;
    let weightedUtilization = 0;

    for (const cat of categories) {
      const weight = weights[cat.category] ?? 0.05;
      weightedUtilization += cat.utilizationPercent * weight;
      totalWeight += weight;
    }

    return totalWeight > 0 ? weightedUtilization / totalWeight : 0;
  }

  /**
   * Generate alerts based on resource state
   */
  private generateAlerts(
    resources: StewardedResource[],
    _categories: CategorySummary[]
  ): ResourceAlert[] {
    const alerts: ResourceAlert[] = [];

    // Check for over-utilized resources
    for (const resource of resources) {
      if (resource.available.value < 0) {
        alerts.push({
          id: `alert-overuse-${resource.id}`,
          severity: 'critical',
          resource: resource.id,
          title: `Over-Allocated: ${resource.name}`,
          message: `Allocated more than capacity. Over by ${Math.abs(resource.available.value)} ${resource.available.unit}`,
          recommendedAction: 'Reduce allocations or increase capacity',
          createdAt: new Date().toISOString(),
        });
      } else if (resource.totalAllocated.value / resource.totalCapacity.value > 0.9) {
        alerts.push({
          id: `alert-near-capacity-${resource.id}`,
          severity: 'warning',
          resource: resource.id,
          title: `Near Capacity: ${resource.name}`,
          message: `${Math.round((resource.totalAllocated.value / resource.totalCapacity.value) * 100)}% allocated`,
          recommendedAction: 'Review allocations or plan capacity increase',
          createdAt: new Date().toISOString(),
        });
      }
    }

    return alerts;
  }

  /**
   * Generate insights about resource patterns
   */
  private generateInsights(
    resources: StewardedResource[],
    _categories: CategorySummary[]
  ): ResourceInsight[] {
    const insights: ResourceInsight[] = [];

    // Find underutilized resources
    const underutilized = resources.filter(
      r => r.totalAllocated.value > 0 && r.totalUsed.value / r.totalAllocated.value < 0.5
    );

    if (underutilized.length > 0) {
      insights.push({
        id: 'insight-underutilized',
        type: 'opportunity',
        title: 'Underutilized Resources',
        description: `${underutilized.length} resources are allocated but underused. You might be able to reallocate this capacity.`,
        basedOnPeriod: 'last 30 days',
        confidence: 80,
      });
    }

    return insights;
  }

  /**
   * Normalize various frequencies to monthly
   */
  private normalizeToMonthly(amount: number, frequency: string): number {
    const conversions: Record<string, number> = {
      daily: 30,
      weekly: 4.3,
      biweekly: 2.17,
      monthly: 1,
      quarterly: 0.33,
      annual: 0.083,
      irregular: 0,
    };
    return amount * (conversions[frequency] ?? 0);
  }

  /**
   * Assess income stability
   */
  private assessIncomeStability(streams: IncomeStream[]): 'stable' | 'variable' | 'uncertain' {
    const guaranteedCount = streams.filter(s => s.isGuaranteed && s.status === 'active').length;
    const averageConfidence =
      streams.reduce((sum, s) => sum + s.confidence, 0) / (streams.length ?? 1);

    if (guaranteedCount > streams.length * 0.7 && averageConfidence > 80) {
      return 'stable';
    }
    if (averageConfidence > 60) {
      return 'variable';
    }
    return 'uncertain';
  }

  /**
   * Persist transition path
   */
  private async persistTransitionPath(transitionPath: {
    id: string;
    resourceId: string;
    stewardId: string;
    [key: string]: unknown;
  }): Promise<void> {
    return this.holochain
      .callZome({
        zomeName: 'content_store',
        fnName: 'create_transition_path',
        payload: {
          transition_path: transitionPath,
        },
      })
      .then(() => undefined)
      .catch(err => {
        // Re-throw persistence errors to caller
        throw err;
      });
  }

  /**
   * Persist transition phase
   */
  private async persistTransitionPhase(
    transitionPathId: string,
    phase: {
      id: string;
      sequenceNumber: number;
      [key: string]: unknown;
    }
  ): Promise<void> {
    return this.holochain
      .callZome({
        zomeName: 'content_store',
        fnName: 'create_transition_phase',
        payload: {
          transition_path_id: transitionPathId,
          phase,
        },
      })
      .then(() => undefined)
      .catch(err => {
        // Re-throw persistence errors to caller
        throw err;
      });
  }
}
