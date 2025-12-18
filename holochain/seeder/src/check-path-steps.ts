/**
 * Quick script to check what steps are stored for a path
 * Usage: HOLOCHAIN_ADMIN_URL=ws://localhost:XXXXX npx tsx src/check-path-steps.ts [path-id]
 */
import { AdminWebsocket, AppWebsocket, encodeHashToBase64 } from '@holochain/client';

const ZOME_NAME = 'content_store';

async function main() {
  const pathId = process.argv[2] || 'elohim-protocol';

  const adminUrl = process.env.HOLOCHAIN_ADMIN_URL;
  if (!adminUrl) {
    console.error('Error: HOLOCHAIN_ADMIN_URL environment variable required');
    process.exit(1);
  }

  console.log(`\n🔍 Checking steps for path: ${pathId}`);
  console.log(`🔌 Connecting to ${adminUrl}...`);

  const adminWs = await AdminWebsocket.connect({ url: new URL(adminUrl) });
  const interfaces = await adminWs.listAppInterfaces();
  const appPort = interfaces[0].port;

  const appWs = await AppWebsocket.connect({
    url: new URL(`ws://localhost:${appPort}`),
  });

  const apps = await adminWs.listApps({});
  const elohimApp = apps.find(a => a.installed_app_id.includes('elohim'));
  if (!elohimApp) {
    console.error('Elohim app not found');
    process.exit(1);
  }

  const cellInfo = Object.values(elohimApp.cell_info)[0]?.[0];
  if (!cellInfo || !('provisioned' in cellInfo)) {
    console.error('Could not get cell info');
    process.exit(1);
  }
  const cellId = cellInfo.provisioned.cell_id;

  // Get path with steps
  const result = await appWs.callZome({
    cell_id: cellId,
    zome_name: ZOME_NAME,
    fn_name: 'get_path_with_steps',
    payload: pathId,
  }) as any;

  if (!result) {
    console.log('❌ Path not found');
    process.exit(1);
  }

  console.log(`\n📖 Path: ${result.path.title}`);
  console.log(`   ID: ${result.path.id}`);
  console.log(`   Steps: ${result.steps.length}`);
  console.log('\n📝 Steps (in order):');

  for (const stepData of result.steps) {
    const step = stepData.step;
    console.log(`   [${step.order_index}] ${step.resource_id}`);
    console.log(`       Title: ${step.step_title || '(none)'}`);
    console.log(`       Type: ${step.step_type}`);
  }

  await adminWs.client.close();
  await appWs.client.close();
}

main().catch(console.error);
