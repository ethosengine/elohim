/*
 * Public API Surface for Angular resilience UI (elohim-library)
 *
 * ResilienceService and ResilienceSnapshotComponent live in the root src/
 * of elohim-library where the Angular compiler + Vitest setup (analogjs) can
 * reach them.  This barrel re-exports them so consumers can import from
 * '@elohim/service/public-api' or via path alias if configured.
 */

export { ResilienceService } from '../../../src/resilience/resilience.service';
export { ResilienceSnapshotComponent } from '../../../src/resilience/resilience-snapshot/resilience-snapshot.component';
export { ResilienceSnapshotDensity } from '../../../src/resilience/resilience-snapshot/resilience-snapshot.types';
