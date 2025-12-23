/**
 * Compute Event Service
 *
 * Generates immutable EconomicEvent entries for compute-related activities:
 * - CPU hours provided to family-community
 * - Storage provided (GB-hours)
 * - Bandwidth provided (Mbps-hours)
 * - Infrastructure-token issuance based on compute contribution
 *
 * Works in conjunction with:
 * - ShefaComputeService: Provides real-time metrics
 * - EconomicService: Records immutable events to Holochain DHT
 * - AllocationSnapshot: Tracks what compute is allocated to each governance level
 *
 * Token Issuance Formula:
 * tokens = (cpu_hours * cpu_rate + storage_gb_hours * storage_rate + bandwidth_mbps_hours * bandwidth_rate) / 3600
 *
 * Where rates are from infrastructure-token pricing model (TBD in Unyt swimlane spec).
 */

import { Injectable } from '@angular/core';
import { BehaviorSubject, Observable, Subject, interval, from, of } from 'rxjs';
import { map, switchMap, tap, catchError, debounceTime, startWith } from 'rxjs/operators';

import {
  AllocationSnapshot,
  ComputeMetrics,
} from '../models/shefa-dashboard.model';

import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { EconomicService, CreateEconomicEventInput } from './economic.service';
import { EconomicEvent } from '@app/elohim/models/economic-event.model';
import { ResourceMeasure } from '../models/stewarded-resources.model';

// CreateEventRequest is an alias for the EconomicEvent creation input
type CreateEventRequest = CreateEconomicEventInput;
import { ShefaComputeService } from './shefa-compute.service';

/**
 * Configuration for compute event generation
 */
interface ComputeEventConfig {
  cpuHourRate: number; // Tokens per CPU-hour
  storageGBHourRate: number; // Tokens per GB-hour (monthly rate normalized to hourly)
  bandwidthMbpsHourRate: number; // Tokens per Mbps-hour
  eventEmissionInterval: number; // Milliseconds between event batches (default: 3600000 = 1 hour)
  aggregationStrategy: 'per-governance-level' | 'per-custodian' | 'aggregate'; // How to group events
}

const DEFAULT_CONFIG: ComputeEventConfig = {
  cpuHourRate: 0.1, // 0.1 tokens per CPU-hour (TBD with Unyt)
  storageGBHourRate: 0.001, // 0.001 tokens per GB-hour
  bandwidthMbpsHourRate: 0.01, // 0.01 tokens per Mbps-hour
  eventEmissionInterval: 3600000, // 1 hour
  aggregationStrategy: 'per-governance-level',
};

/**
 * Compute usage snapshot (what we consumed in last period)
 */
interface ComputeUsageSnapshot {
  timestamp: string;
  cpuCoreHours: number; // Core-hours used
  storageGBHours: number; // GB-hours used
  bandwidthMbpsHours: number; // Mbps-hours used
  governanceLevel?: 'individual' | 'household' | 'community' | 'network';
  custodianId?: string;
}

/**
 * Computed event with all details
 */
interface ComputeEventPayload {
  eventId: string;
  timestamp: string;
  operatorId: string;
  usage: ComputeUsageSnapshot;
  tokensEarned: number;
  economicEventId?: string;
}

@Injectable({
  providedIn: 'root',
})
export class ComputeEventService {
  private config: ComputeEventConfig = DEFAULT_CONFIG;

  // Track last usage metrics to calculate delta
  private lastMetrics$ = new BehaviorSubject<ComputeMetrics | null>(null);
  private lastAllocations$ = new BehaviorSubject<AllocationSnapshot | null>(null);

  // Emit computed events
  private computeEvents$ = new Subject<ComputeEventPayload>();

  // Track emission to avoid duplicates
  private lastEmissionTime = Date.now();

  constructor(
    private holochain: HolochainClientService,
    private economicService: EconomicService,
    private shefaCompute: ShefaComputeService
  ) {}

  /**
   * Initialize event emission for an operator
   * Starts tracking compute usage and emitting events on interval
   */
  initializeEventEmission(operatorId: string, stewardedResourceId: string): Observable<ComputeEventPayload> {
    // Emit events on configured interval
    return interval(this.config.eventEmissionInterval).pipe(
      startWith(0), // Emit immediately
      switchMap(() => {
        const state = this.shefaCompute.getDashboardState();
        if (!state) return [];

        return this.generateComputeEvents(operatorId, state.computeMetrics, state.allocations);
      }),
      tap(event => this.computeEvents$.next(event)),
      catchError(error => {
        console.error('[ComputeEventService] Event emission failed:', error);
        return [];
      })
    );
  }

