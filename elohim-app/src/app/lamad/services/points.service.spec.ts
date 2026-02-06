import { TestBed } from '@angular/core/testing';
import { PointsService } from './points.service';
import { LearnerBackendService } from '@app/elohim/services/learner-backend.service';

describe('PointsService', () => {
  let service: PointsService;
  let backendSpy: jasmine.SpyObj<LearnerBackendService>;

  beforeEach(() => {
    backendSpy = jasmine.createSpyObj('LearnerBackendService', [
      'getMyLamadPointBalance',
      'earnLamadPoints',
      'getMyLamadPointHistory',
    ]);

    TestBed.configureTestingModule({
      providers: [
        PointsService,
        { provide: LearnerBackendService, useValue: backendSpy },
      ],
    });
    service = TestBed.inject(PointsService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('Observable Properties', () => {
    it('should have balance$ observable', () => {
      expect(service.balance$).toBeDefined();
      expect(service.balance$.subscribe).toBeDefined();
    });

    it('should have history$ observable', () => {
      expect(service.history$).toBeDefined();
      expect(service.history$.subscribe).toBeDefined();
    });

    it('should have loading$ observable', () => {
      expect(service.loading$).toBeDefined();
      expect(service.loading$.subscribe).toBeDefined();
    });

    it('should have totalPoints$ observable', () => {
      expect(service.totalPoints$).toBeDefined();
      expect(service.totalPoints$.subscribe).toBeDefined();
    });
  });

  describe('getBalance$()', () => {
    it('should have getBalance$ method', () => {
      expect(service.getBalance$).toBeDefined();
      expect(typeof service.getBalance$).toBe('function');
    });

    it('should return observable', () => {
      backendSpy.getMyLamadPointBalance.and.returnValue(
        Promise.resolve({
          action_hash: new Uint8Array(),
          balance: {
            id: 'balance-1',
            agent_id: 'agent-1',
            total_points: 100,
            points_by_trigger_json: '{}',
            total_earned: 100,
            total_spent: 0,
            last_point_event_id: null,
            last_point_event_at: null,
            created_at: new Date().toISOString(),
            updated_at: new Date().toISOString(),
          },
        })
      );

      const result = service.getBalance$();
      expect(result).toBeDefined();
      expect(result.subscribe).toBeDefined();
    });
  });

  describe('refreshBalance()', () => {
    it('should have refreshBalance method', () => {
      expect(service.refreshBalance).toBeDefined();
      expect(typeof service.refreshBalance).toBe('function');
    });

    it('should call backend getMyLamadPointBalance', (done) => {
      backendSpy.getMyLamadPointBalance.and.returnValue(
        Promise.resolve({
          action_hash: new Uint8Array(),
          balance: {
            id: 'balance-1',
            agent_id: 'agent-1',
            total_points: 100,
            points_by_trigger_json: '{}',
            total_earned: 100,
            total_spent: 0,
            last_point_event_id: null,
            last_point_event_at: null,
            created_at: new Date().toISOString(),
            updated_at: new Date().toISOString(),
          },
        })
      );

      service.refreshBalance();

      setTimeout(() => {
        expect(backendSpy.getMyLamadPointBalance).toHaveBeenCalled();
        done();
      }, 100);
    });
  });

  describe('getBalanceSync()', () => {
    it('should have getBalanceSync method', () => {
      expect(service.getBalanceSync).toBeDefined();
      expect(typeof service.getBalanceSync).toBe('function');
    });

    it('should return balance or null', () => {
      const result = service.getBalanceSync();
      expect(result === null || typeof result === 'object').toBe(true);
    });
  });

  describe('getTotalPointsSync()', () => {
    it('should have getTotalPointsSync method', () => {
      expect(service.getTotalPointsSync).toBeDefined();
      expect(typeof service.getTotalPointsSync).toBe('function');
    });

    it('should return number', () => {
      const result = service.getTotalPointsSync();
      expect(typeof result).toBe('number');
    });

    it('should return 0 or positive number', () => {
      const result = service.getTotalPointsSync();
      expect(result).toBeGreaterThanOrEqual(0);
    });
  });

  describe('earnPoints()', () => {
    it('should have earnPoints method', () => {
      expect(service.earnPoints).toBeDefined();
      expect(typeof service.earnPoints).toBe('function');
    });

    it('should return observable', () => {
      backendSpy.earnLamadPoints.and.returnValue(Promise.resolve(null));
      const result = service.earnPoints('engagement_view', 'content-123');
      expect(result).toBeDefined();
      expect(result.subscribe).toBeDefined();
    });

    it('should call backend earnLamadPoints', (done) => {
      backendSpy.earnLamadPoints.and.returnValue(Promise.resolve(null));

      service.earnPoints('engagement_view', 'content-123').subscribe(() => {
        expect(backendSpy.earnLamadPoints).toHaveBeenCalled();
        done();
      });
    });

    it('should accept trigger parameter', () => {
      backendSpy.earnLamadPoints.and.returnValue(Promise.resolve(null));
      expect(() => {
        service.earnPoints('engagement_view', 'content-123');
      }).not.toThrow();
    });

    it('should accept optional parameters', () => {
      backendSpy.earnLamadPoints.and.returnValue(Promise.resolve(null));
      expect(() => {
        service.earnPoints(
          'path_step_complete',
          'content-123',
          'path-456',
          undefined,
          true,
          'note'
        );
      }).not.toThrow();
    });
  });

  describe('earnViewPoints()', () => {
    it('should have earnViewPoints method', () => {
      expect(service.earnViewPoints).toBeDefined();
      expect(typeof service.earnViewPoints).toBe('function');
    });

    it('should return observable', () => {
      backendSpy.earnLamadPoints.and.returnValue(Promise.resolve(null));
      const result = service.earnViewPoints('content-123');
      expect(result.subscribe).toBeDefined();
    });
  });

  describe('earnPracticePoints()', () => {
    it('should have earnPracticePoints method', () => {
      expect(service.earnPracticePoints).toBeDefined();
      expect(typeof service.earnPracticePoints).toBe('function');
    });

    it('should return observable', () => {
      backendSpy.earnLamadPoints.and.returnValue(Promise.resolve(null));
      const result = service.earnPracticePoints('content-123');
      expect(result.subscribe).toBeDefined();
    });
  });

  describe('earnPathStepPoints()', () => {
    it('should have earnPathStepPoints method', () => {
      expect(service.earnPathStepPoints).toBeDefined();
      expect(typeof service.earnPathStepPoints).toBe('function');
    });

    it('should return observable', () => {
      backendSpy.earnLamadPoints.and.returnValue(Promise.resolve(null));
      const result = service.earnPathStepPoints('path-123', 'content-456');
      expect(result.subscribe).toBeDefined();
    });
  });

  describe('earnPathCompletePoints()', () => {
    it('should have earnPathCompletePoints method', () => {
      expect(service.earnPathCompletePoints).toBeDefined();
      expect(typeof service.earnPathCompletePoints).toBe('function');
    });

    it('should return observable', () => {
      backendSpy.earnLamadPoints.and.returnValue(Promise.resolve(null));
      const result = service.earnPathCompletePoints('path-123');
      expect(result.subscribe).toBeDefined();
    });
  });

  describe('getHistory$()', () => {
    it('should have getHistory$ method', () => {
      expect(service.getHistory$).toBeDefined();
      expect(typeof service.getHistory$).toBe('function');
    });

    it('should return observable', () => {
      const result = service.getHistory$();
      expect(result).toBeDefined();
      expect(result.subscribe).toBeDefined();
    });

    it('should accept optional limit parameter', () => {
      const result = service.getHistory$(5);
      expect(result.subscribe).toBeDefined();
    });
  });

  describe('loadHistory()', () => {
    it('should have loadHistory method', () => {
      expect(service.loadHistory).toBeDefined();
      expect(typeof service.loadHistory).toBe('function');
    });

    it('should return observable', () => {
      backendSpy.getMyLamadPointHistory.and.returnValue(Promise.resolve([]));
      const result = service.loadHistory();
      expect(result.subscribe).toBeDefined();
    });

    it('should accept optional limit parameter', () => {
      backendSpy.getMyLamadPointHistory.and.returnValue(Promise.resolve([]));
      const result = service.loadHistory(10);
      expect(result.subscribe).toBeDefined();
    });
  });

  describe('getHistorySync()', () => {
    it('should have getHistorySync method', () => {
      expect(service.getHistorySync).toBeDefined();
      expect(typeof service.getHistorySync).toBe('function');
    });

    it('should return array', () => {
      const result = service.getHistorySync();
      expect(Array.isArray(result)).toBe(true);
    });
  });

  describe('getPointsByTrigger$()', () => {
    it('should have getPointsByTrigger$ method', () => {
      expect(service.getPointsByTrigger$).toBeDefined();
      expect(typeof service.getPointsByTrigger$).toBe('function');
    });

    it('should return observable', () => {
      const result = service.getPointsByTrigger$();
      expect(result.subscribe).toBeDefined();
    });
  });

  describe('getPointsByTriggerSync()', () => {
    it('should have getPointsByTriggerSync method', () => {
      expect(service.getPointsByTriggerSync).toBeDefined();
      expect(typeof service.getPointsByTriggerSync).toBe('function');
    });

    it('should return object', () => {
      const result = service.getPointsByTriggerSync();
      expect(typeof result).toBe('object');
    });
  });

  describe('getTotalEarnedSync()', () => {
    it('should have getTotalEarnedSync method', () => {
      expect(service.getTotalEarnedSync).toBeDefined();
      expect(typeof service.getTotalEarnedSync).toBe('function');
    });

    it('should return number', () => {
      const result = service.getTotalEarnedSync();
      expect(typeof result).toBe('number');
    });
  });

  describe('getTotalSpentSync()', () => {
    it('should have getTotalSpentSync method', () => {
      expect(service.getTotalSpentSync).toBeDefined();
      expect(typeof service.getTotalSpentSync).toBe('function');
    });

    it('should return number', () => {
      const result = service.getTotalSpentSync();
      expect(typeof result).toBe('number');
    });
  });

  describe('getRecentEventsCount()', () => {
    it('should have getRecentEventsCount method', () => {
      expect(service.getRecentEventsCount).toBeDefined();
      expect(typeof service.getRecentEventsCount).toBe('function');
    });

    it('should return number', () => {
      const result = service.getRecentEventsCount();
      expect(typeof result).toBe('number');
    });

    it('should accept optional days parameter', () => {
      const result = service.getRecentEventsCount(14);
      expect(typeof result).toBe('number');
    });
  });

  describe('getRecentPointsEarned()', () => {
    it('should have getRecentPointsEarned method', () => {
      expect(service.getRecentPointsEarned).toBeDefined();
      expect(typeof service.getRecentPointsEarned).toBe('function');
    });

    it('should return number', () => {
      const result = service.getRecentPointsEarned();
      expect(typeof result).toBe('number');
    });

    it('should accept optional days parameter', () => {
      const result = service.getRecentPointsEarned(14);
      expect(typeof result).toBe('number');
    });
  });

  describe('formatPoints()', () => {
    it('should have formatPoints method', () => {
      expect(service.formatPoints).toBeDefined();
      expect(typeof service.formatPoints).toBe('function');
    });

    it('should return string', () => {
      const result = service.formatPoints(100);
      expect(typeof result).toBe('string');
    });
  });

  describe('getTriggerLabel()', () => {
    it('should have getTriggerLabel method', () => {
      expect(service.getTriggerLabel).toBeDefined();
      expect(typeof service.getTriggerLabel).toBe('function');
    });

    it('should return string', () => {
      const result = service.getTriggerLabel('engagement_view');
      expect(typeof result).toBe('string');
    });
  });

  describe('getPointAmount()', () => {
    it('should have getPointAmount method', () => {
      expect(service.getPointAmount).toBeDefined();
      expect(typeof service.getPointAmount).toBe('function');
    });

    it('should return number', () => {
      const result = service.getPointAmount('engagement_view');
      expect(typeof result).toBe('number');
    });
  });
});
