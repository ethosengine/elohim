import { TestBed } from '@angular/core/testing';
import { HttpClientTestingModule, HttpTestingController } from '@angular/common/http/testing';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';
import { HolochainContentService } from '@app/elohim/services/holochain-content.service';
import { LearningPath, PathIndex, ContentNode, AgentProgress } from '../models';
import { of, throwError } from 'rxjs';

describe('DataLoaderService', () => {
  let service: DataLoaderService;
  let httpMock: HttpTestingController;
  let mockHolochainContent: jasmine.SpyObj<HolochainContentService>;
  const basePath = '/assets/lamad-data';

  const mockContent: ContentNode = {
    id: 'test-content',
    title: 'Test Content',
    description: 'Test content node',
    contentType: 'concept',
    contentFormat: 'markdown',
    content: '# Test',
    tags: [],
    relatedNodeIds: [],
    metadata: {}
  };

  const mockProgress: AgentProgress = {
    agentId: 'test-agent',
    pathId: 'test-path',
    currentStepIndex: 0,
    completedStepIndices: [],
    startedAt: '2025-01-01T00:00:00.000Z',
    lastActivityAt: '2025-01-01T00:00:00.000Z',
    stepAffinity: {},
    stepNotes: {},
    reflectionResponses: {},
    attestationsEarned: []
  };

  beforeEach(() => {
    mockHolochainContent = jasmine.createSpyObj('HolochainContentService', [
      'getContent',
      'getContentByType',
      'getStats',
      'clearCache',
      'isAvailable',
      'getPathIndex',
      'getPathWithSteps'
    ]);

    // Default mock returns
    mockHolochainContent.getStats.and.returnValue(of({ total_count: 0, by_type: {} }));
    mockHolochainContent.isAvailable.and.returnValue(true);
    (mockHolochainContent.getPathIndex as jasmine.Spy).and.returnValue(Promise.resolve({ paths: [], total_count: 0, last_updated: '' }));
    (mockHolochainContent.getPathWithSteps as jasmine.Spy).and.returnValue(Promise.resolve(null));

    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [
        DataLoaderService,
        { provide: HolochainContentService, useValue: mockHolochainContent }
      ]
    });

    service = TestBed.inject(DataLoaderService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('getPath', () => {
    it('should load path from Holochain', (done) => {
      const mockHcPath = {
        action_hash: new Uint8Array(),
        path: {
          id: 'test-path',
          version: '1.0.0',
          title: 'Test Path',
          description: 'A test path',
          purpose: 'Testing',
          created_by: 'test-agent',
          difficulty: 'beginner',
          estimated_duration: '1 hour',
          visibility: 'public',
          path_type: 'journey',
          tags: ['test'],
          created_at: '2025-01-01T00:00:00.000Z',
          updated_at: '2025-01-01T00:00:00.000Z',
        },
        steps: [],
      };

      (mockHolochainContent.getPathWithSteps as jasmine.Spy).and.returnValue(Promise.resolve(mockHcPath));

      service.getPath('test-path').subscribe(path => {
        expect(path.id).toBe('test-path');
        expect(path.title).toBe('Test Path');
        done();
      });
    });
  });

  describe('getContent', () => {
    it('should load content from Holochain', (done) => {
      mockHolochainContent.getContent.and.returnValue(of(mockContent));

      service.getContent('test-content').subscribe(content => {
        expect(content).toEqual(mockContent);
        expect(mockHolochainContent.getContent).toHaveBeenCalledWith('test-content');
        done();
      });
    });

    it('should cache content requests', (done) => {
      mockHolochainContent.getContent.and.returnValue(of(mockContent));

      service.getContent('test-content').subscribe();
      service.getContent('test-content').subscribe(content => {
        expect(content).toEqual(mockContent);
        // Should only call Holochain once due to caching
        expect(mockHolochainContent.getContent).toHaveBeenCalledTimes(1);
        done();
      });
    });

    it('should handle content not found error', (done) => {
      mockHolochainContent.getContent.and.returnValue(of(null));

      service.getContent('missing-content').subscribe({
        error: err => {
          expect(err.message).toContain('Content not found');
          done();
        }
      });
    });
  });

  describe('getPathIndex', () => {
    it('should load path index from Holochain', (done) => {
      const mockHcPathIndex = {
        paths: [
          { id: 'test-path', title: 'Test', description: 'Desc', difficulty: 'beginner', estimated_duration: '1h', step_count: 3, tags: [] }
        ],
        total_count: 1,
        last_updated: '2025-01-01T00:00:00.000Z',
      };

      (mockHolochainContent.getPathIndex as jasmine.Spy).and.returnValue(Promise.resolve(mockHcPathIndex));

      service.getPathIndex().subscribe(index => {
        expect(index.totalCount).toBe(1);
        expect(index.paths.length).toBe(1);
        expect(index.paths[0].id).toBe('test-path');
        done();
      });
    });
  });

  describe('getContentIndex', () => {
    it('should return stats from Holochain', (done) => {
      mockHolochainContent.getStats.and.returnValue(of({ total_count: 5, by_type: { concept: 3, exercise: 2 } }));

      service.getContentIndex().subscribe(index => {
        expect(index.totalCount).toBe(5);
        expect(index.byType).toEqual({ concept: 3, exercise: 2 });
        done();
      });
    });
  });

  describe('getAgentProgress', () => {
    it('should load agent progress from localStorage', (done) => {
      const progressJson = JSON.stringify(mockProgress);
      spyOn(localStorage, 'getItem').and.returnValue(progressJson);

      service.getAgentProgress('test-agent', 'test-path').subscribe(progress => {
        expect(progress).toEqual(mockProgress);
        expect(localStorage.getItem).toHaveBeenCalledWith('lamad-progress-test-agent-test-path');
        done();
      });
    });

    it('should return null if progress not in localStorage', (done) => {
      spyOn(localStorage, 'getItem').and.returnValue(null);

      service.getAgentProgress('test-agent', 'missing-path').subscribe(progress => {
        expect(progress).toBeNull();
        done();
      });
    });
  });

  describe('saveAgentProgress', () => {
    it('should save progress to localStorage', (done) => {
      spyOn(localStorage, 'setItem');

      service.saveAgentProgress(mockProgress).subscribe(() => {
        expect(localStorage.setItem).toHaveBeenCalledWith(
          'lamad-progress-test-agent-test-path',
          jasmine.any(String)
        );
        done();
      });
    });

    it('should handle localStorage errors gracefully', (done) => {
      spyOn(localStorage, 'setItem').and.throwError('QuotaExceededError');

      service.saveAgentProgress(mockProgress).subscribe(() => {
        expect(true).toBe(true); // Should complete without error
        done();
      });
    });
  });

  describe('getLocalProgress', () => {
    it('should retrieve progress from localStorage', () => {
      const progressJson = JSON.stringify(mockProgress);
      spyOn(localStorage, 'getItem').and.returnValue(progressJson);

      const result = service.getLocalProgress('test-agent', 'test-path');
      expect(result).toEqual(mockProgress);
      expect(localStorage.getItem).toHaveBeenCalledWith('lamad-progress-test-agent-test-path');
    });

    it('should return null if no progress in localStorage', () => {
      spyOn(localStorage, 'getItem').and.returnValue(null);

      const result = service.getLocalProgress('test-agent', 'test-path');
      expect(result).toBeNull();
    });

    it('should return null for malformed JSON', () => {
      spyOn(localStorage, 'getItem').and.returnValue('invalid json');

      const result = service.getLocalProgress('test-agent', 'test-path');
      expect(result).toBeNull();
    });
  });

  describe('clearCache', () => {
    it('should clear Holochain content cache', () => {
      service.clearCache();
      expect(mockHolochainContent.clearCache).toHaveBeenCalled();
    });
  });
});
