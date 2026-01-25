import { Routes } from '@angular/router';

export const routes: Routes = [
  {
    path: '',
    loadComponent: () => import('./components/home/home.component').then(m => m.HomeComponent),
  },
  {
    path: 'lamad',
    loadChildren: () => import('./lamad/lamad.routes').then(m => m.LAMAD_ROUTES),
  },
  {
    path: 'community',
    loadChildren: () => import('./qahal/community.routes').then(m => m.COMMUNITY_ROUTES),
  },
  {
    path: 'shefa',
    loadChildren: () => import('./shefa/shefa.routes').then(m => m.SHEFA_ROUTES),
  },
  {
    path: 'identity',
    loadChildren: () => import('./imagodei/imagodei.routes').then(m => m.IMAGODEI_ROUTES),
  },
  {
    path: 'doorway',
    loadChildren: () => import('./doorway/doorway.routes').then(m => m.DOORWAY_ROUTES),
  },
  // OAuth callback route for doorway authentication
  {
    path: 'auth/callback',
    loadComponent: () =>
      import('./imagodei/components/auth-callback/auth-callback.component').then(
        m => m.AuthCallbackComponent
      ),
    title: 'Signing In...',
  },
  // 404 catch-all - must be last
  {
    path: '**',
    loadComponent: () =>
      import('./components/not-found/not-found.component').then(m => m.NotFoundComponent),
  },
];
