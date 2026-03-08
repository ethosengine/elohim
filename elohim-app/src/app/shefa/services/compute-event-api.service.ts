/**
 * Compute Event API Service (thin)
 *
 * Generates immutable EconomicEvent entries for compute-related activities:
 * - CPU hours provided to family-community
 * - Storage provided (GB-hours)
 * - Bandwidth provided (Mbps-hours)
 * - Infrastructure-token issuance based on compute contribution
 *
 * Keeps ALL the UX logic from the original fat ComputeEventService:
 * - Interval-based sampling from ShefaComputeService
 * - Metric-to-usage conversion (CPU core-hours, storage GB-hours, bandwidth Mbps-hours)
 * - Token calculation formula
 * - Aggregation strategies (per-governance-level, per-custodian, aggregate)
 * - Configuration management
 *
 * Replaces the direct zome call (`callZome('content_store', 'create_economic_events_batch')`)
 * with `ECONOMIC_EVENT_FACTORY.createMultipleFromStaged()` which hits the HTTP bulk endpoint.
 *
 * Token Issuance Formula:
 * tokens = (cpu_hours * cpu_rate + storage_gb_hours * storage_rate + bandwidth_mbps_hours * bandwidth_rate)
 */

import { Injectable, inject } from '@angular/core';

import { switchMap, tap, catchError, startWith, map } from 'rxjs/operators';

import { BehaviorSubject, Observable, Subject, interval, of, from } from 'rxjs';

import { LamadEventType } from '@app/elohim/models/economic-event.model';

import { ECONOMIC_EVENT_FACTORY } from '../interfaces';
import { AllocationSnapshot, ComputeMetrics } from '../models/shefa-dashboard.model';

import { ShefaComputeService } from './shefa-compute.service';

import type {
  IComputeEvent,
  ComputeEventConfig,
  ComputeEventPayload,
} from '../interfaces/compute-event.interface';

const DEFAULT_CONFIG: ComputeEventConfig = {
  cpuHourRate: 0.1, // 0.1 tokens per CPU-hour (TBD with Unyt)
  storageGBHourRate: 0.001, // 0.001 tokens per GB-hour
  bandwidthMbpsHourRate: 0.01, // 0.01 tokens per Mbps-hour
  eventEmissionInterval: 3600000, // 1 hour
  aggregationStrategy: 'per-governance-level',
};

@Injectable({
  providedIn: 'root',
})
export class ComputeEventApiService implements IComputeEvent {
  private config: ComputeEventConfig = { ...DEFAULT_CONFIG };

  // Track last usage metrics to calculate delta
  private readonly lastMetrics$ = new BehaviorSubject<ComputeMetrics | null>(null);
  private readonly lastAllocations$ = new BehaviorSubject<AllocationSnapshot | null>(null);

  // Emit computed events
  private readonly computeEvents$ = new Subject<ComputeEventPayload>();

  // Track emission to avoid duplicates
  private lastEmissionTime = Date.now();

  private readonly eventFactory = inject(ECONOMIC_EVENT_FACTORY);
  private readonly shefaCompute = inject(ShefaComputeService);

  /**
   * Initialize event emission for an operator.
   * Starts tracking compute usage and emitting events on interval.
   */
  initializeEventEmission(
    operatorId: string,
    _stewardedResourceId: string
  ): Observable<ComputeEventPayload> {
    return interval(this.config.eventEmissionInterval).pipe(
      startWith(0), // Emit immediately
      switchMap(() => {
        const state = this.shefaCompute.getDashboardState();
        if (!state) return of<ComputeEventPayload>(null as unknown as ComputeEventPayload);

        return this.generateComputeEvents(operatorId, state.computeMetrics, state.allocations);
      }),
      tap(event => {
        if (event) this.computeEvents$.next(event);
      }),
      catchError(() => {
        return of<ComputeEventPayload>(null as unknown as ComputeEventPayload);
      })
    );
  }

  /**
   * Get stream of compute events.
   */
  getComputeEvents$(): Observable<ComputeEventPayload> {
    return this.computeEvents$.asObservable();
  }

  /**
   * Update compute event configuration.
   */
  setConfig(config: Partial<ComputeEventConfig>): void {
    this.config = { ...this.config, ...config };
  }

  /**
   * Get current configuration.
   */
  getConfig(): ComputeEventConfig {
    return this.config;
  }

  // ===========================================================================
  // Event Generation
  // ===========================================================================

