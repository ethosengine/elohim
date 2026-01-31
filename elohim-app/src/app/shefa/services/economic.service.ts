/**
 * Economic Service - Shefa hREA Economic Event Operations
 *
 * From the Manifesto (Part IV-A: The Economy of Honor):
 * "REA doesn't ask 'how much money?' It asks 'what actually happened?'
 * Every event is recorded from what we call the 'independent view'—as
 * a transaction between parties rather than separate ledger entries."
 *
 * This service provides access to hREA EconomicEvent operations.
 * It's domain-agnostic (Shefa layer) - specific domains like Lamad
 * compose these primitives for their use cases.
 *
 * Architecture:
 *   EconomicService → HolochainClientService → Holochain Conductor → DHT
 *
 * Use cases:
 * - Query events by agent (as provider, receiver, or both)
 * - Query events by action type (use, produce, transfer, etc.)
 * - Create new economic events (low-level, prefer domain-specific methods)
 *
 * @see AppreciationService for recognition/appreciation flows
 * @see ContributorService for Lamad-specific contributor recognition
 */

import { Injectable, signal, computed } from '@angular/core';

// @coverage: 29.7% (2026-02-04)

import { catchError, shareReplay } from 'rxjs/operators';

import { Observable, of, from, defer } from 'rxjs';

import {
  EconomicEvent,
  REAAction,
  LamadEventType,
  EventState,
  ResourceClassification,
} from '@app/elohim/models';
import { HolochainClientService } from '@app/elohim/services/holochain-client.service';

// =============================================================================
// Holochain Types (match Rust DNA structures)
// =============================================================================

/**
 * Economic event as stored in Holochain
 * Matches EconomicEvent struct in integrity zome
 */
interface HolochainEconomicEvent {
  id: string;
  action: string;
  provider: string;
  receiver: string;
  resourceConformsTo: string | null;
  resourceInventoriedAs: string | null;
  toResourceInventoriedAs: string | null;
  resourceClassifiedAsJson: string;
  resourceQuantityValue: number | null;
  resourceQuantityUnit: string | null;
  effortQuantityValue: number | null;
  effortQuantityUnit: string | null;
  hasPointInTime: string;
  hasDuration: string | null;
  inputOf: string | null;
  outputOf: string | null;
  fulfillsJson: string;
  realizationOf: string | null;
  agreedIn: string | null;
  satisfiesJson: string;
  inScopeOfJson: string;
  note: string | null;
  state: string;
  triggeredBy: string | null;
  atLocation: string | null;
  image: string | null;
  lamadEventType: string | null;
  metadataJson: string;
  createdAt: string;
}

/**
 * Output from economic event zome calls
 */
interface HolochainEconomicEventOutput {
  actionHash: Uint8Array;
  event: HolochainEconomicEvent;
}

/**
 * Input for creating an economic event
 */
export interface CreateEconomicEventInput {
  action: REAAction;
  providerId: string;
  receiverId: string;
  resourceConformsTo?: string;
  resourceInventoriedAs?: string;
  toResourceInventoriedAs?: string;
  resourceClassifiedAs?: string[];
  resourceQuantityValue?: number;
  resourceQuantityUnit?: string;
  effortQuantityValue?: number;
  effortQuantityUnit?: string;
  inputOf?: string;
  outputOf?: string;
  fulfills?: string[];
  realizationOf?: string;
  agreedIn?: string;
  satisfies?: string[];
  inScopeOf?: string[];
  note?: string;
  lamadEventType?: LamadEventType;
}

// =============================================================================
// Service Implementation
// =============================================================================

@Injectable({
  providedIn: 'root',
})
export class EconomicService {
  /**
   * Whether economic service is available.
   * Mirrors HolochainClientService connection state.
   */
  private readonly availableSignal = signal(false);
  readonly available = this.availableSignal.asReadonly();

  /** Computed: true when Holochain client is connected AND service available */
  readonly ready = computed(() => this.available() && this.holochainClient.isConnected());

  /** Cache for events by agent (provider/receiver) */
  private readonly eventsByAgentCache = new Map<string, Observable<EconomicEvent[]>>();

  constructor(private readonly holochainClient: HolochainClientService) {}

  /**
   * Check if economic service is available.
   */
  isAvailable(): boolean {
    return this.availableSignal();
  }

  // ===========================================================================
  // Query Methods
  // ===========================================================================

