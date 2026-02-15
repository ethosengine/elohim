import { TestBed } from '@angular/core/testing';

import { BehaviorSubject, of, Subject } from 'rxjs';

import { LocalSourceChainService } from '@app/elohim/services/local-source-chain.service';
import { SessionHumanService } from '@app/imagodei/services/session-human.service';

import { ContentMasteryService } from './content-mastery.service';
import { MasteryStatsService } from './mastery-stats.service';
import { PointsService } from './points.service';
import { PracticeService } from './practice.service';

import type { SessionHuman } from '@app/imagodei/models/session-human.model';

import type { ContentMastery } from '../models';
import type { LevelUpEvent } from '../models/learner-mastery-profile.model';

describe('MasteryStatsService', () => {
  let service: MasteryStatsService;
  let mockMasteryService: jasmine.SpyObj<ContentMasteryService>;
  let mockPointsService: jasmine.SpyObj<PointsService>;
  let mockPracticeService: jasmine.SpyObj<PracticeService>;
  let mockSessionHuman: jasmine.SpyObj<SessionHumanService>;
  let mockSourceChain: jasmine.SpyObj<LocalSourceChainService>;

  let masterySubject: BehaviorSubject<Map<string, ContentMastery>>;
  let totalPointsSubject: BehaviorSubject<number>;
  let levelUpSubject: Subject<LevelUpEvent>;
  let sessionSubject: BehaviorSubject<SessionHuman | null>;
  let poolSubject: BehaviorSubject<unknown>;

  const createMockMastery = (
    contentId: string,
    level: 'not_started' | 'seen' | 'remember' | 'understand' | 'apply' | 'analyze' | 'evaluate' | 'create',
    freshness = 1.0
  ): ContentMastery => ({
    contentId,
    humanId: 'test-session',
    level,
    levelAchievedAt: new Date().toISOString(),
    levelHistory: [],
    lastEngagementAt: new Date().toISOString(),
    lastEngagementType: 'view',
    contentVersionAtMastery: '1.0',
    freshness,
    needsRefresh: freshness < 0.7,
    assessmentEvidence: [],
    privileges: [],
  });

  const createMockSession = (): SessionHuman => ({
    sessionId: 'test-session',
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
    masterySubject = new BehaviorSubject<Map<string, ContentMastery>>(new Map());
    totalPointsSubject = new BehaviorSubject<number>(0);
    levelUpSubject = new Subject<LevelUpEvent>();
    sessionSubject = new BehaviorSubject<SessionHuman | null>(createMockSession());
    poolSubject = new BehaviorSubject<unknown>(null);

    mockMasteryService = jasmine.createSpyObj(
      'ContentMasteryService',
      [],
      {
        mastery$: masterySubject.asObservable(),
        levelUp$: levelUpSubject.asObservable(),
      }
    );

    mockPointsService = jasmine.createSpyObj(
      'PointsService',
      ['earnPoints', 'refreshBalance'],
      {
        totalPoints$: totalPointsSubject.asObservable(),
      }
    );
    mockPointsService.earnPoints.and.returnValue(of(null));

    mockPracticeService = jasmine.createSpyObj(
      'PracticeService',
      [],
      {
        pool$: poolSubject,
      }
    );

    mockSessionHuman = jasmine.createSpyObj(
      'SessionHumanService',
      [],
      {
        session$: sessionSubject.asObservable(),
      }
    );

    mockSourceChain = jasmine.createSpyObj('LocalSourceChainService', [
      'isInitialized',
      'createEntry',
      'getEntriesByType',
    ]);
    mockSourceChain.isInitialized.and.returnValue(true);
    mockSourceChain.getEntriesByType.and.returnValue([]);

    TestBed.configureTestingModule({
      providers: [
        MasteryStatsService,
        { provide: ContentMasteryService, useValue: mockMasteryService },
        { provide: PointsService, useValue: mockPointsService },
        { provide: PracticeService, useValue: mockPracticeService },
        { provide: SessionHumanService, useValue: mockSessionHuman },
        { provide: LocalSourceChainService, useValue: mockSourceChain },
      ],
    });

    service = TestBed.inject(MasteryStatsService);
  });

  afterEach(() => {
    service.ngOnDestroy();
  });

  describe('Initialization', () => {
    it('should create the service', () => {
      expect(service).toBeTruthy();
    });

    it('should have learnerProfile$ observable', () => {
      expect(service.learnerProfile$).toBeDefined();
      expect(service.learnerProfile$.subscribe).toBeDefined();
    });

    it('should have streakInfo$ observable', () => {
      expect(service.streakInfo$).toBeDefined();
      expect(service.streakInfo$.subscribe).toBeDefined();
    });

    it('should have recentLevelUps$ observable', () => {
      expect(service.recentLevelUps$).toBeDefined();
      expect(service.recentLevelUps$.subscribe).toBeDefined();
    });
  });

  describe('Empty State', () => {
    it('should produce learner profile with zero values when initialized', done => {
      service.learnerProfile$.subscribe(profile => {
        expect(profile).toBeDefined();
        expect(profile?.totalMasteredNodes).toBe(0);
        expect(profile?.earnedPoints).toBe(0);
        expect(profile?.nodesAboveGate).toBe(0);
        done();
      });
    });

    it('should produce empty streak info initially', done => {
      service.streakInfo$.subscribe(streak => {
        expect(streak).toBeDefined();
        expect(streak.currentStreak).toBe(0);
        expect(streak.bestStreak).toBe(0);
        expect(streak.todayActive).toBe(false);
        expect(streak.lastActiveDate).toBe('');
        done();
      });
    });

    it('should produce empty level ups initially', done => {
      service.recentLevelUps$.subscribe(levelUps => {
        expect(levelUps).toEqual([]);
        done();
      });
    });
  });

  describe('Profile Computation', () => {
    it('should compute profile when mastery and points update', done => {
      const masteries = new Map<string, ContentMastery>([
        ['content-1', createMockMastery('content-1', 'seen')],
        ['content-2', createMockMastery('content-2', 'remember')],
        ['content-3', createMockMastery('content-3', 'apply')],
      ]);

      masterySubject.next(masteries);
      totalPointsSubject.next(500);

      setTimeout(() => {
        service.learnerProfile$.subscribe(profile => {
          expect(profile).toBeDefined();
          expect(profile?.totalMasteredNodes).toBe(3);
          expect(profile?.earnedPoints).toBe(500);
          expect(profile?.levelDistribution.seen).toBe(1);
          expect(profile?.levelDistribution.remember).toBe(1);
          expect(profile?.levelDistribution.apply).toBe(1);
          done();
        });
      }, 50);
    });

    it('should exclude not_started from totalMasteredNodes', done => {
      const masteries = new Map<string, ContentMastery>([
        ['content-1', createMockMastery('content-1', 'not_started')],
        ['content-2', createMockMastery('content-2', 'seen')],
      ]);

      masterySubject.next(masteries);

      setTimeout(() => {
        service.learnerProfile$.subscribe(profile => {
          expect(profile?.totalMasteredNodes).toBe(1);
          done();
        });
      }, 50);
    });

    it('should count nodesAboveGate correctly', done => {
      const masteries = new Map<string, ContentMastery>([
        ['content-1', createMockMastery('content-1', 'seen')],
        ['content-2', createMockMastery('content-2', 'understand')],
        ['content-3', createMockMastery('content-3', 'apply')],
        ['content-4', createMockMastery('content-4', 'analyze')],
      ]);

      masterySubject.next(masteries);

      setTimeout(() => {
        service.learnerProfile$.subscribe(profile => {
          expect(profile?.nodesAboveGate).toBe(2);
          done();
        });
      }, 50);
    });

    it('should compute level distribution correctly', done => {
      const masteries = new Map<string, ContentMastery>([
        ['content-1', createMockMastery('content-1', 'seen')],
        ['content-2', createMockMastery('content-2', 'seen')],
        ['content-3', createMockMastery('content-3', 'remember')],
        ['content-4', createMockMastery('content-4', 'understand')],
        ['content-5', createMockMastery('content-5', 'apply')],
        ['content-6', createMockMastery('content-6', 'apply')],
        ['content-7', createMockMastery('content-7', 'apply')],
      ]);

      masterySubject.next(masteries);

      setTimeout(() => {
        service.learnerProfile$.subscribe(profile => {
          expect(profile?.levelDistribution.seen).toBe(2);
          expect(profile?.levelDistribution.remember).toBe(1);
          expect(profile?.levelDistribution.understand).toBe(1);
          expect(profile?.levelDistribution.apply).toBe(3);
          expect(profile?.levelDistribution.analyze).toBe(0);
          done();
        });
      }, 50);
    });
  });

  describe('getProfileSync()', () => {
    it('should return profile with zero values after initialization', () => {
      const profile = service.getProfileSync();
      expect(profile).toBeDefined();
      expect(profile?.totalMasteredNodes).toBe(0);
    });

    it('should return cached profile after computation', done => {
      const masteries = new Map<string, ContentMastery>([
        ['content-1', createMockMastery('content-1', 'seen')],
      ]);

      masterySubject.next(masteries);

      setTimeout(() => {
        const profile = service.getProfileSync();
        expect(profile).toBeDefined();
        expect(profile?.totalMasteredNodes).toBe(1);
        done();
      }, 50);
    });
  });

  describe('refreshProfile()', () => {
    it('should call points.refreshBalance', () => {
      service.refreshProfile();
      expect(mockPointsService.refreshBalance).toHaveBeenCalled();
    });

    it('should reload streak from source chain', () => {
      service.refreshProfile();
      expect(mockSourceChain.getEntriesByType).toHaveBeenCalled();
    });
  });

  describe('Streak Management', () => {
    it('should record daily engagement', () => {
      service.recordDailyEngagement('view');
      expect(mockSourceChain.createEntry).toHaveBeenCalledWith(
        'streak-record',
        jasmine.objectContaining({
          engagementTypes: ['view'],
        })
      );
    });

    it('should not record duplicate entries for same day', () => {
      const today = new Date().toISOString().slice(0, 10);

      // First call should create entry
      service.recordDailyEngagement('view');
      expect(mockSourceChain.createEntry).toHaveBeenCalledTimes(1);

      // Mock the source chain to return the entry we just created
      mockSourceChain.getEntriesByType.and.returnValue([
        {
          entryHash: 'hash-1',
          content: {
            activeDate: today,
            engagementTypes: ['view'],
          },
          timestamp: new Date().toISOString(),
          authorAgent: 'test-session',
          entryType: 'streak-record',
        },
      ]);

      // Reload streak to update todayActive flag
      service.refreshProfile();

      // Second call for same day should not create another entry
      service.recordDailyEngagement('view');
      expect(mockSourceChain.createEntry).toHaveBeenCalledTimes(1);
    });
  });

  describe('Level-Up Orchestration', () => {
    it('should handle level-up event', () => {
      const event: LevelUpEvent = {
        id: 'levelup-1',
        contentId: 'content-1',
        fromLevel: 'seen',
        toLevel: 'remember',
        timestamp: new Date().toISOString(),
        pointsEarned: 10,
        isGateLevel: false,
      };

      levelUpSubject.next(event);

      expect(mockSourceChain.createEntry).toHaveBeenCalledWith(
        'mastery-level-up',
        jasmine.objectContaining({
          contentId: 'content-1',
          fromLevel: 'seen',
          toLevel: 'remember',
        })
      );
    });

    it('should earn points on level-up', () => {
      const event: LevelUpEvent = {
        id: 'levelup-1',
        contentId: 'content-1',
        fromLevel: 'seen',
        toLevel: 'remember',
        timestamp: new Date().toISOString(),
        pointsEarned: 10,
        isGateLevel: false,
      };

      levelUpSubject.next(event);

      expect(mockPointsService.earnPoints).toHaveBeenCalledWith('level_up', 'content-1');
    });

    it('should record daily engagement on level-up', () => {
      const event: LevelUpEvent = {
        id: 'levelup-1',
        contentId: 'content-1',
        fromLevel: 'seen',
        toLevel: 'remember',
        timestamp: new Date().toISOString(),
        pointsEarned: 10,
        isGateLevel: false,
      };

      levelUpSubject.next(event);

      expect(mockSourceChain.createEntry).toHaveBeenCalledWith(
        'streak-record',
        jasmine.objectContaining({
          engagementTypes: ['level_up'],
        })
      );
    });

    it('should update recent level-ups list', done => {
      const event: LevelUpEvent = {
        id: 'levelup-1',
        contentId: 'content-1',
        fromLevel: 'seen',
        toLevel: 'remember',
        timestamp: new Date().toISOString(),
        pointsEarned: 10,
        isGateLevel: false,
      };

      levelUpSubject.next(event);

      setTimeout(() => {
        service.recentLevelUps$.subscribe(levelUps => {
          expect(levelUps.length).toBe(1);
          expect(levelUps[0].contentId).toBe('content-1');
          done();
        });
      }, 50);
    });
  });

  describe('recordLevelUp()', () => {
    it('should manually record level-up event', () => {
      const event: LevelUpEvent = {
        id: 'levelup-1',
        contentId: 'content-1',
        fromLevel: 'remember',
        toLevel: 'understand',
        timestamp: new Date().toISOString(),
        pointsEarned: 15,
        isGateLevel: false,
      };

      service.recordLevelUp(event);

      expect(mockSourceChain.createEntry).toHaveBeenCalled();
      expect(mockPointsService.earnPoints).toHaveBeenCalled();
    });
  });

  describe('Practice Summary', () => {
    it('should return empty summary when pool is null', done => {
      poolSubject.next(null);

      setTimeout(() => {
        service.learnerProfile$.subscribe(profile => {
          expect(profile?.practice.totalChallenges).toBe(0);
          expect(profile?.practice.totalLevelUps).toBe(0);
          expect(profile?.practice.totalLevelDowns).toBe(0);
          expect(profile?.practice.totalDiscoveries).toBe(0);
          done();
        });
      }, 50);
    });

    it('should extract practice stats from pool', done => {
      // Update pool before mastery/points to ensure it's there when profile computes
      poolSubject.next({
        total_challenges_taken: 25,
        total_level_ups: 10,
        total_level_downs: 3,
        discoveries_unlocked: 5,
        active_content_ids_json: '["c1","c2","c3"]',
        refresh_queue_ids_json: '["c4","c5"]',
      });

      // Trigger profile recomputation by updating mastery
      masterySubject.next(new Map([['content-1', createMockMastery('content-1', 'seen')]]));

      setTimeout(() => {
        const profile = service.getProfileSync();
        expect(profile?.practice.totalChallenges).toBe(25);
        expect(profile?.practice.totalLevelUps).toBe(10);
        expect(profile?.practice.totalLevelDowns).toBe(3);
        expect(profile?.practice.totalDiscoveries).toBe(5);
        expect(profile?.practice.activePoolSize).toBe(3);
        expect(profile?.practice.refreshQueueSize).toBe(2);
        done();
      }, 100);
    });
  });

  describe('Source Chain Integration', () => {
    it('should not create entries when source chain not initialized', () => {
      mockSourceChain.isInitialized.and.returnValue(false);

      service.recordDailyEngagement('view');

      expect(mockSourceChain.createEntry).not.toHaveBeenCalled();
    });

    it('should load streak from source chain on session start', () => {
      sessionSubject.next(createMockSession());

      expect(mockSourceChain.getEntriesByType).toHaveBeenCalledWith('streak-record');
    });

    it('should load level-ups from source chain on session start', () => {
      sessionSubject.next(createMockSession());

      expect(mockSourceChain.getEntriesByType).toHaveBeenCalledWith('mastery-level-up');
    });
  });
});
