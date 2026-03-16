import { Injectable, inject } from '@angular/core';

import { GovernanceApiService } from '@app/elohim/services/governance-api.service';
import type {
  SensemakingResultView,
  ProposalView,
  CreateProposalInputView,
  CreateProposalOptionInputView,
} from '@elohim/storage-client/generated';

/**
 * BracketSynthesisService - Layer B Preparation Seam
 *
 * Converts sensemaking results (Layer A output: clusters + bridging
 * statements) into a ranked-choice proposal (Layer B input).
 *
 * The bridging statements — the ones that find agreement across opinion
 * clusters — become the options in a ranked-choice ballot. This is the
 * mechanism by which community sensemaking produces actionable governance.
 *
 * "The goal is not to win, but to understand." Layer A discovers
 * understanding. This service bridges to Layer B where the community
 * decides what to do with that understanding.
 */
@Injectable({ providedIn: 'root' })
export class BracketSynthesisService {
  private readonly governanceApi = inject(GovernanceApiService);

  /**
   * Create a ranked-choice proposal from bridging statements identified
   * through sensemaking. Each bridging statement becomes a ballot option.
   *
   * @param entityType The governance entity type (e.g. 'content', 'policy')
   * @param entityId The governed entity ID
   * @param sensemakingResult The clustering output from Layer A
   * @returns The created proposal, ready for ranked-choice voting
   */
  async synthesizeBracket(
    entityType: string,
    entityId: string,
    sensemakingResult: SensemakingResultView
  ): Promise<ProposalView> {
    const proposalInput: CreateProposalInputView = {
      id: crypto.randomUUID(),
      contentId: entityId,
      proposerPresenceId: 'system-sensemaking',
      proposalType: 'ranked-choice',
      title: `Community synthesis: What should we do about ${entityType} ${entityId}?`,
      body: 'Ranked-choice vote on bridging statements identified through sensemaking',
      votingAnonymous: false,
      votingMechanism: 'ranked-choice',
      scoreMin: null,
      scoreMax: null,
      dotsPerVoter: null,
      quorumPercentage: null,
      passageThreshold: null,
    };

    const proposal = await this.governanceApi.createProposal(proposalInput);

    const options: CreateProposalOptionInputView[] =
      sensemakingResult.bridgingStatements.map((s, i) => ({
        id: `synth-opt-${i}`,
        label: s.text.substring(0, 100),
        description: s.text,
        position: i,
        source: s.id,
        sourceJustification: s.isBridging
          ? `Bridging statement (agree: ${s.agreeCount}, disagree: ${s.disagreeCount})`
          : null,
      }));

    if (options.length > 0) {
      await this.governanceApi.createProposalOptions(proposal.id, options);
    }

    return proposal;
  }
}
