/**
 * The hosted CI runner's client (`agent:ci-runner-jenkins-matthew`, plan "Jenkins as a
 * stewarded runner", slice B2).
 *
 * The runner is a HOSTED service agent: its cell runs on its sponsor's pool conductor and
 * it has no storage peer of its own. So it speaks three surfaces, all through the doorway:
 *
 * - its OWN cell, via `/hc/connect` + signed zome calls. The doorway grants a runner session
 *   a function-listed capability (doorway `routes/service_agent.rs::session_grant_scope`):
 *   `content_store::compute_task` on the content cell, and the read-only
 *   `mishpat::get_commitment_record` / `get_commitment_authority_links` on the governance
 *   cell. Everything else is refused by the conductor.
 * - the sponsor's chunk custody at `/api/v1/compute/payloads/{cid}` (elohim-storage
 *   `api/hosted_custody.rs`): inputs are stocked leased to the task CID, outputs drawn back
 *   naming the task's request action hash.
 * - nothing else. Tasks and grants are never read from anyone's storage HTTP API.
 *
 * Reuse: the websocket client is `connectConductor({ hosted })` from steward-conductor.ts;
 * the `/hc/connect` answer is canonicalized by steward-credential.ts; the task shape and the
 * chunking are genesis/agentic/compute/submission.mjs + payloads.mjs — the same functions the
 * household requester (workspace.mjs) uses, so a provider's worker cannot tell them apart.
 *
 * Secrets (ADMIN_KEY, the JWT, the derived signing seed and cap secret) arrive only through
 * the environment, live only in memory and in a 0700 temp directory the session removes,
 * and are never written to argv, stdout or an error message.
 */
import { createHash, createPrivateKey, createPublicKey } from 'node:crypto';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import {
  decodeHashFromBase64,
  encodeHashToBase64,
  hashFrom32AndType,
  HoloHashType,
} from '@holochain/client';
import { decode } from '@msgpack/msgpack';

import {
  connectConductor,
  type Conductor,
  type ConductorOptions,
  type HostedConductorReceipt,
} from './steward-conductor.js';
import { canonicalChaperoneReceipt, writeCustodyJson } from './steward-credential.js';

import type { Payload } from '../../../agentic/compute/payloads.mjs';

/** Roles on the hosted app and the one zome each is called through. */
export const CONTENT_ROLE = 'lamad';
export const CONTENT_ZOME = 'content_store';
export const GOVERNANCE_ROLE = 'mishpat';
export const GOVERNANCE_ZOME = 'mishpat';

/** The receipt file the hosted conductor connection reads, in the session's private dir. */
const RECEIPT_FILE = 'receipt.json';
/** Refresh the hosted app token this long before it expires. */
const REFRESH_MARGIN_MS = 60_000;
/** The chaperone's app token lifetime (doorway chaperone.rs issues 3600 s). */
const APP_TOKEN_MS = 3_600_000;

/** A refusal that names its cure. Its message is the preflight idiom:
 * `REFUSED <name>: <why> — <cure>`; it never carries a secret. */
export class RunnerRefusal extends Error {
  constructor(
    readonly refusal: string,
    readonly why: string,
    readonly cure: string
  ) {
    super(`REFUSED ${refusal}: ${why} — ${cure}`);
  }
}

export interface RunnerEnv {
  doorway: string;
  jwt: string;
  adminKey: string;
  /** Override for the hosted app id; unset ⇒ the one the doorway's /hc/connect receipt names
   * (a hosted cell's app is installed per agent, never under the shared `elohim` id). */
  appId?: string;
  executor?: string;
  root: string;
}

