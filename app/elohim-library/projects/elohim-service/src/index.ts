/**
 * Elohim Service - Content Intelligence Module
 *
 * Provides autonomous content import, analysis, and path generation
 * for the Elohim Protocol lamad learning platform.
 *
 * Main capabilities:
 * - Import: Transform raw markdown/gherkin into ContentNode models
 * - Analyze: Assess content quality and identify gaps
 * - Generate: Create learning paths from content graph
 *
 * Architecture:
 * - Source layer: Raw imported files preserved with provenance
 * - Knowledge layer: Derived concepts, paths, relationships
 *
 * Entry points:
 * - CLI: `npx ts-node src/cli/import.ts import`
 * - Skill: `executeSkill({ action: 'import' })`
 * - Programmatic: `runImportPipeline(options)`
 */

// Models
export * from './models/content-node.model';
export * from './models/path-metadata.model';
export * from './models/import-context.model';
export * from './models/manifest.model';

// Parsers
export * from './parsers/path-metadata-parser';
export * from './parsers/markdown-parser';
export * from './parsers/gherkin-parser';

// Transformers
export * from './transformers';

// Adapters (wire format normalization for multi-transport content pipeline)
export * from './adapters';

// Services
export * from './services/relationship-extractor.service';
export * from './services/manifest.service';
export * from './services/import-pipeline.service';

// Cache (framework-agnostic reach-aware caching)
export * from './cache';

// Client (mode-aware content client, mirrors Rust elohim-sdk)
// Note: Selective export to avoid conflicts with cache module types
export {
  // Main client
  ElohimClient,
  WriteBuffer as ClientWriteBuffer,
  ReachEnforcer,

  // Modes
  type ClientMode,
  type BrowserMode,
  type TauriMode,
  type TauriInvoke,

  // Sync configuration
  type DoorwayConfig,
  type NodeSyncConfig,

  // Holochain (parallel connection, not a mode)
  type HolochainConnection,

  // Content interfaces
  type ContentType as ClientContentType,
  type ContentReadable,
  type ContentWriteable,
  type ContentQuery,

  // Config
  type ElohimClientConfig,
  type WriteOp,
  WriteBufferDefaults,

  // Angular integration
  ELOHIM_CLIENT,
  ELOHIM_CLIENT_CONFIG,
  provideElohimClient,
  provideAnonymousBrowserClient,
  detectClientMode,
} from './client';

// Re-export main functions for convenience
export { runImportPipeline, importContent } from './services/import-pipeline.service';

// ============================================================================
// Angular SDK surface (Slice 2.1 — cross-pillar import cleanup)
// ============================================================================
// Models — selective export to avoid conflict with cache module's MasteryLevel constant
// (cache module exports a MasteryLevel const-enum; agent.model exports MasteryLevel string-union)
// Consumers that need the agent MasteryLevel type should import from the sub-path:
//   import type { MasteryLevel } from '@elohim/service/angular/models/agent'
// NOTE: MasteryLevel is NOT re-exported here — it conflicts with cache module's MasteryLevel
// constant-enum. Use sub-path import for the agent string-union type:
//   import type { MasteryLevel } from '@elohim/service/angular/models/agent.model'
export type {
  Agent,
  AgentProgress,
  AgentAttestation,
  NewAttestation,
  AttestationCategory,
  FrontierItem,
  MasteryTier,
} from './angular/models/agent.model';
export {
  getMasteryTier,
  getMasteryProgress,
  isAboveGate,
  compareMasteryLevels,
  MASTERY_LEVEL_VALUES,
  BLOOM_LEVEL_VALUES,
  ATTESTATION_GATE_LEVEL,
} from './angular/models/agent.model';

export * from './angular/models/json-ld.model';
export * from './angular/models/open-graph.model';
export * from './angular/models/gated-response.model';

// Interfaces (Angular DI tokens + abstract contracts)
export * from './angular/interfaces/governance.interface';
export * from './angular/interfaces/content-attestation.interface';
export * from './angular/interfaces/blob-fetcher.interface';

// Environment token
export type { ElohimEnv } from './env/elohim-env';
export { ELOHIM_ENV } from './env/elohim-env';

// Models (angular layer)
export * from './angular/models/source-chain.model';

// Services
export { GovernanceApiService } from './angular/services/governance-api.service';
export { DoorwayClientService } from './angular/services/doorway-client.service';
export type {
  VerifyBlobRequest,
  VerifyBlobResponse,
  CustodianInfo,
  BestCustodianResponse,
  StreamingVariant,
  StreamingManifest,
} from './angular/services/doorway-client.service';
export { LocalSourceChainService } from './angular/services/local-source-chain.service';

// Utils
export * from './angular/utils/access-control.helper';
export * from './angular/utils/epr-ref';
