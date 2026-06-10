/*
 * Framework-free core entrypoint of @elohim/identity — `@elohim/identity/core`.
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
 * Excluded (Angular-coupled — root entry only):
 *   session-human.service — `@angular/core` Injectable + rxjs
 */

// Session identity primitives (models only — the service is Angular-coupled)
export * from './lib/content-access.model';
export * from './lib/session-human.model';

// Attestation primitives
export * from './lib/attestations.model';

// Doorway session client — framework-free auth surface client
export * from './lib/doorway-session-client';