/** Read the runner's configuration from the environment. Pure over `env`. */
export function readRunnerEnv(env: NodeJS.ProcessEnv = process.env): RunnerEnv {
  const doorway = env.DOORWAY_URL?.trim();
  if (!doorway)
    throw new RunnerRefusal(
      'doorway',
      'DOORWAY_URL is unset',
      'export DOORWAY_URL=<the doorway that hosts the runner>'
    );
  let origin: URL;
  try {
    origin = new URL(doorway);
  } catch {
    throw new RunnerRefusal('doorway', 'DOORWAY_URL is not a URL', 'export DOORWAY_URL=https://…');
  }
  if (!['http:', 'https:'].includes(origin.protocol) || origin.username || origin.password)
    throw new RunnerRefusal(
      'doorway',
      'DOORWAY_URL must be a plain http(s) origin',
      'export DOORWAY_URL without credentials or a non-http scheme'
    );
  const jwt = env.RUNNER_JWT?.trim();
  if (!jwt)
    throw new RunnerRefusal(
      'session',
      'RUNNER_JWT is unset',
      'export RUNNER_JWT="$(genesis/scripts/ci/runner-ensure.sh)"'
    );
  const adminKey = env.ADMIN_KEY ?? '';
  if (!adminKey)
    throw new RunnerRefusal(
      'credentials',
      'ADMIN_KEY is unset, so the runner signing key cannot be derived',
      'export ADMIN_KEY from the doorway-admin-bootstrap-key credential'
    );
  return {
    doorway: origin.origin,
    jwt,
    adminKey,
    appId: nonEmpty(env.RUNNER_APP_ID),
    executor: nonEmpty(env.COMPUTE_EXECUTOR),
    root: nonEmpty(env.RUNNER_ROOT) ?? 'genesis/a2o/reports/compute-runner',
  };
}

/** A trimmed value, or undefined when absent or blank. Pure. */
function nonEmpty(value: string | undefined): string | undefined {
  const trimmed = value?.trim();
  if (!trimmed) return undefined;
  return trimmed;
}

export interface RunnerCredentials {
  /** 32-byte Ed25519 seed. */
  seed: Uint8Array;
  publicKey: Uint8Array;
  /** 39-byte Holochain agent key of the signing key (`uhCAk…` when encoded). */
  signingAgentKey: Uint8Array;
  /** 64-byte capability secret. */
  capSecret: Uint8Array;
}

/**
 * The runner's zome-call signing key and cap secret, derived deterministically from the
 * doorway admin bootstrap key with their own domain salts — the same dev-stakes posture as
 * the runner password (runner-ensure.sh, salt `:ci-runner`). Stable across builds, so the
 * doorway authors ONE capability grant per runner, not one per build: a grant is a
 * permanent chain action and nothing revokes it. Pure.
 */
export function deriveRunnerCredentials(adminKey: string): RunnerCredentials {
  if (!adminKey) throw new RunnerRefusal('credentials', 'empty ADMIN_KEY', 'export ADMIN_KEY');
  const seed = new Uint8Array(
    createHash('sha256').update(`${adminKey}:ci-runner-signing`).digest()
  );
  const capSecret = new Uint8Array(
    createHash('sha512').update(`${adminKey}:ci-runner-cap-secret`).digest()
  );
  // RFC 8410 PKCS#8 wrapper for the Ed25519 seed (as steward-conductor's loader does).
  const privateKey = createPrivateKey({
    key: Buffer.concat([Buffer.from('302e020100300506032b657004220420', 'hex'), seed]),
    format: 'der',
    type: 'pkcs8',
  });
  const publicKey = new Uint8Array(
    createPublicKey(privateKey).export({ format: 'der', type: 'spki' }).subarray(-32)
  );
  const signingAgentKey = hashFrom32AndType(publicKey, HoloHashType.Agent);
  return { seed, publicKey, signingAgentKey, capSecret };
}

/** The JWT's `exp` (ms) and `agent_pub_key`, read WITHOUT verifying — the doorway verifies;
 * the runner only plans its refreshes and cross-checks the key. Pure. */
export function jwtClaims(jwt: string): { expiresAtMs?: number; agentPubKey?: string } {
  try {
    const body = JSON.parse(Buffer.from(jwt.split('.')[1] ?? '', 'base64url').toString('utf8'));
    return {
      expiresAtMs: typeof body.exp === 'number' ? body.exp * 1000 : undefined,
      agentPubKey: typeof body.agent_pub_key === 'string' ? body.agent_pub_key : undefined,
    };
  } catch {
    return {};
  }
}

// =============================================================================
// Payload custody through the doorway
// =============================================================================

