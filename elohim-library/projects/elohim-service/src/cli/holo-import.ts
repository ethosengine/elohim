#!/usr/bin/env node
/**
 * Holochain Import CLI
 *
 * Command-line interface for importing content to Holochain conductor.
 * Runs the same transformers as the Kuzu import, different storage layer.
 *
 * Usage:
 *   npx ts-node src/cli/holo-import.ts holo:import --source ./data/content
 *   npx ts-node src/cli/holo-import.ts holo:stats
 *   npx ts-node src/cli/holo-import.ts holo:verify --ids "manifesto,governance-epic"
 *   npx ts-node src/cli/holo-import.ts holo:list --type scenario
 *   npx ts-node src/cli/holo-import.ts holo:test
 */

import { Command } from 'commander';
import * as path from 'path';
import * as fs from 'fs';
import { runImportPipeline } from '../services/import-pipeline.service';
import { HolochainImportService } from '../services/holochain-import.service';
import { HolochainClientService } from '../services/holochain-client.service';
import { HolochainImportConfig } from '../models/holochain.model';

const program = new Command();

// Default configuration
const DEFAULT_CONFIG: HolochainImportConfig = {
  adminUrl: 'wss://holochain-dev.elohim.host',
  appId: 'lamad-spike',
  batchSize: 50,
};

program
  .name('elohim-holo')
  .description('Import content to Holochain conductor')
  .version('1.0.0');

// =============================================================================
// holo:import - Import content from source files to Holochain
// =============================================================================
program
  .command('holo:import')
  .description('Import content from source files to Holochain')
  .option('-s, --source <dir>', 'Source content directory', './data/content')
  .option('--admin-url <url>', 'Holochain admin WebSocket URL', DEFAULT_CONFIG.adminUrl)
  .option('--app-id <id>', 'Holochain app ID', DEFAULT_CONFIG.appId)
  .option('--happ-path <path>', 'Path to .happ file for installation')
  .option('--batch-size <n>', 'Entries per bulk call', String(DEFAULT_CONFIG.batchSize))
  .option('-f, --full', 'Force full reimport', false)
  .option('-v, --verbose', 'Verbose output', false)
  .option('--dry-run', 'Parse and transform but do not write to Holochain', false)
  .option('--skip-relationships', 'Skip relationship extraction', false)
  .action(async (options) => {
    const sourceDir = path.resolve(options.source);

    console.log('Holochain Content Import');
    console.log('========================');
    console.log(`Source: ${sourceDir}`);
    console.log(`Holochain: ${options.adminUrl} (${options.appId})`);
    console.log(`Mode: ${options.full ? 'Full' : 'Incremental'}`);
    console.log(`Batch size: ${options.batchSize}`);
    if (options.dryRun) {
      console.log('Mode: DRY RUN (no Holochain writes)');
    }
    console.log('');

    try {
      // Step 1: Run the standard import pipeline (parse + transform)
      // Use a temp output dir since we're not writing to Kuzu
      const tempDir = path.join(process.cwd(), '.holo-import-temp');

      console.log('Parsing and transforming source files...');
      const pipelineResult = await runImportPipeline({
        mode: options.full ? 'full' : 'incremental',
        sourceDir,
        outputDir: tempDir,
        verbose: options.verbose,
        dryRun: true, // Don't write to Kuzu
        generateSourceNodes: true,
        generateDerivedNodes: true,
        skipRelationships: options.skipRelationships,
      });

      console.log(`\nParsed ${pipelineResult.totalNodes} nodes from ${pipelineResult.totalFiles} files`);
      console.log(`  Types: ${[...new Set(pipelineResult.nodes.map(n => n.contentType))].join(', ')}`);

      if (options.dryRun) {
        console.log('\n[Dry run - no Holochain writes]');

        // Show sample nodes
        if (pipelineResult.nodes.length > 0) {
          console.log('\nSample nodes:');
          for (const node of pipelineResult.nodes.slice(0, 5)) {
            console.log(`  - ${node.id} (${node.contentType}): ${node.title.substring(0, 50)}...`);
          }
          if (pipelineResult.nodes.length > 5) {
            console.log(`  ... and ${pipelineResult.nodes.length - 5} more`);
          }
        }
        return;
      }

      // Step 2: Import to Holochain
      const holoService = new HolochainImportService({
        adminUrl: options.adminUrl,
        appId: options.appId,
        happPath: options.happPath,
        batchSize: parseInt(options.batchSize, 10),
      });

      console.log('\nConnecting to Holochain conductor...');
      const holoResult = await holoService.importNodes(pipelineResult.nodes);

      console.log('\n--- Import Complete ---');
      console.log(`Import ID: ${holoResult.importId}`);
      console.log(`Nodes created: ${holoResult.createdNodes}/${holoResult.totalNodes}`);
      console.log(`Duration: ${holoResult.durationMs}ms`);

      if (holoResult.errors.length > 0) {
        console.log(`\nErrors (${holoResult.errors.length}):`);
        for (const error of holoResult.errors.slice(0, 10)) {
          console.log(`  - ${error}`);
        }
        if (holoResult.errors.length > 10) {
          console.log(`  ... and ${holoResult.errors.length - 10} more`);
        }
      }
    } catch (err) {
      console.error(`\nImport failed: ${err}`);
      process.exit(1);
    }
  });

