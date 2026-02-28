import { TestBed } from '@angular/core/testing';
import { of, throwError } from 'rxjs';
import { PathService, AccessCheckResult } from './path.service';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';
import { AgentService } from '@app/elohim/services/agent.service';
import { ContentMasteryService } from './content-mastery.service';
import { LearnerContextService } from './learner-context.service';
import { LearningPath, PathStep, PathStepView, PathIndex, ContentNode } from '../models';
import { AgentProgress } from '@app/elohim/models/agent.model';
import { vi, Mock } from 'vitest';

describe('PathService', () => {
  let service: PathService;
  let dataLoaderSpy: any;
  let agentServiceSpy: any;
  let contentMasterySpy: any;
  let learnerContextSpy: any;

  const mockPath: LearningPath = {
    id: 'test-path',
    version: '1.0.0',
    title: 'Test Path',
    description: 'A test learning path',
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
      {
        order: 0,
        resourceId: 'content-1',
        stepTitle: 'Step 1',
        stepNarrative: 'First step',
        learningObjectives: [],
        optional: false,
        completionCriteria: [],
      },
      {
        order: 1,
        resourceId: 'content-2',
        stepTitle: 'Step 2',
        stepNarrative: 'Second step',
        learningObjectives: [],
        optional: false,
        completionCriteria: [],
      },
      {
        order: 2,
        resourceId: 'content-3',
        stepTitle: 'Step 3',
        stepNarrative: 'Third step',
        learningObjectives: [],
        optional: true,
        completionCriteria: [],
      },
      {
        order: 3,
        resourceId: 'content-4',
        stepTitle: 'Step 4',
        stepNarrative: 'Fourth step',
        learningObjectives: [],
        optional: false,
        completionCriteria: [],
      },
      {
        order: 4,
        resourceId: 'content-5',
        stepTitle: 'Step 5 (Attestation Required)',
        stepNarrative: 'Fifth step',
        learningObjectives: [],
        optional: false,
        completionCriteria: [],
        attestationRequired: 'test-attestation',
      },
    ],
  };

  const mockContent: ContentNode = {
    id: 'content-1',
    title: 'Test Content',
    description: 'Test content node',
    contentType: 'concept',
    contentFormat: 'markdown',
    content: '# Test Content',
    tags: [],
    relatedNodeIds: [],
    metadata: {},
  };

  const mockProgress: AgentProgress = {
    agentId: 'test-agent',
    pathId: 'test-path',
    currentStepIndex: 1,
    completedStepIndices: [0],
    startedAt: '2025-01-01T00:00:00.000Z',
    lastActivityAt: '2025-01-01T00:00:00.000Z',
    stepAffinity: { 0: 0.8 },
    stepNotes: { 0: 'Great intro!' },
    reflectionResponses: {},
    attestationsEarned: [],
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
        stepCount: 4,
        tags: ['test'],
      },
    ],
  };

  beforeEach(() => {
    const dataLoaderSpyObj = {
    getPath: vi.fn(),
    getContent: vi.fn(),
    getPathIndex: vi.fn(),
  };
    const agentServiceSpyObj = {
    getProgressForPath: vi.fn(),
    getAttestations: vi.fn(),
  };
    const contentMasterySpyObj = {
    getMasteryLevelSync: vi.fn(),
  };
    const learnerContextSpyObj = {
    isMasteryUnlocked: vi.fn(),
    isInSkippedSection: vi.fn(),
    getDiscoveryProfile: vi.fn(),
  };

    TestBed.configureTestingModule({
      providers: [
        PathService,
        { provide: DataLoaderService, useValue: dataLoaderSpyObj },
        { provide: AgentService, useValue: agentServiceSpyObj },
        { provide: ContentMasteryService, useValue: contentMasterySpyObj },
        { provide: LearnerContextService, useValue: learnerContextSpyObj },
      ],
    });

    service = TestBed.inject(PathService);
    dataLoaderSpy = TestBed.inject(DataLoaderService) as { [K in keyof DataLoaderService]?: Mock };
    agentServiceSpy = TestBed.inject(AgentService) as { [K in keyof AgentService]?: Mock };
    contentMasterySpy = TestBed.inject(
      ContentMasteryService
    ) as { [K in keyof ContentMasteryService]?: Mock };
    learnerContextSpy = TestBed.inject(
      LearnerContextService
    ) as { [K in keyof LearnerContextService]?: Mock };

    // Default spy return values
    dataLoaderSpy.getPath.mockReturnValue(of(mockPath));
    dataLoaderSpy.getContent.mockReturnValue(of(mockContent));
    dataLoaderSpy.getPathIndex.mockReturnValue(of(mockPathIndex));
    agentServiceSpy.getProgressForPath.mockReturnValue(of(mockProgress));
    agentServiceSpy.getAttestations.mockReturnValue([]);
    contentMasterySpy.getMasteryLevelSync.mockReturnValue('not_started');
    learnerContextSpy.isMasteryUnlocked.mockReturnValue(false);
    learnerContextSpy.isInSkippedSection.mockReturnValue(false);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('getPath', () => {
    it('should get path metadata', () => new Promise<void>(done => {
      service.getPath('test-path').subscribe(path => {
        expect(path).toEqual(mockPath);
        expect(dataLoaderSpy.getPath).toHaveBeenCalledWith('test-path');
        done();
      });
    }));

    it('should handle path load error', () => new Promise<void>(done => {
      dataLoaderSpy.getPath.mockReturnValue(throwError(() => new Error('Load error')));

      service.getPath('test-path').subscribe({
        error: err => {
          expect(err.message).toBe('Load error');
          done();
        },
      });
    }));
  });

  describe('getPathStep', () => {
    it('should get step with resolved content', () => new Promise<void>(done => {
      service.getPathStep('test-path', 0).subscribe(stepView => {
        expect(stepView.step).toEqual(mockPath.steps[0]);
        expect(stepView.content).toEqual(mockContent);
        expect(stepView.hasPrevious).toBe(false);
        expect(stepView.hasNext).toBe(true);
        expect(stepView.previousStepIndex).toBeUndefined();
        expect(stepView.nextStepIndex).toBe(1);
        expect(stepView.isCompleted).toBe(true);
        expect(stepView.affinity).toBe(0.8);
        expect(stepView.notes).toBe('Great intro!');
        done();
      });
    }));

    it('should handle middle step navigation', () => new Promise<void>(done => {
      service.getPathStep('test-path', 1).subscribe(stepView => {
        expect(stepView.hasPrevious).toBe(true);
        expect(stepView.hasNext).toBe(true);
        expect(stepView.previousStepIndex).toBe(0);
        expect(stepView.nextStepIndex).toBe(2);
        expect(stepView.isCompleted).toBe(false);
        done();
      });
    }));

    it('should handle last step navigation', () => new Promise<void>(done => {
      service.getPathStep('test-path', 4).subscribe(stepView => {
        expect(stepView.hasPrevious).toBe(true);
        expect(stepView.hasNext).toBe(false);
        expect(stepView.previousStepIndex).toBe(3);
        expect(stepView.nextStepIndex).toBeUndefined();
        done();
      });
    }));

    it('should handle step with no progress', () => new Promise<void>(done => {
      agentServiceSpy.getProgressForPath.mockReturnValue(of(null as any));

      service.getPathStep('test-path', 0).subscribe(stepView => {
        expect(stepView.isCompleted).toBe(false);
        expect(stepView.affinity).toBeUndefined();
        expect(stepView.notes).toBeUndefined();
        done();
      });
    }));

    it('should throw error for invalid step index (negative)', () => new Promise<void>(done => {
      service.getPathStep('test-path', -1).subscribe({
        error: err => {
          expect(err.message).toContain('out of range');
          done();
        },
      });
    }));

    it('should throw error for invalid step index (too large)', () => new Promise<void>(done => {
      service.getPathStep('test-path', 10).subscribe({
        error: err => {
          expect(err.message).toContain('out of range');
          done();
        },
      });
    }));

    it('should load correct content for step', () => new Promise<void>(done => {
      service.getPathStep('test-path', 1).subscribe(() => {
        expect(dataLoaderSpy.getContent).toHaveBeenCalledWith('content-2');
        done();
      });
    }));
  });

  describe('listPaths', () => {
    it('should list all available paths', () => new Promise<void>(done => {
      service.listPaths().subscribe(index => {
        expect(index).toEqual(mockPathIndex);
        expect(dataLoaderSpy.getPathIndex).toHaveBeenCalled();
        done();
      });
    }));
  });

  describe('isStepAccessible', () => {
    it('should allow access to step 0 with no progress', () => {
      const result = service.isStepAccessible(mockPath, 0, null, []);
      expect(result.accessible).toBe(true);
    });

    it('should deny access to step 1 with no progress', () => {
      const result = service.isStepAccessible(mockPath, 1, null, []);
      expect(result.accessible).toBe(false);
      expect(result.reason).toBe('Start from the beginning');
    });

    it('should allow access to completed steps', () => {
      const result = service.isStepAccessible(mockPath, 0, mockProgress, []);
      expect(result.accessible).toBe(true);
    });

    it('should allow access to current step', () => {
      const result = service.isStepAccessible(mockPath, 1, mockProgress, []);
      expect(result.accessible).toBe(true);
    });

    it('should allow access to one step ahead', () => {
      const result = service.isStepAccessible(mockPath, 2, mockProgress, []);
      expect(result.accessible).toBe(true);
    });

    it('should deny access to steps too far ahead', () => {
      const result = service.isStepAccessible(mockPath, 3, mockProgress, []);
      expect(result.accessible).toBe(false);
      expect(result.reason).toBe('Complete previous steps first');
    });

    it('should deny access for invalid step index (negative)', () => {
      const result = service.isStepAccessible(mockPath, -1, mockProgress, []);
      expect(result.accessible).toBe(false);
      expect(result.reason).toBe('Invalid step index');
    });

    it('should deny access for invalid step index (too large)', () => {
      const result = service.isStepAccessible(mockPath, 10, mockProgress, []);
      expect(result.accessible).toBe(false);
      expect(result.reason).toBe('Invalid step index');
    });

    it('should deny access if attestation required but not present', () => {
      const progressAtStep4: AgentProgress = {
        ...mockProgress,
        currentStepIndex: 4,
        completedStepIndices: [0, 1, 2, 3],
      };
      const result = service.isStepAccessible(mockPath, 4, progressAtStep4, []);
      expect(result.accessible).toBe(false);
      expect(result.reason).toContain('Requires attestation');
    });

    it('should allow access if attestation required and present', () => {
      const progressAtStep4: AgentProgress = {
        ...mockProgress,
        currentStepIndex: 4,
        completedStepIndices: [0, 1, 2, 3],
      };
      const result = service.isStepAccessible(mockPath, 4, progressAtStep4, ['test-attestation']);
      expect(result.accessible).toBe(true);
    });

    // =========================================================================
    // Path Adaptation: Mastery-Aware Fog-of-War
    // =========================================================================

    it('should unlock step via prior mastery when content is mastered', () => {
      learnerContextSpy.isMasteryUnlocked.mockReturnValue(true);
      // Step 3 is normally locked (only 0,1,2 accessible with progress at step 0)
      const result = service.isStepAccessible(mockPath, 3, mockProgress, []);
      expect(result.accessible).toBe(true);
      expect(result.accessType).toBe('mastery-unlocked');
      expect(result.reason).toBe('Prior mastery');
    });

    it('should NOT bypass attestation gate even with mastery unlock', () => {
      learnerContextSpy.isMasteryUnlocked.mockReturnValue(true);
      const progressAtStep4 = {
        ...mockProgress,
        currentStepIndex: 4,
        completedStepIndices: [0, 1, 2, 3],
      };
      // Step 4 requires 'test-attestation' — mastery should not bypass it
      const result = service.isStepAccessible(mockPath, 4, progressAtStep4, []);
      expect(result.accessible).toBe(false);
      expect(result.reason).toContain('Requires attestation');
    });

    it('should return mastery-unlocked for scattered mastery with no progress', () => {
      learnerContextSpy.isMasteryUnlocked.mockImplementation((resourceId: string | undefined) =>
        resourceId === 'content-4' ? true : false
      );
      // No progress, but step 3 content is mastered
      const result = service.isStepAccessible(mockPath, 3, null, []);
      expect(result.accessible).toBe(true);
      expect(result.accessType).toBe('mastery-unlocked');
    });

    it('should return sequential accessType for normal progression', () => {
      const result = service.isStepAccessible(mockPath, 0, null, []);
      expect(result.accessible).toBe(true);
      expect(result.accessType).toBe('sequential');
    });
  });

  describe('checkStepAccess', () => {
    it('should check step access using current agent state', () => new Promise<void>(done => {
      service.checkStepAccess('test-path', 0).subscribe(result => {
        expect(result.accessible).toBe(true);
        expect(agentServiceSpy.getProgressForPath).toHaveBeenCalledWith('test-path');
        expect(agentServiceSpy.getAttestations).toHaveBeenCalled();
        done();
      });
    }));

    it('should deny access to locked step', () => new Promise<void>(done => {
      service.checkStepAccess('test-path', 3).subscribe(result => {
        expect(result.accessible).toBe(false);
        done();
      });
    }));
  });

  describe('getAccessibleSteps', () => {
    it('should return all accessible step indices', () => new Promise<void>(done => {
      service.getAccessibleSteps('test-path').subscribe(steps => {
        expect(steps).toEqual([0, 1, 2]);
        done();
      });
    }));

    it('should return only step 0 with no progress', () => new Promise<void>(done => {
      agentServiceSpy.getProgressForPath.mockReturnValue(of(null as any));

      service.getAccessibleSteps('test-path').subscribe(steps => {
        expect(steps).toEqual([0]);
        done();
      });
    }));

    it('should include step with attestation if agent has it', () => new Promise<void>(done => {
      const fullProgress: AgentProgress = {
        ...mockProgress,
        completedStepIndices: [0, 1, 2, 3],
      };
      agentServiceSpy.getProgressForPath.mockReturnValue(of(fullProgress));
      agentServiceSpy.getAttestations.mockReturnValue(['test-attestation']);

      service.getAccessibleSteps('test-path').subscribe(steps => {
        expect(steps).toEqual([0, 1, 2, 3, 4]);
        done();
      });
    }));

    it('should exclude step with attestation if agent lacks it', () => new Promise<void>(done => {
      const fullProgress: AgentProgress = {
        ...mockProgress,
        completedStepIndices: [0, 1, 2, 3],
      };
      agentServiceSpy.getProgressForPath.mockReturnValue(of(fullProgress));
      agentServiceSpy.getAttestations.mockReturnValue([]);

      service.getAccessibleSteps('test-path').subscribe(steps => {
        expect(steps).toEqual([0, 1, 2, 3]);
        done();
      });
    }));
  });

  describe('getStepCount', () => {
    it('should return total number of steps', () => new Promise<void>(done => {
      service.getStepCount('test-path').subscribe(count => {
        expect(count).toBe(5);
        done();
      });
    }));
  });

  describe('getCompletionPercentage', () => {
    it('should calculate completion percentage', () => new Promise<void>(done => {
      service.getCompletionPercentage('test-path').subscribe(percentage => {
        // 1 completed out of 4 required steps (step 2 is optional) = 25%
        expect(percentage).toBe(25);
        done();
      });
    }));

    it('should return 0 with no progress', () => new Promise<void>(done => {
      agentServiceSpy.getProgressForPath.mockReturnValue(of(null as any));

      service.getCompletionPercentage('test-path').subscribe(percentage => {
        expect(percentage).toBe(0);
        done();
      });
    }));

    it('should return 100 when all required steps completed', () => new Promise<void>(done => {
      const completeProgress: AgentProgress = {
        ...mockProgress,
        completedStepIndices: [0, 1, 3, 4], // All required steps (excluding optional step 2)
      };
      agentServiceSpy.getProgressForPath.mockReturnValue(of(completeProgress));

      service.getCompletionPercentage('test-path').subscribe(percentage => {
        expect(percentage).toBe(100);
        done();
      });
    }));

    it('should return 100 for path with no required steps', () => new Promise<void>(done => {
      const allOptionalPath: LearningPath = {
        ...mockPath,
        steps: mockPath.steps.map(s => ({ ...s, optional: true })),
      };
      dataLoaderSpy.getPath.mockReturnValue(of(allOptionalPath));

      service.getCompletionPercentage('test-path').subscribe(percentage => {
        expect(percentage).toBe(100);
        done();
      });
    }));

    it('should only count required steps for percentage', () => new Promise<void>(done => {
      const partialProgress: AgentProgress = {
        ...mockProgress,
        completedStepIndices: [0, 2], // Completed step 0 (required) and 2 (optional)
      };
      agentServiceSpy.getProgressForPath.mockReturnValue(of(partialProgress));

      service.getCompletionPercentage('test-path').subscribe(percentage => {
        // 1 required completed out of 4 required total = 25%
        expect(percentage).toBe(25);
        done();
      });
    }));

    it('should handle empty path (no steps)', () => new Promise<void>(done => {
      const emptyPath: LearningPath = {
        ...mockPath,
        steps: [],
      };
      dataLoaderSpy.getPath.mockReturnValue(of(emptyPath));

      service.getCompletionPercentage('test-path').subscribe(percentage => {
        expect(percentage).toBe(0);
        done();
      });
    }));
  });

  describe('additional coverage', () => {
    it('should handle getPath errors in getPathStep', () => new Promise<void>(done => {
      dataLoaderSpy.getPath.mockReturnValue(throwError(() => new Error('Path load failed')));

      service.getPathStep('test-path', 0).subscribe({
        error: err => {
          expect(err.message).toContain('Path load failed');
          done();
        },
      });
    }));

    it('should handle getContent errors in getPathStep', () => new Promise<void>(done => {
      dataLoaderSpy.getContent.mockReturnValue(throwError(() => new Error('Content load failed')));

      service.getPathStep('test-path', 0).subscribe({
        error: err => {
          expect(err.message).toContain('Content load failed');
          done();
        },
      });
    }));
  });
});
