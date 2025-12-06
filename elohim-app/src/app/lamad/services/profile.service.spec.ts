import { TestBed } from '@angular/core/testing';
import { of, throwError, BehaviorSubject } from 'rxjs';
import { ProfileService } from './profile.service';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';
import { PathService } from './path.service';
import { SessionHumanService } from './session-human.service';
import { AffinityTrackingService } from './affinity-tracking.service';
import { AgentService } from './agent.service';
import { LearningPath, PathStep, ContentNode, SessionPathProgress, SessionActivity } from '../models';

describe('ProfileService', () => {
  let service: ProfileService;
  let dataLoaderSpy: jasmine.SpyObj<DataLoaderService>;
  let pathServiceSpy: jasmine.SpyObj<PathService>;
  let sessionHumanSpy: jasmine.SpyObj<SessionHumanService>;
  let affinitySpy: jasmine.SpyObj<AffinityTrackingService>;
  let agentServiceSpy: jasmine.SpyObj<AgentService>;

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

  const mockContent: ContentNode = {
    id: 'resource-1',
    title: 'Test Resource',
    description: 'Test description',
    contentType: 'concept',
    contentFormat: 'markdown',
    content: '# Test',
    tags: [],
    relatedNodeIds: [],
    metadata: {}
  };

  const mockSession = {
    sessionId: 'session-123',
    displayName: 'Test User',
    isAnonymous: true as const,
    accessLevel: 'visitor' as const,
    createdAt: '2025-01-01T00:00:00.000Z',
    lastActiveAt: '2025-01-02T00:00:00.000Z',
    stats: {
      nodesViewed: 10,
      nodesWithAffinity: 3,
      pathsStarted: 2,
      pathsCompleted: 1,
      stepsCompleted: 5,
      totalSessionTime: 3600000,
      averageSessionLength: 720000,
      sessionCount: 5
    }
  };

  const mockPathProgress: SessionPathProgress[] = [
    {
      pathId: 'test-path',
      startedAt: '2025-01-01T00:00:00.000Z',
      currentStepIndex: 1,
      completedStepIndices: [0],
      lastActivityAt: '2025-01-02T00:00:00.000Z',
      stepNotes: { '0': 'My note for step 1' },
      stepAffinity: { 0: 0.8 }
    }
  ];

  const mockActivities: SessionActivity[] = [
    {
      type: 'path-start',
      resourceId: 'test-path',
      resourceType: 'path',
      timestamp: '2025-01-01T00:00:00.000Z'
    },
    {
      type: 'step-complete',
      resourceId: 'test-path',
      resourceType: 'step',
      timestamp: '2025-01-01T01:00:00.000Z',
      metadata: { stepIndex: 0 }
    },
    {
      type: 'affinity',
      resourceId: 'resource-1',
      resourceType: 'content',
      timestamp: '2025-01-01T02:00:00.000Z',
      metadata: { value: 0.8 }
    }
  ];

  beforeEach(() => {
    const dataLoaderSpyObj = jasmine.createSpyObj('DataLoaderService', [
      'getContent',
      'getPathIndex'
    ]);
    const pathServiceSpyObj = jasmine.createSpyObj('PathService', ['getPath']);
    const sessionHumanSpyObj = jasmine.createSpyObj('SessionHumanService', [
      'getSession',
      'getAllPathProgress',
      'getActivityHistory'
    ]);
    const affinitySpyObj = jasmine.createSpyObj('AffinityTrackingService', [], {
      affinitySubject: new BehaviorSubject({ affinity: { 'resource-1': 0.8, 'resource-2': 0.6 } })
    });
    const agentServiceSpyObj = jasmine.createSpyObj('AgentService', [
      'getAgent',
      'getCurrentAgent',
      'getAgentProgress',
      'getAttestations',
      'getCurrentAgentId'
    ]);

    TestBed.configureTestingModule({
      providers: [
        ProfileService,
        { provide: DataLoaderService, useValue: dataLoaderSpyObj },
        { provide: PathService, useValue: pathServiceSpyObj },
        { provide: SessionHumanService, useValue: sessionHumanSpyObj },
        { provide: AffinityTrackingService, useValue: affinitySpyObj },
        { provide: AgentService, useValue: agentServiceSpyObj }
      ]
    });

    dataLoaderSpy = TestBed.inject(DataLoaderService) as jasmine.SpyObj<DataLoaderService>;
    pathServiceSpy = TestBed.inject(PathService) as jasmine.SpyObj<PathService>;
    sessionHumanSpy = TestBed.inject(SessionHumanService) as jasmine.SpyObj<SessionHumanService>;
    affinitySpy = TestBed.inject(AffinityTrackingService) as jasmine.SpyObj<AffinityTrackingService>;
    agentServiceSpy = TestBed.inject(AgentService) as jasmine.SpyObj<AgentService>;

    // Default spy returns
    dataLoaderSpy.getContent.and.returnValue(of(mockContent));
    dataLoaderSpy.getPathIndex.and.returnValue(of({
      lastUpdated: '2025-01-01T00:00:00.000Z',
      totalCount: 1,
      paths: [{ id: 'test-path', title: 'Test Path', description: '', difficulty: 'beginner', estimatedDuration: '1h', stepCount: 3, tags: [] }]
    }));
    pathServiceSpy.getPath.and.returnValue(of(mockPath));
    sessionHumanSpy.getSession.and.returnValue(mockSession);
    sessionHumanSpy.getAllPathProgress.and.returnValue(mockPathProgress);
    sessionHumanSpy.getActivityHistory.and.returnValue(mockActivities);
    agentServiceSpy.getAgent.and.returnValue({ id: 'agent-1', displayName: 'Agent 1' } as any);
    agentServiceSpy.getCurrentAgent.and.returnValue(of({ id: 'agent-1', displayName: 'Agent 1' } as any));
    agentServiceSpy.getAgentProgress.and.returnValue(of([]));
    agentServiceSpy.getAttestations.and.returnValue(['community-member']);

    service = TestBed.inject(ProfileService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  // =========================================================================
  // Core Profile
  // =========================================================================

  describe('getProfile', () => {
    it('should return complete profile with session data', (done) => {
      service.getProfile().subscribe(profile => {
        expect(profile.id).toBe('session-123');
        expect(profile.displayName).toBe('Test User');
        expect(profile.isSessionBased).toBe(true);
        expect(profile.journeyStats).toBeDefined();
        expect(profile.currentFocus).toBeDefined();
        expect(profile.developedCapabilities).toBeDefined();
        done();
      });
    });

    it('should fall back to agent when no session', (done) => {
      sessionHumanSpy.getSession.and.returnValue(null);
      sessionHumanSpy.getAllPathProgress.and.returnValue([]);

      service.getProfile().subscribe(profile => {
        expect(profile.displayName).toBe('Agent 1');
        expect(profile.isSessionBased).toBe(false);
        done();
      });
    });
  });

  describe('getProfileSummary', () => {
    it('should return compact summary', (done) => {
      service.getProfileSummary().subscribe(summary => {
        expect(summary.displayName).toBe('Test User');
        expect(summary.journeysCompleted).toBe(1);
        expect(summary.currentFocusTitle).toBe('Test Path');
        done();
      });
    });
  });

  // =========================================================================
  // Journey Statistics
  // =========================================================================

  describe('getJourneyStats', () => {
    it('should return session-based stats', (done) => {
      service.getJourneyStats().subscribe(stats => {
        expect(stats.territoryExplored).toBe(10);
        expect(stats.journeysStarted).toBe(2);
        expect(stats.journeysCompleted).toBe(1);
        expect(stats.stepsCompleted).toBe(5);
        expect(stats.timeInvested).toBe(3600000);
        done();
      });
    });

    it('should calculate meaningful encounters from affinity data', (done) => {
      service.getJourneyStats().subscribe(stats => {
        // Two nodes with affinity > 0.5
        expect(stats.meaningfulEncounters).toBe(2);
        done();
      });
    });
  });

  // =========================================================================
  // Current Focus
  // =========================================================================

  describe('getCurrentFocus', () => {
    it('should return in-progress paths', (done) => {
      service.getCurrentFocus().subscribe(focus => {
        expect(focus.length).toBeGreaterThan(0);
        expect(focus[0].pathId).toBe('test-path');
        expect(focus[0].pathTitle).toBe('Test Path');
        done();
      });
    });

    it('should calculate progress percentage', (done) => {
      service.getCurrentFocus().subscribe(focus => {
        expect(focus[0].progressPercent).toBe(33); // 1/3 steps complete
        done();
      });
    });

    it('should include next step info', (done) => {
      service.getCurrentFocus().subscribe(focus => {
        expect(focus[0].nextStepTitle).toBe('Step 2');
        done();
      });
    });

    it('should return empty when no progress', (done) => {
      sessionHumanSpy.getAllPathProgress.and.returnValue([]);

      service.getCurrentFocus().subscribe(focus => {
        expect(focus.length).toBe(0);
        done();
      });
    });

    it('should exclude completed paths', (done) => {
      sessionHumanSpy.getAllPathProgress.and.returnValue([
        { ...mockPathProgress[0], completedAt: '2025-01-03T00:00:00.000Z' }
      ]);

      service.getCurrentFocus().subscribe(focus => {
        expect(focus.length).toBe(0);
        done();
      });
    });
  });

  // =========================================================================
  // Developed Capabilities
  // =========================================================================

  describe('getDevelopedCapabilities', () => {
    it('should return capabilities from attestations', (done) => {
      service.getDevelopedCapabilities().subscribe(capabilities => {
        expect(capabilities.length).toBe(1);
        expect(capabilities[0].id).toBe('community-member');
        done();
      });
    });

    it('should format attestation name', (done) => {
      service.getDevelopedCapabilities().subscribe(capabilities => {
        expect(capabilities[0].name).toBe('Community Member');
        done();
      });
    });
  });

  // =========================================================================
  // Timeline
  // =========================================================================

  describe('getTimeline', () => {
    it('should convert activities to timeline events', (done) => {
      service.getTimeline().subscribe(events => {
        expect(events.length).toBeGreaterThan(0);
        done();
      });
    });

    it('should include path-start events', (done) => {
      service.getTimeline().subscribe(events => {
        const pathStart = events.find(e => e.type === 'journey_started');
        expect(pathStart).toBeDefined();
        done();
      });
    });

    it('should include step-complete events', (done) => {
      service.getTimeline().subscribe(events => {
        const stepComplete = events.find(e => e.type === 'step_completed');
        expect(stepComplete).toBeDefined();
        done();
      });
    });

    it('should include high-affinity events as meaningful encounters', (done) => {
      service.getTimeline().subscribe(events => {
        const meaningful = events.find(e => e.type === 'meaningful_encounter');
        expect(meaningful).toBeDefined();
        done();
      });
    });

    it('should respect limit parameter', (done) => {
      service.getTimeline(1).subscribe(events => {
        expect(events.length).toBeLessThanOrEqual(1);
        done();
      });
    });
  });

  // =========================================================================
  // Top Engaged Content
  // =========================================================================

  describe('getTopEngagedContent', () => {
    it('should return content sorted by affinity', (done) => {
      service.getTopEngagedContent().subscribe(content => {
        expect(content.length).toBeGreaterThan(0);
        // First should have highest affinity
        expect(content[0].nodeId).toBe('resource-1');
        expect(content[0].affinity).toBe(0.8);
        done();
      });
    });

    it('should respect limit parameter', (done) => {
      service.getTopEngagedContent(1).subscribe(content => {
        expect(content.length).toBeLessThanOrEqual(1);
        done();
      });
    });

    it('should return empty when no affinity data', (done) => {
      (affinitySpy as any).affinitySubject.next({ affinity: {} });

      service.getTopEngagedContent().subscribe(content => {
        expect(content.length).toBe(0);
        done();
      });
    });
  });

  // =========================================================================
  // Notes
  // =========================================================================

  describe('getAllNotes', () => {
    it('should return notes from path progress', (done) => {
      service.getAllNotes().subscribe(notes => {
        expect(notes.length).toBeGreaterThan(0);
        expect(notes[0].content).toBe('My note for step 1');
        done();
      });
    });

    it('should enrich notes with path context', (done) => {
      service.getAllNotes().subscribe(notes => {
        expect(notes[0].context.pathTitle).toBe('Test Path');
        expect(notes[0].context.stepTitle).toBe('Step 1');
        done();
      });
    });

    it('should return empty when no notes', (done) => {
      sessionHumanSpy.getAllPathProgress.and.returnValue([
        { ...mockPathProgress[0], stepNotes: {} }
      ]);

      service.getAllNotes().subscribe(notes => {
        expect(notes.length).toBe(0);
        done();
      });
    });
  });

  // =========================================================================
  // Resume Point
  // =========================================================================

  describe('getResumePoint', () => {
    it('should return continue_path when in-progress path exists', (done) => {
      service.getResumePoint().subscribe(resume => {
        expect(resume?.type).toBe('continue_path');
        expect(resume?.title).toContain('Test Path');
        done();
      });
    });

    it('should return explore_new when no in-progress paths', (done) => {
      sessionHumanSpy.getAllPathProgress.and.returnValue([]);

      service.getResumePoint().subscribe(resume => {
        expect(resume?.type).toBe('explore_new');
        done();
      });
    });

    it('should include days since active', (done) => {
      service.getResumePoint().subscribe(resume => {
        expect(resume?.daysSinceActive).toBeDefined();
        done();
      });
    });
  });

  // =========================================================================
  // Paths Overview
  // =========================================================================

  describe('getPathsOverview', () => {
    it('should categorize paths correctly', (done) => {
      service.getPathsOverview().subscribe(overview => {
        expect(overview.inProgress.length).toBe(1);
        expect(overview.inProgress[0].pathId).toBe('test-path');
        done();
      });
    });

    it('should include completed paths', (done) => {
      sessionHumanSpy.getAllPathProgress.and.returnValue([
        { ...mockPathProgress[0], completedAt: '2025-01-03T00:00:00.000Z' }
      ]);

      service.getPathsOverview().subscribe(overview => {
        expect(overview.completed.length).toBe(1);
        expect(overview.inProgress.length).toBe(0);
        done();
      });
    });

    it('should calculate progress for each path', (done) => {
      service.getPathsOverview().subscribe(overview => {
        expect(overview.inProgress[0].progressPercent).toBe(33);
        expect(overview.inProgress[0].completedSteps).toBe(1);
        expect(overview.inProgress[0].totalSteps).toBe(3);
        done();
      });
    });
  });

  // =========================================================================
  // Error Handling
  // =========================================================================

  describe('error handling', () => {
    it('should handle path load errors in getCurrentFocus', (done) => {
      pathServiceSpy.getPath.and.returnValue(throwError(() => new Error('Not found')));

      service.getCurrentFocus().subscribe(focus => {
        // Should return empty or filtered results
        expect(focus.length).toBe(0);
        done();
      });
    });

    it('should handle content load errors in getTopEngagedContent', (done) => {
      dataLoaderSpy.getContent.and.returnValue(throwError(() => new Error('Not found')));

      service.getTopEngagedContent().subscribe(content => {
        expect(content.length).toBe(0);
        done();
      });
    });
  });
});
