/**
 * Protocol Event Types — The attention-to-attestation pipeline.
 *
 * Inlined from @app/elohim/models/protocol-event-types.model as part of
 * Wave 2 Slice 2.4 of the cross-pillar import cleanup sprint.
 *
 * These are the event types any app built on the Elohim Protocol would need.
 * The pipeline: view -> engage -> demonstrate -> attest -> capability.
 * Every stage produces REA economic events.
 *
 * Design note: These types belong in @elohim/rea-runtime because they are
 * REA-protocol vocabulary, not Angular-app vocabulary. The originals in
 * @app/elohim/models will be updated to re-export from here in Wave 3.
 */

import type { REAAction } from './rea-action-types';

// =============================================================================
// Protocol Event Types
// =============================================================================

/**
 * ProtocolEventType — Events that any app on the protocol produces.
 *
 * These follow the attention-to-attestation pipeline:
 *   view -> engage -> demonstrate -> attest -> capability
 */
export type ProtocolEventType =
  // Attention (content interaction — "view" and "engage" stages)
  | 'content-view'
  | 'content-complete'
  | 'session-start'
  | 'session-end'

  // Demonstration ("demonstrate" stage)
  | 'assessment-start'
  | 'assessment-complete'

  // Attestation ("attest" and "capability" stages)
  | 'attestation-grant'
  | 'capability-earn'

  // Recognition (value flow)
  | 'recognition-given'
  | 'recognition-received'
  | 'affinity-mark'
  | 'endorsement'
  | 'citation'

  // Stewardship (content governance)
  | 'stewardship-begin'
  | 'presence-claim'
  | 'recognition-transfer'
  | 'invitation-send'

  // Content lifecycle
  | 'content-create'
  | 'content-flag'
  | 'attestation-revoke'

  // Governance
  | 'governance-vote';

// =============================================================================
// Protocol Event Constants
// =============================================================================

export const ProtocolEventTypes = {
  CONTENT_VIEW: 'content-view' as ProtocolEventType,
  CONTENT_COMPLETE: 'content-complete' as ProtocolEventType,
  SESSION_START: 'session-start' as ProtocolEventType,
  SESSION_END: 'session-end' as ProtocolEventType,
  ASSESSMENT_START: 'assessment-start' as ProtocolEventType,
  ASSESSMENT_COMPLETE: 'assessment-complete' as ProtocolEventType,
  ATTESTATION_GRANT: 'attestation-grant' as ProtocolEventType,
  CAPABILITY_EARN: 'capability-earn' as ProtocolEventType,
  RECOGNITION_GIVEN: 'recognition-given' as ProtocolEventType,
  RECOGNITION_RECEIVED: 'recognition-received' as ProtocolEventType,
  AFFINITY_MARK: 'affinity-mark' as ProtocolEventType,
  ENDORSEMENT: 'endorsement' as ProtocolEventType,
  CITATION: 'citation' as ProtocolEventType,
  STEWARDSHIP_BEGIN: 'stewardship-begin' as ProtocolEventType,
  PRESENCE_CLAIM: 'presence-claim' as ProtocolEventType,
  RECOGNITION_TRANSFER: 'recognition-transfer' as ProtocolEventType,
  INVITATION_SEND: 'invitation-send' as ProtocolEventType,
  CONTENT_CREATE: 'content-create' as ProtocolEventType,
  CONTENT_FLAG: 'content-flag' as ProtocolEventType,
  ATTESTATION_REVOKE: 'attestation-revoke' as ProtocolEventType,
  GOVERNANCE_VOTE: 'governance-vote' as ProtocolEventType,
} as const;

// =============================================================================
// Standard Units (protocol-level)
// =============================================================================

export const PROTOCOL_UNITS = {
  EACH: 'unit-each',
  VIEW: 'unit-view',
  SESSION: 'unit-session',
  MINUTE: 'unit-minute',
  AFFINITY: 'unit-affinity',
  ENDORSEMENT: 'unit-endorsement',
  ATTESTATION: 'unit-attestation',
  NODE: 'unit-node',
  TOKEN: 'unit-token',
} as const;

// =============================================================================
// Protocol Event REA Mappings
// =============================================================================

export interface EventREAMapping {
  action: REAAction;
  resourceType: string;
  defaultUnit: string;
}

export const PROTOCOL_EVENT_MAPPINGS: Record<ProtocolEventType, EventREAMapping> = {
  'content-view': { action: 'use', resourceType: 'attention', defaultUnit: PROTOCOL_UNITS.VIEW },
  'content-complete': { action: 'produce', resourceType: 'credential', defaultUnit: PROTOCOL_UNITS.EACH },
  'session-start': { action: 'use', resourceType: 'attention', defaultUnit: PROTOCOL_UNITS.SESSION },
  'session-end': { action: 'use', resourceType: 'attention', defaultUnit: PROTOCOL_UNITS.MINUTE },
  'assessment-start': { action: 'use', resourceType: 'attention', defaultUnit: PROTOCOL_UNITS.EACH },
  'assessment-complete': { action: 'produce', resourceType: 'credential', defaultUnit: PROTOCOL_UNITS.EACH },
  'attestation-grant': { action: 'produce', resourceType: 'credential', defaultUnit: PROTOCOL_UNITS.ATTESTATION },
  'capability-earn': { action: 'produce', resourceType: 'credential', defaultUnit: PROTOCOL_UNITS.EACH },
  'recognition-given': { action: 'raise', resourceType: 'recognition', defaultUnit: PROTOCOL_UNITS.EACH },
  'recognition-received': { action: 'raise', resourceType: 'recognition', defaultUnit: PROTOCOL_UNITS.EACH },
  'affinity-mark': { action: 'raise', resourceType: 'recognition', defaultUnit: PROTOCOL_UNITS.AFFINITY },
  'endorsement': { action: 'raise', resourceType: 'recognition', defaultUnit: PROTOCOL_UNITS.ENDORSEMENT },
  'citation': { action: 'cite', resourceType: 'recognition', defaultUnit: PROTOCOL_UNITS.EACH },
  'stewardship-begin': { action: 'work', resourceType: 'stewardship', defaultUnit: PROTOCOL_UNITS.EACH },
  'presence-claim': { action: 'accept', resourceType: 'recognition', defaultUnit: PROTOCOL_UNITS.AFFINITY },
  'recognition-transfer': { action: 'transfer', resourceType: 'recognition', defaultUnit: PROTOCOL_UNITS.AFFINITY },
  'invitation-send': { action: 'deliver-service', resourceType: 'stewardship', defaultUnit: PROTOCOL_UNITS.EACH },
  'content-create': { action: 'produce', resourceType: 'content', defaultUnit: PROTOCOL_UNITS.NODE },
  'content-flag': { action: 'modify', resourceType: 'content', defaultUnit: PROTOCOL_UNITS.NODE },
  'attestation-revoke': { action: 'modify', resourceType: 'credential', defaultUnit: PROTOCOL_UNITS.ATTESTATION },
  'governance-vote': { action: 'work', resourceType: 'membership', defaultUnit: PROTOCOL_UNITS.EACH },
};
