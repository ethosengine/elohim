#!/usr/bin/env node
/**
 * Content Import CLI
 *
 * Command-line interface for running content imports.
 *
 * Usage:
 *   npx ts-node src/cli/import.ts import --source ./docs/content --output ./output
 *   npx ts-node src/cli/import.ts import --full  # Full reimport
 *   npx ts-node src/cli/import.ts stats          # Show import statistics
 *
 * Memory:
 *   For large imports, increase Node.js memory:
 *   NODE_OPTIONS="--max-old-space-size=4096" npx ts-node src/cli/import.ts import
 */

import * as fs from 'fs';
import * as path from 'path';

import { Command } from 'commander';

import { KuzuClient } from '../db/kuzu-client';
import {
  createHuman,
  createRelationship,
  addHumanToFile,
  addRelationshipToFile,
  importHumansToLamad,
  listHumanCategories,
  listRelationshipTypes,
  HumanCategory,
} from '../services/human.service';
import { runImportPipeline } from '../services/import-pipeline.service';
import { loadManifest, getImportStats, validateManifest } from '../services/manifest.service';
import {
  scaffoldUserType,
  scaffoldEpic,
  scaffoldAll,
  listEpicsAndUsers,
} from '../services/scaffold.service';
import { generateCoverageReport } from '../services/standards.service';
import {
  loadAttestations,
  enrichContentDirectory,
  updateContentIndexWithTrust,
} from '../services/trust.service';

const program = new Command();

program
  .name('elohim-import')
  .description('Import content from source files into lamad ContentNodes')
  .version('1.0.0');

program
  .command('import')
  .description('Import content from source files into Kuzu database')
  .option('-s, --source <dir>', 'Source content directory', './docs/content')
  .option('-d, --db <file>', 'Kuzu database path', './output/lamad.kuzu')
  .option('-f, --full', 'Force full reimport (ignore incremental cache)', false)
  .option('-v, --verbose', 'Verbose output', false)
  .option('--dry-run', 'Dry run - do not write to database', false)
  .option('--skip-relationships', 'Skip relationship extraction (faster, less memory)', false)
  .action(async options => {
    const sourceDir = path.resolve(options.source);
    const dbPath = path.resolve(options.db);

    console.log('Elohim Content Import');
    console.log('=====================');
    console.log(`Source: ${sourceDir}`);
    console.log(`Database: ${dbPath}`);
    console.log(`Mode: ${options.full ? 'Full' : 'Incremental'}`);
    if (options.skipRelationships) {
      console.log('Relationships: SKIPPED');
    }
    console.log('');

    try {
      const result = await runImportPipeline({
        mode: options.full ? 'full' : 'incremental',
        sourceDir,
        outputDir: path.dirname(dbPath), // For manifest storage
        dbPath,
        verbose: options.verbose,
        dryRun: options.dryRun,
        generateSourceNodes: true,
        generateDerivedNodes: true,
        skipRelationships: options.skipRelationships,
      });

      if (result.errors === 0) {
        console.log('\n✓ Import completed successfully');
        console.log(`  Files processed: ${result.created}`);
        console.log(`  Files skipped: ${result.skipped}`);
        console.log(`  Nodes generated: ${result.totalNodes}`);
        console.log(`  Relationships: ${result.totalRelationships}`);
      } else {
        console.error(`\n✗ Import completed with ${result.errors} errors:`);
        for (const fileResult of result.fileResults.filter(r => r.status === 'error')) {
          console.error(`  - ${fileResult.sourcePath}: ${fileResult.error}`);
        }
        process.exit(1);
      }
    } catch (err) {
      console.error(`\n✗ Import failed: ${err}`);
      process.exit(1);
    }
  });

program
  .command('stats')
  .description('Show import statistics from manifest')
  .option('-o, --output <dir>', 'Output directory', './output/lamad')
  .action(options => {
    const outputDir = path.resolve(options.output);

    try {
      const manifest = loadManifest(outputDir);
      const stats = getImportStats(manifest);
      const validation = validateManifest(manifest);

      console.log('Import Statistics');
      console.log('=================');
      console.log(`Schema version: ${stats.schemaVersion}`);
      console.log(`Last import: ${stats.lastImport}`);
      console.log(`Total sources: ${stats.totalSources}`);
      console.log(`Total nodes: ${stats.totalNodes}`);
      console.log(`Migrations: ${stats.migrationCount}`);
      console.log('');
      console.log(`Manifest valid: ${validation.valid ? 'Yes' : 'No'}`);

      if (!validation.valid) {
        console.log('Validation errors:');
        for (const error of validation.errors) {
          console.log(`  - ${error}`);
        }
      }
    } catch (err) {
      console.error(`Failed to load manifest: ${err}`);
      process.exit(1);
    }
  });

program
  .command('validate')
  .description('Validate manifest integrity')
  .option('-o, --output <dir>', 'Output directory', './output/lamad')
  .action(options => {
    const outputDir = path.resolve(options.output);

    try {
      const manifest = loadManifest(outputDir);
      const validation = validateManifest(manifest);

      if (validation.valid) {
        console.log('✓ Manifest is valid');
      } else {
        console.log('✗ Manifest has errors:');
        for (const error of validation.errors) {
          console.log(`  - ${error}`);
        }
        process.exit(1);
      }
    } catch (err) {
      console.error(`Failed to validate: ${err}`);
      process.exit(1);
    }
  });

