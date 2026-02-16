#!/usr/bin/env npx tsx
/**
 * Holochain Diagnostics CLI
 *
 * Usage:
 *   npx tsx src/diagnose.ts                    # Full diagnostics
 *   npx tsx src/diagnose.ts --apps             # List installed apps
 *   npx tsx src/diagnose.ts --cells            # Show cell details
 *   npx tsx src/diagnose.ts --storage          # Check elohim-storage
 *   npx tsx src/diagnose.ts --content          # Check content counts
 *   npx tsx src/diagnose.ts --doorway          # Check doorway health
 *
 * Environment:
 *   HOLOCHAIN_ADMIN_URL - Admin WebSocket URL (required)
 *   DOORWAY_URL         - Doorway HTTP URL (for storage/health checks)
 */

import { AdminWebsocket, AppWebsocket, type CellInfo, type AppInfo } from '@holochain/client';

// =============================================================================
// Configuration
// =============================================================================

const ADMIN_URL = process.env.HOLOCHAIN_ADMIN_URL || process.env.DOORWAY_ADMIN_URL;
const DOORWAY_URL = process.env.DOORWAY_URL;
const APP_ID = process.env.HOLOCHAIN_APP_ID || 'elohim';

// Parse args
const args = process.argv.slice(2);
const showApps = args.includes('--apps') || args.length === 0;
const showCells = args.includes('--cells') || args.length === 0;
const showStorage = args.includes('--storage') || args.length === 0;
const showContent = args.includes('--content') || args.length === 0;
const showDoorway = args.includes('--doorway') || args.length === 0;
const showHelp = args.includes('--help') || args.includes('-h');

// =============================================================================
// Utilities
// =============================================================================

function formatBytes(bytes: Uint8Array | number[], maxLen = 8): string {
  const arr = Array.from(bytes);
  if (arr.length <= maxLen * 2) {
    return arr.map(b => b.toString(16).padStart(2, '0')).join('');
  }
  const prefix = arr.slice(0, maxLen).map(b => b.toString(16).padStart(2, '0')).join('');
  return `${prefix}...`;
}

function toUint8Array(val: any): Uint8Array {
  if (val instanceof Uint8Array) return val;
  if (val?.type === 'Buffer' && Array.isArray(val.data)) return new Uint8Array(val.data);
  if (ArrayBuffer.isView(val)) return new Uint8Array(val.buffer);
  if (Array.isArray(val)) return new Uint8Array(val);
  throw new Error('Cannot convert to Uint8Array');
}

