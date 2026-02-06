/**
 * Import Command Handlers
 *
 * Pure business logic for import-related CLI commands.
 * Handlers accept services via dependency injection for testability.
 */

import * as path from 'path';
import { ServiceContainer } from '../service-container';

/**
 * Options for import command
 */
export interface ImportOptions {
  source: string;
  db: string;
  full: boolean;
  verbose: boolean;
  dryRun: boolean;
  skipRelationships: boolean;
}

/**
 * Handle import command
 */
export async function handleImport(
  options: ImportOptions,
  container: ServiceContainer
): Promise<void> {
  const sourceDir = path.resolve(options.source);
  const dbPath = path.resolve(options.db);

  container.console.log('Elohim Content Import');
  container.console.log('=====================');
  container.console.log(`Source: ${sourceDir}`);
  container.console.log(`Database: ${dbPath}`);
  container.console.log(`Mode: ${options.full ? 'Full' : 'Incremental'}`);
  if (options.skipRelationships) {
    container.console.log('Relationships: SKIPPED');
  }
  container.console.log('');

  try {
    const result = await container.importPipeline.runImportPipeline({
      mode: options.full ? 'full' : 'incremental',
      sourceDir,
      outputDir: path.dirname(dbPath),
      dbPath,
      verbose: options.verbose,
      dryRun: options.dryRun,
      generateSourceNodes: true,
      generateDerivedNodes: true,
      skipRelationships: options.skipRelationships,
    });

    if (result.errors === 0) {
      container.console.log('\n✓ Import completed successfully');
      container.console.log(`  Files processed: ${result.created}`);
      container.console.log(`  Files skipped: ${result.skipped}`);
      container.console.log(`  Nodes generated: ${result.totalNodes}`);
      container.console.log(`  Relationships: ${result.totalRelationships}`);
    } else {
      container.console.error(`\n✗ Import completed with ${result.errors} errors:`);
      for (const fileResult of result.fileResults.filter((r) => r.status === 'error')) {
        container.console.error(`  - ${fileResult.sourcePath}: ${fileResult.error}`);
      }
      container.process.exit(1);
    }
  } catch (err) {
    container.console.error(`\n✗ Import failed: ${err}`);
    container.process.exit(1);
  }
}

/**
 * Options for stats command
 */
export interface StatsOptions {
  output: string;
}

/**
 * Handle stats command
 */
export async function handleStats(
  options: StatsOptions,
  container: ServiceContainer
): Promise<void> {
  const outputDir = path.resolve(options.output);

  try {
    const manifest = container.manifest.loadManifest(outputDir);
    const stats = container.manifest.getImportStats(manifest);
    const validation = container.manifest.validateManifest(manifest);

    container.console.log('Import Statistics');
    container.console.log('=================');
    container.console.log(`Schema version: ${stats.schemaVersion}`);
    container.console.log(`Last import: ${stats.lastImport}`);
    container.console.log(`Total sources: ${stats.totalSources}`);
    container.console.log(`Total nodes: ${stats.totalNodes}`);
    container.console.log(`Migrations: ${stats.migrationCount}`);
    container.console.log('');
    container.console.log(`Manifest valid: ${validation.valid ? 'Yes' : 'No'}`);

    if (!validation.valid) {
      container.console.log('Validation errors:');
      for (const error of validation.errors) {
        container.console.log(`  - ${error}`);
      }
    }
  } catch (err) {
    container.console.error(`Failed to load manifest: ${err}`);
    container.process.exit(1);
  }
}

/**
 * Options for validate command
 */
export interface ValidateOptions {
  output: string;
}

/**
 * Handle validate command
 */
export async function handleValidate(
  options: ValidateOptions,
  container: ServiceContainer
): Promise<void> {
  const outputDir = path.resolve(options.output);

  try {
    const manifest = container.manifest.loadManifest(outputDir);
    const validation = container.manifest.validateManifest(manifest);

    if (validation.valid) {
      container.console.log('✓ Manifest is valid');
    } else {
      container.console.log('✗ Manifest has errors:');
      for (const error of validation.errors) {
        container.console.log(`  - ${error}`);
      }
      container.process.exit(1);
    }
  } catch (err) {
    container.console.error(`Failed to validate: ${err}`);
    container.process.exit(1);
  }
}

