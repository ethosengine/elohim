/**
 * MechanismSelectionService - Governance Mechanism Ladder
 *
 * Given an entity's governance state, determines which feedback mechanism
 * to render and whether Angular (casual, levels 0-2) or Psephos (formal,
 * levels 3-7) handles the rendering.
 *
 * The mechanism ladder:
 *   Level 0: Context menu only (constitutional/settled content)
 *   Level 1: Reactions (casual content, no active proposal)
 *   Level 2: Graduated feedback (content inviting feedback, no active proposal)
 *   Level 3: Approval / Dot-vote (active proposal)
 *   Level 4: Ranked-choice (active proposal)
 *   Level 5: Score-vote / Conviction (active proposal)
 *   Level 6: Consent (active proposal)
 *   Level 7: (reserved for future constitutional mechanisms)
 */

import { Injectable } from '@angular/core';

import type { GovernanceStateView, ProposalView } from '@elohim/storage-client';

export interface MechanismSelection {
  /** 0-7 from mechanism ladder */
  level: number;
  /** Voting mechanism name */
  mechanism: string;
  /** Which renderer handles this level */
  renderTarget: 'angular' | 'psephos';
  /** Level 0: only context menu available */
  contextMenuOnly: boolean;
  /** Levels 1+: reactions allowed */
  allowReactions: boolean;
  /** Levels 2+: graduated feedback allowed */
  allowGraduatedFeedback: boolean;
  /** Levels 3-7: the active proposal to vote on */
  activeProposal?: ProposalView;
}

/** Voting mechanisms that indicate content invites feedback */
const FEEDBACK_CONTENT_TYPES = new Set([
  'discussion',
  'proposal-draft',
  'request-for-comment',
  'reflection',
]);

/** Map from proposal voting mechanism to ladder level */
const MECHANISM_LEVEL_MAP: Record<string, number> = {
  approval: 3,
  'dot-vote': 3,
  'ranked-choice': 4,
  'score-vote': 5,
  conviction: 5,
  consent: 6,
};

/** Governance states that lock content to level 0 (context menu only) */
const SETTLED_STATES = new Set(['constitutional', 'settled']);

@Injectable({ providedIn: 'root' })
export class MechanismSelectionService {
  /**
   * Select the appropriate feedback mechanism for an entity.
   *
   * @param governanceState - The entity's current governance state (null = untracked)
   * @param contentType - The content type of the entity
   * @param activeProposal - An active proposal if one exists for this entity
   * @returns The mechanism selection describing what to render and how
   */
  selectMechanism(
    governanceState: GovernanceStateView | null,
    contentType: string,
    activeProposal?: ProposalView
  ): MechanismSelection {
    // Rule 1: No governance state, or constitutional/settled content -> level 0
    if (!governanceState || SETTLED_STATES.has(governanceState.votingState)) {
      return {
        level: 0,
        mechanism: 'none',
        renderTarget: 'angular',
        contextMenuOnly: true,
        allowReactions: false,
        allowGraduatedFeedback: false,
      };
    }

    // Rule 4: Active proposal -> map mechanism to level 3-7
    if (activeProposal) {
      const level = MECHANISM_LEVEL_MAP[activeProposal.votingMechanism] ?? 3;
      return {
        level,
        mechanism: activeProposal.votingMechanism,
        renderTarget: 'psephos',
        contextMenuOnly: false,
        allowReactions: true,
        allowGraduatedFeedback: true,
        activeProposal,
      };
    }

    // Rule 3: Content that invites feedback -> level 2 (graduated)
    if (FEEDBACK_CONTENT_TYPES.has(contentType)) {
      return {
        level: 2,
        mechanism: 'graduated-feedback',
        renderTarget: 'angular',
        contextMenuOnly: false,
        allowReactions: true,
        allowGraduatedFeedback: true,
      };
    }

    // Rule 2: Casual content, no proposal -> level 1 (reactions only)
    return {
      level: 1,
      mechanism: 'reactions',
      renderTarget: 'angular',
      contextMenuOnly: false,
      allowReactions: true,
      allowGraduatedFeedback: false,
    };
  }
}
