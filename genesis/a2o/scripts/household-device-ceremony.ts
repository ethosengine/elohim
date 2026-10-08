/**
 * One-time device ceremony for the running household's `elohim` app (matthew), in-process:
 * operator = device = the cell agent. Mints a per-payload, exact-payload, assigned,
 * function-listed mandate grant for each gated mishpat ceremony call, then
 * bootstrap -> sign_device_enrollment -> enroll_identity_device -> verify_device_binding.
 * Artifacts persist under genesis/local-dev/household-dowell/device-ceremony/ (authority.json,
 * binding.json, receipts). Refuses if authority.json or binding.json already exist.
 * Usage: tsx scripts/household-device-ceremony.ts [adminPort=4444] [appPort=4445]
 */
import { randomBytes } from 'node:crypto';
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

import {
  AdminWebsocket,
  AppWebsocket,
  encodeHashToBase64,
  generateSigningKeyPair,
  getSigningCredentials,
  setSigningCredentials,
} from '@holochain/client';

import { scopedCapability } from './lib/steward-credential.js';

import type { InvocationMandate } from './lib/steward-credential.js';
import type { CellId } from '@holochain/client';

const adminPort = Number(process.argv[2] ?? 4444);
const appPort = Number(process.argv[3] ?? 4445);
const out = resolve(import.meta.dirname, '../../local-dev/household-dowell/device-ceremony');
const origin = { origin: 'elohim' };
const arr = (h: Uint8Array): number[] => Array.from(h);
const b64 = (h: Uint8Array): string => encodeHashToBase64(new Uint8Array(h));

mkdirSync(out, { recursive: true, mode: 0o700 });
for (const f of ['authority.json', 'binding.json'])
  if (existsSync(join(out, f))) throw new Error(`${f} exists; ceremony already performed`);

const admin = await AdminWebsocket.connect({
  url: new URL(`ws://127.0.0.1:${adminPort}`),
  wsClientOptions: origin,
});
const app = (await admin.listApps({})).find(a => a.installed_app_id === 'elohim');
if (!app) throw new Error('no elohim app');
const cells: Record<string, CellId> = {};
for (const [role, infos] of Object.entries(app.cell_info))
  for (const i of infos as { value?: { cell_id?: CellId } }[])
    if (i.value?.cell_id) cells[role] = i.value.cell_id;
for (const role of ['imagodei', 'mishpat', 'lamad'])
  await admin.authorizeSigningCredentials(cells[role]);
const broad = Object.fromEntries(
  ['imagodei', 'mishpat', 'lamad'].map(r => [r, getSigningCredentials(cells[r])!])
);
const ws = await AppWebsocket.connect({
  url: new URL(`ws://127.0.0.1:${appPort}`),
  token: (await admin.issueAppAuthenticationToken({ installed_app_id: 'elohim' })).token,
  wsClientOptions: origin,
});
const call = async <T>(role: string, zome: string, fn: string, payload: unknown): Promise<T> => {
  setSigningCredentials(cells[role], broad[role]);
  return (await ws.callZome({ cell_id: cells[role], zome_name: zome, fn_name: fn, payload })) as T;
};

const agent = cells.mishpat[1];
const mishpatDna = cells.mishpat[0];
const contentDna = cells.lamad[0];

/** Mint an exact-payload mandate grant on mishpat and return credentials that present it. */
async function exact(operation: string, payload: unknown) {
  const [keyPair, signingKey] = await generateSigningKeyPair();
  const mandate: InvocationMandate = {
    issuer: b64(agent),
    requester: encodeHashToBase64(signingKey),
    dna: b64(mishpatDna),
    delegate: null,
    subjects: [],
    operations: [operation],
    valid_until: (Date.now() + 15 * 60_000) * 1000,
    binding: null,
    policy: 'household-device-ceremony-v1',
    exact_payload_json: JSON.stringify(payload),
  };
  const secret = new Uint8Array(randomBytes(64));
  await admin.grantZomeCallCapability({
    cell_id: cells.mishpat,
    cap_grant: scopedCapability(mandate, 'mishpat', secret),
  });
  return { capSecret: secret, keyPair, signingKey };
}
async function gated<T>(operation: string, payload: unknown): Promise<T> {
  const creds = await exact(operation, payload);
  setSigningCredentials(cells.mishpat, creds);
  return (await ws.callZome({
    cell_id: cells.mishpat,
    zome_name: 'mishpat',
    fn_name: operation,
    payload,
  })) as T;
}

const human = await call<{ action_hash: Uint8Array }>('imagodei', 'imagodei', 'get_my_human', null);
const humanAction = new Uint8Array(human.action_hash);
const boot = await gated<{ action_hash: Uint8Array }>(
  'bootstrap_device_identity',
  arr(humanAction)
);
const authority = {
  humanAction: b64(humanAction),
  authority: b64(boot.action_hash),
  operatorAgent: b64(agent),
  networkDna: b64(mishpatDna),
};
writeFileSync(join(out, 'authority.json'), JSON.stringify(authority, null, 2), { mode: 0o600 });

const intentWire = {
  domain: 'elohim:device-enrollment:v1',
  authority: arr(boot.action_hash),
  identity_root: arr(humanAction),
  device_key: arr(agent),
  network_dna: arr(mishpatDna),
  content_dna: arr(contentDna),
  issued_at: Date.now() * 1000,
  supersedes: null,
};
const intent = {
  ...intentWire,
  authority: new Uint8Array(boot.action_hash),
  identity_root: humanAction,
  device_key: new Uint8Array(agent),
  network_dna: new Uint8Array(mishpatDna),
  content_dna: new Uint8Array(contentDna),
};
const proof = await (async () => {
  const creds = await exact('sign_device_enrollment', intentWire);
  setSigningCredentials(cells.mishpat, creds);
  return await ws.callZome({
    cell_id: cells.mishpat,
    zome_name: 'mishpat',
    fn_name: 'sign_device_enrollment',
    payload: intent,
  });
})();
const binding = await call<{ action_hash: Uint8Array }>(
  'mishpat',
  'mishpat',
  'enroll_identity_device',
  {
    action: 'binds-identity',
    binding_kind: 'device-v1',
    intent,
    controllers: [proof],
    possession: proof,
  }
);
const state = {
  ...authority,
  binding: b64(binding.action_hash),
  deviceAgent: b64(agent),
  contentDna: b64(contentDna),
};
writeFileSync(join(out, 'binding.json'), JSON.stringify(state, null, 2), { mode: 0o600 });
const verification = await call('mishpat', 'mishpat', 'verify_device_binding', {
  binding: new Uint8Array(binding.action_hash),
  expected_device: new Uint8Array(agent),
  expected_content_dna: new Uint8Array(contentDna),
});
writeFileSync(
  join(out, 'verification.json'),
  JSON.stringify(
    verification,
    (_k, v) =>
      v instanceof Uint8Array || v?.type === 'Buffer' ? b64(new Uint8Array(v.data ?? v)) : v,
    2
  )
);
console.log(JSON.stringify(state));
process.exit(0);
