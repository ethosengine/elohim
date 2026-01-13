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
 * - /identity/stewardship/capabilities - My capabilities view (requires auth)
 * - /identity/stewardship/policy/:subjectId - Policy console for subject (requires auth)
 * - /identity/stewardship/appeal/:grantId - Appeal a stewardship grant (requires auth)
 * - /identity/stewardship/intervention - Community intervention (requires auth)
 * - /identity/stewardship/intervention/initiate/:subjectId - Initiate intervention (requires auth)
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
    children: [
      {
        path: '',
        loadComponent: () =>
          import('./components/stewardship-dashboard/stewardship-dashboard.component').then(m => m.StewardshipDashboardComponent),
      },
      {
        path: 'capabilities',
        loadComponent: () =>
          import('./components/capabilities-dashboard/capabilities-dashboard.component').then(m => m.CapabilitiesDashboardComponent),
      },
      {
        path: 'policy/:subjectId',
        loadComponent: () =>
          import('./components/policy-console/policy-console.component').then(m => m.PolicyConsoleComponent),
      },
      {
        path: 'appeal/:grantId',
        loadComponent: () =>
          import('./components/appeal-wizard/appeal-wizard.component').then(m => m.AppealWizardComponent),
      },
      {
        path: 'intervention',
        loadComponent: () =>
          import('./components/community-intervention/community-intervention.component').then(m => m.CommunityInterventionComponent),
      },
      {
        path: 'intervention/view/:interventionId',
        loadComponent: () =>
          import('./components/community-intervention/community-intervention.component').then(m => m.CommunityInterventionComponent),
      },
      {
        path: 'intervention/initiate/:subjectId',
        loadComponent: () =>
          import('./components/community-intervention/community-intervention.component').then(m => m.CommunityInterventionComponent),
      },
    ],
  },
  {
    path: '',
    redirectTo: 'register',
    pathMatch: 'full',
  },
];
