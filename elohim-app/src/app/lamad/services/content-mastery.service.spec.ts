import { TestBed } from '@angular/core/testing';
import { HttpClientTestingModule } from '@angular/common/http/testing';
import { ContentMasteryService } from './content-mastery.service';
import { LocalSourceChainService } from '@app/elohim/services/local-source-chain.service';
import { SessionHumanService } from '@app/imagodei/services/session-human.service';
import { MasteryLevel, FRESHNESS_THRESHOLDS } from '../models';
import { SessionHuman } from '@app/imagodei/models/session-human.model';
import { BehaviorSubject } from 'rxjs';

describe('ContentMasteryService', () => {
  let service: ContentMasteryService;
  let sourceChainService: LocalSourceChainService;
  let sessionHumanService: jasmine.SpyObj<SessionHumanService>;
  let localStorageMock: { [key: string]: string };
  let mockStorage: Storage;
  let sessionSubject: BehaviorSubject<SessionHuman | null>;

  const TEST_SESSION_ID = 'session-test-123';
  const TEST_CONTENT_ID = 'content-abc-123';

  const createMockSession = (): SessionHuman => ({
    sessionId: TEST_SESSION_ID,
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
  });

  beforeEach(() => {
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

    // Create session subject for mock
    sessionSubject = new BehaviorSubject<SessionHuman | null>(createMockSession());

    // Create mock SessionHumanService
    sessionHumanService = jasmine.createSpyObj('SessionHumanService', [
      'getSession',
      'getSessionId',
      'recordContentView',
    ], {
      session$: sessionSubject.asObservable(),
    });
    sessionHumanService.getSession.and.returnValue(createMockSession());
    sessionHumanService.getSessionId.and.returnValue(TEST_SESSION_ID);

    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [
        ContentMasteryService,
        LocalSourceChainService,
        { provide: SessionHumanService, useValue: sessionHumanService },
      ]
    });

    sourceChainService = TestBed.inject(LocalSourceChainService);
    service = TestBed.inject(ContentMasteryService);
  });

  afterEach(() => {
    localStorageMock = {};
    if (sourceChainService.isInitialized()) {
      sourceChainService.resetChain();
    }
  });

  describe('Initialization', () => {
    it('should create the service', () => {
      expect(service).toBeTruthy();
    });

    it('should initialize source chain for session', () => {
      expect(sourceChainService.isInitialized()).toBe(true);
      expect(sourceChainService.getAgentId()).toBe(TEST_SESSION_ID);
    });
  });

  describe('Mastery Recording', () => {
    it('should record view and upgrade to seen', () => {
      service.recordView(TEST_CONTENT_ID);

      const level = service.getMasteryLevelSync(TEST_CONTENT_ID);
      expect(level).toBe('seen');
    });

    it('should not downgrade level on subsequent views', () => {
      service.setMasteryLevel(TEST_CONTENT_ID, 'understand');
      service.recordView(TEST_CONTENT_ID);

      const level = service.getMasteryLevelSync(TEST_CONTENT_ID);
      expect(level).toBe('understand');
    });

    it('should upgrade level on passing recall assessment', () => {
      service.recordView(TEST_CONTENT_ID);
      const newLevel = service.recordAssessment(TEST_CONTENT_ID, 'recall', 0.8);

      expect(newLevel).toBe('remember');
      expect(service.getMasteryLevelSync(TEST_CONTENT_ID)).toBe('remember');
    });

    it('should upgrade level on passing comprehension assessment', () => {
      service.recordView(TEST_CONTENT_ID);
      const newLevel = service.recordAssessment(TEST_CONTENT_ID, 'comprehension', 0.75);

      expect(newLevel).toBe('understand');
    });

    it('should upgrade level on passing application assessment', () => {
      service.recordView(TEST_CONTENT_ID);
      const newLevel = service.recordAssessment(TEST_CONTENT_ID, 'application', 0.9);

      expect(newLevel).toBe('apply');
    });

    it('should upgrade level on passing analysis assessment', () => {
      service.recordView(TEST_CONTENT_ID);
      const newLevel = service.recordAssessment(TEST_CONTENT_ID, 'analysis', 0.85);

      expect(newLevel).toBe('analyze');
    });

    it('should not upgrade level on failing assessment', () => {
      service.recordView(TEST_CONTENT_ID);
      const newLevel = service.recordAssessment(TEST_CONTENT_ID, 'comprehension', 0.5);

      expect(newLevel).toBe('seen');
      expect(service.getMasteryLevelSync(TEST_CONTENT_ID)).toBe('seen');
    });

    it('should not downgrade level on lower assessment', () => {
      service.setMasteryLevel(TEST_CONTENT_ID, 'analyze');
      const newLevel = service.recordAssessment(TEST_CONTENT_ID, 'recall', 0.9);

      expect(newLevel).toBe('analyze');
    });

    it('should set mastery level directly', () => {
      service.setMasteryLevel(TEST_CONTENT_ID, 'apply');

      expect(service.getMasteryLevelSync(TEST_CONTENT_ID)).toBe('apply');
    });

    it('should track level history', (done) => {
      service.setMasteryLevel(TEST_CONTENT_ID, 'seen');
      service.setMasteryLevel(TEST_CONTENT_ID, 'understand');
      service.setMasteryLevel(TEST_CONTENT_ID, 'apply');

      service.getMastery(TEST_CONTENT_ID).subscribe(mastery => {
        expect(mastery).toBeTruthy();
        expect(mastery?.levelHistory.length).toBe(2);
        expect(mastery?.levelHistory[0].fromLevel).toBe('seen');
        expect(mastery?.levelHistory[0].toLevel).toBe('understand');
        expect(mastery?.levelHistory[1].fromLevel).toBe('understand');
        expect(mastery?.levelHistory[1].toLevel).toBe('apply');
        done();
      });
    });

    it('should record activity with session human service', () => {
      service.recordView(TEST_CONTENT_ID);
      expect(sessionHumanService.recordContentView).toHaveBeenCalledWith(TEST_CONTENT_ID);
    });
  });

  describe('Mastery Queries', () => {
    beforeEach(() => {
      service.setMasteryLevel('content-1', 'seen');
      service.setMasteryLevel('content-2', 'understand');
      service.setMasteryLevel('content-3', 'apply');
      service.setMasteryLevel('content-4', 'analyze');
    });

    it('should get mastery for content', (done) => {
      service.getMastery('content-2').subscribe(mastery => {
        expect(mastery).toBeTruthy();
        expect(mastery?.level).toBe('understand');
        done();
      });
    });

    it('should return null for unknown content', (done) => {
      service.getMastery('unknown-content').subscribe(mastery => {
        expect(mastery).toBeNull();
        done();
      });
    });

    it('should get mastery level', (done) => {
      service.getMasteryLevel('content-3').subscribe(level => {
        expect(level).toBe('apply');
        done();
      });
    });

    it('should return not_started for unknown content level', (done) => {
      service.getMasteryLevel('unknown-content').subscribe(level => {
        expect(level).toBe('not_started');
        done();
      });
    });

    it('should get all mastery records', (done) => {
      service.getAllMastery().subscribe(all => {
        expect(all.length).toBe(4);
        done();
      });
    });
  });

  describe('Privileges', () => {
    it('should have view and practice privileges without mastery', (done) => {
      service.hasPrivilege('new-content', 'view').subscribe(has => {
        expect(has).toBe(true);
        done();
      });
    });

    it('should have practice privilege without mastery', (done) => {
      service.hasPrivilege('new-content', 'practice').subscribe(has => {
        expect(has).toBe(true);
        done();
      });
    });

    it('should not have comment privilege at seen level', (done) => {
      service.setMasteryLevel(TEST_CONTENT_ID, 'seen');

      service.hasPrivilege(TEST_CONTENT_ID, 'comment').subscribe(has => {
        expect(has).toBe(false);
        done();
      });
    });

    it('should have comment privilege at analyze level', (done) => {
      service.setMasteryLevel(TEST_CONTENT_ID, 'analyze');

      service.hasPrivilege(TEST_CONTENT_ID, 'comment').subscribe(has => {
        expect(has).toBe(true);
        done();
      });
    });

    it('should have peer_review privilege at evaluate level', (done) => {
      service.setMasteryLevel(TEST_CONTENT_ID, 'evaluate');

      service.hasPrivilege(TEST_CONTENT_ID, 'peer_review').subscribe(has => {
        expect(has).toBe(true);
        done();
      });
    });

    it('should have govern privilege at create level', (done) => {
      service.setMasteryLevel(TEST_CONTENT_ID, 'create');

      service.hasPrivilege(TEST_CONTENT_ID, 'govern').subscribe(has => {
        expect(has).toBe(true);
        done();
      });
    });

    it('should check privilege synchronously', () => {
      service.setMasteryLevel(TEST_CONTENT_ID, 'analyze');

      expect(service.hasPrivilegeSync(TEST_CONTENT_ID, 'comment')).toBe(true);
      expect(service.hasPrivilegeSync(TEST_CONTENT_ID, 'peer_review')).toBe(false);
    });

    it('should get all privileges for content', (done) => {
      service.setMasteryLevel(TEST_CONTENT_ID, 'apply');

      service.getPrivileges(TEST_CONTENT_ID).subscribe(privileges => {
        const viewPriv = privileges.find(p => p.privilege === 'view');
        const commentPriv = privileges.find(p => p.privilege === 'comment');

        expect(viewPriv?.active).toBe(true);
        expect(commentPriv?.active).toBe(false);
        done();
      });
    });
  });

  describe('Attestation Gate', () => {
    it('should not be above gate at seen level', () => {
      service.setMasteryLevel(TEST_CONTENT_ID, 'seen');
      expect(service.isAboveGate(TEST_CONTENT_ID)).toBe(false);
    });

    it('should not be above gate at understand level', () => {
      service.setMasteryLevel(TEST_CONTENT_ID, 'understand');
      expect(service.isAboveGate(TEST_CONTENT_ID)).toBe(false);
    });

    it('should be above gate at apply level', () => {
      service.setMasteryLevel(TEST_CONTENT_ID, 'apply');
      expect(service.isAboveGate(TEST_CONTENT_ID)).toBe(true);
    });

    it('should be above gate at analyze level', () => {
      service.setMasteryLevel(TEST_CONTENT_ID, 'analyze');
      expect(service.isAboveGate(TEST_CONTENT_ID)).toBe(true);
    });

    it('should be above gate at evaluate level', () => {
      service.setMasteryLevel(TEST_CONTENT_ID, 'evaluate');
      expect(service.isAboveGate(TEST_CONTENT_ID)).toBe(true);
    });

    it('should be above gate at create level', () => {
      service.setMasteryLevel(TEST_CONTENT_ID, 'create');
      expect(service.isAboveGate(TEST_CONTENT_ID)).toBe(true);
    });
  });

  describe('Freshness', () => {
    it('should have full freshness when just achieved', () => {
      service.setMasteryLevel(TEST_CONTENT_ID, 'understand');

      const mastery = service.getMasterySync(TEST_CONTENT_ID);
      expect(mastery?.freshness).toBe(1.0);
      expect(mastery?.needsRefresh).toBe(false);
    });

    it('should compute freshness decay', () => {
      // Manually create a mastery with old engagement date
      const oldMastery = {
        contentId: TEST_CONTENT_ID,
        humanId: TEST_SESSION_ID,
        level: 'understand' as MasteryLevel,
        levelAchievedAt: '2025-01-01T00:00:00.000Z',
        levelHistory: [],
        lastEngagementAt: '2024-01-01T00:00:00.000Z',  // 1 year ago
        lastEngagementType: 'view' as const,
        contentVersionAtMastery: '',
        freshness: 1.0,
        needsRefresh: false,
        assessmentEvidence: [],
        privileges: [],
      };

      const freshness = service.computeFreshness(oldMastery);
      expect(freshness).toBeLessThan(1.0);
      expect(freshness).toBeGreaterThan(0);
    });

    it('should identify content needing refresh', (done) => {
      service.setMasteryLevel('content-1', 'seen');
      service.setMasteryLevel('content-2', 'understand');

      service.getContentNeedingRefresh().subscribe(needing => {
        // Fresh content shouldn't need refresh
        expect(needing.length).toBe(0);
        done();
      });
    });
  });

  describe('Statistics', () => {
    beforeEach(() => {
      service.setMasteryLevel('content-1', 'seen');
      service.setMasteryLevel('content-2', 'remember');
      service.setMasteryLevel('content-3', 'understand');
      service.setMasteryLevel('content-4', 'apply');
      service.setMasteryLevel('content-5', 'analyze');
    });

    it('should compute total mastered nodes', (done) => {
      service.getMasteryStats().subscribe(stats => {
        expect(stats.totalMasteredNodes).toBe(5);
        done();
      });
    });

    it('should compute level distribution', (done) => {
      service.getMasteryStats().subscribe(stats => {
        expect(stats.levelDistribution.seen).toBe(1);
        expect(stats.levelDistribution.remember).toBe(1);
        expect(stats.levelDistribution.understand).toBe(1);
        expect(stats.levelDistribution.apply).toBe(1);
        expect(stats.levelDistribution.analyze).toBe(1);
        done();
      });
    });

    it('should compute nodes above gate', (done) => {
      service.getMasteryStats().subscribe(stats => {
        expect(stats.nodesAboveGate).toBe(2);  // apply and analyze
        done();
      });
    });

    it('should compute fresh percentage', (done) => {
      service.getMasteryStats().subscribe(stats => {
        expect(stats.freshPercentage).toBe(100);  // All fresh
        done();
      });
    });
  });

  describe('Source Chain Integration', () => {
    it('should store mastery entries on source chain', () => {
      service.setMasteryLevel(TEST_CONTENT_ID, 'understand');

      const entries = sourceChainService.getEntriesByType('mastery-record');
      expect(entries.length).toBeGreaterThan(0);

      const entry = entries.find(e =>
        (e.content as { contentId: string }).contentId === TEST_CONTENT_ID
      );
      expect(entry).toBeTruthy();
      expect((entry?.content as { level: string }).level).toBe('understand');
    });

    it('should create new entries for level changes (append-only)', () => {
      service.setMasteryLevel(TEST_CONTENT_ID, 'seen');
      service.setMasteryLevel(TEST_CONTENT_ID, 'understand');
      service.setMasteryLevel(TEST_CONTENT_ID, 'apply');

      const entries = sourceChainService.getEntriesByType('mastery-record');
      const contentEntries = entries.filter(e =>
        (e.content as { contentId: string }).contentId === TEST_CONTENT_ID
      );

      // Should have 3 entries (append-only)
      expect(contentEntries.length).toBe(3);
    });
  });
});
