import { TestBed } from '@angular/core/testing';
import { of } from 'rxjs';
import { AgentService } from './agent.service';
import { DataLoaderService } from './data-loader.service';
import { SessionHumanService } from '../../imagodei/services/session-human.service';
import type { AgentProgress } from '../models/agent.model';

/**
 * Unit tests for AgentService
 *
 * Tests agent management, progress tracking, and learning analytics.
 * Coverage target: 60%+ (from 39.8%)
 */
describe('AgentService', () => {
  let service: AgentService;
  let mockDataLoader: jasmine.SpyObj<DataLoaderService>;
  let mockSessionService: jasmine.SpyObj<SessionHumanService>;

  beforeEach(() => {
    mockDataLoader = jasmine.createSpyObj('DataLoaderService', [
      'getAgentProgress',
      'saveAgentProgress',
      'getLocalProgress',
    ]);

    mockSessionService = jasmine.createSpyObj('SessionHumanService', [
      'getSessionId',
      'getAccessLevel',
      'checkContentAccess',
      'recordPathStarted',
      'recordStepCompleted',
      'recordNotesSaved',
    ]);

    Object.defineProperty(mockSessionService, 'session$', {
      get: () => of({
        sessionId: 'test-session-123',
        displayName: 'Test User',
        createdAt: new Date().toISOString(),
        lastActiveAt: new Date().toISOString(),
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
        isAnonymous: true,
        accessLevel: 'visitor' as const,
        sessionState: 'active' as const,
      }),
    });

    TestBed.configureTestingModule({
      providers: [
        AgentService,
        { provide: DataLoaderService, useValue: mockDataLoader },
        { provide: SessionHumanService, useValue: mockSessionService },
      ],
    });

    service = TestBed.inject(AgentService);
  });

  describe('initialization', () => {
    it('should be created', () => {
      expect(service).toBeTruthy();
    });

    it('should create agent from session', done => {
      service.agent$.subscribe(agent => {
        if (agent) {
          expect(agent.id).toBe('test-session-123');
          expect(agent.displayName).toBe('Test User');
          expect(agent.type).toBe('human');
          done();
        }
      });
    });

    it('should create anonymous agent without session service', () => {
      const noSessionService = new AgentService(mockDataLoader, null);

      noSessionService.getCurrentAgent().subscribe(agent => {
        expect(agent).toBeTruthy();
        expect(agent?.id).toContain('anon-');
        expect(agent?.displayName).toBe('Anonymous');
      });
    });
  });

  describe('getCurrentAgent', () => {
    it('should return current agent as observable', done => {
      service.getCurrentAgent().subscribe(agent => {
        expect(agent).toBeTruthy();
        expect(agent?.id).toBe('test-session-123');
        done();
      });
    });

    it('should emit only once (take 1)', done => {
      let emissionCount = 0;

      service.getCurrentAgent().subscribe({
        next: () => {
          emissionCount++;
        },
        complete: () => {
          expect(emissionCount).toBe(1);
          done();
        },
      });
    });
  });

  describe('getAgent (deprecated)', () => {
    it('should return current agent synchronously', () => {
      const agent = service.getAgent();

      expect(agent).toBeTruthy();
      expect(agent?.id).toBe('test-session-123');
    });
  });

  describe('getCurrentAgentId', () => {
    it('should return session ID', () => {
      mockSessionService.getSessionId.and.returnValue('test-session-123');

      const agentId = service.getCurrentAgentId();

      expect(agentId).toBe('test-session-123');
    });

    it('should return anonymous if no session', () => {
      const noSessionService = new AgentService(mockDataLoader, null);
      const agentId = noSessionService.getCurrentAgentId();

      expect(agentId).toContain('anon-');
    });
  });

  describe('isSessionUser', () => {
    it('should return true with session service', () => {
      expect(service.isSessionUser()).toBeTrue();
    });

    it('should return false without session service', () => {
      const noSessionService = new AgentService(mockDataLoader, null);
      expect(noSessionService.isSessionUser()).toBeFalse();
    });
  });

  describe('getAccessLevel', () => {
    it('should return access level from session', () => {
      mockSessionService.getAccessLevel.and.returnValue('member');

      const level = service.getAccessLevel();

      expect(level).toBe('member');
    });

    it('should return visitor without session', () => {
      const noSessionService = new AgentService(mockDataLoader, null);
      const level = noSessionService.getAccessLevel();

      expect(level).toBe('visitor');
    });
  });

  describe('checkContentAccess', () => {
    it('should check access via session service', () => {
      mockSessionService.checkContentAccess.and.returnValue({ canAccess: true });

      const result = service.checkContentAccess({ accessLevel: 'open' });

      expect(result.canAccess).toBeTrue();
      expect(mockSessionService.checkContentAccess).toHaveBeenCalled();
    });

    it('should allow access without metadata', () => {
      const noSessionService = new AgentService(mockDataLoader, null);
      const result = noSessionService.checkContentAccess();

      expect(result.canAccess).toBeTrue();
    });
  });

  describe('getProgressForPath', () => {
    it('should return cached progress', done => {
      const mockProgress: AgentProgress = {
        agentId: 'test-session-123',
        pathId: 'test-path',
        currentStepIndex: 2,
        completedStepIndices: [0, 1],
        startedAt: new Date().toISOString(),
        lastActivityAt: new Date().toISOString(),
        stepAffinity: {},
        stepNotes: {},
        reflectionResponses: {},
        attestationsEarned: [],
      };

      mockDataLoader.getLocalProgress.and.returnValue(mockProgress);

      service.getProgressForPath('test-path').subscribe(progress => {
        expect(progress).toEqual(mockProgress);
        done();
      });
    });

    it('should fall back to DataLoader if not in localStorage', done => {
      mockDataLoader.getLocalProgress.and.returnValue(null);
      mockDataLoader.getAgentProgress.and.returnValue(
        of({
          agentId: 'test-session-123',
          pathId: 'test-path',
          currentStepIndex: 0,
          completedStepIndices: [],
          startedAt: new Date().toISOString(),
          lastActivityAt: new Date().toISOString(),
          stepAffinity: {},
          stepNotes: {},
          reflectionResponses: {},
          attestationsEarned: [],
        })
      );

      service.getProgressForPath('test-path').subscribe(progress => {
        expect(progress).toBeTruthy();
        expect(mockDataLoader.getAgentProgress).toHaveBeenCalled();
        done();
      });
    });

    it('should cache progress after loading', done => {
      const progress: AgentProgress = {
        agentId: 'test-session-123',
        pathId: 'test-path',
        currentStepIndex: 0,
        completedStepIndices: [],
        startedAt: new Date().toISOString(),
        lastActivityAt: new Date().toISOString(),
        stepAffinity: {},
        stepNotes: {},
        reflectionResponses: {},
        attestationsEarned: ['att-1'],
      };

      mockDataLoader.getLocalProgress.and.returnValue(null);
      mockDataLoader.getAgentProgress.and.returnValue(of(progress));

      service.getProgressForPath('test-path').subscribe(() => {
        // Second call should use cache
        service.getProgressForPath('test-path').subscribe(cached => {
          expect(cached).toEqual(progress);
          done();
        });
      });
    });

    it('should track attestations from progress', done => {
      const progress: AgentProgress = {
        agentId: 'test-session-123',
        pathId: 'test-path',
        currentStepIndex: 0,
        completedStepIndices: [],
        startedAt: new Date().toISOString(),
        lastActivityAt: new Date().toISOString(),
        stepAffinity: {},
        stepNotes: {},
        reflectionResponses: {},
        attestationsEarned: ['mastery-javascript'],
      };

      mockDataLoader.getLocalProgress.and.returnValue(progress);

      service.getProgressForPath('test-path').subscribe(() => {
        expect(service.hasAttestation('mastery-javascript')).toBeTrue();
        done();
      });
    });
  });

  describe('completeStep', () => {
    beforeEach(() => {
      mockDataLoader.saveAgentProgress.and.returnValue(of(undefined));
      mockSessionService.getSessionId.and.returnValue('test-session-123');
    });

    it('should complete a step', done => {
      mockDataLoader.getLocalProgress.and.returnValue(null);
      mockDataLoader.getAgentProgress.and.returnValue(of(null));

      service.completeStep('path-1', 0).subscribe(() => {
        expect(mockDataLoader.saveAgentProgress).toHaveBeenCalled();
        done();
      });
    });

    it('should add step to completedStepIndices', done => {
      const existingProgress: AgentProgress = {
        agentId: 'test-session-123',
        pathId: 'path-1',
        currentStepIndex: 0,
        completedStepIndices: [],
        startedAt: new Date().toISOString(),
        lastActivityAt: new Date().toISOString(),
        stepAffinity: {},
        stepNotes: {},
        reflectionResponses: {},
        attestationsEarned: [],
      };

      mockDataLoader.getLocalProgress.and.returnValue(existingProgress);

      service.completeStep('path-1', 0).subscribe(() => {
        const savedProgress = mockDataLoader.saveAgentProgress.calls.mostRecent().args[0];
        expect(savedProgress.completedStepIndices).toContain(0);
        done();
      });
    });

    it('should not duplicate completed steps', done => {
      const existingProgress: AgentProgress = {
        agentId: 'test-session-123',
        pathId: 'path-1',
        currentStepIndex: 1,
        completedStepIndices: [0],
        startedAt: new Date().toISOString(),
        lastActivityAt: new Date().toISOString(),
        stepAffinity: {},
        stepNotes: {},
        reflectionResponses: {},
        attestationsEarned: [],
      };

      mockDataLoader.getLocalProgress.and.returnValue(existingProgress);

      service.completeStep('path-1', 0).subscribe(() => {
        const savedProgress = mockDataLoader.saveAgentProgress.calls.mostRecent().args[0];
        expect(savedProgress.completedStepIndices).toEqual([0]);
        done();
      });
    });

    it('should advance currentStepIndex', done => {
      const existingProgress: AgentProgress = {
        agentId: 'test-session-123',
        pathId: 'path-1',
        currentStepIndex: 0,
        completedStepIndices: [],
        startedAt: new Date().toISOString(),
        lastActivityAt: new Date().toISOString(),
        stepAffinity: {},
        stepNotes: {},
        reflectionResponses: {},
        attestationsEarned: [],
      };

      mockDataLoader.getLocalProgress.and.returnValue(existingProgress);

      service.completeStep('path-1', 0).subscribe(() => {
        const savedProgress = mockDataLoader.saveAgentProgress.calls.mostRecent().args[0];
        expect(savedProgress.currentStepIndex).toBe(1);
        done();
      });
    });

    it('should record path started for new paths', done => {
      mockDataLoader.getLocalProgress.and.returnValue(null);
      mockDataLoader.getAgentProgress.and.returnValue(of(null));

      service.completeStep('new-path', 0).subscribe(() => {
        expect(mockSessionService.recordPathStarted).toHaveBeenCalledWith('new-path');
        done();
      });
    });

    it('should record step completed in session', done => {
      mockDataLoader.getLocalProgress.and.returnValue(null);
      mockDataLoader.getAgentProgress.and.returnValue(of(null));

      service.completeStep('path-1', 0).subscribe(() => {
        expect(mockSessionService.recordStepCompleted).toHaveBeenCalledWith('path-1', 0);
        done();
      });
    });

    it('should track content completion when resourceId provided', done => {
      mockDataLoader.getLocalProgress.and.returnValue(null);
      mockDataLoader.getAgentProgress.and.returnValue(of(null));

      service.completeStep('path-1', 0, 'content-123').subscribe(() => {
        // Should trigger content completion tracking
        expect(mockDataLoader.saveAgentProgress).toHaveBeenCalled();
        done();
      });
    });
  });

  describe('updateAffinity', () => {
    beforeEach(() => {
      mockDataLoader.saveAgentProgress.and.returnValue(of(undefined));
    });

    it('should update step affinity', done => {
      const existingProgress: AgentProgress = {
        agentId: 'test-session-123',
        pathId: 'path-1',
        currentStepIndex: 0,
        completedStepIndices: [],
        startedAt: new Date().toISOString(),
        lastActivityAt: new Date().toISOString(),
        stepAffinity: { 0: 0.5 },
        stepNotes: {},
        reflectionResponses: {},
        attestationsEarned: [],
      };

      mockDataLoader.getLocalProgress.and.returnValue(existingProgress);

      service.updateAffinity('path-1', 0, 0.2).subscribe(() => {
        const savedProgress = mockDataLoader.saveAgentProgress.calls.mostRecent().args[0];
        expect(savedProgress.stepAffinity[0]).toBe(0.7);
        done();
      });
    });

    it('should clamp affinity to 0.0-1.0 range', done => {
      const existingProgress: AgentProgress = {
        agentId: 'test-session-123',
        pathId: 'path-1',
        currentStepIndex: 0,
        completedStepIndices: [],
        startedAt: new Date().toISOString(),
        lastActivityAt: new Date().toISOString(),
        stepAffinity: { 0: 0.9 },
        stepNotes: {},
        reflectionResponses: {},
        attestationsEarned: [],
      };

      mockDataLoader.getLocalProgress.and.returnValue(existingProgress);

      service.updateAffinity('path-1', 0, 0.5).subscribe(() => {
        const savedProgress = mockDataLoader.saveAgentProgress.calls.mostRecent().args[0];
        expect(savedProgress.stepAffinity[0]).toBe(1.0);
        done();
      });
    });

    it('should clamp negative affinity to 0', done => {
      const existingProgress: AgentProgress = {
        agentId: 'test-session-123',
        pathId: 'path-1',
        currentStepIndex: 0,
        completedStepIndices: [],
        startedAt: new Date().toISOString(),
        lastActivityAt: new Date().toISOString(),
        stepAffinity: { 0: 0.1 },
        stepNotes: {},
        reflectionResponses: {},
        attestationsEarned: [],
      };

      mockDataLoader.getLocalProgress.and.returnValue(existingProgress);

      service.updateAffinity('path-1', 0, -0.5).subscribe(() => {
        const savedProgress = mockDataLoader.saveAgentProgress.calls.mostRecent().args[0];
        expect(savedProgress.stepAffinity[0]).toBe(0);
        done();
      });
    });

    it('should return early if no existing progress', done => {
      mockDataLoader.getLocalProgress.and.returnValue(null);
      mockDataLoader.getAgentProgress.and.returnValue(of(null));

      service.updateAffinity('path-1', 0, 0.5).subscribe(() => {
        expect(mockDataLoader.saveAgentProgress).not.toHaveBeenCalled();
        done();
      });
    });
  });

  describe('saveStepNotes', () => {
    beforeEach(() => {
      mockDataLoader.saveAgentProgress.and.returnValue(of(undefined));
      mockSessionService.getSessionId.and.returnValue('test-session-123');
    });

    it('should save notes for a step', done => {
      mockDataLoader.getLocalProgress.and.returnValue(null);
      mockDataLoader.getAgentProgress.and.returnValue(of(null));

      service.saveStepNotes('path-1', 0, 'My notes').subscribe(() => {
        const savedProgress = mockDataLoader.saveAgentProgress.calls.mostRecent().args[0];
        expect(savedProgress.stepNotes[0]).toBe('My notes');
        done();
      });
    });

    it('should record notes saved in session', done => {
      mockDataLoader.getLocalProgress.and.returnValue(null);
      mockDataLoader.getAgentProgress.and.returnValue(of(null));

      service.saveStepNotes('path-1', 0, 'Notes').subscribe(() => {
        expect(mockSessionService.recordNotesSaved).toHaveBeenCalledWith('path-1', 0);
        done();
      });
    });

    it('should create progress if none exists', done => {
      mockDataLoader.getLocalProgress.and.returnValue(null);
      mockDataLoader.getAgentProgress.and.returnValue(of(null));

      service.saveStepNotes('new-path', 0, 'First note').subscribe(() => {
        const savedProgress = mockDataLoader.saveAgentProgress.calls.mostRecent().args[0];
        expect(savedProgress.pathId).toBe('new-path');
        expect(savedProgress.stepNotes[0]).toBe('First note');
        done();
      });
    });
  });

  describe('saveReflectionResponses', () => {
    beforeEach(() => {
      mockDataLoader.saveAgentProgress.and.returnValue(of(undefined));
    });

    it('should save reflection responses', done => {
      const existingProgress: AgentProgress = {
        agentId: 'test-session-123',
        pathId: 'path-1',
        currentStepIndex: 0,
        completedStepIndices: [],
        startedAt: new Date().toISOString(),
        lastActivityAt: new Date().toISOString(),
        stepAffinity: {},
        stepNotes: {},
        reflectionResponses: {},
        attestationsEarned: [],
      };

      mockDataLoader.getLocalProgress.and.returnValue(existingProgress);

      const responses = ['Answer 1', 'Answer 2'];
      service.saveReflectionResponses('path-1', 0, responses).subscribe(() => {
        const savedProgress = mockDataLoader.saveAgentProgress.calls.mostRecent().args[0];
        expect(savedProgress.reflectionResponses[0]).toEqual(responses);
        done();
      });
    });

    it('should return early if no existing progress', done => {
      mockDataLoader.getLocalProgress.and.returnValue(null);
      mockDataLoader.getAgentProgress.and.returnValue(of(null));

      service.saveReflectionResponses('path-1', 0, ['answer']).subscribe(() => {
        expect(mockDataLoader.saveAgentProgress).not.toHaveBeenCalled();
        done();
      });
    });
  });

  describe('attestations', () => {
    it('should grant an attestation', () => {
      service.grantAttestation('mastery-typescript', 'path-completion');

      expect(service.hasAttestation('mastery-typescript')).toBeTrue();
    });

    it('should check for attestations', () => {
      service.grantAttestation('att-1', 'earned');

      expect(service.hasAttestation('att-1')).toBeTrue();
      expect(service.hasAttestation('att-2')).toBeFalse();
    });

    it('should get all attestations', () => {
      service.grantAttestation('att-1', 'earned');
      service.grantAttestation('att-2', 'earned');

      const attestations = service.getAttestations();

      expect(attestations).toContain('att-1');
      expect(attestations).toContain('att-2');
      expect(attestations.length).toBe(2);
    });
  });

  describe('content completion tracking', () => {
    beforeEach(() => {
      mockDataLoader.saveAgentProgress.and.returnValue(of(undefined));
      mockDataLoader.getLocalProgress.and.returnValue(null);
      mockDataLoader.getAgentProgress.and.returnValue(of(null));
      mockSessionService.getSessionId.and.returnValue('test-session-123');
    });

    it('should track content completion globally', done => {
      service.completeContentNode('content-123').subscribe(() => {
        expect(mockDataLoader.saveAgentProgress).toHaveBeenCalled();
        done();
      });
    });

    it('should not duplicate completed content IDs', done => {
      const globalProgress: AgentProgress = {
        agentId: 'test-session-123',
        pathId: '__global__',
        currentStepIndex: 0,
        completedStepIndices: [],
        startedAt: new Date().toISOString(),
        lastActivityAt: new Date().toISOString(),
        stepAffinity: {},
        stepNotes: {},
        reflectionResponses: {},
        attestationsEarned: [],
        completedContentIds: ['content-123'],
      };

      mockDataLoader.getLocalProgress.and.callFake((agentId: string, pathId: string) => {
        return pathId === '__global__' ? globalProgress : null;
      });

      service.completeContentNode('content-123').subscribe(() => {
        const saved = mockDataLoader.saveAgentProgress.calls.mostRecent().args[0];
        expect(saved.completedContentIds).toEqual(['content-123']);
        done();
      });
    });

    it('should check if content is completed', done => {
      const globalProgress: AgentProgress = {
        agentId: 'test-session-123',
        pathId: '__global__',
        currentStepIndex: 0,
        completedStepIndices: [],
        startedAt: new Date().toISOString(),
        lastActivityAt: new Date().toISOString(),
        stepAffinity: {},
        stepNotes: {},
        reflectionResponses: {},
        attestationsEarned: [],
        completedContentIds: ['content-123'],
      };

      mockDataLoader.getLocalProgress.and.callFake((agentId: string, pathId: string) => {
        return pathId === '__global__' ? globalProgress : null;
      });

      service.isContentCompleted('content-123').subscribe(isCompleted => {
        expect(isCompleted).toBeTrue();
        done();
      });
    });

    it('should return false for non-completed content', done => {
      mockDataLoader.getLocalProgress.and.returnValue(null);
      mockDataLoader.getAgentProgress.and.returnValue(of(null));

      service.isContentCompleted('missing-content').subscribe(isCompleted => {
        expect(isCompleted).toBeFalse();
        done();
      });
    });

    it('should get completed content IDs as Set', done => {
      const globalProgress: AgentProgress = {
        agentId: 'test-session-123',
        pathId: '__global__',
        currentStepIndex: 0,
        completedStepIndices: [],
        startedAt: new Date().toISOString(),
        lastActivityAt: new Date().toISOString(),
        stepAffinity: {},
        stepNotes: {},
        reflectionResponses: {},
        attestationsEarned: [],
        completedContentIds: ['c1', 'c2', 'c3'],
      };

      mockDataLoader.getLocalProgress.and.callFake((agentId: string, pathId: string) => {
        return pathId === '__global__' ? globalProgress : null;
      });

      service.getCompletedContentIds().subscribe(ids => {
        expect(ids).toBeInstanceOf(Set);
        expect(ids.size).toBe(3);
        expect(ids.has('c1')).toBeTrue();
        done();
      });
    });
  });

  describe('mastery tracking', () => {
    beforeEach(() => {
      mockDataLoader.saveAgentProgress.and.returnValue(of(undefined));
      mockSessionService.getSessionId.and.returnValue('test-session-123');
    });

    it('should get mastery level for content', done => {
      const globalProgress: AgentProgress = {
        agentId: 'test-session-123',
        pathId: '__global__',
        currentStepIndex: 0,
        completedStepIndices: [],
        startedAt: new Date().toISOString(),
        lastActivityAt: new Date().toISOString(),
        stepAffinity: {},
        stepNotes: {},
        reflectionResponses: {},
        attestationsEarned: [],
        contentMastery: { 'content-123': 'understand' },
      };

      mockDataLoader.getLocalProgress.and.callFake((agentId: string, pathId: string) => {
        return pathId === '__global__' ? globalProgress : null;
      });

      service.getContentMastery('content-123').subscribe(mastery => {
        expect(mastery).toBe('understand');
        done();
      });
    });

    it('should return not_started for unmapped content', done => {
      mockDataLoader.getLocalProgress.and.returnValue(null);
      mockDataLoader.getAgentProgress.and.returnValue(of(null));

      service.getContentMastery('new-content').subscribe(mastery => {
        expect(mastery).toBe('not_started');
        done();
      });
    });

    it('should update content mastery level', done => {
      mockDataLoader.getLocalProgress.and.returnValue(null);
      mockDataLoader.getAgentProgress.and.returnValue(of(null));

      service.updateContentMastery('content-123', 'remember').subscribe(() => {
        const saved = mockDataLoader.saveAgentProgress.calls.mostRecent().args[0];
        expect(saved.contentMastery?.['content-123']).toBe('remember');
        done();
      });
    });

    it('should only increase mastery level (ratchet)', done => {
      const globalProgress: AgentProgress = {
        agentId: 'test-session-123',
        pathId: '__global__',
        currentStepIndex: 0,
        completedStepIndices: [],
        startedAt: new Date().toISOString(),
        lastActivityAt: new Date().toISOString(),
        stepAffinity: {},
        stepNotes: {},
        reflectionResponses: {},
        attestationsEarned: [],
        contentMastery: { 'content-123': 'apply' },
      };

      mockDataLoader.getLocalProgress.and.callFake((agentId: string, pathId: string) => {
        return pathId === '__global__' ? globalProgress : null;
      });

      // Try to downgrade mastery
      service.updateContentMastery('content-123', 'understand').subscribe(() => {
        const saved = mockDataLoader.saveAgentProgress.calls.mostRecent().args[0];
        // Should remain at 'apply' (higher level)
        expect(saved.contentMastery?.['content-123']).toBe('apply');
        done();
      });
    });

    it('should mark as completed when mastery reaches apply', done => {
      mockDataLoader.getLocalProgress.and.returnValue(null);
      mockDataLoader.getAgentProgress.and.returnValue(of(null));

      service.updateContentMastery('content-123', 'apply').subscribe(() => {
        const saved = mockDataLoader.saveAgentProgress.calls.mostRecent().args[0];
        expect(saved.completedContentIds).toContain('content-123');
        done();
      });
    });

    it('should provide convenience method for marking content as seen', done => {
      mockDataLoader.getLocalProgress.and.returnValue(null);
      mockDataLoader.getAgentProgress.and.returnValue(of(null));

      service.markContentSeen('content-123').subscribe(() => {
        const saved = mockDataLoader.saveAgentProgress.calls.mostRecent().args[0];
        expect(saved.contentMastery?.['content-123']).toBe('seen');
        done();
      });
    });
  });

  describe('getAgentProgress', () => {
    it('should return all progress records from localStorage', done => {
      // Ensure mock returns correct session ID
      mockSessionService.getSessionId.and.returnValue('test-session-123');

      // Mock localStorage with actual data
      const progressData1 = {
        agentId: 'test-session-123',
        pathId: 'path1',
        currentStepIndex: 1,
        completedStepIndices: [0],
        startedAt: new Date().toISOString(),
        lastActivityAt: new Date().toISOString(),
        stepAffinity: {},
        stepNotes: {},
        reflectionResponses: {},
        attestationsEarned: [],
      };

      const progressData2 = {
        agentId: 'test-session-123',
        pathId: 'path2',
        currentStepIndex: 0,
        completedStepIndices: [],
        startedAt: new Date().toISOString(),
        lastActivityAt: new Date().toISOString(),
        stepAffinity: {},
        stepNotes: {},
        reflectionResponses: {},
        attestationsEarned: [],
      };

      // Actually set items in localStorage for this test
      localStorage.setItem('lamad-progress-test-session-123-path1', JSON.stringify(progressData1));
      localStorage.setItem('lamad-progress-test-session-123-path2', JSON.stringify(progressData2));

      service.getAgentProgress().subscribe(progress => {
        expect(progress.length).toBeGreaterThanOrEqual(2);

        // Clean up
        localStorage.removeItem('lamad-progress-test-session-123-path1');
        localStorage.removeItem('lamad-progress-test-session-123-path2');

        done();
      });
    });
  });

  describe('clearProgressCache', () => {
    it('should clear progress cache', () => {
      service.clearProgressCache();
      // Should not throw
      expect(true).toBeTrue();
    });
  });

  // TODO(test-generator): [MEDIUM] Add tests for getLearningAnalytics
  // Context: getLearningAnalytics aggregates complex metrics from progress records
  // Story: Learning dashboard analytics and insights
  // Suggested approach:
  //   1. Mock localStorage with multiple progress records
  //   2. Verify streak calculation logic
  //   3. Test affinity averaging
  //   4. Verify attestation aggregation

  // TODO(test-generator): [LOW] Add tests for getLearningFrontier
  // Context: getLearningFrontier scans localStorage for active paths
  // Story: "Resume learning" feature in dashboard
  // Suggested approach:
  //   1. Mock localStorage with mix of completed/incomplete paths
  //   2. Verify only incomplete paths returned
  //   3. Test sorting by lastActivityAt
});
