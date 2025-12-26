#!/usr/bin/env npx tsx
/**
 * Quick diagnostic: Query a path from Holochain and show metadata_json
 */

import { AdminWebsocket, AppWebsocket } from '@holochain/client';
import * as fs from 'fs';
import * as path from 'path';

const LOCAL_DEV_DIR = process.env.LOCAL_DEV_DIR || '/projects/elohim/holochain/local-dev';
const HC_PORTS_FILE = path.join(LOCAL_DEV_DIR, '.hc_ports');

async function main() {
  const pathId = process.argv[2] || 'elohim-protocol';
  console.log(`🔍 Checking path: ${pathId}`);

  // Read ports (shell format: key=value)
  const portsContent = fs.readFileSync(HC_PORTS_FILE, 'utf-8');
  const adminPort = portsContent.match(/admin_port=(\d+)/)?.[1];
  const appPort = portsContent.match(/app_port=(\d+)/)?.[1];
  if (!adminPort || !appPort) {
    console.error('❌ Could not parse ports file');
    process.exit(1);
  }
  const adminWsUrl = `ws://localhost:${adminPort}`;

  console.log(`🔌 Connecting to ${adminWsUrl}...`);

  const adminWs = await AdminWebsocket.connect({
    url: new URL(adminWsUrl),
    wsClientOptions: { origin: 'http://localhost' },
  });
  const apps = await adminWs.listApps({});
  const app = apps.find(a => a.installed_app_id === 'elohim');

  if (!app) {
    console.error('❌ App not found');
    process.exit(1);
  }

  const cellInfo = (app.cell_info as any).elohim?.[0];
  if (!cellInfo) {
    console.error('❌ No elohim cell found');
    process.exit(1);
  }
  const cellId = cellInfo.value?.cell_id;
  if (!cellId) {
    console.error('❌ No cell_id found');
    process.exit(1);
  }

  // Get app auth token
  const token = await adminWs.issueAppAuthenticationToken({ installed_app_id: 'elohim' });

  // Authorize signing credentials
  await adminWs.authorizeSigningCredentials(cellId);

  const appWs = await AppWebsocket.connect({
    url: new URL(`ws://localhost:${appPort}`),
    wsClientOptions: { origin: 'http://localhost' },
    token: token.token,
  });

  console.log(`✅ Connected`);

  // Call get_path_with_steps
  const result = await appWs.callZome({
    cell_id: cellId,
    zome_name: 'content_store',
    fn_name: 'get_path_with_steps',
    payload: pathId,
  });

  if (!result) {
    console.log('❌ Path not found');
    process.exit(1);
  }

  console.log('\n📋 Path data from Holochain:');
  console.log('  id:', result.path.id);
  console.log('  title:', result.path.title);
  console.log('  steps:', result.steps?.length || 0);
  console.log('  metadata_json RAW:', JSON.stringify(result.path.metadata_json));
  console.log('  metadata_json value:', result.path.metadata_json);
  console.log('  All path keys:', Object.keys(result.path));

  // Parse metadata
  if (result.path.metadata_json) {
    try {
      const meta = JSON.parse(result.path.metadata_json);
      console.log('  chapters in metadata:', meta.chapters?.length || 0);
    } catch (e) {
      console.log('  ⚠️ Failed to parse metadata_json');
    }
  }

  await appWs.client.close();
  await adminWs.client.close();
  console.log('\n✅ Done');
}

main().catch(console.error);
