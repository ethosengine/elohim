import { Routes } from '@angular/router';

export const DOCS_ROUTES: Routes = [
  {
    path: '',
    loadComponent: () =>
      import('./components/docs-layout/docs-layout.component').then(
        m => m.DocsLayoutComponent
      ),
    children: [
      {
        path: '',
        loadComponent: () =>
          import('./components/docs-home/docs-home.component').then(
            m => m.DocsHomeComponent
          )
      },
      {
        path: 'epic/:id',
        loadComponent: () =>
          import('./components/epic-viewer/epic-viewer.component').then(
            m => m.EpicViewerComponent
          )
      },
      {
        path: 'feature/:id',
        loadComponent: () =>
          import('./components/feature-viewer/feature-viewer.component').then(
            m => m.FeatureViewerComponent
          )
      },
      {
        path: 'scenario/:id',
        loadComponent: () =>
          import('./components/scenario-detail/scenario-detail.component').then(
            m => m.ScenarioDetailComponent
          )
      },
      {
        path: 'graph',
        loadComponent: () =>
          import('./components/graph-visualizer/graph-visualizer.component').then(
            m => m.GraphVisualizerComponent
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
