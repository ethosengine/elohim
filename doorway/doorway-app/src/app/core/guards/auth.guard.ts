/**
 * Auth Guard
 *
 * Sends a signed-out person to /login for protected routes, carrying the page
 * they asked for as `returnUrl` so the login page can bring them back to it.
 * This is the app's one in-app return mechanism; OAuth returns to another app
 * ride the OAuth parameters instead (see threshold-login).
 */

import { inject } from '@angular/core';
import { CanActivateFn, Router, UrlTree } from '@angular/router';

import { AuthStateService } from '../../services/auth-state.service';

/** Query parameter the login page reads to return a person to where they were. */
export const RETURN_URL_PARAM = 'returnUrl';

/**
 * A return target the login page may navigate to: an absolute path inside this
 * app. Anything that could leave the app (a scheme, `//host`, `/\host`) is
 * refused, so a crafted login link cannot bounce a person elsewhere.
 */
export function safeReturnUrl(value: unknown): string | null {
  if (typeof value !== 'string' || !value.startsWith('/')) return null;
  if (value.startsWith('//') || value.startsWith('/\\')) return null;
  return value;
}

/** The login page, remembering where to come back to. */
export function loginRedirect(router: Router, returnUrl: string): UrlTree {
  const safe = safeReturnUrl(returnUrl);
  return router.createUrlTree(['/login'], {
    queryParams: safe ? { [RETURN_URL_PARAM]: safe } : {},
  });
}

export const authGuard: CanActivateFn = async (_route, state) => {
  const authState = inject(AuthStateService);
  const router = inject(Router);

  // Wait for init to complete if still loading
  if (authState.isLoading()) {
    await authState.init();
  }

  if (authState.isAuthenticated()) {
    return true;
  }

  return loginRedirect(router, state.url);
};
