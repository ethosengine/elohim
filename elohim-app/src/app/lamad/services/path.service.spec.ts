import { TestBed } from '@angular/core/testing';
import { of } from 'rxjs';
import { PathService } from './path.service';
import { DataLoaderService } from './data-loader.service';
import { SessionUserService } from './session-user.service';
import { LearningPath } from '../models/learning-path.model';

describe('PathService', () => {
  let service: PathService;
  let dataLoaderMock: jasmine.SpyObj<DataLoaderService>;
  let sessionUserMock: jasmine.SpyObj<SessionUserService>;

  const mockPath: LearningPath = {
    id: 'test-path',
    title: 'Test Learning Path',
    description: 'A test path for learning',
    steps: [
      { nodeId: 'node-1', stepIndex: 0, narrative: 'Introduction' },
      { nodeId: 'node-2', stepIndex: 1, narrative: 'Getting Started' },
      { nodeId: 'node-3', stepIndex: 2, narrative: 'Advanced Topics' }
    ],
    metadata: {
      difficulty: 'beginner',
      estimatedTime: '30 minutes'
    }
  };

  const mockContent = {
    id: 'node-1',
    title: 'Introduction',
    description: 'An introduction',
    contentType: 'concept',
    contentFormat: 'markdown',
    content: '# Introduction',
    tags: [],
    relatedNodeIds: [],
    metadata: {}
  };

  beforeEach(() => {
    dataLoaderMock = jasmine.createSpyObj('DataLoaderService', [
      'getPaths',
      'getContent'
    ]);
    sessionUserMock = jasmine.createSpyObj('SessionUserService', [
      'getPathProgress',
      'updatePathProgress',
      'recordPathStart',
      'recordPathCompletion'
    ]);

    dataLoaderMock.getPaths.and.returnValue(of([mockPath]));
    dataLoaderMock.getContent.and.returnValue(of(mockContent));
    sessionUserMock.getPathProgress.and.returnValue({
      pathId: 'test-path',
      currentStepIndex: 0,
      completedStepIndices: [],
      startedAt: new Date().toISOString()
    });

    TestBed.configureTestingModule({
      providers: [
        PathService,
        { provide: DataLoaderService, useValue: dataLoaderMock },
        { provide: SessionUserService, useValue: sessionUserMock }
      ]
    });
    service = TestBed.inject(PathService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('getPaths', () => {
    it('should return all paths', (done) => {
      service.getPaths().subscribe(paths => {
        expect(paths.length).toBe(1);
        expect(paths[0].id).toBe('test-path');
        done();
      });
    });
  });

  describe('getPath', () => {
    it('should return path by ID', (done) => {
      service.getPath('test-path').subscribe(path => {
        expect(path).toBeTruthy();
        expect(path?.id).toBe('test-path');
        expect(path?.title).toBe('Test Learning Path');
        done();
      });
    });

    it('should return undefined for nonexistent path', (done) => {
      service.getPath('nonexistent').subscribe(path => {
        expect(path).toBeUndefined();
        done();
      });
    });
  });

  describe('getPathStep', () => {
    it('should return step content', (done) => {
      service.getPathStep('test-path', 0).subscribe(step => {
        expect(step).toBeTruthy();
        expect(step?.content.id).toBe('node-1');
        done();
      });
    });

    it('should include path and step metadata', (done) => {
      service.getPathStep('test-path', 0).subscribe(step => {
        expect(step?.path).toBeTruthy();
        expect(step?.stepIndex).toBe(0);
        expect(step?.narrative).toBe('Introduction');
        done();
      });
    });

    it('should return undefined for invalid step index', (done) => {
      service.getPathStep('test-path', 99).subscribe(step => {
        expect(step).toBeUndefined();
        done();
      });
    });
  });

  describe('getPathProgress', () => {
    it('should return current progress', () => {
      const progress = service.getPathProgress('test-path');
      expect(progress).toBeTruthy();
      expect(progress.pathId).toBe('test-path');
      expect(progress.currentStepIndex).toBe(0);
    });
  });

  describe('advanceStep', () => {
    it('should advance to next step', (done) => {
      service.advanceStep('test-path').subscribe(result => {
        expect(sessionUserMock.updatePathProgress).toHaveBeenCalled();
        done();
      });
    });
  });

  describe('goToStep', () => {
    it('should go to specific step', (done) => {
      service.goToStep('test-path', 2).subscribe(result => {
        expect(sessionUserMock.updatePathProgress).toHaveBeenCalledWith('test-path', 2);
        done();
      });
    });
  });

  describe('startPath', () => {
    it('should record path start', (done) => {
      service.startPath('test-path').subscribe(() => {
        expect(sessionUserMock.recordPathStart).toHaveBeenCalledWith('test-path');
        done();
      });
    });
  });

  describe('completePath', () => {
    it('should record path completion', (done) => {
      service.completePath('test-path').subscribe(() => {
        expect(sessionUserMock.recordPathCompletion).toHaveBeenCalledWith('test-path');
        done();
      });
    });
  });

  describe('getPathStepCount', () => {
    it('should return step count', (done) => {
      service.getPathStepCount('test-path').subscribe(count => {
        expect(count).toBe(3);
        done();
      });
    });
  });

  describe('isPathComplete', () => {
    it('should return false for incomplete path', () => {
      const result = service.isPathComplete('test-path');
      expect(result).toBe(false);
    });
  });
});
