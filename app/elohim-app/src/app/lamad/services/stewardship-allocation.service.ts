/**
 * Stewardship Allocation Service - One-to-Many Content Stewardship
 *
 * From the Manifesto (Part IV-C):
 * "Content isn't ever owned by who might create it, it's stewarded by whoever
 * has the most relational connection to the content itself."
 *
 * This service provides:
 * - Content stewardship queries (who stewards this content?)
 * - Steward portfolio queries (what does this steward care for?)
 * - Allocation management (create, update, dispute)
 * - Recognition flow helpers (distribute recognition to stewards)
 *
 * Architecture:
 *   StewardshipAllocationService → StorageApiService → elohim-storage → SQLite
 *
 * @see StewardService for premium gates and revenue
 * @see ContributorService for contributor impact tracking
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, signal, inject } from '@angular/core';

// @coverage: 14.8% (2026-02-24)

import { Observable, of, BehaviorSubject, map, catchError, tap } from 'rxjs';

import { StorageApiService } from '@app/elohim/services/storage-api.service';

import {
  CreateAllocationInput,
  UpdateAllocationInput,
  BulkAllocationResult,
  GovernanceState,
  ContributionType,
} from '../models/stewardship-allocation.model';

import type {
  StewardshipAllocationView,
  ContentStewardshipView,
} from '@elohim/storage-client/generated';

// =============================================================================
// Service Types
// =============================================================================

/**
 * Steward portfolio summary for UI display.
 */
export interface StewardPortfolio {
  stewardPresenceId: string;
  allocations: StewardshipAllocationView[];
  totalRecognition: number;
  contentCount: number;
  activeDisputeCount: number;
}

@Injectable({
  providedIn: 'root',
})
export class StewardshipAllocationService {
  private readonly storageApi = inject(StorageApiService);
  private readonly http = inject(HttpClient);

  // ============================================================================
  // Reactive State
  // ============================================================================

  /** Currently viewed steward's portfolio */
  private readonly _currentPortfolio$ = new BehaviorSubject<StewardPortfolio | null>(null);
  readonly currentPortfolio$ = this._currentPortfolio$.asObservable();

  /** Currently viewed content's stewardship */
  private readonly _currentStewardship$ = new BehaviorSubject<ContentStewardshipView | null>(null);
  readonly currentStewardship$ = this._currentStewardship$.asObservable();

  /** Loading state */
  private readonly _loading = signal(false);
  readonly loading = this._loading.asReadonly();

  // ============================================================================
  // Content Stewardship Queries
  // ============================================================================

  /**
   * Get all stewardship data for a content piece.
   *
   * Returns aggregate view with:
   * - All active allocations
   * - Primary steward identification
   * - Dispute status
   */
  getContentStewardship(contentId: string): Observable<ContentStewardshipView> {
    this._loading.set(true);

    return this.storageApi.getContentStewardship(contentId).pipe(
      tap(stewardship => {
        this._currentStewardship$.next(stewardship);
        this._loading.set(false);
      }),
      catchError(_error => {
        this._loading.set(false);
        // Return empty stewardship on error
        return of({
          contentId,
          allocations: [],
          totalAllocation: 0,
          hasDisputes: false,
          primarySteward: null,
        } as ContentStewardshipView);
      })
    );
  }

  /**
   * Get the primary steward for a content piece.
   *
   * The primary steward is the one with the highest allocation ratio.
   */
  getPrimarySteward(contentId: string): Observable<StewardshipAllocationView | null> {
    return this.getContentStewardship(contentId).pipe(
      map(stewardship => stewardship.primarySteward)
    );
  }

  /**
   * Check if content has any stewardship allocations.
   */
  hasAllocations(contentId: string): Observable<boolean> {
    return this.storageApi
      .getStewardshipAllocations({
        contentId,
        activeOnly: true,
        limit: 1,
      })
      .pipe(
        map(allocations => allocations.length > 0),
        catchError(() => of(false))
      );
  }

  // ============================================================================
  // Steward Portfolio Queries
  // ============================================================================

  /**
   * Get all stewardship allocations for a household.
   *
   * Calls `GET /api/v1/households/{id}/stewardship-allocations`.
   * Gracefully degrades to empty array if the endpoint is not yet live (Task G2).
   */
  listForHousehold(householdId: string): Observable<StewardshipAllocationView[]> {
    return this.http
      .get<StewardshipAllocationView[]>(
        `/api/v1/households/${encodeURIComponent(householdId)}/stewardship-allocations`
      )
      .pipe(catchError(() => of([])));
  }