program
  .command('explore')
  .description('Explore relationships for a specific node or scope')
  .option('-o, --output <dir>', 'Output directory with nodes.json', './output/lamad')
  .option('-n, --node <id>', 'Explore relationships for a specific node ID')
  .option(
    '-e, --epic <name>',
    'Explore nodes within a specific epic (governance, autonomous_entity, etc.)'
  )
  .option(
    '-u, --user-type <type>',
    'Explore nodes for a specific user type (policy_maker, worker, etc.)'
  )
  .option('-t, --type <contentType>', 'Filter by content type (scenario, role, epic, etc.)')
  .option('--depth <n>', 'Relationship depth to explore (default: 1)', '1')
  .option('--limit <n>', 'Maximum nodes to return (default: 50)', '50')
  .action(async options => {
    const outputDir = path.resolve(options.output);
    const nodesPath = path.join(outputDir, 'nodes.json');

    try {
      // Load nodes
      const fs = await import('fs');
      if (!fs.existsSync(nodesPath)) {
        console.error('No nodes.json found. Run import first.');
        process.exit(1);
      }

      const allNodes = JSON.parse(fs.readFileSync(nodesPath, 'utf-8'));
      let filteredNodes = allNodes;

      // Apply filters
      if (options.epic) {
        filteredNodes = filteredNodes.filter((n: any) => n.metadata?.epic === options.epic);
        console.log(`Filtered to epic: ${options.epic}`);
      }

      if (options.userType) {
        filteredNodes = filteredNodes.filter((n: any) => n.metadata?.userType === options.userType);
        console.log(`Filtered to user type: ${options.userType}`);
      }

      if (options.type) {
        filteredNodes = filteredNodes.filter((n: any) => n.contentType === options.type);
        console.log(`Filtered to content type: ${options.type}`);
      }

      // Apply limit
      const limit = Number.parseInt(options.limit, 10);
      filteredNodes = filteredNodes.slice(0, limit);

      console.log(`\nFound ${filteredNodes.length} nodes:\n`);

      // Display summary
      const byType: Record<string, number> = {};
      for (const node of filteredNodes) {
        byType[node.contentType] = (byType[node.contentType] || 0) + 1;
      }

      console.log('By content type:');
      for (const [type, count] of Object.entries(byType).sort((a, b) => b[1] - a[1])) {
        console.log(`  ${type}: ${count}`);
      }

      // If exploring a specific node
      if (options.node) {
        const targetNode = allNodes.find((n: any) => n.id === options.node);
        if (!targetNode) {
          console.error(`Node not found: ${options.node}`);
          process.exit(1);
        }

        console.log(`\nNode: ${targetNode.id}`);
        console.log(`  Title: ${targetNode.title}`);
        console.log(`  Type: ${targetNode.contentType}`);
        console.log(`  Tags: ${(targetNode.tags || []).join(', ')}`);
        console.log(`  Epic: ${targetNode.metadata?.epic || 'none'}`);
        console.log(`  User Type: ${targetNode.metadata?.userType || 'none'}`);

        // Show related nodes
        const relatedIds = targetNode.relatedNodeIds || [];
        if (relatedIds.length > 0) {
          console.log(`\n  Related nodes (${relatedIds.length}):`);
          for (const relId of relatedIds.slice(0, 10)) {
            const related = allNodes.find((n: any) => n.id === relId);
            if (related) {
              console.log(`    - ${relId}: ${related.title} (${related.contentType})`);
            } else {
              console.log(`    - ${relId}: [not found]`);
            }
          }
          if (relatedIds.length > 10) {
            console.log(`    ... and ${relatedIds.length - 10} more`);
          }
        }
      }

      // Show sample nodes
      console.log('\nSample nodes:');
      for (const node of filteredNodes.slice(0, 10)) {
        console.log(`  ${node.id}`);
        console.log(`    ${node.title} (${node.contentType})`);
      }
    } catch (err) {
      console.error(`Explore failed: ${err}`);
      process.exit(1);
    }
  });

program
  .command('list-epics')
  .description('List all unique epics in the imported content')
  .option('-o, --output <dir>', 'Output directory with nodes.json', './output/lamad')
  .action(async options => {
    const outputDir = path.resolve(options.output);
    const nodesPath = path.join(outputDir, 'nodes.json');

    try {
      const fs = await import('fs');
      if (!fs.existsSync(nodesPath)) {
        console.error('No nodes.json found. Run import first.');
        process.exit(1);
      }

      const nodes = JSON.parse(fs.readFileSync(nodesPath, 'utf-8'));

      // Collect epics with counts
      const epicCounts: Record<string, { total: number; byType: Record<string, number> }> = {};

      for (const node of nodes) {
        const epic = node.metadata?.epic || 'other';
        if (!epicCounts[epic]) {
          epicCounts[epic] = { total: 0, byType: {} };
        }
        epicCounts[epic].total++;
        epicCounts[epic].byType[node.contentType] =
          (epicCounts[epic].byType[node.contentType] || 0) + 1;
      }

      console.log('Epics in imported content:\n');
      for (const [epic, data] of Object.entries(epicCounts).sort(
        (a, b) => b[1].total - a[1].total
      )) {
        console.log(`${epic}: ${data.total} nodes`);
        for (const [type, count] of Object.entries(data.byType).sort((a, b) => b[1] - a[1])) {
          console.log(`  ${type}: ${count}`);
        }
        console.log('');
      }
    } catch (err) {
      console.error(`List epics failed: ${err}`);
      process.exit(1);
    }
  });

program
  .command('list-user-types')
  .description('List all unique user types/archetypes in the imported content')
  .option('-o, --output <dir>', 'Output directory with nodes.json', './output/lamad')
  .option('-e, --epic <name>', 'Filter to specific epic')
  .action(async options => {
    const outputDir = path.resolve(options.output);
    const nodesPath = path.join(outputDir, 'nodes.json');

    try {
      const fs = await import('fs');
      if (!fs.existsSync(nodesPath)) {
        console.error('No nodes.json found. Run import first.');
        process.exit(1);
      }

      let nodes = JSON.parse(fs.readFileSync(nodesPath, 'utf-8'));

      if (options.epic) {
        nodes = nodes.filter((n: any) => n.metadata?.epic === options.epic);
        console.log(`Filtering to epic: ${options.epic}\n`);
      }

      // Collect user types
      const userTypeCounts: Record<string, { total: number; epics: Set<string> }> = {};

      for (const node of nodes) {
        const userType = node.metadata?.userType;
        if (!userType) continue;

        if (!userTypeCounts[userType]) {
          userTypeCounts[userType] = { total: 0, epics: new Set() };
        }
        userTypeCounts[userType].total++;
        if (node.metadata?.epic) {
          userTypeCounts[userType].epics.add(node.metadata.epic);
        }
      }

      console.log('User types/Archetypes:\n');
      for (const [userType, data] of Object.entries(userTypeCounts).sort(
        (a, b) => b[1].total - a[1].total
      )) {
        console.log(`${userType}: ${data.total} nodes`);
        console.log(`  Epics: ${Array.from(data.epics).join(', ')}`);
        console.log('');
      }
    } catch (err) {
      console.error(`List user types failed: ${err}`);
      process.exit(1);
    }
  });

// ============================================================================
// STANDARDS VALIDATION COMMANDS
// ============================================================================

