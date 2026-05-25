/**
 * REA Action Types — split from @app/elohim/models per manifest operator-input #3.
 *
 * REAAction and LamadEventType are the two symbols from the @app/elohim/models
 * bare-barrel that belong in @elohim/rea-runtime rather than @elohim/service.
 * See Wave 1 disposition manifest, R slice, operator-input #3.
 *
 * Source of canonical definitions:
 * - REAAction: app/elohim-app/src/app/elohim/models/rea-bridge.model.ts
 * - LamadEventType: app/elohim-app/src/app/elohim/models/economic-event.model.ts
 *
 * These are re-exported here so @elohim/rea-runtime consumers get the types
 * without reaching into @app/elohim/models. The originals remain in elohim-app
 * for elohim-app's own consumers; full retirement is deferred to Wave 3.
 */

/**
 * REAAction - The verb of an economic event.
 *
 * From ValueFlows specification, fully aligned with hREA action vocabulary.
 * Each action describes what happened to resources.
 *
 * See: https://www.valueflo.ws/concepts/actions/
 */
export type REAAction =
  // Input actions (consume/use resources)
  | 'use' // Use without consuming (view content, attend session)
  | 'consume' // Use up completely (one-time access tokens)
  | 'cite' // Reference another's work (creates recognition flow)

  // Output actions (create/produce resources)
  | 'produce' // Create new resource (author content, synthesize map)
  | 'raise' // Increase quantity (accumulate recognition)
  | 'lower' // Decrease quantity (reduce holdings)

  // Transfer actions (move between agents)
  | 'transfer' // Move resource to another agent (ownership transfer)
  | 'transfer-custody' // Move custody without ownership change
  | 'transfer-all-rights' // Transfer all rights to resource
  | 'move' // Move resource between locations (same agent)

  // Modification actions
  | 'modify' // Change resource properties
  | 'combine' // Merge resources (path extensions into base path)
  | 'separate' // Split resource (fork a learning path)

  // Work actions
  | 'work' // Contribute labor (stewardship, review, curation)
  | 'deliver-service' // Provide service (Elohim synthesis, tutoring)

  // Logistics actions (for physical resource handling)
  | 'pickup' // Take custody of a resource at a location
  | 'dropoff' // Release custody of a resource at a location

  // Exchange actions
  | 'give' // Give a resource (one side of exchange)
  | 'take' // Take a resource (other side of exchange)

  // Acceptance actions
  | 'accept'; // Accept a transfer or commitment (claim presence)

/**
 * LamadEventType - Classification of events in Lamad context.
 *
 * These map to specific action + resource combinations.
 */
export type LamadEventType =
  // Attention Events
  | 'content-view'
  | 'content-complete'
  | 'path-step-complete'
  | 'session-start'
  | 'session-end'

  // Assessment Events
  | 'assessment-start'
  | 'assessment-complete'
  | 'practice-attempt'
  | 'quiz-submit'

  // Recognition Events
  | 'affinity-mark'
  | 'endorsement'
  | 'citation'
  | 'recognition-given'
  | 'recognition-received'

  // Achievement Events
  | 'path-complete'
  | 'attestation-grant'
  | 'capability-earn'

  // Creation Events
  | 'content-create'
  | 'path-create'
  | 'extension-create'

  // Synthesis Events
  | 'map-synthesis'
  | 'analysis-complete'

  // Stewardship Events
  | 'stewardship-begin'
  | 'invitation-send'
  | 'presence-claim'
  | 'recognition-transfer'

  // Governance Events
  | 'attestation-revoke'
  | 'content-flag'
  | 'governance-vote'

  // Currency Events (Unyt integration)
  | 'credit-issue'
  | 'credit-transfer'
  | 'credit-retire'

  // Insurance Mutual Events (Elohim Mutual)
  | 'premium-payment'
  | 'claim-filed'
  | 'claim-evidence-submitted'
  | 'claim-investigated'
  | 'claim-adjusted'
  | 'claim-settled'
  | 'claim-denied'
  | 'claim-appealed'
  | 'risk-reduction-verified'
  | 'preventive-care-completed'
  | 'prevention-incentive-awarded'
  | 'coverage-decision'
  | 'claim-review-initiated'
  | 'reserve-adjustment'

  // Consent & Data Economy Events
  | 'consent-grant'
  | 'consent-withdraw'
  | 'data-access'
  | 'data-value-transfer'
  | 'consent-audit-complete'
  | 'research-response-submit';

/**
 * REAActions - Named constant map for hREA action types used in EventService.
 */
export const REAActions = {
  USE: 'use',
  PRODUCE: 'produce',
  TRANSFER: 'transfer',
  CITE: 'cite',
  APPRECIATE: 'appreciate',
} as const;

/**
 * LamadEventTypes - Named constant map for Lamad event types.
 */
export const LamadEventTypes = {
  CONTENT_VIEW: 'content-view' as LamadEventType,
  CONTENT_COMPLETE: 'content-complete' as LamadEventType,
  PATH_STEP_COMPLETE: 'path-step-complete' as LamadEventType,
  PATH_COMPLETE: 'path-complete' as LamadEventType,
  ASSESSMENT_START: 'assessment-start' as LamadEventType,
  ASSESSMENT_COMPLETE: 'assessment-complete' as LamadEventType,
  PRACTICE_ATTEMPT: 'practice-attempt' as LamadEventType,
  QUIZ_SUBMIT: 'quiz-submit' as LamadEventType,
  RECOGNITION_GIVEN: 'recognition-given' as LamadEventType,
  RECOGNITION_RECEIVED: 'recognition-received' as LamadEventType,
};
