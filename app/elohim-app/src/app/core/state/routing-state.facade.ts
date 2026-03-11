/**
 * Routing State Facade - Aggregates state for route guards and navigation decisions.
 *
 * Purpose:
 * - Provides a testable abstraction layer between guards and state services
 * - Aggregates authentication, session, and authorization state
 * - Decouples routing logic from Holochain implementation details
 *
 * Benefits:
 * - Guards depend on facade, not concrete services
 * - Easy to mock for testing (single dependency)
 * - Centralized routing authorization logic
 * - Can swap backend implementations without changing guards
 *
 * Design:
 * - Read-only computed signals (no state mutation through facade)
 * - Aggregates state from IdentityService, SessionHumanService, AuthService
 * - No business logic - pure state aggregation
 */

import { Injectable, computed, inject } from '@angular/core';

import { isNetworkMode } from '../../imagodei/models/identity.model';
import { IdentityService } from '../../imagodei/services/identity.service';
import { SessionHumanService } from '../../imagodei/services/session-human.service';

/**
 * Routing decision state exposed to guards.
 */
export interface RoutingState {
  /** Is user authenticated via network (hosted or steward) */
  readonly isNetworkAuthenticated: boolean;

  /** Is user authenticated OR has session */
  readonly hasAnyIdentity: boolean;

  /** Has active session (even if not network authenticated) */
  readonly hasActiveSession: boolean;

  /** User's attestations (empty array if not authenticated) */
  readonly attestations: readonly string[];

  /** Current identity mode */
  readonly identityMode: string | null;

  /** Human ID (from network identity or session) */
  readonly humanId: string | null;
}

@Injectable({ providedIn: 'root' })
export class RoutingStateFacade {
  // ==========================================================================
  // Dependencies
  // ==========================================================================

  private readonly identityService = inject(IdentityService);
  private readonly sessionHumanService = inject(SessionHumanService);

  // ==========================================================================
  // Computed Signals (Read-only State)
  // ==========================================================================

  /**
   * Is user authenticated via network (hosted or steward mode)?
   * Used by identityGuard and attestationGuard.
   */
  readonly isNetworkAuthenticated = computed(() => {
    const mode = this.identityService.mode();
    return isNetworkMode(mode) && this.identityService.isAuthenticated();
  });

  /**
   * Does user have any identity (session OR network)?
   * Used by sessionOrAuthGuard.
   */
  readonly hasAnyIdentity = computed(() => {
    return this.isNetworkAuthenticated() || this.sessionHumanService.hasSession();
  });

  /**
   * Does user have an active session?
   * Used by sessionOrAuthGuard fallback.
   */
  readonly hasActiveSession = computed(() => {
    return this.sessionHumanService.hasSession();
  });

  /**
   * User's attestations (empty if not authenticated).
   * Used by attestationGuard.
   */
  readonly attestations = computed(() => {
    return this.identityService.attestations();
  });

  /**
   * Current identity mode.
   */
  readonly identityMode = computed(() => {
    return this.identityService.mode();
  });

  /**
   * Human ID from network identity or session.
   */
  readonly humanId = computed(() => {
    return this.identityService.humanId();
  });

  /**
   * Get complete routing state as object (for logging/debugging).
   */
  readonly routingState = computed<RoutingState>(() => ({
    isNetworkAuthenticated: this.isNetworkAuthenticated(),
    hasAnyIdentity: this.hasAnyIdentity(),
    hasActiveSession: this.hasActiveSession(),
    attestations: this.attestations(),
    identityMode: this.identityMode(),
    humanId: this.humanId(),
  }));

  // ==========================================================================
  // Query Methods (for guards that need boolean checks)
  // ==========================================================================

  /**
   * Check if user has a specific attestation.
   *
   * @param attestationType - Attestation type to check
   * @returns True if user has the attestation
   */
  hasAttestation(attestationType: string): boolean {
    return this.attestations().includes(attestationType);
  }

  /**
   * Check if user can access network-gated content.
   * Requires network authentication (hosted or steward mode).
   */
  canAccessNetworkContent(): boolean {
    return this.isNetworkAuthenticated();
  }

  /**
   * Check if user can access session-gated content.
   * Requires session OR network authentication.
   */
  canAccessSessionContent(): boolean {
    return this.hasAnyIdentity();
  }
}
