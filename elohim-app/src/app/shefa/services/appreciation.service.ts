/**
 * Appreciation Service - Shefa Recognition/Appreciation Flows
 *
 * From the Manifesto (Part IV-A: The Economy of Honor):
 * "Recognition flows are the currency of honor in the network.
 * When a learner engages with content, appreciation automatically
 * flows to the contributors who created that value."
 *
 * This service provides access to hREA Appreciation operations.
 * Appreciations are domain-agnostic (Shefa layer) records of value
 * recognition from one agent to another.
 *
 * Architecture:
 *   AppreciationService → HolochainClientService → Holochain Conductor → DHT
 *
 * Use cases:
 * - Query appreciations received by an agent (recognition earned)
 * - Query appreciations given by an agent (recognition expressed)
 * - Create new appreciation records
 *
 * @see EconomicService for broader economic event operations
 * @see ContributorService for Lamad-specific contributor recognition
 */

import { Injectable, signal, computed } from '@angular/core';

// @coverage: 92.6% (2026-02-04)

import { catchError, shareReplay } from 'rxjs/operators';

import { Observable, of, from, defer } from 'rxjs';

import { HolochainClientService } from '@app/elohim/services/holochain-client.service';

// =============================================================================
// Appreciation Types
// =============================================================================

/**
 * AppreciationDisplay - Flattened appreciation for UI consumption.
 *
 * This is a UI-friendly format with separate quantityValue/quantityUnit fields
 * rather than the nested Measure object from the canonical Appreciation type.
 *
 * @see Appreciation in rea-bridge.model.ts for the canonical type
 */
export interface AppreciationDisplay {
  id: string;
  /** What is being appreciated (event ID, content ID, etc.) */
  appreciationOf: string;
  /** Who is expressing appreciation */
  appreciatedBy: string;
  /** Who receives the appreciation */
  appreciationTo: string;
  /** Quantity of appreciation */
  quantityValue: number;
  /** Unit of appreciation (e.g., 'recognition-points', 'affinity') */
  quantityUnit: string;
  /** Optional note */
  note: string | null;
  /** When created */
  createdAt: string;
}

/**
 * Input for creating an appreciation.
 */
export interface CreateAppreciationInput {
  /** What is being appreciated */
  appreciationOf: string;
  /** Who receives the appreciation */
  appreciationTo: string;
  /** Quantity of appreciation */
  quantityValue: number;
  /** Unit of appreciation */
  quantityUnit: string;
  /** Optional note */
  note?: string;
}

// =============================================================================
// Holochain Types (match Rust DNA structures)
// =============================================================================

interface HolochainAppreciation {
  id: string;
  appreciationOf: string;
  appreciatedBy: string;
  appreciationTo: string;
  quantityValue: number;
  quantityUnit: string;
  note: string | null;
  createdAt: string;
}

interface HolochainAppreciationOutput {
  actionHash: Uint8Array;
  appreciation: HolochainAppreciation;
}

// =============================================================================
// Service Implementation
// =============================================================================

@Injectable({
  providedIn: 'root',
})
export class AppreciationService {
  /**
   * Whether appreciation service is available.
   * Mirrors HolochainClientService connection state.
   */
  private readonly availableSignal = signal(false);
  readonly available = this.availableSignal.asReadonly();

  /** Computed: true when Holochain client is connected AND service available */
  readonly ready = computed(() => this.available() && this.holochainClient.isConnected());

  /** Cache for appreciations received */
  private readonly appreciationsForCache = new Map<string, Observable<AppreciationDisplay[]>>();

  /** Cache for appreciations given */
  private readonly appreciationsByCache = new Map<string, Observable<AppreciationDisplay[]>>();

  constructor(private readonly holochainClient: HolochainClientService) {}

  /**
   * Check if appreciation service is available.
   */
  isAvailable(): boolean {
    return this.availableSignal();
  }

  // ===========================================================================
  // Query Methods
  // ===========================================================================

  /**
   * Get appreciations received by an entity.
   *
   * This answers: "Who has appreciated this person/content/event?"
   *
   * @param appreciatedId - The ID of the appreciated entity
   * @returns Observable of appreciations received
   */
  getAppreciationsFor(appreciatedId: string): Observable<AppreciationDisplay[]> {
    if (!this.isAvailable()) {
      return of([]);
    }

    if (!this.appreciationsForCache.has(appreciatedId)) {
      const request = defer(() => from(this.fetchAppreciationsFor(appreciatedId))).pipe(
        shareReplay(1),
        catchError(_err => {
          return of([]);
        })
      );

      this.appreciationsForCache.set(appreciatedId, request);
    }

    return this.appreciationsForCache.get(appreciatedId)!;
  }

