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
  .description('Import content from source files into ContentNodes')
  .option('-s, --source <dir>', 'Source content directory', './docs/content')
  .option('-o, --output <dir>', 'Output directory for manifest and data', './output/lamad')
  .option('-f, --full', 'Force full reimport (ignore incremental cache)', false)
  .option('-v, --verbose', 'Verbose output', false)
  .option('--dry-run', 'Dry run - parse and transform only, do not save manifest', false)
  .option('--skip-relationships', 'Skip relationship extraction (faster, less memory)', false)
  .action(async (options: any) => {
    const sourceDir = path.resolve(options.source);
    const outputDir = path.resolve(options.output);

    console.log('Elohim Content Import');
    console.log('=====================');
    console.log(`Source: ${sourceDir}`);
    console.log(`Output: ${outputDir}`);
    console.log(`Mode: ${options.full ? 'Full' : 'Incremental'}`);
    if (options.skipRelationships) {
      console.log('Relationships: SKIPPED');
    }
    console.log('');

    try {
      const result = await runImportPipeline({
        mode: options.full ? 'full' : 'incremental',
        sourceDir,
        outputDir,
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
  .action(async (options: any) => {
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
  .action(async (options: any) => {
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
  .action(async (options: any) => {
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
  .action(async (options: any) => {
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
  .action(async (options: any) => {
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
  .action(async (options: any) => {
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
  .action(async (options: any) => {
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

program.parse(process.argv);

// REMOVED: Kuzu database commands (db:init, db:stats, db:dump, db:export,
// path:create, path:add-step, path:list, path:show, content:create,
// content:show, query) — content pipeline uses Holochain storage via
// holo-import.ts and genesis/seeder.