program
  .command('validate-standards')
  .description('Validate standards alignment (DID, JSON-LD, Open Graph)')
  .option('-o, --output <dir>', 'Content directory with nodes.json', './output/lamad')
  .action(async options => {
    const outputDir = path.resolve(options.output);
    const nodesPath = path.join(outputDir, 'nodes.json');

    try {
      if (!fs.existsSync(nodesPath)) {
        console.error('No nodes.json found. Run import first.');
        process.exit(1);
      }

      const nodes = JSON.parse(fs.readFileSync(nodesPath, 'utf-8'));
      const report = generateCoverageReport(nodes);

      console.log('\n' + '='.repeat(60));
      console.log('STANDARDS ALIGNMENT COVERAGE REPORT');
      console.log('='.repeat(60));
      console.log(`\nTotal content nodes analyzed: ${report.total}\n`);

      console.log('Field Coverage:');
      console.log('-'.repeat(60));

      const targets: Record<string, number> = {
        did: 100,
        activityPubType: 100,
        linkedData: 80,
        openGraphMetadata: 80,
      };

      for (const [field, data] of Object.entries(report.coverage)) {
        const _target = targets[field] || 0;
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

        console.log(
          `${status} ${field.padEnd(25)} ${data.count}/${data.total} (${data.percentage.toFixed(1)}%) - ${label}`
        );
      }

      if (report.errors.length > 0) {
        console.log(`\n⚠ Validation Errors Found: ${report.errors.length}`);
        console.log('-'.repeat(60));
        for (const error of report.errors.slice(0, 20)) {
          console.log(`  • ${error}`);
        }
        if (report.errors.length > 20) {
          console.log(`  ... and ${report.errors.length - 20} more errors`);
        }
      }

      console.log('\n' + '='.repeat(60));
      if (report.allTargetsMet && report.errors.length === 0) {
        console.log('STATUS: ✓ All targets met! Standards alignment is excellent.');
      } else if (report.allTargetsMet) {
        console.log('STATUS: ⚠ Coverage targets met, but validation errors found.');
      } else {
        console.log('STATUS: ✗ Some coverage targets not met. Review import settings.');
      }
      console.log('='.repeat(60) + '\n');

      if (!report.allTargetsMet || report.errors.length > 0) {
        process.exit(1);
      }
    } catch (err) {
      console.error(`Validate standards failed: ${err}`);
      process.exit(1);
    }
  });

// ============================================================================
// TRUST ENRICHMENT COMMANDS
// ============================================================================

program
  .command('enrich-trust')
  .description('Enrich content with trust scores from attestations')
  .option('-o, --output <dir>', 'Content directory', './output/lamad')
  .option(
    '-a, --attestations <file>',
    'Attestations index file',
    './output/lamad/attestations/index.json'
  )
  .action(async options => {
    const contentDir = path.resolve(options.output, 'content');
    const attestationsPath = path.resolve(options.attestations);

    console.log('Trust Enrichment');
    console.log('================');
    console.log(`Content directory: ${contentDir}`);
    console.log(`Attestations file: ${attestationsPath}`);
    console.log('');

    try {
      const result = await enrichContentDirectory(contentDir, attestationsPath);

      console.log(`\nProcessed ${result.processed} content files`);
      console.log(`Enriched: ${result.enriched}`);
      console.log(`With attestations: ${result.withAttestations}`);

      if (result.errors.length > 0) {
        console.log(`\nErrors: ${result.errors.length}`);
        for (const error of result.errors) {
          console.log(`  - ${error}`);
        }
        process.exit(1);
      }

      // Update content index
      const indexPath = path.join(contentDir, 'index.json');
      const attestationsByContent = loadAttestations(attestationsPath);
      updateContentIndexWithTrust(indexPath, attestationsByContent);

      console.log('\n✓ Trust enrichment complete');
    } catch (err) {
      console.error(`Trust enrichment failed: ${err}`);
      process.exit(1);
    }
  });

// ============================================================================
// SCAFFOLD COMMANDS
// ============================================================================

program
  .command('scaffold')
  .description('Generate README and TODO templates for user types')
  .option(
    '-b, --base <dir>',
    'Base content directory',
    '/projects/elohim/docs/content/elohim-protocol'
  )
  .option('-e, --epic <name>', 'Epic to scaffold (governance, value_scanner, etc.)')
  .option('-u, --user <name>', 'User type to scaffold')
  .option('--all', 'Scaffold all epics and user types', false)
  .option('--list', 'List available epics and user types', false)
  .action(options => {
    if (options.list) {
      console.log('Available Epics and User Types:\n');
      for (const { epic, description, users } of listEpicsAndUsers()) {
        console.log(`${epic}: ${description}`);
        for (const user of users) {
          console.log(`  - ${user}`);
        }
        console.log('');
      }
      return;
    }

    const basePath = path.resolve(options.base);
    console.log('Scaffold Templates');
    console.log('==================');
    console.log(`Base path: ${basePath}`);

    let result;

    if (options.all) {
      console.log('Scaffolding all epics and user types...\n');
      result = scaffoldAll(basePath);
    } else if (options.epic && options.user) {
      console.log(`Scaffolding ${options.epic}/${options.user}...\n`);
      result = scaffoldUserType(basePath, options.epic, options.user);
    } else if (options.epic) {
      console.log(`Scaffolding all user types for ${options.epic}...\n`);
      result = scaffoldEpic(basePath, options.epic);
    } else {
      console.log('Please specify --epic, --user, --all, or --list');
      process.exit(1);
    }

    console.log(`Created: ${result.created.length} files`);
    for (const file of result.created) {
      console.log(`  ✓ ${file}`);
    }

    console.log(`\nSkipped: ${result.skipped.length} files (already exist)`);

    if (result.errors.length > 0) {
      console.log(`\nErrors: ${result.errors.length}`);
      for (const error of result.errors) {
        console.log(`  ✗ ${error}`);
      }
    }
  });

// ============================================================================
// HUMAN NETWORK COMMANDS
// ============================================================================

program
  .command('add-human')
  .description('Add a human to the network')
  .option('-f, --file <path>', 'Humans JSON file', '/projects/elohim/data/humans/humans.json')
  .option('--name <name>', 'Display name')
  .option('--id <id>', 'Human ID (without human- prefix)')
  .option('--bio <bio>', 'Biography')
  .option('--category <cat>', 'Category (community, workplace, etc.)')
  .option('--location <name>', 'Location name')
  .option('--layer <layer>', 'Governance layer')
  .option('--affinities <list>', 'Comma-separated affinities')
  .option('--list-categories', 'List available categories', false)
  .action(options => {
    if (options.listCategories) {
      console.log('Available categories:');
      for (const cat of listHumanCategories()) {
        console.log(`  - ${cat}`);
      }
      return;
    }

    if (!options.name || !options.id || !options.bio || !options.category) {
      console.error('Required: --name, --id, --bio, --category');
      console.log('\nExample:');
      console.log('  npx ts-node src/cli/import.ts add-human \\');
      console.log('    --name "Alice" --id "alice-activist" \\');
      console.log('    --bio "Community organizer" --category community');
      process.exit(1);
    }

    const filePath = path.resolve(options.file);

    try {
      const human = createHuman({
        id: options.id,
        displayName: options.name,
        bio: options.bio,
        category: options.category as HumanCategory,
        location:
          options.location && options.layer
            ? {
                layer: options.layer,
                name: options.location,
              }
            : undefined,
        affinities: options.affinities?.split(',').map((s: string) => s.trim()),
      });

      addHumanToFile(filePath, human);
      console.log(`✓ Added human: ${human.id}`);
      console.log(`  Name: ${human.displayName}`);
      console.log(`  Category: ${human.category}`);
    } catch (err) {
      console.error(`Failed to add human: ${err}`);
      process.exit(1);
    }
  });

