/**
 * Holochain DNA Migration Tool
 *
 * Migrates data from a previous DNA version to the current version.
 * Uses bridge calls between two DNA roles installed in the same conductor.
 *
 * Usage:
 *   npx tsx src/migrate.ts                    # Migrate from elohim-previous to elohim
 *   npx tsx src/migrate.ts --dry-run          # Show what would be migrated
 *   npx tsx src/migrate.ts --verify-only      # Just verify existing migration
 *   npx tsx src/migrate.ts --source elohim-v1  # Specify source role name
 *
 * Prerequisites:
 *   1. Both DNAs bundled in happ.yaml with role names
 *   2. hApp installed with both DNA roles active
 *   3. Export functions available in source DNA
 *   4. Import functions available in target DNA
 */

import { AdminWebsocket, AppWebsocket, encodeHashToBase64, CellId } from '@holochain/client';
import * as fs from 'fs';

// Configuration
const HC_PORTS_FILE = process.env.HC_PORTS_FILE || '/projects/elohim/holochain/local-dev/.hc_ports';
const APP_ID = process.env.HOLOCHAIN_APP_ID || 'elohim';
const SOURCE_ROLE = process.env.SOURCE_ROLE || 'elohim-previous';
const TARGET_ROLE = process.env.TARGET_ROLE || 'elohim';
const ZOME_NAME = 'content_store';

// Types matching the Holochain zome migration module
interface MigrationReport {
  source_version: string;
  target_version: string;
  started_at: string;
  completed_at: string | null;
  content_migrated: number;
  content_failed: number;
  paths_migrated: number;
  paths_failed: number;
  mastery_migrated: number;
  mastery_failed: number;
  progress_migrated: number;
  progress_failed: number;
  errors: string[];
  verification: MigrationVerification;
}

interface MigrationVerification {
  passed: boolean;
  content_count_match: boolean;
  path_count_match: boolean;
  reference_integrity: boolean;
  notes: string[];
}

interface MigrationCounts {
  content_count: number;
  path_count: number;
  mastery_count: number;
  progress_count: number;
}

interface Content {
  id: string;
  content_type: string;
  title: string;
  description: string;
  content: string;
  content_format: string;
  tags: string[];
  source_path: string | null;
  related_node_ids: string[];
  reach: string;
  metadata_json: string;
  created_at: string;
  updated_at: string;
}

interface ContentOutput {
  action_hash: Uint8Array;
  entry_hash: Uint8Array;
  content: Content;
}

interface PathWithStepsExport {
  path: any;
  steps: any[];
}

interface MigrationExport {
  schema_version: string;
  exported_at: string;
  content: ContentOutput[];
  paths: PathWithStepsExport[];
  mastery: any[];
  progress: any[];
}

/**
 * Read Holochain ports from .hc_ports file
 */
function readHcPorts(): { adminPort: number; appPort: number } {
  try {
    const content = fs.readFileSync(HC_PORTS_FILE, 'utf-8');
    const adminMatch = content.match(/admin_port=(\d+)/);
    const appMatch = content.match(/app_port=(\d+)/);

    if (!adminMatch || !appMatch) {
      throw new Error('Could not parse .hc_ports file');
    }

    return {
      adminPort: parseInt(adminMatch[1], 10),
      appPort: parseInt(appMatch[1], 10),
    };
  } catch (error) {
    console.error(`Could not read ${HC_PORTS_FILE}:`, error);
    console.log('   Falling back to default ports (4444, 4445)');
    return { adminPort: 4444, appPort: 4445 };
  }
}

/**
 * Resolve app WebSocket URL based on admin URL and dynamic port.
 */
function resolveAppUrl(adminUrl: string, port: number): string {
  if (!adminUrl.includes('localhost') && !adminUrl.includes('127.0.0.1')) {
    const url = new URL(adminUrl);
    const baseUrl = `${url.protocol}//${url.host}`;
    const apiKey = url.searchParams.get('apiKey');
    const apiKeyParam = apiKey ? `?apiKey=${encodeURIComponent(apiKey)}` : '';
    return `${baseUrl}/app/${port}${apiKeyParam}`;
  }
  return `ws://localhost:${port}`;
}

/**
 * Parse command line arguments
 */
