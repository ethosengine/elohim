import { AdminWebsocket, AppWebsocket, encodeHashToBase64 } from '@holochain/client';
const APP_ID = 'elohim';
async function main() {
  const admin = await AdminWebsocket.connect({ url: new URL('ws://127.0.0.1:41347'), wsClientOptions: { origin: APP_ID } });
  const apps = await admin.listApps({});
  const app = apps.find((a) => a.installed_app_id === APP_ID) ?? apps[0];
  const cells = new Map<string, any>();
  for (const [role, infos] of Object.entries(app.cell_info))
    for (const info of infos as any[])
      if (info.type === 'provisioned' && info.value?.cell_id) cells.set(role, info.value.cell_id);
  const lamad = cells.get('lamad');
  await admin.authorizeSigningCredentials(lamad);
  const appWs = await AppWebsocket.connect({ url: new URL('ws://127.0.0.1:4485'), token: (await admin.issueAppAuthenticationToken({ installed_app_id: app.installed_app_id })).token, wsClientOptions: { origin: APP_ID } });
  const ev: any = await appWs.callZome({ cell_id: lamad, zome_name: 'content_store', fn_name: 'get_canonical_election_evidence', payload: 'manifesto' });
  if (!ev) { console.log('NO election evidence (extern answered null)'); process.exit(2); }
  console.log('EVIDENCE:', JSON.stringify({ winner: String(ev.election.winner_target), earned: ev.election.canonical_earned, declared_at: ev.election.canonical_declared_at, linkBytes: ev.link_record?.length }));
  process.exit(0);
}
main().catch((e) => { console.error(String(e).slice(0, 300)); process.exit(1); });