  /**
   * Get economic events for an agent.
   *
   * @param agentId - The agent ID to query events for
   * @param direction - Query as 'provider', 'receiver', or 'both'
   * @returns Observable of economic events
   */
  getEventsForAgent(
    agentId: string,
    direction: 'provider' | 'receiver' | 'both' = 'both'
  ): Observable<EconomicEvent[]> {
    if (!this.isAvailable()) {
      return of([]);
    }

    const cacheKey = `${agentId}:${direction}`;

    if (!this.eventsByAgentCache.has(cacheKey)) {
      const request = defer(() => from(this.fetchEventsForAgent(agentId, direction))).pipe(
        shareReplay(1),
        catchError(_err => {
          return of([]);
        })
      );

      this.eventsByAgentCache.set(cacheKey, request);
    }

    return this.eventsByAgentCache.get(cacheKey)!;
  }

  /**
   * Get economic events by action type.
   *
   * @param action - The REA action to filter by (use, produce, transfer, etc.)
   * @returns Observable of economic events with that action
   */
  getEventsByAction(action: REAAction): Observable<EconomicEvent[]> {
    if (!this.isAvailable()) {
      return of([]);
    }

    return defer(() => from(this.fetchEventsByAction(action))).pipe(
      catchError(_err => {
        return of([]);
      })
    );
  }

  /**
   * Get economic events by Lamad event type.
   *
   * @param lamadType - The Lamad-specific event type
   * @returns Observable of economic events with that type
   */
  getEventsByLamadType(lamadType: LamadEventType): Observable<EconomicEvent[]> {
    if (!this.isAvailable()) {
      return of([]);
    }

    return defer(() => from(this.fetchEventsByLamadType(lamadType))).pipe(
      catchError(_err => {
        return of([]);
      })
    );
  }

  // ===========================================================================
  // Create Methods
  // ===========================================================================

  /**
   * Create a new economic event.
   *
   * This is the low-level creation method. For domain-specific events,
   * prefer using domain services (e.g., ContributorService for recognition).
   *
   * @param input - The event creation input
   * @returns Observable of the created event
   */
  createEvent(input: CreateEconomicEventInput): Observable<EconomicEvent> {
    if (!this.isAvailable()) {
      throw new Error('Economic service not available');
    }

    return defer(() => from(this.doCreateEvent(input))).pipe(
      catchError(_err => {
        throw _err;
      })
    );
  }

  // ===========================================================================
  // Cache Management
  // ===========================================================================

  /**
   * Clear all caches (useful after creating events or when data changes)
   */
  clearCache(): void {
    this.eventsByAgentCache.clear();
  }

  /**
   * Test if economic API is reachable.
   * Updates the available signal based on result.
   */
  async testAvailability(): Promise<boolean> {
    try {
      // Simple test: try to get events for a non-existent agent
      const result = await this.holochainClient.callZome<HolochainEconomicEventOutput[]>({
        zomeName: 'content_store',
        fnName: 'get_events_by_provider',
        payload: 'test-availability',
      });

      // Even an empty result means the zome is available
      this.availableSignal.set(result.success);
      return result.success;
    } catch {
      // Service availability check failed - Economic service unavailable
      this.availableSignal.set(false);
      return false;
    }
  }

  // ===========================================================================
  // Private Methods - Zome Calls
  // ===========================================================================

  private async fetchEventsForAgent(
    agentId: string,
    direction: 'provider' | 'receiver' | 'both'
  ): Promise<EconomicEvent[]> {
    const events: EconomicEvent[] = [];

    if (direction === 'provider' || direction === 'both') {
      const providerResult = await this.holochainClient.callZome<HolochainEconomicEventOutput[]>({
        zomeName: 'content_store',
        fnName: 'get_events_by_provider',
        payload: agentId,
      });

      if (providerResult.success && providerResult.data) {
        events.push(...providerResult.data.map(o => this.transformToEconomicEvent(o)));
      }
    }

    if (direction === 'receiver' || direction === 'both') {
      const receiverResult = await this.holochainClient.callZome<HolochainEconomicEventOutput[]>({
        zomeName: 'content_store',
        fnName: 'get_events_by_receiver',
        payload: agentId,
      });

      if (receiverResult.success && receiverResult.data) {
        // Avoid duplicates if querying 'both'
        const existingIds = new Set(events.map(e => e.id));
        const newEvents = receiverResult.data
          .map(o => this.transformToEconomicEvent(o))
          .filter(e => !existingIds.has(e.id));
        events.push(...newEvents);
      }
    }

    // Sort by time, most recent first
    return events.sort(
      (a, b) => new Date(b.hasPointInTime).getTime() - new Date(a.hasPointInTime).getTime()
    );
  }

  private async fetchEventsByAction(action: REAAction): Promise<EconomicEvent[]> {
    const result = await this.holochainClient.callZome<HolochainEconomicEventOutput[]>({
      zomeName: 'content_store',
      fnName: 'get_events_by_action',
      payload: action,
    });

    if (!result.success || !result.data) {
      return [];
    }

    return result.data.map(o => this.transformToEconomicEvent(o));
  }

