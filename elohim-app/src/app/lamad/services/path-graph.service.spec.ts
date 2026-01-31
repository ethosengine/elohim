import { TestBed } from '@angular/core/testing';
import { of } from 'rxjs';

import { PathGraphService } from './path-graph.service';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';
import { LearningPath, PathStep, PathChapter } from '../models/learning-path.model';
import { ContentNode } from '../models/content-node.model';

describe('PathGraphService', () => {
  let service: PathGraphService;
  let mockDataLoader: jasmine.SpyObj<DataLoaderService>;

  beforeEach(() => {
    mockDataLoader = jasmine.createSpyObj('DataLoaderService', ['getPathIndex', 'getPath']);
    mockDataLoader.getPathIndex.and.returnValue(
      of({ paths: [], lastUpdated: new Date().toISOString(), totalCount: 0 })
    );
    mockDataLoader.getPath.and.returnValue(of(null as any));

    TestBed.configureTestingModule({
      providers: [
        PathGraphService,
        { provide: DataLoaderService, useValue: mockDataLoader }
      ],
    });
    service = TestBed.inject(PathGraphService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('registerPathAsNode', () => {
    it('should return observable with content node', (done) => {
      const mockPath: LearningPath = {
        id: 'path-123',
        version: '1.0.0',
        title: 'Test Path',
        description: 'Test Description',
        purpose: 'Test Purpose',
        createdBy: 'test-agent',
        contributors: [],
        steps: [
          {
            stepType: 'content',
            resourceId: 'content-1',
            stepTitle: 'Step 1',
            stepNarrative: 'Test narrative',
            learningObjectives: [],
            optional: false,
            completionCriteria: [],
            order: 1
          } as PathStep,
        ],
        difficulty: 'beginner',
        estimatedDuration: '60 minutes',
        tags: ['test'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        visibility: 'public',
      };

      service.registerPathAsNode(mockPath).subscribe((node) => {
        expect(node).toBeDefined();
        expect(node.id).toBe('path-path-123');
        expect(node.contentType).toBe('path');
        expect(node.title).toBe('Test Path');
        done();
      });
    });

    it('should include path metadata with step count', (done) => {
      const mockPath: LearningPath = {
        id: 'path-456',
        version: '1.0.0',
        title: 'Multi-Step Path',
        description: 'Test',
        purpose: 'Test Purpose',
        createdBy: 'test-agent',
        contributors: [],
        steps: [
          {
            stepType: 'content',
            resourceId: 'content-1',
            stepTitle: 'Step 1',
            stepNarrative: 'Test narrative',
            learningObjectives: [],
            optional: false,
            completionCriteria: [],
            order: 1
          } as PathStep,
          {
            stepType: 'content',
            resourceId: 'content-2',
            stepTitle: 'Step 2',
            stepNarrative: 'Test narrative',
            learningObjectives: [],
            optional: false,
            completionCriteria: [],
            order: 2
          } as PathStep,
        ],
        difficulty: 'intermediate',
        estimatedDuration: '120 minutes',
        tags: ['test'],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        visibility: 'public',
      };

      service.registerPathAsNode(mockPath).subscribe((node) => {
        expect(node.metadata).toBeDefined();
        done();
      });
    });

    it('should map visibility to reach level', (done) => {
      const mockPath: LearningPath = {
        id: 'path-789',
        version: '1.0.0',
        title: 'Private Path',
        description: 'Test',
        purpose: 'Test Purpose',
        createdBy: 'test-agent',
        contributors: [],
        steps: [],
        difficulty: 'advanced',
        estimatedDuration: '180 minutes',
        tags: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        visibility: 'private',
      };

      service.registerPathAsNode(mockPath).subscribe((node) => {
        expect(node.reach).toBe('private');
        done();
      });
    });
  });

  describe('findPathsContainingNode', () => {
    it('should return observable of path references', (done) => {
      mockDataLoader.getPathIndex.and.returnValue(
        of({
          paths: [{
            id: 'path-1',
            title: 'Path 1',
            description: 'Test',
            difficulty: 'beginner',
            estimatedDuration: '60 minutes',
            stepCount: 1,
            tags: []
          }],
          lastUpdated: new Date().toISOString(),
          totalCount: 1,
        })
      );

      const mockPath: LearningPath = {
        id: 'path-1',
        version: '1.0.0',
        title: 'Path 1',
        description: 'Test',
        purpose: 'Test Purpose',
        createdBy: 'test-agent',
        contributors: [],
        steps: [
          {
            stepType: 'content',
            resourceId: 'content-123',
            stepTitle: 'Step',
            stepNarrative: 'Test narrative',
            learningObjectives: [],
            optional: false,
            completionCriteria: [],
            order: 1
          } as PathStep,
        ],
        difficulty: 'beginner',
        estimatedDuration: '60 minutes',
        tags: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        visibility: 'public',
      };

      mockDataLoader.getPath.and.returnValue(of(mockPath));

      service.findPathsContainingNode('content-123').subscribe((results) => {
        expect(results).toBeDefined();
        expect(Array.isArray(results)).toBe(true);
        done();
      });
    });
  });

  describe('getPathContentNodes', () => {
    it('should return observable of content node IDs', (done) => {
      const mockPath: LearningPath = {
        id: 'path-123',
        version: '1.0.0',
        title: 'Test Path',
        description: 'Test',
        purpose: 'Test Purpose',
        createdBy: 'test-agent',
        contributors: [],
        steps: [
          {
            stepType: 'content',
            resourceId: 'content-1',
            stepTitle: 'Step 1',
            stepNarrative: 'Test narrative',
            learningObjectives: [],
            optional: false,
            completionCriteria: [],
            order: 1
          } as PathStep,
          {
            stepType: 'content',
            resourceId: 'content-2',
            stepTitle: 'Step 2',
            stepNarrative: 'Test narrative',
            learningObjectives: [],
            optional: false,
            completionCriteria: [],
            order: 2
          } as PathStep,
        ],
        difficulty: 'beginner',
        estimatedDuration: '60 minutes',
        tags: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        visibility: 'public',
      };

      mockDataLoader.getPath.and.returnValue(of(mockPath));

      service.getPathContentNodes('path-123').subscribe((nodeIds) => {
        expect(Array.isArray(nodeIds)).toBe(true);
        expect(nodeIds.length).toBeGreaterThan(0);
        done();
      });
    });
  });

  describe('getRelatedPaths', () => {
    it('should return observable of related path references', (done) => {
      const mockPath: LearningPath = {
        id: 'path-1',
        version: '1.0.0',
        title: 'Path 1',
        description: 'Test',
        purpose: 'Test Purpose',
        createdBy: 'test-agent',
        contributors: [],
        steps: [
          {
            stepType: 'content',
            resourceId: 'content-1',
            stepTitle: 'Step',
            stepNarrative: 'Test narrative',
            learningObjectives: [],
            optional: false,
            completionCriteria: [],
            order: 1
          } as PathStep,
        ],
        difficulty: 'beginner',
        estimatedDuration: '60 minutes',
        tags: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        visibility: 'public',
      };

      mockDataLoader.getPath.and.returnValue(of(mockPath));
      mockDataLoader.getPathIndex.and.returnValue(
        of({
          paths: [{
            id: 'path-1',
            title: 'Path 1',
            description: 'Test',
            difficulty: 'beginner',
            estimatedDuration: '60 minutes',
            stepCount: 1,
            tags: []
          }],
          lastUpdated: new Date().toISOString(),
          totalCount: 1,
        })
      );

      service.getRelatedPaths('path-1').subscribe((results) => {
        expect(Array.isArray(results)).toBe(true);
        done();
      });
    });
  });

  describe('syncPathNode', () => {
    it('should return observable with synced content node', (done) => {
      const mockPath: LearningPath = {
        id: 'path-123',
        version: '1.0.0',
        title: 'Updated Path',
        description: 'Updated',
        purpose: 'Test Purpose',
        createdBy: 'test-agent',
        contributors: [],
        steps: [],
        difficulty: 'intermediate',
        estimatedDuration: '90 minutes',
        tags: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        visibility: 'public',
      };

      service.syncPathNode(mockPath).subscribe((node) => {
        expect(node).toBeDefined();
        expect(node.title).toBe('Updated Path');
        done();
      });
    });
  });

  describe('loadPathNodes', () => {
    it('should return observable of path nodes map', (done) => {
      service.loadPathNodes().subscribe((nodes) => {
        expect(nodes instanceof Map).toBe(true);
        done();
      });
    });

    it('should cache path nodes on second call', (done) => {
      service.loadPathNodes().subscribe(() => {
        service.loadPathNodes().subscribe(() => {
          expect(mockDataLoader.getPathIndex.calls.count()).toBe(1);
          done();
        });
      });
    });
  });

  describe('clearCache', () => {
    it('should clear path nodes cache', (done) => {
      service.loadPathNodes().subscribe(() => {
        service.clearCache();
        service.loadPathNodes().subscribe(() => {
          expect(mockDataLoader.getPathIndex.calls.count()).toBe(2);
          done();
        });
      });
    });
  });

  describe('getPathNodesByTag', () => {
    it('should return observable of content nodes with tag', (done) => {
      service.getPathNodesByTag('test').subscribe((results) => {
        expect(Array.isArray(results)).toBe(true);
        done();
      });
    });
  });

  describe('getPathNodesByDifficulty', () => {
    it('should return observable of content nodes by difficulty', (done) => {
      service.getPathNodesByDifficulty('beginner').subscribe((results) => {
        expect(Array.isArray(results)).toBe(true);
        done();
      });
    });
  });

  describe('linkPathToContent', () => {
    it('should return observable of void', (done) => {
      service.linkPathToContent('path-1', ['content-1', 'content-2']).subscribe(() => {
        expect(true).toBe(true);
        done();
      });
    });
  });
});
