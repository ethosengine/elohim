/**
 * IStewardshipPolicy — Abstract interface for graduated capability management.
 *
 * Decouples stewardship policy operations from the concrete Holochain implementation.
 * Consumers inject the STEWARDSHIP_POLICY token; the default factory resolves
 * to StewardshipService (Holochain zome calls via imagodei DNA).
 *
 * Tests provide a mock IStewardshipPolicy via the same token — no concrete
 * service import needed.
 *
 * @example
 * ```typescript
 * @Injectable({ providedIn: 'root' })
 * export class PolicyGuard {
 *   private readonly stewardship = inject(STEWARDSHIP_POLICY);
 *
 *   async canAccess(feature: string): Promise<boolean> {
 *     return this.stewardship.checkFeatureAccess(feature);
 *   }
 * }
 * ```
 */

import { InjectionToken, inject } from '@angular/core';

import { StewardshipApiService } from '../services/stewardship-api.service';

import type {
  ComputedPolicy,
  CreateGrantInput,
  DelegateGrantInput,
  FileAppealInput,
  PolicyDecision,
  StewardshipAppeal,
  StewardshipGrant,
} from '../models/stewardship.model';

/**
 * Abstract stewardship policy — policy checks, grant management, and appeals.
 *
 * Implementations handle Holochain zome calls, policy computation,
 * and local caching. Power scales with responsibility, not role assignment.
 */
export interface IStewardshipPolicy {
  // ===========================================================================
  // Policy Checks
  // ===========================================================================

  /** Check if content can be accessed based on current policy */
  checkContentAccess(
    contentHash: string,
    categories: string[],
    ageRating?: string,
    reachLevel?: number
  ): Promise<PolicyDecision>;

  /** Check if a feature is accessible based on current policy */
  checkFeatureAccess(feature: string): Promise<boolean>;

  /** Check if a route is accessible based on current policy */
  checkRouteAccess(route: string): Promise<boolean>;

  /** Get my computed policy (merged from all layers) */
  getMyPolicy(): Promise<ComputedPolicy | null>;

  // ===========================================================================
  // Grant Operations
  // ===========================================================================

  /** Create a stewardship grant for another user */
  createGrant(input: CreateGrantInput): Promise<StewardshipGrant | null>;

  /** Delegate an existing grant to another steward */
  delegateGrant(input: DelegateGrantInput): Promise<StewardshipGrant | null>;

  /** Revoke a stewardship grant */
  revokeGrant(grantId: string): Promise<boolean>;

  /** Get grants where I am steward (my subjects) */
  getMySubjects(): Promise<StewardshipGrant[]>;

  /** Get grants where I am being stewarded (my stewards) */
  getMyStewards(): Promise<StewardshipGrant[]>;

  // ===========================================================================
  // Appeal Operations
  // ===========================================================================

  /** File an appeal against a grant or policy */
  fileAppeal(input: FileAppealInput): Promise<StewardshipAppeal | null>;

  /** Get my appeals (where I am appellant) */
  getMyAppeals(): Promise<StewardshipAppeal[]>;
}

/**
 * Injection token for the stewardship policy service.
 *
 * Default factory resolves to StewardshipService which provides:
 * - Holochain zome calls for grants, policies, and appeals via imagodei DNA
 * - Computed policy merging from all layers
 * - Local signal-based caching with fail-open defaults
 *
 * Override in tests:
 * ```typescript
 * { provide: STEWARDSHIP_POLICY, useValue: mockStewardshipPolicy }
 * ```
 */
export const STEWARDSHIP_POLICY = new InjectionToken<IStewardshipPolicy>('StewardshipPolicy', {
  providedIn: 'root',
  factory: () => inject(StewardshipApiService),
});
