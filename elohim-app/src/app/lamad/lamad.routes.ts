import { Routes } from '@angular/router';

/**
 * Lamad routing strategy (spec-compliant):
 *
 * Path-centric navigation (PRIMARY):
 * - /lamad                              → Home (path discovery, path-centric)
 * - /lamad/path/:pathId                 → Path overview/landing page
 * - /lamad/path/:pathId/step/:stepIndex → Step navigator (main learning UI)
 *
 * Direct resource access (SECONDARY):
 * - /lamad/resource/:resourceId         → Direct content viewing
 *
 * Agent context:
 * - /lamad/me                           → Learner dashboard
 *
 * Research/exploration (TERTIARY):
 * - /lamad/explore                      → Graph exploration (requires attestation)
 * - /lamad/map                          → Meaning map visualization
 * - /lamad/search                       → Search interface
 *
 * Legacy (deprecated):
 * - /lamad/content/:id                  → Old direct access (redirects to resource)
 */
export const LAMAD_ROUTES: Routes = [
  {
    path: '',
    loadComponent: () =>
      import('./components/lamad-layout/lamad-layout.component').then(
        m => m.LamadLayoutComponent
      ),
    children: [
      // ============================================
      // PATH-CENTRIC ROUTES (Primary User Experience)
      // ============================================

      // Path step navigation - the main learning interface
      {
        path: 'path/:pathId/step/:stepIndex',
        loadComponent: () =>
          import('./components/path-navigator/path-navigator.component').then(
            m => m.PathNavigatorComponent
          )
      },

      // Path overview/landing page
      {
        path: 'path/:pathId',
        loadComponent: () =>
          import('./components/path-overview/path-overview.component').then(
            m => m.PathOverviewComponent
          )
      },

      // ============================================
      // DIRECT RESOURCE ACCESS (Secondary)
      // ============================================

      {
        path: 'resource/:resourceId',
        loadComponent: () =>
          import('./components/content-viewer/content-viewer.component').then(
            m => m.ContentViewerComponent
          )
      },

      // ============================================
      // AGENT CONTEXT
      // ============================================

      {
        path: 'me',
        loadComponent: () =>
          import('./components/learner-dashboard/learner-dashboard.component').then(
            m => m.LearnerDashboardComponent
          )
      },

      // ============================================
      // EXPLORATION & RESEARCH (Tertiary)
      // ============================================

      // Graph explorer - visual knowledge map (Khan Academy style)
      {
        path: 'explore',
        loadComponent: () =>
          import('./components/graph-explorer/graph-explorer.component').then(
            m => m.GraphExplorerComponent
          )
      },

      // Meaning map - list/card view alternative
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

      // ============================================
      // LEGACY ROUTES (Deprecated, for backwards compat)
      // ============================================

      // Old direct content access - redirect to new pattern
      {
        path: 'content/:id',
        redirectTo: 'resource/:id',
        pathMatch: 'full'
      },

      // ============================================
      // HOME (Path-centric landing)
      // ============================================

      {
        path: '',
        loadComponent: () =>
          import('./components/lamad-home/lamad-home.component').then(
            m => m.LamadHomeComponent
          )
      }
    ]
  }
];
