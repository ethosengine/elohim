#!/usr/bin/env npx tsx
/**
 * Diagnostic script to check cell discovery on the conductor
 *
 * Usage:
 *   HOLOCHAIN_ADMIN_URL='wss://doorway-dev.elohim.host?apiKey=...' npx tsx src/diagnose-cells.ts
 */

import { AdminWebsocket } from '@holochain/client';

const ADMIN_URL = process.env.HOLOCHAIN_ADMIN_URL || 'ws://localhost:4444';
const APP_ID = process.env.HOLOCHAIN_APP_ID || 'elohim';

async function main() {
  console.log('=== Cell Discovery Diagnostic ===');
  console.log(`Admin URL: ${ADMIN_URL}`);
  console.log(`App ID: ${APP_ID}`);
  console.log('');

  try {
    console.log('Connecting to admin interface...');
    const adminWs = await AdminWebsocket.connect({
      url: new URL(ADMIN_URL),
      wsClientOptions: { origin: 'http://localhost' },
    });
    console.log('Connected!\n');

    // List all apps
    const apps = await adminWs.listApps({});
    console.log(`Found ${apps.length} installed app(s):`);

    for (const app of apps) {
      console.log(`\n--- App: ${app.installed_app_id} ---`);
      console.log(`  Status: ${JSON.stringify(app.status)}`);
      console.log(`  Roles: ${Object.keys(app.cell_info).join(', ')}`);

      for (const [roleName, cells] of Object.entries(app.cell_info)) {
        console.log(`\n  Role "${roleName}":`);
        console.log(`    Cell count: ${(cells as any[]).length}`);

        for (let i = 0; i < (cells as any[]).length; i++) {
          const cell = (cells as any[])[i];
          const cellKeys = Object.keys(cell);
          console.log(`    Cell ${i}: keys = [${cellKeys.join(', ')}]`);

          // Check for provisioned cell
          if ('provisioned' in cell) {
            console.log(`      ✅ PROVISIONED cell found!`);
            const prov = cell.provisioned;
            console.log(`      cell_id type: ${typeof prov.cell_id}`);
            if (prov.cell_id) {
              if (Array.isArray(prov.cell_id)) {
                console.log(`      cell_id is array of length ${prov.cell_id.length}`);
                console.log(`      dna_hash type: ${typeof prov.cell_id[0]}, length: ${prov.cell_id[0]?.length || 'N/A'}`);
                console.log(`      agent_key type: ${typeof prov.cell_id[1]}, length: ${prov.cell_id[1]?.length || 'N/A'}`);
              } else {
                console.log(`      cell_id structure: ${JSON.stringify(Object.keys(prov.cell_id))}`);
              }
            }
          } else if ('type' in cell && cell.type === 'provisioned') {
            console.log(`      ✅ PROVISIONED cell (type format)!`);
            console.log(`      value: ${JSON.stringify(Object.keys(cell.value || {}))}`);
          } else if ('stem' in cell) {
            console.log(`      ⚠️ STEM cell (not provisioned yet)`);
          } else if ('cloned' in cell) {
            console.log(`      📋 CLONED cell`);
          } else {
            console.log(`      ❓ Unknown cell format`);
            console.log(`      Raw: ${JSON.stringify(cell, null, 2).substring(0, 200)}...`);
          }
        }
      }
    }

    // Check specifically for our app
    const ourApp = apps.find(a => a.installed_app_id === APP_ID);
    if (!ourApp) {
      console.log(`\n❌ App '${APP_ID}' NOT FOUND on conductor!`);
      console.log('This means the hApp installation failed or was never attempted.');
    } else {
      console.log(`\n✅ App '${APP_ID}' found`);

      // Count provisioned cells
      let provisionedCount = 0;
      for (const cells of Object.values(ourApp.cell_info)) {
        for (const cell of (cells as any[])) {
          if ('provisioned' in cell || (cell.type === 'provisioned')) {
            provisionedCount++;
          }
        }
      }

      if (provisionedCount === 0) {
        console.log(`❌ No provisioned cells found!`);
        console.log('The app is installed but cells failed to provision.');
        console.log('Check hApp manifest and DNA files.');
      } else {
        console.log(`✅ Found ${provisionedCount} provisioned cell(s)`);
      }
    }

    await adminWs.client.close();
    console.log('\nDone.');
  } catch (error) {
    console.error('Error:', error);
    process.exit(1);
  }
}

main();