program
  .command('add-relationship')
  .description('Add relationship between humans')
  .option('-f, --file <path>', 'Humans JSON file', '/projects/elohim/data/humans/humans.json')
  .option('--from <id>', 'Source human ID')
  .option('--to <id>', 'Target human ID')
  .option('--type <type>', 'Relationship type (neighbor, coworker, etc.)')
  .option('--intimacy <level>', 'Intimacy level (intimate, trusted, connection, recognition)')
  .option('--context <orgId>', 'Context organization ID')
  .option('--list-types', 'List available relationship types', false)
  .action(options => {
    if (options.listTypes) {
      console.log('Available relationship types:');
      for (const { type, layer, intimacy } of listRelationshipTypes()) {
        console.log(`  ${type.padEnd(25)} layer: ${layer.padEnd(15)} intimacy: ${intimacy}`);
      }
      return;
    }

    if (!options.from || !options.to || !options.type) {
      console.error('Required: --from, --to, --type');
      console.log('\nExample:');
      console.log('  npx ts-node src/cli/import.ts add-relationship \\');
      console.log('    --from alice-activist --to bob-baker --type neighbor');
      process.exit(1);
    }

    const filePath = path.resolve(options.file);

    try {
      const relationship = createRelationship({
        sourceId: options.from,
        targetId: options.to,
        relationshipType: options.type,
        intimacy: options.intimacy,
        contextOrgId: options.context,
      });

      addRelationshipToFile(filePath, relationship);
      console.log(`✓ Added relationship: ${relationship.sourceId} → ${relationship.targetId}`);
      console.log(`  Type: ${relationship.relationshipType}`);
      console.log(`  Intimacy: ${relationship.intimacy}`);
      console.log(`  Layer: ${relationship.layer}`);
    } catch (err) {
      console.error(`Failed to add relationship: ${err}`);
      process.exit(1);
    }
  });

program
  .command('import-humans')
  .description('Import humans and relationships from data/humans/')
  .option('-s, --source <file>', 'Humans JSON file', '/projects/elohim/data/humans/humans.json')
  .option('-o, --output <dir>', 'Output directory', './output/lamad')
  .action(async options => {
    const sourcePath = path.resolve(options.source);
    const outputDir = path.resolve(options.output);

    console.log('Import Humans');
    console.log('=============');
    console.log(`Source: ${sourcePath}`);
    console.log(`Output: ${outputDir}`);
    console.log('');

    try {
      const result = await importHumansToLamad(sourcePath, outputDir);

      console.log(`\nHumans imported: ${result.humansImported}`);
      console.log(`Relationships imported: ${result.relationshipsImported}`);

      if (result.errors.length > 0) {
        console.log(`\nErrors: ${result.errors.length}`);
        for (const error of result.errors) {
          console.log(`  - ${error}`);
        }
        process.exit(1);
      }

      console.log('\n✓ Human import complete');
    } catch (err) {
      console.error(`Human import failed: ${err}`);
      process.exit(1);
    }
  });

// ============================================================================
// LEARNING PATH GENERATION COMMANDS
// ============================================================================

