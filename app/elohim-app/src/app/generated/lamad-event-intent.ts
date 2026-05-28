/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: intents/lamad-event-intent.schema.json -- DO NOT EDIT */

/**
 * Source of truth: request body for coordinator-mediated EconomicEvent creation (Category A, existing). The coordinator composes the full EconomicEvent (provider/receiver/action) from the discriminated union using substrate-known PROTOCOL_EVENT_MAPPINGS. This is an intent wire shape, not an entity. The persisted source of truth is the EconomicEvent entry in the elohim DNA content_store zome, projected to the economic_events table in elohim-storage.
 */
export interface LamadEventIntent {
  /**
   * Agent public key of the initiating agent (provider for most event types)
   */
  agentId: string;
  /**
   * Lamad event type discriminant — determines the REA action, provider, and receiver via PROTOCOL_EVENT_MAPPINGS
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
  /**
   * Content node this event relates to (required for content-* and assessment-* types)
   */
  contentId?: string | null;
  /**
   * Learning path this event relates to (required for path-* types)
   */
  pathId?: string | null;
  /**
   * Contributor presence this event relates to (required for recognition-given)
   */
  contributorPresenceId?: string | null;
  /**
   * Quantity of resource affected (e.g. amount of recognition)
   */
  resourceQuantityValue?: number | null;
  /**
   * Unit of resource quantity (e.g. 'recognition', 'unit-view')
   */
  resourceQuantityUnit?: string | null;
  /**
   * Domain-specific metadata payload (e.g. assessmentId, quizId, score, stepId)
   */
  metadata?: Record<string, unknown> | null;
  /**
   * Human-readable note about this event
   */
  note?: string | null;
}
