/**
 * Account Routes — lazy-loaded under /account.
 *
 * All routes are protected by accountGuard which:
 *   1. Exchanges any ?session_token handoff from doorway
 *   2. Confirms authentication
 *   3. Pre-loads AccountView
 *
 * Child components are created in Task 18 (account-shell + panes).
 * Route structure is defined here so app.routes.ts can reference ACCOUNT_ROUTES
 * and the lazy chunk boundary is established in this task.
 */

import { Routes } from '@angular/router';

import { accountGuard } from './guards/account-guard';

export const ACCOUNT_ROUTES: Routes = [
  {
    path: '',
    canActivate: [accountGuard],
    loadComponent: async () =>
      import('./components/account-shell/account-shell.component').then(
        m => m.AccountShellComponent
      ),
    children: [
      { path: '', redirectTo: 'security', pathMatch: 'full' },
      {
        path: 'security',
        loadComponent: async () =>
          import('./components/security-signin-pane/security-signin-pane.component').then(
            m => m.SecuritySigninPaneComponent
          ),
      },
      {
        path: 'personal-info',
        loadComponent: async () =>
          import('./components/personal-info-pane/personal-info-pane.component').then(
            m => m.PersonalInfoPaneComponent
          ),
      },
      {
        path: 'data-privacy',
        loadComponent: async () =>
          import('./components/data-privacy-pane/data-privacy-pane.component').then(
            m => m.DataPrivacyPaneComponent
          ),
      },
      {
        path: 'people-sharing',
        loadComponent: async () =>
          import('./components/people-sharing-pane/people-sharing-pane.component').then(
            m => m.PeopleSharingPaneComponent
          ),
      },
      {
        path: 'third-party-apps',
        loadComponent: async () =>
          import('./components/third-party-apps-pane/third-party-apps-pane.component').then(
            m => m.ThirdPartyAppsPaneComponent
          ),
      },
    ],
  },
];
