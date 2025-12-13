/**
 * Contributor Service - Lamad Contributor Dashboard & Impact Tracking
 *
 * From the Manifesto (Part IV-B: Contributor Presence and the Economics of Honor):
 * "When someone's work is referenced in Lamad—whether a book author, a video
 * contributor, or a researcher—a presence is established for them. This presence
 * is not an account they control (yet), but a place where recognition can
 * accumulate."
 *
 * This service provides access to contributor dashboards and impact metrics.
 * It's the Lamad-specific layer over Shefa economic primitives.
 *
 * Architecture:
 *   ContributorService → HolochainClientService → Holochain Conductor → DHT
 *
 * Use cases:
 * - View contributor dashboard (aggregated impact view)
 * - Track recognition history
 * - Analyze content impact
 * - Get current agent's contributor stats
 *
 * @see AppreciationService for the underlying recognition primitives
 * @see StewardService for steward economy operations
 */

import { Injectable, signal, computed } from '@angular/core';
import { BehaviorSubject, Observable, of, from, defer } from 'rxjs';
import { catchError, map, shareReplay, tap } from 'rxjs/operators';
import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import {
  LamadContributorDashboard,
  LamadContributorImpact,
  LamadContributorRecognition,
  LamadContentImpactSummary,
  LamadRecognitionEventSummary,
} from '../models/steward-economy.model';

// =============================================================================
// Holochain Types (match Rust DNA structures)
// =============================================================================

interface HolochainContributorDashboard {
  contributor_id: string;
  total_recognition_points: number;
  total_learners_reached: number;
  total_content_mastered: number;
  total_discoveries_sparked: number;
  impact_by_content: HolochainContentImpactSummary[];
  recent_events: HolochainRecognitionEventSummary[];
  impact: HolochainContributorImpactOutput | null;
}

interface HolochainContentImpactSummary {
  content_id: string;
  recognition_points: number;
  learners_reached: number;
  mastery_count: number;
}

interface HolochainRecognitionEventSummary {
  learner_id: string;
  content_id: string;
  flow_type: string;
  recognition_points: number;
  occurred_at: string;
}

interface HolochainContributorImpact {
  id: string;
  contributor_id: string;
  total_recognition_points: number;
  total_learners_reached: number;
  total_content_mastered: number;
  total_discoveries_sparked: number;
  unique_content_engaged: number;
  impact_by_content_json: string;
  first_recognition_at: string;
  last_recognition_at: string;
  created_at: string;
  updated_at: string;
}

interface HolochainContributorImpactOutput {
  action_hash: Uint8Array;
  impact: HolochainContributorImpact;
}

interface HolochainContributorRecognition {
  id: string;
  contributor_id: string;
  content_id: string;
  learner_id: string;
  appreciation_of_event_id: string;
  flow_type: string;
  recognition_points: number;
  path_id: string | null;
  challenge_id: string | null;
  note: string | null;
  metadata_json: string;
  occurred_at: string;
}

interface HolochainContributorRecognitionOutput {
  action_hash: Uint8Array;
  recognition: HolochainContributorRecognition;
}

// =============================================================================
// Service Implementation
// =============================================================================

@Injectable({
  providedIn: 'root',
})
export class ContributorService {
  /**
   * Whether contributor service is available.
   * Mirrors HolochainClientService connection state.
   */
  private readonly availableSignal = signal(false);
  readonly available = this.availableSignal.asReadonly();

  /** Computed: true when Holochain client is connected AND service available */
  readonly ready = computed(() => this.available() && this.holochainClient.isConnected());

  /**
   * Current agent's contributor dashboard (reactive state).
   * Updated when getMyDashboard() is called.
   */
  private readonly dashboardSubject = new BehaviorSubject<LamadContributorDashboard | null>(null);
  readonly dashboard$ = this.dashboardSubject.asObservable();

  /** Cache for dashboards by contributor ID */
  private readonly dashboardCache = new Map<string, Observable<LamadContributorDashboard | null>>();

  /** Cache for recognition history */
  private readonly recognitionCache = new Map<string, Observable<LamadContributorRecognition[]>>();

  constructor(private readonly holochainClient: HolochainClientService) {}

  /**
   * Check if contributor service is available.
   */
  isAvailable(): boolean {
    return this.availableSignal();
  }

  // ===========================================================================
  // Dashboard Methods
  // ===========================================================================

