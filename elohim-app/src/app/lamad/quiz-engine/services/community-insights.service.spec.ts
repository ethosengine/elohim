import { TestBed } from '@angular/core/testing';

import type { DiscoveryResult } from '../models/discovery-assessment.model';
import type { AggregationContext } from '../models/community-insights.model';

import { DiscoveryAttestationService } from './discovery-attestation.service';
import { CommunityInsightsService } from './community-insights.service';
import { vi } from 'vitest';

// =============================================================================
// Fixtures
// =============================================================================

const buildPublicResult = (overrides: Partial<DiscoveryResult> = {}): DiscoveryResult =>
  ({
    id: `result-${Math.floor(Math.random() * 10000)}`, // eslint-disable-line sonarjs/pseudo-random
    assessmentId: 'test-instrument',
    framework: 'test',
    category: 'personality',
    completedAt: '2025-06-01T00:00:00Z',
    primaryType: { typeId: 'type-a', name: 'Type A', score: 0.8 },
    subscaleScores: { 'sub-a': 10, 'sub-b': 8 },
    displayString: 'Type A',
    featured: false,
    displayOrder: 0,
    ...overrides,
  }) as unknown as DiscoveryResult;

// =============================================================================
// Tests
// =============================================================================

describe('CommunityInsightsService', () => {
  let service: CommunityInsightsService;
  let attestationSpy: any;

  beforeEach(() => {
    attestationSpy = {
      getResultsForContext: vi.fn().mockReturnValue([]),
      aggregatableResults: vi.fn().mockReturnValue([]),
    };

    TestBed.configureTestingModule({
      providers: [
        CommunityInsightsService,
        { provide: DiscoveryAttestationService, useValue: attestationSpy },
      ],
    });

    service = TestBed.inject(CommunityInsightsService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  // ---------------------------------------------------------------------------
  // getInstrumentDistribution
  // ---------------------------------------------------------------------------

  describe('getInstrumentDistribution()', () => {
    it('should return empty distribution when no results', () => {
      const dist = service.getInstrumentDistribution('nonexistent');

      expect(dist.totalResponses).toBe(0);
      expect(dist.typeDistribution).toEqual([]);
      expect(dist.subscaleStats).toEqual([]);
    });

    it('should compute type distribution', () => {
      const results = [
        buildPublicResult({ primaryType: { typeId: 'a', name: 'Type A', score: 0.8 } }),
        buildPublicResult({ primaryType: { typeId: 'a', name: 'Type A', score: 0.7 } }),
        buildPublicResult({ primaryType: { typeId: 'b', name: 'Type B', score: 0.9 } }),
      ];
      attestationSpy.aggregatableResults.mockReturnValue(results);

      const dist = service.getInstrumentDistribution('test-instrument');

      expect(dist.totalResponses).toBe(3);
      expect(dist.typeDistribution.length).toBe(2);

      const typeA = dist.typeDistribution.find(t => t.typeId === 'a')!;
      expect(typeA.count).toBe(2);

      const typeB = dist.typeDistribution.find(t => t.typeId === 'b')!;
      expect(typeB.count).toBe(1);
    });

    it('should compute subscale stats', () => {
      const results = [
        buildPublicResult({ subscaleScores: { 'sub-a': 10, 'sub-b': 5 } }),
        buildPublicResult({ subscaleScores: { 'sub-a': 20, 'sub-b': 15 } }),
        buildPublicResult({ subscaleScores: { 'sub-a': 30, 'sub-b': 25 } }),
      ];
      attestationSpy.aggregatableResults.mockReturnValue(results);

      const dist = service.getInstrumentDistribution('test-instrument');

      expect(dist.subscaleStats.length).toBe(2);
      const subA = dist.subscaleStats.find(s => s.subscaleId === 'sub-a')!;
      expect(subA.mean).toBe(20);
      expect(subA.min).toBe(10);
      expect(subA.max).toBe(30);
    });

    it('should flag noisy data for small cohorts', () => {
      const results = [buildPublicResult()];
      attestationSpy.aggregatableResults.mockReturnValue(results);

      const dist = service.getInstrumentDistribution('test-instrument');

      expect(dist.noisyData).toBe(true);
    });

    it('should not flag noisy data for large cohorts', () => {
      const results = Array.from({ length: 35 }, () => buildPublicResult());
      attestationSpy.aggregatableResults.mockReturnValue(results);

      const dist = service.getInstrumentDistribution('test-instrument');

      expect(dist.noisyData).toBe(false);
    });

    it('should include consentScope in distribution stats', () => {
      const dist = service.getInstrumentDistribution('test-instrument');
      expect(dist.consentScope).toBe('aggregate-only');
    });

    it('should use custom context when provided', () => {
      const contextResults = [buildPublicResult()];
      attestationSpy.getResultsForContext.mockReturnValue(contextResults);

      const context: AggregationContext = {
        requiredReach: 'commons',
        minimumConsent: 'anonymized',
      };
      const dist = service.getInstrumentDistribution('test-instrument', context);

      expect(attestationSpy.getResultsForContext).toHaveBeenCalledWith(context);
      expect(dist.totalResponses).toBe(1);
      expect(dist.consentScope).toBe('anonymized');
    });
  });

  // ---------------------------------------------------------------------------
  // getPercentile
  // ---------------------------------------------------------------------------

  describe('getPercentile()', () => {
    it('should return 0 percentile when no data', () => {
      const result = service.getPercentile('test-instrument', 'sub-a', 10);

      expect(result.percentile).toBe(0);
      expect(result.totalResponses).toBe(0);
    });

    it('should compute percentile rank', () => {
      const results = [
        buildPublicResult({ subscaleScores: { 'sub-a': 5 } }),
        buildPublicResult({ subscaleScores: { 'sub-a': 10 } }),
        buildPublicResult({ subscaleScores: { 'sub-a': 15 } }),
        buildPublicResult({ subscaleScores: { 'sub-a': 20 } }),
      ];
      attestationSpy.aggregatableResults.mockReturnValue(results);

      // Score of 15 is above 2 of 4 (5, 10) = 50th percentile
      const result = service.getPercentile('test-instrument', 'sub-a', 15);

      expect(result.percentile).toBe(50);
      expect(result.totalResponses).toBe(4);
    });

    it('should handle highest score', () => {
      const results = [
        buildPublicResult({ subscaleScores: { 'sub-a': 5 } }),
        buildPublicResult({ subscaleScores: { 'sub-a': 10 } }),
      ];
      attestationSpy.aggregatableResults.mockReturnValue(results);

      const result = service.getPercentile('test-instrument', 'sub-a', 100);

      expect(result.percentile).toBe(100);
    });

    it('should use custom context when provided', () => {
      const contextResults = [
        buildPublicResult({ subscaleScores: { 'sub-a': 5 } }),
        buildPublicResult({ subscaleScores: { 'sub-a': 15 } }),
      ];
      attestationSpy.getResultsForContext.mockReturnValue(contextResults);

      const context: AggregationContext = {
        requiredReach: 'commons',
        minimumConsent: 'anonymized',
      };
      const result = service.getPercentile('test-instrument', 'sub-a', 15, context);

      expect(attestationSpy.getResultsForContext).toHaveBeenCalledWith(context);
      expect(result.totalResponses).toBe(2);
    });
  });

  // ---------------------------------------------------------------------------
  // getCommunityTrends
  // ---------------------------------------------------------------------------

  describe('getCommunityTrends()', () => {
    it('should return empty when no data', () => {
      expect(service.getCommunityTrends('test-instrument')).toEqual([]);
    });

    it('should group results by month', () => {
      const results = [
        buildPublicResult({ completedAt: '2025-01-15T00:00:00Z' }),
        buildPublicResult({ completedAt: '2025-01-20T00:00:00Z' }),
        buildPublicResult({ completedAt: '2025-02-10T00:00:00Z' }),
      ];
      attestationSpy.aggregatableResults.mockReturnValue(results);

      const trends = service.getCommunityTrends('test-instrument');

      expect(trends.length).toBe(2);
      expect(trends[0].period).toBe('2025-01');
      expect(trends[0].responseCount).toBe(2);
      expect(trends[1].period).toBe('2025-02');
      expect(trends[1].responseCount).toBe(1);
    });

    it('should include type distribution per period', () => {
      const results = [
        buildPublicResult({
          completedAt: '2025-01-15T00:00:00Z',
          primaryType: { typeId: 'a', name: 'Type A', score: 0.8 },
        }),
        buildPublicResult({
          completedAt: '2025-01-20T00:00:00Z',
          primaryType: { typeId: 'b', name: 'Type B', score: 0.9 },
        }),
      ];
      attestationSpy.aggregatableResults.mockReturnValue(results);

      const trends = service.getCommunityTrends('test-instrument');

      expect(trends[0].typeDistribution.length).toBe(2);
    });

    it('should use custom context when provided', () => {
      const contextResults = [buildPublicResult({ completedAt: '2025-03-15T12:00:00Z' })];
      attestationSpy.getResultsForContext.mockReturnValue(contextResults);

      const context: AggregationContext = {
        requiredReach: 'commons',
        minimumConsent: 'anonymized',
      };
      const trends = service.getCommunityTrends('test-instrument', context);

      expect(attestationSpy.getResultsForContext).toHaveBeenCalledWith(context);
      expect(trends.length).toBe(1);
      expect(trends[0].period).toBe('2025-03');
    });
  });
});
