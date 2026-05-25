import { LAMAD_ROUTES } from './lamad.routes';

describe('Lamad Routes', () => {
  it('should have routes defined', () => {
    expect(LAMAD_ROUTES).toBeDefined();
    expect(LAMAD_ROUTES.length).toBeGreaterThan(0);
  });

  it('should have a layout route', () => {
    const layoutRoute = LAMAD_ROUTES[0];
    expect(layoutRoute).toBeDefined();
    expect(layoutRoute.path).toBe('');
    expect(layoutRoute.loadComponent).toBeDefined();
  });

  it('should have child routes', () => {
    const layoutRoute = LAMAD_ROUTES[0];
    expect(layoutRoute.children).toBeDefined();
    expect(layoutRoute.children?.length).toBeGreaterThan(0);
  });

  it('should have home child route', () => {
    const children = LAMAD_ROUTES[0].children;
    const homeRoute = children?.find(r => r.path === '');
    expect(homeRoute).toBeDefined();
  });

  it('should have map child route', () => {
    const children = LAMAD_ROUTES[0].children;
    const mapRoute = children?.find(r => r.path === 'map');
    expect(mapRoute).toBeDefined();
  });

  it('should have resource viewer child route', () => {
    const children = LAMAD_ROUTES[0].children;
    const resourceRoute = children?.find(r => r.path === 'resource/:resourceId');
    expect(resourceRoute).toBeDefined();
  });

  it('should have search child route', () => {
    const children = LAMAD_ROUTES[0].children;
    const searchRoute = children?.find(r => r.path === 'search');
    expect(searchRoute).toBeDefined();
  });
});
