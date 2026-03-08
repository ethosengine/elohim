/**
 * IComputeDashboard -- Abstract interface for fetching the unified compute
 * dashboard state within the Shefa layer.
 *
 * The aggregation logic (combining compute metrics, allocations, protection
 * status, token economics, and constitutional limits) lives in Rust behind
 * the doorway API. Angular simply fetches the pre-assembled state.
 *
 * Consumers inject the COMPUTE_DASHBOARD token; the default factory resolves
 * to ComputeDashboardApiService.
 *
 * Tests provide a mock IComputeDashboard via the same token -- no concrete
 * service import needed.
 *
 * @example
 * ```typescript
 * @Injectable({ providedIn: 'root' })
 * export class SomeComponent {
 *   private readonly dashboard = inject(COMPUTE_DASHBOARD);
 *
 *   async load() {
 *     return this.dashboard.getDashboard();
 *   }
 * }
 * ```
 */

import { InjectionToken, inject } from '@angular/core';

import { ComputeDashboardApiService } from '../services/compute-dashboard-api.service';

// Re-export dashboard types from the model for consumer convenience.
// The canonical definitions live in shefa-dashboard.model.ts.
export type {
  SheafaDashboardState,
  ComputeMetrics,
  AllocationSnapshot,
  AllocationBlock,
  InfrastructureTokenBalance,
  TokenTransaction,
  ExchangeRate,
  RecentEconomicEvent,
  ConstitutionalLimitsStatus,
  ConstitutionalAlert,
  UpTimeMetrics,
  MetricHistory,
  NodeTopologyState,
  OwnedNode,
  NodeClusterStatus,
  NodeRole,
  OfflineNodeAlert,
  BidirectionalCustodianView,
  CustodianRelationship,
  StorageContentDistribution,
  ContentTypeStorage,
  ReachLevelStorage,
  NodeStorageBreakdown,
  ComputeNeedsAssessment,
  ComputeGap,
  NodeRecommendation,
  SheafaDashboardConfig,
} from '../models/shefa-dashboard.model';

import type { SheafaDashboardState } from '../models/shefa-dashboard.model';

// =============================================================================
// INTERFACE
// =============================================================================

/**
 * Abstract compute dashboard provider -- fetches the pre-assembled dashboard
 * state from the Rust API boundary.
 *
 * All aggregation (metrics + allocations + protection + tokens + limits) is
 * performed server-side. The Angular layer is a thin display client.
 */
export interface IComputeDashboard {
  /**
   * Get the current dashboard state for the authenticated operator.
   *
   * @returns Pre-assembled dashboard state
   */
  getDashboard(): Promise<SheafaDashboardState>;

  /**
   * Get dashboard state filtered to a specific governance level.
   *
   * @param level - Governance level: 'individual' | 'household' | 'community' | 'network'
   * @returns Dashboard state scoped to the requested level
   */
  getDashboardForGovernanceLevel(level: string): Promise<SheafaDashboardState>;

  /**
   * Force a refresh of the dashboard state (bypasses server-side cache).
   *
   * @returns Freshly computed dashboard state
   */
  refreshDashboard(): Promise<SheafaDashboardState>;
}

// =============================================================================
// INJECTION TOKEN
// =============================================================================

/**
 * Injection token for compute dashboard access.
 *
 * Default factory resolves to ComputeDashboardApiService which provides:
 * - GET /api/v1/compute/dashboard
 * - GET /api/v1/compute/dashboard?level={level}
 * - POST /api/v1/compute/dashboard/refresh
 *
 * Override in tests:
 * ```typescript
 * { provide: COMPUTE_DASHBOARD, useValue: mockComputeDashboard }
 * ```
 */
export const COMPUTE_DASHBOARD = new InjectionToken<IComputeDashboard>('ComputeDashboard', {
  providedIn: 'root',
  factory: () => inject(ComputeDashboardApiService),
});
