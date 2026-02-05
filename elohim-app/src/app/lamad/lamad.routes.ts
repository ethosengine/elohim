import { Routes } from '@angular/router';

// @coverage: 4.8% (2026-02-05)

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
 * SEO Notes:
 * - Static routes have title/description in route data
 * - Dynamic routes (path/:pathId, resource/:resourceId) update SEO via component
 */
export const LAMAD_ROUTES: Routes = [
  {
    path: '',
    loadComponent: async () =>
      import('./components/lamad-layout/lamad-layout.component').then(m => m.LamadLayoutComponent),
    children: [
      // ============================================
      // PATH-CENTRIC ROUTES (Primary User Experience)
      // ============================================

      // Path step navigation - the main learning interface
      // SEO: Dynamic title set by PathNavigatorComponent
      {
        path: 'path/:pathId/step/:stepIndex',
        loadComponent: async () =>
          import('./components/path-navigator/path-navigator.component').then(
            m => m.PathNavigatorComponent
          ),
      },

      // Path overview/landing page
      // SEO: Dynamic title set by PathOverviewComponent
      {
        path: 'path/:pathId',
        loadComponent: async () =>
          import('./components/path-overview/path-overview.component').then(
            m => m.PathOverviewComponent
          ),
      },

      // ============================================
      // DIRECT RESOURCE ACCESS (Secondary)
      // ============================================

      // SEO: Dynamic title set by ContentViewerComponent
      {
        path: 'resource/:resourceId',
        loadComponent: async () =>
          import('./components/content-viewer/content-viewer.component').then(
            m => m.ContentViewerComponent
          ),
      },

      // Content Editor - edit existing content
      {
        path: 'resource/:resourceId/edit',
        loadComponent: async () =>
          import('./components/content-editor-page/content-editor-page.component').then(
            m => m.ContentEditorPageComponent
          ),
        data: {
          title: 'Edit Content',
          seo: {
            title: 'Edit Content',
            description: 'Edit content in the knowledge graph.',
            openGraph: { ogType: 'website' },
          },
        },
      },

      // ============================================
      // AGENT CONTEXT
      // ============================================

      {
        path: 'me',
        loadComponent: async () =>
          import('./components/learner-dashboard/learner-dashboard.component').then(
            m => m.LearnerDashboardComponent
          ),
        data: {
          title: 'My Learning Dashboard',
          seo: {
            title: 'My Learning Dashboard',
            description:
              'Track your learning progress, view completed paths, and continue your journey.',
            openGraph: { ogType: 'website' },
          },
        },
      },

      // Profile page - session human profile management
      // SEO: Dynamic title set by ProfilePageComponent
      {
        path: 'human',
        loadComponent: async () =>
          import('./components/profile-page/profile-page.component').then(
            m => m.ProfilePageComponent
          ),
        data: {
          title: 'My Profile',
          seo: {
            title: 'My Profile',
            description: 'Manage your profile and preferences.',
            openGraph: { ogType: 'profile' },
          },
        },
      },

      // ============================================
      // EXPLORATION & RESEARCH (Tertiary)
      // ============================================

      // Graph explorer - visual knowledge map (Khan Academy style)
      {
        path: 'explore',
        loadComponent: async () =>
          import('./components/graph-explorer/graph-explorer.component').then(
            m => m.GraphExplorerComponent
          ),
        data: {
          title: 'Knowledge Explorer',
          seo: {
            title: 'Knowledge Explorer',
            description:
              'Explore the knowledge graph visually. Discover connections between concepts and find your learning path.',
            openGraph: { ogType: 'website' },
          },
        },
      },

      // Meaning map - list/card view alternative
      {
        path: 'map',
        loadComponent: async () =>
          import('./components/meaning-map/meaning-map.component').then(m => m.MeaningMapComponent),
        data: {
          title: 'Meaning Map',
          seo: {
            title: 'Meaning Map',
            description: 'Browse and discover learning resources organized by meaning and purpose.',
            openGraph: { ogType: 'website' },
          },
        },
      },

      {
        path: 'search',
        loadComponent: async () =>
          import('./components/search/search.component').then(m => m.SearchComponent),
        data: {
          title: 'Search',
          seo: {
            title: 'Search',
            description: 'Search for learning paths, concepts, and resources.',
            openGraph: { ogType: 'website' },
          },
        },
      },

      // ============================================
      // HOME (Path-centric landing)
      // ============================================

      {
        path: '',
        loadComponent: async () =>
          import('./components/lamad-home/lamad-home.component').then(m => m.LamadHomeComponent),
        data: {
          title: 'Lamad',
          seo: {
            title: 'Lamad - Learning Paths',
            description:
              'Discover curated learning paths for human flourishing. Start your journey today.',
            openGraph: { ogType: 'website' },
          },
        },
      },

      // ============================================
      // 404 - Lamad-specific not found (must be last)
      // ============================================

      {
        path: '**',
        loadComponent: async () =>
          import('./components/not-found/lamad-not-found.component').then(
            m => m.LamadNotFoundComponent
          ),
      },
    ],
  },
];
