/**
 * Account Guard — protects all /account/* routes.
 *
 * Activation sequence:
 * 1. Consume ?session_token=xxx (doorway handoff) if present — delegates to HandoffService.
 * 2. Confirm the user is authenticated — delegates to AuthService.
 * 3. Refresh AccountView so child components render without waiting — delegates to AccountService.
 *
 * On any failure the guard redirects to /identity/login.
 */

import { inject } from '@angular/core';
import { type CanActivateFn, Router } from '@angular/router';

import { AuthService } from '@app/imagodei';

import { AccountService } from '../services/account.service';
import { HandoffService } from '../services/handoff.service';

export const accountGuard: CanActivateFn = async route => {
  const auth = inject(AuthService);
  const handoff = inject(HandoffService);
  const account = inject(AccountService);
  const router = inject(Router);

  // ---------------------------------------------------------------------------
  // Step 1: Consume handoff token if present (doorway → browser session exchange)
  // ---------------------------------------------------------------------------
  const sessionToken = route.queryParamMap.get('session_token');
  if (sessionToken) {
    // doorway_url is appended by the doorway redirect so the exchange call goes
    // to the correct host. Falls back to empty string (exchange will fail and
    // the guard will redirect to login, which is the safe path).
    const doorwayUrl = route.queryParamMap.get('doorway_url') ?? '';
    const exchanged = await handoff.consumeHandoffToken(sessionToken, doorwayUrl);
    if (!exchanged) {
      await router.navigate(['/identity/login']);
      return false;
    }
  }

  // ---------------------------------------------------------------------------
  // Step 2: Confirm authenticated
  // ---------------------------------------------------------------------------
  if (!auth.isAuthenticated()) {
    await router.navigate(['/identity/login']);
    return false;
  }

  // ---------------------------------------------------------------------------
  // Step 3: Pre-load AccountView so child routes render immediately
  // ---------------------------------------------------------------------------
  await account.refresh();

  return true;
};
