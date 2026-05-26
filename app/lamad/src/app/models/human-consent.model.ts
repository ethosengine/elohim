/**
 * Human Consent Model - Graduated intimacy and consent-based relationships.
 *
 * P-migrated from @app/elohim/models/human-consent.model (Slice 2.1c).
 * Moved here because all consumers were in lamad and the model only
 * imports from @elohim/storage-client (no elohim-app deps).
 *
 * This model governs human-to-human relationships in the Elohim Protocol,
 * implementing the "graduated intimacy" pattern where relationship depth
 * is explicitly consented to by both parties.
 */

// Import shared types from protocol-core (canonical source)
import {
  IntimacyLevel,
  ConsentState,
  type ConsentStateChange as BaseConsentStateChange,
} from '@elohim/storage-client';

// @coverage: 66.7% (2026-02-24)

// =========================================================================
// Human Consent Record
// =========================================================================

export interface HumanConsent {
  id: string;
  initiatorId: string;
  participantId: string;
  intimacyLevel: IntimacyLevel;
  consentState: ConsentState;
  createdAt: string;
  updatedAt: string;
  consentedAt?: string;
  expiresAt?: string;
  publicNote?: string;
  initiatorPrivateNote?: string;
  participantPrivateNote?: string;
  requestMessage?: string;
  responseMessage?: string;
  validatingAttestationIds?: string[];
  requiredAttestationType?: string;
  stateHistory: HumanConsentStateChange[];
  elevationAttempts?: number;
}

export interface HumanConsentStateChange extends BaseConsentStateChange {
  fromLevel?: IntimacyLevel;
  toLevel?: IntimacyLevel;
}

// =========================================================================
// Consent Request/Response DTOs
// =========================================================================

export interface ConsentRequest {
  participantId: string;
  requestedLevel: IntimacyLevel;
  message?: string;
  requiredAttestationType?: string;
}

export interface ConsentResponse {
  consentId: string;
  accept: boolean;
  message?: string;
}

export interface ElevationRequest {
  consentId: string;
  newLevel: IntimacyLevel;
  message?: string;
  requiredAttestationType?: string;
}

// =========================================================================
// Relationship Attestation Types
// =========================================================================

export const RELATIONSHIP_ATTESTATION_TYPES = {
  MARRIAGE: 'relationship:marriage',
  PARTNERSHIP: 'relationship:partnership',
  PARENT_CHILD: 'relationship:parent_child',
  SIBLING: 'relationship:sibling',
  CLOSE_FRIEND: 'relationship:close_friend',
  MENTORSHIP: 'relationship:mentorship',
  BUSINESS_PARTNER: 'relationship:business_partner',
  CUSTOM: 'relationship:custom',
} as const;

export type RelationshipAttestationType =
  (typeof RELATIONSHIP_ATTESTATION_TYPES)[keyof typeof RELATIONSHIP_ATTESTATION_TYPES];

// =========================================================================
// Utility Functions
// =========================================================================

export function requiresMutualAttestation(level: IntimacyLevel): boolean {
  return level === 'intimate';
}

export function canElevate(consent: HumanConsent): boolean {
  return (
    (consent.consentState === 'accepted' || consent.consentState === 'not_required') &&
    consent.intimacyLevel !== 'intimate'
  );
}

export {
  INTIMACY_LEVEL_VALUES,
  getNextIntimacyLevel,
  hasMinimumIntimacy,
  type IntimacyLevel,
  isConsentActive,
  type ConsentState,
} from '@elohim/storage-client';
