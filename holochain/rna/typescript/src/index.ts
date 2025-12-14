/**
 * # @holochain/rna - Holochain RNA Migration Toolkit
 *
 * RNA (Ribonucleic Acid) in biology reads DNA and coordinates protein synthesis.
 * In Holochain, RNA transcribes data between DNA versions during migrations.
 *
 * ## The Biological Metaphor
 *
 * | Biology | Holochain Analog |
 * |---------|------------------|
 * | **DNA** | Integrity zome - immutable validation rules |
 * | **RNA** | This module - transcribes data between DNA versions |
 * | **Codon** | Transform function - maps old field patterns to new |
 * | **Ribosome** | Import function - synthesizes new entries |
 * | **mRNA** | Export data - carries information from source DNA |
 * | **tRNA** | Bridge call - transfers data between cells |
 * | **Polymerase** | Orchestrator - coordinates the transcription process |
 *
 * ## Quick Start
 *
 * ```typescript
 * import { connect, MigrationOrchestrator, formatReport } from '@holochain/rna';
 *
 * // 1. Connect to Holochain
 * const conn = await connect({
 *   adminUrl: 'ws://localhost:4444',
 *   appId: 'my-app',
 *   sourceRole: 'my-dna-v1',
 *   targetRole: 'my-dna-v2',
 * });
 *
 * // 2. Create orchestrator
 * const orchestrator = new MigrationOrchestrator(
 *   conn.appWs,
 *   conn.sourceCellId,
 *   conn.targetCellId,
 *   { sourceZome: 'coordinator', targetZome: 'coordinator' }
 * );
 *
 * // 3. Run migration
 * const report = await orchestrator.migrate({ dryRun: false });
 *
 * // 4. Check results
 * console.log(formatReport(report));
 * ```
 *
 * @module
 */

// Configuration
export {
  RNAConfig,
  MigrationOptions,
  ConnectionConfig,
  defaultConfig,
  defaultOptions,
  simpleConfig,
  mergeConfig,
  mergeOptions,
} from './config.js';

// Connection utilities
export {
  PortsConfig,
  HolochainConnection,
  readPorts,
  resolveAppUrl,
  extractCellId,
  formatCellId,
  connect,
  disconnect,
} from './connection.js';

// Report types and utilities
export {
  MigrationReport,
  MigrationCounts,
  MigrationError,
  MigrationPhase,
  MigrationVerification,
  CountCheck,
  createReport,
  recordSuccess,
  recordSkip,
  recordFailure,
  completeReport,
  isSuccess,
  totalImported,
  totalFailed,
  formatReport,
} from './report.js';

// Orchestrator
export {
  OrchestratorCallbacks,
  MigrationOrchestrator,
  createOrchestrator,
} from './orchestrator.js';
