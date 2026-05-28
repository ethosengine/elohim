/**
 * Qahal Services - Community Services
 *
 * Affinity tracking, consent management, and governance services.
 */

export { AffinityTrackingService } from '@app/elohim/services/affinity-tracking.service';
export { HumanConsentService } from '@app/elohim/services/human-consent.service';
export { GovernanceService } from '@app/elohim/services/governance.service';
export { CollectiveService } from './collective.service';
// MechanismSelectionService migrated to @elohim/service in Slice 2.2b closure.
// Re-exported here for backward compatibility with qahal Angular components.
// SignalAccumulationService retired by M-POLICY-1 — use GovernanceApiService.getAccumulationStatus().
export { MechanismSelectionService } from '@elohim/service';
export type { MechanismSelection } from '@elohim/service';
export { GovernanceRecognitionService } from './governance-recognition.service';
export { BracketSynthesisService } from './bracket-synthesis.service';