/** The headers one payload call carries. Pure. */
export function payloadHeaders(input: {
  jwt: string;
  owner: string;
  retention: number;
  request?: string;
  /** The reach the runner declares for the bytes; absent ⇒ its hosting grant's own. */
  reach?: string;
}): Record<string, string> {
  const headers: Record<string, string> = {
    authorization: `Bearer ${input.jwt}`,
    'content-type': 'application/octet-stream',
    'x-compute-owner': input.owner,
    'x-compute-retain-seconds': String(input.retention),
  };
  if (input.request) headers['x-compute-request'] = input.request;
  if (input.reach) headers['x-compute-reach'] = input.reach;
  return headers;
}

/** Map a refused payload answer to a refusal that names its cure. Pure. */
export function payloadRefusal(status: number, body: string): RunnerRefusal {
  let reason = '';
  try {
    reason = (JSON.parse(body) as { reason?: string }).reason ?? '';
  } catch {
    reason = body.slice(0, 120);
  }
  const detail = reason ? ` (${reason})` : '';
  const why = `payload ${status}${detail}`;
  if (status === 401)
    return new RunnerRefusal(
      'payload',
      why,
      're-run genesis/scripts/ci/runner-ensure.sh for a fresh session'
    );
  if (reason === 'local-compute-capability-required')
    return new RunnerRefusal(
      'payload',
      `${why}: no verified performer reached the sponsor`,
      'use a doorway that carries the session agent key on /api/v1/compute/payloads'
    );
  if (reason.startsWith('no-active-hosted-cell-grant') || reason.startsWith('hosted-cell-grant'))
    return new RunnerRefusal(
      'payload',
      `${why}: the sponsor's hosting promise is missing, unverified or withdrawn`,
      'ask the sponsor to re-issue the runner hosted-cell grant (re-register via runner-ensure.sh)'
    );
  if (reason === 'reach-narrower-than-hosting-grant')
    return new RunnerRefusal(
      'payload',
      `${why}: a runner's bytes travel at the reach its hosting grant names`,
      'declare that reach, or none; anything more private needs a person to be private to'
    );
  if (reason === 'rate-limit-exceeded')
    return new RunnerRefusal(
      'payload',
      `${why}: the hosting promise's rate_per_hour is spent`,
      'wait for the hour window to slide, or ask the sponsor for a wider rate'
    );
  if (reason.startsWith('not-a') || reason === 'task-unreadable')
    return new RunnerRefusal(
      'payload',
      why,
      "read only your own task's outputs, naming its request action hash"
    );
  if (status === 404)
    return new RunnerRefusal(
      'payload',
      why,
      'deploy storage with hosted custody (manifest declares /api/v1/compute/payloads) and ELOHIM_COMPUTE_LOCAL_API=1'
    );
  if (status === 503)
    return new RunnerRefusal(
      'payload',
      why,
      'retry; the provider must be reachable from the sponsor peer over the native blob plane'
    );
  return new RunnerRefusal('payload', why, 'inspect the doorway and sponsor storage logs');
}

/** A `payloads.mjs`-compatible transport over the doorway. `request` (the task's request
 * action hash) is sent on reads so the sponsor can admit the runner's own task outputs. */
export function doorwayPayload(input: {
  doorway: string;
  jwt: string;
  request?: string;
  reach?: string;
  fetcher?: typeof fetch;
}): Payload {
  const fetcher = input.fetcher ?? fetch;
  return async (cid, method = 'GET', body, retention = 86400, owner) => {
    if (!owner) throw new Error('Payload lease owner task CID required');
    const response = await fetcher(
      new URL(`/api/v1/compute/payloads/${encodeURIComponent(cid)}`, input.doorway),
      {
        method,
        body,
        redirect: 'error',
        signal: AbortSignal.timeout(300_000),
        headers: payloadHeaders({
          jwt: input.jwt,
          owner,
          retention,
          request: method === 'GET' ? input.request : undefined,
          reach: input.reach,
        }),
      }
    );
    if (!response.ok) throw payloadRefusal(response.status, await response.text());
    if (method !== 'GET') return undefined;
    return Buffer.from(await response.arrayBuffer());
  };
}

// =============================================================================
// The hosted session: /hc/connect, refresh, signed calls
// =============================================================================

