import { Routes } from '@angular/router';

/**
 * Doorway routing - Web hosting configuration for always-on nodes
 *
 * Routes:
 * - /doorway - Web hosting dashboard
 * - /doorway/config - SSL, domain, and proxy configuration
 *
 * Future routes:
 * - /doorway/ssl - SSL certificate management
 * - /doorway/domains - Custom domain configuration
 * - /doorway/logs - Access logs and diagnostics
 */
export const DOORWAY_ROUTES: Routes = [
  {
    path: '',
    loadComponent: () =>
      import('./components/doorway-dashboard/doorway-dashboard.component').then(
        m => m.DoorwayDashboardComponent
      ),
    data: {
      title: 'Doorway - Web Hosting',
      seo: {
        title: 'Doorway Web Hosting',
        description:
          'Configure web hosting, SSL, and domain settings for your always-on Holochain nodes.',
      },
    },
  },
  {
    path: 'config',
    loadComponent: () =>
      import('./components/doorway-dashboard/doorway-dashboard.component').then(
        m => m.DoorwayDashboardComponent
      ),
    data: {
      title: 'Doorway Configuration',
      seo: {
        title: 'Doorway Configuration',
        description: 'SSL certificates, custom domains, and reverse proxy settings.',
      },
    },
  },
];
