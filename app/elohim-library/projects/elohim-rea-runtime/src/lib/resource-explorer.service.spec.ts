/**
 * ResourceExplorerService Tests
 *
 * Migrated from @app/shefa/services/resource-explorer.service.spec.ts
 * as part of Wave 2 Slice 2.4 of the cross-pillar import cleanup sprint.
 */

import { TestBed } from '@angular/core/testing';

import { firstValueFrom, of } from 'rxjs';

import {
  ResourceExplorerService,
  CONTENT_DATA_SOURCE,
  LENS_REGISTRY,
  type ILensRegistry,
} from './resource-explorer.service';
import type { LensProvider } from './resource-explorer.model';

describe('ResourceExplorerService', () => {
  let service: ResourceExplorerService;
  let lensRegistry: ILensRegistry & { registeredProviders: Map<string, LensProvider> };
  let getAllContentTypesSpy: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    getAllContentTypesSpy = vi.fn().mockReturnValue(of(['concept', 'assessment', 'video']));

    // Build a simple mock lens registry that records registrations
    const registeredProviders = new Map<string, LensProvider>();
    lensRegistry = {
      registeredProviders,
      register: vi.fn((p: LensProvider) => { registeredProviders.set(p.type, p); }),
      getProvider: vi.fn((type: string) => registeredProviders.get(type)),
    };

    TestBed.configureTestingModule({
      providers: [
        {
          provide: LENS_REGISTRY,
          useValue: lensRegistry,
        },
        {
          provide: CONTENT_DATA_SOURCE,
          useValue: {
            searchContent: vi.fn().mockReturnValue(
              of([
                {
                  id: 'node-1',
                  title: 'Article One',
                  description: 'Desc',
                  contentType: 'concept',
                  tags: [],
                },
                {
                  id: 'node-2',
                  title: 'Quiz One',
                  description: 'Desc',
                  contentType: 'assessment',
                  tags: [],
                },
                {
                  id: 'node-3',
                  title: 'Video One',
                  description: 'Desc',
                  contentType: 'video',
                  tags: [],
                },
              ])
            ),
            getAllContentTypes: getAllContentTypesSpy,
          },
        },
      ],
    });

    service = TestBed.inject(ResourceExplorerService);
  });

  afterEach(() => {
    service.invalidateCache();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should self-register with the lens registry', () => {
    expect(lensRegistry.getProvider('folders')).toBe(service);
  });

  it('should have the correct lens metadata', () => {
    expect(service.type).toBe('folders');
    expect(service.label).toBe('Folders');
    expect(service.icon).toBe('folder');
    expect(service.appliesTo).toContain('content');
  });

  describe('getTree', () => {
    it('should return root category folders', async () => {
      const nodes = await firstValueFrom(service.getTree());
      expect(nodes.length).toBeGreaterThan(0);
      const content = nodes.find(n => n.id === 'category:content');
      expect(content).toBeDefined();
      expect(content!.nodeType).toBe('folder');
      expect(content!.folderLevel).toBe('category');
    });

    it('should include all resource categories', async () => {
      const nodes = await firstValueFrom(service.getTree());
      const ids = nodes.map(n => n.id);
      expect(ids).toContain('category:content');
      expect(ids).toContain('category:compute');
      expect(ids).toContain('category:storage');
    });

    it('should cache the tree on subsequent calls', async () => {
      await firstValueFrom(service.getTree());
      const callsBefore = getAllContentTypesSpy.mock.calls.length;
      await firstValueFrom(service.getTree());
      expect(getAllContentTypesSpy.mock.calls.length).toBe(callsBefore);
    });
  });

  describe('getChildren', () => {
    it('should return content type folders for category:content', async () => {
      const nodes = await firstValueFrom(service.getChildren('category:content'));
      expect(nodes.length).toBeGreaterThan(0);
      expect(nodes.every(n => n.nodeType === 'folder')).toBe(true);
      expect(nodes.every(n => n.folderLevel === 'type')).toBe(true);
    });

    it('should return empty for non-content categories', async () => {
      const nodes = await firstValueFrom(service.getChildren('category:compute'));
      expect(nodes).toEqual([]);
    });

    it('should return content files for a type folder', async () => {
      const nodes = await firstValueFrom(service.getChildren('type:articles'));
      expect(nodes.length).toBeGreaterThan(0);
      expect(nodes[0].nodeType).toBe('file');
      expect(nodes[0].resourceId).toBe('node-1');
    });

    it('should return empty for unknown folder IDs', async () => {
      const nodes = await firstValueFrom(service.getChildren('unknown:xyz'));
      expect(nodes).toEqual([]);
    });
  });

  describe('getBreadcrumbs', () => {
    it('should return root breadcrumb for category folders', async () => {
      const crumbs = await firstValueFrom(service.getBreadcrumbs('category:content'));
      expect(crumbs).toHaveLength(2);
      expect(crumbs[0]).toEqual({ id: null, title: 'My Resources' });
      expect(crumbs[1].title).toBe('Content');
    });

    it('should return 3-level breadcrumbs for type folders', async () => {
      const crumbs = await firstValueFrom(service.getBreadcrumbs('type:articles'));
      expect(crumbs).toHaveLength(3);
      expect(crumbs[0].title).toBe('My Resources');
      expect(crumbs[1].title).toBe('Content');
      expect(crumbs[2].title).toBe('Articles');
    });

    it('should return root-only breadcrumb for unknown IDs', async () => {
      const crumbs = await firstValueFrom(service.getBreadcrumbs('file:whatever'));
      expect(crumbs).toHaveLength(1);
      expect(crumbs[0]).toEqual({ id: null, title: 'My Resources' });
    });
  });

  describe('invalidateCache', () => {
    it('should clear the tree cache forcing a re-fetch', async () => {
      await firstValueFrom(service.getTree());
      const callsBefore = getAllContentTypesSpy.mock.calls.length;
      service.invalidateCache();
      await firstValueFrom(service.getTree());
      expect(getAllContentTypesSpy.mock.calls.length).toBe(callsBefore + 1);
    });
  });
});
