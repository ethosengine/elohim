import { routes } from './app.routes';

describe('App Routes', () => {
  it('should have routes defined', () => {
    expect(routes).toBeDefined();
    expect(routes.length).toBeGreaterThan(0);
  });

  it('should have a root path route', () => {
    const rootRoute = routes.find(r => r.path === '');
    expect(rootRoute).toBeDefined();
  });

  it('should NOT have a lamad route (lamad is now a standalone app — pillar split)', () => {
    const lamadRoute = routes.find(r => r.path === 'lamad');
    expect(lamadRoute).toBeUndefined();
  });

  // §12.6 Slice 2 (landed): the shell still has no top-level `path` route —
  // path-claimed EPRs render only inside the lamad bundle. Unclaimed refs now
  // mint the universal /epr/:resourceId route (BundleRouteContext, spec §12.3);
  // cross-bundle anchors ride the epr-link interceptor + EprNavService
  // (2026-06-05 omnibar-consolidation spec §4).
  // This canary pins the absence so a stray shell `path` route (or a regression
  // that silently makes these links "work" by accident) is caught.
  it('should NOT have a top-level path route (path is lamad-claimed)', () => {
    const pathRoute = routes.find(r => r.path === 'path' || r.path?.startsWith('path/'));
    expect(pathRoute).toBeUndefined();
  });

  it('should have correct number of routes', () => {
    // home, community, shefa, identity, account, doorway, avodah,
    // auth/callback, deliver/:slug, resource/:resourceId, epr/:resourceId,
    // epr/:resourceId/raw, debug, map, resolve, and 404 catch-all
    expect(routes.length).toBe(16);
  });

  it('should have the universal epr/:resourceId route (§12.6 Slice 2)', () => {
    const eprRoute = routes.find(r => r.path === 'epr/:resourceId');
    expect(eprRoute).toBeDefined();
    expect(eprRoute?.loadComponent).toBeDefined();
    expect(eprRoute?.data?.['protocolContent']).toBe(true);
  });

  it('should have the raw-node inspector route epr/:resourceId/raw (§12.6 Slice 0)', () => {
    const rawRoute = routes.find(r => r.path === 'epr/:resourceId/raw');
    expect(rawRoute).toBeDefined();
    expect(rawRoute?.loadComponent).toBeDefined();
    // Above the ** catch-all so the inspector path actually resolves.
    const catchAllIndex = routes.findIndex(r => r.path === '**');
    const rawIndex = routes.findIndex(r => r.path === 'epr/:resourceId/raw');
    expect(rawIndex).toBeLessThan(catchAllIndex);
  });

  it('should have an auth callback route for OAuth', () => {
    const authCallbackRoute = routes.find(r => r.path === 'auth/callback');
    expect(authCallbackRoute).toBeDefined();
    expect(authCallbackRoute?.loadComponent).toBeDefined();
  });

  it('should have a community lazy loaded route', () => {
    const communityRoute = routes.find(r => r.path === 'community');
    expect(communityRoute).toBeDefined();
    expect(communityRoute?.loadChildren).toBeDefined();
  });

  it('should have a shefa lazy loaded route', () => {
    const shefaRoute = routes.find(r => r.path === 'shefa');
    expect(shefaRoute).toBeDefined();
    expect(shefaRoute?.loadChildren).toBeDefined();
  });

  it('should have an identity lazy loaded route', () => {
    const identityRoute = routes.find(r => r.path === 'identity');
    expect(identityRoute).toBeDefined();
    expect(identityRoute?.loadChildren).toBeDefined();
  });

  it('should have a doorway lazy loaded route', () => {
    const doorwayRoute = routes.find(r => r.path === 'doorway');
    expect(doorwayRoute).toBeDefined();
    expect(doorwayRoute?.loadChildren).toBeDefined();
  });

  it('should have an EPR resolve redirect route', () => {
    const resolveRoute = routes.find(r => r.path === 'resolve');
    expect(resolveRoute).toBeDefined();
    expect(resolveRoute?.loadComponent).toBeDefined();
  });

  it('should have a 404 catch-all route as last route', () => {
    const lastRoute = routes.at(-1);
    expect(lastRoute).toBeDefined();
    expect(lastRoute?.path).toBe('**');
    expect(lastRoute?.loadComponent).toBeDefined();
  });
});
