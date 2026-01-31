import { Routes } from '@angular/router';

// @coverage: 10.0% (2026-02-04)

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
  // OAuth callback route for doorway authentication
  {
    path: 'auth/callback',
    loadComponent: async () =>
      import('./imagodei/components/auth-callback/auth-callback.component').then(
        m => m.AuthCallbackComponent
      ),
    title: 'Signing In...',
  },
  // 404 catch-all - must be last
  {
    path: '**',
    loadComponent: async () =>
      import('./components/not-found/not-found.component').then(m => m.NotFoundComponent),
  },
];
