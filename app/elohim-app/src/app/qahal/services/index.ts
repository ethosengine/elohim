/**
 * Qahal Services - Community Services
 *
 * Affinity tracking, consent management, and governance services.
 */

export { AffinityTrackingService } from '@app/elohim/services/affinity-tracking.service';
export { HumanConsentService } from '@app/elohim/services/human-consent.service';
export { GovernanceService } from '@app/elohim/services/governance.service';
export { CollectiveService } from './collective.service';
// MechanismSelectionService + SignalAccumulationService migrated to
// @elohim/service in Slice 2.2b closure. Re-exported here for backward
// compatibility with qahal Angular components that consume them directly.
export {
  MechanismSelectionService,
  SignalAccumulationService,
} from '@elohim/service';
export type {
  MechanismSelection,
  AccumulationStatus,
} from '@elohim/service';
export { GovernanceRecognitionService } from './governance-recognition.service';
export { BracketSynthesisService } from './bracket-synthesis.service';
