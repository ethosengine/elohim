/**
 * Import Pipeline Service
 *
 * Orchestrates the full content import process:
 * 1. Scan source directory for content files
 * 2. Parse and extract metadata from each file
 * 3. Transform content into ContentNodes
 * 4. Extract relationships between nodes
 * 5. Write output and update manifest
 */

import * as fs from 'fs';
import * as path from 'path';

import { glob } from 'glob';

import { KuzuClient } from '../db/kuzu-client';
import { ContentNode, ContentRelationship } from '../models/content-node.model';
import {
  ImportOptions,
  ImportResult,
  ImportContext,
  ImportFileResult,
  ParsedContent,
} from '../models/import-context.model';
import { parseGherkin } from '../parsers/gherkin-parser';
import { parseMarkdown } from '../parsers/markdown-parser';
import { parsePathMetadata } from '../parsers/path-metadata-parser';
import {
  transformToSourceNode,
  shouldCreateSourceNode,
  transformArchetype,
  isArchetypeContent,
  transformEpic,
  isEpicContent,
  transformScenarios,
  isScenarioContent,
  transformResource,
  isResourceContent,
} from '../transformers';

import {
  loadManifest,
  saveManifest,
  calculateFileHash,
  calculateNodeHash,
  updateSourceHash,
  updateNodeHash,
  getNewOrChangedSources,
  getRemovedSources,
  removeSource,
} from './manifest.service';
import {
  extractRelationships,
  RelationshipExtractionOptions,
} from './relationship-extractor.service';

/**
 * Run the full import pipeline
 */
export async function runImportPipeline(options: ImportOptions): Promise<ImportResult> {
  const startTime = new Date();
  const context = createImportContext(options);

  console.log(`Starting import from: ${options.sourceDir}`);
  console.log(`Output directory: ${options.outputDir}`);
  console.log(`Mode: ${options.mode}`);

  try {
    // 1. Scan source files
    context.stage = 'scanning';
    const sourceFiles = await scanSourceFiles(options.sourceDir);
    context.discoveredFiles = sourceFiles;
    console.log(`Found ${sourceFiles.length} source files`);

    // 2. Calculate hashes and determine what needs importing
    const sourceHashes = new Map<string, string>();
    for (const file of sourceFiles) {
      sourceHashes.set(file, calculateFileHash(file));
    }

    // 3. Determine incremental changes
    let filesToProcess: string[];
    let removedFiles: string[] = [];

    if (options.mode === 'incremental' && context.previousManifest) {
      const newOrChanged = getNewOrChangedSources(context.previousManifest, sourceHashes);
      removedFiles = getRemovedSources(context.previousManifest, new Set(sourceFiles));
      filesToProcess = newOrChanged;

      console.log(
        `Incremental mode: ${newOrChanged.length} new/changed, ${removedFiles.length} removed`
      );
    } else {
      filesToProcess = sourceFiles;
      console.log(`Full import mode: ${filesToProcess.length} files`);
    }

    // 4. Remove stale nodes from removed sources
    for (const removed of removedFiles) {
      if (context.previousManifest) {
        const removedNodeIds = removeSource(context.previousManifest, removed);
        console.log(`Removed ${removedNodeIds.length} nodes from deleted source: ${removed}`);
      }
    }

    // 5. Process each file
    context.stage = 'parsing';
    const fileResults: ImportFileResult[] = [];

    for (const file of filesToProcess) {
      const fileStartTime = Date.now();
      try {
        const nodes = await processFile(file, options.sourceDir, context);

        // Update manifest
        if (context.previousManifest) {
          const hash = sourceHashes.get(file)!;
          const nodeIds = nodes.map(n => n.id);
          updateSourceHash(context.previousManifest, file, hash, nodeIds);

          for (const node of nodes) {
            updateNodeHash(
              context.previousManifest,
              node.id,
              calculateNodeHash(node),
              file,
              node.contentType
            );
          }
        }

        fileResults.push({
          sourcePath: file,
          status: 'created',
          nodeIds: nodes.map(n => n.id),
          processingTime: Date.now() - fileStartTime,
        });
      } catch (err) {
        const errorMsg = `Error processing ${file}: ${err}`;
        console.error(errorMsg);
        fileResults.push({
          sourcePath: file,
          status: 'error',
          nodeIds: [],
          error: errorMsg,
          processingTime: Date.now() - fileStartTime,
        });
      }
    }

    // 6. Load existing nodes if incremental
    let existingNodes: ContentNode[] = [];
    if (options.mode === 'incremental') {
      existingNodes = await loadExistingNodes(options.outputDir, filesToProcess);
    }

    // 7. Merge nodes
    const allNodes = Array.from(context.nodes.values());
    const mergedNodes = mergeNodes(existingNodes, allNodes);

    // 8. Extract relationships (unless skipped)
    context.stage = 'generating';
    let relationships: ContentRelationship[] = [];

    if (options.skipRelationships) {
      console.log('Skipping relationship extraction (--skip-relationships flag)');
    } else {
      console.log('Extracting relationships...');
      const relationshipOptions: RelationshipExtractionOptions = {
        includePath: true,
        includeTags: true,
        includeContent: false, // Disabled by default - expensive
        minScore: 0.5,
        maxPerNode: 10,
      };
      relationships = extractRelationships(mergedNodes, relationshipOptions);
      context.relationships = relationships;
      console.log(`Extracted ${relationships.length} relationships`);
    }

    // 9. Write to Kuzu database
    context.stage = 'writing';
    if (!options.dryRun) {
      if (!options.dbPath) {
        throw new Error('dbPath is required. Use --db to specify the Kuzu database path.');
      }
      await writeToKuzu(mergedNodes, relationships, options.dbPath);

      // Save manifest for incremental tracking
      if (context.previousManifest) {
        context.previousManifest.totalRelationships = relationships.length;
        saveManifest(context.previousManifest, options.outputDir);
      }
    }

    // 10. Build result
    context.stage = 'complete';
    const endTime = new Date();

    const result: ImportResult = {
      startedAt: startTime.toISOString(),
      completedAt: endTime.toISOString(),
      totalFiles: sourceFiles.length,
      created: fileResults.filter(r => r.status === 'created').length,
      updated: 0,
      skipped: sourceFiles.length - filesToProcess.length,
      errors: fileResults.filter(r => r.status === 'error').length,
      totalNodes: mergedNodes.length,
      totalRelationships: relationships.length,
      fileResults,
      nodes: mergedNodes,
      relationships,
    };

    console.log(`\nImport complete in ${endTime.getTime() - startTime.getTime()}ms`);
    console.log(`  Nodes: ${result.totalNodes}`);
    console.log(`  Relationships: ${result.totalRelationships}`);
    console.log(`  Errors: ${result.errors}`);

    return result;
  } catch (err) {
    const errorMsg = `Pipeline failed: ${err}`;
    console.error(errorMsg);

    return {
      startedAt: startTime.toISOString(),
      completedAt: new Date().toISOString(),
      totalFiles: 0,
      created: 0,
      updated: 0,
      skipped: 0,
      errors: 1,
      totalNodes: 0,
      totalRelationships: 0,
      fileResults: [
        {
          sourcePath: '',
          status: 'error',
          nodeIds: [],
          error: errorMsg,
          processingTime: 0,
        },
      ],
      nodes: [],
      relationships: [],
    };
  }
}

