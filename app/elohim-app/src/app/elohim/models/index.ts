/**
 * Elohim Models - Protocol Core Types
 *
 * Cross-pillar primitives shared across all four pillars:
 * - Imago Dei (Identity)
 * - Lamad (Content/Learning)
 * - Qahal (Community)
 * - Shefa (Economy)
 */

// Protocol Core - Shared primitives
export * from './protocol-core.model';

// REA Bridge - hREA/ValueFlows economic coordination
export * from './rea-bridge.model';

// Economic Event - Immutable value flow records
export * from './economic-event.model';

// Contributor Presence - Stewardship lifecycle for absent contributors
export * from './contributor-presence.model';

// Agent types
export * from './agent.model';
export * from './elohim-agent.model';

// Trust system
export * from './trust-badge.model';

// Human consent and relationships
export * from './human-consent.model';

// Banner notification system
export * from './banner-notice.model';

// Elohim presence
export * from './elohim-presence.model';

// Create context (reach negotiation, birth context)
export * from './create-context.model';

// Coordination Envelope - Distributed coordination verbs
export * from './coordination-envelope.model';

// EPR Head (IPLD-compatible content metadata)
export * from './epr-head.model';

// Infrastructure
export * from './source-chain.model';
export * from './json-ld.model';
export * from './open-graph.model';
export * from './verifiable-credential.model';

// Gated Response - Gate evaluation extraction for mutation responses
export { isGatedResponse, extractGateFromResponse } from './gated-response.model';
export type { GatedResponse } from './gated-response.model';

// Zome Wire Types - Centralized Holochain zome response types
export * from './zome-wire-types';
