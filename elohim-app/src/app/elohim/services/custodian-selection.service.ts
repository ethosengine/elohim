import { Injectable, inject, signal, computed } from '@angular/core';

// @coverage: 9.9% (2026-01-31)

import { CustodianCommitmentService } from './custodian-commitment.service';
import { ShefaService } from './shefa.service';

/**
 * CustodianSelectionService
 *
 * Selects the best custodian to serve content based on:
 * 1. Health (uptime %) - 40%
 * 2. Latency (geographic proximity) - 30%
 * 3. Bandwidth (available capacity) - 15%
 * 4. Specialization (domain expertise) - 10%
 * 5. Commitment (explicitly committed to this content) - 5%
 *
 * Returns highest-scoring custodian for CDN-like serving.
 */

export interface Custodian {
  id: string;
  endpoint: string; // HTTP endpoint for doorway
  domain: string;
  epic: string;
}

export interface CustodianScore {
  custodian: Custodian;
  health: number;
  latency: number;
  bandwidth: number;
  specialization: number;
  commitment: boolean;
  finalScore: number;
  breakdown: {
    healthScore: number;
    latencyScore: number;
    bandwidthScore: number;
    specializationScore: number;
    commitmentBonus: number;
  };
}

@Injectable({
  providedIn: 'root',
})
export class CustodianSelectionService {
  private readonly shefa = inject(ShefaService);
  private readonly commitments = inject(CustodianCommitmentService);

  // Cache selection results (2 minute TTL for stability)
  private readonly selectionCache = new Map<string, { score: CustodianScore; timestamp: number }>();
  private readonly CACHE_TTL_MS = 2 * 60 * 1000;

  // Statistics for monitoring
  private readonly statistics = signal({
    selectionsAttempted: 0,
    selectionsSuccessful: 0,
    cacheHits: 0,
    cacheMisses: 0,
  });

  readonly selectionStats = this.statistics.asReadonly();

  readonly successRate = computed(() => {
    const stats = this.statistics();
    if (stats.selectionsAttempted === 0) return 0;
    return (stats.selectionsSuccessful / stats.selectionsAttempted) * 100;
  });

  /**
   * Select the best custodian for content
   *
   * @param contentId - Hash of content to serve
   * @param userLocation - Optional: user's coordinates for latency calculation
   * @returns Best custodian with score breakdown, or null if none available
   */
  async selectBestCustodian(contentId: string): Promise<CustodianScore | null> {
    const stats = this.statistics();
    stats.selectionsAttempted++;

    // Check cache
    const cached = this.selectionCache.get(contentId);
    if (cached && Date.now() - cached.timestamp < this.CACHE_TTL_MS) {
      stats.cacheHits++;
      return cached.score;
    }

    stats.cacheMisses++;

    try {
      // Find all custodians committed to this content
      const custodians = await this.commitments.getCommitmentsForContent(contentId);

      if (custodians.length === 0) {
        return null;
      }

      // Score each custodian
      const scores: CustodianScore[] = [];

      for (const commitment of custodians) {
        try {
          const custodian: Custodian = {
            id: commitment.custodianId,
            endpoint: commitment.doorwayEndpoint,
            domain: commitment.domain,
            epic: commitment.epic,
          };

          const score = await this.scoreCustodian(custodian, commitment.stewardTier);

          if (score && score.finalScore > 0) {
            scores.push(score);
          }
        } catch {
          // Custodian scoring failed - skip this custodian and continue with next
          continue;
        }
      }

      if (scores.length === 0) {
        return null;
      }

      // Select highest-scoring custodian
      const best = scores.reduce(
        (prev, current) => (current.finalScore > prev.finalScore ? current : prev),
        scores[0]
      );

      // Cache result
      this.selectionCache.set(contentId, {
        score: best,
        timestamp: Date.now(),
      });

      stats.selectionsSuccessful++;

      return best;
    } catch {
      // Custodian selection failed - no suitable custodian found, return null as fallback
      return null;
    }
  }

