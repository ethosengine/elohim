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

import { isNetworkMode } from '../models/identity.model';
import { IdentityService } from '../services/identity.service';
import { SessionHumanService } from '../services/session-human.service';

/**
 * Guard that requires network authentication.
 *
 * Redirects to /register if not authenticated via network.
 * Passes return URL as query parameter for post-auth redirect.
 */
export const identityGuard: CanActivateFn = (route, state): boolean | UrlTree => {
  const identityService = inject(IdentityService);
  const router = inject(Router);

  // Check if authenticated via network (hosted or steward)
  const mode = identityService.mode();
  if (isNetworkMode(mode) && identityService.isAuthenticated()) {
    return true;
  }

  // Redirect to register with return URL
  return router.createUrlTree(['/identity/register'], {
    queryParams: { returnUrl: state.url },
  });
};

/**
 * Guard that allows session OR network authentication.
 *
 * Use for pages that work with session but offer enhanced features
 * for authenticated users.
 */
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

  // Neither - redirect to register
  return router.createUrlTree(['/identity/register']);
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
  return (route, state): boolean | UrlTree => {
    const identityService = inject(IdentityService);
    const router = inject(Router);

    // Must be network authenticated (hosted or steward)
    const mode = identityService.mode();
    if (!isNetworkMode(mode) || !identityService.isAuthenticated()) {
      return router.createUrlTree(['/identity/register'], {
        queryParams: { returnUrl: state.url },
      });
    }

    // Check for required attestation
    const attestations = identityService.attestations();
    if (attestations.includes(requiredAttestation)) {
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
