/**
 * Import Context Model
 *
 * Tracks state during the import pipeline execution.
 */

import { ContentNode, ContentRelationship } from './content-node.model';
import { PathMetadata } from './path-metadata.model';

/**
 * Import mode
 */
export type ImportMode =
  | 'full' // Import everything from scratch
  | 'incremental' // Only import changed files
  | 'schema-migrate'; // Update existing to new schema

/**
 * Options for import pipeline
 */
export interface ImportOptions {
  /** Import mode */
  mode: ImportMode;

  /** Source content directory */
  sourceDir: string;

  /** Output directory for generated content */
  outputDir: string;

  /** Specific domains to import (empty = all) */
  domains?: string[];

  /** Specific epics to import (empty = all) */
  epics?: string[];

  /** Node IDs to force reimport even if unchanged */
  forceReimport?: string[];

  /** Fields to preserve from existing nodes during incremental */
  preserveFields?: string[];

  /** Whether to generate source nodes (provenance layer) */
  generateSourceNodes?: boolean;

  /** Whether to generate derived concept nodes */
  generateDerivedNodes?: boolean;

  /** Dry run - don't write files */
  dryRun?: boolean;

  /** Verbose logging */
  verbose?: boolean;

  /** Skip relationship extraction (faster, less memory) */
  skipRelationships?: boolean;

  /** Path to Kuzu database (if set, writes to Kuzu instead of JSON) */
  dbPath?: string;
}

/**
 * Parsed content before transformation
 */
export interface ParsedContent {
  /** Path metadata */
  pathMeta: PathMetadata;

  /** YAML frontmatter (if any) */
  frontmatter: Record<string, unknown>;

  /** Raw content string */
  rawContent: string;

  /** Parsed sections (for markdown) */
  sections?: ParsedSection[];

  /** Parsed scenarios (for gherkin) */
  scenarios?: ParsedScenario[];

  /** Extracted title */
  title: string;

  /** File hash for change detection */
  contentHash: string;
}

/**
 * Markdown section
 */
export interface ParsedSection {
  level: number;
  title: string;
  anchor: string;
  content: string;
  children: ParsedSection[];
}

/**
 * Gherkin scenario
 */
export interface ParsedScenario {
  title: string;
  type: 'scenario' | 'scenario_outline';
  tags: string[];
  steps: {
    keyword: string;
    text: string;
  }[];
}

/**
 * Result of importing a single file
 */
export interface ImportFileResult {
  /** Source file path */
  sourcePath: string;

  /** Status of import */
  status: 'created' | 'updated' | 'skipped' | 'error';

  /** Generated node IDs */
  nodeIds: string[];

  /** Error message if status is 'error' */
  error?: string;

  /** Processing time in ms */
  processingTime: number;
}

/**
 * Aggregate result of import pipeline
 */
export interface ImportResult {
  /** Import started at */
  startedAt: string;

  /** Import completed at */
  completedAt: string;

  /** Total files processed */
  totalFiles: number;

  /** Files created */
  created: number;

  /** Files updated */
  updated: number;

  /** Files skipped (unchanged) */
  skipped: number;

  /** Files with errors */
  errors: number;

  /** Total nodes generated */
  totalNodes: number;

  /** Total relationships generated */
  totalRelationships: number;

  /** Individual file results */
  fileResults: ImportFileResult[];

  /** All generated nodes */
  nodes: ContentNode[];

  /** All generated relationships */
  relationships: ContentRelationship[];
}

/**
 * Import pipeline context - passed through pipeline stages
 */
export interface ImportContext {
  /** Import options */
  options: ImportOptions;

  /** Current stage of pipeline */
  stage: 'scanning' | 'parsing' | 'transforming' | 'generating' | 'writing' | 'complete';

  /** Files discovered during scan */
  discoveredFiles: string[];

  /** Parsed content by file path */
  parsedContent: Map<string, ParsedContent>;

  /** Generated nodes by ID */
  nodes: Map<string, ContentNode>;

  /** Generated relationships */
  relationships: ContentRelationship[];

  /** Import results */
  results: ImportResult;

  /** Manifest from previous import (for incremental) */
  previousManifest?: import('./manifest.model').ContentManifest;
}
