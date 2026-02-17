/**
 * Identity Default Redirect Guard
 *
 * Auth-aware redirect for the /identity root path.
 * - Authenticated (network mode) -> /identity/profile
 * - Not authenticated -> /identity/login (federated identifier input)
 *
 * This guard always returns a UrlTree (never activates the route).
 */

import { inject } from '@angular/core';
import { Router, type CanActivateFn, type UrlTree } from '@angular/router';

import { isNetworkMode } from '../models/identity.model';
import { IdentityService } from '../services/identity.service';

/**
 * Guard that redirects /identity to the appropriate page based on auth state.
 */
export const identityDefaultRedirectGuard: CanActivateFn = (): UrlTree => {
  const identityService = inject(IdentityService);
  const router = inject(Router);

  const mode = identityService.mode();
  if (isNetworkMode(mode) && identityService.isAuthenticated()) {
    return router.createUrlTree(['/identity/profile']);
  }

  return router.createUrlTree(['/identity/login']);
};
