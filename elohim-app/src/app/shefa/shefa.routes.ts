import { Routes } from '@angular/router';

/**
 * Shefa routing - Economy context app
 *
 * Routes:
 * - /shefa - Economy home (landing page)
 * - /shefa/dashboard - Network health and custodian metrics (operator view)
 *
 * Future routes:
 * - /shefa/human - Economy-specific profile settings
 * - /shefa/flows - Value flow dashboard
 * - /shefa/reports - ValueFlows reports
 */
export const SHEFA_ROUTES: Routes = [
  {
    path: '',
    loadComponent: async () =>
      import('./components/shefa-layout/shefa-layout.component').then(m => m.ShefaLayoutComponent),
    children: [
      {
        path: '',
        loadComponent: async () =>
          import('./components/shefa-home/shefa-home.component').then(m => m.ShefaHomeComponent),
        data: {
          title: 'Shefa - Economics of Human Flourishing',
          seo: {
            title: 'Shefa Economy',
            description:
              'Economic coordination layer implementing ValueFlows patterns for multi-dimensional value tracking.',
          },
        },
      },
      {
        path: 'dashboard',
        loadComponent: async () =>
          import('../elohim/components/shefa-dashboard/shefa-dashboard.component').then(
            m => m.ShefaDashboardComponent
          ),
        data: {
          title: 'Shefa Dashboard - Network Metrics',
          seo: {
            title: 'Shefa Dashboard',
            description: 'Network health overview and custodian performance metrics for operators.',
          },
        },
      },
    ],
  },
];
