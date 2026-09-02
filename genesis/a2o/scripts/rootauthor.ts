import { AdminWebsocket, AppWebsocket, encodeHashToBase64 } from '@holochain/client';
const KEY = process.env.API_KEY_ADMIN;
const APP_ID = 'elohim';
async function main() {
  const wsOpts: any = { origin: APP_ID };
  if (KEY) wsOpts.headers = { 'x-api-key': KEY };
  const admin = await AdminWebsocket.connect({
    url: new URL('wss://doorway-alpha.elohim.host/hc/admin'),
    wsClientOptions: wsOpts,
  });
  const apps = await admin.listApps({});
  const app = apps.find(a => a.installed_app_id === APP_ID) ?? apps[0];
  const cells = new Map<string, any>();
  for (const [r, i] of Object.entries(app.cell_info))
    for (const info of i as any[])
      if (info.type === 'provisioned' && info.value?.cell_id) cells.set(r, info.value.cell_id);
  const lamad = cells.get('lamad');
  await admin.authorizeSigningCredentials(lamad);
  const tok = await admin.issueAppAuthenticationToken({ installed_app_id: app.installed_app_id });
  const appWs = await AppWebsocket.connect({
    url: new URL('wss://doorway-alpha.elohim.host/hc/app/4445'),
    token: tok.token,
    wsClientOptions: wsOpts,
  });
  console.log('doorway-alpha conductor agent:', encodeHashToBase64(lamad[1]));
  const h: any = await appWs.callZome({
    cell_id: lamad,
    zome_name: 'content_store',
    fn_name: 'resolve_content_head',
    payload: 'manifesto',
  });
  if (!h) {
    console.log('manifesto: None (not resolvable on this conductor)');
    process.exit(0);
  }
  console.log(
    'manifesto author (root):',
    encodeHashToBase64(h.author),
    'canonical:',
    h.canonical,
    'head:',
    encodeHashToBase64(h.head_action_hash)
  );
  process.exit(0);
}
main().catch(e => {
  console.error(String(e).slice(0, 300));
  process.exit(1);
});
