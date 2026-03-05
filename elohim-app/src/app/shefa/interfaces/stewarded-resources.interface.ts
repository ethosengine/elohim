/**
 * IStewardedResources -- Abstract interface for resource stewardship management.
 *
 * Decouples resource tracking, allocation, and dashboard logic from the
 * concrete Holochain-backed implementation. Consumers inject the
 * STEWARDED_RESOURCES token; the default factory resolves to
 * StewardedResourceService.
 *
 * Tests provide a mock IStewardedResources via the same token -- no
 * concrete service import needed.
 *
 * @example
 * ```typescript
 * @Injectable({ providedIn: 'root' })
 * export class DashboardService {
 *   private readonly resources = inject(STEWARDED_RESOURCES);
 *
 *   async loadDashboard(stewardId: string): Promise<StewardedResourceDashboard> {
 *     return this.resources.buildDashboard(stewardId);
 *   }
 * }
 * ```
 */

import { InjectionToken, inject } from '@angular/core';

import { StewardedResourcesApiService } from '../services/stewarded-resources-api.service';

import type {
  StewardedResource,
  ResourceCategory,
  ResourceMeasure,
  AllocationBlock,
  UsageRecord,
  CategorySummary,
  StewardedResourceDashboard,
  ConstitutionalLimit,
} from '@app/shefa/models/stewarded-resources.model';

/**
 * Abstract resource stewardship manager -- CRUD for resources,
 * allocations, usage, dashboards, and constitutional limits.
 *
 * Implementations handle Holochain persistence, economic event
 * creation, and governance integration.
 */
export interface IStewardedResources {
  /**
   * Create a new stewarded resource.
   *
   * @param stewardId - Human ID who stewards this resource
   * @param category - Resource category (energy, compute, financial-asset, etc.)
   * @param subcategory - More specific resource type
   * @param name - User-friendly name
   * @param totalCapacity - Maximum available capacity
   * @param options - Optional description, governance level, observer, permanent reserve
   * @returns The created StewardedResource
   */
  createResource(
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
  ): Promise<StewardedResource>;

  /**
   * Get a stewarded resource by ID.
   *
   * @param resourceId - Resource identifier
   * @returns The resource, or null if not found
   */
  getResource(resourceId: string): Promise<StewardedResource | null>;

  /**
   * Get all resources for a steward.
   *
   * @param stewardId - Human ID of the steward
   * @returns Array of stewarded resources (empty if none or error)
   */
  getStewardResources(stewardId: string): Promise<StewardedResource[]>;

  /**
   * Get resources in a specific category for a steward.
   *
   * @param stewardId - Human ID of the steward
   * @param category - Resource category to filter by
   * @returns Filtered array of resources
   */
  getResourcesByCategory(
    stewardId: string,
    category: ResourceCategory
  ): Promise<StewardedResource[]>;

  /**
   * Create an allocation block for a resource.
   *
   * @param resourceId - Resource to allocate from
   * @param label - Purpose label (e.g., "Learning", "Work")
   * @param allocatedAmount - Amount to allocate
   * @param options - Optional description, governance level, priority
   * @returns The created AllocationBlock
   */
  createAllocation(
    resourceId: string,
    label: string,
    allocatedAmount: ResourceMeasure,
    options?: {
      description?: string;
      governanceLevel?: string;
      priority?: number;
    }
  ): Promise<AllocationBlock>;

  /**
   * Record usage of an allocated resource.
   *
   * @param resourceId - Resource being used
   * @param allocationBlockId - Which allocation block this usage is from
   * @param amount - Amount used
   * @param options - Optional action type, observer attestation, note
   * @returns The created UsageRecord
   */
  recordUsage(
    resourceId: string,
    allocationBlockId: string,
    amount: ResourceMeasure,
    options?: {
      action?: string;
      observerAttestationId?: string;
      note?: string;
    }
  ): Promise<UsageRecord>;

  /**
   * Get category summary -- aggregates all resources in a category.
   *
   * @param stewardId - Human ID of the steward
   * @param category - Resource category to summarize
   * @returns Aggregated category summary
   */
  getCategorySummary(stewardId: string, category: ResourceCategory): Promise<CategorySummary>;

  /**
   * Build complete stewardship dashboard.
   *
   * @param stewardId - Human ID of the steward
   * @returns Full dashboard with categories, metrics, alerts, insights
   */
  buildDashboard(stewardId: string): Promise<StewardedResourceDashboard>;

  /**
   * Get constitutional limits for a resource category.
   *
   * @param category - Resource category to check limits for
   * @returns Constitutional limit or null if none defined
   */
  getConstitutionalLimits(category: ResourceCategory): Promise<ConstitutionalLimit | null>;
}

/**
 * Injection token for stewarded resources.
 *
 * Default factory resolves to StewardedResourceService which provides:
 * - Holochain-backed resource CRUD
 * - Allocation and usage tracking via EconomicEvents
 * - Dashboard aggregation with health scoring
 * - Constitutional limit enforcement (dignity floors, wealth ceilings)
 *
 * Override in tests:
 * ```typescript
 * { provide: STEWARDED_RESOURCES, useValue: mockStewardedResources }
 * ```
 */
export const STEWARDED_RESOURCES = new InjectionToken<IStewardedResources>('StewardedResources', {
  providedIn: 'root',
  factory: () => inject(StewardedResourcesApiService),
});