program
  .command('generate-path')
  .description('Generate a custom learning path from imported content')
  .option('-o, --output <dir>', 'Output directory with nodes.json', './output/lamad')
  .option('--id <id>', 'Learning path ID (kebab-case)')
  .option('--title <title>', 'Learning path title')
  .option('--description <desc>', 'Learning path description')
  .option('--purpose <purpose>', 'Why learners should follow this path')
  .option('-e, --epic <name>', 'Filter content to specific epic')
  .option('-u, --user-type <type>', 'Filter content to specific user type')
  .option(
    '-t, --type <contentType>',
    'Content types to include (comma-separated)',
    'scenario,role,epic'
  )
  .option(
    '--difficulty <level>',
    'Difficulty level (beginner, intermediate, advanced)',
    'intermediate'
  )
  .option('--max-steps <n>', 'Maximum number of steps', '10')
  .option('--chapters', 'Organize into chapters by content type', false)
  .option('--dry-run', 'Preview path without writing', false)
  .action(async options => {
    const outputDir = path.resolve(options.output);
    const nodesPath = path.join(outputDir, 'nodes.json');
    const pathsDir = path.join(outputDir, 'paths');

    // Validate required options
    if (!options.id || !options.title) {
      console.error('Required: --id and --title');
      console.log('\nExample:');
      console.log('  npx ts-node src/cli/import.ts generate-path \\');
      console.log('    --id governance-intro \\');
      console.log('    --title "Introduction to AI Governance" \\');
      console.log('    --epic governance \\');
      console.log('    --user-type policy_maker \\');
      console.log('    --max-steps 8');
      process.exit(1);
    }

    try {
      if (!fs.existsSync(nodesPath)) {
        console.error('No nodes.json found. Run import first.');
        process.exit(1);
      }

      const allNodes = JSON.parse(fs.readFileSync(nodesPath, 'utf-8'));
      let filteredNodes = allNodes;

      // Apply filters
      if (options.epic) {
        filteredNodes = filteredNodes.filter((n: any) => n.metadata?.epic === options.epic);
      }

      if (options.userType) {
        filteredNodes = filteredNodes.filter((n: any) => n.metadata?.userType === options.userType);
      }

      const contentTypes = new Set(options.type.split(',').map((t: string) => t.trim()));
      filteredNodes = filteredNodes.filter((n: any) => contentTypes.has(n.contentType));

      // Sort by content type priority and then by title
      const typePriority: Record<string, number> = {
        epic: 1,
        role: 2,
        concept: 3,
        scenario: 4,
        example: 5,
        reference: 6,
      };

      filteredNodes.sort((a: any, b: any) => {
        const priorityA = typePriority[a.contentType] || 99;
        const priorityB = typePriority[b.contentType] || 99;
        if (priorityA !== priorityB) return priorityA - priorityB;
        return (a.title || '').localeCompare(b.title || '');
      });

      // Limit steps
      const maxSteps = Number.parseInt(options.maxSteps, 10);
      filteredNodes = filteredNodes.slice(0, maxSteps);

      if (filteredNodes.length === 0) {
        console.error(
          'No content found matching filters. Try different --epic, --user-type, or --type values.'
        );
        process.exit(1);
      }

      // Build learning path structure
      const now = new Date().toISOString();
      const pathId = options.id;

      let steps: any[] = [];
      let chapters: any[] | undefined = undefined;

      if (options.chapters) {
        // Group by content type
        const byType: Record<string, any[]> = {};
        for (const node of filteredNodes) {
          if (!byType[node.contentType]) byType[node.contentType] = [];
          byType[node.contentType].push(node);
        }

        chapters = [];
        let chapterOrder = 1;

        for (const [contentType, nodes] of Object.entries(byType).sort((a, b) => {
          return (typePriority[a[0]] || 99) - (typePriority[b[0]] || 99);
        })) {
          const chapterSteps = nodes.map((node: any, idx: number) => ({
            order: idx + 1,
            stepType: 'content',
            resourceId: node.id,
            stepTitle: node.title,
            stepNarrative: node.description || `Explore ${node.title}`,
            learningObjectives: [`Understand ${node.title}`],
            optional: false,
            completionCriteria: ['Review content'],
          }));

          chapters.push({
            id: `${pathId}-chapter-${chapterOrder}`,
            title: `${contentType.charAt(0).toUpperCase() + contentType.slice(1)}s`,
            description: `Explore ${contentType} content`,
            order: chapterOrder,
            steps: chapterSteps,
            estimatedDuration: `${chapterSteps.length * 10} minutes`,
          });

          chapterOrder++;
        }
      } else {
        // Flat steps
        steps = filteredNodes.map((node: any, idx: number) => ({
          order: idx + 1,
          stepType: 'content',
          resourceId: node.id,
          stepTitle: node.title,
          stepNarrative: node.description || `Explore ${node.title}`,
          learningObjectives: [`Understand ${node.title}`],
          optional: false,
          completionCriteria: ['Review content'],
        }));
      }

      // Flatten chapter steps into top-level steps array (required by PathService)
      const flattenedSteps = chapters ? chapters.flatMap((ch: any) => ch.steps) : steps;

      const learningPath = {
        id: pathId,
        version: '1.0.0',
        title: options.title,
        description:
          options.description ||
          `A learning path exploring ${options.epic || 'Elohim Protocol'} content`,
        purpose:
          options.purpose ||
          `To provide a structured introduction to ${options.epic || 'the Elohim Protocol'}`,
        createdBy: 'cli-generator',
        contributors: [],
        createdAt: now,
        updatedAt: now,
        steps: flattenedSteps,
        chapters: chapters,
        tags: [options.epic, options.userType, 'generated'].filter(Boolean),
        difficulty: options.difficulty,
        estimatedDuration: `${filteredNodes.length * 10} minutes`,
        visibility: 'public',
        pathType: 'journey',
      };

      // Display preview
      console.log('\nGenerated Learning Path');
      console.log('=======================');
      console.log(`ID: ${learningPath.id}`);
      console.log(`Title: ${learningPath.title}`);
      console.log(`Description: ${learningPath.description}`);
      console.log(`Difficulty: ${learningPath.difficulty}`);
      console.log(`Duration: ${learningPath.estimatedDuration}`);
      console.log(`Tags: ${learningPath.tags.join(', ')}`);

      if (chapters) {
        console.log(`\nChapters (${chapters.length}):`);
        for (const chapter of chapters) {
          console.log(`  ${chapter.order}. ${chapter.title} (${chapter.steps.length} steps)`);
          for (const step of chapter.steps.slice(0, 3)) {
            console.log(`      - ${step.stepTitle}`);
          }
          if (chapter.steps.length > 3) {
            console.log(`      ... and ${chapter.steps.length - 3} more`);
          }
        }
      } else {
        console.log(`\nSteps (${steps.length}):`);
        for (const step of steps.slice(0, 10)) {
          console.log(`  ${step.order}. ${step.stepTitle}`);
        }
        if (steps.length > 10) {
          console.log(`  ... and ${steps.length - 10} more`);
        }
      }

      if (options.dryRun) {
        console.log('\n[Dry run - no files written]');
      } else {
        // Write path file
        if (!fs.existsSync(pathsDir)) {
          fs.mkdirSync(pathsDir, { recursive: true });
        }

        const pathFile = path.join(pathsDir, `${pathId}.json`);
        fs.writeFileSync(pathFile, JSON.stringify(learningPath, null, 2));

        // Update paths index
        const indexFile = path.join(pathsDir, 'index.json');
        let pathIndex: any = { lastUpdated: now, totalCount: 0, paths: [] };

        if (fs.existsSync(indexFile)) {
          pathIndex = JSON.parse(fs.readFileSync(indexFile, 'utf-8'));
        }

        // Remove existing entry if updating
        pathIndex.paths = pathIndex.paths.filter((p: any) => p.id !== pathId);

        // Add new entry
        pathIndex.paths.push({
          id: pathId,
          title: learningPath.title,
          description: learningPath.description,
          difficulty: learningPath.difficulty,
          estimatedDuration: learningPath.estimatedDuration,
          stepCount: chapters
            ? chapters.reduce((sum: number, ch: any) => sum + ch.steps.length, 0)
            : steps.length,
          chapterCount: chapters?.length,
          tags: learningPath.tags,
          pathType: learningPath.pathType,
        });

        pathIndex.totalCount = pathIndex.paths.length;
        pathIndex.lastUpdated = now;

        fs.writeFileSync(indexFile, JSON.stringify(pathIndex, null, 2));

        console.log(`\n✓ Written: ${pathFile}`);
        console.log(`✓ Updated: ${indexFile}`);
      }
    } catch (err) {
      console.error(`Generate path failed: ${err}`);
      process.exit(1);
    }
  });

// =============================================================================
// DATABASE COMMANDS (Kuzu Graph Database)
// =============================================================================

