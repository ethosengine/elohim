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
  DevicePolicy,
  FileAppealInput,
  PolicyChainLink,
  PolicyDecision,
  StewardshipAppeal,
  StewardshipGrant,
  TimeAccessDecision,
  UpsertPolicyInput,
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

  // ===========================================================================
  // Policy Management
  // ===========================================================================

  /** Check time-based access status */
  checkTimeAccess(): Promise<TimeAccessDecision>;

  /** Get my grant for a specific subject */
  getGrantForSubject(subjectId: string): Promise<StewardshipGrant | null>;

  /** Get the active device policy for a subject */
  getSubjectPolicy(subjectId: string): Promise<DevicePolicy | null>;

  /** Get the parent's computed policy for a subject */
  getParentPolicy(subjectId: string): Promise<ComputedPolicy | null>;

  /** Get the policy inheritance chain for a subject */
  getPolicyChain(subjectId: string): Promise<PolicyChainLink[]>;

  /** Get my own policy inheritance chain */
  getMyPolicyChain(): Promise<PolicyChainLink[]>;

  /** Create or update a device policy */
  upsertPolicy(input: UpsertPolicyInput): Promise<DevicePolicy | null>;
}

/**
 * Injection token for the stewardship policy service.
 *
 * Default factory resolves to StewardshipApiService which provides:
 * - HTTP calls to `/api/v1/stewardship/*` endpoints on elohim-storage
 * - Computed policy merging from all layers
 * - Fail-open defaults for all policy checks
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
