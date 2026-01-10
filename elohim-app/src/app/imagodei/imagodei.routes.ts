/**
 * Imagodei Routes - Identity and presence management routes.
 *
 * Routes:
 * - /identity/login - Hosted human login
 * - /identity/register - New user registration
 * - /identity/profile - View/edit user profile (requires auth)
 * - /identity/presences - Contributor presence list (requires auth)
 * - /identity/presences/create - Create new contributor presence (requires auth)
 * - /identity/stewardship - Stewardship dashboard (requires auth)
 */

import { Routes } from '@angular/router';
import { identityGuard } from './guards/identity.guard';

export const IMAGODEI_ROUTES: Routes = [
  {
    path: 'login',
    loadComponent: () =>
      import('./components/login/login.component').then(m => m.LoginComponent),
  },
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
    path: 'presences/create',
    canActivate: [identityGuard],
    loadComponent: () =>
      import('./components/create-presence/create-presence.component').then(m => m.CreatePresenceComponent),
  },
  {
    path: 'stewardship',
    canActivate: [identityGuard],
    loadComponent: () =>
      import('./components/stewardship-dashboard/stewardship-dashboard.component').then(m => m.StewardshipDashboardComponent),
  },
  {
    path: '',
    redirectTo: 'register',
    pathMatch: 'full',
  },
];
