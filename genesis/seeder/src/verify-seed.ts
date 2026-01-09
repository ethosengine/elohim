/**
 * Post-Seeding Verification Script
 *
 * Verifies that seeding was successful by checking:
 * 1. Content count >= minimum threshold
 * 2. Path count >= minimum threshold
 * 3. Sample content IDs exist
 *
 * Usage:
 *   HOLOCHAIN_ADMIN_URL=ws://localhost:8888 npx tsx src/verify-seed.ts
 *
 * Environment Variables:
 *   HOLOCHAIN_ADMIN_URL - Holochain admin WebSocket URL (required)
 *   HOLOCHAIN_APP_ID    - Holochain app ID (default: elohim)
 *   MIN_CONTENT_COUNT   - Minimum content count (default: 3400)
 *   MIN_PATH_COUNT      - Minimum path count (default: 5)
 *
 * Exit Codes:
 *   0 - Verification passed
 *   1 - Verification failed
 *   2 - Connection failed
 */

import { AdminWebsocket, AppWebsocket, type CellId } from '@holochain/client';
import { SeedingVerification, ContentStats, PathIndex } from './verification.js';

// =============================================================================
// Configuration
// =============================================================================

const ADMIN_URL = process.env.HOLOCHAIN_ADMIN_URL;
const APP_ID = process.env.HOLOCHAIN_APP_ID || 'elohim';
const ROLE_NAME = process.env.HOLOCHAIN_ROLE || 'lamad';
const ZOME_NAME = 'content_store';

// Thresholds
const MIN_CONTENT_COUNT = parseInt(process.env.MIN_CONTENT_COUNT || '3400', 10);
const MIN_PATH_COUNT = parseInt(process.env.MIN_PATH_COUNT || '5', 10);

// Sample content IDs to verify (well-known content that should exist)
const SAMPLE_CONTENT_IDS = [
  'manifesto',                    // Core Elohim manifesto
  'elohim-lamad',                 // Main learning path intro
  'governance-epic',              // Governance epic
  'quiz-manifesto-foundations',   // Foundation quiz
  'quiz-governance-foundations',  // Governance quiz
];

// =============================================================================
// Main
// =============================================================================

