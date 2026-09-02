// Sovereign-peer spike (2026-08-28): author a content node as the WORKSPACE agent on a conductor
// joined to alpha (`NETWORK_PROFILE=join-alpha hc-start.sh --conductor`), then probe doorway-alpha.
// Run:   cd genesis/seeder && pnpm exec tsx scripts/spikes/sovereign-peer-author.ts
// Result 2026-08-28T03:30Z: create_content 48 ms; local get_content_by_id FOUND; doorway-alpha
// /db/content/<id> 404 for 4 min — a storage-less peer's DHT entry is not served by the fleet
// (backlog: p1-dht-authored-content-not-projected). Glue candidate for
// features/deployment/sovereign-peer-join.feature scenario 3 (RED — the gap).
import { AdminWebsocket, AppWebsocket, encodeHashToBase64 } from '@holochain/client';
const ADMIN = 'ws://127.0.0.1:37253', APP = 'ws://127.0.0.1:4445', APP_ID = 'elohim';
const id = `spike-sovereign-peer-${Date.now()}`;
(async () => {
  const admin = await AdminWebsocket.connect({ url: new URL(ADMIN), wsClientOptions: { origin: 'elohim' } });
  const tok = await admin.issueAppAuthenticationToken({ installed_app_id: APP_ID });
  const app = await AppWebsocket.connect({ url: new URL(APP), token: tok.token, wsClientOptions: { origin: 'elohim' } });
  const info = await app.appInfo();
  const roles = Object.keys(info.cell_info);
  const role = roles.find(r => r.includes('lamad') || r === 'elohim') ?? roles[0];
  const cell = (info.cell_info[role] as any[]).find(c => c.type === 'provisioned' || c.provisioned);
  const cellId = (cell.value ?? cell.provisioned).cell_id;
  console.log('roles', roles, 'using', role, 'agent', encodeHashToBase64(cellId[1]));
  await admin.authorizeSigningCredentials(cellId);
  const t0 = Date.now();
  const created = await app.callZome({ cell_id: cellId, zome_name: 'content_store', fn_name: 'create_content', payload: {
    id, content_type: 'concept', title: 'Sovereign-peer spike', description: 'authored by a workspace conductor joined to alpha',
    content: '# hello from a workspace peer', content_format: 'markdown', reach: 'commons', tags: ['spike'], related_node_ids: [], metadata_json: '{}' } });
  console.log('create_content ok in', Date.now() - t0, 'ms; action', (created as any)?.action_hash ? encodeHashToBase64((created as any).action_hash) : JSON.stringify(created).slice(0, 120));
  const back = await app.callZome({ cell_id: cellId, zome_name: 'content_store', fn_name: 'get_content_by_id', payload: { id } });
  console.log('local get_content_by_id:', back ? 'FOUND' : 'null');
  for (let i = 0; i < 24; i++) {
    const r = await fetch(`https://doorway-alpha.elohim.host/db/content/${id}`);
    console.log(`[${Math.round((Date.now() - t0) / 1000)}s] doorway-alpha /db/content/${id} -> ${r.status}`);
    if (r.status === 200) break;
    await new Promise(res => setTimeout(res, 10000));
  }
  await (app.client as unknown as { close(): unknown }).close();
  await admin.client.close();
  process.exit(0);
})().catch(e => { console.error('SPIKE ERROR', e?.message ?? e); process.exit(1); });
