import { describe, expect, it } from 'vitest';

import {
  ROUTED_CONCERN_KINDS,
  challengeInput,
  concernSignal,
  consensusWords,
  landedSummary,
  reactionCounts,
  reactionSignal,
  wireCount,
} from './epr-home-signals';

import type { SignalAggregateView } from '@elohim/storage-client/generated';

/** bigint-typed fields arrive as JSON numbers on the wire. */
function aggregate(over: Record<string, unknown>): SignalAggregateView {
  return {
    entityType: 'content',
    entityId: 'evolution-of-trust',
    totalSignals: 0,
    byType: {},
    byValue: {},
    uniqueParticipants: 0,
    consensusStrength: 0,
    ...over,
  } as unknown as SignalAggregateView;
}

describe('epr-home-signals', () => {
  it('reads bigint-typed wire counts that arrive as numbers', () => {
    expect(wireCount(3)).toBe(3);
    expect(wireCount(BigInt(4))).toBe(4);
    expect(wireCount(undefined)).toBe(0);
  });

  it('maps the aggregate onto reaction counts, reaction words only', () => {
    const counts = reactionCounts(
      aggregate({ byValue: { moved: 2, hopeful: 1, 'learning:agree': 5, grieving: 0 } })
    );
    expect(counts).toEqual([
      { type: 'moved', count: 2 },
      { type: 'hopeful', count: 1 },
    ]);
    expect(reactionCounts(null)).toEqual([]);
  });

  it('says how it landed in words, never a percentage', () => {
    expect(landedSummary(aggregate({ totalSignals: 0 }))).toBeNull();
    expect(
      landedSummary(aggregate({ totalSignals: 1, byValue: { moved: 1 }, uniqueParticipants: 1 }))
    ).toBe('1 reaction from 1 person');
    const many = landedSummary(
      aggregate({
        totalSignals: 4,
        byValue: { moved: 4 },
        uniqueParticipants: 3,
        consensusStrength: 0.8,
      })
    );
    expect(many).toBe('4 reactions from 3 people · consensus strong');
    expect(many).not.toMatch(/%/);
    expect(consensusWords(0.2)).toBe('divided');
  });

  it('excludes flags, reports and feedback from the reaction total', () => {
    expect(
      landedSummary(
        aggregate({
          totalSignals: 7,
          byValue: { moved: 2, flag: 3, report: 1, feedback: 1 },
          uniqueParticipants: 1,
        })
      )
    ).toBe('2 reactions from 1 person');
    expect(landedSummary(aggregate({ totalSignals: 2, byValue: { flag: 2 } }))).toBeNull();
  });

  it('fills every RecordSignalInputView field for a reaction from the session', () => {
    expect(
      reactionSignal(
        { type: 'moved', entityType: 'content', entityId: 'evolution-of-trust' },
        'human-jessica'
      )
    ).toEqual({
      entityType: 'content',
      entityId: 'evolution-of-trust',
      humanId: 'human-jessica',
      signalType: 'reaction',
      signalValue: 'moved',
      mechanismLevel: 1,
      proxyElohimId: null,
    });
  });

  it('records flag, feedback and report as signals of that kind at the context-menu level', () => {
    for (const kind of ['flag', 'feedback', 'report'] as const) {
      expect(concernSignal(kind, 'evolution-of-trust', 'Lovely.', 'human-jessica')).toEqual({
        entityType: 'content',
        entityId: 'evolution-of-trust',
        humanId: 'human-jessica',
        signalType: kind,
        signalValue: 'Lovely.',
        mechanismLevel: 0,
        proxyElohimId: null,
      });
    }
  });

  it('fills every FileChallengeInputView field, the words as evidence', () => {
    expect(challengeInput('evolution-of-trust', 'The date is wrong', 'human-jessica')).toEqual({
      entityType: 'content',
      entityId: 'evolution-of-trust',
      challengerId: 'human-jessica',
      standingBasis: 'commons-member',
      groundsPrimary: 'other',
      groundsSecondary: null,
      evidence: 'The date is wrong',
      requestedOutcome: null,
    });
  });

  it('routes all four gate kinds, each to a real write', () => {
    expect(ROUTED_CONCERN_KINDS).toEqual(['flag', 'challenge', 'feedback', 'report']);
  });
});
