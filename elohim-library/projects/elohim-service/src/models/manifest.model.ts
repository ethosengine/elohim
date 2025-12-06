/**
 * Content Manifest Model
 *
 * Tracks imported content for incremental updates and schema migrations.
 * Stored at: elohim-app/src/assets/lamad-data/.import-manifest.json
 */

/**
 * Hash entry for a source file
 */
export interface SourceHashEntry {
  /** SHA256 hash of source file content */
  hash: string;

  /** Last modified timestamp of source file */
  lastModified: string;

  /** Node IDs generated from this source */
  generatedNodeIds: string[];
}

/**
 * Hash entry for a generated node
 */
export interface NodeHashEntry {
  /** SHA256 hash of generated JSON */
  hash: string;

  /** Source file this was derived from */
  sourcePath: string;

  /** Content type of the node */
  contentType: string;

  /** Generation timestamp */
  generatedAt: string;
}

/**
 * Schema migration record
 */
export interface SchemaMigration {
  /** Migration identifier */
  id: string;

  /** From schema version */
  fromVersion: string;

  /** To schema version */
  toVersion: string;

  /** When migration was applied */
  appliedAt: string;

  /** Number of nodes migrated */
  nodesMigrated: number;

  /** Migration rules applied */
  rules: MigrationRule[];
}

/**
 * Rule for schema migration
 */
export interface MigrationRule {
  /** Field to transform */
  field: string;

  /** Transformation to apply */
  transform: 'lowercase' | 'uppercase' | 'rename' | 'delete' | 'default' | 'custom';

  /** New field name (for rename) */
  newField?: string;

  /** Default value (for default) */
  defaultValue?: unknown;

  /** Custom transformation function (serialized) */
  customTransform?: string;
}

/**
 * Content manifest - tracks all imported content
 */
export interface ContentManifest {
  /** Manifest schema version */
  manifestVersion: string;

  /** Lamad schema version content was generated for */
  schemaVersion: string;

  /** When manifest was last updated */
  lastUpdated: string;

  /** Import tool version */
  importToolVersion: string;

  /** Total source files tracked */
  totalSourceFiles: number;

  /** Total nodes generated */
  totalNodes: number;

  /** Total relationships generated */
  totalRelationships: number;

  /**
   * Source file hashes for change detection
   * Key: relative source path
   */
  sourceHashes: Record<string, SourceHashEntry>;

  /**
   * Generated node hashes for integrity checking
   * Key: node ID
   */
  nodeHashes: Record<string, NodeHashEntry>;

  /**
   * Schema migrations applied
   */
  migrations: SchemaMigration[];

  /**
   * Import statistics by domain
   */
  domainStats: Record<string, {
    sourceFiles: number;
    nodes: number;
    lastImported: string;
  }>;

  /**
   * Import statistics by content type
   */
  contentTypeStats: Record<string, {
    count: number;
    lastUpdated: string;
  }>;
}

/**
 * Create empty manifest
 */
export function createEmptyManifest(): ContentManifest {
  return {
    manifestVersion: '1.0.0',
    schemaVersion: '1.0.0',
    lastUpdated: new Date().toISOString(),
    importToolVersion: '0.1.0',
    totalSourceFiles: 0,
    totalNodes: 0,
    totalRelationships: 0,
    sourceHashes: {},
    nodeHashes: {},
    migrations: [],
    domainStats: {},
    contentTypeStats: {}
  };
}
