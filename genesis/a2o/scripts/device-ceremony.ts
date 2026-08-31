/**
 * Station-3 ceremony — run with:  cd /projects/elohim/genesis/a2o && pnpm exec tsx <this file> <phase>
 *
 * Phases:
 *   grant    — on MATTHEW'S FLEET CONDUCTOR (via the doorway-alpha admin proxy):
 *              1. content_store.grant_head_delegation(delegate=W, scope, valid_until)
 *              2. mishpat.create_commitment(binds-identity) — the witnessed governance
 *                 record of the same act (chain_root/head_key = matthew, controllers += W)
 *              Prints the delegation as JSON (base64 keys + signature) for the declare phase.
 *   declare  — on the WORKSPACE PEER (localhost:8090): PATCH the manifesto to the repo's
 *              latest bytes with reach=commons, then POST …/head with the delegation as W.
 *
 * The grant phase is the "primary device act": it runs against matthew's own conductor,
 * whose lair signs the delegation. The declare phase is the second device acting under it.
 */
import { AdminWebsocket, AppWebsocket, encodeHashToBase64, decodeHashFromBase64 } from '@holochain/client';
import { readFileSync, writeFileSync } from 'node:fs';

const W = process.env.DEVICE_AGENT ?? 'uhCAkDRf5_8rAphi2xekEmRCrfw4dIUkKG0B01WiNSdp31LFIwZcX'; // override with DEVICE_AGENT after a conductor re-key
const SCRATCH = '/tmp/claude-0/-projects-elohim/bd085a03-fcae-4c1c-b245-fda9ca07a257/scratchpad';
const APP_ID = 'elohim';

function b64FromBytes(u8: Uint8Array): string {
  return Buffer.from(u8).toString('base64');
}

async function connect(adminUrl: string, origin: string) {
  const admin = await AdminWebsocket.connect({
    url: new URL(adminUrl),
    wsClientOptions: { origin },
  });
  const apps = await admin.listApps({});
  const app = apps.find(a => a.installed_app_id === APP_ID) ?? apps[0];
  if (!app) throw new Error(`no installed app on ${adminUrl} (saw: ${apps.map(a => a.installed_app_id).join(',')})`);
  const cells = new Map<string, any>();
  for (const [role, infos] of Object.entries(app.cell_info)) {
    for (const info of infos as any[]) {
      if (info.type === 'provisioned' && info.value?.cell_id) cells.set(role, info.value.cell_id);
    }
  }
  const token = await admin.issueAppAuthenticationToken({ installed_app_id: app.installed_app_id });
  const port = (await admin.listAppInterfaces()).find(Boolean);
  if (port === undefined) throw new Error('no app interface attached');
  const appWs = await AppWebsocket.connect({
    url: new URL(adminUrl.replace(/^wss:\/\/([^?]+).*/, 'wss://$1').replace(/^ws:\/\/[^:]+:\d+.*/, m => m)),
    token: token.token,
    wsClientOptions: { origin },
  } as any);
  return { admin, appWs, cells, appId: app.installed_app_id };
}

