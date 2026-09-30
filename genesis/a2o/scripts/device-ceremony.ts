/** Explicit additive enrollment and a separate root-author content grant ceremony.
 * Usage: device-ceremony.ts credentials|bootstrap|enroll|verify|revoke|grant CONFIG.json
 * CONFIG names operator/device ConductorOptions, humanAction, contentDna and a
 * durable stateDir outside build artifacts. Existing browser identities persist.
 */
import { mkdir, open, readFile, rename, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

import {
  AdminWebsocket,
  decodeHashFromBase64,
  encodeHashToBase64,
  getSigningCredentials,
  generateSigningKeyPair,
  randomCapSecret,
} from '@holochain/client';

import {
  connectConductor,
  conductorSocketOptions,
  loadSigningCredentials,
} from './lib/steward-conductor.js';

import type { ConductorOptions, HostedConductorReceipt } from './lib/steward-conductor.js';
import type { HeadDelegationDocument } from './lib/steward-delegation.js';
import type {
  AppInfo,
  CellId,
  CapGrantInfo,
  GrantZomeCallCapabilityRequest,
} from '@holochain/client';

interface Config {
  operator: ConductorOptions;
  device: ConductorOptions;
  humanAction: string;
  contentDna: string;
  stateDir: string;
  supersedes?: string;
  contentGrant?: {
    manifest: string;
    storage: string;
    source: ConductorOptions;
    grantors: Record<string, ConductorOptions>;
    validUntil: number;
  };
}
export interface ScopedCredentialTarget {
  conductor: ConductorOptions;
  functions: string[];
}
export interface ScopedCredentialsConfig {
  credentialTargets: ScopedCredentialTarget[];
  /** Recover an interrupted call only from its existing exact native grant. */
  resumePending?: boolean;
}
export interface ScopedCredentialAdmin {
  listApps(): Promise<AppInfo[]>;
  listGrants(appId: string): Promise<[CellId, CapGrantInfo[]][]>;
  grant(request: GrantZomeCallCapabilityRequest): Promise<Uint8Array>;
  close(): Promise<void>;
}
interface ScopedReceipt {
  version: 1;
  context: {
    adminWs: string;
    appWs: string;
    appId: string;
    role: string;
    zome: string;
    agent: string;
    dna: string;
    functions: string[];
  };
  keypair: string;
  signingAgentKey: string;
  capSecret: string;
  tag: string;
  capActionHash?: string;
}
const scopedFunctions = new Set([
  'grant_head_delegation',
  'accept_delegated_head',
  'get_accepted_delegated_head',
  'get_content_lineage',
  'get_record_for_action',
  'preflight_head_publication',
  'resolve_content_head_local',
  'resolve_canonical_election',
]);
async function scopedAdmin(config: ConductorOptions): Promise<ScopedCredentialAdmin> {
  const admin = await AdminWebsocket.connect({
    url: new URL(config.adminWs),
    wsClientOptions: await conductorSocketOptions(config, config.adminWs),
  });
  return {
    listApps: async () => admin.listApps({}),
    listGrants: async appId =>
      admin.listCapabilityGrants({ installed_app_id: appId, include_revoked: true }),
    grant: async input => admin.grantZomeCallCapability(input),
    close: async () => {
      await admin.client.close();
    },
  };
}
async function optionalReceipt(path: string): Promise<ScopedReceipt | undefined> {
  try {
    return JSON.parse(await readFile(path, 'utf8')) as ScopedReceipt;
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') return undefined;
    throw new Error('Scoped credential receipt is unreadable; refusing replacement');
  }
}
function sameBytes(a: Uint8Array, b: Uint8Array): boolean {
  return Buffer.from(a).equals(Buffer.from(b));
}
function matchingCap(
  receipt: ScopedReceipt,
  cell: CellId,
  grants: [CellId, CapGrantInfo[]][]
): CapGrantInfo | undefined {
  const signer = Buffer.from(receipt.signingAgentKey, 'hex');
  const candidates = grants
    .flatMap(([id, values]) =>
      sameBytes(id[0], cell[0]) && sameBytes(id[1], cell[1]) ? values : []
    )
    .filter(
      ({ cap_grant: cap }) =>
        cap.tag === receipt.tag ||
        (cap.access.type === 'assigned' &&
          cap.access.value.assignees.some(a => sameBytes(a, signer)))
    );
  if (!candidates.length) return undefined;
  if (candidates.length !== 1)
    throw new Error('Ambiguous native signing grants; refusing replacement');
  const candidate = candidates[0];
  const cap = candidate.cap_grant;
  const expected = receipt.context.functions.map(fn => ['content_store', fn]);
  const actual =
    cap.functions.type === 'listed'
      ? [...cap.functions.value].sort((a, b) => a[1].localeCompare(b[1]))
      : null;
  if (
    (candidate.revoked_at ?? undefined) !== undefined ||
    cap.tag !== receipt.tag ||
    cap.access.type !== 'assigned' ||
    cap.access.value.assignees.length !== 1 ||
    !sameBytes(cap.access.value.assignees[0], signer) ||
    !sameBytes(cap.access.value.secret, Buffer.from(receipt.capSecret, 'hex')) ||
    JSON.stringify(actual) !== JSON.stringify(expected) ||
    (receipt.capActionHash && receipt.capActionHash !== encoded(candidate.action_hash))
  ) {
    throw new Error('Native signing grant is revoked or differs from the scoped receipt');
  }
  return candidate;
}

/** Explicit capability ceremony. Ordinary publication never calls this function. */
export async function scopedCredentials(
  config: ScopedCredentialsConfig,
  connect: (options: ConductorOptions) => Promise<ScopedCredentialAdmin> = scopedAdmin
): Promise<{ agent: string; dna: string; functions: string[]; capActionHash: string }[]> {
  if (!Array.isArray(config.credentialTargets) || !config.credentialTargets.length)
    throw new Error('Explicit scoped credential targets are required');
  const connections: ScopedCredentialAdmin[] = [];
  const plans: {
    admin: ScopedCredentialAdmin;
    cell: CellId;
    dir: string;
    path: string;
    pendingPath: string;
    context: ScopedReceipt['context'];
    receipt?: ScopedReceipt;
    recovered?: CapGrantInfo;
  }[] = [];
  const paths = new Set<string>();
  try {
    // Preflight every native target AND every existing receipt before any write.
    for (const target of config.credentialTargets) {
      const c = target.conductor;
      if (
        c.hosted ||
        c.role !== 'lamad' ||
        c.zome !== 'content_store' ||
        !c.expectedAgent ||
        !c.expectedDna ||
        !c.signingCredentialsDir ||
        !Array.isArray(target.functions) ||
        !target.functions.length ||
        target.functions.some(fn => !scopedFunctions.has(fn)) ||
        new Set(target.functions).size !== target.functions.length
      )
        throw new Error(
          'Scoped credentials require an exact Lamad cell and listed content functions'
        );
      const dir = resolve(c.signingCredentialsDir);
      if (/(^|\/)(target|dist|node_modules|build)(\/|$)/.test(dir))
        throw new Error('Scoped credentials require durable state outside build artifacts');
      // Validate both endpoints before connecting; bearer credentials never enter URLs.
      await conductorSocketOptions(c, c.appWs);
      const admin = await connect(c);
      connections.push(admin);
      const app = (await admin.listApps()).find(a => a.installed_app_id === c.appId);
      const cells = app?.cell_info[c.role]?.filter(info => info.type === 'provisioned') ?? [];
      if (!app || encoded(app.agent_pub_key) !== c.expectedAgent || cells.length !== 1)
        throw new Error('Scoped credential native app or role mismatch');
      const info = cells[0];
      if (info.type !== 'provisioned') throw new Error('Scoped credential cell is not provisioned');
      const cell = info.value.cell_id;
      if (encoded(cell[1]) !== c.expectedAgent || encoded(cell[0]) !== c.expectedDna)
        throw new Error('Scoped credential native cell agent or DNA mismatch');
      const name = `${cell.map(h => Buffer.from(h).toString('hex')).join('-')}.json`;
      const path = join(dir, name),
        pendingPath = join(dir, 'pending', name);
      if (paths.has(path)) throw new Error('Duplicate scoped credential target');
      paths.add(path);
      const context = {
        adminWs: c.adminWs,
        appWs: c.appWs,
        appId: c.appId,
        role: c.role,
        zome: c.zome,
        agent: c.expectedAgent,
        dna: c.expectedDna,
        functions: [...target.functions].sort((a, b) => a.localeCompare(b)),
      };
      const active = await optionalReceipt(path),
        pending = await optionalReceipt(pendingPath);
      for (const saved of [active, pending]) {
        if (
          saved &&
          (saved.version !== 1 || JSON.stringify(saved.context) !== JSON.stringify(context))
        )
          throw new Error('Existing credential scope differs; refusing replacement');
      }
      if (active && !active.capActionHash)
        throw new Error('Existing credential lacks native cap action');
      if (
        active &&
        pending &&
        (active.signingAgentKey !== pending.signingAgentKey ||
          active.capSecret !== pending.capSecret)
      )
        throw new Error('Pending and active credentials differ');
      if (!active && pending && config.resumePending !== true)
        throw new Error('Pending grant requires explicit resumePending; no new grant issued');
      if (!active && !pending && config.resumePending === true)
        throw new Error(
          'Missing durable pending state; resume cannot mint replacement credentials'
        );
      const receipt = active ?? pending;
      if (receipt) await loadSigningCredentials(active ? dir : join(dir, 'pending'), cell);
      const grants = await admin.listGrants(c.appId);
      const recovered = receipt ? matchingCap(receipt, cell, grants) : undefined;
      if (receipt && !recovered)
        throw new Error(
          'Existing native grant not found; receipt remains pending, no grant retried'
        );
      plans.push({ admin, cell, dir, path, pendingPath, context, receipt, recovered });
    }
    const results = [];
    for (const plan of plans) {
      let receipt = plan.receipt;
      if (!receipt) {
        const [keyPair, signingKey] = await generateSigningKeyPair();
        receipt = {
          version: 1,
          context: plan.context,
          keypair: Buffer.from(keyPair.privateKey.slice(0, 32)).toString('hex'),
          signingAgentKey: Buffer.from(signingKey).toString('hex'),
          capSecret: Buffer.from(await randomCapSecret()).toString('hex'),
          tag: `scoped-content-signing:${encoded(signingKey)}`,
        };
        await mkdir(join(plan.dir, 'pending'), { recursive: true, mode: 0o700 });
        await persist(plan.pendingPath, receipt);
        try {
          const action = await plan.admin.grant({
            cell_id: plan.cell,
            cap_grant: {
              tag: receipt.tag,
              functions: {
                type: 'listed',
                value: receipt.context.functions.map(fn => ['content_store', fn]),
              },
              access: {
                type: 'assigned',
                value: {
                  secret: Buffer.from(receipt.capSecret, 'hex'),
                  assignees: [Buffer.from(receipt.signingAgentKey, 'hex')],
                },
              },
            },
          });
          receipt = { ...receipt, capActionHash: encoded(action) };
          if (!matchingCap(receipt, plan.cell, await plan.admin.listGrants(receipt.context.appId)))
            throw new Error('Native grant confirmation remains pending');
        } catch {
          throw new Error(
            'Grant response unavailable; pending signer preserved, explicit resume required'
          );
        }
      } else if (plan.recovered) {
        receipt = { ...receipt, capActionHash: encoded(plan.recovered.action_hash) };
      }
      if (!receipt.capActionHash) throw new Error('Native cap action receipt is missing');
      if (!(await optionalReceipt(plan.path))) await persist(plan.path, receipt);
      results.push({
        agent: receipt.context.agent,
        dna: receipt.context.dna,
        functions: receipt.context.functions,
        capActionHash: receipt.capActionHash,
      });
    }
    return results;
  } finally {
    await Promise.all(connections.map(async admin => admin.close()));
  }
}

interface Receipt {
  action_hash: Uint8Array;
  entry_hash: Uint8Array;
}
interface Proof {
  agent: Uint8Array;
  signature: Uint8Array;
}
interface AuthorityState {
  humanAction: string;
  authority: string;
  operatorAgent: string;
  networkDna: string;
}
type BindingState = AuthorityState & { binding: string; deviceAgent: string; contentDna: string };
const hash = decodeHashFromBase64;
const encoded = encodeHashToBase64;

async function readState<T>(path: string): Promise<T> {
  try {
    return JSON.parse(await readFile(path, 'utf8')) as T;
  } catch {
    throw new Error(`Missing durable identity state ${path}; explicit enrollment is required`);
  }
}
async function persist(path: string, value: unknown): Promise<void> {
  // Exclusive write: restart never overwrites an identity or replaces a key.
  const file = await open(path, 'wx', 0o600);
  try {
    await file.writeFile(`${JSON.stringify(value, null, 2)}\n`);
    await file.sync();
  } finally {
    await file.close();
  }
  const directory = await open(dirname(path), 'r');
  try {
    await directory.sync();
  } finally {
    await directory.close();
  }
}
function options(config: ConductorOptions, role: string): ConductorOptions {
  if (!config.expectedAgent || !config.signingCredentialsDir) {
    throw new Error(
      'Every ceremony participant must explicitly name expectedAgent and signingCredentialsDir'
    );
  }
  return {
    ...config,
    role,
    zome: role === 'lamad' ? 'content_store' : role,
    // expectedDna describes the profile's named role. Cross-role context is
    // separately verified by the signed binding and explicit contentDna.
    expectedDna: config.role === role ? config.expectedDna : undefined,
  };
}

async function credentials(config: ConductorOptions): Promise<void> {
  const checked = options(config, config.role);
  const dir = checked.signingCredentialsDir!;
  if (checked.hosted) {
    await hostedCredentials(checked);
    return;
  }
  const admin = await AdminWebsocket.connect({
    url: new URL(checked.adminWs),
    wsClientOptions: await conductorSocketOptions(checked, checked.adminWs),
  });
  try {
    const apps = await admin.listApps({});
    const app = apps.find(a => a.installed_app_id === checked.appId);
    if (!app) throw new Error('Explicit credential ceremony app is not installed');
    await mkdir(dir, { recursive: true, mode: 0o700 });
    for (const infos of Object.values(app.cell_info)) {
      for (const info of infos) {
        if (info.type !== 'provisioned') continue;
        const cell = info.value.cell_id;
        if (encoded(cell[1]) !== checked.expectedAgent)
          throw new Error('Credential ceremony agent mismatch');
        const path = join(dir, `${cell.map(h => Buffer.from(h).toString('hex')).join('-')}.json`);
        try {
          await readFile(path);
          await loadSigningCredentials(dir, cell);
          continue;
        } catch (error) {
          if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
        }
        // This is the ONLY explicit phase permitted to create app signing grants.
        await admin.authorizeSigningCredentials(cell);
        const signing = getSigningCredentials(cell);
        if (!signing) throw new Error('Conductor did not return signing credentials');
        await persist(path, {
          keypair: Buffer.from(signing.keyPair.privateKey.slice(0, 32)).toString('hex'),
          signingAgentKey: Buffer.from(signing.signingKey).toString('hex'),
          capSecret: Buffer.from(signing.capSecret).toString('hex'),
        });
      }
    }
  } finally {
    await admin.client.close();
  }
}

async function hostedCredentials(config: ConductorOptions): Promise<void> {
  const hosted = config.hosted!;
  const auth = await readState<{ token: string }>(hosted.authFile);
  const sessionResponse = await fetch(new URL('/auth/me', hosted.doorway), {
    headers: { Authorization: `Bearer ${auth.token}` },
  });
  if (!sessionResponse.ok)
    throw new Error('Hosted credentials require a current authenticated session');
  const session = (await sessionResponse.json()) as { agentPubKey: string };
  const sessionAgent = session.agentPubKey.startsWith('u')
    ? session.agentPubKey
    : encoded(new Uint8Array(Buffer.from(session.agentPubKey, 'base64')));
  if (sessionAgent !== config.expectedAgent)
    throw new Error('Authenticated hosted account differs from expected signing agent');
  let stored: { keypair: string; signingAgentKey: string; capSecret: string };
  try {
    stored = JSON.parse(await readFile(hosted.signingFile, 'utf8')) as typeof stored;
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
    try {
      await readFile(hosted.receipt);
      throw new Error(
        'Hosted signing state is missing for an existing receipt; explicit reenrollment required'
      );
    } catch (receiptError) {
      if ((receiptError as NodeJS.ErrnoException).code !== 'ENOENT') throw receiptError;
    }
    const [keyPair, signingKey] = await generateSigningKeyPair();
    stored = {
      keypair: Buffer.from(keyPair.privateKey.slice(0, 32)).toString('hex'),
      signingAgentKey: Buffer.from(signingKey).toString('hex'),
      capSecret: Buffer.from(await randomCapSecret()).toString('hex'),
    };
    await persist(hosted.signingFile, stored);
  }
  const response = await fetch(new URL('/hc/connect', hosted.doorway), {
    method: 'POST',
    headers: { Authorization: `Bearer ${auth.token}`, 'Content-Type': 'application/json' },
    body: JSON.stringify({
      signingKey: Buffer.from(stored.signingAgentKey, 'hex').toString('base64'),
      capSecret: Buffer.from(stored.capSecret, 'hex').toString('base64'),
    }),
  });
  if (!response.ok) throw new Error(`Hosted credential ceremony refused: HTTP ${response.status}`);
  const receipt = (await response.json()) as HostedConductorReceipt;
  // Chaperone's transport hashes are STANDARD base64 bytes, not Holochain
  // multibase strings. Persist canonical hashes for the existing native helper.
  const chaperoneHash = (value: string): string => {
    const raw = new Uint8Array(Buffer.from(value, 'base64'));
    if (raw.length !== 39) throw new Error('Invalid hosted cell hash length');
    return encoded(raw);
  };
  receipt.agentPubKey = chaperoneHash(receipt.agentPubKey);
  for (const [role, pair] of Object.entries(receipt.cellIds)) {
    receipt.cellIds[role] = [chaperoneHash(pair[0]), chaperoneHash(pair[1])];
  }

  if (receipt.agentPubKey !== config.expectedAgent || receipt.installedAppId !== config.appId)
    throw new Error('Hosted credential ceremony agent/app mismatch');
  const requested = receipt.cellIds[config.role];
  if (!requested || (config.expectedDna && requested[0] !== config.expectedDna))
    throw new Error('Hosted credential ceremony DNA mismatch');
  await mkdir(config.signingCredentialsDir!, { recursive: true, mode: 0o700 });
  for (const pair of Object.values(receipt.cellIds)) {
    if (pair[1] !== config.expectedAgent) throw new Error('Hosted role agent mismatch');
    const cell = pair.map(hash) as [Uint8Array, Uint8Array];
    const path = join(
      config.signingCredentialsDir!,
      `${cell.map(h => Buffer.from(h).toString('hex')).join('-')}.json`
    );
    try {
      const existing = JSON.parse(await readFile(path, 'utf8')) as typeof stored;
      if (JSON.stringify(existing) !== JSON.stringify(stored))
        throw new Error('Existing hosted cell credentials differ; refusing replacement');
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
      await persist(path, stored);
    }
    await loadSigningCredentials(config.signingCredentialsDir!, cell);
  }
  receipt.doorway = hosted.doorway;
  // Chaperone issues a reusable one-hour token. Keep a conservative expiry.
  receipt.expiresAt = Date.now() + 3_500_000;
  const temporary = `${hosted.receipt}.pending`;
  await writeFile(temporary, `${JSON.stringify(receipt, null, 2)}\n`, { mode: 0o600 });
  await rename(temporary, hosted.receipt);
}

/** Content stewardship is a separate act by each immutable root's author. */
async function grantContent(config: Config, state: BindingState): Promise<void> {
  const input = config.contentGrant;
  if (!input || !Number.isSafeInteger(input.validUntil) || input.validUntil <= Date.now() * 1000)
    throw new Error(
      'grant requires an explicit contentGrant manifest, source, grantors and future validUntil'
    );
  const ids: unknown = JSON.parse(await readFile(input.manifest, 'utf8'));
  if (!Array.isArray(ids) || !ids.every(id => typeof id === 'string'))
    throw new Error('content grant manifest must be an array of exact content ids');
  const outputPath = join(config.stateDir, 'grants.json');
  let grants: Record<string, HeadDelegationDocument> = {};
  try {
    grants = JSON.parse(await readFile(outputPath, 'utf8')) as typeof grants;
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
  }
  const source = await connectConductor({
    ...options(input.source, 'lamad'),
    expectedDna: config.contentDna,
  });
  try {
    for (const id of ids) {
      const response = await fetch(`${input.storage}/db/content/${encodeURIComponent(id)}`, {
        signal: AbortSignal.timeout(30_000),
      });
      if (!response.ok)
        throw new Error(`Cannot resolve immutable content root for ${id}: HTTP ${response.status}`);
      const row = (await response.json()) as {
        dhtAnchorHash?: string;
        content?: { dhtAnchorHash?: string };
      };
      const anchor = row.content?.dhtAnchorHash ?? row.dhtAnchorHash;
      if (!anchor) throw new Error(`Cannot grant unanchored content ${id}`);
      const lineage = await source.call<{
        root_action_hash: Uint8Array;
        root_author: Uint8Array;
        content_id: string;
        truncated: boolean;
      }>('get_content_lineage', { action_hash: hash(anchor), local: false });
      if (lineage.content_id !== id || lineage.truncated)
        throw new Error(`Incomplete or mismatched lineage for ${id}`);
      const root = encoded(lineage.root_action_hash);
      const author = encoded(lineage.root_author);
      const previous = grants[id];
      if (previous) {
        if (
          previous.rootActionHash !== root ||
          previous.grantor !== author ||
          previous.delegate !== state.deviceAgent ||
          previous.dnaHash !== config.contentDna ||
          previous.scope !== id ||
          previous.validUntil !== input.validUntil
        )
          throw new Error(
            `Existing grant ${id} differs; retain its receipt and use an explicit new grant state directory`
          );
        continue;
      }
      const profile = input.grantors[author];
      if (!profile)
        throw new Error(`No explicitly configured root-author signer for ${id} (${author})`);
      const grantor = await connectConductor({
        ...options(profile, 'lamad'),
        expectedAgent: author,
        expectedDna: config.contentDna,
      });
      try {
        const grant = await grantor.call<{
          payload: {
            grantor: Uint8Array;
            delegate: Uint8Array;
            scope: string;
            valid_until: number;
            root_action_hash: Uint8Array;
            dna_hash: Uint8Array;
          };
          signature: Uint8Array;
        }>('grant_head_delegation', {
          delegate: hash(state.deviceAgent),
          scope: id,
          valid_until: input.validUntil,
          root_action_hash: lineage.root_action_hash,
        });
        grants[id] = {
          grantor: encoded(grant.payload.grantor),
          delegate: encoded(grant.payload.delegate),
          scope: grant.payload.scope,
          validUntil: grant.payload.valid_until,
          rootActionHash: encoded(grant.payload.root_action_hash),
          dnaHash: encoded(grant.payload.dna_hash),
          signature: Buffer.from(grant.signature).toString('base64'),
        };
        const temporary = `${outputPath}.tmp`;
        await writeFile(temporary, `${JSON.stringify(grants, null, 2)}\n`, { mode: 0o600 });
        await rename(temporary, outputPath);
        console.log(`Content-scoped grant recorded for ${id}, root ${root}`);
      } finally {
        await grantor.close();
      }
    }
  } finally {
    await source.close();
  }
}

async function ceremony(phase: string, config: Config): Promise<void> {
  const stateDir = resolve(config.stateDir);
  if (/(^|\/)(target|dist|node_modules|build)(\/|$)/.test(stateDir)) {
    throw new Error('Identity state must live outside disposable build artifacts');
  }
  const authorityPath = join(stateDir, 'authority.json');
  const bindingPath = join(stateDir, 'binding.json');
  if (phase === 'credentials') {
    await credentials(config.device);
    await credentials(config.operator);
    return;
  }
  if (phase === 'bootstrap') {
    const operator = await connectConductor(options(config.operator, 'mishpat'));
    try {
      await mkdir(stateDir, { recursive: true, mode: 0o700 });
      // Require absence BEFORE any notarization; existing state is never replaced.
      try {
        await readFile(authorityPath);
        throw new Error('Identity already bootstrapped');
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
      }
      const result = await operator.call<Receipt>(
        'bootstrap_device_identity',
        hash(config.humanAction)
      );
      await persist(authorityPath, {
        humanAction: config.humanAction,
        authority: encoded(result.action_hash),
        operatorAgent: operator.agent,
        networkDna: operator.dna,
        authorization: 'development-network',
      });
      console.log(`Development-network identity authority recorded: ${authorityPath}`);
    } finally {
      await operator.close();
    }
    return;
  }
  const authority = await readState<AuthorityState>(authorityPath);
  if (
    authority.humanAction !== config.humanAction ||
    authority.operatorAgent !== config.operator.expectedAgent
  ) {
    throw new Error('Configured operator identity differs from durable authority');
  }
  const device = await connectConductor({
    ...options(config.device, 'mishpat'),
    expectedDna: authority.networkDna,
  });
  try {
    if (phase === 'enroll') {
      try {
        await readFile(bindingPath);
        throw new Error(
          'Device already enrolled; use its existing binding or explicit reenrollment state directory'
        );
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
      }
      const content = await connectConductor({
        ...options(config.device, 'lamad'),
        expectedDna: config.contentDna,
      });
      await content.close();
      const intent = {
        domain: 'elohim:device-enrollment:v1',
        authority: hash(authority.authority),
        identity_root: hash(authority.humanAction),
        device_key: hash(device.agent),
        network_dna: hash(device.dna),
        content_dna: hash(config.contentDna),
        issued_at: Date.now() * 1000,
        supersedes: config.supersedes ? hash(config.supersedes) : null,
      };
      const possession = await device.call<Proof>('sign_device_enrollment', intent);
      const operator = await connectConductor({
        ...options(config.operator, 'mishpat'),
        expectedDna: device.dna,
      });
      let controller: Proof;
      try {
        controller = await operator.call<Proof>('sign_device_enrollment', intent);
      } finally {
        await operator.close();
      }
      const binding = await device.call<Receipt>('enroll_identity_device', {
        action: 'binds-identity',
        binding_kind: 'device-v1',
        intent,
        controllers: [controller],
        possession,
      });
      const state: BindingState = {
        ...authority,
        binding: encoded(binding.action_hash),
        deviceAgent: device.agent,
        contentDna: config.contentDna,
      };
      // Persist the exact authored reference BEFORE linking; an interrupted link
      // is resumed by verify, never by creating another identity or binding.
      await persist(bindingPath, state);
    }
    const state = await readState<BindingState>(bindingPath);
    if (state.deviceAgent !== device.agent || state.contentDna !== config.contentDna)
      throw new Error('Durable binding does not match actual device/DNA');
    if (phase === 'grant') {
      await device.call('verify_device_binding', {
        binding: hash(state.binding),
        expected_device: hash(device.agent),
        expected_content_dna: hash(config.contentDna),
      });
      await grantContent(config, state);
      return;
    }
    if (phase === 'revoke') {
      const operator = await connectConductor({
        ...options(config.operator, 'mishpat'),
        expectedDna: device.dna,
      });
      try {
        const revocation = {
          action: 'revokes-commitment',
          binding_kind: 'device-revocation-v1',
          target: hash(state.binding),
          authority: hash(state.authority),
          network_dna: hash(device.dna),
          signatures: [] as Proof[],
        };
        revocation.signatures.push(
          await operator.call<Proof>('sign_device_revocation', revocation)
        );
        const result = await operator.call<Receipt>('revoke_identity_device', revocation);
        await persist(join(stateDir, 'revocation.json'), {
          binding: state.binding,
          action: encoded(result.action_hash),
        });
      } finally {
        await operator.close();
      }
      return;
    }
    if (phase !== 'enroll' && phase !== 'verify')
      throw new Error(`Unknown ceremony phase: ${phase}`);
    const verification = await device.call('verify_device_binding', {
      binding: hash(state.binding),
      expected_device: hash(device.agent),
      expected_content_dna: hash(config.contentDna),
    });
    const identity = await connectConductor(options(config.device, 'imagodei'));
    try {
      // Idempotent reference registration; this never creates another Human.
      await identity.call('register_device_identity', {
        binding: hash(state.binding),
        expected_content_dna: hash(config.contentDna),
      });
      const human = await identity.call('get_my_human', null);
      console.log(
        JSON.stringify({ binding: state.binding, deviceAgent: device.agent, human, verification })
      );
    } finally {
      await identity.close();
    }
  } finally {
    await device.close();
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [phase, configPath] = process.argv.slice(2);
  if (!phase || !configPath) {
    console.error(
      'usage: device-ceremony.ts credentials|bootstrap|enroll|verify|revoke|grant CONFIG.json'
    );
    process.exitCode = 2;
  } else {
    readState<Config | ScopedCredentialsConfig>(configPath)
      .then(async config => {
        if ('credentialTargets' in config) {
          if (phase !== 'credentials')
            throw new Error('Scoped targets support only the explicit credentials phase');
          const receipts = await scopedCredentials(config);
          console.log(JSON.stringify(receipts));
        } else await ceremony(phase, config);
      })
      .catch(error => {
        console.error(error instanceof Error ? error.message : 'Device ceremony failed');
        process.exitCode = 1;
      });
  }
}
