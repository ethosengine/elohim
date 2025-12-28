/**
 * Production Seeder - Dual-path seeding for DNA + Projection Cache
 *
 * Seeds content to both:
 * 1. Holochain DNA (metadata with blob_hash references)
 * 2. Doorway Projection Cache (actual blob content)
 *
 * Architecture:
 * ┌─────────────────────────────────────────────────────────────────┐
 * │  Seed Files (data/lamad/content/*.json)                        │
 * └───────────────────────────┬─────────────────────────────────────┘
 *                             │
 *                             ▼
 * ┌─────────────────────────────────────────────────────────────────┐
 * │  BlobManager                                                    │
 * │  - Extract blobs from large content                             │
 * │  - Compute SHA256 hashes                                        │
 * │  - Separate metadata from blob content                          │
 * └───────────────────────────┬─────────────────────────────────────┘
 *                             │
 *         ┌───────────────────┴───────────────────┐
 *         │                                       │
 *         ▼                                       ▼
 * ┌───────────────────────┐           ┌───────────────────────────┐
 * │  Holochain DNA        │           │  Doorway Projection Cache │
 * │  - Metadata entries   │           │  - Blob content           │
 * │  - blob_hash refs     │           │  - Fast delivery          │
 * │  - Path entries       │           │  - CDN-like serving       │
 * └───────────────────────┘           └───────────────────────────┘
 *
 * Usage:
 *   npx tsx src/seed-production.ts
 *
 * Environment:
 *   HOLOCHAIN_ADMIN_URL - Holochain admin websocket URL
 *   DOORWAY_URL - Doorway base URL
 *   DOORWAY_API_KEY - API key for doorway admin operations
 *   DRY_RUN - Set to 'true' to validate without seeding
 */

import { AdminWebsocket, AppWebsocket, encodeHashToBase64, CellId } from '@holochain/client';
import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';

// ES module compatibility for __dirname
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
import { BlobManager, validateBlobReferences, ContentFile } from './blob-manager';
import { DoorwayClient, validateSeedingPrerequisites } from './doorway-client';
import { validateContentFile, validateAllContent, printValidationReport } from './schema-validation';

// =============================================================================
// Configuration
// =============================================================================

interface SeedConfig {
  // Holochain
  adminUrl: string;
  appId: string;

  // Doorway
  doorwayUrl: string;
  doorwayApiKey?: string;

  // Content
  contentDir: string;
  pathsDir: string;

  // Options
  dryRun: boolean;
  verbose: boolean;
  validateOnly: boolean;
  skipBlobs: boolean;
  skipDna: boolean;
}

interface SeedResults {
  // Pre-flight
  validationPassed: boolean;
  doorwayReady: boolean;

  // DNA seeding
  contentSeeded: number;
  contentErrors: number;
  pathsSeeded: number;
  pathErrors: number;

  // Blob seeding
  blobsExtracted: number;
  blobsPushed: number;
  blobsCached: number;
  blobErrors: number;

  // Timing
  startTime: number;
  endTime: number;
}

// =============================================================================
// Main Seeder
// =============================================================================

