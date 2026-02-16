/**
 * Identity Guards - Route protection based on authentication state.
 *
 * Guards:
 * - identityGuard: Requires network authentication
 * - sessionOrAuthGuard: Allows session OR network authentication
 *
 * Usage in routes:
 *   { path: 'profile', canActivate: [identityGuard], component: ProfileComponent }
 *   { path: 'learn', canActivate: [sessionOrAuthGuard], component: LearnComponent }
 */

import { inject } from '@angular/core';
import { Router, type CanActivateFn, type UrlTree } from '@angular/router';

// @coverage: 100.0% (2026-02-05)

import { isNetworkMode } from '../models/identity.model';
import { AuthService } from '../services/auth.service';
import { IdentityService } from '../services/identity.service';
import { SessionHumanService } from '../services/session-human.service';

/** Login route for unauthenticated users */
const LOGIN_ROUTE = '/identity/login';

/** Max time to wait for identity state to settle after auth (ms) */
const IDENTITY_SETTLE_TIMEOUT = 5000;

/**
 * Guard that requires network authentication.
 *
 * Redirects to /login if not authenticated via network.
 * Passes return URL as query parameter for post-auth redirect.
 *
 * Handles the race between AuthService (updated immediately on login)
 * and IdentityService (async transition to hosted/steward mode).
 * When AuthService reports authenticated but IdentityService hasn't
 * settled yet, waits briefly for the identity state to transition.
 */
export const identityGuard: CanActivateFn = async (route, state): Promise<boolean | UrlTree> => {
  const identityService = inject(IdentityService);
  const authService = inject(AuthService);
  const router = inject(Router);

  // Fast path: identity already in network mode
  const mode = identityService.mode();
  if (isNetworkMode(mode) && identityService.isAuthenticated()) {
    return true;
  }

  // Auth is valid but identity hasn't transitioned yet — wait for it to settle
  if (authService.isAuthenticated()) {
    const settled = await identityService.waitForAuthenticatedState(IDENTITY_SETTLE_TIMEOUT);
    if (settled) {
      return true;
    }
  }

  // Redirect to login with return URL
  return router.createUrlTree([LOGIN_ROUTE], {
    queryParams: { returnUrl: state.url },
  });
};

/**
 * Guard that allows session OR network authentication.
 *
 * Use for pages that work with session but offer enhanced features
 * for authenticated users.
 */
// eslint-disable-next-line sonarjs/function-return-type -- Angular guard specification requires boolean | UrlTree
export const sessionOrAuthGuard: CanActivateFn = (): boolean | UrlTree => {
  const identityService = inject(IdentityService);
  const sessionHumanService = inject(SessionHumanService);
  const router = inject(Router);

  // Allow if authenticated via network (hosted or steward)
  const mode = identityService.mode();
  if (isNetworkMode(mode) && identityService.isAuthenticated()) {
    return true;
  }

  // Allow if has session
  if (sessionHumanService.hasSession()) {
    return true;
  }

  // Neither - redirect to login
  return router.createUrlTree([LOGIN_ROUTE]);
};

/**
 * Guard that requires a specific attestation.
 *
 * Usage:
 *   {
 *     path: 'create-content',
 *     canActivate: [attestationGuard('content-creator')],
 *     component: CreateContentComponent
 *   }
 */
export function attestationGuard(requiredAttestation: string): CanActivateFn {
  // eslint-disable-next-line sonarjs/function-return-type -- Angular guard specification requires boolean | UrlTree
  return (route, state): boolean | UrlTree => {
    const identityService = inject(IdentityService);
    const router = inject(Router);

    // Must be network authenticated (hosted or steward)
    const mode = identityService.mode();
    if (!isNetworkMode(mode) || !identityService.isAuthenticated()) {
      return router.createUrlTree([LOGIN_ROUTE], {
        queryParams: { returnUrl: state.url },
      });
    }

    // Check for required attestation
    const attestations = identityService.attestations();
    if (attestations?.includes(requiredAttestation)) {
      return true;
    }

    // Missing attestation - redirect to access denied
    return router.createUrlTree(['/access-denied'], {
      queryParams: {
        required: requiredAttestation,
        returnUrl: state.url,
      },
    });
  };
}
