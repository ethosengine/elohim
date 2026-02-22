/**
 * Manifest Service
 *
 * Manages the ContentManifest for tracking imports and enabling incremental updates.
 * The manifest tracks:
 * - Source file hashes to detect changes
 * - Node hashes to validate output consistency
 * - Schema versions for migration tracking
 */

import * as crypto from 'crypto';
import * as fs from 'fs';
import * as path from 'path';

import {
  ContentManifest,
  SchemaMigration,
  MigrationRule,
  createEmptyManifest,
} from '../models/manifest.model';

const MANIFEST_FILENAME = 'content-manifest.json';
const CURRENT_SCHEMA_VERSION = '1.0.0';

/**
 * Load manifest from disk
 */
export function loadManifest(outputDir: string): ContentManifest {
  const manifestPath = path.join(outputDir, MANIFEST_FILENAME);

  if (!fs.existsSync(manifestPath)) {
    return createEmptyManifest();
  }

  try {
    const content = fs.readFileSync(manifestPath, 'utf-8');
    return JSON.parse(content) as ContentManifest;
  } catch {
    console.warn('Failed to load manifest, creating new one');
    return createEmptyManifest();
  }
}

/**
 * Save manifest to disk
 */
export function saveManifest(manifest: ContentManifest, outputDir: string): void {
  const manifestPath = path.join(outputDir, MANIFEST_FILENAME);

  // Ensure output directory exists
  if (!fs.existsSync(outputDir)) {
    fs.mkdirSync(outputDir, { recursive: true });
  }

  manifest.lastUpdated = new Date().toISOString();
  const content = JSON.stringify(manifest, null, 2);
  fs.writeFileSync(manifestPath, content, 'utf-8');
}

/**
 * Calculate hash for file content
 */
export function calculateFileHash(filePath: string): string {
  const content = fs.readFileSync(filePath, 'utf-8');
  return crypto.createHash('sha256').update(content).digest('hex');
}

/**
 * Calculate hash for node content
 */
export function calculateNodeHash(node: {
  id: string;
  content?: string;
  metadata?: unknown;
}): string {
  const hashContent = JSON.stringify({
    id: node.id,
    content: node.content,
    metadata: node.metadata,
  });
  return crypto.createHash('sha256').update(hashContent).digest('hex');
}

/**
 * Check if source file has changed since last import
 */
export function hasSourceChanged(
  manifest: ContentManifest,
  sourcePath: string,
  currentHash: string
): boolean {
  const entry = manifest.sourceHashes[sourcePath];

  if (!entry) {
    // New file
    return true;
  }

  return entry.hash !== currentHash;
}

/**
 * Update source hash in manifest
 */
export function updateSourceHash(
  manifest: ContentManifest,
  sourcePath: string,
  hash: string,
  nodeIds: string[]
): void {
  manifest.sourceHashes[sourcePath] = {
    hash,
    lastModified: new Date().toISOString(),
    generatedNodeIds: nodeIds,
  };
  manifest.totalSourceFiles = Object.keys(manifest.sourceHashes).length;
}

/**
 * Update node hash in manifest
 */
export function updateNodeHash(
  manifest: ContentManifest,
  nodeId: string,
  hash: string,
  sourcePath: string,
  contentType: string
): void {
  manifest.nodeHashes[nodeId] = {
    hash,
    sourcePath,
    contentType,
    generatedAt: new Date().toISOString(),
  };
  manifest.totalNodes = Object.keys(manifest.nodeHashes).length;
}

/**
 * Get nodes that need to be regenerated
 */
export function getStaleNodes(
  manifest: ContentManifest,
  currentSourceHashes: Map<string, string>
): string[] {
  const staleNodeIds: string[] = [];

  for (const [sourcePath, entry] of Object.entries(manifest.sourceHashes)) {
    const currentHash = currentSourceHashes.get(sourcePath);

    // Source was removed or changed
    if (!currentHash || currentHash !== entry.hash) {
      staleNodeIds.push(...entry.generatedNodeIds);
    }
  }

  return staleNodeIds;
}

/**
 * Get sources that need to be imported
 */
