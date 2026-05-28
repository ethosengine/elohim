/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/mechanism-selection.schema.json -- DO NOT EDIT */

/**
 * Source of truth: derived projection of GovernanceState × Content.contentType × Proposal? × Manifest{kind: 'pillar-projection', pillar: 'qahal'} payload. Category C operational projection — reconstructable at any time from the substrate entries. Triggers: Signal::GovernanceStateUpdated, Signal::ContentCreated/Updated, Signal::ProposalCreated/Closed, Signal::ManifestUpdated{kind: 'pillar-projection', pillar: 'qahal'}.
 */
export interface MechanismSelectionView {
  /**
   * The type of entity (e.g. 'content', 'path', 'proposal')
   */
  entityType: string;
  /**
   * The unique identifier of the entity
   */
  entityId: string;
  /**
   * Mechanism ladder level (0–7). 0 = context menu only; 1 = reactions; 2 = graduated feedback; 3–7 = formal voting mechanisms
   */
  level: number;
  /**
   * Name of the selected mechanism: 'none' (level 0), 'reactions' (level 1), 'graduated-feedback' (level 2), or the proposal's votingMechanism (levels 3–7)
   */
  mechanism: string;
  /**
   * Which renderer handles this level. 'angular' for levels 0–2; 'psephos' for levels 3–7 (formal voting)
   */
  renderTarget: 'angular' | 'psephos';
  /**
   * True at level 0: only the context menu is available (no reactions, no feedback, no voting)
   */
  contextMenuOnly: boolean;
  /**
   * True at levels 1+: reaction bar is available
   */
  allowReactions: boolean;
  /**
   * True at levels 2+: graduated feedback (Loomio-style scale) is available
   */
  allowGraduatedFeedback: boolean;
  /**
   * For levels 3–7: the ID of the active proposal driving the formal mechanism. Null when no active proposal.
   */
  activeProposalId?: string | null;
  /**
   * For levels 3–7: the votingMechanism of the active proposal (e.g. 'approval', 'ranked-choice'). Null when no active proposal.
   */
  activeProposalMechanism?: string | null;
  /**
   * CID of the pillar-projection Manifest whose mechanism_ladder was applied. Null when protocol defaults applied.
   */
  policyManifestCid?: string | null;
  /**
   * ISO-8601 timestamp when this projection was last computed
   */
  computedAt: string;
}