async function main(): Promise<void> {
  console.log('═'.repeat(70));
  console.log('🔍 POST-SEEDING VERIFICATION');
  console.log('═'.repeat(70));
  console.log(`   Admin URL:        ${ADMIN_URL}`);
  console.log(`   App ID:           ${APP_ID}`);
  console.log(`   Role:             ${ROLE_NAME}`);
  console.log(`   Min Content:      ${MIN_CONTENT_COUNT}`);
  console.log(`   Min Paths:        ${MIN_PATH_COUNT}`);
  console.log('─'.repeat(70));

  if (!ADMIN_URL) {
    console.error('❌ HOLOCHAIN_ADMIN_URL environment variable is required');
    process.exit(2);
  }

  let adminWs: AdminWebsocket | null = null;
  let appWs: AppWebsocket | null = null;

  try {
    // Connect to admin WebSocket
    console.log('\n1. Connecting to conductor...');
    adminWs = await AdminWebsocket.connect({
      url: new URL(ADMIN_URL),
      defaultTimeout: 30000,
    });
    console.log('   ✓ Admin connection established');

    // Get app info
    const apps = await adminWs.listApps({});
    const app = apps.find(a => a.installed_app_id === APP_ID);

    if (!app) {
      console.error(`❌ App '${APP_ID}' not found`);
      console.log('   Available apps:', apps.map(a => a.installed_app_id).join(', '));
      process.exit(2);
    }

    console.log(`   ✓ Found app '${APP_ID}'`);

    // Get cell ID
    const roleCell = app.cell_info[ROLE_NAME]?.[0];
    if (!roleCell || !('provisioned' in roleCell)) {
      console.error(`❌ Role '${ROLE_NAME}' not found or not provisioned`);
      process.exit(2);
    }

    const cellId = (roleCell as { provisioned: { cell_id: CellId } }).provisioned.cell_id;
    console.log(`   ✓ Cell ID: ${Buffer.from(cellId[0]).toString('hex').slice(0, 12)}...`);

    // Attach app interface
    const appInterfaces = await adminWs.listAppInterfaces();
    let appPort: number;

    if (appInterfaces.length > 0) {
      appPort = appInterfaces[0].port;
    } else {
      const attached = await adminWs.attachAppInterface({ port: 0, allowed_origins: '*' });
      appPort = attached.port;
    }

    // Connect app WebSocket
    const appUrl = ADMIN_URL.replace(/:\d+/, `:${appPort}`);
    appWs = await AppWebsocket.connect({
      url: new URL(appUrl),
      defaultTimeout: 30000,
    });
    console.log(`   ✓ App connection established (port ${appPort})`);

    // Run verification
    console.log('\n2. Checking content count...');
    const stats = await appWs.callZome({
      cell_id: cellId,
      zome_name: ZOME_NAME,
      fn_name: 'get_content_stats',
      payload: null,
    }) as ContentStats;

    console.log(`   Content count: ${stats.total_count}`);

    if (stats.total_count < MIN_CONTENT_COUNT) {
      console.error(`   ❌ Content count ${stats.total_count} < minimum ${MIN_CONTENT_COUNT}`);
      process.exit(1);
    }
    console.log(`   ✓ Content count >= ${MIN_CONTENT_COUNT}`);

    // Check paths
    console.log('\n3. Checking path count...');
    const paths = await appWs.callZome({
      cell_id: cellId,
      zome_name: ZOME_NAME,
      fn_name: 'get_all_paths',
      payload: null,
    }) as PathIndex;

    console.log(`   Path count: ${paths.total_count}`);

    if (paths.total_count < MIN_PATH_COUNT) {
      console.error(`   ❌ Path count ${paths.total_count} < minimum ${MIN_PATH_COUNT}`);
      process.exit(1);
    }
    console.log(`   ✓ Path count >= ${MIN_PATH_COUNT}`);

    // Sample verification
    console.log('\n4. Verifying sample content...');
    let samplesPassed = 0;
    let samplesFailed: string[] = [];

    for (const id of SAMPLE_CONTENT_IDS) {
      try {
        const result = await appWs.callZome({
          cell_id: cellId,
          zome_name: ZOME_NAME,
          fn_name: 'get_content_by_id',
          payload: { id },
        });

        if (result) {
          samplesPassed++;
          console.log(`   ✓ ${id}`);
        } else {
          samplesFailed.push(id);
          console.log(`   ✗ ${id} (not found)`);
        }
      } catch (error) {
        samplesFailed.push(id);
        console.log(`   ✗ ${id} (error: ${error})`);
      }
    }

    if (samplesFailed.length > 0) {
      console.warn(`   ⚠️ ${samplesFailed.length}/${SAMPLE_CONTENT_IDS.length} samples missing`);
    } else {
      console.log(`   ✓ All ${SAMPLE_CONTENT_IDS.length} samples verified`);
    }

    // Summary
    console.log('\n' + '═'.repeat(70));
    console.log('📊 VERIFICATION SUMMARY');
    console.log('═'.repeat(70));
    console.log(`   Content: ${stats.total_count} (min: ${MIN_CONTENT_COUNT}) ✓`);
    console.log(`   Paths:   ${paths.total_count} (min: ${MIN_PATH_COUNT}) ✓`);
    console.log(`   Samples: ${samplesPassed}/${SAMPLE_CONTENT_IDS.length}`);

    // Content type breakdown
    if (stats.by_type) {
      console.log('\n   Content by type:');
      const sortedTypes = Object.entries(stats.by_type)
        .sort((a, b) => b[1] - a[1])
        .slice(0, 10);
      for (const [type, count] of sortedTypes) {
        console.log(`     - ${type}: ${count}`);
      }
    }

    // Determine success
    const success = stats.total_count >= MIN_CONTENT_COUNT &&
                   paths.total_count >= MIN_PATH_COUNT &&
                   samplesFailed.length <= 2; // Allow up to 2 missing samples

    if (success) {
      console.log('\n   ✅ VERIFICATION PASSED');
      console.log('═'.repeat(70));
    } else {
      console.log('\n   ❌ VERIFICATION FAILED');
      console.log('═'.repeat(70));
      process.exit(1);
    }

  } catch (error) {
    console.error('\n❌ Verification error:', error);
    process.exit(2);
  } finally {
    if (appWs) {
      try { await appWs.client.close(); } catch {}
    }
    if (adminWs) {
      try { await adminWs.client.close(); } catch {}
    }
  }
}

main().catch(error => {
  console.error('Fatal error:', error);
  process.exit(2);
});
