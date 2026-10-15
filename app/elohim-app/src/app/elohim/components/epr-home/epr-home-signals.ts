/**
 * Pure adapters between the governance wire views and the Lit elements the
 * atom home composes (spec 2026-09-02 §11.2, "How it landed" / "Raise a
 * concern"). No domain arithmetic: counts are read from the aggregate the
 * storage peer computed, and every write field is sourced from the session or
 * a declared vocabulary — never invented.
 */

import type {
  FileChallengeInputView,
  RecordSignalInputView,
  SignalAggregateView,
} from '@elohim/storage-client/generated';
import type {
  EmotionalReactionType,
  GateFeedbackType,
  ReactionCount,
  ReactionSubmitEvent,
} from 'elohim-core';

/** The reaction bar's own vocabulary (elohim-core `EmotionalReactionType`). */
export const REACTION_TYPES: readonly EmotionalReactionType[] = [
  'moved',
  'grateful',
  'inspired',
  'hopeful',
  'grieving',
  'challenged',
  'concerned',
  'uncomfortable',
];

/**
 * Mechanism ladder levels, as declared in
 * elohim/sdk/schemas/v1/views/mechanism-selection.schema.json (`level`):
 * 0 = context menu only, 1 = reactions.
 */
export const CONTEXT_MENU_MECHANISM_LEVEL = 0;
export const REACTION_MECHANISM_LEVEL = 1;

/**
 * Every gate-feedback kind has a real write: `challenge` files a challenge;
 * `flag`, `feedback` and `report` record a governance signal of that type.
 */
export const ROUTED_CONCERN_KINDS: readonly GateFeedbackType[] = [
  'flag',
  'challenge',
  'feedback',
  'report',
];

/** The kinds recorded as a governance signal (everything but a challenge). */
export type SignalConcernKind = Exclude<GateFeedbackType, 'challenge'>;

/** bigint-typed wire fields arrive as JSON numbers; read either honestly. */
export function wireCount(v: bigint | number | undefined | null): number {
  if (v === undefined || v === null) return 0;
  const n = Number(v);
  return Number.isFinite(n) ? n : 0;
}

/**
 * Reaction counts for `<elohim-reaction-bar>.counts`. The aggregate carries
 * two marginal dictionaries (byType, byValue), not a type×value table; a
 * reaction's value IS its reaction word, so the per-reaction count is read from
 * `byValue` restricted to the reaction vocabulary.
 */
export function reactionCounts(agg: SignalAggregateView | null | undefined): ReactionCount[] {
  if (!agg) return [];
  const out: ReactionCount[] = [];
  for (const type of REACTION_TYPES) {
    const count = wireCount(agg.byValue[type]);
    if (count > 0) out.push({ type, count });
  }
  return out;
}

/** Consensus strength (0..1, computed by the peer) in words — never a percentage. */
export function consensusWords(strength: number): string {
  if (strength >= 0.75) return 'strong';
  if (strength >= 0.5) return 'leaning one way';
  return 'divided';
}

/** "4 reactions from 3 people · consensus strong"; null when nobody has reacted. */
export function landedSummary(agg: SignalAggregateView | null | undefined): string | null {
  if (!agg) return null;
  const total = wireCount(agg.totalSignals);
  if (total === 0) return null;
  const people = wireCount(agg.uniqueParticipants);
  const reactions = total === 1 ? '1 reaction' : `${total} reactions`;
  const from = people === 1 ? 'from 1 person' : `from ${people} people`;
  const consensus = people > 1 ? ` · consensus ${consensusWords(agg.consensusStrength)}` : '';
  return `${reactions} ${from}${consensus}`;
}

/** A reaction-bar submission as the governance signal it records. */
export function reactionSignal(event: ReactionSubmitEvent, humanId: string): RecordSignalInputView {
  return {
    entityType: event.entityType,
    entityId: event.entityId,
    humanId,
    signalType: 'reaction',
    signalValue: event.type,
    mechanismLevel: REACTION_MECHANISM_LEVEL,
    proxyElohimId: null,
  };
}

/**
 * A flag, feedback or report raised through the gate's context menu: a
 * governance signal whose type is the kind and whose value is the person's
 * words, at the context-menu ladder level (0).
 */
export function concernSignal(
  kind: SignalConcernKind,
  contentId: string,
  text: string,
  humanId: string
): RecordSignalInputView {
  return {
    entityType: 'content',
    entityId: contentId,
    humanId,
    signalType: kind,
    signalValue: text,
    mechanismLevel: CONTEXT_MENU_MECHANISM_LEVEL,
    proxyElohimId: null,
  };
}

/**
 * A challenge raised through the gate. The trigger offers no grounds picker,
 * so the grounds are `other` and the person's own words are the evidence; no
 * outcome is requested on their behalf.
 */
export function challengeInput(
  contentId: string,
  text: string,
  humanId: string
): FileChallengeInputView {
  return {
    entityType: 'content',
    entityId: contentId,
    challengerId: humanId,
    standingBasis: 'commons-member',
    groundsPrimary: 'other',
    groundsSecondary: null,
    evidence: text,
    requestedOutcome: null,
  };
}
