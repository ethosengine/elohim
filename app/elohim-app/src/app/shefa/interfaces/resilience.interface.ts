/**
 * IResilience -- Abstract interface for human resilience profile
 * computation and access within the Shefa layer.
 *
 * Decouples the resilience projection lifecycle (shard health, commitment
 * analysis, trust circle depth, content risk bucketing, elohim assessment)
 * from the concrete persistence layer. Consumers inject the RESILIENCE
 * token; the default factory will resolve to ResilienceApiService.
 *
 * Tests provide a mock IResilience via the same token -- no concrete
 * service import needed.
 *
 * @example
 * ```typescript
 * @Injectable({ providedIn: 'root' })
 * export class ResilienceDashboard {
 *   private readonly resilience = inject(RESILIENCE);
 *
 *   loadProfile(humanId: string) {
 *     return this.resilience.computeProfile(humanId);
 *   }
 * }
 * ```
 */

import { InjectionToken, inject } from '@angular/core';

import type { Observable } from 'rxjs';

import type {
  ResilienceProfile,
  ProtectionStatus,
  ResilienceAction,
  ContentRiskBucket,
  ElohimResilienceAssessment,
} from '../models/resilience-profile.model';
import { ResilienceApiService } from '../services/resilience-api.service';

// =============================================================================
// INTERFACE
// =============================================================================

/**
 * Abstract resilience profile manager -- computes and exposes a human's
 * P2P data protection posture across shard health, commitment coverage,
 * trust circle depth, content risk, and elohim narrative assessment.
 *
 * Implementations handle DHT projection queries for shard manifests,
 * mutual aid commitments, trust graphs, and elohim agent assessments.
 */
export interface IResilience {
  /**
   * Compute a fresh resilience profile for a human.
   *
   * Aggregates shard health, commitment coverage, trust circle depth,
   * content risk bucketing, and elohim assessment into a single profile.
   *
   * @param humanId - Agent public key of the human
   * @returns Promise resolving to the computed resilience profile
   */
  computeProfile(humanId: string): Promise<ResilienceProfile>;

  /**
   * Get a resilience profile as a refreshing Observable stream.
   *
   * Returns an Observable that emits updated resilience profiles on
   * the configured refresh interval.
   *
   * @param humanId - Agent public key of the human
   * @param refreshInterval - Milliseconds between refreshes (default 60000)
   * @returns Observable stream of resilience profile updates
   */
  getProfile$(humanId: string, refreshInterval?: number): Observable<ResilienceProfile>;

  /**
   * Get current resilience profile snapshot.
   *
   * @returns Current profile or null if not yet computed
   */
  getProfile(): ResilienceProfile | null;

  /**
   * Get current overall protection status.
   *
   * @returns Current protection status or null if not yet computed
   */
  getProtectionStatus(): ProtectionStatus | null;

  /**
   * Get the next recommended action to improve resilience.
   *
   * @returns Next action or null if no actions recommended
   */
  getNextAction(): ResilienceAction | null;

  /**
   * Get content risk breakdown by reach level.
   *
   * Returns risk buckets showing how well different categories of
   * content (by reach level) are protected across the network.
   *
   * @returns Array of content risk buckets
   */
  getContentRiskBreakdown(): ContentRiskBucket[];

  /**
   * Get the most recent elohim narrative assessment.
   *
   * Returns the LLM-generated assessment including narrative,
   * memories, concerns, and constitutional attestations.
   *
   * @returns Elohim assessment or null if not yet assessed
   */
  getElohimAssessment(): ElohimResilienceAssessment | null;
}

// =============================================================================
// INJECTION TOKEN
// =============================================================================

/**
 * Injection token for resilience profile computation and access.
 *
 * Default factory will resolve to ResilienceApiService (Task 3) which
 * provides:
 * - Resilience profile computation from DHT projections
 * - Observable streams with configurable refresh intervals
 * - Shard health, commitment, and trust circle analysis
 * - Content risk bucketing by reach level
 * - Elohim narrative assessment integration
 *
 * Override in tests:
 * ```typescript
 * { provide: RESILIENCE, useValue: mockResilience }
 * ```
 */
export const RESILIENCE = new InjectionToken<IResilience>('Resilience', {
  providedIn: 'root',
  factory: () => inject(ResilienceApiService),
});