/** Ask the doorway to connect this runner's signing key to its own hosted cells. */
export async function hostedConnect(input: {
  doorway: string;
  jwt: string;
  credentials: RunnerCredentials;
  reuseCredentials: boolean;
  fetcher?: typeof fetch;
  now?: () => number;
}): Promise<HostedConductorReceipt> {
  const fetcher = input.fetcher ?? fetch;
  const now = input.now ?? Date.now;
  const response = await fetcher(new URL('/hc/connect', input.doorway), {
    method: 'POST',
    redirect: 'error',
    headers: { authorization: `Bearer ${input.jwt}`, 'content-type': 'application/json' },
    body: JSON.stringify({
      signingKey: Buffer.from(input.credentials.signingAgentKey).toString('base64'),
      capSecret: Buffer.from(input.credentials.capSecret).toString('base64'),
      reuseCredentials: input.reuseCredentials,
    }),
    signal: AbortSignal.timeout(120_000),
  });
  if (response.status === 401)
    throw new RunnerRefusal(
      'session',
      'the doorway refused the runner session (401)',
      're-run genesis/scripts/ci/runner-ensure.sh for a fresh JWT'
    );
  if (response.status === 409)
    throw new RunnerRefusal(
      'session',
      'the runner is not hosted by this doorway (409)',
      'register the runner on THIS doorway with genesis/scripts/ci/runner-ensure.sh'
    );
  if (!response.ok)
    throw new RunnerRefusal(
      'session',
      `/hc/connect answered ${response.status}`,
      'check the doorway conductor pool; the runner cell must be installed'
    );
  const answer = canonicalChaperoneReceipt(
    (await response.json()) as Omit<HostedConductorReceipt, 'doorway' | 'expiresAt'>
  );
  const jwtExpiry = jwtClaims(answer.token || input.jwt).expiresAtMs ?? Infinity;
  return {
    ...answer,
    doorway: input.doorway,
    expiresAt: Math.min(jwtExpiry, now() + APP_TOKEN_MS),
  };
}

/** Lowercase hex of raw bytes (the stored signing-profile encoding). Pure. */
function hex(bytes: Uint8Array): string {
  return Buffer.from(bytes).toString('hex');
}

/** Does this receipt need a refresh before the next call? Pure. */
export function receiptNeedsRefresh(receipt: HostedConductorReceipt | undefined, now: number) {
  return !receipt || now >= receipt.expiresAt - REFRESH_MARGIN_MS;
}

export interface RunnerSessionOptions {
  env: RunnerEnv;
  fetcher?: typeof fetch;
  connect?: (o: ConductorOptions) => Promise<Conductor>;
  now?: () => number;
}

/**
 * One runner session over its hosted cells. Refreshes the app token before it lapses
 * (re-asking `/hc/connect` with `reuseCredentials: true` — transport only, no new grant)
 * and authors the function-listed grant only when the runner's existing credentials are
 * refused: the first call is a probe; if it fails, ONE connect with `reuseCredentials:
 * false` lets the doorway grant, and the probe runs again.
 */
export class RunnerSession {
  readonly credentials: RunnerCredentials;
  private readonly fetcher: typeof fetch;
  private readonly connect: (o: ConductorOptions) => Promise<Conductor>;
  private readonly now: () => number;
  private receipt?: HostedConductorReceipt;
  private readonly conductors = new Map<string, Conductor>();
  private dir?: string;
  private granted = false;

  constructor(private readonly options: RunnerSessionOptions) {
    this.credentials = deriveRunnerCredentials(options.env.adminKey);
    this.fetcher = options.fetcher ?? fetch;
    this.connect = options.connect ?? connectConductor;
    this.now = options.now ?? Date.now;
    const claims = jwtClaims(options.env.jwt);
    if (claims.expiresAtMs !== undefined && claims.expiresAtMs <= this.now())
      throw new RunnerRefusal(
        'session',
        'RUNNER_JWT has expired',
        're-run genesis/scripts/ci/runner-ensure.sh for a fresh JWT'
      );
  }

  /** The runner's own agent key (its hosted cell), once connected. */
  get agent(): string | undefined {
    return this.receipt?.agentPubKey;
  }

  private privateDir(): string {
    this.dir ??= mkdtempSync(join(tmpdir(), 'compute-runner-'));
    return this.dir;
  }

