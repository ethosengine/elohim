/**
 * LamadEventIntent — high-level intent wire shape for POST /api/v1/lamad/events.
 *
 * Source of truth: elohim/sdk/schemas/v1/intents/lamad-event-intent.schema.json
 * Canonical generated file: app/elohim-library/projects/elohim-service/src/generated/lamad-event-intent.ts
 *
 * This file re-declares the interface locally so @elohim/rea-runtime can import
 * it without a cross-project dep on @elohim/service (which has no tsconfig path
 * alias yet). Both definitions MUST stay in sync with the schema.
 *
 * TODO(post-M-REA-1): once @elohim/service gains a tsconfig path alias, remove
 * this file and import from '@elohim/service' directly.
 */

/**
 * High-level intent discriminated union for POST /api/v1/lamad/events.
 *
 * The substrate (elohim-storage) composes the full EconomicEvent
 * (action, provider, receiver) from this intent using PROTOCOL_EVENT_MAPPINGS.
 * No client-side REA composition required.
 */
export interface LamadEventIntent {
  /** Agent public key of the initiating agent (provider for most event types) */
  agentId: string;
  /**
   * Lamad event type discriminant — determines the REA action, provider, and
   * receiver via PROTOCOL_EVENT_MAPPINGS.
   */
  lamadEventType:
    | 'content-view'
    | 'content-complete'
    | 'path-step-complete'
    | 'path-complete'
    | 'session-start'
    | 'session-end'
    | 'assessment-start'
    | 'assessment-complete'
    | 'practice-attempt'
    | 'quiz-submit'
    | 'affinity-mark'
    | 'endorsement'
    | 'citation'
    | 'recognition-given'
    | 'recognition-received'
    | 'attestation-grant'
    | 'capability-earn'
    | 'content-create'
    | 'path-create'
    | 'extension-create'
    | 'map-synthesis'
    | 'analysis-complete'
    | 'stewardship-begin'
    | 'invitation-send'
    | 'presence-claim'
    | 'recognition-transfer'
    | 'attestation-revoke'
    | 'content-flag'
    | 'governance-vote';
  /** Content node this event relates to (required for content-* and assessment-* types) */
  contentId?: string | null;
  /** Learning path this event relates to (required for path-* types) */
  pathId?: string | null;
  /** Contributor presence this event relates to (required for recognition-given) */
  contributorPresenceId?: string | null;
  /** Quantity of resource affected (e.g. amount of recognition) */
  resourceQuantityValue?: number | null;
  /** Unit of resource quantity (e.g. 'recognition', 'unit-view') */
  resourceQuantityUnit?: string | null;
  /** Domain-specific metadata payload (e.g. assessmentId, quizId, score, stepId) */
  metadata?: Record<string, unknown> | null;
  /** Human-readable note about this event */
  note?: string | null;
}
