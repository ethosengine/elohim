import { TestBed } from '@angular/core/testing';

import { of } from 'rxjs';

import { LensRegistryService } from './lens-registry.service';

import type { LensProvider, ResourceCategory } from '@app/shefa/models/resource-explorer.model';

function mockProvider(overrides: Partial<LensProvider> = {}): LensProvider {
  return {
    type: 'test-lens',
    label: 'Test Lens',
    icon: 'science',
    description: 'A test lens',
    appliesTo: ['content'] as ResourceCategory[],
    getTree: () => of([]),
    getChildren: () => of([]),
    getBreadcrumbs: () => of([]),
    ...overrides,
  };
}

describe('LensRegistryService', () => {
  let service: LensRegistryService;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    service = TestBed.inject(LensRegistryService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should start with no lens definitions', () => {
    expect(service.lensDefinitions()).toEqual([]);
  });

  describe('register', () => {
    it('should register a lens provider', () => {
      service.register(mockProvider());
      expect(service.lensDefinitions()).toHaveLength(1);
      expect(service.lensDefinitions()[0].type).toBe('test-lens');
    });

    it('should replace an existing provider with the same type', () => {
      service.register(mockProvider({ label: 'First' }));
      service.register(mockProvider({ label: 'Second' }));
      expect(service.lensDefinitions()).toHaveLength(1);
      expect(service.lensDefinitions()[0].label).toBe('Second');
    });

    it('should register multiple providers', () => {
      service.register(mockProvider({ type: 'lens-a' }));
      service.register(mockProvider({ type: 'lens-b' }));
      expect(service.lensDefinitions()).toHaveLength(2);
    });
  });

  describe('unregister', () => {
    it('should remove a registered provider', () => {
      service.register(mockProvider());
      service.unregister('test-lens');
      expect(service.lensDefinitions()).toHaveLength(0);
    });

    it('should be a no-op for unregistered types', () => {
      service.register(mockProvider());
      service.unregister('nonexistent');
      expect(service.lensDefinitions()).toHaveLength(1);
    });
  });

  describe('getProvider', () => {
    it('should return a registered provider', () => {
      const provider = mockProvider();
      service.register(provider);
      expect(service.getProvider('test-lens')).toBe(provider);
    });

    it('should return undefined for unregistered types', () => {
      expect(service.getProvider('nonexistent')).toBeUndefined();
    });
  });

  describe('getLensesForCategory', () => {
    it('should return lenses that apply to the given category', () => {
      service.register(mockProvider({ type: 'a', appliesTo: ['content'] }));
      service.register(mockProvider({ type: 'b', appliesTo: ['compute'] }));
      const result = service.getLensesForCategory('content');
      expect(result).toHaveLength(1);
      expect(result[0].type).toBe('a');
    });

    it('should return multiple lenses for the same category', () => {
      service.register(mockProvider({ type: 'a', appliesTo: ['content'] }));
      service.register(mockProvider({ type: 'b', appliesTo: ['content', 'compute'] }));
      expect(service.getLensesForCategory('content')).toHaveLength(2);
    });

    it('should return empty array when no lenses match', () => {
      service.register(mockProvider({ appliesTo: ['compute'] }));
      expect(service.getLensesForCategory('content')).toHaveLength(0);
    });
  });

  describe('lensDefinitions', () => {
    it('should extract lightweight definitions from providers', () => {
      service.register(
        mockProvider({
          type: 'folders',
          label: 'Folders',
          icon: 'folder',
          description: 'Browse by folder',
          appliesTo: ['content', 'compute'],
        })
      );

      const defs = service.lensDefinitions();
      expect(defs).toHaveLength(1);
      expect(defs[0]).toEqual({
        type: 'folders',
        label: 'Folders',
        icon: 'folder',
        description: 'Browse by folder',
        appliesTo: ['content', 'compute'],
      });
    });
  });
});
