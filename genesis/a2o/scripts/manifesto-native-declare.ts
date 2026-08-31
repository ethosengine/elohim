/**
 * Device-native manifesto head-move, workspace side (no pipeline, no doorway seed).
 * W2 (this workspace's device agent) authors the manifesto with the real repo bytes
 * and declares it as the EARNED cross-root canonical head; the fleet adopts that
 * over its own per-root election, so both doorways converge on the workspace bytes.
 *
 * Run: cd genesis/a2o && DEVICE_AGENT=<W2> pnpm exec tsx scripts/manifesto-native-declare.ts
 */
import { AdminWebsocket, AppWebsocket, encodeHashToBase64, decodeHashFromBase64 } from '@holochain/client';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';

const STORAGE = 'http://localhost:8090';
const ADMIN = `ws://127.0.0.1:${process.env.WS_ADMIN_PORT ?? '41347'}`;
const APP_PORT = Number(process.env.WS_APP_PORT ?? 4485);
const APP_ID = 'elohim';
const ID = 'manifesto';
const MD = '/projects/elohim/genesis/docs/content/elohim-protocol/manifesto.md';

async function main() {
  const body = readFileSync(MD, 'utf8');
  const hash = 'sha256-' + createHash('sha256').update(body, 'utf8').digest('hex');
  console.log('manifesto bytes', Buffer.byteLength(body), 'blobHash', hash.slice(0, 22));

  // 1. blob PUT (localhost, no auth)
  const put = await fetch(`${STORAGE}/blob/${hash}`, {
    method: 'PUT', headers: { 'content-type': 'text/markdown' }, body,
  });
  console.log('blob PUT', put.status);

  // 2. ensure a manifesto row exists (bulk is idempotent — skipped if present)
  await fetch(`${STORAGE}/db/content/bulk`, {
    method: 'POST', headers: { 'content-type': 'application/json' },
    body: JSON.stringify([{
      id: ID, title: 'Elohim Protocol — Manifesto', description: 'Digital Infrastructure for Human Flourishing',
      contentType: 'article', contentFormat: 'markdown', reach: 'commons',
      tags: ['manifesto'], contentBody: body, blobHash: hash,
    }]),
  }).then(r => r.text()).then(t => console.log('bulk', t.slice(0, 80)));

  // 3. PATCH with the notarized fields → re-notarize through W2's conductor (anchors it)
  const patch = await fetch(`${STORAGE}/db/content/${ID}`, {
    method: 'PATCH', headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ blobHash: hash, reach: 'commons', contentBody: body }),
  });
  const patched: any = await patch.json().catch(() => ({}));
  const target = patched.dhtAnchorHash ?? patched.content?.dhtAnchorHash;
  console.log('PATCH', patch.status, 'anchor', String(target).slice(0, 22), 'reach', patched.reach ?? patched.content?.reach);
  if (!target) throw new Error(`no anchor after PATCH: ${JSON.stringify(patched).slice(0, 200)}`);

  // 4. EARNED cross-root canonical declare, as W2 (local root author of the id).
  //    This is the link the fleet honours over its own per-root election.
  const admin = await AdminWebsocket.connect({ url: new URL(ADMIN), wsClientOptions: { origin: APP_ID } });
  const apps = await admin.listApps({});
  const app = apps.find(a => a.installed_app_id === APP_ID) ?? apps[0];
  const cells = new Map<string, any>();
  for (const [role, infos] of Object.entries(app.cell_info))
    for (const info of infos as any[])
      if (info.type === 'provisioned' && info.value?.cell_id) cells.set(role, info.value.cell_id);
  const lamad = cells.get('lamad');
  console.log('workspace agent (W2 / local root author):', encodeHashToBase64(lamad[1]));
  await admin.authorizeSigningCredentials(lamad);
  const token = await admin.issueAppAuthenticationToken({ installed_app_id: app.installed_app_id });
  const appWs = await AppWebsocket.connect({ url: new URL(`ws://127.0.0.1:${APP_PORT}`), token: token.token, wsClientOptions: { origin: APP_ID } });
  const out: any = await appWs.callZome({
    cell_id: lamad, zome_name: 'content_store', fn_name: 'declare_earned_canonical_head',
    payload: { id: ID, head_action_hash: target, carried_record: null, adopt_before_author: false, delegation: null },
  });
  console.log('EARNED canonical declared:', JSON.stringify({
    head: encodeHashToBase64(out.head_action_hash), author: encodeHashToBase64(out.author), canonical: out.canonical,
  }));
  console.log('blobHash for the watch:', hash);
  process.exit(0);
}
main().catch(e => { console.error(String(e).slice(0, 500)); process.exit(1); });
