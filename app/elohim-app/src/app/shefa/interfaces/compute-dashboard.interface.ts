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
  StewardedNode,
  NodeStewardshipSummary,
  NodeAvailability,
  ResourceAvailability,
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

import type {
  SheafaDashboardState,
  NodeTopologyState,
  ComputeNeedsAssessment,
  StorageContentDistribution,
  BidirectionalCustodianView,
} from '../models/shefa-dashboard.model';

import { Observable } from 'rxjs';

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

  /**
   * Initialize a real-time dashboard stream for an operator + resource pair.
   * Returns an Observable that emits updated state on each refresh interval.
   */
  initializeDashboard(
    operatorId: string,
    stewardedResourceId: string
  ): Observable<SheafaDashboardState>;

  /**
   * Get the last-known dashboard state synchronously (cached from most recent fetch).
   * Returns null if no dashboard has been fetched yet.
   */
  getDashboardState(): SheafaDashboardState | null;

  /**
   * Get node topology for an operator (owned nodes, cluster health, alerts).
   */
  getNodeTopology(operatorId: string): Observable<NodeTopologyState>;

  /**
   * Get compute needs assessment (gaps, recommendations, help-flow CTA).
   */
  getComputeNeedsAssessment(operatorId: string): Observable<ComputeNeedsAssessment>;

  /**
   * Get storage content distribution breakdown (by type, reach, node).
   */
  getStorageContentDistribution(operatorId: string): Observable<StorageContentDistribution>;

  /**
   * Get bidirectional custodian view (who I help, who helps me).
   */
  getBidirectionalCustodianView(operatorId: string): Observable<BidirectionalCustodianView>;
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