function parseArgs(): {
  dryRun: boolean;
  verifyOnly: boolean;
  sourceRole: string;
  targetRole: string;
} {
  const args = process.argv.slice(2);
  return {
    dryRun: args.includes('--dry-run'),
    verifyOnly: args.includes('--verify-only'),
    sourceRole: args.find(a => a.startsWith('--source='))?.split('=')[1] || SOURCE_ROLE,
    targetRole: args.find(a => a.startsWith('--target='))?.split('=')[1] || TARGET_ROLE,
  };
}

/**
 * Main migration function
 */
async function main() {
  const options = parseArgs();

  console.log('='.repeat(60));
  console.log('  Holochain DNA Migration Tool');
  console.log('='.repeat(60));
  console.log(`  Source Role: ${options.sourceRole}`);
  console.log(`  Target Role: ${options.targetRole}`);
  console.log(`  Dry Run: ${options.dryRun}`);
  console.log(`  Verify Only: ${options.verifyOnly}`);
  console.log('='.repeat(60));

  // Read ports
  const ports = readHcPorts();
  const ADMIN_WS_URL = process.env.HOLOCHAIN_ADMIN_URL || `ws://localhost:${ports.adminPort}`;

  // Connect to admin websocket
  console.log('\nConnecting to Holochain admin...');
  console.log(`Admin WebSocket: ${ADMIN_WS_URL}`);

  let adminWs: AdminWebsocket;
  try {
    adminWs = await AdminWebsocket.connect({ url: new URL(ADMIN_WS_URL) });
    console.log('Connected to admin WebSocket');
  } catch (error) {
    console.error('Failed to connect to admin WebSocket:', error);
    console.log('\nIs Holochain running? Try: npm run hc:start');
    process.exit(1);
  }

  // List installed apps
  console.log('\nListing installed apps...');
  const apps = await adminWs.listApps({});
  const app = apps.find(a => a.installed_app_id === APP_ID);

  if (!app) {
    console.error(`App "${APP_ID}" not found`);
    console.log('Available apps:', apps.map(a => a.installed_app_id).join(', '));
    process.exit(1);
  }
  console.log(`Found app: ${APP_ID}`);

  // Get cell IDs for both roles
  const cells = app.cell_info;
  console.log('Available roles:', Object.keys(cells).join(', '));

  // Check if source role exists
  const sourceCell = cells[options.sourceRole];
  if (!sourceCell) {
    console.error(`\nSource role "${options.sourceRole}" not found`);
    console.log('\nThis migration tool requires both DNA versions installed.');
    console.log('To set up migration:');
    console.log('  1. Add both DNAs to happ.yaml with separate role names');
    console.log('  2. Reinstall the hApp');
    console.log('  3. Run this tool again');
    console.log('\nExample happ.yaml:');
    console.log('  roles:');
    console.log('    - name: elohim           # Current version');
    console.log('      dna:');
    console.log('        bundled: ./elohim.dna');
    console.log('    - name: elohim-previous  # Previous version');
    console.log('      dna:');
    console.log('        bundled: ./archive/elohim-v1.dna');
    process.exit(1);
  }

  const targetCell = cells[options.targetRole];
  if (!targetCell) {
    console.error(`Target role "${options.targetRole}" not found`);
    process.exit(1);
  }

  // Extract cell IDs
  const sourceCellId = extractCellId(sourceCell);
  const targetCellId = extractCellId(targetCell);

  if (!sourceCellId || !targetCellId) {
    console.error('Could not extract cell IDs from app info');
    process.exit(1);
  }

  console.log(`Source cell: ${encodeHashToBase64(sourceCellId[0]).slice(0, 20)}...`);
  console.log(`Target cell: ${encodeHashToBase64(targetCellId[0]).slice(0, 20)}...`);

  // Get app auth token
  console.log('\nGetting app auth token...');
  const token = await adminWs.issueAppAuthenticationToken({
    installed_app_id: APP_ID,
    single_use: false,
    expiry_seconds: 3600,
  });
  console.log('Got auth token');

  // Authorize signing credentials
  console.log('Authorizing signing credentials...');
  await adminWs.authorizeSigningCredentials(targetCellId);
  console.log('Signing credentials authorized');

  // Get app interface
  console.log('\nSetting up app interface...');
  let appPort: number;
  const existingInterfaces = await adminWs.listAppInterfaces();
  if (existingInterfaces.length > 0) {
    appPort = existingInterfaces[0].port;
    console.log(`Using existing app interface on port ${appPort}`);
  } else {
    const { port } = await adminWs.attachAppInterface({ allowed_origins: '*' });
    appPort = port;
    console.log(`Created app interface on port ${appPort}`);
  }

  const appWsUrl = process.env.HOLOCHAIN_APP_URL || resolveAppUrl(ADMIN_WS_URL, appPort);
  console.log(`App WebSocket: ${appWsUrl}`);

  // Connect to app websocket
  console.log('\nConnecting to Holochain app...');
  let appWs: AppWebsocket;
  try {
    appWs = await AppWebsocket.connect({
      url: new URL(appWsUrl),
      wsClientOptions: { origin: 'http://localhost' },
      token: token.token,
    });
    console.log('Connected to app WebSocket');
  } catch (error) {
    console.error('Failed to connect to app WebSocket:', error);
    process.exit(1);
  }

  // If verify-only, just check existing data
  if (options.verifyOnly) {
    await verifyMigration(appWs, sourceCellId, targetCellId);
    await appWs.client.close();
    await adminWs.client.close();
    return;
  }

  // Step 1: Export data from source DNA
  console.log('\n' + '='.repeat(60));
  console.log('  Step 1: Export data from source DNA');
  console.log('='.repeat(60));

  let exportData: MigrationExport;
  try {
    // Get schema version
    const schemaVersion = await appWs.callZome({
      cell_id: sourceCellId,
      zome_name: ZOME_NAME,
      fn_name: 'export_schema_version',
      payload: null,
    });
    console.log(`Source schema version: ${schemaVersion}`);

    // Export all data
    console.log('Exporting all data...');
    exportData = await appWs.callZome({
      cell_id: sourceCellId,
      zome_name: ZOME_NAME,
      fn_name: 'export_for_migration',
      payload: null,
    }) as MigrationExport;

    console.log(`Exported:`);
    console.log(`  - ${exportData.content.length} content items`);
    console.log(`  - ${exportData.paths.length} learning paths`);
    console.log(`  - ${exportData.mastery.length} mastery records`);
    console.log(`  - ${exportData.progress.length} progress records`);
  } catch (error: any) {
    console.error('Failed to export data from source:', error.message || error);
    console.log('\nMake sure the source DNA has export functions:');
    console.log('  - export_schema_version()');
    console.log('  - export_for_migration()');
    process.exit(1);
  }

  // Check if there's anything to migrate
  if (exportData.content.length === 0 && exportData.paths.length === 0) {
    console.log('\nNo data to migrate from source DNA');
    await appWs.client.close();
    await adminWs.client.close();
    return;
  }

  // Dry run - just show what would be migrated
  if (options.dryRun) {
    console.log('\n' + '='.repeat(60));
    console.log('  DRY RUN - No changes will be made');
    console.log('='.repeat(60));
    console.log('\nContent to migrate:');
    for (const item of exportData.content.slice(0, 10)) {
      console.log(`  - ${item.content.id}: ${item.content.title.slice(0, 50)}...`);
    }
    if (exportData.content.length > 10) {
      console.log(`  ... and ${exportData.content.length - 10} more`);
    }
    console.log('\nPaths to migrate:');
    for (const item of exportData.paths.slice(0, 5)) {
      console.log(`  - ${item.path.id}: ${item.path.title} (${item.steps.length} steps)`);
    }
    if (exportData.paths.length > 5) {
      console.log(`  ... and ${exportData.paths.length - 5} more`);
    }

    await appWs.client.close();
    await adminWs.client.close();
    return;
  }

  // Step 2: Import data into target DNA
  console.log('\n' + '='.repeat(60));
  console.log('  Step 2: Import data into target DNA');
  console.log('='.repeat(60));

  const report: MigrationReport = {
    source_version: exportData.schema_version,
    target_version: 'current',
    started_at: new Date().toISOString(),
    completed_at: null,
    content_migrated: 0,
    content_failed: 0,
    paths_migrated: 0,
    paths_failed: 0,
    mastery_migrated: 0,
    mastery_failed: 0,
    progress_migrated: 0,
    progress_failed: 0,
    errors: [],
    verification: {
      passed: false,
      content_count_match: false,
      path_count_match: false,
      reference_integrity: false,
      notes: [],
    },
  };

  // Import content
  console.log('\nImporting content...');
  for (const item of exportData.content) {
    try {
      // Check if already exists
      const existing = await appWs.callZome({
        cell_id: targetCellId,
        zome_name: ZOME_NAME,
        fn_name: 'get_content_by_id',
        payload: { id: item.content.id },
      });

      if (existing) {
        report.content_migrated++;
        console.log(`  [skip] ${item.content.id.slice(0, 40)}... (exists)`);
        continue;
      }

      // Create in target
      await appWs.callZome({
        cell_id: targetCellId,
        zome_name: ZOME_NAME,
        fn_name: 'create_content',
        payload: {
          id: item.content.id,
          content_type: item.content.content_type,
          title: item.content.title,
          description: item.content.description,
          content: item.content.content,
          content_format: item.content.content_format,
          tags: item.content.tags,
          source_path: item.content.source_path,
          related_node_ids: item.content.related_node_ids,
          reach: item.content.reach,
          metadata_json: item.content.metadata_json,
        },
      });

      report.content_migrated++;
      console.log(`  [ok] ${item.content.id.slice(0, 40)}...`);
    } catch (error: any) {
      report.content_failed++;
      report.errors.push(`Content ${item.content.id}: ${error.message || error}`);
      console.error(`  [err] ${item.content.id}: ${error.message || error}`);
    }
  }

  // Import paths
  console.log('\nImporting learning paths...');
  for (const pathExport of exportData.paths) {
    try {
      // Check if path exists
      const existing = await appWs.callZome({
        cell_id: targetCellId,
        zome_name: ZOME_NAME,
        fn_name: 'get_path_by_id',
        payload: pathExport.path.id,
      });

      if (existing) {
        report.paths_migrated++;
        console.log(`  [skip] ${pathExport.path.id} (exists)`);
        continue;
      }

      // Create path
      await appWs.callZome({
        cell_id: targetCellId,
        zome_name: ZOME_NAME,
        fn_name: 'create_learning_path',
        payload: {
          id: pathExport.path.id,
          version: pathExport.path.version,
          title: pathExport.path.title,
          description: pathExport.path.description,
          purpose: pathExport.path.purpose,
          difficulty: pathExport.path.difficulty,
          estimated_duration: pathExport.path.estimated_duration,
          visibility: pathExport.path.visibility,
          path_type: pathExport.path.path_type,
          tags: pathExport.path.tags,
        },
      });

      // Add steps
      for (const step of pathExport.steps) {
        await appWs.callZome({
          cell_id: targetCellId,
          zome_name: ZOME_NAME,
          fn_name: 'add_path_step',
          payload: {
            path_id: pathExport.path.id,
            order_index: step.order_index,
            step_type: step.step_type,
            resource_id: step.resource_id,
            step_title: step.step_title,
            step_narrative: step.step_narrative,
            is_optional: step.is_optional,
          },
        });
      }

      report.paths_migrated++;
      console.log(`  [ok] ${pathExport.path.id} (${pathExport.steps.length} steps)`);
    } catch (error: any) {
      report.paths_failed++;
      report.errors.push(`Path ${pathExport.path.id}: ${error.message || error}`);
      console.error(`  [err] ${pathExport.path.id}: ${error.message || error}`);
    }
  }

  // Step 3: Verify migration
  console.log('\n' + '='.repeat(60));
  console.log('  Step 3: Verify migration');
  console.log('='.repeat(60));

  try {
    const expectedCounts: MigrationCounts = {
      content_count: exportData.content.length,
      path_count: exportData.paths.length,
      mastery_count: exportData.mastery.length,
      progress_count: exportData.progress.length,
    };

    const verification = await appWs.callZome({
      cell_id: targetCellId,
      zome_name: ZOME_NAME,
      fn_name: 'verify_migration',
      payload: expectedCounts,
    }) as MigrationVerification;

    report.verification = verification;
    console.log(`Verification passed: ${verification.passed}`);
    console.log(`  Content count match: ${verification.content_count_match}`);
    console.log(`  Path count match: ${verification.path_count_match}`);
    console.log(`  Reference integrity: ${verification.reference_integrity}`);

    if (verification.notes.length > 0) {
      console.log('Notes:');
      for (const note of verification.notes) {
        console.log(`  - ${note}`);
      }
    }
  } catch (error: any) {
    console.error('Verification failed:', error.message || error);
    report.verification.notes.push(`Verification error: ${error.message || error}`);
  }

  // Complete report
  report.completed_at = new Date().toISOString();

  // Summary
  console.log('\n' + '='.repeat(60));
  console.log('  Migration Summary');
  console.log('='.repeat(60));
  console.log(`Content: ${report.content_migrated} migrated, ${report.content_failed} failed`);
  console.log(`Paths: ${report.paths_migrated} migrated, ${report.paths_failed} failed`);
  console.log(`Mastery: ${report.mastery_migrated} migrated, ${report.mastery_failed} failed`);
  console.log(`Progress: ${report.progress_migrated} migrated, ${report.progress_failed} failed`);

  if (report.errors.length > 0) {
    console.log('\nErrors:');
    for (const error of report.errors.slice(0, 10)) {
      console.log(`  - ${error}`);
    }
    if (report.errors.length > 10) {
      console.log(`  ... and ${report.errors.length - 10} more errors`);
    }
  }

  const success = report.content_failed === 0 && report.paths_failed === 0;
  console.log(`\nMigration ${success ? 'SUCCESSFUL' : 'COMPLETED WITH ERRORS'}`);

  // Save report
  const reportFile = `/tmp/migration-report-${Date.now()}.json`;
  fs.writeFileSync(reportFile, JSON.stringify(report, null, 2));
  console.log(`Report saved to: ${reportFile}`);

  // Cleanup
  await appWs.client.close();
  await adminWs.client.close();

  process.exit(success ? 0 : 1);
}