program
  .command('db:init')
  .description('Initialize Kuzu database from existing JSON data')
  .option('-i, --input <dir>', 'Input lamad-data directory', './output/lamad')
  .option('-o, --output <file>', 'Output database file', './output/lamad.kuzu')
  .option('--force', 'Overwrite existing database', false)
  .action(async options => {
    const inputDir = path.resolve(options.input);
    const dbPath = path.resolve(options.output);

    console.log('Kuzu Database Initialization');
    console.log('============================');
    console.log(`Input: ${inputDir}`);
    console.log(`Output: ${dbPath}`);

    // Check if database exists
    if (fs.existsSync(dbPath) && !options.force) {
      console.error(`Database already exists at ${dbPath}. Use --force to overwrite.`);
      process.exit(1);
    }

    // Remove existing database if force
    if (fs.existsSync(dbPath) && options.force) {
      fs.rmSync(dbPath, { recursive: true });
      console.log('Removed existing database.');
    }

    try {
      const client = new KuzuClient(dbPath);
      await client.initialize();

      // Load nodes.json or content index
      const nodesPath = path.join(inputDir, 'nodes.json');
      const contentIndexPath = path.join(inputDir, 'content', 'index.json');

      let allNodes: any[] = [];
      if (fs.existsSync(nodesPath)) {
        console.log('\nImporting content nodes from nodes.json...');
        allNodes = JSON.parse(fs.readFileSync(nodesPath, 'utf-8'));
      } else if (fs.existsSync(contentIndexPath)) {
        console.log('\nImporting from content index...');
        const contentIndex = JSON.parse(fs.readFileSync(contentIndexPath, 'utf-8'));
        allNodes = contentIndex.nodes || [];
      }

      if (allNodes.length > 0) {
        // Deduplicate nodes by ID (keep first occurrence)
        const seenIds = new Set<string>();
        const uniqueNodes = allNodes.filter((node: any) => {
          if (seenIds.has(node.id)) {
            return false;
          }
          seenIds.add(node.id);
          return true;
        });
        console.log(`  Found ${allNodes.length} nodes, ${uniqueNodes.length} unique`);
        const inserted = await client.bulkInsertContentNodes(uniqueNodes);
        console.log(`  Inserted ${inserted} content nodes`);
      }

      // Load relationships
      const relsPath = path.join(inputDir, 'graph', 'relationships.json');
      if (fs.existsSync(relsPath)) {
        console.log('\nImporting relationships...');
        const relsData = JSON.parse(fs.readFileSync(relsPath, 'utf-8'));
        const inserted = await client.bulkInsertRelationships(relsData.relationships || []);
        console.log(`  Inserted ${inserted} relationships`);
      }

      // Load all path files from the paths directory
      const pathsDir = path.join(inputDir, 'paths');
      if (fs.existsSync(pathsDir)) {
        console.log('\nImporting learning paths...');
        const pathFiles = fs
          .readdirSync(pathsDir)
          .filter(f => f.endsWith('.json') && f !== 'index.json');

        let pathCount = 0;
        const seenPathIds = new Set<string>();

        for (const pathFile of pathFiles) {
          const pathFilePath = path.join(pathsDir, pathFile);
          try {
            const pathData = JSON.parse(fs.readFileSync(pathFilePath, 'utf-8'));

            // Skip duplicates by ID
            if (seenPathIds.has(pathData.id)) {
              continue;
            }
            seenPathIds.add(pathData.id);

            await client.createPath(pathData);
            pathCount++;
          } catch (err) {
            const pathId = pathFile.replace('.json', '');
            console.error(`  Failed to import path ${pathId}: ${(err as Error).message}`);
          }
        }
        console.log(`  Inserted ${pathCount} learning paths`);
      }

      // Show stats
      const stats = await client.getStats();
      console.log('\nDatabase Statistics:');
      for (const [table, count] of Object.entries(stats)) {
        console.log(`  ${table}: ${count}`);
      }

      client.close();
      console.log(`\n✓ Database initialized at ${dbPath}`);
    } catch (err) {
      console.error(`Database initialization failed: ${err}`);
      process.exit(1);
    }
  });

program
  .command('db:stats')
  .description('Show database statistics')
  .option('-d, --db <file>', 'Database file', './output/lamad.kuzu')
  .action(async options => {
    const dbPath = path.resolve(options.db);

    if (!fs.existsSync(dbPath)) {
      console.error(`Database not found: ${dbPath}`);
      process.exit(1);
    }

    try {
      const client = new KuzuClient(dbPath);
      await client.initialize();

      const stats = await client.getStats();
      console.log('Database Statistics');
      console.log('===================');
      for (const [table, count] of Object.entries(stats)) {
        console.log(`${table}: ${count}`);
      }

      client.close();
    } catch (err) {
      console.error(`Failed to get stats: ${err}`);
      process.exit(1);
    }
  });

program
  .command('db:dump')
  .description('Export database to Cypher seed file (git-friendly)')
  .option('-d, --db <file>', 'Database file', './output/lamad.kuzu')
  .option('-o, --output <file>', 'Output Cypher file', './output/lamad-seed.cypher')
  .action(async options => {
    const dbPath = path.resolve(options.db);
    const outputFile = path.resolve(options.output);

    if (!fs.existsSync(dbPath)) {
      console.error(`Database not found: ${dbPath}`);
      process.exit(1);
    }

    try {
      const client = new KuzuClient(dbPath);
      await client.initialize();

      const lines: string[] = [
        '// Elohim Protocol - Kuzu Database Seed File',
        `// Generated: ${new Date().toISOString()}`,
        '// This file can be used to initialize a fresh database',
        '',
        '// ============================================',
        '// Content Nodes',
        '// ============================================',
        '',
      ];

      // Export content nodes
      console.log('Exporting content nodes...');
      const nodes = await client.query<any>('MATCH (n:ContentNode) RETURN n');
      for (const row of nodes) {
        const n = row.n;
        const escapeCypher = (s: string) =>
          s
            ? `"${String(s)
                .replace(/\\/g, '\\\\')
                .replace(/"/g, String.raw`\"`)
                .replace(/\n/g, String.raw`\n`)}"`
            : '""';
        const escapeArray = (arr: string[]) =>
          arr?.length ? `[${arr.map(escapeCypher).join(', ')}]` : '[]';

        lines.push(
          `CREATE (:ContentNode {id: ${escapeCypher(n.id)}, contentType: ${escapeCypher(n.contentType)}, title: ${escapeCypher(n.title)}, description: ${escapeCypher(n.description || '')}, tags: ${escapeArray(n.tags || [])}});`
        );
      }
      console.log(`  ${nodes.length} content nodes`);

      // Export learning paths
      lines.push('');
      lines.push('// ============================================');
      lines.push('// Learning Paths');
      lines.push('// ============================================');
      lines.push('');

      console.log('Exporting learning paths...');
      const paths = await client.query<any>('MATCH (p:LearningPath) RETURN p');
      for (const row of paths) {
        const p = row.p;
        const escapeCypher = (s: string) =>
          s
            ? `"${String(s)
                .replace(/\\/g, '\\\\')
                .replace(/"/g, String.raw`\"`)
                .replace(/\n/g, String.raw`\n`)}"`
            : '""';
        const escapeArray = (arr: string[]) =>
          arr?.length ? `[${arr.map(escapeCypher).join(', ')}]` : '[]';

        lines.push(
          `CREATE (:LearningPath {id: ${escapeCypher(p.id)}, title: ${escapeCypher(p.title)}, description: ${escapeCypher(p.description || '')}, difficulty: ${escapeCypher(p.difficulty || 'intermediate')}, pathType: ${escapeCypher(p.pathType || 'journey')}, tags: ${escapeArray(p.tags || [])}});`
        );
      }
      console.log(`  ${paths.length} learning paths`);

      // Export path steps
      lines.push('');
      lines.push('// ============================================');
      lines.push('// Path Steps');
      lines.push('// ============================================');
      lines.push('');

      console.log('Exporting path steps...');
      const steps = await client.query<any>('MATCH (s:PathStep) RETURN s');
      for (const row of steps) {
        const s = row.s;
        const escapeCypher = (str: string) =>
          str
            ? `"${String(str)
                .replace(/\\/g, '\\\\')
                .replace(/"/g, String.raw`\"`)
                .replace(/\n/g, String.raw`\n`)}"`
            : '""';

        lines.push(
          `CREATE (:PathStep {id: ${escapeCypher(s.id)}, pathId: ${escapeCypher(s.pathId)}, orderIndex: ${s.orderIndex}, resourceId: ${escapeCypher(s.resourceId)}, stepTitle: ${escapeCypher(s.stepTitle || '')}});`
        );
      }
      console.log(`  ${steps.length} path steps`);

      // Export relationships
      lines.push('');
      lines.push('// ============================================');
      lines.push('// Relationships');
      lines.push('// ============================================');
      lines.push('');

      console.log('Exporting relationships...');
      const relTypes = [
        'CONTAINS',
        'RELATES_TO',
        'DEPENDS_ON',
        'PATH_HAS_STEP',
        'STEP_USES_CONTENT',
      ];
      let relCount = 0;
      for (const relType of relTypes) {
        try {
          const rels = await client.query<any>(
            `MATCH (a)-[r:${relType}]->(b) RETURN a.id as source, b.id as target`
          );
          for (const rel of rels) {
            lines.push(
              `MATCH (a {id: "${rel.source}"}), (b {id: "${rel.target}"}) CREATE (a)-[:${relType}]->(b);`
            );
            relCount++;
          }
        } catch {
          // Relationship type might not exist
        }
      }
      console.log(`  ${relCount} relationships`);

      // Write file
      fs.mkdirSync(path.dirname(outputFile), { recursive: true });
      fs.writeFileSync(outputFile, lines.join('\n'), 'utf-8');

      client.close();
      console.log(`\n✓ Exported to ${outputFile}`);
      console.log(`  File size: ${(fs.statSync(outputFile).size / 1024).toFixed(1)} KB`);
    } catch (err) {
      console.error(`Failed to dump database: ${err}`);
      process.exit(1);
    }
  });

