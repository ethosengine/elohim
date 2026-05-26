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

export { DistributionService } from './distribution/distribution.service';
export { DistributionBadgeComponent } from './distribution/distribution-badge/distribution-badge.component';

export { ObservationService } from './services/observation.service';

export type { ElohimEnv } from './env/elohim-env';
export { ELOHIM_ENV } from './env/elohim-env';

export * from './angular/models/source-chain.model';

export { DoorwayClientService } from './angular/services/doorway-client.service';
export { LocalSourceChainService } from './angular/services/local-source-chain.service';
export type {
  VerifyBlobRequest,
  VerifyBlobResponse,
  CustodianInfo,
  BestCustodianResponse,
  StreamingVariant,
  StreamingManifest,
} from './angular/services/doorway-client.service';
