/** One-shot coordinator hot-swap on the WORKSPACE conductor (W2): applies the
 *  freshly built content_store coordinator wasm via admin update_coordinators —
 *  same primitive happ_manager::sync_coordinators uses; agent key, cells, DHT
 *  state all preserved. */
import { readFileSync } from 'fs';

import { AdminWebsocket } from '@holochain/client';
const APP_ID = 'elohim';
const WASM =
  '/projects/elohim/elohim/holochain/target/wasm32-unknown-unknown/release/content_store.wasm';
async function main() {
  const admin = await AdminWebsocket.connect({
    url: new URL('ws://127.0.0.1:41347'),
    wsClientOptions: { origin: APP_ID },
  });
  const apps = await admin.listApps({});
  const app = apps.find(a => a.installed_app_id === APP_ID) ?? apps[0];
  const cells = new Map<string, any>();
  for (const [role, infos] of Object.entries(app.cell_info))
    for (const info of infos as any[])
      if (info.type === 'provisioned' && info.value?.cell_id) cells.set(role, info.value.cell_id);
  const lamad = cells.get('lamad');
  const wasm = new Uint8Array(readFileSync(WASM));
  console.log('applying coordinator hot-swap, wasm bytes:', wasm.length);
  await admin.updateCoordinators({
    cell_id: lamad,
    source: {
      type: 'bundle',
      value: {
        manifest: {
          zomes: [
            {
              name: 'content_store',
              path: 'content_store.wasm',
              dependencies: [{ name: 'content_store_integrity' }],
            },
          ],
        },
        resources: { 'content_store.wasm': wasm },
      },
    },
  } as any);
  console.log('update_coordinators OK');
  process.exit(0);
}
main().catch(e => {
  console.error(String(e).slice(0, 400));
  process.exit(1);
});
