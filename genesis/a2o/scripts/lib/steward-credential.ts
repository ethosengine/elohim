/** Explicit owner ceremony. Ordinary publication never calls this module. */
import { randomBytes } from 'node:crypto';
import {
  closeSync,
  fsyncSync,
  mkdirSync,
  openSync,
  readFileSync,
  renameSync,
  writeFileSync,
} from 'node:fs';
import { dirname, join, resolve } from 'node:path';

import { AdminWebsocket, decodeHashFromBase64, encodeHashToBase64 } from '@holochain/client';

import {
  conductorSocketOptions,
  loadSigningCredentials,
  withCellWriteLock,
  type ConductorOptions,
  HostedConductorReceipt,
} from './steward-conductor.js';

import type { CellId, SigningCredentials, ZomeCallCapGrant } from '@holochain/client';

export const MANDATE_TAG = 'elohim:invocation-mandate:v1:';
export interface InvocationMandate {
  issuer: string;
  requester: string;
  dna: string;
  delegate: string | null;
  subjects: { id: string; root: string }[];
  operations: string[];
  valid_until: number;
  binding: string | null;
  policy: string;
  exact_payload_json: string | null;
}

export interface CredentialPlan {
  connection: ConductorOptions;
  /** New private directory; existing daemon credentials are never overwritten. */
  outputDir: string;
  mandate: Omit<InvocationMandate, 'issuer' | 'requester' | 'dna'>;
}

const CONTENT_OPERATIONS = new Set([
  'grant_head_delegation',
  'accept_delegated_head',
  'stage_delegated_head_acceptance',
  'get_accepted_delegated_head',
  'preflight_head_publication',
  'declare_earned_canonical_head',
]);
const CEREMONY_OPERATIONS = new Set([
  'bootstrap_device_identity',
  'sign_device_consent',
  'sign_device_approval',
  'sign_device_enrollment',
  'sign_device_revocation',
  'revoke_identity_device',
  'affirm_identity_device',
  'witness_device_publication',
]);
const READS = ['get_content_lineage', 'resolve_content_head_local', 'resolve_canonical_election'];

/** Validate the concrete private grant before any admin write. Native callers
 * independently enforce these same bounds on the selected signed CapGrant. */
export function scopedCapability(
  mandate: InvocationMandate,
  zome: string,
  secret: Uint8Array,
  now = Date.now() * 1000
): ZomeCallCapGrant {
  for (const key of [
    mandate.issuer,
    mandate.requester,
    mandate.dna,
    mandate.delegate,
    mandate.binding,
  ]) {
    if (key !== null && decodeHashFromBase64(key).length !== 39)
      throw new Error('mandate must name native identities');
  }
  if (
    !Number.isSafeInteger(mandate.valid_until) ||
    mandate.valid_until <= now ||
    secret.length !== 64
  )
    throw new Error('mandate requires future expiry and a private capability secret');
  if (
    !mandate.policy.trim() ||
    mandate.subjects.length > 128 ||
    !mandate.operations.length ||
    mandate.operations.length > 8
  )
    throw new Error('mandate policy, subjects or operations invalid');
  const identity = mandate.operations.every(op => CEREMONY_OPERATIONS.has(op));
  const content = mandate.operations.every(op => CONTENT_OPERATIONS.has(op));
  if (
    (!identity && !content) ||
    (identity && zome !== 'mishpat') ||
    (content && zome !== 'content_store')
  )
    throw new Error('mandate mixes authority surfaces or names an unsupported operation');
  if (identity && !mandate.exact_payload_json)
    throw new Error('identity ceremony requires an exact canonical native payload');
  if (content && (!mandate.binding || !mandate.delegate || !mandate.subjects.length))
    throw new Error('content mandate requires a device binding, delegate and exact roots');
  const seen = new Set<string>();
  for (const subject of mandate.subjects) {
    if (
      !subject.id ||
      subject.id.includes('*') ||
      seen.has(subject.id) ||
      decodeHashFromBase64(subject.root).length !== 39
    )
      throw new Error('subjects must name unique exact content roots');
    seen.add(subject.id);
  }
  const tag = MANDATE_TAG + JSON.stringify(mandate);
  if (Buffer.byteLength(tag) > 32768) throw new Error('mandate exceeds native verification bound');
  return {
    tag,
    access: {
      type: 'assigned',
      value: { secret, assignees: [decodeHashFromBase64(mandate.requester)] },
    },
    functions: {
      type: 'listed',
      value: [
        ...new Set([
          ...mandate.operations,
          ...(content ? READS : ['verify_device_binding', 'verify_device_affirmation']),
        ]),
      ].map(fn => [zome, fn]),
    },
  };
}

