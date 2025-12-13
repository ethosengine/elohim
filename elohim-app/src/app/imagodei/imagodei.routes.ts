/**
 * Imagodei Routes - Identity and presence management routes.
 *
 * Routes:
 * - /identity/register - New user registration
 * - /identity/profile - View/edit user profile (requires auth)
 * - /identity/presences - Contributor presence list (requires auth)
 */

import { Routes } from '@angular/router';
import { identityGuard } from './guards/identity.guard';

export const IMAGODEI_ROUTES: Routes = [
  {
    path: 'register',
    loadComponent: () =>
      import('./components/register/register.component').then(m => m.RegisterComponent),
  },
  {
    path: 'profile',
    canActivate: [identityGuard],
    loadComponent: () =>
      import('./components/profile/profile.component').then(m => m.ProfileComponent),
  },
  {
    path: 'presences',
    canActivate: [identityGuard],
    loadComponent: () =>
      import('./components/presence-list/presence-list.component').then(m => m.PresenceListComponent),
  },
  {
    path: '',
    redirectTo: 'register',
    pathMatch: 'full',
  },
];
