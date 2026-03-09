/**
 * IPresenceLifecycle — Abstract interface for contributor presence management.
 *
 * Decouples presence lifecycle operations from the concrete Holochain implementation.
 * Consumers inject the PRESENCE_LIFECYCLE token; the default factory resolves
 * to PresenceService (Holochain zome calls via imagodei DNA).
 *
 * Tests provide a mock IPresenceLifecycle via the same token — no concrete
 * service import needed.
 *
 * @example
 * ```typescript
 * @Injectable({ providedIn: 'root' })
 * export class ContributorComponent {
 *   private readonly presence = inject(PRESENCE_LIFECYCLE);
 *
 *   async loadPresence(id: string): Promise<ContributorPresenceView | null> {
 *     return this.presence.getPresenceById(id);
 *   }
 * }
 * ```
 */

import { InjectionToken, inject } from '@angular/core';

import { PresenceApiService } from '../services/presence-api.service';

import type {
  ContributorPresenceView,
  CreatePresenceRequest,
  InitiateClaimRequest,
  PresenceState,
} from '../models/presence.model';

/**
 * Abstract presence lifecycle — manages contributor presence states.
 *
 * Implementations handle Holochain zome calls, wire format mapping,
 * and local caching. The lifecycle flows: UNCLAIMED -> STEWARDED -> CLAIMED.
 */
export interface IPresenceLifecycle {
  /** Create a new contributor presence for an absent contributor */
  createPresence(request: CreatePresenceRequest): Promise<ContributorPresenceView>;

  /** Begin stewardship of an unclaimed presence */
  beginStewardship(presenceId: string, commitmentNote?: string): Promise<ContributorPresenceView>;

  /** Get presences I'm currently stewarding */
  getMyStewardedPresences(): Promise<ContributorPresenceView[]>;

  /** Initiate a claim on a presence */
  initiateClaim(request: InitiateClaimRequest): Promise<ContributorPresenceView>;

  /** Verify and finalize a claim */
  verifyClaim(presenceId: string): Promise<ContributorPresenceView>;

  /** Get a presence by ID */
  getPresenceById(id: string): Promise<ContributorPresenceView | null>;

  /** Get presences by state */
  getPresencesByState(state: PresenceState): Promise<ContributorPresenceView[]>;
}

/**
 * Injection token for the presence lifecycle service.
 *
 * Default factory resolves to PresenceService which provides:
 * - Holochain zome calls for presence CRUD via imagodei DNA
 * - Wire format mapping (snake_case -> camelCase)
 * - Local signal-based caching
 *
 * Override in tests:
 * ```typescript
 * { provide: PRESENCE_LIFECYCLE, useValue: mockPresenceLifecycle }
 * ```
 */
export const PRESENCE_LIFECYCLE = new InjectionToken<IPresenceLifecycle>('PresenceLifecycle', {
  providedIn: 'root',
  factory: () => inject(PresenceApiService),
});