async function seedProduction(config: SeedConfig): Promise<SeedResults> {
  const results: SeedResults = {
    validationPassed: false,
    doorwayReady: false,
    contentSeeded: 0,
    contentErrors: 0,
    pathsSeeded: 0,
    pathErrors: 0,
    blobsExtracted: 0,
    blobsPushed: 0,
    blobsCached: 0,
    blobErrors: 0,
    startTime: Date.now(),
    endTime: 0,
  };

  console.log('═'.repeat(70));
  console.log('PRODUCTION SEEDER - DNA + Projection Cache');
  console.log('═'.repeat(70));
  console.log(`\nContent Dir: ${config.contentDir}`);
  console.log(`Paths Dir:   ${config.pathsDir}`);
  console.log(`Doorway:     ${config.doorwayUrl}`);
  console.log(`Holochain:   ${config.adminUrl}`);
  console.log(`Mode:        ${config.dryRun ? 'DRY RUN' : 'PRODUCTION'}`);

  // =========================================================================
  // Phase 1: Pre-flight Validation
  // =========================================================================
  console.log('\n' + '─'.repeat(70));
  console.log('PHASE 1: Pre-flight Validation');
  console.log('─'.repeat(70));

  // Validate seed file structure
  console.log('\n1.1 Validating seed file structure...');
  const validationReport = await validateAllContent(config.contentDir);
  printValidationReport(validationReport);

  if (validationReport.invalidFiles > 0) {
    console.error(`\n❌ ${validationReport.invalidFiles} files have validation errors`);
    if (!config.dryRun) {
      console.error('Fix validation errors before seeding');
      results.endTime = Date.now();
      return results;
    }
  }
  results.validationPassed = validationReport.invalidFiles === 0;

  // Validate blob references
  console.log('\n1.2 Validating blob references...');
  const contentFiles = fs.readdirSync(config.contentDir)
    .filter(f => f.endsWith('.json') && f !== 'index.json');

  let blobRefErrors = 0;
  let blobRefWarnings = 0;

  for (const file of contentFiles) {
    try {
      const content: ContentFile = JSON.parse(
        fs.readFileSync(path.join(config.contentDir, file), 'utf-8')
      );
      const validation = validateBlobReferences(content);
      if (!validation.valid) {
        blobRefErrors++;
        console.error(`  ❌ ${file}:`);
        validation.errors.forEach(e => console.error(`     ${e}`));
      }
      if (validation.warnings.length > 0) {
        blobRefWarnings++;
        if (config.verbose) {
          console.warn(`  ⚠️  ${file}:`);
          validation.warnings.forEach(w => console.warn(`     ${w}`));
        }
      }
    } catch (e) {
      console.error(`  ❌ ${file}: Parse error`);
      blobRefErrors++;
    }
  }

  console.log(`\nBlob reference validation: ${blobRefErrors} errors, ${blobRefWarnings} warnings`);

  // Validate doorway availability
  console.log('\n1.3 Checking doorway availability...');
  if (!config.skipBlobs) {
    const doorwayCheck = await validateSeedingPrerequisites(
      config.doorwayUrl,
      config.doorwayApiKey
    );

    if (!doorwayCheck.ready) {
      console.error('❌ Doorway not ready:');
      doorwayCheck.issues.forEach(i => console.error(`   - ${i}`));

      if (!config.dryRun) {
        results.endTime = Date.now();
        return results;
      }
    } else {
      console.log('✅ Doorway ready for blob seeding');
      results.doorwayReady = true;
    }
  } else {
    console.log('⏭️  Skipping doorway check (--skip-blobs)');
  }

  if (config.validateOnly) {
    console.log('\n✅ Validation complete (--validate-only)');
    results.endTime = Date.now();
    return results;
  }

  // =========================================================================
  // Phase 2: Blob Extraction and Caching
  // =========================================================================
  console.log('\n' + '─'.repeat(70));
  console.log('PHASE 2: Blob Extraction');
  console.log('─'.repeat(70));

  const blobManager = new BlobManager({
    doorwayUrl: config.doorwayUrl,
    apiKey: config.doorwayApiKey,
    dryRun: config.dryRun,
  });

  const doorwayClient = new DoorwayClient({
    baseUrl: config.doorwayUrl,
    apiKey: config.doorwayApiKey,
    dryRun: config.dryRun,
  });

  const processedContents: Array<{ id: string; metadata: Record<string, unknown> }> = [];
  const blobsToPush: Array<{ hash: string; data: Buffer; metadata: { hash: string; sizeBytes: number; mimeType: string; entryPoint?: string } }> = [];

  console.log(`\nProcessing ${contentFiles.length} content files...`);

  for (const file of contentFiles) {
    try {
      const content: ContentFile = JSON.parse(
        fs.readFileSync(path.join(config.contentDir, file), 'utf-8')
      );

      const processed = await blobManager.processContent(content, config.contentDir);

      if (processed.extracted && processed.blob && processed.blobMetadata) {
        results.blobsExtracted++;
        blobsToPush.push({
          hash: processed.blobMetadata.hash,
          data: processed.blob,
          metadata: processed.blobMetadata,
        });
        if (config.verbose) {
          console.log(`  📦 ${file} → blob ${processed.blobMetadata.hash.slice(0, 20)}...`);
        }
      }

      processedContents.push({
        id: content.id,
        metadata: processed.metadata,
      });
    } catch (e) {
      console.error(`  ❌ ${file}: ${e instanceof Error ? e.message : 'Unknown error'}`);
      results.contentErrors++;
    }
  }

  console.log(`\nExtracted ${results.blobsExtracted} blobs from ${contentFiles.length} files`);

  // =========================================================================
  // Phase 3: Push Blobs to Projection Cache
  // =========================================================================
  if (!config.skipBlobs && blobsToPush.length > 0) {
    console.log('\n' + '─'.repeat(70));
    console.log('PHASE 3: Push Blobs to Projection Cache');
    console.log('─'.repeat(70));

    console.log(`\nPushing ${blobsToPush.length} blobs to ${config.doorwayUrl}...`);

    const pushResult = await doorwayClient.pushBlobs(blobsToPush);
    results.blobsPushed = pushResult.success;
    results.blobErrors = pushResult.failed;

    console.log(`\nBlob push: ${pushResult.success} success, ${pushResult.failed} failed`);
  }

  // =========================================================================
  // Phase 4: Seed DNA
  // =========================================================================
  if (!config.skipDna) {
    console.log('\n' + '─'.repeat(70));
    console.log('PHASE 4: Seed Holochain DNA');
    console.log('─'.repeat(70));

    if (config.dryRun) {
      console.log('\n[DRY RUN] Would seed to DNA:');
      console.log(`  - ${processedContents.length} content entries`);
    } else {
      // Connect to Holochain
      console.log('\nConnecting to Holochain...');

      try {
        const adminWs = await AdminWebsocket.connect({
          url: new URL(config.adminUrl),
          wsClientOptions: { origin: 'http://localhost' },
        });
        const apps = await adminWs.listApps({});
        const app = apps.find(a => a.installed_app_id === config.appId);

        if (!app) {
          throw new Error(`App '${config.appId}' not found`);
        }

        // Find the first role with provisioned cells
        const roleNames = Object.keys(app.cell_info);
        const roleName = roleNames.find(name => {
          const cells = app.cell_info[name];
          return cells && cells.length > 0 && 'provisioned' in cells[0];
        });

        if (!roleName) {
          throw new Error('No provisioned cells found');
        }

        const cellInfo = app.cell_info[roleName][0];
        if (!('provisioned' in cellInfo)) {
          throw new Error('Cell not provisioned');
        }

        const cellId: CellId = cellInfo.provisioned.cell_id;
        const appInfo = await adminWs.attachAppInterface({ allowed_origins: '*' });
        const token = await adminWs.issueAppAuthenticationToken({ installed_app_id: config.appId });
        const appWsUrl = `ws://localhost:${appInfo.port}`;
        const appWs = await AppWebsocket.connect({
          url: new URL(appWsUrl),
          wsClientOptions: { origin: 'http://localhost' },
          token: token.token,
        });

        console.log(`Connected to app: ${config.appId}`);
        console.log(`Cell: ${encodeHashToBase64(cellId[0])}`);

        // Seed content
        console.log(`\nSeeding ${processedContents.length} content entries...`);

        for (const { id, metadata } of processedContents) {
          try {
            await appWs.callZome({
              cell_id: cellId,
              zome_name: 'content_store',
              fn_name: 'create_content',
              payload: {
                id: metadata.id,
                content_type: metadata.contentType || 'concept',
                title: metadata.title || id,
                description: metadata.description || '',
                content: metadata.content ? JSON.stringify(metadata.content) : '',
                content_format: metadata.contentFormat || 'markdown',
                tags: metadata.tags || [],
                related_node_ids: metadata.relatedNodeIds || [],
                metadata_json: JSON.stringify({
                  blob_hash: metadata.blob_hash,
                  blob_url: metadata.blob_url,
                  entry_point: metadata.entry_point,
                  fallback_url: metadata.fallback_url,
                }),
              },
            });
            results.contentSeeded++;
            if (config.verbose) {
              console.log(`  ✓ ${id}`);
            }
          } catch (e) {
            results.contentErrors++;
            const error = e instanceof Error ? e.message : 'Unknown error';
            if (error.includes('already exists')) {
              if (config.verbose) {
                console.log(`  ⏭️  ${id} (already exists)`);
              }
            } else {
              console.error(`  ❌ ${id}: ${error}`);
            }
          }
        }

        // Seed paths
        const pathFiles = fs.readdirSync(config.pathsDir)
          .filter(f => f.endsWith('.json') && f !== 'index.json');

        console.log(`\nSeeding ${pathFiles.length} learning paths...`);

        for (const file of pathFiles) {
          try {
            const pathData = JSON.parse(
              fs.readFileSync(path.join(config.pathsDir, file), 'utf-8')
            );

            await appWs.callZome({
              cell_id: cellId,
              zome_name: 'content_store',
              fn_name: 'create_path',
              payload: {
                id: pathData.id,
                version: pathData.version || '1.0.0',
                title: pathData.title,
                description: pathData.description || '',
                purpose: pathData.purpose || '',
                difficulty: pathData.difficulty || 'beginner',
                estimated_duration: pathData.estimatedDuration || '',
                tags: pathData.tags || [],
                visibility: pathData.visibility || 'public',
                metadata_json: JSON.stringify({}),
              },
            });
            results.pathsSeeded++;

            // Add path steps
            if (pathData.steps && Array.isArray(pathData.steps)) {
              for (let i = 0; i < pathData.steps.length; i++) {
                const step = pathData.steps[i];
                await appWs.callZome({
                  cell_id: cellId,
                  zome_name: 'content_store',
                  fn_name: 'add_path_step',
                  payload: {
                    path_id: pathData.id,
                    order_index: step.order ?? i,
                    step_type: step.stepType || 'content',
                    resource_id: step.resourceId,
                    step_title: step.stepTitle || `Step ${i + 1}`,
                    step_narrative: step.stepNarrative || '',
                    is_optional: step.optional || false,
                  },
                });
              }
            }

            if (config.verbose) {
              console.log(`  ✓ ${pathData.id} (${pathData.steps?.length || 0} steps)`);
            }
          } catch (e) {
            results.pathErrors++;
            const error = e instanceof Error ? e.message : 'Unknown error';
            if (error.includes('already exists')) {
              if (config.verbose) {
                console.log(`  ⏭️  ${file} (already exists)`);
              }
            } else {
              console.error(`  ❌ ${file}: ${error}`);
            }
          }
        }

        await appWs.client.close();
        await adminWs.client.close();
      } catch (e) {
        console.error(`\n❌ Holochain error: ${e instanceof Error ? e.message : 'Unknown'}`);
      }
    }
  } else {
    console.log('\n⏭️  Skipping DNA seeding (--skip-dna)');
  }

  // =========================================================================
  // Summary
  // =========================================================================
  results.endTime = Date.now();
  const duration = (results.endTime - results.startTime) / 1000;

  console.log('\n' + '═'.repeat(70));
  console.log('SEEDING COMPLETE');
  console.log('═'.repeat(70));
  console.log(`\nDuration: ${duration.toFixed(1)}s`);
  console.log(`\nDNA Results:`);
  console.log(`  Content: ${results.contentSeeded} seeded, ${results.contentErrors} errors`);
  console.log(`  Paths:   ${results.pathsSeeded} seeded, ${results.pathErrors} errors`);
  console.log(`\nBlob Results:`);
  console.log(`  Extracted: ${results.blobsExtracted}`);
  console.log(`  Pushed:    ${results.blobsPushed}`);
  console.log(`  Errors:    ${results.blobErrors}`);
  console.log('═'.repeat(70));

  return results;
}

