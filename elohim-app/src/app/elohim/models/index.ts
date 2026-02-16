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

// Infrastructure
export * from './source-chain.model';
export * from './json-ld.model';
export * from './open-graph.model';
export * from './verifiable-credential.model';
