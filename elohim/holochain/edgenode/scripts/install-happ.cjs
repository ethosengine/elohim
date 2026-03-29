#!/usr/bin/env node
/**
 * hApp Installation Script
 *
 * Waits for conductor to be ready, then installs the hApp if not already installed.
 * Runs as a sidecar container in Kubernetes.
 *
 * Environment variables:
 *   CONDUCTOR_URL - WebSocket URL for admin interface (default: ws://localhost:4444)
 *   HAPP_PATH - Path to .happ file (default: /opt/holochain/elohim.happ)
 *   APP_ID - Installed app ID (default: elohim)
 *   MAX_RETRIES - Maximum connection retries (default: 30)
 *   RETRY_DELAY_MS - Delay between retries in ms (default: 2000)
 */

const { AdminWebsocket } = require('@holochain/client');

const CONDUCTOR_URL = process.env.CONDUCTOR_URL || 'ws://localhost:4444';
const HAPP_PATH = process.env.HAPP_PATH || '/opt/holochain/elohim.happ';
const APP_ID = process.env.APP_ID || 'elohim';
const MAX_RETRIES = parseInt(process.env.MAX_RETRIES || '30', 10);
const RETRY_DELAY_MS = parseInt(process.env.RETRY_DELAY_MS || '2000', 10);

async function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

const APP_INTERFACE_PORT = parseInt(process.env.APP_INTERFACE_PORT || '4445', 10);

/**
 * Ensure app interface is attached on port 4445
 * This is idempotent - safe to call multiple times
 */
async function ensureAppInterface(adminWs) {
  console.log(`Ensuring app interface on port ${APP_INTERFACE_PORT}...`);
  try {
    await adminWs.attachAppInterface({ port: APP_INTERFACE_PORT, allowed_origins: '*' });
    console.log(`App interface attached on port ${APP_INTERFACE_PORT}`);
  } catch (err) {
    // Interface may already exist - that's fine
    if (err.message && err.message.includes('already in use')) {
      console.log(`App interface already exists on port ${APP_INTERFACE_PORT}`);
    } else {
      throw err;
    }
  }
}

async function waitForConductor() {
  console.log(`Waiting for conductor at ${CONDUCTOR_URL}...`);

  for (let attempt = 1; attempt <= MAX_RETRIES; attempt++) {
    try {
      const adminWs = await AdminWebsocket.connect({
        url: new URL(CONDUCTOR_URL),
        wsClientOptions: { origin: 'http://localhost' }
      });
      console.log(`Connected to conductor on attempt ${attempt}`);
      return adminWs;
    } catch (err) {
      console.log(`Attempt ${attempt}/${MAX_RETRIES} failed: ${err.message}`);
      if (attempt < MAX_RETRIES) {
        await sleep(RETRY_DELAY_MS);
      }
    }
  }

  throw new Error(`Failed to connect to conductor after ${MAX_RETRIES} attempts`);
}

async function installHapp(adminWs) {
  // Check if app is already installed
  const apps = await adminWs.listApps({});
  const existingApp = apps.find(app => app.installed_app_id === APP_ID);

  if (existingApp) {
    console.log(`App '${APP_ID}' is already installed (status: ${JSON.stringify(existingApp.status)})`);

    // Check for stale install — verify all expected roles have provisioned cells
    const expectedRoles = ['lamad', 'infrastructure', 'imagodei', 'mishpat', 'node_registry'];
    const cellInfo = existingApp.cell_info || {};
    const installedRoles = Object.keys(cellInfo);

    // Log full cell_info structure for debugging
    console.log(`Installed roles: [${installedRoles.join(', ')}]`);
    console.log(`Expected roles: [${expectedRoles.join(', ')}]`);
    for (const [role, cells] of Object.entries(cellInfo)) {
      const cellCount = Array.isArray(cells) ? cells.length : (cells ? 1 : 0);
      console.log(`  ${role}: ${cellCount} cell(s)`);
    }

    // Check 1: All expected roles must be present as keys
    const missingRoles = expectedRoles.filter(r => !installedRoles.includes(r));

    // Check 2: Each role must have at least one provisioned cell
    const emptyRoles = installedRoles.filter(r => {
      const cells = cellInfo[r];
      return !cells || (Array.isArray(cells) && cells.length === 0);
    });

    // Check 3: Total cell count should match expected DNA count
    let totalCells = 0;
    for (const cells of Object.values(cellInfo)) {
      totalCells += Array.isArray(cells) ? cells.length : (cells ? 1 : 0);
    }

    const isStale = missingRoles.length > 0 || emptyRoles.length > 0 || totalCells < expectedRoles.length;

    if (isStale) {
      if (missingRoles.length > 0) {
        console.log(`Stale hApp detected — missing roles: ${missingRoles.join(', ')}`);
      }
      if (emptyRoles.length > 0) {
        console.log(`Stale hApp detected — roles with no cells: ${emptyRoles.join(', ')}`);
      }
      if (totalCells < expectedRoles.length) {
        console.log(`Stale hApp detected — only ${totalCells} cells, expected ${expectedRoles.length}`);
      }
      console.log(`Uninstalling stale hApp '${APP_ID}'...`);
      await adminWs.uninstallApp({ installed_app_id: APP_ID });
      console.log(`Stale hApp removed. Will reinstall with full bundle.`);
      // Fall through to fresh install below
    } else {
      // Enable if disabled (status is an object: {type: "disabled", value: {...}})
      if (existingApp.status?.type === 'disabled') {
        console.log(`Enabling app '${APP_ID}'...`);
        await adminWs.enableApp({ installed_app_id: APP_ID });
        console.log(`App '${APP_ID}' enabled`);
      }

      // Still need to ensure app interface is attached (may not exist after restart)
      await ensureAppInterface(adminWs);
      return;
    }
  }

  // Generate agent key
  console.log('Generating agent public key...');
  const agentPubKey = await adminWs.generateAgentPubKey();
  console.log(`Agent key generated`);

  // Install the hApp
  console.log(`Installing hApp from ${HAPP_PATH}...`);
  const appInfo = await adminWs.installApp({
    source: { type: 'path', value: HAPP_PATH },
    installed_app_id: APP_ID,
    agent_key: agentPubKey,
  });
  console.log(`App '${APP_ID}' installed`);

  // Enable the app
  console.log(`Enabling app '${APP_ID}'...`);
  await adminWs.enableApp({ installed_app_id: APP_ID });
  console.log(`App '${APP_ID}' enabled`);

  // Attach app interface so clients can make zome calls
  await ensureAppInterface(adminWs);

  // List apps to confirm
  const finalApps = await adminWs.listApps({});
  console.log('Installed apps:', finalApps.map(a => `${a.installed_app_id} (${a.status})`).join(', '));
}

async function main() {
  console.log('=== hApp Installation Script ===');
  console.log(`Conductor URL: ${CONDUCTOR_URL}`);
  console.log(`hApp Path: ${HAPP_PATH}`);
  console.log(`App ID: ${APP_ID}`);
  console.log('');

  let adminWs;
  try {
    adminWs = await waitForConductor();
    await installHapp(adminWs);
    console.log('\n=== Installation complete ===');
  } catch (err) {
    console.error('Installation failed:', err.message);
    process.exit(1);
  } finally {
    if (adminWs) {
      await adminWs.client.close();
    }
  }
}

main();
