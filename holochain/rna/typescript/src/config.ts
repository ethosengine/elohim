/**
 * RNA Migration Configuration Types
 *
 * # RNA Metaphor: Promoter Sequences
 *
 * In biology, promoter sequences are DNA regions that initiate transcription.
 * They determine when and how genes are expressed.
 *
 * These configuration types serve a similar purpose - they control when and
 * how migration "transcription" occurs.
 */

/**
 * Configuration for the RNA migration orchestrator
 */
export interface RNAConfig {
  /** Source DNA role name in happ.yaml */
  sourceRole: string;
  /** Target DNA role name in happ.yaml */
  targetRole: string;
  /** Zome name in source DNA */
  sourceZome: string;
  /** Zome name in target DNA */
  targetZome: string;
  /** Export function name */
  exportFn: string;
  /** Import function name */
  importFn: string;
  /** Verify function name */
  verifyFn: string;
  /** Schema version function name */
  versionFn: string;
}

/**
 * Default configuration values
 */
export const defaultConfig: RNAConfig = {
  sourceRole: 'previous',
  targetRole: 'current',
  sourceZome: 'coordinator',
  targetZome: 'coordinator',
  exportFn: 'export_for_migration',
  importFn: 'import_migrated',
  verifyFn: 'verify_migration',
  versionFn: 'export_schema_version',
};

/**
 * Options for controlling migration behavior
 */
export interface MigrationOptions {
  /** Show what would be migrated without making changes */
  dryRun: boolean;
  /** Only verify existing migration, don't import */
  verifyOnly: boolean;
  /** Entry types to migrate (empty = all) */
  entryTypes: string[];
  /** Maximum entries per type (for testing) */
  limit?: number;
}

/**
 * Default migration options
 */
export const defaultOptions: MigrationOptions = {
  dryRun: false,
  verifyOnly: false,
  entryTypes: [],
  limit: undefined,
};

/**
 * Connection configuration for Holochain
 */
export interface ConnectionConfig {
  /** Admin WebSocket URL (e.g., "ws://localhost:4444") */
  adminUrl: string;
  /** App ID to connect to (e.g., "my-app") */
  appId: string;
  /** Path to .hc_ports file (optional) */
  portsFile?: string;
}

/**
 * Create an RNAConfig for simple same-zome migrations
 */
export function simpleConfig(
  sourceRole: string,
  targetRole: string,
  zome: string
): RNAConfig {
  return {
    ...defaultConfig,
    sourceRole,
    targetRole,
    sourceZome: zome,
    targetZome: zome,
  };
}

/**
 * Merge user config with defaults
 */
export function mergeConfig(partial: Partial<RNAConfig>): RNAConfig {
  return { ...defaultConfig, ...partial };
}

/**
 * Merge user options with defaults
 */
export function mergeOptions(partial: Partial<MigrationOptions>): MigrationOptions {
  return { ...defaultOptions, ...partial };
}