  /**
   * Get contributor dashboard by ID.
   *
   * The dashboard aggregates:
   * - Total recognition points earned
   * - Number of unique learners reached
   * - Content mastery events
   * - Impact breakdown by content piece
   * - Recent recognition events
   *
   * @param contributorId - The contributor presence ID
   * @returns Observable of the contributor dashboard
   */
  getDashboard(contributorId: string): Observable<LamadContributorDashboard | null> {
    if (!this.isAvailable()) {
      return of(null);
    }

    if (!this.dashboardCache.has(contributorId)) {
      const request = defer(() =>
        from(this.fetchDashboard(contributorId))
      ).pipe(
        shareReplay(1),
        catchError((err) => {
          console.warn(`[ContributorService] Failed to fetch dashboard for "${contributorId}":`, err);
          return of(null);
        })
      );

      this.dashboardCache.set(contributorId, request);
    }

    return this.dashboardCache.get(contributorId)!;
  }

  /**
   * Get the current agent's contributor dashboard.
   *
   * Also updates the reactive dashboard$ observable.
   *
   * @returns Observable of the current agent's dashboard
   */
  getMyDashboard(): Observable<LamadContributorDashboard | null> {
    if (!this.isAvailable()) {
      return of(null);
    }

    return defer(() =>
      from(this.fetchMyDashboard())
    ).pipe(
      tap(dashboard => this.dashboardSubject.next(dashboard)),
      catchError((err) => {
        console.warn('[ContributorService] Failed to fetch my dashboard:', err);
        return of(null);
      })
    );
  }

  // ===========================================================================
  // Impact Methods
  // ===========================================================================

  /**
   * Get detailed impact metrics for a contributor.
   *
   * @param contributorId - The contributor presence ID
   * @returns Observable of the impact details (or null if not found)
   */
  getImpact(contributorId: string): Observable<LamadContributorImpact | null> {
    if (!this.isAvailable()) {
      return of(null);
    }

    return defer(() =>
      from(this.fetchImpact(contributorId))
    ).pipe(
      catchError((err) => {
        console.warn(`[ContributorService] Failed to fetch impact for "${contributorId}":`, err);
        return of(null);
      })
    );
  }

  /**
   * Get content impact breakdown for a contributor.
   *
   * Shows how much recognition each content piece has earned.
   *
   * @param contributorId - The contributor presence ID
   * @returns Observable of content impact summaries
   */
  getContentImpact(contributorId: string): Observable<LamadContentImpactSummary[]> {
    // This is derived from the dashboard
    return this.getDashboard(contributorId).pipe(
      map(dashboard => dashboard?.impactByContent ?? []),
      catchError(() => of([])),
    );
  }

  // ===========================================================================
  // Recognition History Methods
  // ===========================================================================

  /**
   * Get recognition history for a contributor.
   *
   * Shows individual recognition events received over time.
   *
   * @param contributorId - The contributor presence ID
   * @returns Observable of recognition events (most recent first)
   */
  getRecognitionHistory(contributorId: string): Observable<LamadContributorRecognition[]> {
    if (!this.isAvailable()) {
      return of([]);
    }

    if (!this.recognitionCache.has(contributorId)) {
      const request = defer(() =>
        from(this.fetchRecognitionHistory(contributorId))
      ).pipe(
        shareReplay(1),
        catchError((err) => {
          console.warn(`[ContributorService] Failed to fetch recognition for "${contributorId}":`, err);
          return of([]);
        })
      );

      this.recognitionCache.set(contributorId, request);
    }

    return this.recognitionCache.get(contributorId)!;
  }

  // ===========================================================================
  // Cache Management
  // ===========================================================================

  /**
   * Clear all caches (useful when data changes)
   */
  clearCache(): void {
    this.dashboardCache.clear();
    this.recognitionCache.clear();
  }

  /**
   * Refresh the current agent's dashboard.
   * Clears cache and fetches fresh data.
   */
  refreshMyDashboard(): Observable<LamadContributorDashboard | null> {
    this.dashboardCache.clear();
    return this.getMyDashboard();
  }

  /**
   * Test if contributor API is reachable.
   * Updates the available signal based on result.
   */
  async testAvailability(): Promise<boolean> {
    try {
      const result = await this.holochainClient.callZome<HolochainContributorDashboard | null>({
        zomeName: 'content_store',
        fnName: 'get_my_contributor_dashboard',
        payload: null,
      });

      // Even a null result (no dashboard yet) means the zome is available
      this.availableSignal.set(result.success);
      return result.success;
    } catch (err) {
      console.warn('[ContributorService] Availability test failed:', err);
      this.availableSignal.set(false);
      return false;
    }
  }

