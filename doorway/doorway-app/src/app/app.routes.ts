import { Routes } from '@angular/router';

import { authGuard } from './core/guards/auth.guard';

/** The device approval page and its element registration, loaded together. */
const loadDeviceConsent = () =>
  Promise.all([
    import('./elements/register-identity-elements'),
    import('./components/consent/device-consent.component'),
  ]).then(([, m]) => m.DeviceConsentComponent);

/**
 * Development-only routes. `ngDevMode` is replaced with `false` in optimized
 * (production) builds, so this branch — the route AND its lazy chunk — is
 * removed at build time: no sample-data page exists on a live doorway.
 */
const devOnlyRoutes: Routes =
  typeof ngDevMode === 'undefined' || ngDevMode
    ? [
        {
          path: 'consent/device/preview',
          loadComponent: () =>
            Promise.all([
              import('./elements/register-identity-elements'),
              import('./components/consent/device-consent-preview.component'),
            ]).then(([, m]) => m.DeviceConsentPreviewComponent),
          title: 'Approve a device (preview)',
        },
      ]
    : [];

export const routes: Routes = [
  {
    path: '',
    loadComponent: () =>
      import('./components/landing/doorway-landing.component').then(m => m.DoorwayLandingComponent),
    title: 'Welcome',
  },
  {
    path: 'dashboard',
    loadComponent: () =>
      import('./components/dashboard/doorway-dashboard.component').then(
        m => m.DoorwayDashboardComponent
      ),
    title: 'Operator Dashboard',
    canActivate: [authGuard],
  },
  {
    path: 'login',
    loadComponent: () =>
      import('./components/login/threshold-login.component').then(m => m.ThresholdLoginComponent),
    title: 'Sign In',
  },
  {
    path: 'register',
    loadComponent: () =>
      import('./components/register/threshold-register.component').then(
        m => m.ThresholdRegisterComponent
      ),
    title: 'Create Account',
  },
  {
    path: 'consent/device',
    loadComponent: loadDeviceConsent,
    title: 'Approve a device',
    canActivate: [authGuard],
  },
  ...devOnlyRoutes,
  {
    path: 'doorways',
    loadComponent: () =>
      import('./components/doorway-browser/doorway-browser.component').then(
        m => m.DoorwayBrowserComponent
      ),
    title: 'Select Doorway',
  },
  {
    path: 'account',
    loadComponent: () =>
      import('./components/account/doorway-account.component').then(m => m.DoorwayAccountComponent),
    title: 'My Account',
    canActivate: [authGuard],
  },
  {
    path: '**',
    redirectTo: '',
  },
];
