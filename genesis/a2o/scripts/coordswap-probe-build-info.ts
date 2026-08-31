// Rung-1 proof probe: call zome_build_info on each mesh conductor.
// Usage: tsx probe-build-info.ts "matthew=4444:4445,jessica=4454:4455,james=4464:4465"
import { AdminWebsocket, AppWebsocket } from '@holochain/client';
const APP_ID = 'elohim';
async function probe(name: string, adminPort: number, appPort: number) {
  const admin = await AdminWebsocket.connect({ url: new URL(`ws://127.0.0.1:${adminPort}`), wsClientOptions: { origin: APP_ID } });
  const apps = await admin.listApps({});
  const app = apps.find((a) => a.installed_app_id === APP_ID) ?? apps[0];
  const cells = new Map<string, any>();
  for (const [role, infos] of Object.entries(app.cell_info))
    for (const info of infos as any[])
      if (info.type === 'provisioned' && info.value?.cell_id) cells.set(role, info.value.cell_id);
  const lamad = cells.get('lamad');
  await admin.authorizeSigningCredentials(lamad);
  const appWs = await AppWebsocket.connect({ url: new URL(`ws://127.0.0.1:${appPort}`), token: (await admin.issueAppAuthenticationToken({ installed_app_id: app.installed_app_id })).token, wsClientOptions: { origin: APP_ID } });
  try {
    const info: any = await appWs.callZome({ cell_id: lamad, zome_name: 'content_store', fn_name: 'zome_build_info', payload: null });
    console.log(`${name}: LIVE ${JSON.stringify(info)}`);
  } catch (e) {
    console.log(`${name}: ABSENT (${String(e).slice(0, 90)})`);
  }
}
async function main() {
  for (const entry of process.argv[2].split(',')) {
    const [name, ports] = entry.split('=');
    const [a, p] = ports.split(':').map(Number);
    await probe(name, a, p).catch((e) => console.log(`${name}: ERROR ${String(e).slice(0, 90)}`));
  }
  process.exit(0);
}
main();
