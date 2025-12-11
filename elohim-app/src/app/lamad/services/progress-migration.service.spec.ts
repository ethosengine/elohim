import { TestBed } from '@angular/core/testing';
import { of, throwError } from 'rxjs';
import { ProgressMigrationService } from './progress-migration.service';
import { DataLoaderService } from './data-loader.service';
import { AgentProgress } from '../models/agent.model';
import { LearningPath, PathStep } from '../models/learning-path.model';

describe('ProgressMigrationService', () => {
  let service: ProgressMigrationService;
  let dataLoaderSpy: jasmine.SpyObj<DataLoaderService>;
  let localStorageMock: { [key: string]: string };
  let mockStorage: Storage;

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
    steps: [
      { resourceId: 'resource-1', stepTitle: 'Step 1', stepNarrative: 'First step' },
      { resourceId: 'resource-2', stepTitle: 'Step 2', stepNarrative: 'Second step' },
      { resourceId: 'resource-3', stepTitle: 'Step 3', stepNarrative: 'Third step' }
    ] as PathStep[]
  };

  const mockProgress1: AgentProgress = {
    agentId: 'agent-1',
    pathId: 'test-path',
    currentStepIndex: 2,
    completedStepIndices: [0, 1],
    startedAt: '2025-01-01T00:00:00.000Z',
    lastActivityAt: '2025-01-02T00:00:00.000Z',
    stepAffinity: {},
    stepNotes: {},
    reflectionResponses: {},
    attestationsEarned: []
  };

  const mockProgress2: AgentProgress = {
    agentId: 'agent-1',
    pathId: 'another-path',
    currentStepIndex: 1,
    completedStepIndices: [0],
    startedAt: '2025-01-01T00:00:00.000Z',
    lastActivityAt: '2025-01-02T00:00:00.000Z',
    stepAffinity: {},
    stepNotes: {},
    reflectionResponses: {},
    attestationsEarned: []
  };

  const mockProgress3: AgentProgress = {
    agentId: 'agent-2',
    pathId: 'test-path',
    currentStepIndex: 1,
    completedStepIndices: [0],
    startedAt: '2025-01-03T00:00:00.000Z',
    lastActivityAt: '2025-01-03T00:00:00.000Z',
    stepAffinity: {},
    stepNotes: {},
    reflectionResponses: {},
    attestationsEarned: []
  };

  beforeEach(() => {
    const dataLoaderSpyObj = jasmine.createSpyObj('DataLoaderService', [
      'getPath',
      'getLocalProgress',
      'saveAgentProgress'
    ]);

    // Mock localStorage
    localStorageMock = {};
    mockStorage = {
      getItem: (key: string) => localStorageMock[key] || null,
      setItem: (key: string, value: string) => { localStorageMock[key] = value; },
      removeItem: (key: string) => { delete localStorageMock[key]; },
      key: (index: number) => Object.keys(localStorageMock)[index] || null,
      get length() { return Object.keys(localStorageMock).length; },
      clear: () => { localStorageMock = {}; }
    };
    spyOnProperty(window, 'localStorage', 'get').and.returnValue(mockStorage);

    TestBed.configureTestingModule({
      providers: [
        ProgressMigrationService,
        { provide: DataLoaderService, useValue: dataLoaderSpyObj }
      ]
    });

    dataLoaderSpy = TestBed.inject(DataLoaderService) as jasmine.SpyObj<DataLoaderService>;
    dataLoaderSpy.getPath.and.returnValue(of(mockPath));
    dataLoaderSpy.getLocalProgress.and.returnValue(null);
    dataLoaderSpy.saveAgentProgress.and.returnValue(of(undefined));

    service = TestBed.inject(ProgressMigrationService);
  });

  afterEach(() => {
    localStorageMock = {};
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  // =========================================================================
  // migrateAllProgress
  // =========================================================================

  describe('migrateAllProgress', () => {
    it('should return empty stats when no progress records exist', (done) => {
      service.migrateAllProgress().subscribe(result => {
        expect(result.agentsMigrated).toBe(0);
        expect(result.pathsMigrated).toBe(0);
        expect(result.contentNodesMigrated).toBe(0);
        expect(result.errors.length).toBe(1);
        expect(result.errors[0]).toContain('No progress records found');
        done();
      });
    });

    it('should migrate progress for single agent', (done) => {
      // Setup localStorage with progress record
      localStorageMock['lamad-progress-agent-1-test-path'] = JSON.stringify(mockProgress1);

      service.migrateAllProgress().subscribe(result => {
        expect(result.agentsMigrated).toBe(1);
        expect(result.pathsMigrated).toBe(1);
        expect(result.contentNodesMigrated).toBe(2); // Two completed steps
        expect(dataLoaderSpy.saveAgentProgress).toHaveBeenCalled();
        done();
      });
    });

    it('should migrate progress for multiple agents', (done) => {
      localStorageMock['lamad-progress-agent-1-test-path'] = JSON.stringify(mockProgress1);
      localStorageMock['lamad-progress-agent-2-test-path'] = JSON.stringify(mockProgress3);

      service.migrateAllProgress().subscribe(result => {
        expect(result.agentsMigrated).toBe(2);
        expect(result.pathsMigrated).toBe(2);
        done();
      });
    });

    it('should migrate multiple paths for same agent', (done) => {
      localStorageMock['lamad-progress-agent-1-test-path'] = JSON.stringify(mockProgress1);
      localStorageMock['lamad-progress-agent-1-another-path'] = JSON.stringify(mockProgress2);

      const anotherPath = { ...mockPath, id: 'another-path' };
      dataLoaderSpy.getPath.and.callFake((pathId: string) => {
        if (pathId === 'another-path') return of(anotherPath);
        return of(mockPath);
      });

      service.migrateAllProgress().subscribe(result => {
        expect(result.agentsMigrated).toBe(1);
        expect(result.pathsMigrated).toBe(2);
        done();
      });
    });

    it('should handle errors loading paths gracefully', (done) => {
      localStorageMock['lamad-progress-agent-1-test-path'] = JSON.stringify(mockProgress1);
      dataLoaderSpy.getPath.and.returnValue(throwError(() => new Error('Path not found')));

      service.migrateAllProgress().subscribe(result => {
        expect(result.agentsMigrated).toBe(1);
        expect(result.contentNodesMigrated).toBe(0); // Nothing migrated due to error
        done();
      });
    });

    it('should skip malformed localStorage entries', (done) => {
      localStorageMock['lamad-progress-agent-1-test-path'] = 'invalid json';
      localStorageMock['lamad-progress-agent-2-test-path'] = JSON.stringify(mockProgress3);

      service.migrateAllProgress().subscribe(result => {
        expect(result.agentsMigrated).toBe(1); // Only agent-2 migrated
        done();
      });
    });

    it('should merge with existing global progress', (done) => {
      localStorageMock['lamad-progress-agent-1-test-path'] = JSON.stringify(mockProgress1);

      const existingGlobal: AgentProgress = {
        agentId: 'agent-1',
        pathId: '__global__',
        currentStepIndex: 0,
        completedStepIndices: [],
        startedAt: '2025-01-01T00:00:00.000Z',
        lastActivityAt: '2025-01-01T00:00:00.000Z',
        stepAffinity: {},
        stepNotes: {},
        reflectionResponses: {},
        attestationsEarned: [],
        completedContentIds: ['existing-resource']
      };
      dataLoaderSpy.getLocalProgress.and.returnValue(existingGlobal);

      service.migrateAllProgress().subscribe(() => {
        const savedProgress = dataLoaderSpy.saveAgentProgress.calls.mostRecent().args[0];
        expect(savedProgress.completedContentIds).toContain('existing-resource');
        expect(savedProgress.completedContentIds).toContain('resource-1');
        done();
      });
    });
  });

  // =========================================================================
  // previewMigration
  // =========================================================================

  describe('previewMigration', () => {
    it('should return empty preview when no progress', (done) => {
      service.previewMigration().subscribe(preview => {
        expect(preview.totalAgents).toBe(0);
        expect(preview.totalPaths).toBe(0);
        expect(preview.estimatedContentNodes).toBe(0);
        done();
      });
    });

    it('should preview migration stats', (done) => {
      localStorageMock['lamad-progress-agent-1-test-path'] = JSON.stringify(mockProgress1);
      localStorageMock['lamad-progress-agent-2-test-path'] = JSON.stringify(mockProgress3);

      service.previewMigration().subscribe(preview => {
        expect(preview.totalAgents).toBe(2);
        expect(preview.totalPaths).toBe(2);
        expect(preview.agents.length).toBe(2);
        done();
      });
    });

    it('should skip existing __global__ records in preview', (done) => {
      const globalProgress: AgentProgress = {
        ...mockProgress1,
        pathId: '__global__'
      };
      localStorageMock['lamad-progress-agent-1-__global__'] = JSON.stringify(globalProgress);
      localStorageMock['lamad-progress-agent-1-test-path'] = JSON.stringify(mockProgress1);

      service.previewMigration().subscribe(preview => {
        expect(preview.totalPaths).toBe(1); // Only test-path, not __global__
        done();
      });
    });

    it('should estimate content nodes from completed steps', (done) => {
      localStorageMock['lamad-progress-agent-1-test-path'] = JSON.stringify(mockProgress1);

      service.previewMigration().subscribe(preview => {
        const agent1 = preview.agents.find(a => a.agentId === 'agent-1');
        expect(agent1?.estimatedContentNodes).toBe(2); // Two completed step indices
        done();
      });
    });
  });

  // =========================================================================
  // verifyMigration
  // =========================================================================

  describe('verifyMigration', () => {
    it('should return valid when all agents have global progress', (done) => {
      localStorageMock['lamad-progress-agent-1-test-path'] = JSON.stringify(mockProgress1);
      localStorageMock['lamad-progress-agent-1-__global__'] = JSON.stringify({
        ...mockProgress1,
        pathId: '__global__'
      });

      service.verifyMigration().subscribe(result => {
        expect(result.valid).toBe(true);
        expect(result.missingGlobalProgress.length).toBe(0);
        done();
      });
    });

    it('should return invalid when some agents missing global progress', (done) => {
      localStorageMock['lamad-progress-agent-1-test-path'] = JSON.stringify(mockProgress1);
      // No __global__ record for agent-1

      service.verifyMigration().subscribe(result => {
        expect(result.valid).toBe(false);
        expect(result.missingGlobalProgress).toContain('agent-1');
        done();
      });
    });

    it('should count agents correctly', (done) => {
      localStorageMock['lamad-progress-agent-1-test-path'] = JSON.stringify(mockProgress1);
      localStorageMock['lamad-progress-agent-1-__global__'] = JSON.stringify({
        ...mockProgress1,
        pathId: '__global__'
      });
      localStorageMock['lamad-progress-agent-2-test-path'] = JSON.stringify(mockProgress3);

      service.verifyMigration().subscribe(result => {
        expect(result.agentsWithProgress).toBe(2);
        expect(result.agentsWithGlobalProgress).toBe(1);
        expect(result.missingGlobalProgress).toContain('agent-2');
        done();
      });
    });
  });
});