  /**
   * Get stream of compute events
   */
  getComputeEvents$(): Observable<ComputeEventPayload> {
    return this.computeEvents$.asObservable();
  }

  /**
   * Generate compute events from current metrics and allocations
   * Returns Observable array of generated events
   */
  private generateComputeEvents(
    operatorId: string,
    metrics: ComputeMetrics,
    allocations: AllocationSnapshot
  ): Observable<ComputeEventPayload> {
    const lastMetrics = this.lastMetrics$.value;
    const lastAllocations = this.lastAllocations$.value;

    // Calculate usage delta since last measurement
    const cpuCoreHours = this.calculateCpuCoreHours(lastMetrics, metrics);
    const storageGBHours = this.calculateStorageGBHours(lastMetrics, metrics);
    const bandwidthMbpsHours = this.calculateBandwidthMbpsHours(lastMetrics, metrics);

    // Update baselines
    this.lastMetrics$.next(metrics);
    this.lastAllocations$.next(allocations);

    // Generate events per governance level
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

    // Persist all events to Holochain
    return this.persistComputeEvents(operatorId, events).pipe(
      map(persistedEvents => {
        // Emit each event
        persistedEvents.forEach(e => this.computeEvents$.next(e));
        return persistedEvents[0] || ({} as ComputeEventPayload);
      })
    );
  }

