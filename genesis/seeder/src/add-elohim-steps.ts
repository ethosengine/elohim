/**
 * Quick script to add steps to the elohim-protocol path
 * Usage: HOLOCHAIN_ADMIN_URL=ws://localhost:XXXXX npx tsx src/add-elohim-steps.ts
 */
import { AdminWebsocket, AppWebsocket, encodeHashToBase64 } from '@holochain/client';
import * as fs from 'fs';
import * as path from 'path';

const ZOME_NAME = 'content_store';

interface StepJson {
  order: number;
  stepType?: string;
  resourceId: string;
  stepTitle?: string;
  stepNarrative?: string;
  optional?: boolean;
}

interface ChapterJson {
  id: string;
  title: string;
  steps?: StepJson[];
}

interface PathJson {
  id: string;
  chapters?: ChapterJson[];
}

interface AddPathStepInput {
  path_id: string;
  order_index: number;
  step_type: string;
  resource_id: string;
  step_title: string | null;
  step_narrative: string | null;
  is_optional: boolean;
}

async function main() {
  const adminUrl = process.env.HOLOCHAIN_ADMIN_URL;
  if (!adminUrl) {
    console.error('Error: HOLOCHAIN_ADMIN_URL environment variable required');
    console.error('Usage: HOLOCHAIN_ADMIN_URL=ws://localhost:XXXXX npx tsx src/add-elohim-steps.ts');
    process.exit(1);
  }

  console.log(`\n🔌 Connecting to Holochain admin at ${adminUrl}...`);
  const adminWs = await AdminWebsocket.connect({ url: new URL(adminUrl) });
  console.log('✅ Connected to admin');

  // Get app interface
  const interfaces = await adminWs.listAppInterfaces();
  if (interfaces.length === 0) {
    console.error('No app interfaces found');
    process.exit(1);
  }
  const appPort = interfaces[0].port;
  console.log(`📱 Using app interface on port ${appPort}`);

  // Connect to app
  const appWs = await AppWebsocket.connect({
    url: new URL(`ws://localhost:${appPort}`),
  });
  console.log('✅ Connected to app');

  // Get cell ID
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
  const cellId = (cellInfo as { provisioned: { cell_id: any } }).provisioned.cell_id;
  console.log(`📦 Using cell: ${encodeHashToBase64(cellId[0]).slice(0, 15)}...`);

  // Load elohim-protocol.json
  const pathJsonPath = path.resolve(__dirname, '../../../data/lamad/paths/elohim-protocol.json');
  console.log(`\n📖 Loading path from ${pathJsonPath}...`);

  const pathJson: PathJson = JSON.parse(fs.readFileSync(pathJsonPath, 'utf-8'));
  console.log(`   Path: ${pathJson.id}`);
  console.log(`   Chapters: ${pathJson.chapters?.length || 0}`);

  // Collect steps from chapters
  const steps: AddPathStepInput[] = [];
  if (pathJson.chapters && pathJson.chapters.length > 0) {
    let stepIndex = 0;
    for (const chapter of pathJson.chapters) {
      console.log(`   Chapter: ${chapter.title} (${chapter.steps?.length || 0} steps)`);
      for (const step of chapter.steps || []) {
        steps.push({
          path_id: pathJson.id,
          order_index: stepIndex++,
          step_type: step.stepType || 'read',
          resource_id: step.resourceId,
          step_title: step.stepTitle || null,
          step_narrative: step.stepNarrative || null,
          is_optional: step.optional || false,
        });
      }
    }
  }

  console.log(`\n📝 Adding ${steps.length} steps to path...`);

  if (steps.length === 0) {
    console.log('No steps to add');
    process.exit(0);
  }

  try {
    const result = await appWs.callZome({
      cell_id: cellId,
      zome_name: ZOME_NAME,
      fn_name: 'batch_add_path_steps',
      payload: { steps },
    }) as { created_count: number; errors: string[] };

    console.log(`\n✅ Added ${result.created_count} steps`);
    if (result.errors.length > 0) {
      console.log(`⚠️ Errors (${result.errors.length}):`);
      for (const err of result.errors.slice(0, 5)) {
        console.log(`   - ${err}`);
      }
    }
  } catch (error: any) {
    console.error(`\n❌ Failed to add steps: ${error.message || error}`);
    process.exit(1);
  }

  await adminWs.client.close();
  await appWs.client.close();
  console.log('\n✅ Done!');
}

main().catch(console.error);
