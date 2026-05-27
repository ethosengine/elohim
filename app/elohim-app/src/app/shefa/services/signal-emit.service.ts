/**
 * SignalEmitService — POST /api/v1/signal/emit (EPR Phase 2B Batch C, Task C.3).
 *
 * Implementation has moved to @elohim/rea-runtime. This file re-exports the
 * service and companion types so that existing @app/shefa consumers continue
 * to work without import-path changes during the cross-pillar cleanup sprint.
 *
 * Canonical source: app/elohim-library/projects/elohim-rea-runtime/src/lib/
 *   - signal-emit.service.ts  (SignalEmitService class)
 *   - signal-emit-types.ts    (SignalIntent, SignalEmitSuccessResponse, SignalEmitResult)
 *
 * Cross-pillar consumers (lamad, etc.) should import from @elohim/rea-runtime
 * directly rather than from @app/shefa.
 */

export { SignalEmitService } from '@elohim/rea-runtime';
export type {
  SignalIntent,
  SignalEmitSuccessResponse,
  SignalEmitResult,
} from '@elohim/rea-runtime';