  /**
   * Generate compute events from current metrics and allocations.
   * Returns Observable of the first generated event (or empty payload).
   */
  private generateComputeEvents(
    operatorId: string,
    metrics: ComputeMetrics,
    allocations: AllocationSnapshot
  ): Observable<ComputeEventPayload> {
    const lastMetrics = this.lastMetrics$.value;

    // Calculate usage delta since last measurement
    const cpuCoreHours = this.calculateCpuCoreHours(lastMetrics, metrics);
    const storageGBHours = this.calculateStorageGBHours(lastMetrics, metrics);
    const bandwidthMbpsHours = this.calculateBandwidthMbpsHours(lastMetrics, metrics);

    // Update baselines
    this.lastMetrics$.next(metrics);
    this.lastAllocations$.next(allocations);

    // Generate events per aggregation strategy
    const events: ComputeEventPayload[] = [];

    switch (this.config.aggregationStrategy) {
      case 'per-governance-level':
        events.push(
          ...this.generatePerGovernanceLevelEvents(
            operatorId,
            cpuCoreHours,
            storageGBHours,
            bandwidthMbpsHours,
            allocations
          )
        );
        break;

      case 'per-custodian':
        events.push(
          ...this.generatePerCustodianEvents(
            operatorId,
            cpuCoreHours,
            storageGBHours,
            bandwidthMbpsHours,
            allocations
          )
        );
        break;

      case 'aggregate':
      default:
        events.push(
          this.generateAggregateEvent(
            operatorId,
            cpuCoreHours,
            storageGBHours,
            bandwidthMbpsHours,
            allocations
          )
        );
        break;
    }

    // Persist all events via ECONOMIC_EVENT_FACTORY
    return this.persistComputeEvents(operatorId, events).pipe(
      map(persistedEvents => {
        // Emit each event
        persistedEvents.forEach(e => this.computeEvents$.next(e));
        return persistedEvents[0] ?? ({} as ComputeEventPayload);
      })
    );
  }

  // ===========================================================================
  // Aggregation Strategies
  // ===========================================================================

  /**
   * Generate separate events for each governance level.
   */
  private generatePerGovernanceLevelEvents(
    operatorId: string,
    totalCpuHours: number,
    totalStorageGBHours: number,
    totalBandwidthHours: number,
    allocations: AllocationSnapshot
  ): ComputeEventPayload[] {
    const events: ComputeEventPayload[] = [];
    const levels = ['individual', 'household', 'community', 'network'] as const;

    levels.forEach(level => {
      const levelAlloc = allocations.byGovernanceLevel[level];

      // Proportional distribution by allocation percentages
      const cpuHours = (totalCpuHours * levelAlloc.cpuPercent) / 100;
      const storageHours = (totalStorageGBHours * levelAlloc.storagePercent) / 100;
      const bandwidthHours = (totalBandwidthHours * levelAlloc.bandwidthPercent) / 100;

      const tokensEarned = this.calculateTokensEarned(cpuHours, storageHours, bandwidthHours);

      events.push({
        eventId: this.generateEventId(),
        timestamp: new Date().toISOString(),
        operatorId,
        usage: {
          timestamp: new Date().toISOString(),
          cpuCoreHours: cpuHours,
          storageGBHours: storageHours,
          bandwidthMbpsHours: bandwidthHours,
          governanceLevel: level,
        },
        tokensEarned,
      });
    });

    return events;
  }

  /**
   * Generate separate events per custodian protecting data.
   */
  private generatePerCustodianEvents(
    operatorId: string,
    totalCpuHours: number,
    totalStorageGBHours: number,
    totalBandwidthHours: number,
    allocations: AllocationSnapshot
  ): ComputeEventPayload[] {
    const events: ComputeEventPayload[] = [];

    allocations.allocationBlocks.forEach(block => {
      if (block.relatedAgents && block.relatedAgents.length > 0) {
        const perCustodian = {
          cpuHours: (totalCpuHours * block.cpu.percent) / 100,
          storageHours: (totalStorageGBHours * block.storage.percent) / 100,
          bandwidthHours: (totalBandwidthHours * block.bandwidth.percent) / 100,
        };

        block.relatedAgents.forEach(custodianId => {
          const tokensEarned = this.calculateTokensEarned(
            perCustodian.cpuHours / block.relatedAgents!.length,
            perCustodian.storageHours / block.relatedAgents!.length,
            perCustodian.bandwidthHours / block.relatedAgents!.length
          );

          events.push({
            eventId: this.generateEventId(),
            timestamp: new Date().toISOString(),
            operatorId,
            usage: {
              timestamp: new Date().toISOString(),
              cpuCoreHours: perCustodian.cpuHours / block.relatedAgents!.length,
              storageGBHours: perCustodian.storageHours / block.relatedAgents!.length,
              bandwidthMbpsHours: perCustodian.bandwidthHours / block.relatedAgents!.length,
              custodianId,
            },
            tokensEarned,
          });
        });
      }
    });

    return events;
  }