  private async renew(reuseCredentials: boolean): Promise<void> {
    await this.closeConductors();
    const env = this.options.env;
    const receipt = await hostedConnect({
      doorway: env.doorway,
      jwt: env.jwt,
      credentials: this.credentials,
      reuseCredentials,
      fetcher: this.fetcher,
      now: this.now,
    });
    const expected = jwtClaims(env.jwt).agentPubKey;
    if (expected && receipt.agentPubKey !== expected)
      throw new RunnerRefusal(
        'session',
        'the doorway connected a different agent than the session names',
        'register the runner again with genesis/scripts/ci/runner-ensure.sh'
      );
    const dir = this.privateDir();
    writeCustodyJson(join(dir, RECEIPT_FILE), receipt);
    for (const pair of Object.values(receipt.cellIds)) {
      const name = pair.map(h => hex(decodeHashFromBase64(h))).join('-');
      writeCustodyJson(join(dir, `${name}.json`), {
        keypair: hex(this.credentials.seed),
        signingAgentKey: hex(this.credentials.signingAgentKey),
        capSecret: hex(this.credentials.capSecret),
      });
    }
    this.receipt = receipt;
  }

  private async conductor(role: string, zome: string): Promise<Conductor> {
    if (receiptNeedsRefresh(this.receipt, this.now())) await this.renew(true);
    const key = `${role}/${zome}`;
    const open = this.conductors.get(key);
    if (open) return open;
    const dir = this.privateDir();
    const conductor = await this.connect({
      hosted: {
        doorway: this.options.env.doorway,
        receipt: join(dir, RECEIPT_FILE),
        authFile: join(dir, RECEIPT_FILE),
        signingFile: '',
      },
      // Unused on the hosted path (connectHosted reaches the app interface through the
      // doorway's /hc/app/{port}); present only because the options type requires them.
      adminWs: 'ws://unused.invalid',
      appWs: 'ws://unused.invalid',
      appId: this.options.env.appId ?? this.receipt!.installedAppId,
      role,
      zome,
      signingCredentialsDir: dir,
      expectedAgent: this.receipt!.agentPubKey,
    });
    this.conductors.set(key, conductor);
    return conductor;
  }

  /** A read every runner grant admits and that changes nothing: list the tasks naming
   * this runner as PROVIDER (none). It proves the listed capability exists. */
  private async probe(): Promise<void> {
    const conductor = await this.conductor(CONTENT_ROLE, CONTENT_ZOME);
    await conductor.call('compute_task', {
      operation: 'list',
      provider: this.receipt!.agentPubKey,
      offset: 0,
      limit: 1,
    });
  }

  /** Make sure the runner's credentials are granted on its cells — authoring the grant
   * at most once, and only when the existing credentials are refused. A real operation's
   * own failure never triggers a grant. */
  private async ensureGranted(): Promise<void> {
    if (this.granted) return;
    try {
      await this.probe();
    } catch {
      await this.renew(false);
      try {
        await this.probe();
      } catch {
        throw new RunnerRefusal(
          'session',
          "the runner's own cell refused its capability even after a fresh /hc/connect",
          'use a doorway that grants service-agent sessions content_store::compute_task'
        );
      }
    }
    this.granted = true;
  }

  /** Connect (granting at most once) and return the runner's own agent key. */
  async whoami(): Promise<string> {
    await this.ensureGranted();
    return this.receipt!.agentPubKey;
  }

  /** Call `fnName` on `role`'s cell as the runner. */
  async call<T>(role: string, zome: string, fnName: string, payload: unknown): Promise<T> {
    await this.ensureGranted();
    return (await this.conductor(role, zome)).call<T>(fnName, payload);
  }

  private async closeConductors(): Promise<void> {
    for (const conductor of this.conductors.values()) await conductor.close().catch(() => {});
    this.conductors.clear();
  }

  async close(): Promise<void> {
    await this.closeConductors();
    if (this.dir) rmSync(this.dir, { recursive: true, force: true });
    this.dir = undefined;
  }
}

// =============================================================================
// Task operations
// =============================================================================