function credentialName(cell: CellId): string {
  return `${cell.map(hash => Buffer.from(hash).toString('hex')).join('-')}.json`;
}

export function readCredentialPlan(path: string): CredentialPlan {
  return JSON.parse(readFileSync(path, 'utf8')) as CredentialPlan;
}

export async function issueCredential(
  plan: CredentialPlan
): Promise<{ capabilityAction: string; profile: string }> {
  const o = plan.connection;
  if (!o.expectedAgent || !o.expectedDna || !o.signingCredentialsDir || o.hosted)
    throw new Error(
      'issuance requires the owner admin connection and exact cell; doorway clients reuse the issued profile'
    );
  if (resolve(plan.outputDir) === resolve(o.signingCredentialsDir))
    throw new Error('ceremony output must not replace existing daemon credentials');
  const cell: CellId = [decodeHashFromBase64(o.expectedDna), decodeHashFromBase64(o.expectedAgent)];
  const credentials = await loadSigningCredentials(o.signingCredentialsDir, cell);
  const mandate: InvocationMandate = {
    ...plan.mandate,
    issuer: o.expectedAgent,
    dna: o.expectedDna,
    requester: encodeHashToBase64(credentials.signingKey),
  };
  // A distinct secret prevents the conductor selecting an overlapping old broad
  // grant for this durable signing key. Secrets never enter public descriptors.
  const secret = new Uint8Array(randomBytes(64));
  const grant = scopedCapability(mandate, o.zome ?? 'content_store', secret);
  mkdirSync(plan.outputDir, { recursive: true, mode: 0o700 });
  const profile = join(plan.outputDir, credentialName(cell));
  // Reserve/persist custody before the chain write. A crash never loses the new
  // secret; an incomplete descriptor means explicit reconciliation is required.
  const stored = privateProfile(credentials, secret);
  writeCustodyJson(profile, stored, true);
  const admin = await AdminWebsocket.connect({
    url: new URL(o.adminWs),
    wsClientOptions: await conductorSocketOptions(o, o.adminWs),
  });
  try {
    const apps = await admin.listApps({});
    const app = apps.find(a => a.installed_app_id === o.appId);
    if (
      !app?.cell_info[o.role]?.some(
        info =>
          info.type === 'provisioned' &&
          encodeHashToBase64(info.value.cell_id[0]) === o.expectedDna &&
          encodeHashToBase64(info.value.cell_id[1]) === o.expectedAgent
      )
    )
      throw new Error('ceremony owner app/cell differs from plan');
    const action = await withCellWriteLock(o.chainWriteLockDir, cell, async remainingMs =>
      admin.grantZomeCallCapability({ cell_id: cell, cap_grant: grant }, remainingMs)
    );
    const capabilityAction = encodeHashToBase64(action);
    writeCustodyJson(
      join(plan.outputDir, 'descriptor.json'),
      {
        mandate,
        capabilityAction,
        connection: {
          ...o,
          signingCredentialsDir: resolve(plan.outputDir),
          identitySigningCredentialsDir: o.identitySigningCredentialsDir ?? o.signingCredentialsDir,
        },
      },
      true
    );
    return { capabilityAction, profile };
  } finally {
    await admin.client.close();
  }
}

function privateProfile(credentials: SigningCredentials, secret: Uint8Array) {
  const privateKey = (credentials.keyPair as { privateKey: Uint8Array }).privateKey;
  if (!(privateKey instanceof Uint8Array) || privateKey.length < 32)
    throw new Error('existing signing custody has no valid seed');
  return {
    keypair: Buffer.from(privateKey.slice(0, 32)).toString('hex'),
    signingAgentKey: Buffer.from(credentials.signingKey).toString('hex'),
    capSecret: Buffer.from(secret).toString('hex'),
  };
}

/** The chaperone refreshes transport only. Both direct and hosted calls still
 * exercise the same native mandate, using the existing scoped signing profile. */
