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

// Protocol event types — the attention-to-attestation pipeline primitives
export * from './protocol-event-types.model';

// Economic Event - Immutable value flow records
export * from './economic-event.model';

// Contributor Presence - Stewardship lifecycle for absent contributors
export * from './contributor-presence.model';

// Agent types (migrated to @elohim/service — Slice 2.1)
export type {
  Agent,
  AgentProgress,
  AgentAttestation,
  NewAttestation,
  AttestationCategory,
  FrontierItem,
  MasteryTier,
} from '@elohim/service/angular/models/agent.model';
export {
  getMasteryTier,
  getMasteryProgress,
  isAboveGate,
  compareMasteryLevels,
  MASTERY_LEVEL_VALUES,
  BLOOM_LEVEL_VALUES,
  ATTESTATION_GATE_LEVEL,
} from '@elohim/service/angular/models/agent.model';
export * from './elohim-agent.model';

// Trust system
export * from './trust-badge.model';

// Human consent and relationships
// Selective re-export — IntimacyLevel / ConsentState / INTIMACY_LEVEL_VALUES /
// isConsentActive / hasMinimumIntimacy / getNextIntimacyLevel are already
// exported by the sibling protocol-core.model above; avoid TS2308 barrel
// ambiguity by NOT re-exporting them here. Consumers needing the shared
// symbols reach them via this barrel (which routes them through
// protocol-core.model) or import from the specific human-consent file.
export type {
  HumanConsent,
  HumanConsentStateChange,
  ConsentRequest,
  ConsentResponse,
  ElevationRequest,
  RelationshipAttestationType,
} from './human-consent.model';
export {
  RELATIONSHIP_ATTESTATION_TYPES,
  requiresMutualAttestation,
  canElevate,
} from './human-consent.model';

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
// source-chain.model — migrated to @elohim/service (Slice 2.1b)
export * from '@elohim/service/angular/models/source-chain.model';
// json-ld.model, open-graph.model — migrated to @elohim/service (Slice 2.1)
export * from '@elohim/service/angular/models/json-ld.model';
export * from '@elohim/service/angular/models/open-graph.model';
export * from './verifiable-credential.model';

// Gated Response — migrated to @elohim/service (Slice 2.1)
export {
  isGatedResponse,
  extractGateFromResponse,
} from '@elohim/service/angular/models/gated-response.model';
export type { GatedResponse } from '@elohim/service/angular/models/gated-response.model';

// Zome Wire Types - Centralized Holochain zome response types
export * from './zome-wire-types';