  /**
   * Score a single custodian (0-100)
   *
   * Scoring formula:
   * score = (health × 0.4) +
   *         (latencyScore × 0.3) +
   *         (bandwidthScore × 0.15) +
   *         (specialization × 0.1) +
   *         (commitment ? 0.05 : 0) × 100
   */
  private async scoreCustodian(
    custodian: Custodian,
    stewardTier: 1 | 2 | 3 | 4
  ): Promise<CustodianScore | null> {
    try {
      // Get metrics from Shefa
      const metrics = await this.shefa.getMetrics(custodian.id);

      if (!metrics) {
        return null;
      }

      // Skip unhealthy custodians
      if (metrics.health.uptimePercent < 50 || !metrics.health.availability) {
        return null;
      }

      // Calculate component scores (0-1)
      const healthScore = this.calculateHealthScore(metrics.health.uptimePercent);

      const latencyScore = this.calculateLatencyScore(metrics.health.responseTimeP95Ms);

      const bandwidthScore = this.calculateBandwidthScore(
        metrics.bandwidth.currentUsageMbps,
        metrics.bandwidth.declaredMbps
      );

      const specializationScore = this.calculateSpecializationScore(
        metrics.reputation.specializationBonus
      );

      // Steward tier bonus (higher tier = better)
      const tierBonus = this.getTierBonus(stewardTier);

      // SLA compliance bonus
      const slaBonus = metrics.health.slaCompliance ? 0.05 : 0;

      // Weighted sum (result is 0-1)
      const rawScore =
        healthScore * 0.4 +
        latencyScore * 0.3 +
        bandwidthScore * 0.15 +
        specializationScore * 0.1 +
        tierBonus * 0.05 +
        slaBonus * 0;

      // Convert to 0-100 scale
      const finalScore = Math.min(100, Math.max(0, rawScore * 100));

      return {
        custodian,
        health: metrics.health.uptimePercent,
        latency: metrics.health.responseTimeP95Ms,
        bandwidth: metrics.bandwidth.currentUsageMbps / metrics.bandwidth.declaredMbps,
        specialization: metrics.reputation.specializationBonus,
        commitment: true, // We only score committed custodians
        finalScore,
        breakdown: {
          healthScore: Math.round(healthScore * 100 * 10) / 10,
          latencyScore: Math.round(latencyScore * 100 * 10) / 10,
          bandwidthScore: Math.round(bandwidthScore * 100 * 10) / 10,
          specializationScore: Math.round(specializationScore * 100 * 10) / 10,
          commitmentBonus: 5,
        },
      };
    } catch {
      // Custodian metrics fetch failed - unable to score, return null as fallback
      return null;
    }
  }

  /**
   * Calculate health score (0-1)
   * 100% uptime = 1.0
   * 95% uptime = 0.95
   * 50% uptime = 0.5
   * <50% = 0
   */
  private calculateHealthScore(uptimePercent: number): number {
    if (uptimePercent < 50) return 0;
    return Math.min(1, uptimePercent / 100);
  }

  /**
   * Calculate latency score (0-1)
   * 50ms = 1.0 (perfect)
   * 200ms = 0.9
   * 500ms = 0.5
   * 2000ms+ = 0
   */
  private calculateLatencyScore(latencyMs: number): number {
    if (latencyMs > 2000) return 0;
    if (latencyMs < 50) return 1.0;

    // Linear interpolation
    return 1.0 - (latencyMs - 50) / (2000 - 50);
  }

  /**
   * Calculate bandwidth score (0-1)
   * Low utilization (< 50%) = 1.0 (has capacity)
   * Medium utilization (50-80%) = 0.75
   * High utilization (80-95%) = 0.25 (limited capacity)
   * >95% = 0 (no capacity)
   */
  private calculateBandwidthScore(currentMbps: number, declaredMbps: number): number {
    const utilizationPercent = (currentMbps / declaredMbps) * 100;

    if (utilizationPercent > 95) return 0;
    if (utilizationPercent > 80) return 0.25;
    if (utilizationPercent > 50) return 0.75;
    return 1.0;
  }

  /**
   * Calculate specialization score (0-1)
   * Specialization bonus ranges from 0 to 0.1 (10%)
   * Convert to 0-1 scale
   */
  private calculateSpecializationScore(specializationBonus: number): number {
    // Specialization bonus is 0-0.1
    // Convert to 0-1 score
    return Math.min(1, specializationBonus * 10);
  }

  /**
   * Get tier bonus (0-0.05)
   * Pioneer (4) gets full bonus
   * Expert (3) gets 75% bonus
   * Curator (2) gets 50% bonus
   * Caretaker (1) gets 25% bonus
   */
  private getTierBonus(tier: 1 | 2 | 3 | 4): number {
    const bonuses: Record<number, number> = {
      1: 0.0125, // 25% of 0.05
      2: 0.025, // 50% of 0.05
      3: 0.0375, // 75% of 0.05
      4: 0.05, // 100% of 0.05
    };

    return bonuses[tier] || 0;
  }

  /**
   * Batch score multiple custodians
   * Useful for analytics and monitoring
   */
  async scoreAllCustodians(): Promise<CustodianScore[]> {
    try {
      const allMetrics = await this.shefa.getAllMetrics();

      const scores: CustodianScore[] = [];

      for (const metrics of allMetrics) {
        const custodian: Custodian = {
          id: metrics.custodianId,
          endpoint: `https://${metrics.custodianId}.example.com`,
          domain: 'unknown',
          epic: 'unknown',
        };

        const score = await this.scoreCustodian(custodian, metrics.economic.stewardTier);

        if (score) {
          scores.push(score);
        }
      }

      return scores.sort((a, b) => b.finalScore - a.finalScore);
    } catch {
      // Batch scoring failed - no custodians scored, return empty list as fallback
      return [];
    }
  }

  /**
   * Get top N custodians by score
   */
  async getTopCustodians(limit = 10): Promise<CustodianScore[]> {
    const scores = await this.scoreAllCustodians();
    return scores.slice(0, limit);
  }

  /**
   * Clear selection cache
   */
  clearCache(): void {
    this.selectionCache.clear();
  }

  /**
   * Get current selection statistics
   */
  getStatistics() {
    return this.statistics();
  }
}