export async function refreshHostedCredential(o: ConductorOptions): Promise<void> {
  if (!o.hosted || !o.expectedAgent || !o.expectedDna || !o.signingCredentialsDir)
    throw new Error('hosted refresh requires an existing explicit scoped profile');
  const doorway = new URL(o.hosted.doorway);
  if (
    doorway.protocol !== 'https:' ||
    doorway.username ||
    doorway.password ||
    doorway.search ||
    doorway.hash
  )
    throw new Error('hosted ceremony requires an HTTPS issuing origin');
  const auth = JSON.parse(readFileSync(o.hosted.authFile, 'utf8')) as {
    token?: string;
    doorwayUrl?: string;
    expiresAt?: number;
  };
  if (
    !auth.token ||
    /[\r\n]/.test(auth.token) ||
    !auth.doorwayUrl ||
    new URL(auth.doorwayUrl).origin !== doorway.origin ||
    !auth.expiresAt ||
    auth.expiresAt * 1000 <= Date.now()
  )
    throw new Error('issuing doorway session is missing, expired or mismatched');
  const cell: CellId = [decodeHashFromBase64(o.expectedDna), decodeHashFromBase64(o.expectedAgent)];
  const credentials = await loadSigningCredentials(o.signingCredentialsDir, cell);
  const response = await fetch(new URL('/hc/connect', doorway), {
    method: 'POST',
    redirect: 'error',
    headers: { Authorization: `Bearer ${auth.token}`, 'content-type': 'application/json' },
    body: JSON.stringify({
      signingKey: Buffer.from(credentials.signingKey).toString('base64'),
      capSecret: Buffer.from(credentials.capSecret).toString('base64'),
      reuseCredentials: true,
    }),
    signal: AbortSignal.timeout(120_000),
  });
  if (!response.ok)
    throw new Error(`hosted credential transport refresh refused (${response.status})`);
  const result = (await response.json()) as Omit<HostedConductorReceipt, 'doorway' | 'expiresAt'>;
  // The chaperone's transport hashes are STANDARD base64 bytes, not Holochain
  // multibase strings (doorway chaperone.rs ConnectResponse); canonicalize them
  // before comparing and before persisting, as device-ceremony.ts already does.
  const chaperoneHash = (value: string): string => {
    if (/^u[A-Za-z0-9_-]{52}$/.test(value)) return value;
    const raw = new Uint8Array(Buffer.from(value, 'base64'));
    if (raw.length !== 39) throw new Error('hosted ceremony returned an invalid cell hash');
    return encodeHashToBase64(raw);
  };
  result.agentPubKey = chaperoneHash(result.agentPubKey);
  for (const [role, pair] of Object.entries(result.cellIds ?? {}))
    result.cellIds[role] = [chaperoneHash(pair[0]), chaperoneHash(pair[1])];
  if (
    result.agentPubKey !== o.expectedAgent ||
    result.installedAppId !== o.appId ||
    result.cellIds?.[o.role]?.[0] !== o.expectedDna ||
    result.cellIds?.[o.role]?.[1] !== o.expectedAgent
  )
    throw new Error('hosted ceremony resolved a different peer or network');
  writeCustodyJson(o.hosted.receipt, {
    ...result,
    doorway: doorway.origin,
    expiresAt: Math.min(auth.expiresAt * 1000, Date.now() + 3_600_000),
  });
}

/** Preserve native hash/signature bytes before Buffer.toJSON changes their shape. */
export function ceremonyResultDocument(value: unknown): unknown {
  if (value instanceof Uint8Array) return Array.from(value);
  if (Array.isArray(value)) return value.map(ceremonyResultDocument);
  if (value && typeof value === 'object')
    return Object.fromEntries(
      Object.entries(value).map(([key, item]) => [key, ceremonyResultDocument(item)])
    );
  return value;
}

/** Durable private custody. Exclusive issuance reserves the path before any
 * native write; replacement journals use a flushed atomic rename. */
export function writeCustodyJson(path: string, value: unknown, exclusive = false): void {
  const temporary = exclusive ? path : `${path}.${process.pid}.tmp`;
  const fd = openSync(temporary, exclusive ? 'wx' : 'w', 0o600);
  try {
    writeFileSync(fd, `${JSON.stringify(value, null, 2)}\n`);
    fsyncSync(fd);
  } finally {
    closeSync(fd);
  }
  if (!exclusive) renameSync(temporary, path);
  const directory = openSync(dirname(path), 'r');
  try {
    fsyncSync(directory);
  } finally {
    closeSync(directory);
  }
}
