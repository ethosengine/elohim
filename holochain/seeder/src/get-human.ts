/**
 * Get existing human info from Holochain.
 */

import { AdminWebsocket, AppWebsocket } from '@holochain/client';

async function getHumanInfo() {
  const adminWs = await AdminWebsocket.connect({
    url: new URL('ws://localhost:38961'),
    wsClientOptions: { origin: 'http://localhost' },
  });

  const apps = await adminWs.listApps({});
  const app = apps.find(a => a.installed_app_id === 'lamad-spike');
  if (app === undefined) throw new Error('App not found');

  const cellInfo = Object.values(app.cell_info)[0];
  if (cellInfo === undefined || cellInfo[0]?.type !== 'provisioned') throw new Error('Cell not found');

  const cellId = cellInfo[0].value.cell_id as [Uint8Array, Uint8Array];
  const agentPubKey = Buffer.from(cellId[1]).toString('base64');

  // Get app interfaces
  const interfaces = await adminWs.listAppInterfaces();
  const appPort = interfaces[0]?.port ?? 4445;

  // Authorize and connect
  await adminWs.authorizeSigningCredentials(cellId);
  const token = await adminWs.issueAppAuthenticationToken({
    installed_app_id: 'lamad-spike',
    single_use: false,
    expiry_seconds: 60,
  });

  const appWs = await AppWebsocket.connect({
    url: new URL(`ws://localhost:${appPort}`),
    wsClientOptions: { origin: 'http://localhost' },
    token: token.token,
  });

  // Get current human
  const human = await appWs.callZome({
    cell_id: cellId,
    zome_name: 'content_store',
    fn_name: 'get_current_human',
    payload: null,
  });

  console.log('Current human:', JSON.stringify(human, null, 2));
  console.log('Agent PubKey (base64):', agentPubKey);

  await appWs.client.close();
  await adminWs.client.close();
}

getHumanInfo().catch(console.error);
