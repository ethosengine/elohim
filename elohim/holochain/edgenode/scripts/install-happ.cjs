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

// DNA-drift auto-reinstall gate. When the bundle's DNA content changes but the
// role STRUCTURE stays the same (e.g. an integrity-zome fix → new DNA hash, same
// roles), the structural stale-check below reads "not stale" and the conductor
// keeps the OLD DNA from its PVC across pod restarts — so the new DNA never
// deploys (this is exactly how the Gap-F deny_unknown_fields fix sat built-but-
// undeployed: edgenode image updated, hApp never reinstalled). The DNA-hash
// drift check (see installHapp) closes that gap.
//
// Auto-reinstall is GATED because uninstall+reinstall mints a new agent key /
// cell — fine for ephemeral re-seeded envs (alpha/dev), but NOT the production
// upgrade path (which needs DNA migration/lineage per the lineage_rna upgrade
// design, not a blind wipe). On production leave ALLOW_DNA_REINSTALL unset: drift
// is logged loudly for the operator instead of auto-applied.
const ALLOW_DNA_REINSTALL =
  (process.env.ALLOW_DNA_REINSTALL || 'false').toLowerCase() === 'true';

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

/**
 * Encode a DnaHash (Uint8Array / number[] / Buffer) to a base64 string for
 * comparison. Defensive across @holochain/client byte representations.
 */
function hashToB64(bytes) {
  if (!bytes) return null;
  try {
    return Buffer.from(bytes).toString('base64');
  } catch (_) {
    return null;
  }
}

/**
 * Extract the DnaHash (base64) from a single CellInfo, handling the tagged-enum
 * shapes @holochain/client has used: `{ type: 'provisioned', value: { cell_id }}`
 * and the older `{ provisioned: { cell_id }}`. cell_id is `[DnaHash, AgentPubKey]`.
 * Only provisioned cells carry the role's canonical DNA hash.
 */
function dnaHashFromCell(cell) {
  if (!cell) return null;
  const prov =
    (cell.type === 'provisioned' && cell.value) ||
    cell.provisioned ||
    (cell.value && cell.value.cell_id ? cell.value : null);
  const cellId = prov && prov.cell_id;
  if (!cellId || !cellId[0]) return null;
  return hashToB64(cellId[0]);
}

/**
 * Map role → DnaHash(b64) for the provisioned cell of each role in a cell_info.
 */
function dnaHashesByRole(cellInfo) {
  const out = {};
  for (const [role, cells] of Object.entries(cellInfo || {})) {
    const list = Array.isArray(cells) ? cells : [cells];
    for (const cell of list) {
      const h = dnaHashFromCell(cell);
      if (h) {
        out[role] = h;
        break; // first provisioned cell is the role's DNA
      }
    }
  }
  return out;
}

/**
 * Compute the bundle's expected per-role DNA hashes by installing it under a
 * throwaway app_id WITHOUT enabling it (no DHT join / network side effects),
 * reading the cell DNA hashes, then uninstalling. A DNA hash is derived from DNA
 * content + modifiers (network_seed/properties) and is independent of the agent
 * key, so the probe's hashes equal what a fresh main-app install of this bundle
 * would produce — making it a faithful "what's in the bundle" reference.
 */
async function bundleDnaHashesByRole(adminWs) {
  const probeAppId = `${APP_ID}-version-probe`;
  // Clean any leftover probe from a prior interrupted run.
  try {
    const apps = await adminWs.listApps({});
    if (apps.find(a => a.installed_app_id === probeAppId)) {
      await adminWs.uninstallApp({ installed_app_id: probeAppId });
    }
  } catch (_) {
    /* best effort */
  }

  const probeKey = await adminWs.generateAgentPubKey();
  const probeInfo = await adminWs.installApp({
    source: { type: 'path', value: HAPP_PATH },
    installed_app_id: probeAppId,
    agent_key: probeKey,
  });
  // Intentionally NOT enabled — install registers the cells (giving us DNA
  // hashes) without joining any network.
  const hashes = dnaHashesByRole(probeInfo.cell_info || {});
  try {
    await adminWs.uninstallApp({ installed_app_id: probeAppId });
  } catch (e) {
    console.log(`Warning: failed to clean up version-probe app: ${e.message}`);
  }
  return hashes;
}

/**
 * Compare installed per-role DNA hashes against the bundle's. Returns the list
 * of roles whose DNA content has drifted (installed ≠ bundle). Defensive: any
 * failure returns [] (treat as no-drift) so a probe error never breaks a deploy
 * — worst case is the prior behavior (keep the installed hApp).
 */
async function detectDnaDrift(adminWs, installedCellInfo) {
  try {
    const installed = dnaHashesByRole(installedCellInfo);
    const bundle = await bundleDnaHashesByRole(adminWs);
    const drifted = [];
    for (const [role, bundleHash] of Object.entries(bundle)) {
      const installedHash = installed[role];
      if (installedHash && bundleHash && installedHash !== bundleHash) {
        drifted.push(role);
        console.log(
          `  DNA drift — role '${role}': installed ${installedHash} ≠ bundle ${bundleHash}`
        );
      }
    }
    return drifted;
  } catch (e) {
    console.log(
      `DNA-drift check skipped (non-fatal): ${e.message} — keeping installed hApp`
    );
    return [];
  }
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
      // Structurally complete (all roles present with cells). Now check for DNA
      // CONTENT drift: same roles, but a changed DNA (e.g. an integrity-zome fix
      // → new DNA hash). The structural check above can't see this, so without
      // this the conductor keeps the stale DNA across pod restarts and the new
      // bundle never deploys.
      console.log('Checking for DNA content drift (installed vs bundle hashes)...');
      const drifted = await detectDnaDrift(adminWs, cellInfo);

      if (drifted.length > 0) {
        if (ALLOW_DNA_REINSTALL) {
          console.log(
            `DNA drift in role(s) [${drifted.join(', ')}] — ALLOW_DNA_REINSTALL=true; ` +
              `uninstalling to reinstall the bundle's DNA.`
          );
          await adminWs.uninstallApp({ installed_app_id: APP_ID });
          console.log(`Drifted hApp removed. Will reinstall with current bundle.`);
          // Fall through to fresh install below.
        } else {
          console.log(
            `⚠️  DNA DRIFT DETECTED in role(s) [${drifted.join(', ')}] but ` +
              `ALLOW_DNA_REINSTALL is not set — NOT auto-reinstalling.\n` +
              `   The conductor is running OLDER DNA than the deployed bundle.\n` +
              `   Ephemeral envs (alpha/dev): set ALLOW_DNA_REINSTALL=true to auto-apply.\n` +
              `   Production: this needs a DNA migration (lineage), not a blind reinstall.`
          );
          // Keep the existing install (preserve identity); operator decides.
          if (existingApp.status?.type === 'disabled') {
            console.log(`Enabling app '${APP_ID}'...`);
            await adminWs.enableApp({ installed_app_id: APP_ID });
          }
          await ensureAppInterface(adminWs);
          return;
        }
      } else {
        console.log('No DNA drift — installed hApp matches the bundle.');
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
