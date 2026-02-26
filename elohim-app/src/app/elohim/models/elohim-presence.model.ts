/**
 * Elohim Presence Model - Types for the elohim presence system.
 *
 * When and where the elohim appears in the learner experience.
 * Each presence moment carries constitutional reasoning for transparency.
 */

import type {
  ElohimCapability,
  ConstitutionalReasoning,
  ElohimComputationCost,
} from './elohim-agent.model';

/**
 * What kind of presence moment this is.
 *
 * Maps to the three elohim modes from the manifesto:
 * - nudge/insight/recommendation → "nudge" mode
 * - care → "resolve" mode
 * - celebration → "play" mode
 */
export type ElohimPresenceType = 'nudge' | 'insight' | 'recommendation' | 'care' | 'celebration';

/**
 * A moment when the elohim makes its presence known.
 *
 * This is what the UI renders — the visible manifestation of
 * an elohim's constitutional reasoning about a learner moment.
 */
export interface ElohimPresenceMoment {
  /** Unique moment ID */
  id: string;

  /** Kind of presence */
  type: ElohimPresenceType;

  /** Which capability produced this moment */
  capability: ElohimCapability;

  /** Human-readable summary */
  message: string;

  /** Constitutional reasoning — always transparent */
  reasoning: ConstitutionalReasoning;

  /** Optional actions the learner can take */
  actions?: PresenceAction[];

  /** Computation cost for transparency */
  cost?: ElohimComputationCost;

  /** Whether the learner can dismiss this moment */
  dismissible: boolean;

  /** When this moment expires (ISO string) */
  expiresAt?: string;

  /** When this moment was created (ISO string) */
  createdAt: string;

  /** Optional context data for action handlers (e.g., recommended pathId) */
  metadata?: Record<string, unknown>;
}

/**
 * An action the learner can take in response to a presence moment.
 */
export interface PresenceAction {
  /** Unique action ID */
  id: string;

  /** Display label */
  label: string;

  /** Optional icon */
  icon?: string;

  /** Whether this is the primary/recommended action */
  primary?: boolean;
}