/**
 * Options for explore command
 */
export interface ExploreOptions {
  output: string;
  node?: string;
  epic?: string;
  userType?: string;
  type?: string;
  depth: string;
  limit: string;
}

/**
 * Handle explore command
 */
export async function handleExplore(
  options: ExploreOptions,
  container: ServiceContainer
): Promise<void> {
  const outputDir = path.resolve(options.output);
  const nodesPath = path.join(outputDir, 'nodes.json');

  try {
    if (!container.fs.existsSync(nodesPath)) {
      container.console.error('No nodes.json found. Run import first.');
      container.process.exit(1);
    }

    const allNodes = JSON.parse(container.fs.readFileSync(nodesPath, 'utf-8'));
    let filteredNodes = allNodes;

    // Apply filters
    if (options.epic) {
      filteredNodes = filteredNodes.filter((n: any) => n.metadata?.epic === options.epic);
      container.console.log(`Filtered to epic: ${options.epic}`);
    }

    if (options.userType) {
      filteredNodes = filteredNodes.filter((n: any) => n.metadata?.userType === options.userType);
      container.console.log(`Filtered to user type: ${options.userType}`);
    }

    if (options.type) {
      filteredNodes = filteredNodes.filter((n: any) => n.contentType === options.type);
      container.console.log(`Filtered to content type: ${options.type}`);
    }

    // Apply limit
    const limit = parseInt(options.limit, 10);
    filteredNodes = filteredNodes.slice(0, limit);

    container.console.log(`\nFound ${filteredNodes.length} nodes:\n`);

    // Display summary by type
    const byType: Record<string, number> = {};
    for (const node of filteredNodes) {
      byType[node.contentType] = (byType[node.contentType] || 0) + 1;
    }

    container.console.log('By content type:');
    for (const [type, count] of Object.entries(byType).sort((a, b) => b[1] - a[1])) {
      container.console.log(`  ${type}: ${count}`);
    }

    // If exploring a specific node
    if (options.node) {
      const targetNode = allNodes.find((n: any) => n.id === options.node);
      if (!targetNode) {
        container.console.error(`Node not found: ${options.node}`);
        container.process.exit(1);
      }

      container.console.log(`\nNode: ${targetNode.id}`);
      container.console.log(`  Title: ${targetNode.title}`);
      container.console.log(`  Type: ${targetNode.contentType}`);
      container.console.log(`  Tags: ${(targetNode.tags || []).join(', ')}`);
      container.console.log(`  Epic: ${targetNode.metadata?.epic || 'none'}`);
      container.console.log(`  User Type: ${targetNode.metadata?.userType || 'none'}`);

      // Show related nodes
      const relatedIds = targetNode.relatedNodeIds || [];
      if (relatedIds.length > 0) {
        container.console.log(`\n  Related nodes (${relatedIds.length}):`);
        for (const relId of relatedIds.slice(0, 10)) {
          const related = allNodes.find((n: any) => n.id === relId);
          if (related) {
            container.console.log(`    - ${relId}: ${related.title} (${related.contentType})`);
          } else {
            container.console.log(`    - ${relId}: [not found]`);
          }
        }
        if (relatedIds.length > 10) {
          container.console.log(`    ... and ${relatedIds.length - 10} more`);
        }
      }
    }

    // Show sample nodes
    container.console.log('\nSample nodes:');
    for (const node of filteredNodes.slice(0, 10)) {
      container.console.log(`  ${node.id}`);
      container.console.log(`    ${node.title} (${node.contentType})`);
    }
  } catch (err) {
    container.console.error(`Explore failed: ${err}`);
    container.process.exit(1);
  }
}

/**
 * Options for validate-standards command
 */
export interface ValidateStandardsOptions {
  output: string;
}

/**
 * Handle validate-standards command
 */
