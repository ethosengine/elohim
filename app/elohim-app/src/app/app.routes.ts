import { Routes } from '@angular/router';

// @coverage: 8.3% (2026-03-03)

export const routes: Routes = [
  {
    path: '',
    loadComponent: async () =>
      import('./components/home/home.component').then(m => m.HomeComponent),
    data: { protocolContent: true, fallbackCid: 'elohim-host-landing' },
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
    path: 'account',
    loadChildren: async () => import('./account/account.routes').then(m => m.ACCOUNT_ROUTES),
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
  // Full-page content delivery with protocol omnibar (no app chrome)
  {
    path: 'deliver/:slug',
    loadComponent: async () =>
      import('./elohim/components/content-delivery/content-delivery.component').then(
        m => m.ContentDeliveryComponent
      ),
    data: {
      title: 'Content',
    },
  },
  // Cross-pillar resource viewer (ContentNodes are protocol primitives, not pillar-scoped).
  // Legacy surface — durable /resource URLs exist in the wild; new minting targets the
  // /epr/:resourceId universal address below (same component, §12.3).
  {
    path: 'resource/:resourceId',
    loadComponent: async () =>
      import('@app/lamad/components/content-viewer/content-viewer.component').then(
        m => m.ContentViewerComponent
      ),
    data: { protocolContent: true },
  },
  // Universal EPR address (§12.1) — the atom's own home, shell-owned (spec
  // 2026-09-02-epr-atom-home-shell-component-design). Reachable-but-unclaimed
  // atoms render here; the doorway serves this bundle for any /epr/* path.
  {
    path: 'epr/:resourceId',
    loadComponent: async () =>
      import('./elohim/components/epr-home/epr-home.component').then(m => m.EprHomeComponent),
    data: { protocolContent: true },
  },
  // Raw-node inspector (§12.6 Slice 0) — shows the EPR AS AN ATOM (its own
  // fields/provenance), distinct from the rich pillar viewer above. Doorway
  // serves the shell for /epr/{id}/raw (landed separately). Three segments, so
  // it cannot cross-match the two-segment epr/:resourceId route regardless of
  // order; the only ordering constraint is that BOTH stay above the ** catch-all
  // (they do). Same resourceId param name.
  {
    path: 'epr/:resourceId/raw',
    loadComponent: async () =>
      import('./elohim/components/epr-raw-node/epr-raw-node.component').then(
        m => m.EprRawNodeComponent
      ),
    data: { protocolContent: true },
    title: 'Raw node',
  },
  // Hidden-but-accessible debug surface (chrome://flags model). Always resolves
  // by URL; the nav entry is gated by DebugModeService. No guard — it reads only
  // already-public endpoints, so gating the route would protect nothing.
  {
    path: 'debug',
    loadComponent: async () =>
      import('./debug/debug-shell.component').then(m => m.DebugShellComponent),
    title: 'Protocol Debug',
  },
  // Dev-only doc-sync harness — un-tree-shakes ContentDocSyncService and is the
  // look-rail render target for the Automerge content-sync browser leg.
  // /dev/doc-sync?id=<contentId>. Reads only the already-public /sync surface.
  {
    path: 'dev/doc-sync',
    loadComponent: async () =>
      import('./elohim/components/dev/doc-sync-harness.component').then(
        m => m.DocSyncHarnessComponent
      ),
    title: 'Doc-sync harness',
  },
  // Spatial map — cross-pillar geospatial view (Places, resources, governance)
  {
    path: 'map',
    loadComponent: async () =>
      import('./elohim/components/spatial-map/spatial-map.component').then(
        m => m.SpatialMapComponent
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
