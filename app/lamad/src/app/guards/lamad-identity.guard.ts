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
import { type CanActivateFn } from '@angular/router';

import { LAMAD_EPR_NAV } from '../interfaces/cross-pillar.interface';
import { LAMAD_IDENTITY } from '../interfaces/cross-pillar.interface';
import { isNetworkMode } from '../utils/identity.utils';

/** Login route for unauthenticated users (shell mount — cross-bundle from lamad). */
const LOGIN_ROUTE = '/identity/login';

/**
 * Lamad route guard: requires network authentication (hosted or steward mode).
 *
 * Delegates identity state to the LAMAD_IDENTITY token — the concrete binding
 * (IdentityService) is registered in app.config.ts.
 */
export const lamadIdentityGuard: CanActivateFn = (_route, state): boolean => {
  const identityService = inject(LAMAD_IDENTITY);
  const eprNav = inject(LAMAD_EPR_NAV);

  const mode = identityService.mode();
  if (isNetworkMode(mode) && identityService.isAuthenticated()) {
    return true;
  }

  // /identity lives in the shell: full-load handoff with the PUBLIC return URL
  // (state.url is base-stripped — re-prefix the /lamad mount).
  const returnUrl = encodeURIComponent(`/lamad${state.url}`);
  eprNav.navigate(`${LOGIN_ROUTE}?returnUrl=${returnUrl}`);
  return false;
};
