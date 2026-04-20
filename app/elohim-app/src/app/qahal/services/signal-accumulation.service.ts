import { Injectable, inject } from '@angular/core';

import { GovernanceApiService } from '@app/elohim/services/governance-api.service';

export interface AccumulationStatus {
  totalSignals: number;
  uniqueParticipants: number;
  consensusStrength: number;
  readyForSensemaking: boolean;
  controversyDetected: boolean;
  settled: boolean;
}

/**
 * SignalAccumulationService — Derives governance thresholds from signal aggregates.
 *
 * Interprets raw aggregate data into actionable status flags:
 * - readyForSensemaking: enough signals but no consensus yet (deliberation needed)
 * - controversyDetected: significant participation with highly diverse views
 * - settled: strong consensus with sufficient participation
 */
@Injectable({ providedIn: 'root' })
export class SignalAccumulationService {
  private readonly governanceApi = inject(GovernanceApiService);

  async getAccumulationStatus(entityType: string, entityId: string): Promise<AccumulationStatus> {
    const aggregate = await this.governanceApi.getSignalAggregate(entityType, entityId);
    const totalSignals = Number(aggregate.totalSignals);
    const uniqueParticipants = Number(aggregate.uniqueParticipants);
    const consensusStrength = aggregate.consensusStrength;

    return {
      totalSignals,
      uniqueParticipants,
      consensusStrength,
      readyForSensemaking: totalSignals >= 20 && consensusStrength < 0.7,
      controversyDetected: totalSignals >= 10 && consensusStrength < 0.3,
      settled: totalSignals >= 30 && consensusStrength >= 0.85,
    };
  }
}