/**
 * Create import context
 */
function createImportContext(options: ImportOptions): ImportContext {
  const manifest = options.mode === 'incremental' ? loadManifest(options.outputDir) : undefined;

  return {
    options,
    stage: 'scanning',
    discoveredFiles: [],
    parsedContent: new Map(),
    nodes: new Map(),
    relationships: [],
    results: {
      startedAt: new Date().toISOString(),
      completedAt: '',
      totalFiles: 0,
      created: 0,
      updated: 0,
      skipped: 0,
      errors: 0,
      totalNodes: 0,
      totalRelationships: 0,
      fileResults: [],
      nodes: [],
      relationships: [],
    },
    previousManifest: manifest,
  };
}

/**
 * Scan source directory for content files
 */
async function scanSourceFiles(sourceDir: string): Promise<string[]> {
  const patterns = [path.join(sourceDir, '**/*.md'), path.join(sourceDir, '**/*.feature')];

  const files: string[] = [];

  for (const pattern of patterns) {
    const matches = await glob(pattern, { nodir: true });
    files.push(...matches);
  }

  // Sort for deterministic processing
  return files.sort();
}

/**
 * Process a single file
 */
async function processFile(
  filePath: string,
  sourceDir: string,
  context: ImportContext
): Promise<ContentNode[]> {
  const nodes: ContentNode[] = [];

  // Parse path metadata
  const pathMeta = parsePathMetadata(filePath, { contentRoot: sourceDir });

  // Read file content
  const content = fs.readFileSync(filePath, 'utf-8');

  // Parse based on extension
  let parsed: ParsedContent;
  if (pathMeta.extension === '.feature') {
    parsed = parseGherkin(content, pathMeta);
  } else {
    parsed = parseMarkdown(content, pathMeta);
  }

  // Store parsed content
  context.parsedContent.set(filePath, parsed);

  // Create source node for provenance
  let sourceNodeId: string | undefined;
  if (shouldCreateSourceNode(parsed)) {
    const sourceNode = transformToSourceNode(parsed);
    nodes.push(sourceNode);
    context.nodes.set(sourceNode.id, sourceNode);
    sourceNodeId = sourceNode.id;
  }

  // Transform based on content type
  if (isEpicContent(parsed)) {
    const epicNode = transformEpic(parsed, sourceNodeId);
    nodes.push(epicNode);
    context.nodes.set(epicNode.id, epicNode);
  } else if (isArchetypeContent(parsed)) {
    const archetypeNode = transformArchetype(parsed, sourceNodeId);
    nodes.push(archetypeNode);
    context.nodes.set(archetypeNode.id, archetypeNode);
  } else if (isScenarioContent(parsed)) {
    const scenarioNodes = transformScenarios(parsed, sourceNodeId);
    for (const node of scenarioNodes) {
      nodes.push(node);
      context.nodes.set(node.id, node);
    }
  } else if (isResourceContent(parsed)) {
    const resourceNode = transformResource(parsed, sourceNodeId);
    nodes.push(resourceNode);
    context.nodes.set(resourceNode.id, resourceNode);
  }

  return nodes;
}

