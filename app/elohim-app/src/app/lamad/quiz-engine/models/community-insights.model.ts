/**
 * Community Insights Model - Types for aggregated community statistics.
 *
 * These types support community-level insights from instrument data
 * while preserving individual privacy via differential privacy for
 * small cohorts (< 30 responses).
 */

import { type ReachLevel } from '@app/elohim/models/protocol-core.model';
import { type ResearchConsentScope } from '@app/lamad/models/knowledge-map.model';

// ─────────────────────────────────────────────────────────────────────────────
// Aggregation Context
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Specifies the viewer's context for consent-aware aggregation filtering.
 */
export interface AggregationContext {
  /** Viewer's reach level - only results at or above this scope are included */
  requiredReach: ReachLevel;

  /** Minimum consent level required for inclusion */
  minimumConsent: ResearchConsentScope;

  /** Exclude results from specific domains */
  excludedDomains?: string[];
}

// ─────────────────────────────────────────────────────────────────────────────
// Distribution
// ─────────────────────────────────────────────────────────────────────────────

export interface DistributionStats {
  instrumentId: string;
  totalResponses: number;

  /** Distribution of primary result types */
  typeDistribution: TypeDistributionEntry[];

  /** Per-subscale statistics */
  subscaleStats: SubscaleStats[];

  /** Whether noise was added (differential privacy for small cohorts) */
  noisyData: boolean;

  /** Consent level used for this aggregation */
  consentScope: ResearchConsentScope;

  /** Timestamp of aggregation */
  aggregatedAt: number;
}

export interface TypeDistributionEntry {
  typeId: string;
  typeName: string;
  count: number;
  percentage: number;
}

export interface SubscaleStats {
  subscaleId: string;
  subscaleName: string;
  mean: number;
  median: number;
  stdDev: number;
  min: number;
  max: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// Percentile
// ─────────────────────────────────────────────────────────────────────────────

export interface PercentileResult {
  instrumentId: string;
  subscaleId: string;
  score: number;
  percentile: number;
  totalResponses: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// Trends
// ─────────────────────────────────────────────────────────────────────────────

export interface TrendData {
  instrumentId: string;
  period: string;
  typeDistribution: TypeDistributionEntry[];
  responseCount: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// Recommended Retake
// ─────────────────────────────────────────────────────────────────────────────

export interface RecommendedRetake {
  instrumentId: string;
  instrumentName: string;
  lastTakenAt: number;
  recommendedInterval: string;
  dueAt: number;
  overdueDays: number;
}
