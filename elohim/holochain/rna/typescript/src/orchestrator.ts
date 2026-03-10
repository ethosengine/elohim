/**
 * Migration Orchestrator
 *
 * # RNA Metaphor: Polymerase
 *
 * RNA Polymerase is the enzyme that catalyzes transcription -
 * it moves along the DNA template, reads it, and coordinates
 * the synthesis of RNA.
 *
 * The MigrationOrchestrator similarly coordinates the migration
 * process: reading from source DNA, transforming data, and
 * synthesizing entries in the target DNA.
 */

import { AppWebsocket, CellId } from '@holochain/client';
import { RNAConfig, MigrationOptions, defaultConfig, defaultOptions } from './config.js';
import {
  MigrationReport,
  MigrationVerification,
  createReport,
  completeReport,
  formatReport,
} from './report.js';

/**
 * Callbacks for migration progress reporting
 */
export interface OrchestratorCallbacks<ExportData> {
  /** Called when export phase starts */
  onExportStart?: () => void;
  /** Called when export completes with data */
  onExportComplete?: (data: ExportData) => void;
  /** Called during import with progress */
  onImportProgress?: (current: number, total: number, entryType: string) => void;
  /** Called when migration completes */
  onComplete?: (report: MigrationReport) => void;
  /** Called on any error */
  onError?: (error: Error, phase: string) => void;
}

/**
 * Generic migration orchestrator
 *
 * The Polymerase of the RNA module - coordinates the full
 * export → transform → import → verify pipeline.
 *
 * @typeParam ExportData - Shape of data exported from source DNA
 * @typeParam ImportData - Shape of data imported to target DNA (may differ after transform)
 *
 * # Example
 *
 * ```typescript
 * const orchestrator = new MigrationOrchestrator(
 *   connection.appWs,
 *   connection.sourceCellId,
 *   connection.targetCellId,
 *   {
 *     sourceRole: 'my-dna-v1',
 *     targetRole: 'my-dna-v2',
 *     sourceZome: 'coordinator',
 *     targetZome: 'coordinator',
 *   }
 * );
 *
 * const report = await orchestrator.migrate({ dryRun: false });
 * console.log(formatReport(report));
 * ```
 */
export class MigrationOrchestrator<ExportData = unknown, ImportData = ExportData> {
  private appWs: AppWebsocket;
  private sourceCellId: CellId;
  private targetCellId: CellId;
  private config: RNAConfig;
  private callbacks: OrchestratorCallbacks<ExportData>;

  constructor(
    appWs: AppWebsocket,
    sourceCellId: CellId,
    targetCellId: CellId,
    config: Partial<RNAConfig> = {},
    callbacks: OrchestratorCallbacks<ExportData> = {}
  ) {
    this.appWs = appWs;
    this.sourceCellId = sourceCellId;
    this.targetCellId = targetCellId;
    this.config = { ...defaultConfig, ...config };
    this.callbacks = callbacks;
  }

  /**
   * Execute the full migration pipeline
   *
   * @param options - Migration options (dry run, verify only, etc.)
   * @param transformer - Optional function to transform export data before import
   */
  async migrate(
    options: Partial<MigrationOptions> = {},
    transformer?: (data: ExportData) => ImportData
  ): Promise<MigrationReport> {
    const opts = { ...defaultOptions, ...options };

    // Get schema version from source
    let sourceVersion = 'unknown';
    try {
      sourceVersion = await this.callSource<string>(this.config.versionFn, null);
    } catch (e) {
      console.warn('Could not get source schema version:', e);
    }

    const report = createReport(sourceVersion, 'current');

    try {
      // Step 1: Export from source
      console.log('\n[1/4] Exporting from source DNA...');
      this.callbacks.onExportStart?.();

      const exportData = await this.export();
      this.callbacks.onExportComplete?.(exportData);

      this.logExportSummary(exportData);

      // Verify-only mode: skip to verification
      if (opts.verifyOnly) {
        console.log('\n[VERIFY ONLY] Skipping import...');
        report.verification = await this.verify(exportData);
        completeReport(report);
        this.callbacks.onComplete?.(report);
        return report;
      }

      // Dry-run mode: just show what would happen
      if (opts.dryRun) {
        console.log('\n[DRY RUN] No changes will be made');
        completeReport(report);
        return report;
      }

      // Step 2: Transform (if transformer provided)
      console.log('\n[2/4] Transforming data...');
      const importData: ImportData = transformer
        ? transformer(exportData)
        : (exportData as unknown as ImportData);

      // Step 3: Import into target
      console.log('\n[3/4] Importing into target DNA...');
      const importReport = await this.import(importData);

      // Merge import report
      Object.assign(report.entry_counts, importReport.entry_counts);
      report.errors.push(...importReport.errors);

      // Step 4: Verify
      console.log('\n[4/4] Verifying migration...');
      report.verification = await this.verify(exportData);

      completeReport(report);
      this.callbacks.onComplete?.(report);

      console.log('\n' + formatReport(report));

      return report;
    } catch (error) {
      this.callbacks.onError?.(error as Error, 'migration');
      report.errors.push({
        entry_type: 'system',
        entry_id: null,
        phase: 'Import',
        message: String(error),
      });
      completeReport(report);
      throw error;
    }
  }

  /**
   * Export data from source DNA
   */
  async export(): Promise<ExportData> {
    return this.callSource<ExportData>(this.config.exportFn, null);
  }

  /**
   * Import data into target DNA
   */
  async import(data: ImportData): Promise<MigrationReport> {
    return this.callTarget<MigrationReport>(this.config.importFn, data);
  }

  /**
   * Verify migration completeness
   */
  async verify(sourceData: ExportData): Promise<MigrationVerification> {
    const expectedCounts = this.extractCounts(sourceData);
    return this.callTarget<MigrationVerification>(
      this.config.verifyFn,
      expectedCounts
    );
  }

  /**
   * Call a function on the source DNA
   */
  async callSource<T>(fnName: string, payload: unknown): Promise<T> {
    return this.appWs.callZome({
      cell_id: this.sourceCellId,
      zome_name: this.config.sourceZome,
      fn_name: fnName,
      payload,
    }) as Promise<T>;
  }

  /**
   * Call a function on the target DNA
   */
  async callTarget<T>(fnName: string, payload: unknown): Promise<T> {
    return this.appWs.callZome({
      cell_id: this.targetCellId,
      zome_name: this.config.targetZome,
      fn_name: fnName,
      payload,
    }) as Promise<T>;
  }

  /**
   * Extract counts from export data for verification
   */
  private extractCounts(data: ExportData): Record<string, number> {
    const counts: Record<string, number> = {};
    if (data && typeof data === 'object') {
      for (const [key, value] of Object.entries(data)) {
        if (Array.isArray(value)) {
          counts[key] = value.length;
        }
      }
    }
    return counts;
  }

  /**
   * Log export summary
   */
  private logExportSummary(data: ExportData): void {
    console.log('Export summary:');
    if (data && typeof data === 'object') {
      for (const [key, value] of Object.entries(data)) {
        if (Array.isArray(value)) {
          console.log(`  ${key}: ${value.length} items`);
        }
      }
    }
  }
}

/**
 * Create a simple orchestrator with minimal configuration
 */
export function createOrchestrator<E = unknown, I = E>(
  appWs: AppWebsocket,
  sourceCellId: CellId,
  targetCellId: CellId,
  zomeName: string
): MigrationOrchestrator<E, I> {
  return new MigrationOrchestrator(
    appWs,
    sourceCellId,
    targetCellId,
    {
      sourceZome: zomeName,
      targetZome: zomeName,
    }
  );
}