/**
 * Extract CellId from cell info
 */
function extractCellId(cellInfo: any): CellId | null {
  if (Array.isArray(cellInfo)) {
    for (const info of cellInfo) {
      if (info.provisioned) {
        return info.provisioned.cell_id;
      }
      if (info.cloned) {
        return info.cloned.cell_id;
      }
    }
  }
  return null;
}

/**
 * Verify migration only (no import)
 */
async function verifyMigration(
  appWs: AppWebsocket,
  sourceCellId: CellId,
  targetCellId: CellId
) {
  console.log('\n' + '='.repeat(60));
  console.log('  Verification Only Mode');
  console.log('='.repeat(60));

  // Get counts from source
  console.log('\nGetting counts from source DNA...');
  let sourceExport: MigrationExport;
  try {
    sourceExport = await appWs.callZome({
      cell_id: sourceCellId,
      zome_name: ZOME_NAME,
      fn_name: 'export_for_migration',
      payload: null,
    }) as MigrationExport;

    console.log(`Source has:`);
    console.log(`  - ${sourceExport.content.length} content items`);
    console.log(`  - ${sourceExport.paths.length} learning paths`);
  } catch (error: any) {
    console.error('Failed to get source counts:', error.message || error);
    return;
  }

  // Verify target
  console.log('\nVerifying target DNA...');
  try {
    const expectedCounts: MigrationCounts = {
      content_count: sourceExport.content.length,
      path_count: sourceExport.paths.length,
      mastery_count: sourceExport.mastery.length,
      progress_count: sourceExport.progress.length,
    };

    const verification = await appWs.callZome({
      cell_id: targetCellId,
      zome_name: ZOME_NAME,
      fn_name: 'verify_migration',
      payload: expectedCounts,
    }) as MigrationVerification;

    console.log(`\nVerification Result:`);
    console.log(`  Passed: ${verification.passed}`);
    console.log(`  Content count match: ${verification.content_count_match}`);
    console.log(`  Path count match: ${verification.path_count_match}`);
    console.log(`  Reference integrity: ${verification.reference_integrity}`);

    if (verification.notes.length > 0) {
      console.log('\nNotes:');
      for (const note of verification.notes) {
        console.log(`  - ${note}`);
      }
    }
  } catch (error: any) {
    console.error('Verification failed:', error.message || error);
  }
}

// Run
main().catch(console.error);
