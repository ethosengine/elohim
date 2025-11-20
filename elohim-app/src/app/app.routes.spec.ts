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
    expect(routes.length).toBe(2);
  });
});
