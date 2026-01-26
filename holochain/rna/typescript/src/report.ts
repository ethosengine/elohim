/**
 * Migration Report Types
 *
 * # RNA Metaphor: Transcription Reports
 *
 * These types match the Rust report structures for serialization
 * between TypeScript and the Holochain zome.
 */

/**
 * Complete migration report
 */
export interface MigrationReport {
  /** Schema version of source DNA */
  source_version: string;
  /** Schema version of target DNA */
  target_version: string;
  /** When migration started */
  started_at: string;
  /** When migration completed (null if still running) */
  completed_at: string | null;
  /** Per-entry-type migration counts */
  entry_counts: Record<string, MigrationCounts>;
  /** List of errors encountered */
  errors: MigrationError[];
  /** Post-migration verification results */
  verification: MigrationVerification;
}

/**
 * Counts for a single entry type
 */
export interface MigrationCounts {
  /** Number exported from source */
  exported: number;
  /** Number transformed */
  transformed: number;
  /** Number successfully imported */
  imported: number;
  /** Number skipped (already exist) */
  skipped: number;
  /** Number failed to import */
  failed: number;
}

/**
 * A single migration error
 */
export interface MigrationError {
  /** Entry type that failed */
  entry_type: string;
  /** Specific entry ID if available */
  entry_id: string | null;
  /** Phase where error occurred */
  phase: MigrationPhase;
  /** Error message */
  message: string;
}

/**
 * Migration phase for error tracking
 */
export type MigrationPhase = 'Export' | 'Transform' | 'Import' | 'Verify';

/**
 * Verification results
 */
export interface MigrationVerification {
  /** Overall verification passed */
  passed: boolean;
  /** Per-type count checks */
  count_checks: Record<string, CountCheck>;
  /** Reference integrity passed */
  reference_integrity: boolean;
  /** Additional notes */
  notes: string[];
}

/**
 * Single count check result
 */
export interface CountCheck {
  /** Expected count from source */
  expected: number;
  /** Actual count in target */
  actual: number;
  /** Whether this check passed */
  passed: boolean;
}

/**
 * Create an empty migration report
 */
export function createReport(
  sourceVersion: string,
  targetVersion: string
): MigrationReport {
  return {
    source_version: sourceVersion,
    target_version: targetVersion,
    started_at: new Date().toISOString(),
    completed_at: null,
    entry_counts: {},
    errors: [],
    verification: {
      passed: false,
      count_checks: {},
      reference_integrity: false,
      notes: [],
    },
  };
}

/**
 * Record a successful import in the report
 */
export function recordSuccess(report: MigrationReport, entryType: string): void {
  if (!report.entry_counts[entryType]) {
    report.entry_counts[entryType] = {
      exported: 0,
      transformed: 0,
      imported: 0,
      skipped: 0,
      failed: 0,
    };
  }
  report.entry_counts[entryType].imported++;
}

/**
 * Record a skipped entry (already exists)
 */
export function recordSkip(report: MigrationReport, entryType: string): void {
  if (!report.entry_counts[entryType]) {
    report.entry_counts[entryType] = {
      exported: 0,
      transformed: 0,
      imported: 0,
      skipped: 0,
      failed: 0,
    };
  }
  report.entry_counts[entryType].skipped++;
}

/**
 * Record a failed import
 */
export function recordFailure(
  report: MigrationReport,
  entryType: string,
  entryId: string | null,
  message: string
): void {
  if (!report.entry_counts[entryType]) {
    report.entry_counts[entryType] = {
      exported: 0,
      transformed: 0,
      imported: 0,
      skipped: 0,
      failed: 0,
    };
  }
  report.entry_counts[entryType].failed++;
  report.errors.push({
    entry_type: entryType,
    entry_id: entryId,
    phase: 'Import',
    message,
  });
}

/**
 * Mark the report as complete
 */
export function completeReport(report: MigrationReport): void {
  report.completed_at = new Date().toISOString();
}

/**
 * Check if migration was successful
 */
export function isSuccess(report: MigrationReport): boolean {
  const noFailures = Object.values(report.entry_counts).every(
    (c) => c.failed === 0
  );
  return noFailures && report.verification.passed;
}

/**
 * Get total imported count
 */
export function totalImported(report: MigrationReport): number {
  return Object.values(report.entry_counts).reduce(
    (sum, c) => sum + c.imported,
    0
  );
}

/**
 * Get total failed count
 */
export function totalFailed(report: MigrationReport): number {
  return Object.values(report.entry_counts).reduce(
    (sum, c) => sum + c.failed,
    0
  );
}

/**
 * Format report for console output
 */
export function formatReport(report: MigrationReport): string {
  const lines: string[] = [
    '='.repeat(60),
    '  Migration Report',
    '='.repeat(60),
    `Source: ${report.source_version} -> Target: ${report.target_version}`,
    `Started: ${report.started_at}`,
    `Completed: ${report.completed_at || 'In progress'}`,
    '',
    'Entry Counts:',
  ];

  for (const [type, counts] of Object.entries(report.entry_counts)) {
    lines.push(
      `  ${type}: ${counts.imported} imported, ${counts.skipped} skipped, ${counts.failed} failed`
    );
  }

  if (report.errors.length > 0) {
    lines.push('', 'Errors:');
    for (const err of report.errors.slice(0, 10)) {
      const id = err.entry_id ? ` (${err.entry_id})` : '';
      lines.push(`  [${err.phase}] ${err.entry_type}${id}: ${err.message}`);
    }
    if (report.errors.length > 10) {
      lines.push(`  ... and ${report.errors.length - 10} more`);
    }
  }

  lines.push('', `Verification: ${report.verification.passed ? 'PASSED' : 'FAILED'}`);

  if (report.verification.notes.length > 0) {
    for (const note of report.verification.notes) {
      lines.push(`  - ${note}`);
    }
  }

  lines.push('='.repeat(60));

  return lines.join('\n');
}