export function getNewOrChangedSources(
  manifest: ContentManifest,
  currentSourceHashes: Map<string, string>
): string[] {
  const toImport: string[] = [];

  for (const [sourcePath, hash] of currentSourceHashes) {
    if (hasSourceChanged(manifest, sourcePath, hash)) {
      toImport.push(sourcePath);
    }
  }

  return toImport;
}

/**
 * Get sources that were removed
 */
export function getRemovedSources(
  manifest: ContentManifest,
  currentSourcePaths: Set<string>
): string[] {
  const removed: string[] = [];

  for (const sourcePath of Object.keys(manifest.sourceHashes)) {
    if (!currentSourcePaths.has(sourcePath)) {
      removed.push(sourcePath);
    }
  }

  return removed;
}

/**
 * Remove source from manifest
 */
export function removeSource(manifest: ContentManifest, sourcePath: string): string[] {
  const entry = manifest.sourceHashes[sourcePath];

  if (!entry) {
    return [];
  }

  // Get node IDs that need to be removed
  const nodeIds = entry.generatedNodeIds;

  // Remove source entry
  delete manifest.sourceHashes[sourcePath];

  // Remove node entries
  for (const nodeId of nodeIds) {
    delete manifest.nodeHashes[nodeId];
  }

  // Update counts
  manifest.totalSourceFiles = Object.keys(manifest.sourceHashes).length;
  manifest.totalNodes = Object.keys(manifest.nodeHashes).length;

  return nodeIds;
}

/**
 * Add migration record
 */
export function addMigration(
  manifest: ContentManifest,
  fromVersion: string,
  toVersion: string,
  rules: MigrationRule[],
  nodesMigrated: number
): void {
  const migration: SchemaMigration = {
    id: `migration-${Date.now()}`,
    fromVersion,
    toVersion,
    appliedAt: new Date().toISOString(),
    nodesMigrated,
    rules,
  };

  manifest.migrations.push(migration);
  manifest.schemaVersion = toVersion;
}

/**
 * Check if schema migration is needed
 */
export function needsMigration(manifest: ContentManifest): boolean {
  return manifest.schemaVersion !== CURRENT_SCHEMA_VERSION;
}

/**
 * Get migration path from current version to target
 */
export function getMigrationPath(
  currentVersion: string,
  targetVersion: string = CURRENT_SCHEMA_VERSION
): string[] {
  // Simple linear versioning for now
  const versions = ['0.9.0', '1.0.0', '1.1.0', '2.0.0'];
  const currentIndex = versions.indexOf(currentVersion);
  const targetIndex = versions.indexOf(targetVersion);

  if (currentIndex === -1 || targetIndex === -1) {
    return [targetVersion]; // Force migration to target
  }

  if (currentIndex >= targetIndex) {
    return []; // Already at or past target
  }

  return versions.slice(currentIndex + 1, targetIndex + 1);
}

/**
 * Get import statistics from manifest
 */
export function getImportStats(manifest: ContentManifest): {
  totalSources: number;
  totalNodes: number;
  lastImport: string;
  schemaVersion: string;
  migrationCount: number;
} {
  return {
    totalSources: manifest.totalSourceFiles,
    totalNodes: manifest.totalNodes,
    lastImport: manifest.lastUpdated,
    schemaVersion: manifest.schemaVersion,
    migrationCount: manifest.migrations.length,
  };
}

/**
 * Validate manifest integrity
 */
export function validateManifest(manifest: ContentManifest): {
  valid: boolean;
  errors: string[];
} {
  const errors: string[] = [];

  // Check version
  if (!manifest.schemaVersion) {
    errors.push('Missing schema version');
  }

  // Check for orphaned node references
  const allNodeIds = new Set<string>();
  for (const entry of Object.values(manifest.sourceHashes)) {
    for (const nodeId of entry.generatedNodeIds) {
      allNodeIds.add(nodeId);
    }
  }

  for (const nodeId of Object.keys(manifest.nodeHashes)) {
    if (!allNodeIds.has(nodeId)) {
      errors.push(`Orphaned node hash: ${nodeId}`);
    }
  }

  // Check for missing node hashes
  for (const nodeId of allNodeIds) {
    if (!manifest.nodeHashes[nodeId]) {
      errors.push(`Missing node hash for referenced node: ${nodeId}`);
    }
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}
