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
// GovernanceRecognitionService retired by M-REA-3 — use GovernanceApiService.postParticipation().
// The substrate reads recognition_weight_by_level from the standing-policy Manifest and
// composes the EconomicEvent (action: work, provider: humanId, receiver: entityId).
// Route: POST /api/v1/mishpat/recognition/participation
// The client-side WEIGHT_BY_LEVEL constant (F-POLICY-3) is retired with it.
// Callers: reaction-bar.component.ts + graduated-feedback.component.ts (both updated in M-REA-3).
export { BracketSynthesisService } from './bracket-synthesis.service';