  /**
   * Generate single aggregate event for all compute.
   */
  private generateAggregateEvent(
    operatorId: string,
    totalCpuHours: number,
    totalStorageGBHours: number,
    totalBandwidthHours: number,
    _allocations: AllocationSnapshot
  ): ComputeEventPayload {
    const tokensEarned = this.calculateTokensEarned(
      totalCpuHours,
      totalStorageGBHours,
      totalBandwidthHours
    );

    return {
      eventId: this.generateEventId(),
      timestamp: new Date().toISOString(),
      operatorId,
      usage: {
        timestamp: new Date().toISOString(),
        cpuCoreHours: totalCpuHours,
        storageGBHours: totalStorageGBHours,
        bandwidthMbpsHours: totalBandwidthHours,
      },
      tokensEarned,
    };
  }

  // ===========================================================================
  // Persistence (via ECONOMIC_EVENT_FACTORY)
  // ===========================================================================

  /**
   * Persist compute events via ECONOMIC_EVENT_FACTORY.createMultipleFromStaged().
   *
   * Converts ComputeEventPayloads to a staged-transaction-compatible format
   * and delegates to the HTTP bulk endpoint.
   */
  private persistComputeEvents(
    operatorId: string,
    events: ComputeEventPayload[]
  ): Observable<ComputeEventPayload[]> {
    // Skip if no events or if emission is too frequent
    if (
      events.length === 0 ||
      Date.now() - this.lastEmissionTime < this.config.eventEmissionInterval * 0.8
    ) {
      return of([]);
    }

    this.lastEmissionTime = Date.now();

    // Convert to staged-transaction-compatible format for the factory
    const stagedEvents = events.map(e => this.convertToStagedFormat(operatorId, e));

    return from(this.eventFactory.createMultipleFromStaged(stagedEvents)).pipe(
      map(results => {
        // Link persisted event IDs back to payloads
        return events.map((e, i) => ({
          ...e,
          economicEventId: results[i]?.id ?? undefined,
        }));
      }),
      catchError(() => {
        return of([]);
      })
    );
  }

  /**
   * Convert a ComputeEventPayload to staged-transaction-compatible format.
   *
   * The ECONOMIC_EVENT_FACTORY expects StagedTransaction-shaped objects.
   * Compute events aren't literally staged bank transactions -- the API
   * handles the transformation server-side. We use `as any` for the type
   * cast since the shapes differ.
   */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  private convertToStagedFormat(operatorId: string, payload: ComputeEventPayload): any {
    const { cpuHours, storageHours, bandwidthHours } = this.getUsageSummary(payload);

    // Determine primary resource type for the event
    let quantity: number = cpuHours;
    let unit = 'cpu-hour';
    let lamadEventType: LamadEventType = 'compute-provided' as LamadEventType;

    if (storageHours > cpuHours && storageHours > bandwidthHours) {
      quantity = storageHours;
      unit = 'gb-hour';
      lamadEventType = 'storage-provided' as LamadEventType;
    } else if (bandwidthHours > cpuHours) {
      quantity = bandwidthHours;
      unit = 'mbps-hour';
      lamadEventType = 'bandwidth-provided' as LamadEventType;
    }

    const receiver =
      payload.usage.governanceLevel ?? payload.usage.custodianId ?? 'family-community';
    const note = `Compute provided to ${receiver}: ${cpuHours.toFixed(2)} CPU-hours, ${storageHours.toFixed(2)} GB-hours, ${bandwidthHours.toFixed(2)} Mbps-hours`;

    // Shape as a staged transaction for the factory
    return {
      id: payload.eventId,
      batchId: `compute-${Date.now()}`,
      stewardId: operatorId,
      plaidTransactionId: payload.eventId, // Reuse event ID (no Plaid source)
      plaidAccountId: 'compute-node',
      financialAssetId: 'infrastructure-compute',
      timestamp: payload.timestamp,
      type: 'credit' as const,
      amount: { value: quantity, unit },
      description: note,
      category: lamadEventType,
      categoryConfidence: 100,
      categorySource: 'rule' as const,
      isDuplicate: false,
      reviewStatus: 'approved' as const,
      economicEventId: undefined,
      metadata: {
        action: 'produce',
        providerId: operatorId,
        receiverId: receiver,
        resourceClassifiedAs: ['compute', 'infrastructure'],
        lamadEventType,
        computeUsage: {
          cpuCoreHours: cpuHours,
          storageGBHours: storageHours,
          bandwidthMbpsHours: bandwidthHours,
          governanceLevel: payload.usage.governanceLevel,
          custodianId: payload.usage.custodianId,
        },
        tokensEarned: payload.tokensEarned,
      },
      createdAt: payload.timestamp,
    };
  }

