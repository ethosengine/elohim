// src/app/elohim/models/protocol-event-types.model.ts
/**
 * Protocol Event Types -- The attention-to-attestation pipeline.
 *
 * These are the event types any app built on the Elohim Protocol would need.
 * The pipeline: view -> engage -> demonstrate -> attest -> capability
 * Every stage produces REA economic events. Every stage is subject to governance.
 *
 * Domain-specific event types (quiz-submit, claim-filed, governance-vote)
 * extend these from their respective pillar models.
 */

import type { REAAction, ResourceClassification } from './rea-bridge.model';

// =============================================================================
// Protocol Event Types
// =============================================================================

/**
 * ProtocolEventType -- Events that any app on the protocol produces.
 *
 * These follow the attention-to-attestation pipeline:
 *   view -> engage -> demonstrate -> attest -> capability
 *
 * Test: "Is this a STAGE of the pipeline, or an INSTRUMENT that implements a stage?"
 * Stages -> protocol. Instruments (quiz, simulation, peer review) -> domain.
 *
 * Learning is a universal capacity of the protocol, not a lamad feature.
 * All content is attestable. Assessment is the "demonstrate" stage -- protocol.
 * HOW you assess (Sophia quiz, portfolio, peer review) -- domain.
 */
export type ProtocolEventType =
  // Attention (content interaction -- "view" and "engage" stages)
  | 'content-view' // Agent viewed content (use + attention)
  | 'content-complete' // Agent completed content (produce + achievement)
  | 'session-start' // Agent began a session (use + attention)
  | 'session-end' // Agent ended a session (use + attention)

  // Demonstration ("demonstrate" stage -- assessment happened, not HOW)
  | 'assessment-start' // Agent began demonstrating understanding (use + attention)
  | 'assessment-complete' // Agent finished demonstrating understanding (produce + credential)

  // Attestation ("attest" and "capability" stages -- the pipeline's output)
  | 'attestation-grant' // Attestation granted (produce + attestation)
  | 'capability-earn' // Capability developed (produce + credential)

  // Recognition (value flow)
  | 'recognition-given' // Recognition given to contributor (appreciate + recognition)
  | 'recognition-received' // Recognition received (appreciate + recognition)
  | 'affinity-mark' // Agent marked affinity (appreciate + recognition)
  | 'endorsement' // Formal endorsement (appreciate + endorsement)
  | 'citation' // Content cited another (cite + recognition)

  // Stewardship (content governance)
  | 'stewardship-begin' // Steward began stewardship (work + stewardship)
  | 'presence-claim' // Contributor claimed presence (accept + recognition)
  | 'recognition-transfer' // Recognition transferred (transfer + recognition)
  | 'invitation-send' // Invitation sent (deliver-service)

  // Content lifecycle
  | 'content-create' // Content created (produce + content)
  | 'content-flag' // Content flagged (modify + content)
  | 'attestation-revoke' // Attestation revoked (modify + attestation)

  // Governance
  | 'governance-vote'; // Vote cast (work + governance)

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
  resourceType: ResourceClassification;
  defaultUnit: string;
}

export const PROTOCOL_EVENT_MAPPINGS: Record<ProtocolEventType, EventREAMapping> = {
  'content-view': { action: 'use', resourceType: 'attention', defaultUnit: PROTOCOL_UNITS.VIEW },
  'content-complete': {
    action: 'produce',
    resourceType: 'credential',
    defaultUnit: PROTOCOL_UNITS.EACH,
  },
  'session-start': {
    action: 'use',
    resourceType: 'attention',
    defaultUnit: PROTOCOL_UNITS.SESSION,
  },
  'session-end': { action: 'use', resourceType: 'attention', defaultUnit: PROTOCOL_UNITS.MINUTE },
  'assessment-start': {
    action: 'use',
    resourceType: 'attention',
    defaultUnit: PROTOCOL_UNITS.EACH,
  },
  'assessment-complete': {
    action: 'produce',
    resourceType: 'credential',
    defaultUnit: PROTOCOL_UNITS.EACH,
  },
  'attestation-grant': {
    action: 'produce',
    resourceType: 'credential',
    defaultUnit: PROTOCOL_UNITS.ATTESTATION,
  },
  'capability-earn': {
    action: 'produce',
    resourceType: 'credential',
    defaultUnit: PROTOCOL_UNITS.EACH,
  },
  'recognition-given': {
    action: 'raise',
    resourceType: 'recognition',
    defaultUnit: PROTOCOL_UNITS.EACH,
  },
  'recognition-received': {
    action: 'raise',
    resourceType: 'recognition',
    defaultUnit: PROTOCOL_UNITS.EACH,
  },
  'affinity-mark': {
    action: 'raise',
    resourceType: 'recognition',
    defaultUnit: PROTOCOL_UNITS.AFFINITY,
  },
  endorsement: {
    action: 'raise',
    resourceType: 'recognition',
    defaultUnit: PROTOCOL_UNITS.ENDORSEMENT,
  },
  citation: { action: 'cite', resourceType: 'recognition', defaultUnit: PROTOCOL_UNITS.EACH },
  'stewardship-begin': {
    action: 'work',
    resourceType: 'stewardship',
    defaultUnit: PROTOCOL_UNITS.EACH,
  },
  'presence-claim': {
    action: 'accept',
    resourceType: 'recognition',
    defaultUnit: PROTOCOL_UNITS.AFFINITY,
  },
  'recognition-transfer': {
    action: 'transfer',
    resourceType: 'recognition',
    defaultUnit: PROTOCOL_UNITS.AFFINITY,
  },
  'invitation-send': {
    action: 'deliver-service' as REAAction,
    resourceType: 'stewardship',
    defaultUnit: PROTOCOL_UNITS.EACH,
  },
  'content-create': {
    action: 'produce',
    resourceType: 'content',
    defaultUnit: PROTOCOL_UNITS.NODE,
  },
  'content-flag': { action: 'modify', resourceType: 'content', defaultUnit: PROTOCOL_UNITS.NODE },
  'attestation-revoke': {
    action: 'modify',
    resourceType: 'credential',
    defaultUnit: PROTOCOL_UNITS.ATTESTATION,
  },
  'governance-vote': {
    action: 'work',
    resourceType: 'membership',
    defaultUnit: PROTOCOL_UNITS.EACH,
  },
};