  /**
   * Get appreciations given by an agent.
   *
   * This answers: "What has this agent appreciated?"
   *
   * @param appreciatorId - The ID of the appreciating agent
   * @returns Observable of appreciations given
   */
  getAppreciationsBy(appreciatorId: string): Observable<AppreciationDisplay[]> {
    if (!this.isAvailable()) {
      return of([]);
    }

    if (!this.appreciationsByCache.has(appreciatorId)) {
      const request = defer(() => from(this.fetchAppreciationsBy(appreciatorId))).pipe(
        shareReplay(1),
        catchError(_err => {
          return of([]);
        })
      );

      this.appreciationsByCache.set(appreciatorId, request);
    }

    return this.appreciationsByCache.get(appreciatorId)!;
  }

  // ===========================================================================
  // Create Methods
  // ===========================================================================

  /**
   * Create a new appreciation record.
   *
   * The appreciatedBy field is automatically set to the current agent.
   *
   * @param input - The appreciation input
   * @returns Observable of the created appreciation
   */
  appreciate(input: CreateAppreciationInput): Observable<AppreciationDisplay> {
    if (!this.isAvailable()) {
      throw new Error('Appreciation service not available');
    }

    return defer(() => from(this.doAppreciate(input))).pipe(
      catchError(_err => {
        throw _err;
      })
    );
  }

  // ===========================================================================
  // Cache Management
  // ===========================================================================

  /**
   * Clear all caches (useful after creating appreciations)
   */
  clearCache(): void {
    this.appreciationsForCache.clear();
    this.appreciationsByCache.clear();
  }

  /**
   * Test if appreciation API is reachable.
   * Updates the available signal based on result.
   */
  async testAvailability(): Promise<boolean> {
    try {
      // Simple test: try to get appreciations for a non-existent entity
      const result = await this.holochainClient.callZome<HolochainAppreciationOutput[]>({
        zomeName: 'content_store',
        fnName: 'get_appreciations_for',
        payload: 'test-availability',
      });

      // Even an empty result means the zome is available
      this.availableSignal.set(result.success);
      return result.success;
    } catch {
      // Service availability check failed - Appreciation service unavailable
      this.availableSignal.set(false);
      return false;
    }
  }

  // ===========================================================================
  // Private Methods - Zome Calls
  // ===========================================================================

  private async fetchAppreciationsFor(appreciatedId: string): Promise<AppreciationDisplay[]> {
    const result = await this.holochainClient.callZome<HolochainAppreciationOutput[]>({
      zomeName: 'content_store',
      fnName: 'get_appreciations_for',
      payload: appreciatedId,
    });

    if (!result.success || !result.data) {
      return [];
    }

    return result.data
      .map(o => this.transformToAppreciation(o))
      .sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime());
  }

  private async fetchAppreciationsBy(appreciatorId: string): Promise<AppreciationDisplay[]> {
    const result = await this.holochainClient.callZome<HolochainAppreciationOutput[]>({
      zomeName: 'content_store',
      fnName: 'get_appreciations_by',
      payload: appreciatorId,
    });

    if (!result.success || !result.data) {
      return [];
    }

    return result.data
      .map(o => this.transformToAppreciation(o))
      .sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime());
  }

  private async doAppreciate(input: CreateAppreciationInput): Promise<AppreciationDisplay> {
    const payload = {
      appreciation_of: input.appreciationOf,
      appreciation_to: input.appreciationTo,
      quantity_value: input.quantityValue,
      quantity_unit: input.quantityUnit,
      note: input.note ?? null,
    };

    const result = await this.holochainClient.callZome<HolochainAppreciationOutput>({
      zomeName: 'content_store',
      fnName: 'create_appreciation',
      payload,
    });

    if (!result.success || !result.data) {
      throw new Error(result.error ?? 'Failed to create appreciation');
    }

    // Clear caches since we created a new appreciation
    this.clearCache();

    return this.transformToAppreciation(result.data);
  }

  // ===========================================================================
  // Transformation - Holochain Entry → AppreciationDisplay
  // ===========================================================================

  private transformToAppreciation(output: HolochainAppreciationOutput): AppreciationDisplay {
    const hc = output.appreciation;

    return {
      id: hc.id,
      appreciationOf: hc.appreciationOf,
      appreciatedBy: hc.appreciatedBy,
      appreciationTo: hc.appreciationTo,
      quantityValue: hc.quantityValue,
      quantityUnit: hc.quantityUnit,
      note: hc.note,
      createdAt: hc.createdAt,
    };
  }
}
