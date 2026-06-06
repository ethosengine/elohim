/*
 * Public API Surface for elohim-library
 *
 * Protocol-vocabulary widgets and supporting services used across
 * elohim-app and any future shell. Consumers import from
 * '@elohim/service/public-api'.
 *
 * Dimensions:
 *   - Resilience snapshot — collective-grain "is this safe / who is
 *     committed" reading. Backed by ResilienceService.
 *   - Distribution badge — replica/peer-grain "where is this and how
 *     widely spread" reading. Backed by DistributionService (lazy
 *     deep tier).
 */

export { ResilienceService } from './resilience/resilience.service';
export { ResilienceSnapshotComponent } from './resilience/resilience-snapshot/resilience-snapshot.component';
export type { ResilienceSnapshotDensity } from './resilience/resilience-snapshot/resilience-snapshot.types';
/** Re-exported so hosts can type the resilience hypercard `actions` input without reaching into elohim-core. */
export type { ContextMenuItem } from 'elohim-core';

export { DistributionService } from './distribution/distribution.service';
export { DistributionBadgeComponent } from './distribution/distribution-badge/distribution-badge.component';

export { ObservationService } from './services/observation.service';

export type { ElohimEnv } from './env/elohim-env';
export { ELOHIM_ENV } from './env/elohim-env';

export * from './angular/models/source-chain.model';

export { DoorwayClientService } from './angular/services/doorway-client.service';
/** @deprecated M-AGGR-2: migrate read paths to HolochainSourceChainService. Write paths migrate in M-REA-1 + M-AGGR-1. */
export { LocalSourceChainService } from './angular/services/local-source-chain.service';
// M-AGGR-2: Holochain source-chain cutover — thin HTTP read wrapper.
export { HolochainSourceChainService } from './angular/services/holochain-source-chain.service';
export type {
  VerifyBlobRequest,
  VerifyBlobResponse,
  CustodianInfo,
  BestCustodianResponse,
  StreamingVariant,
  StreamingManifest,
} from './angular/services/doorway-client.service';

// MechanismSelectionService retired by M-POLICY-2: use GovernanceApiService.getMechanismSelection().
// The substrate computes mechanism selection from GovernanceState × Content.contentType ×
// Proposal? × Manifest{kind:"pillar-projection", pillar:"qahal"} payload_json.
// MechanismSelectionView (from @elohim/storage-client/generated) is the wire type.
// SignalAccumulationService retired by M-POLICY-1: use GovernanceApiService.getAccumulationStatus().
