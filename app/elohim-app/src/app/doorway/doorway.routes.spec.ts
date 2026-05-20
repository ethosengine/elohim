import { Routes } from '@angular/router';

import { DOORWAY_ROUTES } from './doorway.routes';

describe('DOORWAY_ROUTES', () => {
  it('should be a non-empty route array', () => {
    expect(Array.isArray(DOORWAY_ROUTES)).toBe(true);
    expect(DOORWAY_ROUTES.length).toBeGreaterThan(0);
  });

  describe('layout shell route', () => {
    const shell = DOORWAY_ROUTES[0];

    it('should mount DoorwayLayoutComponent at the root path', () => {
      expect(shell.path).toBe('');
    });

    it('should use loadComponent to lazy-load DoorwayLayoutComponent', async () => {
      expect(typeof shell.loadComponent).toBe('function');
      const mod = await (shell.loadComponent as () => Promise<unknown>)();
      // Class name may be minified/suffixed in test transforms; check it starts correctly.
      expect((mod as { name: string }).name).toContain('DoorwayLayoutComponent');
    });

    it('should declare children', () => {
      expect(Array.isArray(shell.children)).toBe(true);
      expect((shell.children as Routes).length).toBeGreaterThan(0);
    });

    it('should not carry data itself (title lives on children)', () => {
      expect(shell.data).toBeUndefined();
    });
  });

  describe('children', () => {
    let children: Routes;

    beforeEach(() => {
      children = DOORWAY_ROUTES[0].children as Routes;
    });

    describe('dashboard route (path: "")', () => {
      it('should exist with pathMatch full', () => {
        const route = children.find(r => r.path === '' && r.pathMatch === 'full');
        expect(route).toBeDefined();
      });

      it('should lazy-load DoorwayDashboardComponent', async () => {
        const route = children.find(r => r.path === '' && r.pathMatch === 'full')!;
        const mod = await (route.loadComponent as () => Promise<unknown>)();
        expect((mod as { name: string }).name).toContain('DoorwayDashboardComponent');
      });

      it('should carry a title', () => {
        const route = children.find(r => r.path === '' && r.pathMatch === 'full')!;
        expect(route.data?.['title']).toBeTruthy();
      });

      it('should carry seo metadata', () => {
        const route = children.find(r => r.path === '' && r.pathMatch === 'full')!;
        expect(route.data?.['seo']).toBeDefined();
        expect(route.data?.['seo'].title).toBeTruthy();
        expect(route.data?.['seo'].description).toBeTruthy();
      });
    });

    describe('elohim backend route (path: "elohim")', () => {
      it('should exist', () => {
        const route = children.find(r => r.path === 'elohim');
        expect(route).toBeDefined();
      });

      it('should lazy-load ElohimConfigComponent', async () => {
        const route = children.find(r => r.path === 'elohim')!;
        const mod = await (route.loadComponent as () => Promise<unknown>)();
        expect((mod as { name: string }).name).toContain('ElohimConfigComponent');
      });

      it('should carry a title', () => {
        const route = children.find(r => r.path === 'elohim')!;
        expect(route.data?.['title']).toBeTruthy();
      });
    });

    describe('config route (path: "config")', () => {
      it('should exist', () => {
        const route = children.find(r => r.path === 'config');
        expect(route).toBeDefined();
      });

      it('should lazy-load DoorwayDashboardComponent', async () => {
        const route = children.find(r => r.path === 'config')!;
        const mod = await (route.loadComponent as () => Promise<unknown>)();
        expect((mod as { name: string }).name).toContain('DoorwayDashboardComponent');
      });

      it('should carry a title', () => {
        const route = children.find(r => r.path === 'config')!;
        expect(route.data?.['title']).toBeTruthy();
      });

      it('should carry seo metadata', () => {
        const route = children.find(r => r.path === 'config')!;
        expect(route.data?.['seo']).toBeDefined();
        expect(route.data?.['seo'].title).toBeTruthy();
        expect(route.data?.['seo'].description).toBeTruthy();
      });
    });

    it('should not expose a bare "" child without pathMatch full', () => {
      const ambiguousEmpty = children.filter(r => r.path === '' && r.pathMatch !== 'full');
      expect(ambiguousEmpty.length).toBe(0);
    });

    it('should cover exactly the three expected paths', () => {
      const paths = (children as Routes).map(r => r.path).sort();
      expect(paths).toEqual(['', 'config', 'elohim']);
    });
  });
});
