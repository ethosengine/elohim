/**
 * Lamad-local identity guard.
 *
 * P+inversion disposition (Slice 2.3 residual): The concrete identityGuard in
 * @app/imagodei/guards/identity.guard depends on AuthService and IdentityService
 * directly (both out-of-scope imagodei services). This lamad-local guard delegates
 * to the LAMAD_IDENTITY token so lamad.routes.ts has no imagodei cross-pillar import.
 *
 * Registered at composition root (app.config.ts):
 *   { provide: LAMAD_IDENTITY, useExisting: IdentityService }
 *
 * This guard intentionally does NOT replicate the AuthService settle-wait logic from
 * the imagodei guard — that logic belongs in the identity layer. The LAMAD_IDENTITY
 * token exposes isAuthenticated() which already reflects the settled state.
 */

import { inject } from '@angular/core';
import { Router, type CanActivateFn, type UrlTree } from '@angular/router';

import { LAMAD_IDENTITY } from '../interfaces/cross-pillar.interface';
import { isNetworkMode } from '../utils/identity.utils';

/** Login route for unauthenticated users. */
const LOGIN_ROUTE = '/identity/login';

/**
 * Lamad route guard: requires network authentication (hosted or steward mode).
 *
 * Delegates identity state to the LAMAD_IDENTITY token — the concrete binding
 * (IdentityService) is registered in app.config.ts.
 */
export const lamadIdentityGuard: CanActivateFn = (route, state): boolean | UrlTree => {
  const identityService = inject(LAMAD_IDENTITY);
  const router = inject(Router);

  const mode = identityService.mode();
  if (isNetworkMode(mode) && identityService.isAuthenticated()) {
    return true;
  }

  return router.createUrlTree([LOGIN_ROUTE], {
    queryParams: { returnUrl: state.url },
  });
};
