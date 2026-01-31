import { TestBed } from '@angular/core/testing';
import { PathContextService } from './path-context.service';
import { PathContext, DetourInfo } from '../models/exploration-context.model';

describe('PathContextService', () => {
  let service: PathContextService;

  const mockPathContextTemplate: PathContext = {
    pathId: 'test-path',
    pathTitle: 'Test Learning Path',
    stepIndex: 2,
    totalSteps: 10,
    chapterTitle: 'Introduction',
    returnRoute: ['/lamad/path', 'test-path', 'step', '2'],
    detourStack: [],
  };

  const mockDetourTemplate: DetourInfo = {
    fromContentId: 'concept-1',
    toContentId: 'related-concept',
    detourType: 'related',
    timestamp: new Date().toISOString(),
  };

  // Helper function to get fresh copies of mock data
  const getMockPathContext = (): PathContext => JSON.parse(JSON.stringify(mockPathContextTemplate));
  const getMockDetour = (): DetourInfo => JSON.parse(JSON.stringify(mockDetourTemplate));

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [PathContextService],
    });

    // Create a new instance for each test to ensure isolation
    service = new PathContextService();
  });

  afterEach(() => {
    service.clearAll();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('enterPath', () => {
    it('should set active path context', done => {
      service.context$.subscribe(context => {
        if (context) {
          expect(context.pathId).toBe('test-path');
          expect(context.pathTitle).toBe('Test Learning Path');
          expect(context.stepIndex).toBe(2);
          done();
        }
      });

      service.enterPath(getMockPathContext());
    });

    it('should initialize detour stack', () => {
      const contextWithoutStack = { ...getMockPathContext() };
      delete (contextWithoutStack as any).detourStack;

      service.enterPath(contextWithoutStack);

      const current = service.currentContext;
      expect(current?.detourStack).toEqual([]);
    });

    it('should update existing path position', () => {
      service.enterPath(getMockPathContext());
      expect(service.currentContext?.stepIndex).toBe(2);

      service.enterPath({
        ...getMockPathContext(),
        stepIndex: 5,
        chapterTitle: 'Advanced Topics',
      });

      expect(service.currentContext?.stepIndex).toBe(5);
      expect(service.currentContext?.chapterTitle).toBe('Advanced Topics');
    });

    it('should support nested paths', () => {
      service.enterPath(getMockPathContext());
      service.enterPath({
        pathId: 'nested-path',
        pathTitle: 'Nested Path',
        stepIndex: 0,
        totalSteps: 5,
        returnRoute: ['/lamad/path', 'nested-path', 'step', '0'],
      });

      expect(service.currentContext?.pathId).toBe('nested-path');
      expect((service as any).contextStack.length).toBe(2);
    });
  });

  describe('currentContext', () => {
    it('should return null when no context', () => {
      expect(service.currentContext).toBeNull();
    });

    it('should return active context', () => {
      service.enterPath(getMockPathContext());
      expect(service.currentContext).toEqual(jasmine.objectContaining(getMockPathContext()));
    });
  });

  describe('hasPathContext', () => {
    it('should return false when no path context', () => {
      expect(service.hasPathContext).toBe(false);
    });

    it('should return true when in a path', () => {
      service.enterPath(getMockPathContext());
      expect(service.hasPathContext).toBe(true);
    });
  });

  describe('isInDetour', () => {
    it('should return false when not in detour', () => {
      service.enterPath(getMockPathContext());
      expect(service.isInDetour).toBe(false);
    });

    it('should return true when in detour', () => {
      service.enterPath(getMockPathContext());
      service.startDetour(getMockDetour());
      expect(service.isInDetour).toBe(true);
    });

    it('should return false when no path context', () => {
      expect(service.isInDetour).toBe(false);
    });
  });

  describe('detourDepth', () => {
    it('should return 0 when not in detour', () => {
      service.enterPath(getMockPathContext());
      expect(service.detourDepth).toBe(0);
    });

    it('should return detour count', () => {
      service.enterPath(getMockPathContext());
      service.startDetour(getMockDetour());
      expect(service.detourDepth).toBe(1);

      service.startDetour({
        ...getMockDetour(),
        fromContentId: 'related-concept',
        toContentId: 'deeper-concept',
      });
      expect(service.detourDepth).toBe(2);
    });
  });

  describe('updatePosition', () => {
    it('should update step index', () => {
      service.enterPath(getMockPathContext());
      service.updatePosition(5);
      expect(service.currentContext?.stepIndex).toBe(5);
    });

    it('should update chapter title', () => {
      service.enterPath(getMockPathContext());
      service.updatePosition(5, 'New Chapter');
      expect(service.currentContext?.chapterTitle).toBe('New Chapter');
    });

    it('should update return route', () => {
      service.enterPath(getMockPathContext());
      service.updatePosition(5);
      expect(service.currentContext?.returnRoute).toEqual([
        '/lamad/path',
        'test-path',
        'step',
        '5',
      ]);
    });

    it('should do nothing when no context', () => {
      expect(() => service.updatePosition(5)).not.toThrow();
    });
  });

  describe('startDetour', () => {
    it('should add detour to stack', () => {
      service.enterPath(getMockPathContext());
      service.startDetour(getMockDetour());

      const context = service.currentContext;
      expect(context?.detourStack?.length).toBe(1);
      expect(context?.detourStack?.[0]).toEqual(getMockDetour());
    });

    it('should support nested detours', () => {
      service.enterPath(getMockPathContext());
      service.startDetour(getMockDetour());
      service.startDetour({
        ...getMockDetour(),
        fromContentId: 'related-concept',
        toContentId: 'deeper-concept',
      });

      expect(service.currentContext?.detourStack?.length).toBe(2);
    });

    it('should do nothing when no path context', () => {
      expect(() => service.startDetour(getMockDetour())).not.toThrow();
    });

    it('should initialize detour stack if undefined', () => {
      service.enterPath({ ...getMockPathContext(), detourStack: undefined });
      service.startDetour(getMockDetour());
      expect(service.currentContext?.detourStack?.length).toBe(1);
    });
  });

  describe('returnFromDetour', () => {
    it('should return null when not in detour', () => {
      service.enterPath(getMockPathContext());
      const route = service.returnFromDetour();
      expect(route).toBeNull();
    });

    it('should pop detour and return to path', () => {
      service.enterPath(getMockPathContext());
      service.startDetour(getMockDetour());

      const route = service.returnFromDetour();
      expect(route).toEqual(getMockPathContext().returnRoute);
      expect(service.detourDepth).toBe(0);
    });

    it('should return to previous detour when nested', () => {
      service.enterPath(getMockPathContext());
      service.startDetour(getMockDetour());
      service.startDetour({
        ...getMockDetour(),
        fromContentId: 'related-concept',
        toContentId: 'deeper-concept',
      });

      const route = service.returnFromDetour();
      expect(route).toEqual(['/lamad/resource', 'related-concept']);
      expect(service.detourDepth).toBe(1);
    });

    it('should return null when no context', () => {
      expect(service.returnFromDetour()).toBeNull();
    });
  });

  describe('returnToPath', () => {
    it('should clear all detours and return path route', () => {
      service.enterPath(getMockPathContext());
      service.startDetour(getMockDetour());
      service.startDetour({
        ...getMockDetour(),
        fromContentId: 'related-concept',
        toContentId: 'deeper-concept',
      });

      const route = service.returnToPath();
      expect(route).toEqual(getMockPathContext().returnRoute);
      expect(service.detourDepth).toBe(0);
    });

    it('should return null when no context', () => {
      expect(service.returnToPath()).toBeNull();
    });
  });

  describe('exitPath', () => {
    it('should remove current path from stack', () => {
      service.enterPath(getMockPathContext());
      expect(service.hasPathContext).toBe(true);

      service.exitPath();
      expect(service.hasPathContext).toBe(false);
    });

    it('should restore previous path if nested', () => {
      service.enterPath(getMockPathContext());
      const nestedContext: PathContext = {
        pathId: 'nested-path',
        pathTitle: 'Nested Path',
        stepIndex: 0,
        totalSteps: 5,
        returnRoute: ['/lamad/path', 'nested-path', 'step', '0'],
      };
      service.enterPath(nestedContext);

      service.exitPath();
      expect(service.currentContext?.pathId).toBe('test-path');
    });

    it('should handle exit when no context', () => {
      expect(() => service.exitPath()).not.toThrow();
    });
  });

  describe('clearAll', () => {
    it('should clear all contexts', () => {
      service.enterPath(getMockPathContext());
      service.startDetour(getMockDetour());
      expect(service.hasPathContext).toBe(true);

      service.clearAll();
      expect(service.hasPathContext).toBe(false);
      expect(service.currentContext).toBeNull();
    });
  });

  describe('getBreadcrumbs', () => {
    it('should return empty array when no context', () => {
      expect(service.getBreadcrumbs()).toEqual([]);
    });

    it('should return basic breadcrumbs for path', () => {
      service.enterPath(getMockPathContext());
      const breadcrumbs = service.getBreadcrumbs();

      expect(breadcrumbs.length).toBe(4); // Paths, Path Title, Chapter, Step
      expect(breadcrumbs[0].label).toBe('Paths');
      expect(breadcrumbs[1].label).toBe('Test Learning Path');
      expect(breadcrumbs[2].label).toBe('Introduction');
      expect(breadcrumbs[3].label).toBe('Step 3'); // stepIndex 2 + 1
    });

    it('should mark current item correctly', () => {
      service.enterPath(getMockPathContext());
      const breadcrumbs = service.getBreadcrumbs();
      const currentItem = breadcrumbs.find(b => b.isCurrent);
      expect(currentItem?.label).toBe('Step 3');
    });

    it('should include detour breadcrumbs', () => {
      service.enterPath(getMockPathContext());
      service.startDetour(getMockDetour());

      const breadcrumbs = service.getBreadcrumbs();
      const detourItem = breadcrumbs.find(b => b.isDetour);
      expect(detourItem).toBeDefined();
      expect(detourItem?.label).toContain('related-concept');
    });

    it('should mark last detour as current', () => {
      service.enterPath(getMockPathContext());
      service.startDetour(getMockDetour());

      const breadcrumbs = service.getBreadcrumbs();
      const detourItem = breadcrumbs.find(b => b.isDetour);
      expect(detourItem?.isCurrent).toBe(true);
    });

    it('should handle path without chapter title', () => {
      const contextNoChapter = { ...getMockPathContext(), chapterTitle: undefined };
      service.enterPath(contextNoChapter);

      const breadcrumbs = service.getBreadcrumbs();
      expect(breadcrumbs.length).toBe(3); // No chapter breadcrumb
    });
  });

  describe('getContextSummary', () => {
    it('should return null when no context', () => {
      expect(service.getContextSummary()).toBeNull();
    });

    it('should return context summary', () => {
      service.enterPath(getMockPathContext());
      const summary = service.getContextSummary();

      expect(summary).toEqual({
        pathTitle: 'Test Learning Path',
        stepIndex: 2,
        totalSteps: 10,
        chapterTitle: 'Introduction',
        detourCount: 0,
        returnRoute: getMockPathContext().returnRoute,
      });
    });

    it('should include detour count', () => {
      service.enterPath(getMockPathContext());
      service.startDetour(getMockDetour());
      service.startDetour({ ...getMockDetour(), toContentId: 'another-concept' });

      const summary = service.getContextSummary();
      expect(summary?.detourCount).toBe(2);
    });
  });

  describe('context$ observable', () => {
    it('should emit when context changes', done => {
      let emitCount = 0;
      service.context$.subscribe(() => {
        emitCount++;
        if (emitCount === 2) {
          // Initial null + enterPath
          done();
        }
      });

      service.enterPath(getMockPathContext());
    });

    it('should emit when detour starts', done => {
      let emitCount = 0;
      service.context$.subscribe(context => {
        emitCount++;
        if (emitCount === 3) {
          // Initial null + enterPath + startDetour
          expect(context?.detourStack?.length).toBe(1);
          done();
        }
      });

      service.enterPath(getMockPathContext());
      service.startDetour(getMockDetour());
    });

    it('should emit when position updates', done => {
      let emitCount = 0;
      service.context$.subscribe(context => {
        emitCount++;
        if (emitCount === 3) {
          // Initial null + enterPath + updatePosition
          expect(context?.stepIndex).toBe(7);
          done();
        }
      });

      service.enterPath(getMockPathContext());
      service.updatePosition(7);
    });
  });

  describe('edge cases', () => {
    it('should handle multiple paths and detours correctly', () => {
      // Path 1
      service.enterPath(getMockPathContext());
      service.startDetour(getMockDetour());

      // Path 2 (nested)
      service.enterPath({
        pathId: 'path-2',
        pathTitle: 'Path 2',
        stepIndex: 0,
        totalSteps: 5,
        returnRoute: ['/lamad/path', 'path-2', 'step', '0'],
      });

      expect((service as any).contextStack.length).toBe(2);
      expect(service.currentContext?.pathId).toBe('path-2');

      // Exit path 2
      service.exitPath();
      expect(service.currentContext?.pathId).toBe('test-path');
      expect(service.detourDepth).toBe(1); // Still in detour from path 1
    });

    it('should handle returning from detour when stack is modified', () => {
      service.enterPath(getMockPathContext());
      service.startDetour(getMockDetour());

      // Manually corrupt the detour stack
      const context = service.currentContext;
      if (context?.detourStack) {
        context.detourStack = [];
      }

      const route = service.returnFromDetour();
      expect(route).toBeNull();
    });
  });
});
