import { Routes } from '@angular/router';

export const routes: Routes = [
  {
    path: '',
    loadComponent: () =>
      import('./components/home/home.component').then(m => m.HomeComponent)
  },
  {
    path: 'lamad',
    loadChildren: () =>
      import('./lamad/lamad.routes').then(m => m.LAMAD_ROUTES)
  },
  // 404 catch-all - must be last
  {
    path: '**',
    loadComponent: () =>
      import('./components/not-found/not-found.component').then(m => m.NotFoundComponent)
  }
];