// =============================================================================
// holo:stats - Show Holochain content statistics
// =============================================================================
program
  .command('holo:stats')
  .description('Show Holochain content statistics')
  .option('--admin-url <url>', 'Holochain admin WebSocket URL', DEFAULT_CONFIG.adminUrl)
  .option('--app-id <id>', 'Holochain app ID', DEFAULT_CONFIG.appId)
  .action(async (options) => {
    try {
      const holoService = new HolochainImportService({
        adminUrl: options.adminUrl,
        appId: options.appId,
        batchSize: 50,
      });

      console.log('Fetching Holochain content statistics...\n');
      const stats = await holoService.getStats();

      console.log('Holochain Content Statistics');
      console.log('============================');
      console.log(`Total nodes: ${stats.total_count}`);

      if (Object.keys(stats.by_type).length > 0) {
        console.log('\nBy content type:');
        const sortedTypes = Object.entries(stats.by_type).sort((a, b) => b[1] - a[1]);
        for (const [type, count] of sortedTypes) {
          console.log(`  ${type.padEnd(20)} ${count}`);
        }
      } else {
        console.log('\nNo content found in Holochain.');
      }
    } catch (err) {
      console.error(`Failed to get stats: ${err}`);
      process.exit(1);
    }
  });

// =============================================================================
// holo:verify - Verify content exists in Holochain
// =============================================================================
program
  .command('holo:verify')
  .description('Verify content exists in Holochain')
  .option('-i, --ids <ids>', 'Comma-separated content IDs to verify')
  .option('-f, --file <path>', 'File with IDs (one per line)')
  .option('--admin-url <url>', 'Holochain admin WebSocket URL', DEFAULT_CONFIG.adminUrl)
  .option('--app-id <id>', 'Holochain app ID', DEFAULT_CONFIG.appId)
  .action(async (options) => {
    let ids: string[] = [];

    if (options.ids) {
      ids = options.ids.split(',').map((s: string) => s.trim());
    } else if (options.file && fs.existsSync(options.file)) {
      ids = fs
        .readFileSync(options.file, 'utf-8')
        .split('\n')
        .map((s) => s.trim())
        .filter(Boolean);
    } else {
      console.error('Provide --ids or --file');
      process.exit(1);
    }

    console.log(`Verifying ${ids.length} content IDs in Holochain...\n`);

    try {
      const holoService = new HolochainImportService({
        adminUrl: options.adminUrl,
        appId: options.appId,
        batchSize: 50,
      });

      const result = await holoService.verifyContent(ids);

      console.log('Verification Results');
      console.log('====================');
      console.log(`Found:   ${result.found.length}/${ids.length}`);
      console.log(`Missing: ${result.missing.length}`);

      if (result.found.length > 0 && result.found.length <= 20) {
        console.log('\nFound IDs:');
        for (const id of result.found) {
          console.log(`  ✓ ${id}`);
        }
      }

      if (result.missing.length > 0) {
        console.log('\nMissing IDs:');
        for (const id of result.missing) {
          console.log(`  ✗ ${id}`);
        }
      }

      // Exit with error code if any missing
      if (result.missing.length > 0) {
        process.exit(1);
      }
    } catch (err) {
      console.error(`Verification failed: ${err}`);
      process.exit(1);
    }
  });