/**
 * Load existing nodes from output directory (for incremental)
 */
async function loadExistingNodes(
  outputDir: string,
  excludeSourcePaths: string[]
): Promise<ContentNode[]> {
  const nodesPath = path.join(outputDir, 'nodes.json');

  if (!fs.existsSync(nodesPath)) {
    return [];
  }

  try {
    const content = fs.readFileSync(nodesPath, 'utf-8');
    const allNodes = JSON.parse(content) as ContentNode[];

    // Filter out nodes from sources being reprocessed
    const excludeSet = new Set(excludeSourcePaths);
    return allNodes.filter(node => !excludeSet.has(node.sourcePath || ''));
  } catch {
    return [];
  }
}

/**
 * Merge existing and new nodes
 */
function mergeNodes(existing: ContentNode[], newNodes: ContentNode[]): ContentNode[] {
  const nodeMap = new Map<string, ContentNode>();

  // Add existing nodes
  for (const node of existing) {
    nodeMap.set(node.id, node);
  }

  // Overwrite with new nodes
  for (const node of newNodes) {
    nodeMap.set(node.id, node);
  }

  return Array.from(nodeMap.values());
}

/**
 * Write to Kuzu database (new primary flow)
 */
async function writeToKuzu(
  nodes: ContentNode[],
  relationships: ContentRelationship[],
  dbPath: string
): Promise<void> {
  console.log(`\nWriting to Kuzu database: ${dbPath}`);

  const client = new KuzuClient(dbPath);
  await client.initialize();

  try {
    // Deduplicate nodes by ID
    const seenIds = new Set<string>();
    const uniqueNodes = nodes.filter(node => {
      if (seenIds.has(node.id)) {
        return false;
      }
      seenIds.add(node.id);
      return true;
    });

    console.log(`  Found ${nodes.length} nodes, ${uniqueNodes.length} unique`);

    // Insert content nodes
    const insertedNodes = await client.bulkInsertContentNodes(uniqueNodes);
    console.log(`  Inserted ${insertedNodes} content nodes`);

    // Insert relationships
    const insertedRels = await client.bulkInsertRelationships(relationships);
    console.log(`  Inserted ${insertedRels} relationships`);

    // Show stats
    const stats = await client.getStats();
    console.log('\n  Database Statistics:');
    for (const [table, count] of Object.entries(stats)) {
      if (count > 0) {
        console.log(`    ${table}: ${count}`);
      }
    }
  } finally {
    client.close();
  }
}

/**
 * Generate import summary for logging
 */
function _generateSummary(
  nodes: ContentNode[],
  relationships: ContentRelationship[]
): Record<string, unknown> {
  // Count by content type
  const byType: Record<string, number> = {};
  for (const node of nodes) {
    byType[node.contentType] = (byType[node.contentType] || 0) + 1;
  }

  // Count relationships by type
  const relByType: Record<string, number> = {};
  for (const rel of relationships) {
    relByType[rel.relationshipType] = (relByType[rel.relationshipType] || 0) + 1;
  }

  // Extract unique epics
  const epics = new Set<string>();
  for (const node of nodes) {
    if (node.metadata?.epic) {
      epics.add(node.metadata.epic);
    }
  }

  // Extract unique user types
  const userTypes = new Set<string>();
  for (const node of nodes) {
    if (node.metadata?.userType) {
      userTypes.add(node.metadata.userType);
    }
  }

  return {
    generatedAt: new Date().toISOString(),
    totals: {
      nodes: nodes.length,
      relationships: relationships.length,
    },
    nodesByType: byType,
    relationshipsByType: relByType,
    epics: Array.from(epics).sort(),
    userTypes: Array.from(userTypes).sort(),
  };
}

/**
 * Simple import function with sensible defaults
 */
export async function importContent(
  sourceDir: string,
  outputDir: string,
  incremental = true
): Promise<ImportResult> {
  return runImportPipeline({
    mode: incremental ? 'incremental' : 'full',
    sourceDir,
    outputDir,
    generateSourceNodes: true,
    generateDerivedNodes: true,
    verbose: true,
  });
}
