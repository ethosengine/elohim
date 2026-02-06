/**
 * GovernanceSignalService Tests
 *
 * Coverage focus:
 * - Service creation
 * - Signal recording methods
 * - Signal retrieval methods
 * - Observable patterns
 */

import { TestBed } from '@angular/core/testing';

import { GovernanceSignalService } from './governance-signal.service';

describe('GovernanceSignalService', () => {
  let service: GovernanceSignalService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [GovernanceSignalService],
    });

    service = TestBed.inject(GovernanceSignalService);
  });

  // ==========================================================================
  // Service Creation Tests
  // ==========================================================================

  describe('service creation', () => {
    it('should be created', () => {
      expect(service).toBeTruthy();
    });

    it('should have service instance', () => {
      expect(service).toBeDefined();
    });

    it('should be injectable service', () => {
      expect(service instanceof GovernanceSignalService).toBe(true);
    });
  });

  // ==========================================================================
  // Signal Recording Tests
  // ==========================================================================

  describe('signal recording', () => {
    it('should have recordReaction method', () => {
      expect(typeof service.recordReaction).toBe('function');
    });

    it('should have recordMediationProceed method', () => {
      expect(typeof service.recordMediationProceed).toBe('function');
    });

    it('should have recordGraduatedFeedback method', () => {
      expect(typeof service.recordGraduatedFeedback).toBe('function');
    });

    it('should recordReaction return Observable', (done) => {
      const reaction = {
        type: 'positive' as any,
        context: 'test',
        private: false,
        respondedAt: new Date().toISOString(),
        responderId: 'user-1'
      };
      service.recordReaction('content-123', reaction).subscribe({
        next: (result) => {
          expect(result).toBeDefined();
          done();
        },
        error: done.fail
      });
    });

    it('should recordMediationProceed return Observable', (done) => {
      const log = {
        contentId: 'content-123',
        initialReaction: 'negative' as any,
        mediationChosen: 'proceed' as any,
        timestamp: new Date().toISOString(),
        userId: 'user-123',
        contentType: 'learning',
        reactionType: 'negative',
        reasoningShown: true,
        mediationMessage: 'test'
      } as any;
      service.recordMediationProceed(log).subscribe({
        next: (result) => {
          expect(result).toBeDefined();
          done();
        },
        error: done.fail
      });
    });

    it('should recordGraduatedFeedback return Observable', (done) => {
      const feedback = {
        contentId: 'content-123',
        rating: 4,
        dimension: 'clarity' as any,
        comment: 'Clear explanation',
        context: 'learning',
        position: 'middle',
        positionIndex: 1,
        intensity: 'moderate'
      } as any;
      service.recordGraduatedFeedback('user-123', feedback).subscribe({
        next: (result) => {
          expect(result).toBeDefined();
          done();
        },
        error: done.fail
      });
    });
  });

  // ==========================================================================
  // Signal Retrieval Tests
  // ==========================================================================

  describe('signal retrieval', () => {
    it('should have getReactions method', () => {
      expect(typeof service.getReactions).toBe('function');
    });

    it('should have getReactionCounts method', () => {
      expect(typeof service.getReactionCounts).toBe('function');
    });

    it('should have getGraduatedFeedback method', () => {
      expect(typeof service.getGraduatedFeedback).toBe('function');
    });

    it('should have getFeedbackStats method', () => {
      expect(typeof service.getFeedbackStats).toBe('function');
    });

    it('should getReactions return Observable', (done) => {
      service.getReactions('content-123').subscribe({
        next: (reactions) => {
          expect(Array.isArray(reactions)).toBe(true);
          done();
        },
        error: done.fail
      });
    });

    it('should getReactionCounts return Observable', (done) => {
      service.getReactionCounts('content-123').subscribe({
        next: (counts) => {
          expect(counts).toBeDefined();
          done();
        },
        error: done.fail
      });
    });

    it('should getGraduatedFeedback return Observable', (done) => {
      service.getGraduatedFeedback('content-123').subscribe({
        next: (feedback) => {
          expect(Array.isArray(feedback)).toBe(true);
          done();
        },
        error: done.fail
      });
    });

    it('should getFeedbackStats return Observable', (done) => {
      service.getFeedbackStats('content-123').subscribe({
        next: (stats) => {
          expect(stats).toBeDefined();
          done();
        },
        error: done.fail
      });
    });
  });

  // ==========================================================================
  // Observable Pattern Tests
  // ==========================================================================

  describe('observable patterns', () => {
    it('should have signalChanges$ observable', () => {
      expect(service.signalChanges$).toBeDefined();
      expect(service.signalChanges$.subscribe).toBeDefined();
    });

    it('should have signalChanges$ subscription', (done) => {
      let sub: any;
      sub = service.signalChanges$.subscribe({
        next: () => {
          if (sub) sub.unsubscribe();
          done();
        },
        error: done.fail
      });
    });
  });

  // ==========================================================================
  // Method Existence Tests
  // ==========================================================================

  describe('public API completeness', () => {
    it('should have all record methods', () => {
      expect(typeof service.recordReaction).toBe('function');
      expect(typeof service.recordMediationProceed).toBe('function');
      expect(typeof service.recordGraduatedFeedback).toBe('function');
    });

    it('should have all retrieval methods', () => {
      expect(typeof service.getReactions).toBe('function');
      expect(typeof service.getReactionCounts).toBe('function');
      expect(typeof service.getGraduatedFeedback).toBe('function');
      expect(typeof service.getFeedbackStats).toBe('function');
    });
  });

  // ==========================================================================
  // Return Type Tests
  // ==========================================================================

  describe('return types', () => {
    it('recordReaction should return Observable<boolean>', (done) => {
      const reaction = {
        type: 'positive' as any,
        context: 'test',
        private: false,
        respondedAt: new Date().toISOString(),
        responderId: 'user-1'
      };
      const result = service.recordReaction('content-123', reaction);

      expect(result.subscribe).toBeDefined();
      result.subscribe({
        next: (val) => {
          expect(typeof val === 'boolean').toBe(true);
          done();
        },
        error: done.fail
      });
    });

    it('getReactions should return Observable<Array>', (done) => {
      const result = service.getReactions('content-123');

      expect(result.subscribe).toBeDefined();
      result.subscribe({
        next: (val) => {
          expect(Array.isArray(val)).toBe(true);
          done();
        },
        error: done.fail
      });
    });

    it('getReactionCounts should return Observable<Object>', (done) => {
      const result = service.getReactionCounts('content-123');

      expect(result.subscribe).toBeDefined();
      result.subscribe({
        next: (val) => {
          expect(typeof val === 'object').toBe(true);
          done();
        },
        error: done.fail
      });
    });

    it('getFeedbackStats should return Observable<Object>', (done) => {
      const result = service.getFeedbackStats('content-123');

      expect(result.subscribe).toBeDefined();
      result.subscribe({
        next: (val) => {
          expect(typeof val === 'object').toBe(true);
          done();
        },
        error: done.fail
      });
    });

    it('signalChanges$ should be Observable', () => {
      expect(service.signalChanges$.subscribe).toBeDefined();
    });
  });

  // ==========================================================================
  // Parameter Validation Tests
  // ==========================================================================

  describe('parameter validation', () => {
    it('should accept contentId for getReactions', () => {
      expect(() => service.getReactions('content-123')).not.toThrow();
    });

    it('should accept optional includePrivate flag', () => {
      expect(() => service.getReactions('content-123', true)).not.toThrow();
      expect(() => service.getReactions('content-123', false)).not.toThrow();
    });

    it('should accept reaction object for recordReaction', () => {
      const reaction = {
        type: 'positive' as any,
        context: 'test',
        private: false,
        respondedAt: new Date().toISOString(),
        responderId: 'user-1'
      };
      expect(() => service.recordReaction('content-123', reaction)).not.toThrow();
    });

    it('should accept userId and feedback for recordGraduatedFeedback', () => {
      const feedback = {
        contentId: 'content-123',
        rating: 4,
        dimension: 'clarity' as any,
        comment: 'Clear',
        context: 'learning',
        position: 'middle',
        positionIndex: 1,
        intensity: 'moderate'
      } as any;
      expect(() => service.recordGraduatedFeedback('user-123', feedback)).not.toThrow();
    });
  });

  // ==========================================================================
  // Integration Tests
  // ==========================================================================

  describe('integration patterns', () => {
    it('should allow multiple record operations', () => {
      const reaction = {
        type: 'positive' as any,
        context: 'test',
        private: false,
        respondedAt: new Date().toISOString(),
        responderId: 'user-1'
      };

      expect(() => {
        service.recordReaction('content-123', reaction);
      }).not.toThrow();
    });

    it('should allow concurrent retrieval operations', (done) => {
      let completed = 0;
      const checkDone = () => {
        completed++;
        if (completed === 2) done();
      };

      service.getReactions('content-123').subscribe({
        next: () => checkDone(),
        error: done.fail
      });

      service.getGraduatedFeedback('content-123').subscribe({
        next: () => checkDone(),
        error: done.fail
      });
    });
  });
});
