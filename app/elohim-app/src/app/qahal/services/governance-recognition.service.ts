import { inject, Injectable } from '@angular/core';

import { firstValueFrom } from 'rxjs';

import {
  RecognitionApiService,
  RecognitionTrigger,
} from '@app/elohim/services/recognition-api.service';

/**
 * GovernanceRecognitionService - REA Economic Events for Governance Participation
 *
 * After each governance signal submission (reaction, graduated feedback, vote),
 * this service generates an REA economic event via the recognition pipeline.
 * This connects governance participation to stewardship -- curation acts build affinity.
 *
 * Mechanism levels map to recognition weight:
 * - Level 0-1 (context menu, reactions): low weight (0.1)
 * - Level 2-3 (graduated feedback, approval): medium weight (0.3-0.5)
 * - Level 4-6 (ranked-choice, score, consent): high weight (0.7-0.8)
 * - Level 7 (deliberation): highest weight (1.0)
 *
 * The weight determines how much stewardship affinity this participation earns.
 * Higher-friction mechanisms earn more because they demand more thoughtful engagement.
 */
@Injectable({ providedIn: 'root' })
export class GovernanceRecognitionService {
  private readonly recognitionApi = inject(RecognitionApiService);

  /**
   * Recognition weight by mechanism level.
   *
   * Mirrors the graduated friction model: low-friction acts earn less,
   * high-friction deliberative acts earn more. This is the economic
   * expression of "curation acts build affinity."
   */
  private static readonly WEIGHT_BY_LEVEL: Record<number, number> = {
    0: 0.1, // context-menu-only (passive signal)
    1: 0.1, // reaction (low friction)
    2: 0.3, // graduated feedback (medium friction)
    3: 0.5, // approval voting
    4: 0.7, // ranked-choice voting
    5: 0.7, // score voting
    6: 0.8, // consent-based decision
    7: 1, // deliberation (highest friction)
  };

  /**
   * Record a governance participation event in the REA recognition pipeline.
   *
   * This is fire-and-forget from the caller's perspective -- recognition
   * failures must never block governance signal recording. The caller
   * should catch errors silently.
   *
   * @param params.entityType - What is being governed (e.g. 'content', 'proposal')
   * @param params.entityId - ID of the governed entity
   * @param params.humanId - Who participated
   * @param params.mechanismLevel - Governance mechanism level (0-7)
   * @param params.participationType - Type of participation act
   */
  async recordParticipation(params: {
    entityType: string;
    entityId: string;
    humanId: string;
    mechanismLevel: number;
    participationType: 'reaction' | 'graduated-feedback' | 'vote' | 'block';
  }): Promise<void> {
    const weight = this.getRecognitionWeight(params.mechanismLevel);

    const trigger: RecognitionTrigger = {
      contentId: params.entityId,
      eventType: `governance-${params.participationType}`,
      rawAmount: weight,
      triggeredBy: params.humanId,
    };

    await firstValueFrom(this.recognitionApi.distribute(trigger));
  }

  /**
   * Map mechanism level to recognition weight.
   * Unknown levels default to the lowest weight.
   */
  private getRecognitionWeight(level: number): number {
    return GovernanceRecognitionService.WEIGHT_BY_LEVEL[level] ?? 0.1;
  }
}
