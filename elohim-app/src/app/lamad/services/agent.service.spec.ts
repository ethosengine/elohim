import { TestBed } from '@angular/core/testing';
import { of, BehaviorSubject } from 'rxjs';
import { AgentService } from '@app/elohim/services/agent.service';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';
import { SessionHumanService } from '@app/imagodei/services/session-human.service';
import { Agent, AgentProgress, FrontierItem, SessionHuman, AccessLevel, ContentAccessMetadata } from '../models';

describe('AgentService', () => {
  let service: AgentService;
  let dataLoaderSpy: jasmine.SpyObj<DataLoaderService>;
  let sessionHumanServiceSpy: jasmine.SpyObj<SessionHumanService>;
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
      sessionCount: 1
    }
  };

  const mockAgent: Agent = {
    id: 'test-agent',
    displayName: 'Test Agent',
    type: 'human',
    visibility: 'private',
    createdAt: '2025-01-01T00:00:00.000Z',
    updatedAt: '2025-01-01T00:00:00.000Z'
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
    attestationsEarned: []
  };

  beforeEach(() => {
    const dataLoaderSpyObj = jasmine.createSpyObj('DataLoaderService', [
      'getAgent',
      'getAgentProgress',
      'getLocalProgress',
      'saveAgentProgress'
    ]);
    const sessionHumanServiceSpyObj = jasmine.createSpyObj('SessionHumanService', [
      'getSessionId',
      'getAccessLevel',
      'checkContentAccess',
      'recordPathStarted',
      'recordStepCompleted',
      'recordNotesSaved'
    ]);

    // Mock localStorage
    localStorageMock = {};

    // Create a complete Storage mock
    mockStorage = {
      getItem: (key: string) => localStorageMock[key] || null,
      setItem: (key: string, value: string) => { localStorageMock[key] = value; },
      removeItem: (key: string) => { delete localStorageMock[key]; },
      key: (index: number) => Object.keys(localStorageMock)[index] || null,
      get length() { return Object.keys(localStorageMock).length; },
      clear: () => { localStorageMock = {}; }
    };

    // Replace global localStorage with our mock
    spyOnProperty(window, 'localStorage', 'get').and.returnValue(mockStorage);

    TestBed.configureTestingModule({
      providers: [
        AgentService,
        { provide: DataLoaderService, useValue: dataLoaderSpyObj },
        { provide: SessionHumanService, useValue: sessionHumanServiceSpyObj }
      ]
    });

    dataLoaderSpy = TestBed.inject(DataLoaderService) as jasmine.SpyObj<DataLoaderService>;
    sessionHumanServiceSpy = TestBed.inject(SessionHumanService) as jasmine.SpyObj<SessionHumanService>;

    // Default spy return values
    const sessionSubject = new BehaviorSubject<SessionHuman | null>(mockSessionHuman);
    Object.defineProperty(sessionHumanServiceSpy, 'session$', {
      get: () => sessionSubject.asObservable()
    });
    sessionHumanServiceSpy.getSessionId.and.returnValue('session-123');
    sessionHumanServiceSpy.getAccessLevel.and.returnValue('visitor');
    sessionHumanServiceSpy.checkContentAccess.and.returnValue({ canAccess: true });
    dataLoaderSpy.getAgent.and.returnValue(of(mockAgent));
    dataLoaderSpy.getAgentProgress.and.returnValue(of(mockProgress));
    dataLoaderSpy.getLocalProgress.and.returnValue(null);
    dataLoaderSpy.saveAgentProgress.and.returnValue(of(undefined));

    service = TestBed.inject(AgentService);
  });

  afterEach(() => {
    localStorageMock = {};
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('initialization with session service', () => {
    it('should initialize agent from session user', (done) => {
      service.getCurrentAgent().subscribe(agent => {
        expect(agent).toBeTruthy();
        expect(agent?.id).toBe('session-123');
        expect(agent?.displayName).toBe('Test User');
        done();
      });
    });

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
        requirements: { minLevel: 'member' }
      };
      const result = service.checkContentAccess(metadata);
      expect(sessionHumanServiceSpy.checkContentAccess).toHaveBeenCalledWith(metadata);
    });
  });

  describe('getProgressForPath', () => {
    it('should get progress from localStorage first', (done) => {
      dataLoaderSpy.getLocalProgress.and.returnValue(mockProgress);

      service.getProgressForPath('test-path').subscribe(progress => {
        expect(progress).toEqual(mockProgress);
        expect(dataLoaderSpy.getLocalProgress).toHaveBeenCalledWith('session-123', 'test-path');
        expect(dataLoaderSpy.getAgentProgress).not.toHaveBeenCalled();
        done();
      });
    });

    it('should fall back to JSON file if no localStorage data', (done) => {
      dataLoaderSpy.getLocalProgress.and.returnValue(null);

      service.getProgressForPath('test-path').subscribe(progress => {
        expect(progress).toEqual(mockProgress);
        expect(dataLoaderSpy.getLocalProgress).toHaveBeenCalled();
        expect(dataLoaderSpy.getAgentProgress).toHaveBeenCalledWith('session-123', 'test-path');
        done();
      });
    });

    it('should cache progress for subsequent calls', (done) => {
      service.getProgressForPath('test-path').subscribe(() => {
        dataLoaderSpy.getLocalProgress.calls.reset();
        dataLoaderSpy.getAgentProgress.calls.reset();

        service.getProgressForPath('test-path').subscribe(progress => {
          expect(progress).toEqual(mockProgress);
          expect(dataLoaderSpy.getLocalProgress).not.toHaveBeenCalled();
          expect(dataLoaderSpy.getAgentProgress).not.toHaveBeenCalled();
          done();
        });
      });
    });

    it('should collect attestations from progress', (done) => {
      const progressWithAttestations: AgentProgress = {
        ...mockProgress,
        attestationsEarned: ['test-attestation']
      };
      dataLoaderSpy.getAgentProgress.and.returnValue(of(progressWithAttestations));

      service.getProgressForPath('test-path').subscribe(() => {
        expect(service.hasAttestation('test-attestation')).toBe(true);
        done();
      });
    });
  });

  describe('completeStep', () => {
    beforeEach(() => {
      service.clearProgressCache();
      dataLoaderSpy.getAgentProgress.calls.reset();
      dataLoaderSpy.saveAgentProgress.calls.reset();
      // Return a fresh copy of mockProgress to avoid mutation between tests
      dataLoaderSpy.getAgentProgress.and.returnValue(of({
        ...mockProgress,
        completedStepIndices: [...mockProgress.completedStepIndices]
      }));
    });

    it('should mark step as completed', (done) => {
      service.completeStep('test-path', 2).subscribe(() => {
        expect(dataLoaderSpy.saveAgentProgress).toHaveBeenCalled();
        const savedProgress = dataLoaderSpy.saveAgentProgress.calls.mostRecent().args[0];
        expect(savedProgress.completedStepIndices).toContain(2);
        done();
      });
    });

    it('should not duplicate completed steps', (done) => {
      service.completeStep('test-path', 0).subscribe(() => {
        const savedProgress = dataLoaderSpy.saveAgentProgress.calls.mostRecent().args[0];
        expect(savedProgress.completedStepIndices.filter((i: number) => i === 0).length).toBe(1);
        done();
      });
    });

    it('should update current step index', (done) => {
      service.completeStep('test-path', 2).subscribe(() => {
        const savedProgress = dataLoaderSpy.saveAgentProgress.calls.mostRecent().args[0];
        expect(savedProgress.currentStepIndex).toBe(3);
        done();
      });
    });

    it('should create new progress if none exists', (done) => {
      dataLoaderSpy.getAgentProgress.and.returnValue(of(null as any));
      dataLoaderSpy.getLocalProgress.and.returnValue(null);

      service.completeStep('new-path', 0).subscribe(() => {
        const savedProgress = dataLoaderSpy.saveAgentProgress.calls.mostRecent().args[0];
        expect(savedProgress.pathId).toBe('new-path');
        expect(savedProgress.completedStepIndices).toEqual([0]);
        expect(savedProgress.currentStepIndex).toBe(1);
        done();
      });
    });

    it('should record path started in session on first step', (done) => {
      dataLoaderSpy.getAgentProgress.and.returnValue(of(null as any));
      dataLoaderSpy.getLocalProgress.and.returnValue(null);

      service.completeStep('new-path', 0).subscribe(() => {
        expect(sessionHumanServiceSpy.recordPathStarted).toHaveBeenCalledWith('new-path');
        done();
      });
    });

    it('should record step completed in session', (done) => {
      service.completeStep('test-path', 1).subscribe(() => {
        expect(sessionHumanServiceSpy.recordStepCompleted).toHaveBeenCalledWith('test-path', 1);
        done();
      });
    });

    it('should keep steps sorted', (done) => {
      service.completeStep('test-path', 3).subscribe(() => {
        service.completeStep('test-path', 2).subscribe(() => {
          const savedProgress = dataLoaderSpy.saveAgentProgress.calls.mostRecent().args[0];
          const indices = savedProgress.completedStepIndices;
          for (let i = 1; i < indices.length; i++) {
            expect(indices[i]).toBeGreaterThan(indices[i - 1]);
          }
          done();
        });
      });
    });
  });

  describe('updateAffinity', () => {
    beforeEach(() => {
      service.clearProgressCache();
      dataLoaderSpy.getAgentProgress.calls.reset();
      dataLoaderSpy.saveAgentProgress.calls.reset();
      // Return a fresh copy of mockProgress to avoid mutation between tests
      dataLoaderSpy.getAgentProgress.and.returnValue(of({
        ...mockProgress,
        stepAffinity: {}
      }));
    });

    it('should update affinity for a step', (done) => {
      service.updateAffinity('test-path', 1, 0.3).subscribe(() => {
        const savedProgress = dataLoaderSpy.saveAgentProgress.calls.mostRecent().args[0];
        expect(savedProgress.stepAffinity[1]).toBe(0.3);
        done();
      });
    });

    it('should clamp affinity to 0.0-1.0 range (upper)', (done) => {
      service.updateAffinity('test-path', 1, 2.0).subscribe(() => {
        const savedProgress = dataLoaderSpy.saveAgentProgress.calls.mostRecent().args[0];
        expect(savedProgress.stepAffinity[1]).toBe(1.0);
        done();
      });
    });

    it('should clamp affinity to 0.0-1.0 range (lower)', (done) => {
      service.updateAffinity('test-path', 1, -2.0).subscribe(() => {
        const savedProgress = dataLoaderSpy.saveAgentProgress.calls.mostRecent().args[0];
        expect(savedProgress.stepAffinity[1]).toBe(0.0);
        done();
      });
    });

    it('should handle delta updates', (done) => {
      const progressWithAffinity: AgentProgress = {
        ...mockProgress,
        stepAffinity: { 1: 0.5 }
      };
      dataLoaderSpy.getAgentProgress.and.returnValue(of(progressWithAffinity));

      service.updateAffinity('test-path', 1, 0.2).subscribe(() => {
        const savedProgress = dataLoaderSpy.saveAgentProgress.calls.mostRecent().args[0];
        expect(savedProgress.stepAffinity[1]).toBe(0.7);
        done();
      });
    });

    it('should not update affinity without progress', (done) => {
      dataLoaderSpy.getAgentProgress.and.returnValue(of(null as any));
      dataLoaderSpy.getLocalProgress.and.returnValue(null);

      service.updateAffinity('test-path', 1, 0.5).subscribe(() => {
        expect(dataLoaderSpy.saveAgentProgress).not.toHaveBeenCalled();
        done();
      });
    });
  });

  describe('saveStepNotes', () => {
    it('should save notes for a step', (done) => {
      service.saveStepNotes('test-path', 1, 'My notes').subscribe(() => {
        const savedProgress = dataLoaderSpy.saveAgentProgress.calls.mostRecent().args[0];
        expect(savedProgress.stepNotes[1]).toBe('My notes');
        done();
      });
    });

    it('should create progress if none exists', (done) => {
      dataLoaderSpy.getAgentProgress.and.returnValue(of(null as any));
      dataLoaderSpy.getLocalProgress.and.returnValue(null);

      service.saveStepNotes('new-path', 0, 'First note').subscribe(() => {
        const savedProgress = dataLoaderSpy.saveAgentProgress.calls.mostRecent().args[0];
        expect(savedProgress.pathId).toBe('new-path');
        expect(savedProgress.stepNotes[0]).toBe('First note');
        done();
      });
    });

    it('should record notes saved in session', (done) => {
      service.saveStepNotes('test-path', 1, 'Notes').subscribe(() => {
        expect(sessionHumanServiceSpy.recordNotesSaved).toHaveBeenCalledWith('test-path', 1);
        done();
      });
    });
  });

  describe('saveReflectionResponses', () => {
    it('should save reflection responses', (done) => {
      const responses = ['Response 1', 'Response 2'];
      service.saveReflectionResponses('test-path', 1, responses).subscribe(() => {
        const savedProgress = dataLoaderSpy.saveAgentProgress.calls.mostRecent().args[0];
        expect(savedProgress.reflectionResponses[1]).toEqual(responses);
        done();
      });
    });

    it('should not save reflections without progress', (done) => {
      dataLoaderSpy.getAgentProgress.and.returnValue(of(null as any));
      dataLoaderSpy.getLocalProgress.and.returnValue(null);

      service.saveReflectionResponses('test-path', 1, ['Response']).subscribe(() => {
        expect(dataLoaderSpy.saveAgentProgress).not.toHaveBeenCalled();
        done();
      });
    });
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
    it('should return active paths from localStorage', (done) => {
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
        attestationsEarned: []
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
        attestationsEarned: []
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
    });

    it('should exclude completed paths', (done) => {
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
        attestationsEarned: []
      };

      localStorageMock['lamad-progress-session-123-completed-path'] = JSON.stringify(completedProgress);

      service.getLearningFrontier().subscribe(frontier => {
        expect(frontier.length).toBe(0);
        done();
      });
    });

    it('should handle malformed localStorage entries', (done) => {
      localStorageMock['lamad-progress-session-123-bad'] = 'invalid json';

      service.getLearningFrontier().subscribe(frontier => {
        expect(frontier.length).toBe(0);
        done();
      });
    });

    it('should only include paths for current agent', (done) => {
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
        attestationsEarned: []
      };

      localStorageMock['lamad-progress-other-agent-path-1'] = JSON.stringify(otherAgentProgress);

      service.getLearningFrontier().subscribe(frontier => {
        expect(frontier.length).toBe(0);
        done();
      });
    });
  });

  describe('clearProgressCache', () => {
    it('should clear the progress cache', (done) => {
      service.getProgressForPath('test-path').subscribe(() => {
        service.clearProgressCache();
        dataLoaderSpy.getAgentProgress.calls.reset();

        service.getProgressForPath('test-path').subscribe(() => {
          expect(dataLoaderSpy.getAgentProgress).toHaveBeenCalled();
          done();
        });
      });
    });
  });
});
