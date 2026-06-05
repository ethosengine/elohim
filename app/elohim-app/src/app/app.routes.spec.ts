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

  // TODO(#12-6 Slice 2): the shell has no top-level `path`/`path/:id` route —
  // `path`-shaped EPR routes minted by eprToRoute()/resolveInContext() (e.g.
  // `['/path', id]`, spec §12.3) render only inside the lamad bundle (base href
  // `/lamad/`). From the shell they hit the `**` catch-all. Cross-bundle anchors
  // are now handled by the epr-link interceptor + EprNavService (2026-06-05
  // omnibar-consolidation spec §4) pending the Slice-2 /epr resolver; NOT a
  // regression: the prior `/lamad/path` literal was equally unreachable here.
  // This canary pins the absence so a stray shell `path` route (or a regression
  // that silently makes these links "work" by accident) is caught.
  it('should NOT have a top-level path route (lamad-bundle-only until Slice 2 /epr resolver)', () => {
    const pathRoute = routes.find(r => r.path === 'path' || r.path?.startsWith('path/'));
    expect(pathRoute).toBeUndefined();
  });

  it('should have correct number of routes', () => {
    // home, community, shefa, identity, account, doorway, avodah,
    // auth/callback, deliver/:slug, resource/:resourceId, map, resolve,
    // and 404 catch-all
    expect(routes.length).toBe(13);
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
