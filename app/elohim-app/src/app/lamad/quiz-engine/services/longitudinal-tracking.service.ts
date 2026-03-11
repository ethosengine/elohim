/**
 * Longitudinal Tracking Service - Retake history and growth tracking.
 *
 * Tracks retake history per instrument, computes changes between
 * assessments, detects pattern alerts for significant shifts, and
 * surfaces recommended retake intervals from instrument metadata.
 *
 * NOTE: This service operates on the user's OWN data (not community aggregation),
 * so it does not need consent-aware filtering via AggregationContext. All methods
 * read from `attestationService.results()` which contains only the current user's
 * discovery results.
 */

import { Injectable, inject, computed } from '@angular/core';

// @coverage: 73.1% (2026-02-24)

import { getInstrument, getRegisteredInstrumentIds } from '../instruments/instrument-registry';

import { DiscoveryAttestationService } from './discovery-attestation.service';

import type { RecommendedRetake } from '../models/community-insights.model';
import type { DiscoveryResult } from '../models/discovery-assessment.model';
import type { LongitudinalChange, PatternAlert } from '@app/lamad/models/knowledge-map.model';

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/** Minimum score change (as fraction of range) to be considered significant */
const SIGNIFICANCE_THRESHOLD = 0.15;

/** Minimum score change to be considered marginal */
const MARGINAL_THRESHOLD = 0.05;

/** Default recommended retake interval in milliseconds (6 months) */
const DEFAULT_RETAKE_INTERVAL_MS = 180 * 24 * 60 * 60 * 1000;

// ─────────────────────────────────────────────────────────────────────────────
// Service
// ─────────────────────────────────────────────────────────────────────────────

@Injectable({ providedIn: 'root' })
export class LongitudinalTrackingService {
  private readonly attestationService = inject(DiscoveryAttestationService);

  /** All results from the attestation service */
  private readonly allResults = computed(() => this.attestationService.results());

  // ---------------------------------------------------------------------------
  // Public Methods
  // ---------------------------------------------------------------------------

  /**
   * Get retake history for an instrument, sorted by completion date.
   */
  getRetakeHistory(instrumentId: string): DiscoveryResult[] {
    return this.allResults()
      .filter(r => r.assessmentId === instrumentId)
      .sort((a, b) => new Date(a.completedAt).getTime() - new Date(b.completedAt).getTime());
  }

  /**
   * Compute longitudinal changes between the two most recent retakes.
   */
  computeChanges(instrumentId: string): LongitudinalChange[] {
    const history = this.getRetakeHistory(instrumentId);
    if (history.length < 2) return [];

    const previous = history.at(-2)!;
    const current = history.at(-1)!;

    const allSubscales = new Set([
      ...Object.keys(previous.subscaleScores ?? {}),
      ...Object.keys(current.subscaleScores ?? {}),
    ]);

    const changes: LongitudinalChange[] = [];

    for (const subscale of allSubscales) {
      const prevScore = previous.subscaleScores?.[subscale] ?? 0;
      const currScore = current.subscaleScores?.[subscale] ?? 0;
      const delta = currScore - prevScore;
      const absDelta = Math.abs(delta);

      let changeDirection: LongitudinalChange['changeDirection'];
      if (delta > MARGINAL_THRESHOLD) changeDirection = 'increased';
      else if (delta < -MARGINAL_THRESHOLD) changeDirection = 'decreased';
      else changeDirection = 'stable';

      let significance: LongitudinalChange['significance'];
      if (absDelta >= SIGNIFICANCE_THRESHOLD) significance = 'significant';
      else if (absDelta >= MARGINAL_THRESHOLD) significance = 'marginal';
      else significance = 'none';

      changes.push({
        subscale,
        previousScore: prevScore,
        currentScore: currScore,
        changeDirection,
        significance,
        interpretation: this.buildChangeInterpretation(subscale, changeDirection, significance),
      });
    }

    return changes;
  }

  /**
   * Get instruments that are past their recommended retake interval.
   */
  getRecommendedRetakes(): RecommendedRetake[] {
    const now = Date.now();
    const retakes: RecommendedRetake[] = [];

    for (const instrumentId of getRegisteredInstrumentIds()) {
      const history = this.getRetakeHistory(instrumentId);
      if (history.length === 0) continue;

      const lastResult = history.at(-1)!;
      const lastTakenAt = new Date(lastResult.completedAt).getTime();
      const entry = getInstrument(instrumentId);
      const intervalMs = DEFAULT_RETAKE_INTERVAL_MS;

      const dueAt = lastTakenAt + intervalMs;
      if (now < dueAt) continue;

      retakes.push({
        instrumentId,
        instrumentName: entry?.config.name ?? instrumentId,
        lastTakenAt,
        recommendedInterval: 'P6M',
        dueAt,
        overdueDays: Math.floor((now - dueAt) / (24 * 60 * 60 * 1000)),
      });
    }

    return retakes;
  }

  /**
   * Detect pattern alerts from longitudinal data.
   * Looks for significant shifts that may warrant attention.
   */
  getPatternAlerts(): PatternAlert[] {
    const alerts: PatternAlert[] = [];

    for (const instrumentId of getRegisteredInstrumentIds()) {
      const changes = this.computeChanges(instrumentId);
      const significantChanges = changes.filter(c => c.significance === 'significant');

      for (const change of significantChanges) {
        alerts.push({
          id: `alert-${instrumentId}-${change.subscale}-${Date.now()}`,
          detectedAt: new Date().toISOString(),
          patternType:
            change.changeDirection === 'increased' ? 'growth-opportunity' : 'longitudinal-shift',
          urgency: 'gentle-nudge',
          description: change.interpretation,
          evidence: [
            {
              assessmentResultId: instrumentId,
              subscale: change.subscale,
              observation: `Score changed from ${change.previousScore.toFixed(2)} to ${change.currentScore.toFixed(2)}`,
              weight: 1,
            },
          ],
          suggestedActions: [],
          acknowledged: false,
        });
      }
    }

    return alerts;
  }

  // ---------------------------------------------------------------------------
  // Private Helpers
  // ---------------------------------------------------------------------------

  private buildChangeInterpretation(
    subscale: string,
    direction: LongitudinalChange['changeDirection'],
    significance: LongitudinalChange['significance']
  ): string {
    if (significance === 'none') return `Your ${subscale} score has remained stable.`;
    const qualifier = significance === 'significant' ? 'notably' : 'slightly';
    const verb = direction === 'increased' ? 'increased' : 'decreased';
    return `Your ${subscale} score has ${qualifier} ${verb} since your last assessment.`;
  }
}
