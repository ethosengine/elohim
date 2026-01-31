import { TestBed } from '@angular/core/testing';

import { PracticeService } from './practice.service';
import { LearnerBackendService } from '@app/elohim/services/learner-backend.service';
import { of } from 'rxjs';
import { PracticePool, MasteryChallenge, ChallengeResult } from '../models/practice.model';

describe('PracticeService', () => {
  let service: PracticeService;
  let mockBackend: jasmine.SpyObj<LearnerBackendService>;

  beforeEach(() => {
    mockBackend = jasmine.createSpyObj('LearnerBackendService', [
      'getOrCreatePracticePool',
      'addPathToPool',
      'refreshPracticePool',
      'getPoolRecommendations',
      'checkChallengeCooldown',
      'startMasteryChallenge',
      'submitMasteryChallenge',
      'getChallengeHistory',
    ]);

    TestBed.configureTestingModule({
      providers: [
        PracticeService,
        { provide: LearnerBackendService, useValue: mockBackend },
      ],
    });
    service = TestBed.inject(PracticeService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('Pool Management', () => {
    it('should have pool$ observable', () => {
      expect(service.pool$).toBeDefined();
      expect(service.pool$.subscribe).toBeDefined();
    });

    it('should have currentChallenge$ observable', () => {
      expect(service.currentChallenge$).toBeDefined();
      expect(service.currentChallenge$.subscribe).toBeDefined();
    });

    it('should have recommendations$ observable', () => {
      expect(service.recommendations$).toBeDefined();
      expect(service.recommendations$.subscribe).toBeDefined();
    });

    it('should have cooldown$ observable', () => {
      expect(service.cooldown$).toBeDefined();
      expect(service.cooldown$.subscribe).toBeDefined();
    });

    it('should have challengeHistory$ observable', () => {
      expect(service.challengeHistory$).toBeDefined();
      expect(service.challengeHistory$.subscribe).toBeDefined();
    });

    it('should have initializePool method', () => {
      expect(service.initializePool).toBeDefined();
      expect(typeof service.initializePool).toBe('function');
    });

    it('should have addPathToPool method', () => {
      expect(service.addPathToPool).toBeDefined();
      expect(typeof service.addPathToPool).toBe('function');
    });

    it('should have refreshPool method', () => {
      expect(service.refreshPool).toBeDefined();
      expect(typeof service.refreshPool).toBe('function');
    });

    it('should have getPoolSync method', () => {
      expect(service.getPoolSync).toBeDefined();
      expect(typeof service.getPoolSync).toBe('function');
    });
  });

  describe('Recommendations', () => {
    it('should have getRecommendations$ method', () => {
      expect(service.getRecommendations$).toBeDefined();
      expect(typeof service.getRecommendations$).toBe('function');
    });

    it('should have refreshRecommendations method', () => {
      expect(service.refreshRecommendations).toBeDefined();
      expect(typeof service.refreshRecommendations).toBe('function');
    });
  });

  describe('Challenge Cooldown', () => {
    it('should have checkCooldown method', () => {
      expect(service.checkCooldown).toBeDefined();
      expect(typeof service.checkCooldown).toBe('function');
    });

    it('should have canTakeChallenge$ method', () => {
      expect(service.canTakeChallenge$).toBeDefined();
      expect(typeof service.canTakeChallenge$).toBe('function');
    });

    it('should have getCooldownSync method', () => {
      expect(service.getCooldownSync).toBeDefined();
      expect(typeof service.getCooldownSync).toBe('function');
    });
  });

  describe('Challenge Flow', () => {
    it('should have startChallenge method', () => {
      expect(service.startChallenge).toBeDefined();
      expect(typeof service.startChallenge).toBe('function');
    });

    it('should have submitChallenge method', () => {
      expect(service.submitChallenge).toBeDefined();
      expect(typeof service.submitChallenge).toBe('function');
    });

    it('should have abandonChallenge method', () => {
      expect(service.abandonChallenge).toBeDefined();
      expect(typeof service.abandonChallenge).toBe('function');
    });

    it('should have getCurrentChallengeSync method', () => {
      expect(service.getCurrentChallengeSync).toBeDefined();
      expect(typeof service.getCurrentChallengeSync).toBe('function');
    });

    it('should have hasActiveChallenge method', () => {
      expect(service.hasActiveChallenge).toBeDefined();
      expect(typeof service.hasActiveChallenge).toBe('function');
    });
  });

  describe('Challenge History', () => {
    it('should have loadChallengeHistory method', () => {
      expect(service.loadChallengeHistory).toBeDefined();
      expect(typeof service.loadChallengeHistory).toBe('function');
    });

    it('should have getChallengeHistorySync method', () => {
      expect(service.getChallengeHistorySync).toBeDefined();
      expect(typeof service.getChallengeHistorySync).toBe('function');
    });
  });

  describe('Pool Analysis', () => {
    it('should have getActiveCount method', () => {
      expect(service.getActiveCount).toBeDefined();
      expect(typeof service.getActiveCount).toBe('function');
    });

    it('should return 0 when pool is null', () => {
      expect(service.getActiveCount()).toBe(0);
    });

    it('should have getRefreshCount method', () => {
      expect(service.getRefreshCount).toBeDefined();
      expect(typeof service.getRefreshCount).toBe('function');
    });

    it('should return 0 for refresh count when pool is null', () => {
      expect(service.getRefreshCount()).toBe(0);
    });

    it('should have getDiscoveryCount method', () => {
      expect(service.getDiscoveryCount).toBeDefined();
      expect(typeof service.getDiscoveryCount).toBe('function');
    });

    it('should return 0 for discovery count when pool is null', () => {
      expect(service.getDiscoveryCount()).toBe(0);
    });

    it('should have getPoolStats method', () => {
      expect(service.getPoolStats).toBeDefined();
      expect(typeof service.getPoolStats).toBe('function');
    });

    it('should return default stats when pool is null', () => {
      const stats = service.getPoolStats();
      expect(stats.active).toBe(0);
      expect(stats.refresh).toBe(0);
      expect(stats.discovery).toBe(0);
      expect(stats.totalChallenges).toBe(0);
      expect(stats.levelUps).toBe(0);
      expect(stats.levelDowns).toBe(0);
      expect(stats.discoveries).toBe(0);
    });
  });

  describe('initializePool', () => {
    it('should call backend with path IDs', (done) => {
      mockBackend.getOrCreatePracticePool.and.returnValue(Promise.resolve(null));
      service.initializePool(['path-1', 'path-2']).subscribe(() => {
        expect(mockBackend.getOrCreatePracticePool).toHaveBeenCalled();
        done();
      });
    });

    it('should return observable', () => {
      mockBackend.getOrCreatePracticePool.and.returnValue(Promise.resolve(null));
      const result = service.initializePool(['path-1']);
      expect(result.subscribe).toBeDefined();
    });
  });

  describe('addPathToPool', () => {
    it('should call backend with path ID', (done) => {
      mockBackend.addPathToPool.and.returnValue(Promise.resolve(null));
      service.addPathToPool('path-1').subscribe(() => {
        expect(mockBackend.addPathToPool).toHaveBeenCalled();
        done();
      });
    });

    it('should return observable', () => {
      mockBackend.addPathToPool.and.returnValue(Promise.resolve(null));
      const result = service.addPathToPool('path-1');
      expect(result.subscribe).toBeDefined();
    });
  });

  describe('refreshPool', () => {
    it('should call backend', (done) => {
      mockBackend.refreshPracticePool.and.returnValue(Promise.resolve(null));
      service.refreshPool().subscribe(() => {
        expect(mockBackend.refreshPracticePool).toHaveBeenCalled();
        done();
      });
    });

    it('should return observable', () => {
      mockBackend.refreshPracticePool.and.returnValue(Promise.resolve(null));
      const result = service.refreshPool();
      expect(result.subscribe).toBeDefined();
    });
  });

  describe('abandonChallenge', () => {
    it('should be a synchronous method', () => {
      const result = service.abandonChallenge();
      expect(result).toBeUndefined();
    });
  });

  describe('hasActiveChallenge', () => {
    it('should return false when no challenge', () => {
      expect(service.hasActiveChallenge()).toBe(false);
    });
  });

  describe('getChallengeHistorySync', () => {
    it('should return array', () => {
      const history = service.getChallengeHistorySync();
      expect(Array.isArray(history)).toBe(true);
    });

    it('should return empty array initially', () => {
      const history = service.getChallengeHistorySync();
      expect(history.length).toBe(0);
    });
  });
});
