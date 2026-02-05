import { TestBed } from '@angular/core/testing';
import { AttemptCooldownService } from './attempt-cooldown.service';

describe('AttemptCooldownService', () => {
  let service: AttemptCooldownService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [AttemptCooldownService],
    });
    service = TestBed.inject(AttemptCooldownService);
    localStorage.clear();
  });

  afterEach(() => {
    localStorage.clear();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('configure()', () => {
    it('should have configure method', () => {
      expect(service.configure).toBeDefined();
      expect(typeof service.configure).toBe('function');
    });

    it('should accept configuration object', () => {
      const config = { masteryAttemptsPerDay: 3 };
      service.configure(config);
      expect(service).toBeTruthy();
    });
  });

  describe('getConfig()', () => {
    it('should have getConfig method', () => {
      expect(service.getConfig).toBeDefined();
      expect(typeof service.getConfig).toBe('function');
    });

    it('should return configuration object', () => {
      const config = service.getConfig();
      expect(config).toBeDefined();
      expect(typeof config).toBe('object');
    });

    it('should return object with masteryAttemptsPerDay property', () => {
      const config = service.getConfig();
      expect(config.masteryAttemptsPerDay).toBeDefined();
      expect(typeof config.masteryAttemptsPerDay).toBe('number');
    });
  });

  describe('checkAttempt()', () => {
    it('should have checkAttempt method', () => {
      expect(service.checkAttempt).toBeDefined();
      expect(typeof service.checkAttempt).toBe('function');
    });

    it('should return AttemptCheckResult', () => {
      const result = service.checkAttempt('content-123', 'human-456');
      expect(result).toBeDefined();
      expect(typeof result).toBe('object');
    });

    it('should return object with allowed property', () => {
      const result = service.checkAttempt('content-123', 'human-456');
      expect(result.allowed).toBeDefined();
      expect(typeof result.allowed).toBe('boolean');
    });

    it('should return true for new attempt', () => {
      const result = service.checkAttempt('content-123', 'human-456');
      expect(result.allowed).toBe(true);
    });
  });

  describe('checkAttempt$()', () => {
    it('should have checkAttempt$ method', () => {
      expect(service.checkAttempt$).toBeDefined();
      expect(typeof service.checkAttempt$).toBe('function');
    });

    it('should return observable', () => {
      const result = service.checkAttempt$('content-123', 'human-456');
      expect(result).toBeDefined();
      expect(result.subscribe).toBeDefined();
    });
  });

  describe('canAttempt()', () => {
    it('should have canAttempt method', () => {
      expect(service.canAttempt).toBeDefined();
      expect(typeof service.canAttempt).toBe('function');
    });

    it('should return boolean', () => {
      const result = service.canAttempt('content-123', 'human-456');
      expect(typeof result).toBe('boolean');
    });

    it('should return true for new attempt', () => {
      const result = service.canAttempt('content-123', 'human-456');
      expect(result).toBe(true);
    });
  });

  describe('getRemainingAttempts()', () => {
    it('should have getRemainingAttempts method', () => {
      expect(service.getRemainingAttempts).toBeDefined();
      expect(typeof service.getRemainingAttempts).toBe('function');
    });

    it('should return number', () => {
      const result = service.getRemainingAttempts('content-123', 'human-456');
      expect(typeof result).toBe('number');
    });

    it('should return positive number for new attempt', () => {
      const result = service.getRemainingAttempts('content-123', 'human-456');
      expect(result).toBeGreaterThan(0);
    });
  });

  describe('recordAttempt()', () => {
    it('should have recordAttempt method', () => {
      expect(service.recordAttempt).toBeDefined();
      expect(typeof service.recordAttempt).toBe('function');
    });

    it('should return AttemptRecord', () => {
      const mockResult = {
        sessionId: 'session-1',
        type: 'mastery' as const,
        humanId: 'human-456',
        score: 0.75,
        passed: true,
        passingThreshold: 0.8,
        correctCount: 3,
        totalQuestions: 4,
        contentScores: [],
        masteryChanges: [],
        attestationsGranted: [],
        timing: {
          totalDurationMs: 60000,
          averageTimePerQuestion: 15000,
          fastestAnswerMs: 10000,
          slowestAnswerMs: 20000,
        },
        completedAt: new Date().toISOString(),
      };
      const result = service.recordAttempt('content-123', 'human-456', mockResult);
      expect(result).toBeDefined();
      expect(typeof result).toBe('object');
    });

    it('should accept contentId, humanId, and result parameters', () => {
      const mockResult = {
        sessionId: 'session-1',
        type: 'mastery' as const,
        humanId: 'human-456',
        score: 0.75,
        passed: true,
        passingThreshold: 0.8,
        correctCount: 3,
        totalQuestions: 4,
        contentScores: [],
        masteryChanges: [],
        attestationsGranted: [],
        timing: {
          totalDurationMs: 60000,
          averageTimePerQuestion: 15000,
          fastestAnswerMs: 10000,
          slowestAnswerMs: 20000,
        },
        completedAt: new Date().toISOString(),
      };
      expect(() => {
        service.recordAttempt('content-123', 'human-456', mockResult);
      }).not.toThrow();
    });
  });

  describe('getCooldownStatus()', () => {
    it('should have getCooldownStatus method', () => {
      expect(service.getCooldownStatus).toBeDefined();
      expect(typeof service.getCooldownStatus).toBe('function');
    });

    it('should return CooldownStatus object', () => {
      const result = service.getCooldownStatus('content-123', 'human-456');
      expect(result).toBeDefined();
      expect(typeof result).toBe('object');
    });

    it('should return object with inCooldown property', () => {
      const result = service.getCooldownStatus('content-123', 'human-456');
      expect(result.inCooldown).toBeDefined();
      expect(typeof result.inCooldown).toBe('boolean');
    });
  });

  describe('getCooldownStatus$()', () => {
    it('should have getCooldownStatus$ method', () => {
      expect(service.getCooldownStatus$).toBeDefined();
      expect(typeof service.getCooldownStatus$).toBe('function');
    });

    it('should return observable', () => {
      const result = service.getCooldownStatus$('content-123', 'human-456');
      expect(result).toBeDefined();
      expect(result.subscribe).toBeDefined();
    });
  });

  describe('isInCooldown()', () => {
    it('should have isInCooldown method', () => {
      expect(service.isInCooldown).toBeDefined();
      expect(typeof service.isInCooldown).toBe('function');
    });

    it('should return boolean', () => {
      const result = service.isInCooldown('content-123', 'human-456');
      expect(typeof result).toBe('boolean');
    });

    it('should return false for new attempt', () => {
      const result = service.isInCooldown('content-123', 'human-456');
      expect(result).toBe(false);
    });
  });

  describe('getTimeRemaining()', () => {
    it('should have getTimeRemaining method', () => {
      expect(service.getTimeRemaining).toBeDefined();
      expect(typeof service.getTimeRemaining).toBe('function');
    });

    it('should return string', () => {
      const result = service.getTimeRemaining('content-123', 'human-456');
      expect(typeof result).toBe('string');
    });
  });

  describe('getRecord()', () => {
    it('should have getRecord method', () => {
      expect(service.getRecord).toBeDefined();
      expect(typeof service.getRecord).toBe('function');
    });

    it('should return null for nonexistent record', () => {
      const result = service.getRecord('nonexistent-123', 'human-456');
      expect(result).toBeNull();
    });
  });

  describe('isMastered()', () => {
    it('should have isMastered method', () => {
      expect(service.isMastered).toBeDefined();
      expect(typeof service.isMastered).toBe('function');
    });

    it('should return boolean', () => {
      const result = service.isMastered('content-123', 'human-456');
      expect(typeof result).toBe('boolean');
    });

    it('should return false for new attempt', () => {
      const result = service.isMastered('content-123', 'human-456');
      expect(result).toBe(false);
    });
  });

  describe('getBestScore()', () => {
    it('should have getBestScore method', () => {
      expect(service.getBestScore).toBeDefined();
      expect(typeof service.getBestScore).toBe('function');
    });

    it('should return number', () => {
      const result = service.getBestScore('content-123', 'human-456');
      expect(typeof result).toBe('number');
    });

    it('should return 0 for new attempt', () => {
      const result = service.getBestScore('content-123', 'human-456');
      expect(result).toBe(0);
    });
  });

  describe('getAttemptHistory()', () => {
    it('should have getAttemptHistory method', () => {
      expect(service.getAttemptHistory).toBeDefined();
      expect(typeof service.getAttemptHistory).toBe('function');
    });

    it('should return array', () => {
      const result = service.getAttemptHistory('content-123', 'human-456');
      expect(Array.isArray(result)).toBe(true);
    });

    it('should return empty array for new attempt', () => {
      const result = service.getAttemptHistory('content-123', 'human-456');
      expect(result.length).toBe(0);
    });
  });

  describe('getAllRecordsForHuman()', () => {
    it('should have getAllRecordsForHuman method', () => {
      expect(service.getAllRecordsForHuman).toBeDefined();
      expect(typeof service.getAllRecordsForHuman).toBe('function');
    });

    it('should return array', () => {
      const result = service.getAllRecordsForHuman('human-456');
      expect(Array.isArray(result)).toBe(true);
    });

    it('should return empty array initially', () => {
      const result = service.getAllRecordsForHuman('human-456');
      expect(result.length).toBe(0);
    });
  });

  describe('clearRecord()', () => {
    it('should have clearRecord method', () => {
      expect(service.clearRecord).toBeDefined();
      expect(typeof service.clearRecord).toBe('function');
    });

    it('should accept contentId and humanId', () => {
      expect(() => {
        service.clearRecord('content-123', 'human-456');
      }).not.toThrow();
    });
  });

  describe('clearAllForHuman()', () => {
    it('should have clearAllForHuman method', () => {
      expect(service.clearAllForHuman).toBeDefined();
      expect(typeof service.clearAllForHuman).toBe('function');
    });

    it('should accept humanId', () => {
      expect(() => {
        service.clearAllForHuman('human-456');
      }).not.toThrow();
    });
  });

  describe('resetCooldown()', () => {
    it('should have resetCooldown method', () => {
      expect(service.resetCooldown).toBeDefined();
      expect(typeof service.resetCooldown).toBe('function');
    });

    it('should accept contentId and humanId', () => {
      expect(() => {
        service.resetCooldown('content-123', 'human-456');
      }).not.toThrow();
    });
  });

  describe('getBulkCooldownStatus()', () => {
    it('should have getBulkCooldownStatus method', () => {
      expect(service.getBulkCooldownStatus).toBeDefined();
      expect(typeof service.getBulkCooldownStatus).toBe('function');
    });

    it('should return Map', () => {
      const result = service.getBulkCooldownStatus(
        ['content-1', 'content-2'],
        'human-456'
      );
      expect(result instanceof Map).toBe(true);
    });

    it('should return map with entries for all content', () => {
      const result = service.getBulkCooldownStatus(
        ['content-1', 'content-2'],
        'human-456'
      );
      expect(result.size).toBe(2);
    });
  });

  describe('getBulkMasteryStatus()', () => {
    it('should have getBulkMasteryStatus method', () => {
      expect(service.getBulkMasteryStatus).toBeDefined();
      expect(typeof service.getBulkMasteryStatus).toBe('function');
    });

    it('should return Map', () => {
      const result = service.getBulkMasteryStatus(
        ['content-1', 'content-2'],
        'human-456'
      );
      expect(result instanceof Map).toBe(true);
    });

    it('should return map with entries for all content', () => {
      const result = service.getBulkMasteryStatus(
        ['content-1', 'content-2'],
        'human-456'
      );
      expect(result.size).toBe(2);
    });
  });
});
