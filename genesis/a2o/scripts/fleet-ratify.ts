/**
 * Steward-ratified fleet adoption of the device-authored manifesto head.
 *
 * The device (W2) authored the manifesto content + declared it locally. A fresh
 * joiner's cross-root canonical link does not gossip/adopt on the fleet, so:
 *   1. pull W2's manifesto Content record from the WORKSPACE conductor
 *      (get_record_for_action), and
 *   2. have MATTHEW'S conductor (the manifesto's fleet root author, reached via
 *      the doorway admin proxy) declare the EARNED canonical head for that action
 *      CARRYING W2's record. matthew is a real fleet authority: his canonical link
 *      gossips fleet-wide and the carried record makes W2's version retrievable.
 *
 * The device proposed the content; the steward's authority lands it on the fleet.
 * Run: cd genesis/a2o && API_KEY_ADMIN=<key> pnpm exec tsx scripts/fleet-ratify.ts
 */
import { AdminWebsocket, AppWebsocket, encodeHashToBase64 } from '@holochain/client';

const WS_ADMIN = `ws://127.0.0.1:${process.env.WS_ADMIN_PORT ?? '41347'}`;
const WS_APP = Number(process.env.WS_APP_PORT ?? 4485);
const FLEET_ADMIN = process.env.FLEET_ADMIN_URL ?? 'wss://doorway-alpha.elohim.host/hc/admin';
const FLEET_APP = Number(process.env.FLEET_APP_PORT ?? 4445);
const KEY = process.env.API_KEY_ADMIN;
const APP_ID = 'elohim';
const ID = 'manifesto';
const W2_ACTION = process.env.W2_ACTION ?? 'uhCkk1kms69G6zRVK-5HXw-PnPJ9EZ_PpP49eLHY5oS5xZrssikDu';

async function connect(adminUrl: string, appPort: number, key?: string) {
  const wsOpts: any = { origin: APP_ID };
  if (key) wsOpts.headers = { 'x-api-key': key };
  const admin = await AdminWebsocket.connect({ url: new URL(adminUrl), wsClientOptions: wsOpts });
  const apps = await admin.listApps({});
  const app = apps.find(a => a.installed_app_id === APP_ID) ?? apps[0];
  const cells = new Map<string, any>();
  for (const [role, infos] of Object.entries(app.cell_info))
    for (const info of infos as any[])
      if (info.type === 'provisioned' && info.value?.cell_id) cells.set(role, info.value.cell_id);
  const lamad = cells.get('lamad');
  await admin.authorizeSigningCredentials(lamad);
  const au = new URL(adminUrl);
  const loop = au.hostname === '127.0.0.1' || au.hostname === 'localhost';
  const appUrl = loop
    ? new URL(`ws://${au.hostname}:${appPort}`)
    : new URL(`${au.protocol}//${au.host}/hc/app/${appPort}`);
  const appWs = await AppWebsocket.connect({
    url: appUrl,
    token: (await admin.issueAppAuthenticationToken({ installed_app_id: app.installed_app_id }))
      .token,
    wsClientOptions: wsOpts,
  });
  return { admin, appWs, lamad, agent: encodeHashToBase64(lamad[1]) };
}

async function main() {
  // 1. Pull W2's manifesto record from the workspace conductor.
  const ws = await connect(WS_ADMIN, WS_APP);
  console.log('workspace W2:', ws.agent);
  const carried: any = await ws.appWs.callZome({
    cell_id: ws.lamad,
    zome_name: 'content_store',
    fn_name: 'get_record_for_action',
    payload: { action_hash: W2_ACTION },
  });
  if (!carried?.record)
    throw new Error(
      `workspace could not serve record for ${W2_ACTION}: ${JSON.stringify(carried).slice(0, 150)}`
    );
  const recordBytes: Uint8Array = carried.record;
  console.log('carried record bytes:', recordBytes.length);

  // 2. matthew's conductor declares the earned canonical head carrying W2's record.
  const fleet = await connect(FLEET_ADMIN, FLEET_APP, KEY);
  console.log('fleet grantor (manifesto root author):', fleet.agent);
  const out: any = await fleet.appWs.callZome({
    cell_id: fleet.lamad,
    zome_name: 'content_store',
    fn_name: 'declare_earned_canonical_head',
    payload: {
      id: ID,
      head_action_hash: W2_ACTION,
      carried_record: recordBytes,
      adopt_before_author: true,
      delegation: null,
    },
  });
  console.log(
    'FLEET EARNED canonical declared:',
    JSON.stringify({
      head: encodeHashToBase64(out.head_action_hash),
      author: encodeHashToBase64(out.author),
      canonical: out.canonical,
    })
  );
  process.exit(0);
}
main().catch(e => {
  console.error(String(e).slice(0, 500));
  process.exit(1);
});
