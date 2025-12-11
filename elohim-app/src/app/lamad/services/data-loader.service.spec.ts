import { TestBed } from '@angular/core/testing';
import { HttpClientTestingModule, HttpTestingController } from '@angular/common/http/testing';
import { DataLoaderService } from './data-loader.service';
import { LearningPath, PathIndex } from '../models/learning-path.model';
import { ContentNode } from '../models/content-node.model';
import { AgentProgress } from '../models/agent.model';

describe('DataLoaderService', () => {
  let service: DataLoaderService;
  let httpMock: HttpTestingController;
  const basePath = '/assets/lamad-data';

  const mockPath: LearningPath = {
    id: 'test-path',
    version: '1.0.0',
    title: 'Test Path',
    description: 'A test path',
    purpose: 'Testing',
    createdBy: 'test-user',
    contributors: [],
    createdAt: '2025-01-01T00:00:00.000Z',
    updatedAt: '2025-01-01T00:00:00.000Z',
    difficulty: 'beginner',
    estimatedDuration: '1 hour',
    tags: ['test'],
    visibility: 'public',
    steps: []
  };

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

  const mockPathIndex: PathIndex = {
    lastUpdated: '2025-01-01T00:00:00.000Z',
    totalCount: 1,
    paths: [
      {
        id: 'test-path',
        title: 'Test Path',
        description: 'A test path',
        difficulty: 'beginner',
        estimatedDuration: '1 hour',
        stepCount: 0,
        tags: ['test']
      }
    ]
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
    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [DataLoaderService]
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
    it('should load a path', (done) => {
      service.getPath('test-path').subscribe(path => {
        expect(path).toEqual(mockPath);
        done();
      });

      const req = httpMock.expectOne(`${basePath}/paths/test-path.json`);
      expect(req.request.method).toBe('GET');
      req.flush(mockPath);
    });

    it('should cache path requests', (done) => {
      service.getPath('test-path').subscribe();
      service.getPath('test-path').subscribe(path => {
        expect(path).toEqual(mockPath);
        done();
      });

      const req = httpMock.expectOne(`${basePath}/paths/test-path.json`);
      req.flush(mockPath);
    });

    it('should handle path not found error', (done) => {
      service.getPath('missing-path').subscribe({
        error: err => {
          expect(err.message).toContain('Path not found');
          done();
        }
      });

      const req = httpMock.expectOne(`${basePath}/paths/missing-path.json`);
      req.error(new ProgressEvent('error'), { status: 404 });
    });
  });

  describe('getContent', () => {
    it('should load content', (done) => {
      service.getContent('test-content').subscribe(content => {
        expect(content).toEqual(mockContent);
        done();
      });

      const req = httpMock.expectOne(`${basePath}/content/test-content.json`);
      expect(req.request.method).toBe('GET');
      req.flush(mockContent);
    });

    it('should cache content requests', (done) => {
      service.getContent('test-content').subscribe();
      service.getContent('test-content').subscribe(content => {
        expect(content).toEqual(mockContent);
        done();
      });

      const req = httpMock.expectOne(`${basePath}/content/test-content.json`);
      req.flush(mockContent);
    });

    it('should handle content not found error', (done) => {
      service.getContent('missing-content').subscribe({
        error: err => {
          expect(err.message).toContain('Content not found');
          done();
        }
      });

      const req = httpMock.expectOne(`${basePath}/content/missing-content.json`);
      req.error(new ProgressEvent('error'), { status: 404 });
    });
  });

  describe('getPathIndex', () => {
    it('should load path index', (done) => {
      service.getPathIndex().subscribe(index => {
        expect(index).toEqual(mockPathIndex);
        done();
      });

      const req = httpMock.expectOne(`${basePath}/paths/index.json`);
      req.flush(mockPathIndex);
    });

    it('should handle path index load error', (done) => {
      service.getPathIndex().subscribe(index => {
        expect(index.paths).toEqual([]);
        expect(index.totalCount).toBe(0);
        done();
      });

      const req = httpMock.expectOne(`${basePath}/paths/index.json`);
      req.error(new ProgressEvent('error'), { status: 500 });
    });
  });

  describe('getContentIndex', () => {
    it('should load content index', (done) => {
      const mockIndex = { nodes: [] };
      service.getContentIndex().subscribe(index => {
        expect(index).toEqual(mockIndex);
        done();
      });

      const req = httpMock.expectOne(`${basePath}/content/index.json`);
      req.flush(mockIndex);
    });

    it('should handle content index load error', (done) => {
      service.getContentIndex().subscribe(index => {
        expect(index.nodes).toEqual([]);
        done();
      });

      const req = httpMock.expectOne(`${basePath}/content/index.json`);
      req.error(new ProgressEvent('error'), { status: 500 });
    });
  });

  describe('getAgentProgress', () => {
    it('should load agent progress', (done) => {
      service.getAgentProgress('test-agent', 'test-path').subscribe(progress => {
        expect(progress).toEqual(mockProgress);
        done();
      });

      const req = httpMock.expectOne(`${basePath}/progress/test-agent/test-path.json`);
      req.flush(mockProgress);
    });

    it('should return null if progress not found', (done) => {
      service.getAgentProgress('test-agent', 'missing-path').subscribe(progress => {
        expect(progress).toBeNull();
        done();
      });

      const req = httpMock.expectOne(`${basePath}/progress/test-agent/missing-path.json`);
      req.error(new ProgressEvent('error'), { status: 404 });
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
});
