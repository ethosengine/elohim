import { TestBed } from '@angular/core/testing';
import { Router } from '@angular/router';

import { describe, it, expect, beforeEach, vi } from 'vitest';

import { EprNavService } from './epr-nav.service';
import { ProtocolRouteContextService } from './protocol-route-context.service';
import { SessionNavStackService } from './session-nav-stack.service';

describe('EprNavService', () => {
  let service: EprNavService;
  let router: Router;
  const navStack = { record: vi.fn() };
  const routeCtx = { cid: () => 'elohim-host-landing' };

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [
        EprNavService,
        { provide: SessionNavStackService, useValue: navStack },
        { provide: ProtocolRouteContextService, useValue: routeCtx },
        {
          provide: Router,
          useValue: {
            config: [
              { path: '' },
              { path: 'community' },
              { path: 'shefa' },
              { path: 'doorway' },
              { path: 'deliver/:slug' },
              { path: '**' },
            ],
            url: '/current',
            navigateByUrl: vi.fn().mockResolvedValue(true),
            createUrlTree: vi.fn((cmds: string[]) => ({ toString: () => cmds.join('/') })),
          },
        },
      ],
    });
    service = TestBed.inject(EprNavService);
    router = TestBed.inject(Router);
    navStack.record.mockClear();
  });

  it('owns top-level paths present in the router config (catch-all excluded)', () => {
    expect(service.ownsPath('/')).toBe(true);
    expect(service.ownsPath('/community')).toBe(true);
    expect(service.ownsPath('/deliver/some-slug')).toBe(true);
    expect(service.ownsPath('/lamad')).toBe(false);
    expect(service.ownsPath('/lamad/path/x')).toBe(false);
  });

  it('routes same-bundle paths through the Angular router', () => {
    service.navigate('/community');
    expect(router.navigateByUrl).toHaveBeenCalledWith('/community');
    expect(navStack.record).not.toHaveBeenCalled();
  });

  it('hands off cross-bundle paths: nav-stack record + full load', () => {
    const assign = vi.fn();
    (service as unknown as { assign: (h: string) => void }).assign = assign;
    service.navigate('/lamad/path/abc');
    expect(navStack.record).toHaveBeenCalledWith({
      url: '/current',
      cid: 'elohim-host-landing',
      label: document.title,
    });
    expect(assign).toHaveBeenCalledWith('/lamad/path/abc');
    expect(router.navigateByUrl).not.toHaveBeenCalled();
  });

  describe('ownsPath with a pathless layout root (pillar-bundle shape)', () => {
    beforeEach(() => {
      // lamad-shaped config: everything hangs off a path:'' layout root.
      (router as unknown as { config: unknown[] }).config = [
        {
          path: '',
          children: [
            { path: 'path/:pathId', children: [] },
            { path: 'explore', children: [] },
            { path: 'resource/:resourceId/edit', children: [] },
            { path: '**', children: [] },
          ],
        },
      ];
    });

    it('owns routes declared under the layout root', () => {
      expect(service.ownsPath('/path/foundations/step/0')).toBe(true);
      expect(service.ownsPath('/explore')).toBe(true);
      expect(service.ownsPath('/resource/abc/edit')).toBe(true);
    });

    it('does not own foreign top segments', () => {
      expect(service.ownsPath('/epr/abc')).toBe(false);
      expect(service.ownsPath('/identity/login')).toBe(false);
    });
  });
});