  private async fetchEventsByLamadType(lamadType: LamadEventType): Promise<EconomicEvent[]> {
    const result = await this.holochainClient.callZome<HolochainEconomicEventOutput[]>({
      zomeName: 'content_store',
      fnName: 'get_events_by_lamad_type',
      payload: lamadType,
    });

    if (!result.success || !result.data) {
      return [];
    }

    return result.data.map(o => this.transformToEconomicEvent(o));
  }

  private async doCreateEvent(input: CreateEconomicEventInput): Promise<EconomicEvent> {
    const payload = {
      action: input.action,
      provider: input.providerId,
      receiver: input.receiverId,
      resource_conforms_to: input.resourceConformsTo ?? null,
      resource_inventoried_as: input.resourceInventoriedAs ?? null,
      to_resource_inventoried_as: input.toResourceInventoriedAs ?? null,
      resource_classified_as: input.resourceClassifiedAs ?? [],
      resource_quantity_value: input.resourceQuantityValue ?? null,
      resource_quantity_unit: input.resourceQuantityUnit ?? null,
      effort_quantity_value: input.effortQuantityValue ?? null,
      effort_quantity_unit: input.effortQuantityUnit ?? null,
      input_of: input.inputOf ?? null,
      output_of: input.outputOf ?? null,
      fulfills: input.fulfills ?? [],
      realization_of: input.realizationOf ?? null,
      agreed_in: input.agreedIn ?? null,
      satisfies: input.satisfies ?? [],
      in_scope_of: input.inScopeOf ?? [],
      note: input.note ?? null,
      lamad_event_type: input.lamadEventType ?? null,
    };

    const result = await this.holochainClient.callZome<HolochainEconomicEventOutput>({
      zomeName: 'content_store',
      fnName: 'create_economic_event',
      payload,
    });

    if (!result.success || !result.data) {
      throw new Error(result.error ?? 'Failed to create economic event');
    }

    // Clear cache since we created a new event
    this.clearCache();

    return this.transformToEconomicEvent(result.data);
  }

  // ===========================================================================
  // Transformation - Holochain Entry → EconomicEvent
  // ===========================================================================

  private transformToEconomicEvent(output: HolochainEconomicEventOutput): EconomicEvent {
    const hc = output.event;

    // Parse JSON fields
    const resourceClassifiedAs = this.safeParseJson<string[]>(hc.resourceClassifiedAsJson, []);
    const fulfills = this.safeParseJson<string[]>(hc.fulfillsJson, []);
    const satisfies = this.safeParseJson<string[]>(hc.satisfiesJson, []);
    const inScopeOf = this.safeParseJson<string[]>(hc.inScopeOfJson, []);
    const metadata = this.safeParseJson<Record<string, unknown>>(hc.metadataJson, {});

    return {
      id: hc.id,
      action: hc.action as REAAction,
      provider: hc.provider,
      receiver: hc.receiver,
      resourceConformsTo: hc.resourceConformsTo ?? undefined,
      resourceInventoriedAs: hc.resourceInventoriedAs ?? undefined,
      toResourceInventoriedAs: hc.toResourceInventoriedAs ?? undefined,
      resourceClassifiedAs:
        resourceClassifiedAs.length > 0
          ? (resourceClassifiedAs as ResourceClassification[])
          : undefined,
      resourceQuantity: hc.resourceQuantityValue
        ? {
            hasNumericalValue: hc.resourceQuantityValue,
            hasUnit: hc.resourceQuantityUnit ?? 'unit',
          }
        : undefined,
      effortQuantity: hc.effortQuantityValue
        ? { hasNumericalValue: hc.effortQuantityValue, hasUnit: hc.effortQuantityUnit ?? 'unit' }
        : undefined,
      hasPointInTime: hc.hasPointInTime,
      hasDuration: hc.hasDuration ?? undefined,
      inputOf: hc.inputOf ?? undefined,
      outputOf: hc.outputOf ?? undefined,
      fulfills: fulfills.length > 0 ? fulfills : undefined,
      realizationOf: hc.realizationOf ?? undefined,
      agreedIn: hc.agreedIn ?? undefined,
      satisfies: satisfies.length > 0 ? satisfies : undefined,
      inScopeOf: inScopeOf.length > 0 ? inScopeOf : undefined,
      note: hc.note ?? undefined,
      state: hc.state as EventState,
      triggeredBy: hc.triggeredBy ?? undefined,
      atLocation: hc.atLocation ?? undefined,
      image: hc.image ?? undefined,
      metadata: {
        ...metadata,
        lamadEventType: hc.lamadEventType ?? undefined,
      },
    };
  }

  private safeParseJson<T>(json: string | null | undefined, defaultValue: T): T {
    if (!json) return defaultValue;
    try {
      return JSON.parse(json) as T;
    } catch {
      return defaultValue;
    }
  }
}