// =============================================================================
// holo:list - List content in Holochain by type
// =============================================================================
program
  .command('holo:list')
  .description('List content in Holochain by type')
  .option('-t, --type <type>', 'Content type to list', 'scenario')
  .option('-l, --limit <n>', 'Maximum entries', '20')
  .option('--admin-url <url>', 'Holochain admin WebSocket URL', DEFAULT_CONFIG.adminUrl)
  .option('--app-id <id>', 'Holochain app ID', DEFAULT_CONFIG.appId)
  .action(async (options) => {
    try {
      const holoService = new HolochainImportService({
        adminUrl: options.adminUrl,
        appId: options.appId,
        batchSize: 50,
      });

      console.log(`Fetching content of type '${options.type}'...\n`);
      const results = await holoService.getContentByType(
        options.type,
        parseInt(options.limit, 10)
      );

      console.log(`Content (type: ${options.type})`);
      console.log('='.repeat(50));

      if (results.length === 0) {
        console.log('No content found.');
        return;
      }

      for (const item of results) {
        console.log(`\n${item.content.id}`);
        console.log(`  Title: ${item.content.title}`);
        console.log(`  Format: ${item.content.content_format}`);
        console.log(`  Tags: ${item.content.tags.join(', ') || '(none)'}`);
        console.log(`  Reach: ${item.content.reach}`);
      }

      console.log(`\n--- Total: ${results.length} ---`);
    } catch (err) {
      console.error(`List failed: ${err}`);
      process.exit(1);
    }
  });

// =============================================================================
// holo:get - Get a single content by ID
// =============================================================================
program
  .command('holo:get')
  .description('Get a single content by ID')
  .argument('<id>', 'Content ID to fetch')
  .option('--admin-url <url>', 'Holochain admin WebSocket URL', DEFAULT_CONFIG.adminUrl)
  .option('--app-id <id>', 'Holochain app ID', DEFAULT_CONFIG.appId)
  .option('--json', 'Output as JSON', false)
  .action(async (id, options) => {
    try {
      const holoService = new HolochainImportService({
        adminUrl: options.adminUrl,
        appId: options.appId,
        batchSize: 50,
      });

      const result = await holoService.getContentById(id);

      if (!result) {
        console.error(`Content not found: ${id}`);
        process.exit(1);
      }

      if (options.json) {
        console.log(JSON.stringify(result.content, null, 2));
        return;
      }

      console.log('Content Details');
      console.log('===============');
      console.log(`ID:          ${result.content.id}`);
      console.log(`Type:        ${result.content.content_type}`);
      console.log(`Title:       ${result.content.title}`);
      console.log(`Format:      ${result.content.content_format}`);
      console.log(`Reach:       ${result.content.reach}`);
      console.log(`Trust Score: ${result.content.trust_score}`);
      console.log(`Tags:        ${result.content.tags.join(', ') || '(none)'}`);
      console.log(`Created:     ${result.content.created_at}`);
      console.log(`Updated:     ${result.content.updated_at}`);
      console.log('');
      console.log('Description:');
      console.log(`  ${result.content.description}`);
      console.log('');
      console.log('Content (first 500 chars):');
      console.log(`  ${result.content.content.substring(0, 500)}...`);
    } catch (err) {
      console.error(`Get failed: ${err}`);
      process.exit(1);
    }
  });

// =============================================================================
// holo:test - Test Holochain connection
// =============================================================================
program
  .command('holo:test')
  .description('Test Holochain connection')
  .option('--admin-url <url>', 'Holochain admin WebSocket URL', DEFAULT_CONFIG.adminUrl)
  .option('--app-id <id>', 'Holochain app ID', DEFAULT_CONFIG.appId)
  .action(async (options) => {
    console.log('Testing Holochain connection...');
    console.log(`Admin URL: ${options.adminUrl}`);
    console.log(`App ID: ${options.appId}`);
    console.log('');

    const client = new HolochainClientService({
      adminUrl: options.adminUrl,
      appId: options.appId,
    });

    try {
      console.log('Connecting to admin interface...');
      await client.connect();
      console.log('[OK] Connected to conductor');

      console.log('Testing zome call (get_content_stats)...');
      const stats = await client.callZome<{ total_count: number }>({
        zomeName: 'content_store',
        fnName: 'get_content_stats',
        payload: null,
      });
      console.log(`[OK] Zome call successful`);
      console.log(`  Content count: ${stats.total_count}`);

      await client.disconnect();
      console.log('[OK] Disconnected cleanly');

      console.log('\n--- Connection test PASSED ---');
    } catch (err) {
      console.error(`\n[FAIL] ${err}`);
      process.exit(1);
    }
  });

// =============================================================================
// Run CLI
// =============================================================================
program.parse();