  // ===========================================================================
  // Metric Calculations
  // ===========================================================================

  /**
   * Get usage summary from payload.
   */
  private getUsageSummary(payload: ComputeEventPayload) {
    return {
      cpuHours: payload.usage.cpuCoreHours,
      storageHours: payload.usage.storageGBHours,
      bandwidthHours: payload.usage.bandwidthMbpsHours,
    };
  }

  /**
   * Calculate CPU core-hours from metrics delta.
   */
  private calculateCpuCoreHours(
    lastMetrics: ComputeMetrics | null,
    currentMetrics: ComputeMetrics
  ): number {
    if (!lastMetrics) {
      // First measurement - estimate average usage over last interval
      return (
        (currentMetrics.cpu.usagePercent / 100) *
        currentMetrics.cpu.totalCores *
        (this.config.eventEmissionInterval / 3600000)
      );
    }

    // Use average usage over period
    const avgUsagePercent =
      (lastMetrics.cpu.usagePercent + currentMetrics.cpu.usagePercent) / 2 / 100;
    const hoursElapsed = this.config.eventEmissionInterval / 3600000;

    return avgUsagePercent * currentMetrics.cpu.totalCores * hoursElapsed;
  }

  /**
   * Calculate storage GB-hours.
   */
  private calculateStorageGBHours(
    lastMetrics: ComputeMetrics | null,
    currentMetrics: ComputeMetrics
  ): number {
    if (!lastMetrics) {
      return currentMetrics.storage.usedGB * (this.config.eventEmissionInterval / 3600000);
    }

    const avgUsedGB = (lastMetrics.storage.usedGB + currentMetrics.storage.usedGB) / 2;
    const hoursElapsed = this.config.eventEmissionInterval / 3600000;

    return avgUsedGB * hoursElapsed;
  }

  /**
   * Calculate bandwidth Mbps-hours.
   */
  private calculateBandwidthMbpsHours(
    lastMetrics: ComputeMetrics | null,
    currentMetrics: ComputeMetrics
  ): number {
    if (!lastMetrics) {
      return (
        currentMetrics.network.bandwidth.usedUpstreamMbps *
        (this.config.eventEmissionInterval / 3600000)
      );
    }

    const avgMbps =
      ((lastMetrics.network.bandwidth.usedUpstreamMbps +
        currentMetrics.network.bandwidth.usedUpstreamMbps) /
        2 +
        (lastMetrics.network.bandwidth.usedDownstreamMbps +
          currentMetrics.network.bandwidth.usedDownstreamMbps) /
          2) /
      2;

    const hoursElapsed = this.config.eventEmissionInterval / 3600000;
    return avgMbps * hoursElapsed;
  }

  /**
   * Calculate tokens earned from compute.
   * Formula: cpu_hours * cpu_rate + storage_gb_hours * storage_rate + bandwidth_mbps_hours * bandwidth_rate
   */
  private calculateTokensEarned(
    cpuHours: number,
    storageGBHours: number,
    bandwidthMbpsHours: number
  ): number {
    return (
      cpuHours * this.config.cpuHourRate +
      storageGBHours * this.config.storageGBHourRate +
      bandwidthMbpsHours * this.config.bandwidthMbpsHourRate
    );
  }

  /**
   * Generate unique event ID.
   */
  private generateEventId(): string {
    return `ce-${Date.now()}-${(crypto.getRandomValues(new Uint32Array(1))[0] / 2 ** 32).toString(36).substring(2, 11)}`;
  }
}