  /**
   * Get all allocations for a steward (their portfolio).
   */
  getStewardPortfolio(stewardPresenceId: string): Observable<StewardPortfolio> {
    this._loading.set(true);

    return this.storageApi.getAllocationsForSteward(stewardPresenceId).pipe(
      map(allocations => {
        const portfolio: StewardPortfolio = {
          stewardPresenceId,
          allocations,
          totalRecognition: allocations.reduce((sum, a) => sum + a.recognitionAccumulated, 0),
          contentCount: allocations.length,
          activeDisputeCount: allocations.filter(a => a.governanceState === 'disputed').length,
        };
        this._currentPortfolio$.next(portfolio);
        return portfolio;
      }),
      tap(() => this._loading.set(false)),
      catchError(_error => {
        this._loading.set(false);
        return of({
          stewardPresenceId,
          allocations: [],
          totalRecognition: 0,
          contentCount: 0,
          activeDisputeCount: 0,
        });
      })
    );
  }

  /**
   * Get content IDs stewarded by a specific presence.
   */
  getStewartedContentIds(stewardPresenceId: string): Observable<string[]> {
    return this.storageApi.getAllocationsForSteward(stewardPresenceId).pipe(
      map(allocations => allocations.map(a => a.contentId)),
      catchError(() => of([]))
    );
  }

  // ============================================================================
  // Allocation Management
  // ============================================================================

  /**
   * Create a new stewardship allocation.
   */
  createAllocation(input: CreateAllocationInput): Observable<StewardshipAllocationView> {
    return this.storageApi.createStewardshipAllocation(input);
  }

  /**
   * Update an existing allocation.
   */
  updateAllocation(
    allocationId: string,
    input: UpdateAllocationInput
  ): Observable<StewardshipAllocationView> {
    return this.storageApi.updateStewardshipAllocation(allocationId, input);
  }

  /**
   * Delete an allocation (hard delete - use with caution).
   */
  deleteAllocation(allocationId: string): Observable<void> {
    return this.storageApi.deleteStewardshipAllocation(allocationId);
  }

  /**
   * Bulk create allocations (for seeding/import operations).
   */
  bulkCreateAllocations(inputs: CreateAllocationInput[]): Observable<BulkAllocationResult> {
    return this.storageApi.bulkCreateAllocations(inputs).pipe(
      tap(_result => {
        // Handle any errors if needed in the future
      })
    );
  }

  // ============================================================================
  // Dispute Management
  // ============================================================================

  /**
   * File a dispute on an allocation.
   *
   * Used when a steward believes their allocation ratio is incorrect
   * or they're being subject to a "hostile takeover".
   * Dispute ID is generated server-side.
   */
  fileDispute(
    allocationId: string,
    disputedBy: string,
    reason: string
  ): Observable<StewardshipAllocationView> {
    return this.storageApi.fileAllocationDispute(allocationId, {
      disputedBy,
      reason,
    });
  }

  /**
   * Resolve a dispute (Elohim ratification).
   *
   * Only Elohim governance can resolve disputes.
   */
  resolveDispute(
    allocationId: string,
    ratifierId: string,
    newState: GovernanceState
  ): Observable<StewardshipAllocationView> {
    return this.storageApi.resolveAllocationDispute(allocationId, {
      ratifierId,
      newState,
    });
  }

  /**
   * Get all disputed allocations (for governance review).
   */
  getDisputedAllocations(): Observable<StewardshipAllocationView[]> {
    return this.storageApi.getStewardshipAllocations({
      governanceState: 'disputed',
    });
  }

  // ============================================================================
  // Bootstrap Helpers
  // ============================================================================

  /**
   * Assign a steward to all content that has no allocations.
   *
   * Used for bootstrapping when a single steward (e.g., Matthew Dowell)
   * should be assigned to all genesis content.
   */
  bootstrapSteward(
    stewardPresenceId: string,
    contentIds: string[],
    contributionType: ContributionType = 'inherited'
  ): Observable<BulkAllocationResult> {
    const inputs: CreateAllocationInput[] = contentIds.map(contentId => ({
      contentId,
      stewardPresenceId,
      allocationRatio: 1,
      allocationMethod: 'manual',
      contributionType,
      note: 'Bootstrap steward assignment',
    }));

    return this.bulkCreateAllocations(inputs);
  }

  // ============================================================================
  // Utilities
  // ============================================================================

  /**
   * Clear cached state.
   */
  clearCache(): void {
    this._currentPortfolio$.next(null);
    this._currentStewardship$.next(null);
  }
}
