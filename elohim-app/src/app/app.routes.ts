import { Routes } from '@angular/router';

export const routes: Routes = [
  {
    path: '',
    loadComponent: () =>
      import('./components/home/home.component').then(m => m.HomeComponent)
  },
  {
    path: 'docs',
    loadChildren: () =>
      import('./docs/docs.routes').then(m => m.DOCS_ROUTES)
  }
];
