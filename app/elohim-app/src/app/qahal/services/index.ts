/**
 * Qahal Services - Community Services
 *
 * Affinity tracking, consent management, and governance services.
 */

export { AffinityTrackingService } from '@app/elohim/services/affinity-tracking.service';
export { HumanConsentService } from '@app/elohim/services/human-consent.service';
export { GovernanceService } from '@app/elohim/services/governance.service';
export { CollectiveService } from './collective.service';
// MechanismSelectionService retired by M-POLICY-2 — use GovernanceApiService.getMechanismSelection().
// The substrate now computes level/mechanism/renderTarget from GovernanceState × Content.contentType ×
// Proposal? × pillar-projection Manifest. MechanismSelectionView is the wire type.
// SignalAccumulationService retired by M-POLICY-1 — use GovernanceApiService.getAccumulationStatus().
export { GovernanceRecognitionService } from './governance-recognition.service';
export { BracketSynthesisService } from './bracket-synthesis.service';
