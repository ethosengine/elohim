/**
 * MechanismSelectionService Tests
 *
 * Tests the governance mechanism ladder: mapping governance state,
 * content type, and active proposal to the correct feedback level.
 */

import { describe, expect, it } from 'vitest';

import type { GovernanceStateView, ProposalView } from '@elohim/storage-client';

import { MechanismSelectionService } from './mechanism-selection.service';

describe('MechanismSelectionService', () => {
  const service = new MechanismSelectionService();

  /** Helper to build a minimal GovernanceStateView with a given votingState */
  function makeState(votingState: string): GovernanceStateView {
    return {
      id: 'gs-1',
      entityType: 'content',
      entityId: 'entity-1',
      reach: 'community',
      labels: null,
      votingState,
      signalCount: 0,
      createdAt: '2026-03-15T00:00:00Z',
      updatedAt: '2026-03-15T00:00:00Z',
    };
  }

  /** Helper to build a minimal ProposalView with a given votingMechanism */
  function makeProposal(votingMechanism: string): ProposalView {
    return {
      id: 'prop-1',
      contentId: 'entity-1',
      proposerPresenceId: 'presence-1',
      proposalType: 'amendment',
      title: 'Test Proposal',
      body: 'Body text',
      status: 'open',
      votesFor: 0,
      votesAgainst: 0,
      votingAnonymous: false,
      createdAt: '2026-03-15T00:00:00Z',
      updatedAt: '2026-03-15T00:00:00Z',
      votingMechanism,
      scoreMin: null,
      scoreMax: null,
      dotsPerVoter: null,
      quorumPercentage: null,
      passageThreshold: null,
    };
  }

  describe('Level 0 — context menu only', () => {
    it('returns level 0 when governance state is null', () => {
      const result = service.selectMechanism(null, 'article');
      expect(result.level).toBe(0);
      expect(result.renderTarget).toBe('angular');
      expect(result.contextMenuOnly).toBe(true);
      expect(result.allowReactions).toBe(false);
      expect(result.allowGraduatedFeedback).toBe(false);
      expect(result.mechanism).toBe('none');
    });

    it('returns level 0 for constitutional governance state', () => {
      const result = service.selectMechanism(makeState('constitutional'), 'article');
      expect(result.level).toBe(0);
      expect(result.renderTarget).toBe('angular');
      expect(result.contextMenuOnly).toBe(true);
    });

    it('returns level 0 for settled governance state', () => {
      const result = service.selectMechanism(makeState('settled'), 'article');
      expect(result.level).toBe(0);
      expect(result.contextMenuOnly).toBe(true);
    });
  });

  describe('Level 1 — reactions (casual content, no proposal)', () => {
    it('returns level 1 for active state with no proposal and non-feedback content', () => {
      const result = service.selectMechanism(makeState('active'), 'article');
      expect(result.level).toBe(1);
      expect(result.mechanism).toBe('reactions');
      expect(result.renderTarget).toBe('angular');
      expect(result.allowReactions).toBe(true);
      expect(result.allowGraduatedFeedback).toBe(false);
      expect(result.contextMenuOnly).toBe(false);
    });
  });

  describe('Level 2 — graduated feedback (feedback content, no proposal)', () => {
    it('returns level 2 for discussion content type', () => {
      const result = service.selectMechanism(makeState('active'), 'discussion');
      expect(result.level).toBe(2);
      expect(result.mechanism).toBe('graduated-feedback');
      expect(result.renderTarget).toBe('angular');
      expect(result.allowReactions).toBe(true);
      expect(result.allowGraduatedFeedback).toBe(true);
    });

    it('returns level 2 for proposal-draft content type', () => {
      const result = service.selectMechanism(makeState('active'), 'proposal-draft');
      expect(result.level).toBe(2);
    });

    it('returns level 2 for request-for-comment content type', () => {
      const result = service.selectMechanism(makeState('active'), 'request-for-comment');
      expect(result.level).toBe(2);
    });

    it('returns level 2 for reflection content type', () => {
      const result = service.selectMechanism(makeState('active'), 'reflection');
      expect(result.level).toBe(2);
    });
  });

  describe('Levels 3-7 — active proposal mechanisms', () => {
    it('returns level 4 and psephos for ranked-choice', () => {
      const result = service.selectMechanism(
        makeState('active'),
        'article',
        makeProposal('ranked-choice'),
      );
      expect(result.level).toBe(4);
      expect(result.renderTarget).toBe('psephos');
      expect(result.mechanism).toBe('ranked-choice');
      expect(result.allowReactions).toBe(true);
      expect(result.allowGraduatedFeedback).toBe(true);
      expect(result.activeProposal).toBeDefined();
    });

    it('returns level 3 and psephos for approval', () => {
      const result = service.selectMechanism(
        makeState('active'),
        'article',
        makeProposal('approval'),
      );
      expect(result.level).toBe(3);
      expect(result.renderTarget).toBe('psephos');
      expect(result.mechanism).toBe('approval');
    });

    it('returns level 6 and psephos for consent', () => {
      const result = service.selectMechanism(
        makeState('active'),
        'article',
        makeProposal('consent'),
      );
      expect(result.level).toBe(6);
      expect(result.renderTarget).toBe('psephos');
      expect(result.mechanism).toBe('consent');
    });

    it('returns level 3 and psephos for dot-vote', () => {
      const result = service.selectMechanism(
        makeState('active'),
        'article',
        makeProposal('dot-vote'),
      );
      expect(result.level).toBe(3);
      expect(result.renderTarget).toBe('psephos');
      expect(result.mechanism).toBe('dot-vote');
    });

    it('returns level 5 and psephos for score-vote', () => {
      const result = service.selectMechanism(
        makeState('active'),
        'article',
        makeProposal('score-vote'),
      );
      expect(result.level).toBe(5);
      expect(result.renderTarget).toBe('psephos');
      expect(result.mechanism).toBe('score-vote');
    });

    it('returns level 5 and psephos for conviction', () => {
      const result = service.selectMechanism(
        makeState('active'),
        'article',
        makeProposal('conviction'),
      );
      expect(result.level).toBe(5);
      expect(result.renderTarget).toBe('psephos');
    });

    it('defaults to level 3 for unknown mechanism', () => {
      const result = service.selectMechanism(
        makeState('active'),
        'article',
        makeProposal('some-future-mechanism'),
      );
      expect(result.level).toBe(3);
      expect(result.renderTarget).toBe('psephos');
    });

    it('includes the activeProposal in the result', () => {
      const proposal = makeProposal('approval');
      const result = service.selectMechanism(makeState('active'), 'article', proposal);
      expect(result.activeProposal).toBe(proposal);
    });
  });

  describe('Priority: proposal overrides content type', () => {
    it('returns proposal level even for feedback content type when proposal is active', () => {
      const result = service.selectMechanism(
        makeState('active'),
        'discussion',
        makeProposal('consent'),
      );
      // Proposal takes priority over feedback content type
      expect(result.level).toBe(6);
      expect(result.renderTarget).toBe('psephos');
    });
  });
});
