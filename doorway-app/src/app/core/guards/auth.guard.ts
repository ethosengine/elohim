/**
 * Auth Guard
 *
 * Redirects unauthenticated users to /login for protected routes.
 */

import { inject } from '@angular/core';
import { CanActivateFn, Router } from '@angular/router';

import { AuthStateService } from '../../services/auth-state.service';

export const authGuard: CanActivateFn = async () => {
  const authState = inject(AuthStateService);
  const router = inject(Router);

  // Wait for init to complete if still loading
  if (authState.isLoading()) {
    await authState.init();
  }

  if (authState.isAuthenticated()) {
    return true;
  }

  return router.createUrlTree(['/login']);
};
