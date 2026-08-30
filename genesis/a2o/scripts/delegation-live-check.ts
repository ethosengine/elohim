/**
 * Secret-free live proof that the DEPLOYED head-delegation primitive works on the
 * fleet's hot-swapped coordinators — same code path as matthew→W, with W as the
 * grantor (a key this workspace holds) instead of matthew (a key it does not).
 *
 * 1. W (workspace agent, root author of the gate-reading node) mints a fresh
 *    device key D via the workspace conductor's admin interface.
 * 2. W signs a HeadDelegation for D (content_store.grant_head_delegation).
 * 3. Through the DEPLOYED storage route, D declares the gate-reading node's head
 *    carrying that delegation — the workspace conductor re-verifies the signature
 *    in-wasm and accepts a NON-author's declaration.
 *
 * If step 3 returns 200 with trust:notarized, the primitive is proven live; the
 * only thing the matthew→W ceremony adds is a different grantor key.
 */
import { AdminWebsocket, AppWebsocket, encodeHashToBase64, decodeHashFromBase64 } from '@holochain/client';

const ADMIN = 'ws://127.0.0.1:42665';
const APP_ID = 'elohim';
const ID = 'gate-reading-manifesto-20260830T131032Z';
const STORAGE = 'http://localhost:8090';

async function main() {
  const admin = await AdminWebsocket.connect({ url: new URL(ADMIN), wsClientOptions: { origin: 'elohim' } });
  const apps = await admin.listApps({});
  const app = apps.find(a => a.installed_app_id === APP_ID) ?? apps[0];
  const cells = new Map<string, any>();
  for (const [role, infos] of Object.entries(app.cell_info)) {
    for (const info of infos as any[]) {
      if (info.type === 'provisioned' && info.value?.cell_id) cells.set(role, info.value.cell_id);
    }
  }
  const lamad = cells.get('lamad');
  const W = encodeHashToBase64(lamad[1]);
  console.log('grantor W (root author):', W);

  // 1. fresh device key D
  const D = await admin.generateAgentPubKey();
  const Db64 = encodeHashToBase64(D);
  console.log('fresh device key D:', Db64);

  await admin.authorizeSigningCredentials(lamad);
  // 2. grant via the app interface
  const token = await admin.issueAppAuthenticationToken({ installed_app_id: app.installed_app_id });
  const appWs = await AppWebsocket.connect({ url: new URL(ADMIN.replace('42665','4485')), token: token.token, wsClientOptions: { origin: 'elohim' } });
  const validUntil = (Date.now() + 24 * 3600 * 1000) * 1000;
  const delegation: any = await appWs.callZome({
    cell_id: lamad, zome_name: 'content_store', fn_name: 'grant_head_delegation',
    payload: { delegate: D, scope: '*', valid_until: validUntil },
  });
  const json = {
    grantor: encodeHashToBase64(delegation.payload.grantor),
    delegate: encodeHashToBase64(delegation.payload.delegate),
    scope: delegation.payload.scope,
    validUntil: Number(delegation.payload.valid_until),
    signature: Buffer.from(delegation.signature).toString('base64'),
  };
  console.log('delegation minted: grantor', json.grantor.slice(0, 12), 'delegate', json.delegate.slice(0, 12));

  // 3. read current head target, then declare it as D under the delegation
  const cur = await (await fetch(`${STORAGE}/db/content/${ID}/head`)).json().catch(() => ({}));
  const target = (cur as any).headActionHash ?? (cur as any).dhtAnchorHash;
  console.log('current head target:', String(target).slice(0, 20));
  const head = await fetch(`${STORAGE}/db/content/${ID}/head`, {
    method: 'POST',
    headers: { 'content-type': 'application/json', 'X-Agent-Cid': Db64 },
    body: JSON.stringify({ headActionHash: target, delegation: json }),
  });
  const body = await head.text();
  console.log(`DELEGATED DECLARE as D: ${head.status} ${body.slice(0, 300)}`);
  process.exit(head.ok ? 0 : 1);
}
main().catch(e => { console.error(String(e).slice(0, 400)); process.exit(1); });