  /**
   * Generate separate events for each governance level
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
   * Generate separate events per custodian protecting data
   */
  private generatePerCustodianEvents(
    operatorId: string,
    totalCpuHours: number,
    totalStorageGBHours: number,
    totalBandwidthHours: number,
    allocations: AllocationSnapshot
  ): ComputeEventPayload[] {
    const events: ComputeEventPayload[] = [];

    // For each custodian in allocations
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
   * Generate single aggregate event for all compute
   */
  private generateAggregateEvent(
    operatorId: string,
    totalCpuHours: number,
    totalStorageGBHours: number,
    totalBandwidthHours: number,
    allocations: AllocationSnapshot
  ): ComputeEventPayload {
    const tokensEarned = this.calculateTokensEarned(totalCpuHours, totalStorageGBHours, totalBandwidthHours);

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

  /**
   * Persist compute events to Holochain as EconomicEvent entries
   */
  private persistComputeEvents(
    operatorId: string,
    events: ComputeEventPayload[]
  ): Observable<ComputeEventPayload[]> {
    // Skip if no events or if emission is too frequent
    if (events.length === 0 || Date.now() - this.lastEmissionTime < this.config.eventEmissionInterval * 0.8) {
      return of([]);
    }

    this.lastEmissionTime = Date.now();

    // Convert to EconomicEvent requests and create via EconomicService
    const eventRequests = events.map(e =>
      this.convertToEconomicEvent(operatorId, e)
    );

    // Batch create events
    return from(
      this.holochain.callZome<any[]>({
        zomeName: 'content_store',
        fnName: 'create_economic_events_batch',
        payload: { events: eventRequests },
      })
    ).pipe(
      map((result) => {
        const results = result.success ? result.data || [] : [];
        // Link persisted event IDs back to payloads
        return events.map((e, i) => ({
          ...e,
          economicEventId: results[i]?.id,
        }));
      }),
      catchError(error => {
        console.error('[ComputeEventService] Failed to persist compute events:', error);
        return of([]);
      })
    );
  }

  /**
   * Convert ComputeEventPayload to EconomicEvent request
   */
  private convertToEconomicEvent(operatorId: string, payload: ComputeEventPayload): CreateEventRequest {
    const { cpuHours, storageHours, bandwidthHours } = this.getUsageSummary(payload);

    // Determine action and quantity based on primary resource type
    let action: 'produce' | 'use' | 'transfer' = 'produce';
    let quantity: number = cpuHours;
    let unit: string = 'cpu-hour';
    let lamadEventType: string = 'compute-provided';

    if (storageHours > cpuHours && storageHours > bandwidthHours) {
      quantity = storageHours;
      unit = 'gb-hour';
      lamadEventType = 'storage-provided';
    } else if (bandwidthHours > cpuHours) {
      quantity = bandwidthHours;
      unit = 'mbps-hour';
      lamadEventType = 'bandwidth-provided';
    }

    return {
      action,
      providerId: operatorId, // My node is the provider
      receiverId: payload.usage.governanceLevel || payload.usage.custodianId || 'family-community',
      resourceQuantityValue: quantity,
      resourceQuantityUnit: unit,
      resourceClassifiedAs: ['compute', 'infrastructure'],
      note: `Compute provided to ${payload.usage.governanceLevel || 'family-community'}: ${cpuHours.toFixed(2)} CPU-hours, ${storageHours.toFixed(2)} GB-hours, ${bandwidthHours.toFixed(2)} Mbps-hours`,
      lamadEventType: lamadEventType as any,
    };
  }

  /**
   * Helper: Get usage summary from payload
   */
  private getUsageSummary(payload: ComputeEventPayload) {
    return {
      cpuHours: payload.usage.cpuCoreHours,
      storageHours: payload.usage.storageGBHours,
      bandwidthHours: payload.usage.bandwidthMbpsHours,
    };
  }

  /**
   * Helper: Calculate CPU core-hours from metrics
   */
  private calculateCpuCoreHours(lastMetrics: ComputeMetrics | null, currentMetrics: ComputeMetrics): number {
    if (!lastMetrics) {
      // First measurement - estimate average usage over last interval
      return (currentMetrics.cpu.usagePercent / 100) * currentMetrics.cpu.totalCores * (this.config.eventEmissionInterval / 3600000);
    }

    // Use average usage over period
    const avgUsagePercent = (lastMetrics.cpu.usagePercent + currentMetrics.cpu.usagePercent) / 2 / 100;
    const hoursElapsed = this.config.eventEmissionInterval / 3600000;

    return avgUsagePercent * currentMetrics.cpu.totalCores * hoursElapsed;
  }

  /**
   * Helper: Calculate storage GB-hours
   */
  private calculateStorageGBHours(lastMetrics: ComputeMetrics | null, currentMetrics: ComputeMetrics): number {
    if (!lastMetrics) {
      // First measurement
      return currentMetrics.storage.usedGB * (this.config.eventEmissionInterval / 3600000);
    }

    // Average storage over period
    const avgUsedGB = (lastMetrics.storage.usedGB + currentMetrics.storage.usedGB) / 2;
    const hoursElapsed = this.config.eventEmissionInterval / 3600000;

    return avgUsedGB * hoursElapsed;
  }

  /**
   * Helper: Calculate bandwidth Mbps-hours
   */
  private calculateBandwidthMbpsHours(lastMetrics: ComputeMetrics | null, currentMetrics: ComputeMetrics): number {
    if (!lastMetrics) {
      return currentMetrics.network.bandwidth.usedUpstreamMbps * (this.config.eventEmissionInterval / 3600000);
    }

    const avgMbps = (
      (lastMetrics.network.bandwidth.usedUpstreamMbps + currentMetrics.network.bandwidth.usedUpstreamMbps) / 2 +
      (lastMetrics.network.bandwidth.usedDownstreamMbps + currentMetrics.network.bandwidth.usedDownstreamMbps) / 2
    ) / 2;

    const hoursElapsed = this.config.eventEmissionInterval / 3600000;
    return avgMbps * hoursElapsed;
  }

  /**
   * Helper: Calculate tokens earned from compute
   * Formula: (cpu_hours * cpu_rate + storage_gb_hours * storage_rate + bandwidth_mbps_hours * bandwidth_rate)
   */
  private calculateTokensEarned(cpuHours: number, storageGBHours: number, bandwidthMbpsHours: number): number {
    return (
      cpuHours * this.config.cpuHourRate +
      storageGBHours * this.config.storageGBHourRate +
      bandwidthMbpsHours * this.config.bandwidthMbpsHourRate
    );
  }

  /**
   * Helper: Generate unique event ID
   */
  private generateEventId(): string {
    return `ce-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
  }

  /**
   * Update compute event configuration
   */
  setConfig(config: Partial<ComputeEventConfig>): void {
    this.config = { ...this.config, ...config };
  }

  /**
   * Get current configuration
   */
  getConfig(): ComputeEventConfig {
    return this.config;
  }
}
