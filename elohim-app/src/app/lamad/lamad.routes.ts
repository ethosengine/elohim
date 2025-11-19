import { Routes } from '@angular/router';

/**
 * Lamad routing strategy:
 *
 * Hierarchical path-based navigation:
 * - /lamad                              → Home (epics list)
 * - /lamad/:epic                        → Epic view with features pane
 * - /lamad/:epic/:feature               → Feature view with scenarios pane
 * - /lamad/:epic/:feature/:scenario     → Scenario view
 * - Can go deeper as domain defines
 *
 * Special routes:
 * - /lamad/map                          → Meaning map visualization
 * - /lamad/search                       → Search interface
 * - /lamad/content/:id                  → Direct content access (fallback)
 *
 * Query parameters for context:
 * - ?target=elohim-protocol             → Target subject for orientation
 * - ?attestation=civic-2                → Attestation journey tracking
 * - ?step=3                             → Step in suggested path
 * - ?from=node-id                       → Source node (breadcrumb)
 * - ?depth=2                            → Graph traversal depth
 */
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
        path: 'search',
        loadComponent: () =>
          import('./components/search/search.component').then(
            m => m.SearchComponent
          )
      },
      {
        path: 'content/:id',
        loadComponent: () =>
          import('./components/content-viewer/content-viewer.component').then(
            m => m.ContentViewerComponent
          )
      },
      // Hierarchical path navigation (catch-all for graph traversal)
      // Matches: /:epic, /:epic/:feature, /:epic/:feature/:scenario, etc.
      {
        path: '**',
        loadComponent: () =>
          import('./components/content-viewer/content-viewer.component').then(
            m => m.ContentViewerComponent
          )
      }
    ]
  }
];
