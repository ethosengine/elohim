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