async function fetchJson(url: string): Promise<any> {
  const response = await fetch(url, {
    headers: { 'Accept': 'application/json' }
  });
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}: ${response.statusText}`);
  }
  return response.json();
}

// =============================================================================
// Diagnostics
// =============================================================================

async function diagnoseApps(adminWs: AdminWebsocket): Promise<AppInfo[]> {
  console.log('\n📱 INSTALLED APPS');
  console.log('─'.repeat(60));

  const apps = await adminWs.listApps({});

  if (apps.length === 0) {
    console.log('   ⚠️  No apps installed');
    return [];
  }

  for (const app of apps) {
    const status = typeof app.status === 'string' ? app.status : JSON.stringify(app.status);
    const roles = Object.keys(app.cell_info);
    console.log(`   📦 ${app.installed_app_id}`);
    console.log(`      Status: ${status}`);
    console.log(`      Roles: ${roles.join(', ')}`);
  }

  return apps;
}

async function diagnoseCells(adminWs: AdminWebsocket, apps: AppInfo[]): Promise<void> {
  console.log('\n🧬 CELL DETAILS');
  console.log('─'.repeat(60));

  for (const app of apps) {
    console.log(`\n   App: ${app.installed_app_id}`);

    for (const [roleName, cells] of Object.entries(app.cell_info)) {
      console.log(`   ├─ Role: ${roleName}`);

      for (const cellInfo of cells as CellInfo[]) {
        // Cell info is a union type with different structures
        const cellType = Object.keys(cellInfo)[0];
        const cellData = (cellInfo as any)[cellType];

        if (cellType === 'provisioned') {
          const cellId = cellData.cell_id;
          const dnaHash = formatBytes(toUint8Array(cellId[0]));
          const agentKey = formatBytes(toUint8Array(cellId[1]));
          console.log(`   │  ├─ Type: provisioned ✅`);
          console.log(`   │  ├─ DNA: ${dnaHash}`);
          console.log(`   │  └─ Agent: ${agentKey}`);
        } else if (cellType === 'cloned') {
          console.log(`   │  └─ Type: cloned`);
        } else if (cellType === 'stem') {
          console.log(`   │  └─ Type: stem (not yet provisioned)`);
        } else {
          console.log(`   │  └─ Type: ${cellType}`);
        }
      }
    }
  }
}

async function diagnoseContent(adminWs: AdminWebsocket, apps: AppInfo[]): Promise<void> {
  console.log('\n📊 CONTENT COUNTS');
  console.log('─'.repeat(60));

  const app = apps.find(a => a.installed_app_id === APP_ID);
  if (!app) {
    console.log(`   ⚠️  App '${APP_ID}' not found`);
    return;
  }

  // Find a provisioned cell
  let cellId: any = null;
  let roleName: string | null = null;

  for (const [role, cells] of Object.entries(app.cell_info)) {
    for (const cellInfo of cells as CellInfo[]) {
      // Handle both cell formats: native { type: "provisioned", value: {...} } and JS { provisioned: {...} }
      if ('provisioned' in cellInfo) {
        cellId = (cellInfo as any).provisioned.cell_id;
        roleName = role;
        break;
      } else if ((cellInfo as any).type === 'provisioned') {
        cellId = (cellInfo as any).value.cell_id;
        roleName = role;
        break;
      }
    }
    if (cellId) break;
  }

  if (!cellId) {
    console.log('   ⚠️  No provisioned cell found');
    return;
  }

  try {
    // Get app interface
    const interfaces = await adminWs.listAppInterfaces();
    const appPort = interfaces.length > 0 ? interfaces[0].port : 4445;

    // Issue token
    const token = await adminWs.issueAppAuthenticationToken({
      installed_app_id: APP_ID,
      single_use: false,
      expiry_seconds: 60,
    });

    // Authorize signing
    const normalizedCellId: [Uint8Array, Uint8Array] = [
      toUint8Array(cellId[0]),
      toUint8Array(cellId[1]),
    ];
    await adminWs.authorizeSigningCredentials(normalizedCellId);

    // Derive app URL
    let appUrl: string;
    if (ADMIN_URL && !ADMIN_URL.includes('localhost') && !ADMIN_URL.includes('127.0.0.1')) {
      const url = new URL(ADMIN_URL);
      const apiKey = url.searchParams.get('apiKey');
      const apiKeyParam = apiKey ? `?apiKey=${encodeURIComponent(apiKey)}` : '';
      appUrl = `${url.protocol}//${url.host}/app/${appPort}${apiKeyParam}`;
    } else {
      appUrl = `ws://localhost:${appPort}`;
    }

    // Connect to app
    const appWs = await AppWebsocket.connect({
      url: new URL(appUrl),
      wsClientOptions: { origin: 'http://localhost' },
      token: token.token,
    });

    // Call get_content_count
    const zomeName = 'content_store';
    try {
      const countResult = await appWs.callZome({
        cell_id: normalizedCellId,
        zome_name: zomeName,
        fn_name: 'get_content_count',
        payload: null,
      });
      console.log(`   Content count: ${countResult}`);
    } catch (e: any) {
      // Try alternative function
      try {
        const allContent = await appWs.callZome({
          cell_id: normalizedCellId,
          zome_name: zomeName,
          fn_name: 'get_all_content',
          payload: null,
        });
        const count = Array.isArray(allContent) ? allContent.length : 0;
        console.log(`   Content count: ${count} (via get_all_content)`);
      } catch (e2: any) {
        console.log(`   ⚠️  Could not get content count: ${e.message}`);
      }
    }

    // Call get_all_paths
    try {
      const pathsResult = await appWs.callZome({
        cell_id: normalizedCellId,
        zome_name: zomeName,
        fn_name: 'get_all_paths',
        payload: null,
      });
      const paths = (pathsResult as any)?.paths || pathsResult || [];
      console.log(`   Path count: ${Array.isArray(paths) ? paths.length : 0}`);
    } catch (e: any) {
      console.log(`   ⚠️  Could not get path count: ${e.message}`);
    }

    await appWs.client.close();
  } catch (e: any) {
    console.log(`   ❌ Error: ${e.message}`);
  }
}

