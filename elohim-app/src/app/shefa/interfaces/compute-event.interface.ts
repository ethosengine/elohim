/**
 * IComputeEvent -- Abstract interface for compute-event generation and
 * emission within the Shefa economic layer.
 *
 * Decouples the compute-event lifecycle (interval sampling, metric conversion,
 * aggregation, config management) from the concrete persistence layer.
 * Consumers inject the COMPUTE_EVENT token; the default factory resolves to
 * ComputeEventApiService.
 *
 * Tests provide a mock IComputeEvent via the same token -- no concrete
 * service import needed.
 *
 * @example
 * ```typescript
 * @Injectable({ providedIn: 'root' })
 * export class DashboardService {
 *   private readonly computeEvent = inject(COMPUTE_EVENT);
 *
 *   startTracking(operatorId: string, resourceId: string) {
 *     return this.computeEvent.initializeEventEmission(operatorId, resourceId);
 *   }
 * }
 * ```
 */

import { InjectionToken, inject } from '@angular/core';

import { Observable } from 'rxjs';

import { ComputeEventApiService } from '../services/compute-event-api.service';

/**
 * Configuration for compute event generation.
 *
 * Controls token issuance rates, emission intervals, and how events are
 * grouped (per governance level, per custodian, or aggregate).
 */
export interface ComputeEventConfig {
  cpuHourRate: number; // Tokens per CPU-hour
  storageGBHourRate: number; // Tokens per GB-hour (monthly rate normalized to hourly)
  bandwidthMbpsHourRate: number; // Tokens per Mbps-hour
  eventEmissionInterval: number; // Milliseconds between event batches (default: 3600000 = 1 hour)
  aggregationStrategy: 'per-governance-level' | 'per-custodian' | 'aggregate'; // How to group events
}

/**
 * Compute usage snapshot -- what we consumed in the last period.
 *
 * Captures resource consumption metrics for a single measurement window.
 */
export interface ComputeUsageSnapshot {
  timestamp: string;
  cpuCoreHours: number; // Core-hours used
  storageGBHours: number; // GB-hours used
  bandwidthMbpsHours: number; // Mbps-hours used
  governanceLevel?: 'individual' | 'household' | 'community' | 'network';
  custodianId?: string;
}

/**
 * Computed event with all details.
 *
 * Represents a single compute contribution event with usage metrics,
 * token calculation, and optional link to the persisted EconomicEvent.
 */
export interface ComputeEventPayload {
  eventId: string;
  timestamp: string;
  operatorId: string;
  usage: ComputeUsageSnapshot;
  tokensEarned: number;
  economicEventId?: string;
}

/**
 * Abstract compute event manager -- generates, tracks, and emits
 * compute-related economic events on a configured interval.
 *
 * Implementations handle metric sampling from COMPUTE_DASHBOARD,
 * token calculation, aggregation strategy, and persistence delegation.
 */
export interface IComputeEvent {
  /**
   * Initialize event emission for an operator.
   *
   * Starts tracking compute usage and emitting events on the configured
   * interval. The returned Observable emits each generated ComputeEventPayload.
   *
   * @param operatorId - Agent public key of the compute operator
   * @param stewardedResourceId - FK to the StewardedResource entry
   * @returns Observable stream of compute event payloads
   */
  initializeEventEmission(
    operatorId: string,
    stewardedResourceId: string
  ): Observable<ComputeEventPayload>;

  /**
   * Get stream of all emitted compute events.
   *
   * @returns Observable stream of compute event payloads (hot observable)
   */
  getComputeEvents$(): Observable<ComputeEventPayload>;

  /**
   * Update compute event configuration.
   *
   * Merges partial config into current configuration.
   *
   * @param config - Partial config to merge
   */
  setConfig(config: Partial<ComputeEventConfig>): void;

  /**
   * Get current configuration.
   *
   * @returns Current ComputeEventConfig
   */
  getConfig(): ComputeEventConfig;
}

/**
 * Injection token for compute event management.
 *
 * Default factory resolves to ComputeEventApiService which provides:
 * - Interval-based compute metric sampling from COMPUTE_DASHBOARD
 * - Token calculation from CPU, storage, bandwidth metrics
 * - Aggregation strategies (per-governance-level, per-custodian, aggregate)
 * - Persistence via ECONOMIC_EVENT_FACTORY (HTTP bulk endpoint)
 *
 * Override in tests:
 * ```typescript
 * { provide: COMPUTE_EVENT, useValue: mockComputeEvent }
 * ```
 */
export const COMPUTE_EVENT = new InjectionToken<IComputeEvent>('ComputeEvent', {
  providedIn: 'root',
  factory: () => inject(ComputeEventApiService),
});
