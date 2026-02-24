import { TestBed } from '@angular/core/testing';

import type { DiscoveryResult } from '../models/discovery-assessment.model';

import { DiscoveryAttestationService } from './discovery-attestation.service';
import { LongitudinalTrackingService } from './longitudinal-tracking.service';

// =============================================================================
// Fixtures
// =============================================================================

const buildResult = (overrides: Partial<DiscoveryResult> = {}): DiscoveryResult =>
  ({
    id: `result-${Date.now()}`,
    assessmentId: 'test-instrument',
    framework: 'test',
    category: 'personality',
    completedAt: new Date().toISOString(),
    primaryType: { typeId: 'type-a', name: 'Type A', score: 0.8 },
    subscaleScores: { 'sub-a': 0.6, 'sub-b': 0.4 },
    displayString: 'Type A',
    featured: false,
    displayOrder: 0,
    ...overrides,
  }) as unknown as DiscoveryResult;

// =============================================================================
// Tests
// =============================================================================

describe('LongitudinalTrackingService', () => {
  let service: LongitudinalTrackingService;
  let attestationSpy: jasmine.SpyObj<DiscoveryAttestationService>;

  beforeEach(() => {
    attestationSpy = jasmine.createSpyObj('DiscoveryAttestationService', ['results'], {
      results: jasmine.createSpy('results').and.returnValue([]),
    });

    TestBed.configureTestingModule({
      providers: [
        LongitudinalTrackingService,
        { provide: DiscoveryAttestationService, useValue: attestationSpy },
      ],
    });

    service = TestBed.inject(LongitudinalTrackingService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  // ---------------------------------------------------------------------------
  // getRetakeHistory
  // ---------------------------------------------------------------------------

  describe('getRetakeHistory()', () => {
    it('should return empty array when no results', () => {
      expect(service.getRetakeHistory('nonexistent')).toEqual([]);
    });

    it('should return results for matching instrument sorted by date', () => {
      const old = buildResult({
        assessmentId: 'inst-1',
        completedAt: '2025-01-01T00:00:00Z',
      });
      const recent = buildResult({
        assessmentId: 'inst-1',
        completedAt: '2025-06-01T00:00:00Z',
      });
      attestationSpy.results.and.returnValue([recent, old]);

      const history = service.getRetakeHistory('inst-1');

      expect(history.length).toBe(2);
      expect(history[0].completedAt).toBe('2025-01-01T00:00:00Z');
      expect(history[1].completedAt).toBe('2025-06-01T00:00:00Z');
    });

    it('should filter by instrument ID', () => {
      const a = buildResult({ assessmentId: 'inst-a' });
      const b = buildResult({ assessmentId: 'inst-b' });
      attestationSpy.results.and.returnValue([a, b]);

      expect(service.getRetakeHistory('inst-a').length).toBe(1);
      expect(service.getRetakeHistory('inst-b').length).toBe(1);
    });
  });

  // ---------------------------------------------------------------------------
  // computeChanges
  // ---------------------------------------------------------------------------

  describe('computeChanges()', () => {
    it('should return empty when fewer than 2 retakes', () => {
      attestationSpy.results.and.returnValue([buildResult({ assessmentId: 'single' })]);

      expect(service.computeChanges('single')).toEqual([]);
    });

    it('should compute changes between two retakes', () => {
      const prev = buildResult({
        assessmentId: 'change-test',
        completedAt: '2025-01-01T00:00:00Z',
        subscaleScores: { 'sub-a': 0.4, 'sub-b': 0.6 },
      });
      const curr = buildResult({
        assessmentId: 'change-test',
        completedAt: '2025-06-01T00:00:00Z',
        subscaleScores: { 'sub-a': 0.7, 'sub-b': 0.5 },
      });
      attestationSpy.results.and.returnValue([prev, curr]);

      const changes = service.computeChanges('change-test');

      expect(changes.length).toBe(2);

      const subA = changes.find(c => c.subscale === 'sub-a')!;
      expect(subA.previousScore).toBe(0.4);
      expect(subA.currentScore).toBe(0.7);
      expect(subA.changeDirection).toBe('increased');
      expect(subA.significance).toBe('significant');

      const subB = changes.find(c => c.subscale === 'sub-b')!;
      expect(subB.changeDirection).toBe('decreased');
      expect(subB.significance).toBe('marginal');
    });

    it('should classify stable scores', () => {
      const prev = buildResult({
        assessmentId: 'stable-test',
        completedAt: '2025-01-01T00:00:00Z',
        subscaleScores: { 'sub-a': 0.5 },
      });
      const curr = buildResult({
        assessmentId: 'stable-test',
        completedAt: '2025-06-01T00:00:00Z',
        subscaleScores: { 'sub-a': 0.52 },
      });
      attestationSpy.results.and.returnValue([prev, curr]);

      const changes = service.computeChanges('stable-test');

      expect(changes[0].changeDirection).toBe('stable');
      expect(changes[0].significance).toBe('none');
    });

    it('should use the two most recent retakes when more than 2 exist', () => {
      const r1 = buildResult({
        assessmentId: 'multi',
        completedAt: '2024-01-01T00:00:00Z',
        subscaleScores: { 'sub-a': 0.1 },
      });
      const r2 = buildResult({
        assessmentId: 'multi',
        completedAt: '2025-01-01T00:00:00Z',
        subscaleScores: { 'sub-a': 0.5 },
      });
      const r3 = buildResult({
        assessmentId: 'multi',
        completedAt: '2025-06-01T00:00:00Z',
        subscaleScores: { 'sub-a': 0.7 },
      });
      attestationSpy.results.and.returnValue([r1, r2, r3]);

      const changes = service.computeChanges('multi');
      const subA = changes.find(c => c.subscale === 'sub-a')!;

      // Should compare r2 (0.5) to r3 (0.7), not r1 (0.1) to r3
      expect(subA.previousScore).toBe(0.5);
      expect(subA.currentScore).toBe(0.7);
    });
  });

  // ---------------------------------------------------------------------------
  // getPatternAlerts
  // ---------------------------------------------------------------------------

  describe('getPatternAlerts()', () => {
    it('should return empty when no significant changes', () => {
      expect(service.getPatternAlerts()).toEqual([]);
    });
  });
});