async function diagnoseDoorway(): Promise<void> {
  if (!DOORWAY_URL) {
    console.log('\n🌐 DOORWAY HEALTH');
    console.log('─'.repeat(60));
    console.log('   ⚠️  DOORWAY_URL not set');
    return;
  }

  console.log('\n🌐 DOORWAY HEALTH');
  console.log('─'.repeat(60));

  try {
    const health = await fetchJson(`${DOORWAY_URL}/health`);
    console.log(`   Healthy: ${health.healthy ? '✅ Yes' : '❌ No'}`);
    console.log(`   Version: ${health.version || 'unknown'}`);
    console.log(`   Mode: ${health.mode || 'unknown'}`);
    console.log(`   Cache: ${health.cacheEnabled ? 'enabled' : 'disabled'}`);

    if (health.conductor) {
      console.log(`   Conductor: ${health.conductor.connected ? '✅ connected' : '❌ disconnected'}`);
      console.log(`   Workers: ${health.conductor.connected_workers}/${health.conductor.total_workers}`);
    }
  } catch (e: any) {
    console.log(`   ❌ Error: ${e.message}`);
  }
}

async function diagnoseStorage(): Promise<void> {
  if (!DOORWAY_URL) {
    console.log('\n💾 ELOHIM-STORAGE');
    console.log('─'.repeat(60));
    console.log('   ⚠️  DOORWAY_URL not set');
    return;
  }

  console.log('\n💾 ELOHIM-STORAGE');
  console.log('─'.repeat(60));

  // Check import batches
  try {
    const batches = await fetchJson(`${DOORWAY_URL}/import/batches`);
    console.log(`   Active batches: ${batches.total || 0}`);

    if (batches.batches && batches.batches.length > 0) {
      for (const batch of batches.batches.slice(0, 5)) {
        console.log(`   ├─ ${batch.batch_id}: ${batch.status} (${batch.processed_count}/${batch.total_items})`);
      }
      if (batches.batches.length > 5) {
        console.log(`   └─ ... and ${batches.batches.length - 5} more`);
      }
    }
  } catch (e: any) {
    if (e.message.includes('404')) {
      console.log('   ⚠️  Import API not available (ENABLE_IMPORT_API=false?)');
    } else {
      console.log(`   ⚠️  Could not check batches: ${e.message}`);
    }
  }
}

// =============================================================================
// Main
// =============================================================================

async function main() {
  if (showHelp) {
    console.log(`
Holochain Diagnostics CLI

Usage:
  npx tsx src/diagnose.ts [options]

Options:
  --apps      List installed apps
  --cells     Show cell details
  --storage   Check elohim-storage import batches
  --content   Check content counts via zome calls
  --doorway   Check doorway health
  --help      Show this help

Environment:
  HOLOCHAIN_ADMIN_URL   Admin WebSocket URL (required)
  DOORWAY_URL           Doorway HTTP URL (for storage/health checks)
  HOLOCHAIN_APP_ID      App ID to check (default: elohim)

Examples:
  # Full diagnostics on dev
  HOLOCHAIN_ADMIN_URL='wss://doorway-alpha.elohim.host?apiKey=dev-elohim-auth-2024' \\
  DOORWAY_URL='https://doorway-alpha.elohim.host' \\
  npx tsx src/diagnose.ts

  # Quick app/cell check
  HOLOCHAIN_ADMIN_URL='ws://localhost:4444' npx tsx src/diagnose.ts --apps --cells
`);
    return;
  }

  console.log('═'.repeat(60));
  console.log('  HOLOCHAIN DIAGNOSTICS');
  console.log('═'.repeat(60));

  if (!ADMIN_URL) {
    console.log('\n❌ HOLOCHAIN_ADMIN_URL not set');
    console.log('   Set environment variable and try again');
    process.exit(1);
  }

  console.log(`\n📡 Connecting to: ${ADMIN_URL.replace(/apiKey=[^&]+/, 'apiKey=***')}`);

  let adminWs: AdminWebsocket;
  try {
    adminWs = await AdminWebsocket.connect({
      url: new URL(ADMIN_URL),
      wsClientOptions: { origin: 'http://localhost' },
    });
  } catch (e: any) {
    console.log(`\n❌ Connection failed: ${e.message}`);
    process.exit(1);
  }

  console.log('   ✅ Connected\n');

  let apps: AppInfo[] = [];

  if (showApps) {
    apps = await diagnoseApps(adminWs);
  } else {
    apps = await adminWs.listApps({});
  }

  if (showCells) {
    await diagnoseCells(adminWs, apps);
  }

  if (showContent) {
    await diagnoseContent(adminWs, apps);
  }

  if (showDoorway) {
    await diagnoseDoorway();
  }

  if (showStorage) {
    await diagnoseStorage();
  }

  await adminWs.client.close();

  console.log('\n' + '═'.repeat(60));
}

main().catch(e => {
  console.error('Fatal error:', e.message);
  process.exit(1);
});
