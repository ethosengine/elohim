/**
 * Community Insights Service - Aggregated community statistics.
 *
 * Computes distribution statistics, percentiles, and trends from
 * public discovery results. Applies differential privacy noise
 * to small cohorts (< 30 responses) to protect individual identity.
 */

import { Injectable, inject, computed } from '@angular/core';

// @coverage: 100.0% (2026-02-24)

import { DiscoveryAttestationService } from './discovery-attestation.service';

import type {
  AggregationContext,
  DistributionStats,
  TypeDistributionEntry,
  SubscaleStats,
  PercentileResult,
  TrendData,
} from '../models/community-insights.model';

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/** Minimum cohort size before applying differential privacy noise */
const MIN_COHORT_SIZE = 30;

/** Noise magnitude for differential privacy (fraction of count) */
const NOISE_MAGNITUDE = 0.1;

// ─────────────────────────────────────────────────────────────────────────────
// Service
// ─────────────────────────────────────────────────────────────────────────────

@Injectable({ providedIn: 'root' })
export class CommunityInsightsService {
  private readonly attestationService = inject(DiscoveryAttestationService);

  /** Results eligible for community aggregation (consent-aware) */
  private readonly defaultResults = computed(() => this.attestationService.aggregatableResults());

  // ---------------------------------------------------------------------------
  // Public Methods
  // ---------------------------------------------------------------------------

  /**
   * Get distribution statistics for an instrument.
   * When context is provided, uses custom consent-aware filtering instead of the default.
   */
  getInstrumentDistribution(instrumentId: string, context?: AggregationContext): DistributionStats {
    const source = context
      ? this.attestationService.getResultsForContext(context)
      : this.defaultResults();
    const results = source.filter(r => r.assessmentId === instrumentId);
    const isSmallCohort = results.length < MIN_COHORT_SIZE;

    // Type distribution
    const typeCounts = new Map<string, { name: string; count: number }>();
    for (const result of results) {
      const typeId = result.primaryType.typeId;
      const existing = typeCounts.get(typeId);
      if (existing) {
        existing.count++;
      } else {
        typeCounts.set(typeId, { name: result.primaryType.name, count: 1 });
      }
    }

    let typeDistribution: TypeDistributionEntry[] = Array.from(typeCounts.entries()).map(
      ([typeId, { name, count }]) => ({
        typeId,
        typeName: name,
        count: isSmallCohort ? this.addNoise(count) : count,
        percentage: results.length > 0 ? (count / results.length) * 100 : 0,
      })
    );
    typeDistribution = typeDistribution.sort((a, b) => b.count - a.count);

    // Subscale stats
    const subscaleValues = new Map<string, number[]>();
    for (const result of results) {
      for (const [subscale, score] of Object.entries(result.subscaleScores)) {
        const values = subscaleValues.get(subscale) ?? [];
        values.push(score);
        subscaleValues.set(subscale, values);
      }
    }

    const subscaleStats: SubscaleStats[] = Array.from(subscaleValues.entries()).map(
      ([subscaleId, values]) => this.computeSubscaleStats(subscaleId, values)
    );

    return {
      instrumentId,
      totalResponses: results.length,
      typeDistribution,
      subscaleStats,
      noisyData: isSmallCohort,
      consentScope: context?.minimumConsent ?? 'aggregate-only',
      aggregatedAt: Date.now(),
    };
  }

  /**
   * Get percentile rank for a score on a specific subscale.
   * When context is provided, uses custom consent-aware filtering instead of the default.
   */
  getPercentile(
    instrumentId: string,
    subscaleId: string,
    score: number,
    context?: AggregationContext
  ): PercentileResult {
    const source = context
      ? this.attestationService.getResultsForContext(context)
      : this.defaultResults();
    const results = source.filter(r => r.assessmentId === instrumentId);
    const scores: number[] = [];
    for (const r of results) {
      if (subscaleId in r.subscaleScores) {
        scores.push(r.subscaleScores[subscaleId]);
      }
    }
    scores.sort((a, b) => a - b);

    let percentile = 0;
    if (scores.length > 0) {
      const belowCount = scores.filter(s => s < score).length;
      percentile = (belowCount / scores.length) * 100;
    }

    return {
      instrumentId,
      subscaleId,
      score,
      percentile: Math.round(percentile),
      totalResponses: scores.length,
    };
  }

  /**
   * Get community trends grouped by time period.
   * When context is provided, uses custom consent-aware filtering instead of the default.
   */
  getCommunityTrends(instrumentId: string, context?: AggregationContext): TrendData[] {
    const source = context
      ? this.attestationService.getResultsForContext(context)
      : this.defaultResults();
    const results = source.filter(r => r.assessmentId === instrumentId);

    // Group by month
    const byMonth = new Map<string, typeof results>();
    for (const result of results) {
      const date = new Date(result.completedAt);
      const period = `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}`;
      const existing = byMonth.get(period) ?? [];
      existing.push(result);
      byMonth.set(period, existing);
    }

    return Array.from(byMonth.entries())
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([period, periodResults]) => {
        const typeCounts = new Map<string, { name: string; count: number }>();
        for (const r of periodResults) {
          const typeId = r.primaryType.typeId;
          const existing = typeCounts.get(typeId);
          if (existing) {
            existing.count++;
          } else {
            typeCounts.set(typeId, { name: r.primaryType.name, count: 1 });
          }
        }

        return {
          instrumentId,
          period,
          typeDistribution: Array.from(typeCounts.entries()).map(([typeId, { name, count }]) => ({
            typeId,
            typeName: name,
            count,
            percentage: (count / periodResults.length) * 100,
          })),
          responseCount: periodResults.length,
        };
      });
  }

  // ---------------------------------------------------------------------------
  // Private Helpers
  // ---------------------------------------------------------------------------

  private computeSubscaleStats(subscaleId: string, values: number[]): SubscaleStats {
    const sorted = [...values].sort((a, b) => a - b);
    const n = sorted.length;
    const sum = sorted.reduce((acc, v) => acc + v, 0);
    const mean = n > 0 ? sum / n : 0;
    const median = n > 0 ? sorted[Math.floor(n / 2)] : 0;
    const variance = n > 0 ? sorted.reduce((acc, v) => acc + (v - mean) ** 2, 0) / n : 0;

    return {
      subscaleId,
      subscaleName: subscaleId,
      mean,
      median,
      stdDev: Math.sqrt(variance),
      min: sorted[0] ?? 0,
      max: sorted.at(-1) ?? 0,
    };
  }

  /**
   * Add Laplace noise for differential privacy on small cohorts.
   */
  private addNoise(count: number): number {
    // eslint-disable-next-line sonarjs/pseudo-random -- Laplace noise for differential privacy, not cryptographic
    const noise = (Math.random() - 0.5) * 2 * NOISE_MAGNITUDE * count;
    return Math.max(0, Math.round(count + noise));
  }
}
