/**
 * Seeding Service Tests
 *
 * Coverage focus:
 * - Service creation and initialization
 * - Bulk content operations
 * - Bulk path operations
 * - Recovery and sync operations
 * - State management and observables
 * - Progress tracking
 */

import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { SeedingService, SeedingMode, BatchResult, SeedingProgress } from './seeding.service';
import { ContentNode } from '../../lamad/models/content-node.model';
import { LearningPath } from '../../lamad/models/learning-path.model';

describe('SeedingService', () => {
  let service: SeedingService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [
        SeedingService,
        provideHttpClient(),
        provideHttpClientTesting(),
      ],
    });

    service = TestBed.inject(SeedingService);
  });

  afterEach(() => {
    service.ngOnDestroy();
  });

  // ==========================================================================
  // Service Creation Tests
  // ==========================================================================

  describe('service creation', () => {
    it('should be created', () => {
      expect(service).toBeTruthy();
    });

    it('should be a SeedingService instance', () => {
      expect(service instanceof SeedingService).toBe(true);
    });
  });

  // ==========================================================================
  // Initialization Tests
  // ==========================================================================

  describe('initialization', () => {
    it('should have initialize method', () => {
      expect(typeof service.initialize).toBe('function');
    });

    it('initialize should return Promise', () => {
      const promise = service.initialize('import');
      expect(promise instanceof Promise).toBe(true);
    });

    it('should initialize with import mode', async () => {
      await service.initialize('import');
      expect(service.state).toBe('idle');
    });

    it('should initialize with recovery mode', async () => {
      await service.initialize('recovery');
      expect(service.state).toBe('idle');
    });

    it('should initialize with sync mode', async () => {
      await service.initialize('sync');
      expect(service.state).toBe('idle');
    });

    it('should initialize without mode parameter', async () => {
      await service.initialize();
      expect(service.state).toBe('idle');
    });
  });

  // ==========================================================================
  // Bulk Content Creation Tests
  // ==========================================================================

  describe('bulk content operations', () => {
    it('should have bulkCreateContent method', () => {
      expect(typeof service.bulkCreateContent).toBe('function');
    });

    it('bulkCreateContent should return Promise<BatchResult>', () => {
      const mockContent: ContentNode[] = [];
      const promise = service.bulkCreateContent(mockContent);
      expect(promise instanceof Promise).toBe(true);
    });

    it('should accept empty content array', async () => {
      const mockContent: ContentNode[] = [];
      const promise = service.bulkCreateContent(mockContent);
      expect(promise instanceof Promise).toBe(true);
    });

    it('should accept multiple content nodes', async () => {
      const mockContent: ContentNode[] = [
        {
          id: 'content-1',
          contentType: 'concept',
          title: 'Content 1',
          description: 'Description 1',
          content: 'Content body 1',
          contentFormat: 'markdown',
          tags: ['tag1'],
          relatedNodeIds: [],
          metadata: {},
        },
        {
          id: 'content-2',
          contentType: 'video',
          title: 'Content 2',
          description: 'Description 2',
          content: 'Content body 2',
          contentFormat: 'html5-app',
          tags: ['tag2'],
          relatedNodeIds: [],
          metadata: {},
        },
      ];

      const promise = service.bulkCreateContent(mockContent);
      expect(promise instanceof Promise).toBe(true);
    });

    it('bulkCreateContent should auto-initialize if not ready', async () => {
      const mockContent: ContentNode[] = [];
      await service.bulkCreateContent(mockContent);
      expect(service.state).toBe('idle');
    });
  });

  // ==========================================================================
  // Bulk Path Creation Tests
  // ==========================================================================

  describe('bulk path operations', () => {
    it('should have bulkCreatePaths method', () => {
      expect(typeof service.bulkCreatePaths).toBe('function');
    });

    it('bulkCreatePaths should return Promise<BatchResult>', () => {
      const mockPaths: LearningPath[] = [];
      const promise = service.bulkCreatePaths(mockPaths);
      expect(promise instanceof Promise).toBe(true);
    });

    it('should accept empty paths array', async () => {
      const mockPaths: LearningPath[] = [];
      const promise = service.bulkCreatePaths(mockPaths);
      expect(promise instanceof Promise).toBe(true);
    });

    it('should accept multiple paths', async () => {
      const mockPaths: LearningPath[] = [
        {
          id: 'path-1',
          version: '1.0.0',
          title: 'Path 1',
          description: 'Description 1',
          purpose: 'Test purpose',
          difficulty: 'beginner',
          steps: [],
          visibility: 'public',
          createdBy: 'test-author',
          contributors: [],
          tags: [],
          estimatedDuration: '1 hour',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
      ];

      const promise = service.bulkCreatePaths(mockPaths);
      expect(promise instanceof Promise).toBe(true);
    });

    it('bulkCreatePaths should auto-initialize if not ready', async () => {
      const mockPaths: LearningPath[] = [];
      await service.bulkCreatePaths(mockPaths);
      expect(service.state).toBe('idle');
    });
  });

  // ==========================================================================
  // Recovery and Sync Tests
  // ==========================================================================

  describe('recovery and sync operations', () => {
    it('should have recoverySync method', () => {
      expect(typeof service.recoverySync).toBe('function');
    });

    it('should have incrementalSync method', () => {
      expect(typeof service.incrementalSync).toBe('function');
    });

    it('recoverySync should return Promise with content and paths', () => {
      const mockContent: ContentNode[] = [];
      const mockPaths: LearningPath[] = [];
      const promise = service.recoverySync(mockContent, mockPaths);
      expect(promise instanceof Promise).toBe(true);
    });

    it('incrementalSync should return Promise<BatchResult>', () => {
      const mockContent: ContentNode[] = [];
      const promise = service.incrementalSync(mockContent);
      expect(promise instanceof Promise).toBe(true);
    });

    it('incrementalSync should accept content array', async () => {
      const mockContent: ContentNode[] = [
        {
          id: 'content-1',
          contentType: 'concept',
          title: 'Updated Content',
          description: 'Updated',
          content: 'Updated body',
          contentFormat: 'markdown',
          tags: [],
          relatedNodeIds: [],
          metadata: {},
        },
      ];

      const promise = service.incrementalSync(mockContent);
      expect(promise instanceof Promise).toBe(true);
    });
  });

  // ==========================================================================
  // State Management Tests
  // ==========================================================================

  describe('state management', () => {
    it('should expose state$ as observable', () => {
      expect(service.state$).toBeDefined();
      expect(service.state$.subscribe).toBeDefined();
    });

    it('should expose progress$ as observable', () => {
      expect(service.progress$).toBeDefined();
      expect(service.progress$.subscribe).toBeDefined();
    });

    it('should expose bufferStats$ as observable', () => {
      expect(service.bufferStats$).toBeDefined();
      expect(service.bufferStats$.subscribe).toBeDefined();
    });

    it('should expose backpressure$ as observable', () => {
      expect(service.backpressure$).toBeDefined();
      expect(service.backpressure$.subscribe).toBeDefined();
    });

    it('should have state getter', () => {
      expect(service.state).toBeDefined();
      expect(['idle', 'seeding', 'error']).toContain(service.state);
    });

    it('should have isSeeding getter', () => {
      expect(service.isSeeding).toBeDefined();
      expect(typeof service.isSeeding).toBe('boolean');
    });

    it('initial state should be idle', () => {
      expect(service.state).toBe('idle');
    });

    it('initial isSeeding should be false', () => {
      expect(service.isSeeding).toBe(false);
    });
  });

  // ==========================================================================
  // Cancellation Tests
  // ==========================================================================

  describe('cancellation', () => {
    it('should have cancel method', () => {
      expect(typeof service.cancel).toBe('function');
    });

    it('cancel should return void', async () => {
      await service.initialize('import');
      const result = service.cancel();
      expect(result).toBeUndefined();
    });

    it('cancel should reset state to idle', async () => {
      await service.initialize('import');
      service.cancel();
      expect(service.state).toBe('idle');
    });
  });

  // ==========================================================================
  // Buffer Statistics Tests
  // ==========================================================================

  describe('buffer statistics', () => {
    it('should have getBufferStats method', () => {
      expect(typeof service.getBufferStats).toBe('function');
    });

    it('getBufferStats should return WriteBufferStats or null', async () => {
      await service.initialize('import');
      const result = service.getBufferStats();
      expect(result === null || typeof result === 'object').toBe(true);
    });
  });

  // ==========================================================================
  // Method Existence Tests
  // ==========================================================================

  describe('method existence', () => {
    it('should have initialize', () => {
      expect(typeof service.initialize).toBe('function');
    });

    it('should have bulkCreateContent', () => {
      expect(typeof service.bulkCreateContent).toBe('function');
    });

    it('should have bulkCreatePaths', () => {
      expect(typeof service.bulkCreatePaths).toBe('function');
    });

    it('should have recoverySync', () => {
      expect(typeof service.recoverySync).toBe('function');
    });

    it('should have incrementalSync', () => {
      expect(typeof service.incrementalSync).toBe('function');
    });

    it('should have cancel', () => {
      expect(typeof service.cancel).toBe('function');
    });

    it('should have getBufferStats', () => {
      expect(typeof service.getBufferStats).toBe('function');
    });

    it('should have ngOnDestroy', () => {
      expect(typeof service.ngOnDestroy).toBe('function');
    });
  });

  // ==========================================================================
  // Parameter Acceptance Tests
  // ==========================================================================

  describe('parameter acceptance', () => {
    it('should accept SeedingMode for initialize', () => {
      expect(() => service.initialize('import')).not.toThrow();
      expect(() => service.initialize('recovery')).not.toThrow();
      expect(() => service.initialize('sync')).not.toThrow();
    });

    it('should accept undefined for initialize', () => {
      expect(() => service.initialize()).not.toThrow();
    });

    it('should accept ContentNode[] for bulkCreateContent', () => {
      const mockContent: ContentNode[] = [];
      expect(() => service.bulkCreateContent(mockContent)).not.toThrow();
    });

    it('should accept LearningPath[] for bulkCreatePaths', () => {
      const mockPaths: LearningPath[] = [];
      expect(() => service.bulkCreatePaths(mockPaths)).not.toThrow();
    });

    it('should accept ContentNode[] and LearningPath[] for recoverySync', () => {
      const mockContent: ContentNode[] = [];
      const mockPaths: LearningPath[] = [];
      expect(() => service.recoverySync(mockContent, mockPaths)).not.toThrow();
    });

    it('should accept ContentNode[] for incrementalSync', () => {
      const mockContent: ContentNode[] = [];
      expect(() => service.incrementalSync(mockContent)).not.toThrow();
    });
  });

  // ==========================================================================
  // Return Type Tests
  // ==========================================================================

  describe('return types', () => {
    it('initialize should return Promise<void>', () => {
      const result = service.initialize();
      expect(result instanceof Promise).toBe(true);
    });

    it('bulkCreateContent should return Promise<BatchResult>', () => {
      const result = service.bulkCreateContent([]);
      expect(result instanceof Promise).toBe(true);
    });

    it('bulkCreatePaths should return Promise<BatchResult>', () => {
      const result = service.bulkCreatePaths([]);
      expect(result instanceof Promise).toBe(true);
    });

    it('recoverySync should return Promise', () => {
      const result = service.recoverySync([], []);
      expect(result instanceof Promise).toBe(true);
    });

    it('incrementalSync should return Promise<BatchResult>', () => {
      const result = service.incrementalSync([]);
      expect(result instanceof Promise).toBe(true);
    });

    it('cancel should return void', async () => {
      await service.initialize('import');
      const result = service.cancel();
      expect(result).toBeUndefined();
    });

    it('state$ should be Observable', () => {
      expect(service.state$.subscribe).toBeDefined();
    });

    it('progress$ should be Observable', () => {
      expect(service.progress$.subscribe).toBeDefined();
    });
  });

  // ==========================================================================
  // Edge Cases Tests
  // ==========================================================================

  describe('edge cases', () => {
    it('should handle empty content array', async () => {
      const mockContent: ContentNode[] = [];
      const promise = service.bulkCreateContent(mockContent);
      expect(promise instanceof Promise).toBe(true);
    });

    it('should handle empty paths array', async () => {
      const mockPaths: LearningPath[] = [];
      const promise = service.bulkCreatePaths(mockPaths);
      expect(promise instanceof Promise).toBe(true);
    });

    it('should handle recovery sync with empty content', async () => {
      const promise = service.recoverySync([], []);
      expect(promise instanceof Promise).toBe(true);
    });

    it('should handle multiple initialize calls', async () => {
      await service.initialize('import');
      await service.initialize('recovery');
      expect(service.state).toBe('idle');
    });

    it('should handle cancel on idle state', async () => {
      await service.initialize('import');
      expect(service.state).toBe('idle');
      service.cancel();
      expect(service.state).toBe('idle');
    });
  });

  // ==========================================================================
  // Lifecycle Tests
  // ==========================================================================

  describe('lifecycle', () => {
    it('should have ngOnDestroy method', () => {
      expect(typeof service.ngOnDestroy).toBe('function');
    });

    it('should handle ngOnDestroy without error', () => {
      expect(() => service.ngOnDestroy()).not.toThrow();
    });
  });
});
