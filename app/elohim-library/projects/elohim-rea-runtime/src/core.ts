/*
 * Framework-free core entrypoint of @elohim/rea-runtime — `@elohim/rea-runtime/core`.
 *
 * Canon: genesis/docs/architecture/elohim-sdk.md §4.1 (Node-Consumable Core
 * Entrypoints). This entry is framework-free ONLY — zero `@angular/*` anywhere
 * in its transitive import closure. Non-Angular consumers (Node tooling, a2o,
 * agents, future runtimes) import ONLY this subpath; the root entry
 * (`public-api.ts`) remains the Angular surface.
 *
 * The boundary is enforced, not aspirational: `core.boundary.spec.ts` walks
 * this file's transitive import closure and fails on any `@angular/*` import.
 * If you add a re-export here, that test is the gate.
 *
 * Pre-paved per the §4.1 born-with rule: `CommitmentService` lands here
 * (che-network-agency arc Phase 4) — it has the identical Node+browser
 * consumer set and arrives INTO this entry, not before it.
 *
 * Excluded (Angular-coupled — root entry only):
 *   event.service, attention-tracker.service, attention-tending-api.service,
 *     signal-emit.service, resource-explorer.service — `@angular/core`
 *     Injectable/inject (+ `@angular/common/http` where applicable)
 *   shefa-di-tokens — `@angular/core` InjectionToken declarations
 *   resource-explorer.model — imports `Observable` from rxjs; reactive-UI
 *     explorer vocabulary with no non-Angular consumer today, so it stays on
 *     the root entry rather than forcing rxjs onto Node consumers
 */

// REA action vocabulary — ValueFlows-aligned verbs + lamad event types
export * from './lib/rea-action-types.js';

// Protocol event types — the attention-to-attestation pipeline and its
// event→REA mappings (PROTOCOL_EVENT_MAPPINGS, PROTOCOL_UNITS)
export * from './lib/protocol-event-types.js';

// LamadEventIntent — intent wire shape for POST /api/v1/lamad/events
export * from './lib/lamad-event-intent.types.js';

// SignalIntent and companions — EPR write-through wire shapes for
// POST /api/v1/signal/emit
export * from './lib/signal-emit-types.js';
