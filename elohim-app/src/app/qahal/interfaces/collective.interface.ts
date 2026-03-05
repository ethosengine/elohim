/**
 * ICollective — Abstract interface for governance context operations.
 *
 * Decouples collective management from the concrete HTTP implementation.
 * Consumers inject the COLLECTIVE token; the default factory resolves
 * to CollectiveService (HTTP client for storage API).
 *
 * Tests provide a mock ICollective via the same token — no concrete
 * service import needed.
 *
 * @example
 * ```typescript
 * @Injectable({ providedIn: 'root' })
 * export class GovernanceComponent {
 *   private readonly collective = inject(COLLECTIVE);
 *
 *   loadCollectives(): Observable<CollectiveView[]> {
 *     return this.collective.listCollectives();
 *   }
 * }
 * ```
 */

import { InjectionToken, inject } from '@angular/core';

import { CollectiveService } from '../services/collective.service';

import type {
  CollectiveParticipationView,
  CollectiveView,
  CreateCollectiveInputView,
  GovernanceLayer,
} from '@app/qahal/models/collective.model';
import type { Observable } from 'rxjs';

/**
 * Abstract collective service — manages governance contexts and participation.
 *
 * Implementations handle HTTP transport, caching, and response mapping.
 * The contract covers CRUD for collectives and participation management.
 */
export interface ICollective {
  /** List collectives, optionally filtered by governance layer */
  listCollectives(params?: {
    governanceLayer?: GovernanceLayer;
    reach?: string;
    activeOnly?: boolean;
  }): Observable<CollectiveView[]>;

  /** Get a single collective by ID */
  getCollective(id: string): Observable<CollectiveView>;

  /** Create a collective */
  createCollective(input: CreateCollectiveInputView): Observable<CollectiveView>;

  /** Get all participants of a collective */
  getParticipants(collectiveId: string): Observable<CollectiveParticipationView[]>;

  /** Add a participant to a collective */
  addParticipant(
    collectiveId: string,
    humanId: string,
    options?: { intimacyLevel?: string; roleContext?: string }
  ): Observable<CollectiveParticipationView>;

  /** Depart from a collective (soft exit) */
  departCollective(collectiveId: string, humanId: string): Observable<void>;

  /** Get all collectives a human participates in */
  getCollectivesForHuman(humanId: string): Observable<CollectiveParticipationView[]>;
}

/**
 * Injection token for the collective service.
 *
 * Default factory resolves to CollectiveService which provides:
 * - HTTP CRUD for collectives via storage API
 * - Participation management with graduated intimacy
 *
 * Override in tests:
 * ```typescript
 * { provide: COLLECTIVE, useValue: mockCollective }
 * ```
 */
export const COLLECTIVE = new InjectionToken<ICollective>('Collective', {
  providedIn: 'root',
  factory: () => inject(CollectiveService),
});
