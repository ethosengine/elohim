import { Routes } from '@angular/router';

/**
 * Shefa routing - Economy context app
 *
 * Routes:
 * - /shefa - Economy home (landing page)
 *
 * Future routes:
 * - /shefa/human - Economy-specific profile settings
 * - /shefa/flows - Value flow dashboard
 * - /shefa/reports - ValueFlows reports
 */
export const SHEFA_ROUTES: Routes = [
  {
    path: '',
    loadComponent: () =>
      import('./components/shefa-layout/shefa-layout.component').then(
        m => m.ShefaLayoutComponent
      ),
    children: [
      {
        path: '',
        loadComponent: () =>
          import('./components/shefa-home/shefa-home.component').then(
            m => m.ShefaHomeComponent
          ),
        data: {
          title: 'Shefa - Economics of Human Flourishing',
          seo: {
            title: 'Shefa Economy',
            description: 'Economic coordination layer implementing ValueFlows patterns for multi-dimensional value tracking.',
          }
        }
      }
    ]
  }
];
