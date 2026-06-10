/*
 * Public API Surface of @elohim/identity
 *
 * Identity SDK surface — session management, identity primitives,
 * profile models, attestation models, and route guards.
 *
 * Aligned with cradle-to-grave-capability-gradient §4 elohim mediation roles.
 *
 * Migration status: Slice 2.3 of cross-pillar import cleanup sprint.
 * Modules land here as they are cleanly liftable; modules with
 * unresolved L-slice (Slice 2.1) or out-of-scope imagodei deps are
 * noted as PENDING below.
 *
 * PENDING — blocked by Slice 2.1 (L-slice) deps not yet landed:
 *   identity.model    — depends on agency.model (out-of-scope imagodei)
 *   profile.model     — depends on @app/elohim/models/{agent,json-ld,open-graph}
 *   profile.service   — depends on L-slice services + @app/lamad/services
 *   identity.service  — depends on out-of-scope imagodei services + L-slice
 *   identity.guard    — depends on identity.service + auth.service (out-of-scope)
 *
 * Modules will be uncommented as each dependency slice lands.
 */

// ============================================================================
// Session identity primitives — LANDED (session-human.model, session-human.service)
// ============================================================================
export * from './lib/content-access.model';
export * from './lib/session-human.model';
export * from './lib/session-human.service';

// ============================================================================
// Attestation primitives — LANDED
// ============================================================================
export * from './lib/attestations.model';

// ============================================================================
// Doorway session client — LANDED (arc plan 2026-06-10 Phase 1 Task 1.1)
// Framework-free auth surface client; consolidates hand-rolled auth walking
// (elohim-app AuthService / a2o DoorwayClient / a2o BrowserDevice).
// ============================================================================
export * from './lib/doorway-session-client';
