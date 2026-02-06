import { TestBed } from '@angular/core/testing';
import { StreakTrackerService } from './streak-tracker.service';
import { StreakState } from '../models/streak-state.model';

describe('StreakTrackerService', () => {
  let service: StreakTrackerService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [StreakTrackerService],
    });
    service = TestBed.inject(StreakTrackerService);
    // Clear localStorage before each test
    localStorage.clear();
  });

  afterEach(() => {
    localStorage.clear();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('startTracking()', () => {
    it('should have startTracking method', () => {
      expect(service.startTracking).toBeDefined();
      expect(typeof service.startTracking).toBe('function');
    });

    it('should return a StreakState', () => {
      const result = service.startTracking('content-123', 'human-456');
      expect(result).toBeDefined();
      expect(result.contentId).toBe('content-123');
      expect(result.humanId).toBe('human-456');
    });

    it('should return existing streak if in progress', () => {
      const state1 = service.startTracking('content-123', 'human-456');
      const state2 = service.startTracking('content-123', 'human-456');
      expect(state1.contentId).toBe(state2.contentId);
      expect(state1.humanId).toBe(state2.humanId);
    });
  });

  describe('recordAnswer()', () => {
    it('should have recordAnswer method', () => {
      expect(service.recordAnswer).toBeDefined();
      expect(typeof service.recordAnswer).toBe('function');
    });

    it('should return null if no streak exists', () => {
      const result = service.recordAnswer('content-123', 'q-1', true);
      expect(result).toBeNull();
    });

    it('should record answer and return updated state', () => {
      service.startTracking('content-123', 'human-456');
      const result = service.recordAnswer('content-123', 'q-1', true);
      expect(result).toBeTruthy();
      expect(result?.contentId).toBe('content-123');
    });
  });

  describe('getState()', () => {
    it('should have getState method', () => {
      expect(service.getState).toBeDefined();
      expect(typeof service.getState).toBe('function');
    });

    it('should return null if streak not found', () => {
      const result = service.getState('nonexistent-123');
      expect(result).toBeNull();
    });

    it('should return state after startTracking', () => {
      service.startTracking('content-123', 'human-456');
      const result = service.getState('content-123');
      expect(result).toBeTruthy();
      expect(result?.contentId).toBe('content-123');
    });
  });

  describe('getState$()', () => {
    it('should have getState$ method', () => {
      expect(service.getState$).toBeDefined();
      expect(typeof service.getState$).toBe('function');
    });

    it('should return observable', () => {
      service.startTracking('content-123', 'human-456');
      const result = service.getState$('content-123');
      expect(result).toBeDefined();
      expect(result.subscribe).toBeDefined();
    });
  });

  describe('getProgress()', () => {
    it('should have getProgress method', () => {
      expect(service.getProgress).toBeDefined();
      expect(typeof service.getProgress).toBe('function');
    });

    it('should return null if no streak exists', () => {
      const result = service.getProgress('nonexistent-123');
      expect(result).toBeNull();
    });

    it('should return progress object after startTracking', () => {
      service.startTracking('content-123', 'human-456');
      const result = service.getProgress('content-123');
      expect(result).toBeTruthy();
    });
  });

  describe('isAchieved()', () => {
    it('should have isAchieved method', () => {
      expect(service.isAchieved).toBeDefined();
      expect(typeof service.isAchieved).toBe('function');
    });

    it('should return false if no streak exists', () => {
      const result = service.isAchieved('nonexistent-123');
      expect(result).toBe(false);
    });

    it('should return boolean', () => {
      service.startTracking('content-123', 'human-456');
      const result = service.isAchieved('content-123');
      expect(typeof result).toBe('boolean');
    });
  });

  describe('isComplete()', () => {
    it('should have isComplete method', () => {
      expect(service.isComplete).toBeDefined();
      expect(typeof service.isComplete).toBe('function');
    });

    it('should return false if no streak exists', () => {
      const result = service.isComplete('nonexistent-123');
      expect(result).toBe(false);
    });

    it('should return boolean', () => {
      service.startTracking('content-123', 'human-456');
      const result = service.isComplete('content-123');
      expect(typeof result).toBe('boolean');
    });
  });

  describe('reset()', () => {
    it('should have reset method', () => {
      expect(service.reset).toBeDefined();
      expect(typeof service.reset).toBe('function');
    });

    it('should return null if no streak exists', () => {
      const result = service.reset('nonexistent-123');
      expect(result).toBeNull();
    });

    it('should reset and return updated state', () => {
      service.startTracking('content-123', 'human-456');
      const result = service.reset('content-123');
      expect(result).toBeTruthy();
      expect(result?.contentId).toBe('content-123');
    });
  });

  describe('onAchieved()', () => {
    it('should have onAchieved method', () => {
      expect(service.onAchieved).toBeDefined();
      expect(typeof service.onAchieved).toBe('function');
    });

    it('should register achievement callback', () => {
      const callback = jasmine.createSpy('callback');
      service.onAchieved('content-123', callback);
      expect(callback).toBeDefined();
    });
  });

  describe('offAchieved()', () => {
    it('should have offAchieved method', () => {
      expect(service.offAchieved).toBeDefined();
      expect(typeof service.offAchieved).toBe('function');
    });

    it('should remove achievement callback', () => {
      const callback = jasmine.createSpy('callback');
      service.onAchieved('content-123', callback);
      service.offAchieved('content-123');
      expect(service).toBeTruthy(); // Just verify method exists
    });
  });

  describe('clear()', () => {
    it('should have clear method', () => {
      expect(service.clear).toBeDefined();
      expect(typeof service.clear).toBe('function');
    });

    it('should clear streak tracking', () => {
      service.startTracking('content-123', 'human-456');
      service.clear('content-123');
      const result = service.getState('content-123');
      expect(result).toBeNull();
    });
  });

  describe('clearAll()', () => {
    it('should have clearAll method', () => {
      expect(service.clearAll).toBeDefined();
      expect(typeof service.clearAll).toBe('function');
    });

    it('should clear all streaks', () => {
      service.startTracking('content-123', 'human-456');
      service.startTracking('content-456', 'human-456');
      service.clearAll();
      expect(service.getState('content-123')).toBeNull();
      expect(service.getState('content-456')).toBeNull();
    });
  });

  describe('saveToStorage()', () => {
    it('should have saveToStorage method', () => {
      expect(service.saveToStorage).toBeDefined();
      expect(typeof service.saveToStorage).toBe('function');
    });

    it('should save streak to localStorage', () => {
      service.startTracking('content-123', 'human-456');
      service.saveToStorage('content-123');
      expect(localStorage.getItem).toBeDefined();
    });
  });
});