program
  .command('path:create')
  .description('Create a new learning path in the database')
  .option('-d, --db <file>', 'Database file', './output/lamad.kuzu')
  .requiredOption('--id <id>', 'Path ID (kebab-case)')
  .requiredOption('--title <title>', 'Path title')
  .option('--description <desc>', 'Path description')
  .option('--purpose <purpose>', 'Learning purpose')
  .option('--difficulty <level>', 'Difficulty (beginner, intermediate, advanced)', 'intermediate')
  .option('--path-type <type>', 'Path type (journey, quest, expedition, practice)', 'journey')
  .option('--tags <tags>', 'Comma-separated tags')
  .action(async options => {
    const dbPath = path.resolve(options.db);

    try {
      const client = new KuzuClient(dbPath);
      await client.initialize();

      const pathData = {
        id: options.id,
        title: options.title,
        description: options.description || '',
        purpose: options.purpose || '',
        difficulty: options.difficulty,
        pathType: options.pathType,
        tags: options.tags ? options.tags.split(',').map((t: string) => t.trim()) : [],
        visibility: 'public',
        createdBy: 'cli',
      };

      await client.createPath(pathData);
      console.log(`✓ Created path: ${options.id}`);
      console.log(`  Title: ${options.title}`);
      console.log(`  Difficulty: ${options.difficulty}`);

      client.close();
    } catch (err) {
      console.error(`Failed to create path: ${err}`);
      process.exit(1);
    }
  });

program
  .command('path:add-step')
  .description('Add a content step to a learning path')
  .option('-d, --db <file>', 'Database file', './output/lamad.kuzu')
  .requiredOption('--path <pathId>', 'Path ID')
  .requiredOption('--content <contentId>', 'Content node ID to add')
  .option('--position <n>', 'Step position (0-indexed)', '0')
  .option('--title <title>', 'Step title (defaults to content title)')
  .option('--narrative <text>', 'Step narrative/description')
  .option('--optional', 'Mark step as optional', false)
  .action(async options => {
    const dbPath = path.resolve(options.db);

    try {
      const client = new KuzuClient(dbPath);
      await client.initialize();

      // Get content node for default title
      const content = await client.getContentNode(options.content);
      const stepTitle = options.title || content?.title || options.content;
      const stepNarrative = options.narrative || content?.description || '';

      const step = {
        order: Number.parseInt(options.position, 10),
        stepType: 'content',
        resourceId: options.content,
        stepTitle,
        stepNarrative,
        optional: options.optional,
        learningObjectives: [],
        completionCriteria: ['Review content'],
      };

      await client.addPathStep(options.path, step);
      console.log(`✓ Added step to path ${options.path}`);
      console.log(`  Position: ${step.order}`);
      console.log(`  Content: ${options.content}`);
      console.log(`  Title: ${stepTitle}`);

      client.close();
    } catch (err) {
      console.error(`Failed to add step: ${err}`);
      process.exit(1);
    }
  });

program
  .command('path:list')
  .description('List all learning paths in the database')
  .option('-d, --db <file>', 'Database file', './output/lamad.kuzu')
  .action(async options => {
    const dbPath = path.resolve(options.db);

    try {
      const client = new KuzuClient(dbPath);
      await client.initialize();

      const paths = await client.listPaths();
      console.log('Learning Paths');
      console.log('==============');
      for (const p of paths) {
        console.log(`\n${p.id}`);
        console.log(`  Title: ${p.title}`);
        console.log(`  Difficulty: ${p.difficulty}`);
        console.log(`  Type: ${p.pathType}`);
      }
      console.log(`\nTotal: ${paths.length} paths`);

      client.close();
    } catch (err) {
      console.error(`Failed to list paths: ${err}`);
      process.exit(1);
    }
  });

program
  .command('path:show')
  .description('Show details of a learning path')
  .option('-d, --db <file>', 'Database file', './output/lamad.kuzu')
  .requiredOption('--id <pathId>', 'Path ID')
  .action(async options => {
    const dbPath = path.resolve(options.db);

    try {
      const client = new KuzuClient(dbPath);
      await client.initialize();

      const pathData = await client.getPath(options.id);
      if (!pathData) {
        console.error(`Path not found: ${options.id}`);
        process.exit(1);
      }

      console.log('Learning Path Details');
      console.log('=====================');
      console.log(`ID: ${pathData.id}`);
      console.log(`Title: ${pathData.title}`);
      console.log(`Description: ${pathData.description}`);
      console.log(`Purpose: ${pathData.purpose}`);
      console.log(`Difficulty: ${pathData.difficulty}`);
      console.log(`Type: ${pathData.pathType}`);
      console.log(`Tags: ${pathData.tags?.join(', ') || 'none'}`);

      if (pathData.chapters?.length) {
        console.log(`\nChapters (${pathData.chapters.length}):`);
        for (const chapter of pathData.chapters) {
          console.log(`  ${chapter.order}. ${chapter.title} (${chapter.steps.length} steps)`);
          for (const step of chapter.steps) {
            console.log(`      - ${step.stepTitle || step.resourceId}`);
          }
        }
      } else if (pathData.steps?.length) {
        console.log(`\nSteps (${pathData.steps.length}):`);
        for (const step of pathData.steps) {
          console.log(`  ${step.order}. ${step.stepTitle || step.resourceId}`);
        }
      }

      client.close();
    } catch (err) {
      console.error(`Failed to show path: ${err}`);
      process.exit(1);
    }
  });