async function grant() {
  // Grantor = matthew's conductor. Reach it directly (a `kubectl port-forward`
  // of matthew's pod's admin+app ports to localhost is the reliable path —
  // FLEET_ADMIN_URL=ws://127.0.0.1:4444, FLEET_APP_PORT=4445, no api-key needed
  // on a loopback admin socket). The doorway admin proxy is permission-gated and
  // does not route app-interface zome calls, so the port-forward is preferred.
  const adminUrl = process.env.FLEET_ADMIN_URL ?? 'wss://doorway-alpha.elohim.host/hc/admin';
  const appPort = Number(process.env.FLEET_APP_PORT ?? 4445);
  const adminKey = process.env.API_KEY_ADMIN; // doorway wants it as an x-api-key HEADER
  const wsOpts: any = { origin: APP_ID };
  if (adminKey) wsOpts.headers = { 'x-api-key': adminKey };
  const admin = await AdminWebsocket.connect({ url: new URL(adminUrl), wsClientOptions: wsOpts });
  const apps = await admin.listApps({});
  const app = apps.find(a => a.installed_app_id === APP_ID) ?? apps[0];
  if (!app) throw new Error(`no app on fleet conductor (saw ${apps.map(a => a.installed_app_id).join(',')})`);
  const cells = new Map<string, any>();
  for (const [role, infos] of Object.entries(app.cell_info)) {
    for (const info of infos as any[]) {
      if (info.type === 'provisioned' && info.value?.cell_id) cells.set(role, info.value.cell_id);
    }
  }
  const lamad = cells.get('lamad');
  const mishpat = cells.get('mishpat');
  if (!lamad) throw new Error(`no lamad cell (roles: ${[...cells.keys()].join(',')})`);
  const matthewKey = encodeHashToBase64(lamad[1]);
  console.log(`fleet agent (grantor / matthew's conductor key): ${matthewKey}`);

  // Signing credentials for the lamad cell — the missing step my earlier draft
  // skipped (the app zome call is refused without it).
  await admin.authorizeSigningCredentials(lamad);
  // Ensure an app interface exists on the expected port, then connect to it.
  const ifaces = await admin.listAppInterfaces();
  if (!ifaces.some((i: any) => (i.port ?? i) === appPort)) {
    await admin.attachAppInterface({ port: appPort, allowed_origins: APP_ID });
  }
  const token = await admin.issueAppAuthenticationToken({ installed_app_id: app.installed_app_id });
  const au = new URL(adminUrl);
  const loopback = au.hostname === '127.0.0.1' || au.hostname === 'localhost';
  // Direct conductor → its own app port; through the doorway → the /hc/app/{port} proxy.
  const appUrl = loopback
    ? new URL(`ws://${au.hostname}:${appPort}`)
    : new URL(`${au.protocol}//${au.host}/hc/app/${appPort}`);
  const appWs = await AppWebsocket.connect({ url: appUrl, token: token.token, wsClientOptions: wsOpts });

  const validUntil = (Date.now() + 30 * 24 * 3600 * 1000) * 1000; // 30 days, µs
  const delegation: any = await appWs.callZome({
    cell_id: lamad,
    zome_name: 'content_store',
    fn_name: 'grant_head_delegation',
    payload: { delegate: decodeHashFromBase64(W), scope: '*', valid_until: validUntil },
  });
  const json = {
    grantor: encodeHashToBase64(delegation.payload.grantor),
    delegate: encodeHashToBase64(delegation.payload.delegate),
    scope: delegation.payload.scope,
    validUntil: Number(delegation.payload.valid_until),
    signature: b64FromBytes(delegation.signature),
  };
  writeFileSync(`${SCRATCH}/delegation.json`, JSON.stringify(json, null, 2));
  console.log('delegation minted →', `${SCRATCH}/delegation.json`);
  console.log(JSON.stringify(json));

  if (mishpat) {
    // Witnessed governance record: binds-identity naming W a controller.
    const payload = {
      action: 'binds-identity',
      chain_root: matthewKey,
      head_key: matthewKey,
      controllers: [matthewKey, W],
      controller_policy: 'self+stewarded-device',
      device_delegation: json,
      signed_at: new Date().toISOString(),
    };
    try {
      const out: any = await appWs.callZome({
        cell_id: mishpat,
        zome_name: 'mishpat',
        fn_name: 'create_commitment',
        payload: { action: 'binds-identity', payload_json: JSON.stringify(payload), signed_at: payload.signed_at },
      });
      console.log('binds-identity commitment:', JSON.stringify(out).slice(0, 200));
    } catch (e) {
      console.log('binds-identity commitment failed (non-fatal for the ceremony):', String(e).slice(0, 300));
    }
  }
  process.exit(0);
}

async function declare() {
  const delegation = JSON.parse(readFileSync(`${SCRATCH}/delegation.json`, 'utf8'));
  const md = readFileSync('/projects/elohim/genesis/docs/content/elohim-protocol/manifesto.md', 'utf8');
  // The seeder strips frontmatter; manifesto.md has none beyond the cites block? Use the
  // CID twin's canonical content so bytes match the fleet's declared blobHash.
  const twin = JSON.parse(readFileSync('/projects/elohim/genesis/data/lamad/content/manifesto.json', 'utf8'));
  const node = Array.isArray(twin) ? twin[0] : twin;
  const base = 'http://localhost:8090';

  // 1. PATCH the workspace's manifesto row to the latest bytes + commons reach
  //    (the workspace has no `manifesto` row yet → create it first via bulk).
  let r = await fetch(`${base}/db/content/manifesto`, { method: 'GET' });
  if (r.status === 404) {
    const bulk = await fetch(`${base}/db/content/bulk`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify([{ ...node, id: 'manifesto' }]),
    });
    console.log('bulk create manifesto:', bulk.status, (await bulk.text()).slice(0, 150));
  }
  const patch = await fetch(`${base}/db/content/manifesto`, {
    method: 'PATCH',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      contentBody: node.content ?? md,
      blobHash: node.blobHash,
      reach: 'commons',
      description: node.description,
    }),
  });
  const patched: any = await patch.json().catch(() => ({}));
  console.log('PATCH manifesto:', patch.status, JSON.stringify(patched).slice(0, 200));
  const target = patched.dhtAnchorHash;
  if (!target) throw new Error('no dhtAnchorHash after PATCH — conductor anchor did not land');

  // 2. Declare the EARNED canonical head as W under matthew's delegation.
  const head = await fetch(`${base}/db/content/manifesto/head`, {
    method: 'POST',
    headers: { 'content-type': 'application/json', 'X-Agent-Cid': W },
    body: JSON.stringify({ headActionHash: target, delegation }),
  });
  console.log('DECLARE head (delegated):', head.status, (await head.text()).slice(0, 300));
  process.exit(head.ok ? 0 : 1);
}

const phase = process.argv[2];
if (phase === 'grant') grant().catch(e => { console.error(e); process.exit(1); });
else if (phase === 'declare') declare().catch(e => { console.error(e); process.exit(1); });
else { console.error('usage: device-ceremony.ts grant|declare'); process.exit(2); }