  // ===========================================================================
  // Private Methods - Zome Calls
  // ===========================================================================

  private async fetchDashboard(contributorId: string): Promise<LamadContributorDashboard | null> {
    const result = await this.holochainClient.callZome<HolochainContributorDashboard | null>({
      zomeName: 'content_store',
      fnName: 'get_contributor_dashboard',
      payload: contributorId,
    });

    if (!result.success || !result.data) {
      return null;
    }

    return this.transformDashboard(result.data);
  }

  private async fetchMyDashboard(): Promise<LamadContributorDashboard | null> {
    const result = await this.holochainClient.callZome<HolochainContributorDashboard | null>({
      zomeName: 'content_store',
      fnName: 'get_my_contributor_dashboard',
      payload: null,
    });

    if (!result.success || !result.data) {
      return null;
    }

    return this.transformDashboard(result.data);
  }

  private async fetchImpact(contributorId: string): Promise<LamadContributorImpact | null> {
    const result = await this.holochainClient.callZome<HolochainContributorImpactOutput | null>({
      zomeName: 'content_store',
      fnName: 'get_contributor_impact',
      payload: contributorId,
    });

    if (!result.success || !result.data) {
      return null;
    }

    return this.transformImpact(result.data);
  }

  private async fetchRecognitionHistory(contributorId: string): Promise<LamadContributorRecognition[]> {
    const result = await this.holochainClient.callZome<HolochainContributorRecognitionOutput[]>({
      zomeName: 'content_store',
      fnName: 'get_recognition_by_contributor',
      payload: contributorId,
    });

    if (!result.success || !result.data) {
      return [];
    }

    return result.data
      .map(o => this.transformRecognition(o))
      .sort((a, b) => new Date(b.occurredAt).getTime() - new Date(a.occurredAt).getTime());
  }

  // ===========================================================================
  // Transformation - Holochain → Angular
  // ===========================================================================

  private transformDashboard(hc: HolochainContributorDashboard): LamadContributorDashboard {
    return {
      contributorId: hc.contributor_id,
      totalRecognitionPoints: hc.total_recognition_points,
      totalLearnersReached: hc.total_learners_reached,
      totalContentMastered: hc.total_content_mastered,
      totalDiscoveriesSparked: hc.total_discoveries_sparked,
      impactByContent: hc.impact_by_content.map(ic => ({
        contentId: ic.content_id,
        recognitionPoints: ic.recognition_points,
        learnersReached: ic.learners_reached,
        masteryCount: ic.mastery_count,
      })),
      recentEvents: hc.recent_events.map(re => ({
        learnerId: re.learner_id,
        contentId: re.content_id,
        flowType: re.flow_type,
        recognitionPoints: re.recognition_points,
        occurredAt: re.occurred_at,
      })),
      impact: hc.impact ? this.transformImpact(hc.impact) : null,
    };
  }

  private transformImpact(output: HolochainContributorImpactOutput): LamadContributorImpact {
    const hc = output.impact;
    const impactByContent = this.safeParseJson<LamadContentImpactSummary[]>(
      hc.impact_by_content_json,
      []
    );

    return {
      id: hc.id,
      contributorId: hc.contributor_id,
      totalRecognitionPoints: hc.total_recognition_points,
      totalLearnersReached: hc.total_learners_reached,
      totalContentMastered: hc.total_content_mastered,
      totalDiscoveriesSparked: hc.total_discoveries_sparked,
      uniqueContentEngaged: hc.unique_content_engaged,
      impactByContent,
      firstRecognitionAt: hc.first_recognition_at,
      lastRecognitionAt: hc.last_recognition_at,
      createdAt: hc.created_at,
      updatedAt: hc.updated_at,
    };
  }

  private transformRecognition(output: HolochainContributorRecognitionOutput): LamadContributorRecognition {
    const hc = output.recognition;
    const metadata = this.safeParseJson<Record<string, unknown>>(hc.metadata_json, {});

    return {
      id: hc.id,
      contributorId: hc.contributor_id,
      contentId: hc.content_id,
      learnerId: hc.learner_id,
      appreciationOfEventId: hc.appreciation_of_event_id,
      flowType: hc.flow_type,
      recognitionPoints: hc.recognition_points,
      pathId: hc.path_id,
      challengeId: hc.challenge_id,
      note: hc.note,
      metadata,
      occurredAt: hc.occurred_at,
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