program
  .command('content:create')
  .description('Create a new content node in the database')
  .option('-d, --db <file>', 'Database file', './output/lamad.kuzu')
  .requiredOption('--id <id>', 'Content ID')
  .requiredOption('--title <title>', 'Content title')
  .requiredOption('--type <type>', 'Content type (concept, scenario, epic, role, etc.)')
  .option('--description <desc>', 'Content description')
  .option('--content <text>', 'Content body (or use --file)')
  .option('--file <path>', 'Read content from file')
  .option('--format <fmt>', 'Content format', 'markdown')
  .option('--tags <tags>', 'Comma-separated tags')
  .option('--reach <reach>', 'Content reach', 'commons')
  .action(async options => {
    const dbPath = path.resolve(options.db);

    try {
      const client = new KuzuClient(dbPath);
      await client.initialize();

      let content = options.content || '';
      if (options.file && fs.existsSync(options.file)) {
        content = fs.readFileSync(options.file, 'utf-8');
      }

      const node = {
        id: options.id,
        contentType: options.type,
        title: options.title,
        description: options.description || '',
        content,
        contentFormat: options.format,
        tags: options.tags ? options.tags.split(',').map((t: string) => t.trim()) : [],
        reach: options.reach,
      };

      await client.createContentNode(node);
      console.log(`✓ Created content: ${options.id}`);
      console.log(`  Title: ${options.title}`);
      console.log(`  Type: ${options.type}`);

      client.close();
    } catch (err) {
      console.error(`Failed to create content: ${err}`);
      process.exit(1);
    }
  });

program
  .command('content:show')
  .description('Show a content node from the database')
  .option('-d, --db <file>', 'Database file', './output/lamad.kuzu')
  .requiredOption('--id <id>', 'Content ID')
  .action(async options => {
    const dbPath = path.resolve(options.db);

    try {
      const client = new KuzuClient(dbPath);
      await client.initialize();

      const node = await client.getContentNode(options.id);
      if (!node) {
        console.error(`Content not found: ${options.id}`);
        process.exit(1);
      }

      console.log('Content Node');
      console.log('============');
      console.log(`ID: ${node.id}`);
      console.log(`Title: ${node.title}`);
      console.log(`Type: ${node.contentType}`);
      console.log(`Format: ${node.contentFormat}`);
      console.log(`Description: ${node.description}`);
      console.log(`Tags: ${node.tags?.join(', ') || 'none'}`);
      console.log(`Reach: ${node.reach}`);
      console.log(`\nContent (first 500 chars):`);
      console.log(node.content?.substring(0, 500) || '(empty)');

      client.close();
    } catch (err) {
      console.error(`Failed to show content: ${err}`);
      process.exit(1);
    }
  });

program
  .command('query')
  .description('Execute a Cypher query on the database')
  .option('-d, --db <file>', 'Database file', './output/lamad.kuzu')
  .option('-q, --query <cypher>', 'Cypher query string')
  .option('-f, --file <path>', 'Read query from file')
  .option('--json', 'Output as JSON', false)
  .action(async options => {
    const dbPath = path.resolve(options.db);

    let query = options.query || '';
    if (options.file && fs.existsSync(options.file)) {
      query = fs.readFileSync(options.file, 'utf-8');
    }

    if (!query) {
      console.error('Provide a query with -q or -f');
      process.exit(1);
    }

    try {
      const client = new KuzuClient(dbPath);
      await client.initialize();

      const results = await client.query(query);

      if (options.json) {
        console.log(JSON.stringify(results, null, 2));
      } else {
        console.log('Query Results');
        console.log('=============');
        if (results.length === 0) {
          console.log('(no results)');
        } else {
          for (const row of results) {
            console.log(JSON.stringify(row));
          }
          console.log(`\n${results.length} row(s)`);
        }
      }

      client.close();
    } catch (err) {
      console.error(`Query failed: ${err}`);
      process.exit(1);
    }
  });

program
  .command('db:export')
  .description('Export database to JSON files')
  .option('-d, --db <file>', 'Database file', './output/lamad.kuzu')
  .option('-o, --output <dir>', 'Output directory', './output/lamad-export')
  .action(async options => {
    const dbPath = path.resolve(options.db);
    const outputDir = path.resolve(options.output);

    try {
      const client = new KuzuClient(dbPath);
      await client.initialize();

      // Create output directories
      fs.mkdirSync(path.join(outputDir, 'content'), { recursive: true });
      fs.mkdirSync(path.join(outputDir, 'paths'), { recursive: true });

      // Export content nodes
      console.log('Exporting content nodes...');
      const nodes = await client.query<any>('MATCH (n:ContentNode) RETURN n');
      const contentIndex = {
        lastUpdated: new Date().toISOString(),
        totalCount: nodes.length,
        nodes: nodes.map(row => row.n),
      };
      fs.writeFileSync(
        path.join(outputDir, 'content', 'index.json'),
        JSON.stringify(contentIndex, null, 2)
      );
      console.log(`  Exported ${nodes.length} content nodes`);

      // Export individual content files
      for (const row of nodes) {
        const node = row.n;
        fs.writeFileSync(
          path.join(outputDir, 'content', `${node.id}.json`),
          JSON.stringify(node, null, 2)
        );
      }

      // Export paths
      console.log('Exporting learning paths...');
      const paths = await client.listPaths();
      const pathsIndex = {
        lastUpdated: new Date().toISOString(),
        totalCount: paths.length,
        paths: paths.map(p => ({
          id: p.id,
          title: p.title,
          description: p.description,
          difficulty: p.difficulty,
          pathType: p.pathType,
          tags: p.tags,
        })),
      };
      fs.writeFileSync(
        path.join(outputDir, 'paths', 'index.json'),
        JSON.stringify(pathsIndex, null, 2)
      );

      // Export individual path files with full data
      for (const p of paths) {
        const fullPath = await client.getPath(p.id);
        if (fullPath) {
          fs.writeFileSync(
            path.join(outputDir, 'paths', `${p.id}.json`),
            JSON.stringify(fullPath, null, 2)
          );
        }
      }
      console.log(`  Exported ${paths.length} learning paths`);

      client.close();
      console.log(`\n✓ Exported to ${outputDir}`);
    } catch (err) {
      console.error(`Export failed: ${err}`);
      process.exit(1);
    }
  });

program.parse();