export async function handleValidateStandards(
  options: ValidateStandardsOptions,
  container: ServiceContainer
): Promise<void> {
  const outputDir = path.resolve(options.output);
  const nodesPath = path.join(outputDir, 'nodes.json');

  try {
    if (!container.fs.existsSync(nodesPath)) {
      container.console.error('No nodes.json found. Run import first.');
      container.process.exit(1);
    }

    const nodes = JSON.parse(container.fs.readFileSync(nodesPath, 'utf-8'));
    const report = container.standards.generateCoverageReport(nodes);

    container.console.log('\n' + '='.repeat(60));
    container.console.log('STANDARDS ALIGNMENT COVERAGE REPORT');
    container.console.log('='.repeat(60));
    container.console.log(`\nTotal content nodes analyzed: ${report.total}\n`);

    container.console.log('Field Coverage:');
    container.console.log('-'.repeat(60));

    const targets: Record<string, number> = {
      did: 100,
      activityPubType: 100,
      linkedData: 80,
      openGraphMetadata: 80,
    };

    for (const [field, data] of Object.entries(report.coverage)) {
      const target = targets[field] || 0;
      let status = '✗';
      let label = 'POOR';

      if (data.percentage >= 95) {
        status = '✓';
        label = 'EXCELLENT';
      } else if (data.percentage >= 80) {
        status = '✓';
        label = 'GOOD';
      } else if (data.percentage >= 50) {
        status = '⚠';
        label = 'NEEDS IMPROVEMENT';
      }

      container.console.log(
        `${status} ${field.padEnd(25)} ${data.count}/${data.total} (${data.percentage.toFixed(
          1
        )}%) - ${label}`
      );
    }

    if (report.errors.length > 0) {
      container.console.log(`\n⚠ Validation Errors Found: ${report.errors.length}`);
      container.console.log('-'.repeat(60));
      for (const error of report.errors.slice(0, 20)) {
        container.console.log(`  • ${error}`);
      }
      if (report.errors.length > 20) {
        container.console.log(`  ... and ${report.errors.length - 20} more errors`);
      }
    }

    container.console.log('\n' + '='.repeat(60));
    if (report.allTargetsMet && report.errors.length === 0) {
      container.console.log('STATUS: ✓ All targets met! Standards alignment is excellent.');
    } else if (report.allTargetsMet) {
      container.console.log('STATUS: ⚠ Coverage targets met, but validation errors found.');
    } else {
      container.console.log('STATUS: ✗ Some coverage targets not met. Review import settings.');
    }
    container.console.log('='.repeat(60) + '\n');

    if (!report.allTargetsMet || report.errors.length > 0) {
      container.process.exit(1);
    }
  } catch (err) {
    container.console.error(`Validate standards failed: ${err}`);
    container.process.exit(1);
  }
}

/**
 * Options for enrich-trust command
 */
export interface EnrichTrustOptions {
  output: string;
  attestations: string;
}

/**
 * Handle enrich-trust command
 */
export async function handleEnrichTrust(
  options: EnrichTrustOptions,
  container: ServiceContainer
): Promise<void> {
  const contentDir = path.resolve(options.output, 'content');
  const attestationsPath = path.resolve(options.attestations);

  container.console.log('Trust Enrichment');
  container.console.log('================');
  container.console.log(`Content directory: ${contentDir}`);
  container.console.log(`Attestations file: ${attestationsPath}`);
  container.console.log('');

  try {
    const result = await container.trust.enrichContentDirectory(contentDir, attestationsPath);

    container.console.log(`\nProcessed ${result.processed} content files`);
    container.console.log(`Enriched: ${result.enriched}`);
    container.console.log(`With attestations: ${result.withAttestations}`);

    if (result.errors.length > 0) {
      container.console.log(`\nErrors: ${result.errors.length}`);
      for (const error of result.errors) {
        container.console.log(`  - ${error}`);
      }
      container.process.exit(1);
    }

    // Update content index
    const indexPath = path.join(contentDir, 'index.json');
    const attestationsByContent = container.trust.loadAttestations(attestationsPath);
    container.trust.updateContentIndexWithTrust(indexPath, attestationsByContent);

    container.console.log('\n✓ Trust enrichment complete');
  } catch (err) {
    container.console.error(`Trust enrichment failed: ${err}`);
    container.process.exit(1);
  }
}
