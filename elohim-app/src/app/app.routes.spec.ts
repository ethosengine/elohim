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

  it('should have a lamad lazy loaded route', () => {
    const lamadRoute = routes.find(r => r.path === 'lamad');
    expect(lamadRoute).toBeDefined();
    expect(lamadRoute?.loadChildren).toBeDefined();
  });

  it('should have correct number of routes', () => {
    expect(routes.length).toBe(8); // home, lamad, community, shefa, identity, doorway, auth/callback, and 404 catch-all
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

  it('should have a 404 catch-all route as last route', () => {
    const lastRoute = routes[routes.length - 1];
    expect(lastRoute.path).toBe('**');
    expect(lastRoute.loadComponent).toBeDefined();
  });
});
