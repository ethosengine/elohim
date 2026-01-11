import { Routes } from '@angular/router';

export const routes: Routes = [
  {
    path: '',
    loadComponent: () =>
      import('./components/dashboard/doorway-dashboard.component').then(
        m => m.DoorwayDashboardComponent
      ),
    title: 'Doorway Operator Dashboard',
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
    path: '**',
    redirectTo: '',
  },
];