/** Refuse an envelope this runner cannot submit as itself. Pure. */
export function checkEnvelope(
  envelope: Record<string, unknown>,
  runnerAgent: string,
  provider?: string
): void {
  if (envelope.requester !== runnerAgent)
    throw new RunnerRefusal(
      'submit',
      'the task names a different requester than this runner',
      `build the stage with --requester ${runnerAgent}`
    );
  if (provider && envelope.provider !== provider)
    throw new RunnerRefusal(
      'submit',
      'the task names a different provider than --provider',
      'build the stage for that provider, or drop --provider'
    );
}

/** Is a task terminal? Pure. */
export function isTerminal(task: Record<string, unknown>): boolean {
  return Boolean(task.completion) || Boolean(task.refusal);
}

// =============================================================================
// Grant status
// =============================================================================

export interface StateLink {
  author: string;
  state: string;
  signed_at: string;
}

export interface GrantStatus {
  grantActionHash: string;
  cid?: string;
  state:
    | 'active'
    | 'withdrawn'
    | 'lapsed'
    | 'not-yet-valid'
    | 'inactive'
    | 'unverified'
    | 'unknown';
  provider?: string;
  recipient?: string;
  scope?: string;
  validUntil?: string;
  daysRemaining?: number;
}

/** Decide a grant's standing from its signed policy, its author and its lifecycle links —
 * the same reading compute launch makes (`grant_links_allow`), plus the window. Pure. */
export function grantStatus(input: {
  grantActionHash: string;
  cid: string;
  author: string;
  policy: Record<string, unknown>;
  links: StateLink[];
  now: number;
}): GrantStatus {
  const { policy, links } = input;
  const provider = typeof policy.provider === 'string' ? policy.provider : undefined;
  const validUntil = typeof policy.valid_until === 'string' ? policy.valid_until : undefined;
  const until = validUntil ? Date.parse(validUntil) : Number.NaN;
  const from = typeof policy.valid_from === 'string' ? Date.parse(policy.valid_from) : Number.NaN;
  const base = {
    grantActionHash: input.grantActionHash,
    cid: input.cid,
    provider,
    recipient: typeof policy.recipient === 'string' ? policy.recipient : undefined,
    scope: typeof policy.scope === 'string' ? policy.scope : undefined,
    validUntil,
    daysRemaining: Number.isFinite(until)
      ? Math.floor((until - input.now) / 86_400_000)
      : undefined,
  };
  if (!provider || input.author !== provider) return { ...base, state: 'unverified' };
  const authored = links.filter(link => link.author === provider);
  if (authored.some(link => ['revoked', 'cancelled', 'sunset'].includes(link.state)))
    return { ...base, state: 'withdrawn' };
  if (!authored.some(link => link.state === 'active')) return { ...base, state: 'inactive' };
  if (!Number.isFinite(until) || input.now > until) return { ...base, state: 'lapsed' };
  if (Number.isFinite(from) && input.now < from) return { ...base, state: 'not-yet-valid' };
  return { ...base, state: 'active' };
}

interface NativeRecord {
  signed_action: { hashed: { content: { author?: Uint8Array; entry_hash?: Uint8Array } } };
  entry: { Present?: { entry_type: string; entry: Uint8Array } };
}

/** Read a grant through the runner's own governance cell and decide its standing. */
export async function readGrantStatus(
  session: Pick<RunnerSession, 'call'>,
  grantActionHash: string,
  now = Date.now()
): Promise<GrantStatus> {
  const record = await session.call<NativeRecord | null>(
    GOVERNANCE_ROLE,
    GOVERNANCE_ZOME,
    'get_commitment_record',
    decodeHashFromBase64(grantActionHash)
  );
  const action = record?.signed_action?.hashed?.content;
  const bytes = record?.entry?.Present?.entry;
  if (!record || !action?.author || !action.entry_hash || !bytes)
    return { grantActionHash, state: 'unknown' };
  const entry = decode(bytes) as { action?: string; payload_json?: string };
  const policy = JSON.parse(entry.payload_json ?? '{}') as Record<string, unknown>;
  const cid = encodeHashToBase64(action.entry_hash);
  const links = await session.call<StateLink[]>(
    GOVERNANCE_ROLE,
    GOVERNANCE_ZOME,
    'get_commitment_authority_links',
    cid
  );
  return grantStatus({
    grantActionHash,
    cid,
    author: encodeHashToBase64(action.author),
    policy: { ...policy, action: entry.action },
    links,
    now,
  });
}