// =============================================================================
// CLI Entry Point
// =============================================================================

async function main() {
  const args = process.argv.slice(2);

  const config: SeedConfig = {
    adminUrl: process.env.HOLOCHAIN_ADMIN_URL || 'ws://localhost:8888',
    appId: process.env.HOLOCHAIN_APP_ID || 'elohim',
    doorwayUrl: process.env.DOORWAY_URL || 'http://localhost:3000',
    doorwayApiKey: process.env.DOORWAY_API_KEY,
    contentDir: process.env.CONTENT_DIR || path.join(__dirname, '../../data/lamad/content'),
    pathsDir: process.env.PATHS_DIR || path.join(__dirname, '../../data/lamad/paths'),
    dryRun: process.env.DRY_RUN === 'true' || args.includes('--dry-run'),
    verbose: args.includes('-v') || args.includes('--verbose'),
    validateOnly: args.includes('--validate-only'),
    skipBlobs: args.includes('--skip-blobs'),
    skipDna: args.includes('--skip-dna'),
  };

  if (args.includes('--help') || args.includes('-h')) {
    console.log(`
Production Seeder - Dual-path seeding for DNA + Projection Cache

Usage:
  npx tsx src/seed-production.ts [options]

Options:
  --dry-run        Validate and log without actually seeding
  --validate-only  Only run validation, don't seed
  --skip-blobs     Skip blob extraction and cache seeding
  --skip-dna       Skip DNA seeding (blobs only)
  -v, --verbose    Verbose output
  -h, --help       Show this help

Environment:
  HOLOCHAIN_ADMIN_URL  Holochain admin websocket URL (default: ws://localhost:8888)
  HOLOCHAIN_APP_ID     Holochain app ID (default: elohim)
  DOORWAY_URL          Doorway base URL (default: http://localhost:3000)
  DOORWAY_API_KEY      API key for doorway admin operations
  CONTENT_DIR          Content directory (default: ../data/lamad/content)
  PATHS_DIR            Paths directory (default: ../data/lamad/paths)
  DRY_RUN              Set to 'true' for dry run mode
`);
    process.exit(0);
  }

  try {
    const results = await seedProduction(config);

    const hasErrors = results.contentErrors > 0 ||
                     results.pathErrors > 0 ||
                     results.blobErrors > 0;

    process.exit(hasErrors ? 1 : 0);
  } catch (e) {
    console.error(`\n❌ Fatal error: ${e instanceof Error ? e.message : 'Unknown'}`);
    process.exit(1);
  }
}

main();
