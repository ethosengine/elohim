import { TestBed } from '@angular/core/testing';

import { LearnerBackendService } from './learner-backend.service';
import { HolochainClientService } from './holochain-client.service';

describe('LearnerBackendService', () => {
  let service: LearnerBackendService;
  let mockHolochainClient: jasmine.SpyObj<HolochainClientService>;

  beforeEach(() => {
    mockHolochainClient = jasmine.createSpyObj('HolochainClientService', [
      'callZome',
      'isConnected',
    ]);
    mockHolochainClient.isConnected.and.returnValue(true);

    TestBed.configureTestingModule({
      providers: [
        LearnerBackendService,
        { provide: HolochainClientService, useValue: mockHolochainClient },
      ],
    });

    service = TestBed.inject(LearnerBackendService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  // ===========================================================================
  // Connection Status
  // ===========================================================================

  describe('isAvailable', () => {
    it('should have isAvailable method', () => {
      expect(service.isAvailable).toBeDefined();
      expect(typeof service.isAvailable).toBe('function');
    });

    it('should return true when holochain is connected', () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      const result = service.isAvailable();
      expect(result).toBe(true);
    });

    it('should return false when holochain is disconnected', () => {
      mockHolochainClient.isConnected.and.returnValue(false);
      const result = service.isAvailable();
      expect(result).toBe(false);
    });
  });

  // ===========================================================================
  // Content Mastery Operations
  // ===========================================================================

  describe('initializeMastery', () => {
    it('should have initializeMastery method', () => {
      expect(service.initializeMastery).toBeDefined();
      expect(typeof service.initializeMastery).toBe('function');
    });

    it('should return mastery output on success', async () => {
      const mockOutput = { id: 'mastery-1', level: 'not_started' } as any;
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockOutput })
      );

      const result = await service.initializeMastery('content-123');

      expect(result).toEqual(mockOutput);
      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          zomeName: 'content_store',
          fnName: 'initialize_mastery',
          payload: { content_id: 'content-123' },
        })
      );
    });

    it('should return null on zome call failure', async () => {
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: false })
      );

      const result = await service.initializeMastery('content-123');

      expect(result).toBeNull();
    });

    it('should return null when zome returns no data', async () => {
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: null })
      );

      const result = await service.initializeMastery('content-123');

      expect(result).toBeNull();
    });
  });

  describe('recordEngagement', () => {
    it('should have recordEngagement method', () => {
      expect(service.recordEngagement).toBeDefined();
      expect(typeof service.recordEngagement).toBe('function');
    });

    it('should call zome with correct engagement input', async () => {
      const mockOutput = { id: 'mastery-1', level: 'practicing' } as any;
      const engagementInput = {
        content_id: 'content-123',
        engagement_type: 'view',
      };

      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockOutput })
      );

      const result = await service.recordEngagement(engagementInput);

      expect(result).toEqual(mockOutput);
      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          fnName: 'record_engagement',
          payload: engagementInput,
        })
      );
    });

    it('should return null on engagement failure', async () => {
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: false })
      );

      const result = await service.recordEngagement({
        content_id: 'content-123',
        engagement_type: 'view',
      });

      expect(result).toBeNull();
    });
  });

  describe('recordAssessment', () => {
    it('should have recordAssessment method', () => {
      expect(service.recordAssessment).toBeDefined();
      expect(typeof service.recordAssessment).toBe('function');
    });

    it('should call zome with assessment input', async () => {
      const mockOutput = { id: 'mastery-1', level: 'mastered' } as any;
      const assessmentInput = {
        content_id: 'content-123',
        assessment_type: 'quiz',
        score: 95,
        passing_threshold: 70,
        time_spent_seconds: 300,
        question_count: 10,
        correct_count: 9,
      };

      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockOutput })
      );

      const result = await service.recordAssessment(assessmentInput);

      expect(result).toEqual(mockOutput);
      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          fnName: 'record_assessment',
          payload: assessmentInput,
        })
      );
    });

    it('should return null on assessment failure', async () => {
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: false })
      );

      const result = await service.recordAssessment({
        content_id: 'content-123',
        assessment_type: 'quiz',
        score: 50,
        passing_threshold: 70,
        time_spent_seconds: 200,
        question_count: 10,
        correct_count: 5,
      });

      expect(result).toBeNull();
    });
  });

  describe('getMyMastery', () => {
    it('should have getMyMastery method', () => {
      expect(service.getMyMastery).toBeDefined();
      expect(typeof service.getMyMastery).toBe('function');
    });

    it('should return mastery for specific content', async () => {
      const mockOutput = { id: 'mastery-1', level: 'practicing' } as any;
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockOutput })
      );

      const result = await service.getMyMastery('content-123');

      expect(result).toEqual(mockOutput);
      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          fnName: 'get_my_mastery',
          payload: 'content-123',
        })
      );
    });

    it('should return null when mastery not found', async () => {
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: false })
      );

      const result = await service.getMyMastery('content-123');

      expect(result).toBeNull();
    });
  });

  describe('getMyAllMastery', () => {
    it('should have getMyAllMastery method', () => {
      expect(service.getMyAllMastery).toBeDefined();
      expect(typeof service.getMyAllMastery).toBe('function');
    });

    it('should return array of mastery records', async () => {
      const mockOutput = [
        { id: 'mastery-1', level: 'practicing' },
        { id: 'mastery-2', level: 'mastered' },
      ] as any;

      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockOutput })
      );

      const result = await service.getMyAllMastery();

      expect(result).toEqual(mockOutput);
      expect(Array.isArray(result)).toBe(true);
    });

    it('should return empty array on failure', async () => {
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: false })
      );

      const result = await service.getMyAllMastery();

      expect(result).toEqual([]);
    });

    it('should return empty array when data is null', async () => {
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: null })
      );

      const result = await service.getMyAllMastery();

      expect(result).toEqual([]);
    });
  });

  describe('getMasteryBatch', () => {
    it('should have getMasteryBatch method', () => {
      expect(service.getMasteryBatch).toBeDefined();
      expect(typeof service.getMasteryBatch).toBe('function');
    });

    it('should return batch mastery snapshots', async () => {
      const mockOutput = [
        { contentId: 'c1', level: 'practicing' },
        { contentId: 'c2', level: 'mastered' },
      ] as any;

      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockOutput })
      );

      const result = await service.getMasteryBatch(['c1', 'c2']);

      expect(result).toEqual(mockOutput);
      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          fnName: 'get_mastery_batch',
          payload: ['c1', 'c2'],
        })
      );
    });

    it('should return empty array on failure', async () => {
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: false })
      );

      const result = await service.getMasteryBatch(['c1', 'c2']);

      expect(result).toEqual([]);
    });
  });

  describe('getPathMasteryOverview', () => {
    it('should have getPathMasteryOverview method', () => {
      expect(service.getPathMasteryOverview).toBeDefined();
      expect(typeof service.getPathMasteryOverview).toBe('function');
    });

    it('should return path mastery overview', async () => {
      const mockOutput = { pathId: 'path-1', nodes: [] } as any;
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockOutput })
      );

      const result = await service.getPathMasteryOverview('path-1');

      expect(result).toEqual(mockOutput);
      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          fnName: 'get_path_mastery_overview',
          payload: 'path-1',
        })
      );
    });

    it('should return null when overview not available', async () => {
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: false })
      );

      const result = await service.getPathMasteryOverview('path-1');

      expect(result).toBeNull();
    });
  });

  describe('getMyMasteryStats', () => {
    it('should have getMyMasteryStats method', () => {
      expect(service.getMyMasteryStats).toBeDefined();
      expect(typeof service.getMyMasteryStats).toBe('function');
    });

    it('should return mastery statistics', async () => {
      const mockOutput = { nodesAttempted: 10, nodesMastered: 5 } as any;
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockOutput })
      );

      const result = await service.getMyMasteryStats();

      expect(result).toEqual(mockOutput);
      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          fnName: 'get_my_mastery_stats',
          payload: null,
        })
      );
    });

    it('should return null when stats unavailable', async () => {
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: false })
      );

      const result = await service.getMyMasteryStats();

      expect(result).toBeNull();
    });
  });

  describe('checkPrivilege', () => {
    it('should have checkPrivilege method', () => {
      expect(service.checkPrivilege).toBeDefined();
      expect(typeof service.checkPrivilege).toBe('function');
    });

    it('should check privilege for content', async () => {
      const mockOutput = { allowed: true, reason: 'Licensed' } as any;
      const checkInput = { content_id: 'content-123', privilege: 'view' };

      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockOutput })
      );

      const result = await service.checkPrivilege(checkInput);

      expect(result).toEqual(mockOutput);
      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          fnName: 'check_privilege',
          payload: checkInput,
        })
      );
    });

    it('should return null on privilege check failure', async () => {
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: false })
      );

      const result = await service.checkPrivilege({
        content_id: 'content-123',
        privilege: 'view',
      });

      expect(result).toBeNull();
    });
  });

  // ===========================================================================
  // Practice Pool Operations
  // ===========================================================================

  describe('getOrCreatePracticePool', () => {
    it('should have getOrCreatePracticePool method', () => {
      expect(service.getOrCreatePracticePool).toBeDefined();
      expect(typeof service.getOrCreatePracticePool).toBe('function');
    });

    it('should create or retrieve practice pool', async () => {
      const mockOutput = { id: 'pool-1', contentIds: [] } as any;
      const poolInput = { contributing_path_ids: ['path-1', 'path-2'] };

      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockOutput })
      );

      const result = await service.getOrCreatePracticePool(poolInput);

      expect(result).toEqual(mockOutput);
      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          fnName: 'get_or_create_practice_pool',
          payload: poolInput,
        })
      );
    });

    it('should return null on pool creation failure', async () => {
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: false })
      );

      const result = await service.getOrCreatePracticePool({
        contributing_path_ids: ['path-1'],
      });

      expect(result).toBeNull();
    });
  });

  describe('refreshPracticePool', () => {
    it('should have refreshPracticePool method', () => {
      expect(service.refreshPracticePool).toBeDefined();
      expect(typeof service.refreshPracticePool).toBe('function');
    });

    it('should refresh practice pool', async () => {
      const mockOutput = { id: 'pool-1', contentIds: ['c1', 'c2'] } as any;
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockOutput })
      );

      const result = await service.refreshPracticePool();

      expect(result).toEqual(mockOutput);
      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          fnName: 'refresh_practice_pool',
          payload: null,
        })
      );
    });

    it('should return null on refresh failure', async () => {
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: false })
      );

      const result = await service.refreshPracticePool();

      expect(result).toBeNull();
    });
  });

  describe('addPathToPool', () => {
    it('should have addPathToPool method', () => {
      expect(service.addPathToPool).toBeDefined();
      expect(typeof service.addPathToPool).toBe('function');
    });

    it('should add path to practice pool', async () => {
      const mockOutput = { id: 'pool-1', contentIds: ['c1', 'c2', 'c3'] } as any;
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockOutput })
      );

      const result = await service.addPathToPool('path-1');

      expect(result).toEqual(mockOutput);
      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          fnName: 'add_path_to_pool',
          payload: 'path-1',
        })
      );
    });

    it('should return null on add failure', async () => {
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: false })
      );

      const result = await service.addPathToPool('path-1');

      expect(result).toBeNull();
    });
  });

  describe('getPoolRecommendations', () => {
    it('should have getPoolRecommendations method', () => {
      expect(service.getPoolRecommendations).toBeDefined();
      expect(typeof service.getPoolRecommendations).toBe('function');
    });

    it('should return pool recommendations', async () => {
      const mockOutput = { recommendations: ['c1', 'c2', 'c3'] } as any;
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockOutput })
      );

      const result = await service.getPoolRecommendations();

      expect(result).toEqual(mockOutput);
      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          fnName: 'get_pool_recommendations',
          payload: null,
        })
      );
    });

    it('should return null on recommendation failure', async () => {
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: false })
      );

      const result = await service.getPoolRecommendations();

      expect(result).toBeNull();
    });
  });

  describe('checkChallengeCooldown', () => {
    it('should have checkChallengeCooldown method', () => {
      expect(service.checkChallengeCooldown).toBeDefined();
      expect(typeof service.checkChallengeCooldown).toBe('function');
    });

    it('should check challenge cooldown status', async () => {
      const mockOutput = { allowed: true, nextAvailableAt: 0 } as any;
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockOutput })
      );

      const result = await service.checkChallengeCooldown();

      expect(result).toEqual(mockOutput);
      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          fnName: 'check_challenge_cooldown',
          payload: null,
        })
      );
    });

    it('should return null on cooldown check failure', async () => {
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: false })
      );

      const result = await service.checkChallengeCooldown();

      expect(result).toBeNull();
    });
  });

  // ===========================================================================
  // Mastery Challenge Operations
  // ===========================================================================

  describe('startMasteryChallenge', () => {
    it('should have startMasteryChallenge method', () => {
      expect(service.startMasteryChallenge).toBeDefined();
      expect(typeof service.startMasteryChallenge).toBe('function');
    });

    it('should start mastery challenge', async () => {
      const mockOutput = { id: 'challenge-1', questions: [] } as any;
      const challengeInput = { question_count: 5, include_discoveries: true };

      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockOutput })
      );

      const result = await service.startMasteryChallenge(challengeInput);

      expect(result).toEqual(mockOutput);
      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          fnName: 'start_mastery_challenge',
          payload: challengeInput,
        })
      );
    });

    it('should return null on challenge start failure', async () => {
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: false })
      );

      const result = await service.startMasteryChallenge({
        question_count: 5,
        include_discoveries: false,
      });

      expect(result).toBeNull();
    });
  });

  describe('submitMasteryChallenge', () => {
    it('should have submitMasteryChallenge method', () => {
      expect(service.submitMasteryChallenge).toBeDefined();
      expect(typeof service.submitMasteryChallenge).toBe('function');
    });

    it('should submit challenge responses', async () => {
      const mockOutput = { challengeId: 'challenge-1', score: 80 } as any;
      const submitInput = {
        challenge_id: 'challenge-1',
        responses: [
          { content_id: 'c1', question_index: 0, response: 'A', correct: true, time_taken_ms: 5000 },
        ],
        actual_time_seconds: 120,
      };

      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockOutput })
      );

      const result = await service.submitMasteryChallenge(submitInput);

      expect(result).toEqual(mockOutput);
      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          fnName: 'submit_mastery_challenge',
          payload: submitInput,
        })
      );
    });

    it('should return null on challenge submission failure', async () => {
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: false })
      );

      const result = await service.submitMasteryChallenge({
        challenge_id: 'challenge-1',
        responses: [],
        actual_time_seconds: 0,
      });

      expect(result).toBeNull();
    });
  });

  describe('getChallengeHistory', () => {
    it('should have getChallengeHistory method', () => {
      expect(service.getChallengeHistory).toBeDefined();
      expect(typeof service.getChallengeHistory).toBe('function');
    });

    it('should return challenge history array', async () => {
      const mockOutput = [
        { id: 'challenge-1', score: 85 },
        { id: 'challenge-2', score: 90 },
      ] as any;

      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockOutput })
      );

      const result = await service.getChallengeHistory();

      expect(result).toEqual(mockOutput);
      expect(Array.isArray(result)).toBe(true);
    });

    it('should return empty array on history fetch failure', async () => {
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: false })
      );

      const result = await service.getChallengeHistory();

      expect(result).toEqual([]);
    });

    it('should return empty array when data is null', async () => {
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: null })
      );

      const result = await service.getChallengeHistory();

      expect(result).toEqual([]);
    });
  });

  // ===========================================================================
  // Learning Points Operations (Shefa Integration)
  // ===========================================================================

  describe('earnLamadPoints', () => {
    it('should have earnLamadPoints method', () => {
      expect(service.earnLamadPoints).toBeDefined();
      expect(typeof service.earnLamadPoints).toBe('function');
    });

    it('should earn learning points', async () => {
      const mockOutput = { pointsEarned: 10, newBalance: 100 } as any;
      const pointsInput = {
        trigger: 'engagement_view' as const,
        content_id: 'content-1',
      };

      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockOutput })
      );

      const result = await service.earnLamadPoints(pointsInput);

      expect(result).toEqual(mockOutput);
      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          fnName: 'earn_lamad_points',
          payload: pointsInput,
        })
      );
    });

    it('should return null on point earning failure', async () => {
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: false })
      );

      const result = await service.earnLamadPoints({
        trigger: 'engagement_practice' as const,
        content_id: 'content-1',
      });

      expect(result).toBeNull();
    });
  });

  describe('getMyLamadPointBalance', () => {
    it('should have getMyLamadPointBalance method', () => {
      expect(service.getMyLamadPointBalance).toBeDefined();
      expect(typeof service.getMyLamadPointBalance).toBe('function');
    });

    it('should return point balance', async () => {
      const mockOutput = { balance: 150 } as any;
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockOutput })
      );

      const result = await service.getMyLamadPointBalance();

      expect(result).toEqual(mockOutput);
      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          fnName: 'get_my_lamad_point_balance',
          payload: null,
        })
      );
    });

    it('should return null on balance fetch failure', async () => {
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: false })
      );

      const result = await service.getMyLamadPointBalance();

      expect(result).toBeNull();
    });
  });

  describe('getMyLamadPointHistory', () => {
    it('should have getMyLamadPointHistory method', () => {
      expect(service.getMyLamadPointHistory).toBeDefined();
      expect(typeof service.getMyLamadPointHistory).toBe('function');
    });

    it('should return point history array', async () => {
      const mockOutput = [
        { id: 'event-1', points: 10, type: 'completion' },
        { id: 'event-2', points: 5, type: 'engagement' },
      ] as any;

      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockOutput })
      );

      const result = await service.getMyLamadPointHistory();

      expect(result).toEqual(mockOutput);
      expect(Array.isArray(result)).toBe(true);
    });

    it('should accept optional limit parameter', async () => {
      const mockOutput = [{ id: 'event-1', points: 10 }] as any;

      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockOutput })
      );

      const result = await service.getMyLamadPointHistory(10);

      expect(result).toEqual(mockOutput);
      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          fnName: 'get_my_lamad_point_history',
          payload: 10,
        })
      );
    });

    it('should send null when no limit specified', async () => {
      const mockOutput: any[] = [];

      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockOutput })
      );

      await service.getMyLamadPointHistory();

      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          payload: null,
        })
      );
    });

    it('should return empty array on history fetch failure', async () => {
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: false })
      );

      const result = await service.getMyLamadPointHistory();

      expect(result).toEqual([]);
    });

    it('should return empty array when data is null', async () => {
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: null })
      );

      const result = await service.getMyLamadPointHistory();

      expect(result).toEqual([]);
    });
  });
});
