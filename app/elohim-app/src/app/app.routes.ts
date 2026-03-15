import { Routes } from '@angular/router';

// @coverage: 8.3% (2026-03-03)

export const routes: Routes = [
  {
    path: '',
    loadComponent: async () =>
      import('./components/home/home.component').then(m => m.HomeComponent),
  },
  {
    path: 'lamad',
    loadChildren: async () => import('./lamad/lamad.routes').then(m => m.LAMAD_ROUTES),
  },
  {
    path: 'community',
    loadChildren: async () => import('./qahal/community.routes').then(m => m.COMMUNITY_ROUTES),
  },
  {
    path: 'shefa',
    loadChildren: async () => import('./shefa/shefa.routes').then(m => m.SHEFA_ROUTES),
  },
  {
    path: 'identity',
    loadChildren: async () => import('./imagodei/imagodei.routes').then(m => m.IMAGODEI_ROUTES),
  },
  {
    path: 'doorway',
    loadChildren: async () => import('./doorway/doorway.routes').then(m => m.DOORWAY_ROUTES),
  },
  {
    path: 'avodah',
    loadChildren: async () => import('./avodah/avodah.routes').then(m => m.AVODAH_ROUTES),
  },
  // OAuth callback route for doorway authentication
  {
    path: 'auth/callback',
    loadComponent: async () =>
      import('./imagodei/components/auth-callback/auth-callback.component').then(
        m => m.AuthCallbackComponent
      ),
    title: 'Signing In...',
  },
  // Cross-pillar resource viewer (ContentNodes are protocol primitives, not pillar-scoped)
  {
    path: 'resource/:resourceId',
    loadComponent: async () =>
      import('./lamad/components/content-viewer/content-viewer.component').then(
        m => m.ContentViewerComponent
      ),
  },
  // EPR protocol handler redirect (web+epr:// links from outside the app)
  {
    path: 'resolve',
    loadComponent: async () =>
      import('./elohim/components/epr-resolve-redirect/epr-resolve-redirect.component').then(
        m => m.EprResolveRedirectComponent
      ),
  },
  // 404 catch-all - must be last
  {
    path: '**',
    loadComponent: async () =>
      import('./components/not-found/not-found.component').then(m => m.NotFoundComponent),
  },
];
