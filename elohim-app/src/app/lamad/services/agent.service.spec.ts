import { TestBed } from '@angular/core/testing';
import { of, BehaviorSubject } from 'rxjs';
import { AgentService } from '@app/elohim/services/agent.service';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';
import { SessionHumanService } from '@app/imagodei/services/session-human.service';
import { AccessLevel, ContentAccessMetadata } from '../models';
import { Agent, AgentProgress, FrontierItem } from '@app/elohim/models/agent.model';
import { SessionHuman } from '@app/imagodei/models/session-human.model';
import { vi, Mock } from 'vitest';

describe('AgentService', () => {
  let service: AgentService;
  let dataLoaderSpy: any;
  let sessionHumanServiceSpy: any;
  let localStorageMock: { [key: string]: string };
  let mockStorage: Storage;

  const mockSessionHuman: SessionHuman = {
    sessionId: 'session-123',
    displayName: 'Test User',
    isAnonymous: true,
    accessLevel: 'visitor',
    sessionState: 'active',
    createdAt: '2025-01-01T00:00:00.000Z',
    lastActiveAt: '2025-01-01T00:00:00.000Z',
    stats: {
      nodesViewed: 0,
      nodesWithAffinity: 0,
      pathsStarted: 0,
      pathsCompleted: 0,
      stepsCompleted: 0,
      totalSessionTime: 0,
      averageSessionLength: 0,
      sessionCount: 1,
    },
  };

  const mockAgent: Agent = {
    id: 'test-agent',
    displayName: 'Test Agent',
    type: 'human',
    visibility: 'private',
    createdAt: '2025-01-01T00:00:00.000Z',
    updatedAt: '2025-01-01T00:00:00.000Z',
  };

  const mockProgress: AgentProgress = {
    agentId: 'session-123',
    pathId: 'test-path',
    currentStepIndex: 1,
    completedStepIndices: [0],
    startedAt: '2025-01-01T00:00:00.000Z',
    lastActivityAt: '2025-01-01T00:00:00.000Z',
    stepAffinity: {},
    stepNotes: {},
    reflectionResponses: {},
    attestationsEarned: [],
  };

  beforeEach(() => {
    const dataLoaderSpyObj = {
      getAgent: vi.fn(),
      getAgentProgress: vi.fn(),
      getLocalProgress: vi.fn(),
      saveAgentProgress: vi.fn(),
    };
    const sessionHumanServiceSpyObj = {
      getSessionId: vi.fn(),
      getAccessLevel: vi.fn(),
      checkContentAccess: vi.fn(),
      recordPathStarted: vi.fn(),
      recordStepCompleted: vi.fn(),
      recordNotesSaved: vi.fn(),
    };

    // Mock localStorage
    localStorageMock = {};

    // Create a complete Storage mock
    mockStorage = {
      getItem: (key: string) => localStorageMock[key] || null,
      setItem: (key: string, value: string) => {
        localStorageMock[key] = value;
      },
      removeItem: (key: string) => {
        delete localStorageMock[key];
      },
      key: (index: number) => Object.keys(localStorageMock)[index] || null,
      get length() {
        return Object.keys(localStorageMock).length;
      },
      clear: () => {
        localStorageMock = {};
      },
    };

    // Replace global localStorage with our mock
    vi.spyOn(window, 'localStorage', 'get').mockReturnValue(mockStorage);

    TestBed.configureTestingModule({
      providers: [
        AgentService,
        { provide: DataLoaderService, useValue: dataLoaderSpyObj },
        { provide: SessionHumanService, useValue: sessionHumanServiceSpyObj },
      ],
    });

    dataLoaderSpy = TestBed.inject(DataLoaderService) as { [K in keyof DataLoaderService]?: Mock };
    sessionHumanServiceSpy = TestBed.inject(SessionHumanService) as {
      [K in keyof SessionHumanService]?: Mock;
    };

    // Default spy return values
    const sessionSubject = new BehaviorSubject<SessionHuman | null>(mockSessionHuman);
    Object.defineProperty(sessionHumanServiceSpy, 'session$', {
      get: () => sessionSubject.asObservable(),
    });
    sessionHumanServiceSpy.getSessionId.mockReturnValue('session-123');
    sessionHumanServiceSpy.getAccessLevel.mockReturnValue('visitor');
    sessionHumanServiceSpy.checkContentAccess.mockReturnValue({ canAccess: true });
    dataLoaderSpy.getAgent.mockReturnValue(of(mockAgent));
    dataLoaderSpy.getAgentProgress.mockReturnValue(of(mockProgress));
    dataLoaderSpy.getLocalProgress.mockReturnValue(null);
    dataLoaderSpy.saveAgentProgress.mockReturnValue(of(undefined));

    service = TestBed.inject(AgentService);
  });

  afterEach(() => {
    localStorageMock = {};
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('initialization with session service', () => {
    it('should initialize agent from session user', () =>
      new Promise<void>(done => {
        service.getCurrentAgent().subscribe(agent => {
          expect(agent).toBeTruthy();
          expect(agent?.id).toBe('session-123');
          expect(agent?.displayName).toBe('Test User');
          done();
        });
      }));

    it('should return session ID as current agent ID', () => {
      const agentId = service.getCurrentAgentId();
      expect(agentId).toBe('session-123');
    });

    it('should recognize session user', () => {
      expect(service.isSessionUser()).toBe(true);
    });

    it('should get access level from session service', () => {
      const level = service.getAccessLevel();
      expect(level).toBe('visitor');
      expect(sessionHumanServiceSpy.getAccessLevel).toHaveBeenCalled();
    });

    it('should check content access via session service', () => {
      const metadata: ContentAccessMetadata = {
        accessLevel: 'gated',
        requirements: { minLevel: 'member' },
      };
      const result = service.checkContentAccess(metadata);
      expect(sessionHumanServiceSpy.checkContentAccess).toHaveBeenCalledWith(metadata);
    });
  });

  describe('getProgressForPath', () => {
    it('should get progress from localStorage first', () =>
      new Promise<void>(done => {
        dataLoaderSpy.getLocalProgress.mockReturnValue(mockProgress);

        service.getProgressForPath('test-path').subscribe(progress => {
          expect(progress).toEqual(mockProgress);
          expect(dataLoaderSpy.getLocalProgress).toHaveBeenCalledWith('session-123', 'test-path');
          expect(dataLoaderSpy.getAgentProgress).not.toHaveBeenCalled();
          done();
        });
      }));

    it('should fall back to JSON file if no localStorage data', () =>
      new Promise<void>(done => {
        dataLoaderSpy.getLocalProgress.mockReturnValue(null);

        service.getProgressForPath('test-path').subscribe(progress => {
          expect(progress).toEqual(mockProgress);
          expect(dataLoaderSpy.getLocalProgress).toHaveBeenCalled();
          expect(dataLoaderSpy.getAgentProgress).toHaveBeenCalledWith('session-123', 'test-path');
          done();
        });
      }));

    it('should cache progress for subsequent calls', () =>
      new Promise<void>(done => {
        service.getProgressForPath('test-path').subscribe(() => {
          dataLoaderSpy.getLocalProgress.mockClear();
          dataLoaderSpy.getAgentProgress.mockClear();

          service.getProgressForPath('test-path').subscribe(progress => {
            expect(progress).toEqual(mockProgress);
            expect(dataLoaderSpy.getLocalProgress).not.toHaveBeenCalled();
            expect(dataLoaderSpy.getAgentProgress).not.toHaveBeenCalled();
            done();
          });
        });
      }));

    it('should collect attestations from progress', () =>
      new Promise<void>(done => {
        const progressWithAttestations: AgentProgress = {
          ...mockProgress,
          attestationsEarned: ['test-attestation'],
        };
        dataLoaderSpy.getAgentProgress.mockReturnValue(of(progressWithAttestations));

        service.getProgressForPath('test-path').subscribe(() => {
          expect(service.hasAttestation('test-attestation')).toBe(true);
          done();
        });
      }));
  });

  describe('completeStep', () => {
    beforeEach(() => {
      service.clearProgressCache();
      dataLoaderSpy.getAgentProgress.mockClear();
      dataLoaderSpy.saveAgentProgress.mockClear();
      // Return a fresh copy of mockProgress to avoid mutation between tests
      dataLoaderSpy.getAgentProgress.mockReturnValue(
        of({
          ...mockProgress,
          completedStepIndices: [...mockProgress.completedStepIndices],
        })
      );
    });

    it('should mark step as completed', () =>
      new Promise<void>(done => {
        service.completeStep('test-path', 2).subscribe(() => {
          expect(dataLoaderSpy.saveAgentProgress).toHaveBeenCalled();
          const savedProgress = dataLoaderSpy.saveAgentProgress.mock.lastCall[0];
          expect(savedProgress.completedStepIndices).toContain(2);
          done();
        });
      }));

    it('should not duplicate completed steps', () =>
      new Promise<void>(done => {
        service.completeStep('test-path', 0).subscribe(() => {
          const savedProgress = dataLoaderSpy.saveAgentProgress.mock.lastCall[0];
          expect(savedProgress.completedStepIndices.filter((i: number) => i === 0).length).toBe(1);
          done();
        });
      }));

    it('should update current step index', () =>
      new Promise<void>(done => {
        service.completeStep('test-path', 2).subscribe(() => {
          const savedProgress = dataLoaderSpy.saveAgentProgress.mock.lastCall[0];
          expect(savedProgress.currentStepIndex).toBe(3);
          done();
        });
      }));

    it('should create new progress if none exists', () =>
      new Promise<void>(done => {
        dataLoaderSpy.getAgentProgress.mockReturnValue(of(null as any));
        dataLoaderSpy.getLocalProgress.mockReturnValue(null);

        service.completeStep('new-path', 0).subscribe(() => {
          const savedProgress = dataLoaderSpy.saveAgentProgress.mock.lastCall[0];
          expect(savedProgress.pathId).toBe('new-path');
          expect(savedProgress.completedStepIndices).toEqual([0]);
          expect(savedProgress.currentStepIndex).toBe(1);
          done();
        });
      }));

    it('should record path started in session on first step', () =>
      new Promise<void>(done => {
        dataLoaderSpy.getAgentProgress.mockReturnValue(of(null as any));
        dataLoaderSpy.getLocalProgress.mockReturnValue(null);

        service.completeStep('new-path', 0).subscribe(() => {
          expect(sessionHumanServiceSpy.recordPathStarted).toHaveBeenCalledWith('new-path');
          done();
        });
      }));

    it('should record step completed in session', () =>
      new Promise<void>(done => {
        service.completeStep('test-path', 1).subscribe(() => {
          expect(sessionHumanServiceSpy.recordStepCompleted).toHaveBeenCalledWith('test-path', 1);
          done();
        });
      }));

    it('should keep steps sorted', () =>
      new Promise<void>(done => {
        service.completeStep('test-path', 3).subscribe(() => {
          service.completeStep('test-path', 2).subscribe(() => {
            const savedProgress = dataLoaderSpy.saveAgentProgress.mock.lastCall[0];
            const indices = savedProgress.completedStepIndices;
            for (let i = 1; i < indices.length; i++) {
              expect(indices[i]).toBeGreaterThan(indices[i - 1]);
            }
            done();
          });
        });
      }));
  });

  describe('updateAffinity', () => {
    beforeEach(() => {
      service.clearProgressCache();
      dataLoaderSpy.getAgentProgress.mockClear();
      dataLoaderSpy.saveAgentProgress.mockClear();
      // Return a fresh copy of mockProgress to avoid mutation between tests
      dataLoaderSpy.getAgentProgress.mockReturnValue(
        of({
          ...mockProgress,
          stepAffinity: {},
        })
      );
    });

    it('should update affinity for a step', () =>
      new Promise<void>(done => {
        service.updateAffinity('test-path', 1, 0.3).subscribe(() => {
          const savedProgress = dataLoaderSpy.saveAgentProgress.mock.lastCall[0];
          expect(savedProgress.stepAffinity[1]).toBe(0.3);
          done();
        });
      }));

    it('should clamp affinity to 0.0-1.0 range (upper)', () =>
      new Promise<void>(done => {
        service.updateAffinity('test-path', 1, 2.0).subscribe(() => {
          const savedProgress = dataLoaderSpy.saveAgentProgress.mock.lastCall[0];
          expect(savedProgress.stepAffinity[1]).toBe(1.0);
          done();
        });
      }));

    it('should clamp affinity to 0.0-1.0 range (lower)', () =>
      new Promise<void>(done => {
        service.updateAffinity('test-path', 1, -2.0).subscribe(() => {
          const savedProgress = dataLoaderSpy.saveAgentProgress.mock.lastCall[0];
          expect(savedProgress.stepAffinity[1]).toBe(0.0);
          done();
        });
      }));

    it('should handle delta updates', () =>
      new Promise<void>(done => {
        const progressWithAffinity: AgentProgress = {
          ...mockProgress,
          stepAffinity: { 1: 0.5 },
        };
        dataLoaderSpy.getAgentProgress.mockReturnValue(of(progressWithAffinity));

        service.updateAffinity('test-path', 1, 0.2).subscribe(() => {
          const savedProgress = dataLoaderSpy.saveAgentProgress.mock.lastCall[0];
          expect(savedProgress.stepAffinity[1]).toBe(0.7);
          done();
        });
      }));

    it('should not update affinity without progress', () =>
      new Promise<void>(done => {
        dataLoaderSpy.getAgentProgress.mockReturnValue(of(null as any));
        dataLoaderSpy.getLocalProgress.mockReturnValue(null);

        service.updateAffinity('test-path', 1, 0.5).subscribe(() => {
          expect(dataLoaderSpy.saveAgentProgress).not.toHaveBeenCalled();
          done();
        });
      }));
  });

  describe('saveStepNotes', () => {
    it('should save notes for a step', () =>
      new Promise<void>(done => {
        service.saveStepNotes('test-path', 1, 'My notes').subscribe(() => {
          const savedProgress = dataLoaderSpy.saveAgentProgress.mock.lastCall[0];
          expect(savedProgress.stepNotes[1]).toBe('My notes');
          done();
        });
      }));

    it('should create progress if none exists', () =>
      new Promise<void>(done => {
        dataLoaderSpy.getAgentProgress.mockReturnValue(of(null as any));
        dataLoaderSpy.getLocalProgress.mockReturnValue(null);

        service.saveStepNotes('new-path', 0, 'First note').subscribe(() => {
          const savedProgress = dataLoaderSpy.saveAgentProgress.mock.lastCall[0];
          expect(savedProgress.pathId).toBe('new-path');
          expect(savedProgress.stepNotes[0]).toBe('First note');
          done();
        });
      }));

    it('should record notes saved in session', () =>
      new Promise<void>(done => {
        service.saveStepNotes('test-path', 1, 'Notes').subscribe(() => {
          expect(sessionHumanServiceSpy.recordNotesSaved).toHaveBeenCalledWith('test-path', 1);
          done();
        });
      }));
  });

  describe('saveReflectionResponses', () => {
    it('should save reflection responses', () =>
      new Promise<void>(done => {
        const responses = ['Response 1', 'Response 2'];
        service.saveReflectionResponses('test-path', 1, responses).subscribe(() => {
          const savedProgress = dataLoaderSpy.saveAgentProgress.mock.lastCall[0];
          expect(savedProgress.reflectionResponses[1]).toEqual(responses);
          done();
        });
      }));

    it('should not save reflections without progress', () =>
      new Promise<void>(done => {
        dataLoaderSpy.getAgentProgress.mockReturnValue(of(null as any));
        dataLoaderSpy.getLocalProgress.mockReturnValue(null);

        service.saveReflectionResponses('test-path', 1, ['Response']).subscribe(() => {
          expect(dataLoaderSpy.saveAgentProgress).not.toHaveBeenCalled();
          done();
        });
      }));
  });

  describe('attestations', () => {
    it('should grant attestation', () => {
      service.grantAttestation('test-attestation', 'completed-path');
      expect(service.hasAttestation('test-attestation')).toBe(true);
    });

    it('should check for attestation', () => {
      expect(service.hasAttestation('nonexistent')).toBe(false);
      service.grantAttestation('test-attestation', 'test');
      expect(service.hasAttestation('test-attestation')).toBe(true);
    });

    it('should get all attestations', () => {
      service.grantAttestation('attestation-1', 'test');
      service.grantAttestation('attestation-2', 'test');
      const attestations = service.getAttestations();
      expect(attestations).toContain('attestation-1');
      expect(attestations).toContain('attestation-2');
      expect(attestations.length).toBe(2);
    });
  });

  describe('getLearningFrontier', () => {
    it('should return active paths from localStorage', () =>
      new Promise<void>(done => {
        const progress1: AgentProgress = {
          agentId: 'session-123',
          pathId: 'path-1',
          currentStepIndex: 2,
          completedStepIndices: [0, 1],
          startedAt: '2025-01-01T00:00:00.000Z',
          lastActivityAt: '2025-01-02T00:00:00.000Z',
          stepAffinity: {},
          stepNotes: {},
          reflectionResponses: {},
          attestationsEarned: [],
        };

        const progress2: AgentProgress = {
          agentId: 'session-123',
          pathId: 'path-2',
          currentStepIndex: 1,
          completedStepIndices: [0],
          startedAt: '2025-01-01T00:00:00.000Z',
          lastActivityAt: '2025-01-03T00:00:00.000Z',
          stepAffinity: {},
          stepNotes: {},
          reflectionResponses: {},
          attestationsEarned: [],
        };

        localStorageMock['lamad-progress-session-123-path-1'] = JSON.stringify(progress1);
        localStorageMock['lamad-progress-session-123-path-2'] = JSON.stringify(progress2);

        service.getLearningFrontier().subscribe(frontier => {
          expect(frontier.length).toBe(2);
          // Should be sorted by most recent first
          expect(frontier[0].pathId).toBe('path-2');
          expect(frontier[0].nextStepIndex).toBe(1);
          expect(frontier[1].pathId).toBe('path-1');
          done();
        });
      }));

    it('should exclude completed paths', () =>
      new Promise<void>(done => {
        const completedProgress: AgentProgress = {
          agentId: 'session-123',
          pathId: 'completed-path',
          currentStepIndex: 3,
          completedStepIndices: [0, 1, 2],
          startedAt: '2025-01-01T00:00:00.000Z',
          lastActivityAt: '2025-01-02T00:00:00.000Z',
          completedAt: '2025-01-02T00:00:00.000Z',
          stepAffinity: {},
          stepNotes: {},
          reflectionResponses: {},
          attestationsEarned: [],
        };

        localStorageMock['lamad-progress-session-123-completed-path'] =
          JSON.stringify(completedProgress);

        service.getLearningFrontier().subscribe(frontier => {
          expect(frontier.length).toBe(0);
          done();
        });
      }));

    it('should handle malformed localStorage entries', () =>
      new Promise<void>(done => {
        localStorageMock['lamad-progress-session-123-bad'] = 'invalid json';

        service.getLearningFrontier().subscribe(frontier => {
          expect(frontier.length).toBe(0);
          done();
        });
      }));

    it('should only include paths for current agent', () =>
      new Promise<void>(done => {
        const otherAgentProgress: AgentProgress = {
          agentId: 'other-agent',
          pathId: 'path-1',
          currentStepIndex: 1,
          completedStepIndices: [0],
          startedAt: '2025-01-01T00:00:00.000Z',
          lastActivityAt: '2025-01-02T00:00:00.000Z',
          stepAffinity: {},
          stepNotes: {},
          reflectionResponses: {},
          attestationsEarned: [],
        };

        localStorageMock['lamad-progress-other-agent-path-1'] = JSON.stringify(otherAgentProgress);

        service.getLearningFrontier().subscribe(frontier => {
          expect(frontier.length).toBe(0);
          done();
        });
      }));
  });

  describe('clearProgressCache', () => {
    it('should clear the progress cache', () =>
      new Promise<void>(done => {
        service.getProgressForPath('test-path').subscribe(() => {
          service.clearProgressCache();
          dataLoaderSpy.getAgentProgress.mockClear();

          service.getProgressForPath('test-path').subscribe(() => {
            expect(dataLoaderSpy.getAgentProgress).toHaveBeenCalled();
            done();
          });
        });
      }));
  });
});
