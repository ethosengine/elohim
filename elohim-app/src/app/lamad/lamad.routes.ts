import { Routes } from '@angular/router';

export const LAMAD_ROUTES: Routes = [
  {
    path: '',
    loadComponent: () =>
      import('./components/lamad-layout/lamad-layout.component').then(
        m => m.LamadLayoutComponent
      ),
    children: [
      {
        path: '',
        loadComponent: () =>
          import('./components/lamad-home/lamad-home.component').then(
            m => m.LamadHomeComponent
          )
      },
      {
        path: 'map',
        loadComponent: () =>
          import('./components/meaning-map/meaning-map.component').then(
            m => m.MeaningMapComponent
          )
      },
      {
        path: 'content/:id',
        loadComponent: () =>
          import('./components/content-viewer/content-viewer.component').then(
            m => m.ContentViewerComponent
          )
      },
      {
        path: 'search',
        loadComponent: () =>
          import('./components/search/search.component').then(
            m => m.SearchComponent
          )
      }
    ]
  }
];
