import { Routes } from '@angular/router';

export const routes: Routes = [
  {
    path: '',
    loadComponent: () =>
      import('./components/landing/doorway-landing.component').then(
        m => m.DoorwayLandingComponent
      ),
    title: 'Welcome',
  },
  {
    path: 'dashboard',
    loadComponent: () =>
      import('./components/dashboard/doorway-dashboard.component').then(
        m => m.DoorwayDashboardComponent
      ),
    title: 'Operator Dashboard',
  },
  {
    path: 'login',
    loadComponent: () =>
      import('./components/login/threshold-login.component').then(
        m => m.ThresholdLoginComponent
      ),
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
      import('./components/account/doorway-account.component').then(
        m => m.DoorwayAccountComponent
      ),
    title: 'My Account',
  },
  {
    path: '**',
    redirectTo: '',
  },
];
