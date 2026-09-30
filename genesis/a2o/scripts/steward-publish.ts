/**
 * Steward publish — a steward publishes commons learning content from their OWN peer.
 *
 * No pipeline, no doorway seed. For each item the steward's storage PATCH re-notarizes
 * the row through the steward's conductor, and the steward's agent then signs that
 * version as the EARNED canonical head (`declare_earned_canonical_head`), which other
 * peers adopt over their own per-root election. This generalizes the one-item precedent
 * scripts/manifesto-native-declare.ts into a batch.
 *
 * Acceptance story: genesis/a2o/features/lms/commons-path-steward-publish.feature
 * (scenario 1 — the update reaches another peer; scenario 4 — intimate content refused).
 *
 * Usage (from genesis/a2o):
 *   pnpm exec tsx scripts/steward-publish.ts <id> [<id> …] [--manifest ids.txt]
 *       [--dry-run] [--await-peer http://localhost:8091 …] [--await-timeout 75]
 *       [--storage URL] [--admin-ws URL] [--app-ws URL] [--app-id elohim] [--role lamad]
 *       [--data-dir DIR] [--signing-credentials-dir DIR] [--receipt-dir DIR]
 *       --device-agent KEY --dna-hash HASH --binding ACTION_HASH
 *       [--delegations FILE] [--grantor-connections FILE] [--redeclare] [--closure]
 *
 *   ids        resolved to <data-dir>/content/<id>.json or <data-dir>/paths/<id>.json
 *   --manifest a file listing ids (one per line, or a JSON array; `#` comments allowed)
 *   --dry-run  read and plan only; no blob, row, chain or conductor writes
 *   --await-peer  poll every receiver concurrently under one shared deadline per
 *              publication for the seed hash, body and exact head (repeatable)
 *   --binding  verified additive device enrollment; never an administration grant
 *   --delegations  JSON map: content id -> root-author signed, root/DNA-scoped grant
 *   --grantor-connections  JSON map: root author -> existing conductor credentials
 *   --native-receivers     JSON map: receiver name -> existing conductor credentials;
 *                          observes native election/history within the same deadline
 *              used to acknowledge the exact version; never mints a new grant
 *   --receipt-dir  required for writes: durable recovery records outside build output;
 *              a pending version
 *              is declared on resumption without authoring another update
 *   --closure  also publish every item a named path's walk points at (its steps and
 *              concept ids), so a path never names an item no peer holds. Each of them
 *              still passes the commons fence.
 *   --redeclare   re-sign the earned declaration even when the row already reads as
 *              current. `unchanged` requires the same declared and earned head;
 *              interrupted writes resume their exact authored version automatically.
 *
 * Environment defaults (the household mesh, matthew's peer):
 *   STEWARD_STORAGE_URL  http://localhost:8090
 *   STEWARD_ADMIN_WS     ws://localhost:4444     (conductor admin interface)
 *   STEWARD_APP_WS       ws://localhost:4445     (conductor app interface)
 *   STEWARD_APP_ID       elohim                  (installed app the storage bridge uses)
 *   STEWARD_ROLE         lamad                   (role whose cell holds the content_store zome)
 *   STEWARD_SIGNING_CREDENTIALS_DIR  required for writes; existing exact-cell credentials
 *   DATA_DIR             genesis/data/lamad      (repo-relative)
 *
 * Output: one line per item — `<id>  reach=<reach>  action=<action>  head=<hash>` where
 * action is published | declared | unchanged | refused | blocked | pending | failed. Exit 0 when
 * every item is published/declared/unchanged, 2 when the commons fence refused anything
 * (nothing is written in that case), 1 on any other problem.
 */
import { createHash } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, extname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { decodeHashFromBase64, encodeHashToBase64 } from '@holochain/client';

import { buildContentInput, buildPathInput } from '../../seeder/src/content-input.js';

import {
  connectConductor,
  type Conductor,
  type ConductorOptions,
} from './lib/steward-conductor.js';
import { delegationWire, type HeadDelegationDocument } from './lib/steward-delegation.js';
import {
  expandClosure,
  loadRepoItem,
  publicationOrder,
  pathReferences,
  type RepoItem,
} from './lib/steward-items.js';
import { awaitNativeReceiver } from './lib/steward-native-proof.js';
import {
  pendingPublication,
  savePublication,
  type PublicationReceipt,
} from './lib/steward-publication-receipt.js';
import {
  authoredReach,
  commonsFenceRefusal,
  landedMismatches,
  planItem,
  publishPatch,
  type ExistingRow,
  type ItemPlan,
} from './lib/steward-publish-plan.js';

import type { CreateContentInput } from '../../seeder/src/generated/create-content-input.js';

const HERE = dirname(fileURLToPath(import.meta.url));
const GENESIS_DIR = resolve(HERE, '..', '..');

// ─── options ────────────────────────────────────────────────────────────────

interface Options {
  ids: string[];
  dataDir: string;
  storage: string;
  adminWs: string;
  appWs: string;
  appId: string;
  role: string;
  signingCredentialsDir?: string;
  expectedAgent?: string;
  expectedDna?: string;
  receiptDir: string;
  binding?: string;
  delegationsFile?: string;
  grantorsFile?: string;
  nativeReceiversFile?: string;
  dryRun: boolean;
  redeclare: boolean;
  closure: boolean;
  awaitPeers: string[];
  awaitTimeoutS: number;
}

function usage(msg?: string): never {
  if (msg) console.error(`steward-publish: ${msg}`);
  console.error(
    'usage: steward-publish.ts <id>… [--manifest FILE] [--dry-run] [--redeclare] [--closure] ' +
      '[--await-peer URL]… [--await-timeout S] [--storage URL] [--admin-ws URL] ' +
      '[--app-ws URL] [--app-id ID] [--role ROLE] [--data-dir DIR] [--signing-credentials-dir DIR] [--device-agent KEY] [--dna-hash HASH] [--receipt-dir DIR] [--binding ACTION] [--delegations FILE] [--grantor-connections FILE] [--native-receivers FILE]'
  );
  process.exit(2);
}

function readManifest(file: string): string[] {
  const raw = readFileSync(file, 'utf8');
  const trimmed = raw.trim();
  if (trimmed.startsWith('[')) {
    const parsed: unknown = JSON.parse(trimmed);
    if (!Array.isArray(parsed) || !parsed.every(x => typeof x === 'string')) {
      usage(`${file}: a JSON manifest must be an array of id strings`);
    }
    return parsed;
  }
  return raw
    .split('\n')
    .map(l => l.replace(/#.*/, '').trim())
    .filter(Boolean);
}

function parseArgs(argv: string[]): Options {
  const env = process.env;
  const o: Options = {
    ids: [],
    dataDir: env.DATA_DIR ?? join(GENESIS_DIR, 'data', 'lamad'),
    storage: env.STEWARD_STORAGE_URL ?? 'http://localhost:8090',
    adminWs: env.STEWARD_ADMIN_WS ?? 'ws://localhost:4444',
    appWs: env.STEWARD_APP_WS ?? 'ws://localhost:4445',
    appId: env.STEWARD_APP_ID ?? 'elohim',
    role: env.STEWARD_ROLE ?? 'lamad',
    receiptDir: env.STEWARD_RECEIPT_DIR ?? '',
    dryRun: false,
    redeclare: false,
    closure: false,
    awaitPeers: [],
    awaitTimeoutS: 75,
  };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    const val = (): string => {
      if (i + 1 >= argv.length) usage(`${a} needs a value`);
      return argv[++i];
    };
    switch (a) {
      case '--manifest':
        o.ids.push(...readManifest(val()));
        break;
      case '--data-dir':
        o.dataDir = resolve(val());
        break;
      case '--storage':
        o.storage = val();
        break;
      case '--admin-ws':
        o.adminWs = val();
        break;
      case '--app-ws':
        o.appWs = val();
        break;
      case '--app-id':
        o.appId = val();
        break;
      case '--role':
        o.role = val();
        break;
      case '--signing-credentials-dir':
        o.signingCredentialsDir = resolve(val());
        break;
      case '--device-agent':
        o.expectedAgent = val();
        break;
      case '--dna-hash':
        o.expectedDna = val();
        break;
      case '--binding':
        o.binding = val();
        break;
      case '--delegations':
        o.delegationsFile = resolve(val());
        break;
      case '--grantor-connections':
        o.grantorsFile = resolve(val());
        break;
      case '--native-receivers':
        o.nativeReceiversFile = resolve(val());
        break;
      case '--receipt-dir':
        o.receiptDir = resolve(val());
        break;
      case '--dry-run':
        o.dryRun = true;
        break;
      case '--redeclare':
        o.redeclare = true;
        break;
      case '--closure':
        o.closure = true;
        break;
      case '--await-peer':
        o.awaitPeers.push(val().replace(/\/$/, ''));
        break;
      case '--await-timeout':
        o.awaitTimeoutS = Number(val());
        break;
      case '-h':
      case '--help':
        return usage();
      default:
        if (a.startsWith('-')) usage(`unknown flag ${a}`);
        o.ids.push(a);
    }
  }
  o.storage = o.storage.replace(/\/$/, '');
  o.ids = [...new Set(o.ids)];
  if (o.ids.length === 0) usage('no ids given');
  return o;
}

// ─── blob sources ───────────────────────────────────────────────────────────
//
// The blob a row links (an html5-app zip, a path thumbnail) is part of the row's
// authored fields, so it must resolve exactly as the pipeline seeder resolves it.
// These two mirror `findHtml5AppBlob` / `findThumbnailBlob` in
// genesis/seeder/src/seed-sqlite.ts, which are not exported; they should move beside
// the builders in content-input.ts (or a sibling) so both writers import one copy.

interface BlobSource {
  hash: string;
  data?: Buffer;
  mimeType: string;
}

function sha256Address(data: Buffer): string {
  return `sha256-${createHash('sha256').update(data).digest('hex')}`;
}

/** Legacy `sha256-` prefix only on bare hex; a CID passes through untouched. */
function normalizeBlobHash(h: string): string {
  return /^[0-9a-fA-F]{64}$/.test(h) ? `sha256-${h}` : h;
}

function html5AppBlob(item: RepoItem, dataDir: string): BlobSource | undefined {
  const j = item.json;
  const format = (j.contentFormat ?? '').toLowerCase();
  if (format !== 'html5-app') return undefined;
  // seed-sqlite reads only `blobHash` when a local zip exists, and `blobHash || blob_hash`
  // when none does — mirrored as-is so the linked hash cannot differ between writers.
  const zipHash = j.blobHash ? normalizeBlobHash(j.blobHash) : undefined;
  const refHash =
    (j.blobHash ?? j.blob_hash) ? normalizeBlobHash(j.blobHash ?? j.blob_hash ?? '') : undefined;
  const localZip = j.metadata?.localZipPath;
  const candidates = [
    typeof localZip === 'string' ? join(GENESIS_DIR, localZip) : undefined,
    join(dataDir, 'content', `${item.id}.zip`),
  ].filter((p): p is string => Boolean(p));
  for (const zip of candidates) {
    if (existsSync(zip)) {
      const data = readFileSync(zip);
      return { hash: zipHash ?? sha256Address(data), data, mimeType: 'application/zip' };
    }
  }
  return refHash ? { hash: refHash, mimeType: 'application/zip' } : undefined;
}

const IMAGE_TYPES: Record<string, string> = {
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.gif': 'image/gif',
  '.webp': 'image/webp',
  '.svg': 'image/svg+xml',
};

function thumbnailBlob(item: RepoItem): BlobSource | undefined {
  const url = item.json.thumbnailUrl;
  if (!url) return undefined;
  let file: string | undefined;
  if (url.startsWith('/images/')) file = join(GENESIS_DIR, 'assets', url.slice(1));
  else if (url.startsWith('images/')) file = join(GENESIS_DIR, 'assets', url);
  else if (url.startsWith('assets/')) file = join(GENESIS_DIR, url);
  else if (url.startsWith('/assets/')) file = join(GENESIS_DIR, url.slice(1));
  if (!file || !existsSync(file)) return undefined;
  const data = readFileSync(file);
  return {
    hash: sha256Address(data),
    data,
    mimeType: IMAGE_TYPES[extname(file).toLowerCase()] ?? 'application/octet-stream',
  };
}

// ─── storage HTTP ───────────────────────────────────────────────────────────

async function http(url: string, init?: RequestInit): Promise<{ status: number; text: string }> {
  const r = await fetch(url, { ...init, signal: init?.signal ?? AbortSignal.timeout(180_000) });
  return { status: r.status, text: await r.text() };
}

async function getRow(
  storage: string,
  id: string,
  signal?: AbortSignal
): Promise<ExistingRow | undefined> {
  const r = await http(`${storage}/db/content/${encodeURIComponent(id)}`, { signal });
  if (r.status === 404) return undefined;
  if (r.status !== 200) throw new Error(`GET ${id}: ${r.status} ${r.text.slice(0, 200)}`);
  const body = JSON.parse(r.text) as ExistingRow & { content?: ExistingRow };
  return body.content ?? body;
}

interface HeadView {
  headActionHash: string;
  declared: boolean;
  earned: boolean;
  dhtAnchorHash?: string | null;
}

async function getHead(
  storage: string,
  id: string,
  signal?: AbortSignal
): Promise<HeadView | undefined> {
  const r = await http(`${storage}/db/content/${encodeURIComponent(id)}/head`, { signal });
  return r.status === 200 ? (JSON.parse(r.text) as HeadView) : undefined;
}

async function blobPresent(storage: string, hash: string): Promise<boolean> {
  const r = await fetch(`${storage}/blob/${hash}`, {
    method: 'HEAD',
    signal: AbortSignal.timeout(30_000),
  });
  return r.ok;
}

/** Make the linked blob present on the steward's peer; returns what was done. */
async function ensureBlob(storage: string, blob: BlobSource): Promise<string> {
  if (await blobPresent(storage, blob.hash)) return 'present';
  if (!blob.data) throw new Error(`blob ${blob.hash} is not on this peer and no local bytes exist`);
  const put = await http(`${storage}/blob/${blob.hash}`, {
    method: 'PUT',
    headers: { 'content-type': blob.mimeType },
    body: new Uint8Array(blob.data),
  });
  if (put.status >= 300)
    throw new Error(`blob PUT ${blob.hash}: ${put.status} ${put.text.slice(0, 200)}`);
  return 'uploaded';
}

// ─── conductor ──────────────────────────────────────────────────────────────

/** Sign `head` as the EARNED canonical head of `id` with the steward's agent. */
async function declareEarned(
  c: Conductor,
  id: string,
  head: string,
  delegation: unknown,
  timeoutMs: number
): Promise<{ head: string; author: string; canonical: boolean }> {
  const out = await c.call<{
    head_action_hash: Uint8Array;
    author: Uint8Array;
    canonical: boolean;
    canonical_earned?: boolean;
  }>(
    'declare_earned_canonical_head',
    {
      id,
      head_action_hash: head,
      carried_record: null,
      adopt_before_author: false,
      delegation,
    },
    timeoutMs
  );
  return {
    head: encodeHashToBase64(out.head_action_hash),
    author: encodeHashToBase64(out.author),
    canonical: out.canonical && out.canonical_earned === true,
  };
}

// ─── the run ────────────────────────────────────────────────────────────────

interface Prepared {
  item: RepoItem;
  input: CreateContentInput;
  blob?: BlobSource;
  row?: ExistingRow;
  plan: ItemPlan;
  pending?: PublicationReceipt;
  delegation?: unknown;
  grantor?: ConductorOptions;
}

type Outcome =
  | 'published'
  | 'declared'
  | 'unchanged'
  | 'refused'
  | 'blocked'
  | 'failed'
  | 'pending'
  | 'planned';

function line(id: string, reach: string, action: Outcome, head: string, note = ''): void {
  console.log(
    `${id}  reach=${reach}  action=${action}  head=${head || '-'}` + (note ? `  (${note})` : '')
  );
}

function norm(v: string | null | undefined): string | null {
  return v === undefined || v === null || v === '' ? null : v;
}

async function verifyBinding(o: Options, c: Conductor): Promise<void> {
  const identity = await connectConductor({
    ...o,
    role: 'mishpat',
    zome: 'mishpat',
    expectedDna: undefined,
  });
  try {
    await identity.call('verify_device_binding', {
      binding: decodeHashFromBase64(o.binding!),
      expected_device: decodeHashFromBase64(c.agent),
      expected_content_dna: decodeHashFromBase64(c.dna),
    });
  } finally {
    await identity.close();
  }
}

async function publishOne(
  o: Options,
  p: Prepared,
  conductor: () => Promise<Conductor>,
  deadline: number
): Promise<string> {
  const { input, plan } = p;
  const id = input.id;
  const c = await conductor();
  await verifyBinding(o, c);
  let head = p.pending?.head ?? p.row?.dhtAnchorHash ?? '';
  if (!p.pending && (plan.action === 'create' || plan.action === 'update')) {
    if (p.blob) await ensureBlob(o.storage, p.blob);
    if (plan.action === 'create') {
      // Insert WITHOUT an anchor: the PATCH below then bootstraps the entry through
      // the conductor's create_content, which carries the body.
      const bulk = await http(`${o.storage}/db/content/bulk`, {
        method: 'POST',
        headers: { 'content-type': 'application/json', 'x-schema-version': '1' },
        body: JSON.stringify([{ ...input, dhtAnchorHash: undefined }]),
      });
      if (bulk.status >= 300)
        throw new Error(`bulk insert: ${bulk.status} ${bulk.text.slice(0, 200)}`);
    }
    const patch = await http(`${o.storage}/db/content/${encodeURIComponent(id)}`, {
      method: 'PATCH',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(publishPatch(input, plan.seedHash)),
    });
    if (patch.status >= 300) throw new Error(`PATCH: ${patch.status} ${patch.text.slice(0, 300)}`);
    const landed = await getRow(o.storage, id);
    if (!landed) throw new Error('row vanished after PATCH');
    const off = landedMismatches(input, plan.seedHash, landed);
    if (off.length > 0) {
      throw new Error(
        `PATCH returned ${patch.status} but the row does not hold: ${off.join(', ')} — not declaring`
      );
    }
    head = landed.dhtAnchorHash ?? '';
  }
  if (!head) throw new Error('no dhtAnchorHash to declare');
  const receipt: PublicationReceipt = p.pending ?? {
    id,
    seedHash: plan.seedHash,
    agent: c.agent,
    dna: c.dna,
    storage: o.storage,
    head,
    authoredAt: new Date().toISOString(),
  };
  savePublication(o.receiptDir, receipt);
  if (p.delegation) {
    if (!p.grantor) throw new Error('root-author acceptance connection missing');
    const author = await connectConductor(p.grantor);
    try {
      for (;;) {
        const remaining = deadline - Date.now();
        if (remaining <= 0) throw new Error('publication pending: acceptance deadline elapsed');
        try {
          p.delegation = await author.call(
            'accept_delegated_head',
            {
              id,
              head_action_hash: decodeHashFromBase64(head),
              delegation: p.delegation,
            },
            remaining
          );
          break;
        } catch (error) {
          // Missing integrated history can arrive later. Authorization failures
          // are definitive; never retry them into a different grant or version.
          if (!/not retrievable.*PENDING|accepted version unavailable/is.test(String(error)))
            throw error;
          if (Date.now() >= deadline) throw error;
          await new Promise(resolve => setTimeout(resolve, Math.min(1000, deadline - Date.now())));
        }
      }
    } finally {
      await author.close();
    }
  }
  // The steward's own storage keeps authoring on the same source chain (its backfill
  // and witness sweeps re-author freshly inserted rows), so a declaration can lose
  // the race for the chain head. Retry the exact authored action; a concurrent
  // writer changing the projected row must never change what this run declares.
  for (let attempt = 1; ; attempt++) {
    try {
      const remaining = deadline - Date.now();
      if (remaining <= 0) throw new Error('publication pending: declaration deadline elapsed');
      const out = await declareEarned(c, id, head, p.delegation ?? null, remaining);
      if (out.head !== head || !out.canonical) {
        throw new Error(
          `publication pending: earned election has not accepted ${head} (elected ${out.head})`
        );
      }
      savePublication(o.receiptDir, { ...receipt, declaredAt: new Date().toISOString() });
      return head;
    } catch (e) {
      const raced = /source chain head has moved/i.test(String(e));
      if (!raced || attempt >= 4) throw e;
      await new Promise(r => setTimeout(r, 1500 * attempt));
      // Retry only this authored version. A source-chain race is not permission
      // to declare a different version that a concurrent writer has projected.
    }
  }
}

async function awaitPeer(
  o: Options,
  peer: string,
  done: Prepared[],
  heads: Map<string, string>,
  deadline: number
): Promise<boolean> {
  const pending = new Map(done.map(p => [p.input.id, p]));
  const started = Date.now();
  const lastWhy = new Map<string, string>();
  while (pending.size > 0 && Date.now() < deadline) {
    for (const [id, p] of pending) {
      if (Date.now() >= deadline) break;
      let row: ExistingRow | undefined;
      try {
        row = await getRow(peer, id, AbortSignal.timeout(Math.max(1, deadline - Date.now())));
      } catch (e) {
        lastWhy.set(id, String(e).slice(0, 120));
        continue;
      }
      if (!row) {
        lastWhy.set(id, 'no row');
        continue;
      }
      const rowObservedAt = Date.now();
      let electionObservedAt: number | undefined;
      const off = landedMismatches(p.input, p.plan.seedHash, row);
      const head = heads.get(id);
      if (head && row.dhtAnchorHash !== head)
        off.push(`dhtAnchorHash ${row.dhtAnchorHash ?? 'null'} ≠ ${head}`);
      if (off.length === 0 && head) {
        try {
          const elected = await getHead(
            peer,
            id,
            AbortSignal.timeout(Math.max(1, deadline - Date.now()))
          );
          electionObservedAt = Date.now();
          if (!elected?.declared || !elected.earned || elected.headActionHash !== head)
            off.push('receiver has not projected the exact earned election');
        } catch (e) {
          off.push(String(e).slice(0, 120));
        }
      }
      const servedAt = Date.now();
      if (servedAt > deadline) off.push('shared publication deadline elapsed');
      if (off.length === 0) {
        console.log(
          `  ${peer}  ${id}  converged after ${Math.round((servedAt - started) / 1000)}s`
        );
        console.log(
          JSON.stringify({
            checkId: 'federation-deploy',
            outcome: 'passed',
            summary: `${peer}: exact content bytes and earned head served within the shared deadline`,
            observed: {
              id,
              head,
              seedHash: p.plan.seedHash,
              peer,
              rowObservedAt,
              electionObservedAt,
              servedAt,
              deadlineMs: deadline,
            },
          })
        );
        pending.delete(id);
      } else {
        lastWhy.set(id, off.join(', '));
      }
    }
    if (pending.size > 0)
      await new Promise(r => setTimeout(r, Math.min(1000, Math.max(0, deadline - Date.now()))));
  }
  for (const id of pending.keys()) {
    console.log(
      `  ${peer}  ${id}  NOT converged after ${o.awaitTimeoutS}s — ${lastWhy.get(id) ?? '?'}`
    );
  }
  return pending.size === 0;
}

/** All receiver observations consume the publication's single absolute budget.
 * Native observations are evidence inputs; op timing correlation separately
 * establishes that the observed records were validated and integrated locally. */
async function awaitReceivers(
  o: Options,
  p: Prepared,
  heads: Map<string, string>,
  receivers: Map<string, Conductor>,
  author: () => Promise<Conductor>,
  deadline: number
): Promise<boolean> {
  const serving = o.awaitPeers.map(peer => awaitPeer(o, peer, [p], heads, deadline));
  const native = async (): Promise<boolean[]> => {
    if (receivers.size === 0) return [];
    const head = heads.get(p.input.id);
    if (!head) throw new Error('Native receiver proof requires an exact publication head');
    const source = await author();
    const remaining = deadline - Date.now();
    if (remaining <= 0) return [false];
    const lineage = await source.call<{
      root_action_hash: Uint8Array;
      content_id: string;
      truncated: boolean;
    }>(
      'get_content_lineage',
      { action_hash: decodeHashFromBase64(head), local: true },
      Math.min(5000, remaining)
    );
    if (lineage.content_id !== p.input.id || lineage.truncated)
      throw new Error('Native receiver proof requires complete exact source lineage');
    return Promise.all(
      [...receivers].map(async ([name, receiver]) => {
        const proof = await awaitNativeReceiver(
          receiver,
          {
            id: p.input.id,
            head,
            root: encodeHashToBase64(lineage.root_action_hash),
            dna: source.dna,
          },
          deadline
        );
        console.log(
          JSON.stringify({
            checkId: 'federation-deploy',
            outcome: proof.accepted ? 'passed' : 'failed',
            summary: `${name}: receiver-local head, election and ancestry observation; integration timing correlation still required`,
            observed: proof,
          })
        );
        return proof.accepted;
      })
    );
  };
  const [httpResults, nativeResults] = await Promise.all([Promise.all(serving), native()]);
  return [...httpResults, ...nativeResults].every(Boolean);
}

async function main(): Promise<void> {
  const o = parseArgs(process.argv.slice(2));

  // 1. Resolve every id and apply the commons fence to ALL of them before anything
  //    else touches a peer. One refusal refuses the batch.
  const items: RepoItem[] = [];
  let refused = 0;
  // A path's walk, then one hop along every typed edge the named items author, so
  // nothing published points at an item no peer holds. Every added id still passes
  // the commons fence below.
  const ids = o.closure ? expandClosure(o.dataDir, o.ids) : [...new Set(o.ids)];
  for (const id of ids) {
    const { item: loaded, error } = loadRepoItem(o.dataDir, id);
    if (!loaded) {
      line(id, '?', 'refused', '', error);
      refused++;
      continue;
    }
    const why = commonsFenceRefusal(id, loaded.json);
    if (why) {
      line(id, authoredReach(loaded.json) ?? 'none', 'refused', '', why);
      refused++;
      continue;
    }
    items.push(loaded);
  }
  if (refused > 0) {
    console.error(
      `steward-publish: ${refused} item(s) refused by the commons fence — nothing was written`
    );
    process.exit(2);
  }

  // 2. Build each input with the seeder's own builders, resolve its linked blob, read
  //    the steward's row and plan. Reads only.
  const prepared: Prepared[] = [];
  for (const item of publicationOrder(items)) {
    let blob: BlobSource | undefined;
    let input: CreateContentInput;
    if (item.kind === 'path') {
      blob = thumbnailBlob(item);
      input = buildPathInput(item.json, { thumbnailHash: blob?.hash });
    } else {
      blob = html5AppBlob(item, o.dataDir);
      input = buildContentInput(item.json, { blobHash: blob?.hash });
    }
    if (input.reach !== 'commons') {
      line(
        item.id,
        input.reach ?? 'none',
        'refused',
        '',
        `builder resolved reach "${input.reach}"`
      );
      process.exit(2);
    }
    const row = await getRow(o.storage, item.id);
    let earned: string | undefined;
    if (row?.dhtAnchorHash && !o.redeclare) {
      const h = await getHead(o.storage, item.id);
      if (h?.declared && h.earned && h.headActionHash === row.dhtAnchorHash)
        earned = h.headActionHash;
    }
    prepared.push({ item, input, blob, row, plan: planItem(input, row, earned) });
  }

  // 3. Execute. The conductor is connected BEFORE the first write, so a wrong port,
  //    app or role fails with nothing written; pending receipts are checked as well.
  //    Existing credentials are reused; this connection creates no signing grant.
  let conductor: Conductor | undefined;
  const getConductor = async (): Promise<Conductor> => {
    if (!conductor) {
      conductor = await connectConductor(o);
      console.log(
        `steward agent ${conductor.agent} (app ${o.appId}, role ${o.role}, ${o.adminWs})`
      );
    }
    return conductor;
  };
  const receivers = new Map<string, Conductor>();
  if (!o.dryRun && o.nativeReceiversFile) {
    const profiles = JSON.parse(readFileSync(o.nativeReceiversFile, 'utf8')) as Record<
      string,
      ConductorOptions
    >;
    const source = await getConductor();
    for (const [name, profile] of Object.entries(profiles)) {
      if (
        !profile.expectedAgent ||
        profile.expectedDna !== source.dna ||
        !profile.signingCredentialsDir
      )
        throw new Error(
          `Native receiver ${name} requires its explicit agent, matching DNA and existing credentials`
        );
      const receiver = await connectConductor(profile);
      if (
        receiver.agent === source.agent ||
        [...receivers.values()].some(peer => peer.agent === receiver.agent)
      ) {
        await receiver.close();
        throw new Error(`Native receiver ${name} must have an independent signing key`);
      }
      receivers.set(name, receiver);
    }
    if (receivers.size === 0) throw new Error('Native receiver configuration is empty');
  }
  const writes = prepared.some(p => ['create', 'update', 'declare'].includes(p.plan.action));
  // Pending recovery is checked even when storage already projects a declaration.
  // Projection is not a receipt for this exact run's authored action.
  if (!o.dryRun && (writes || existsSync(o.receiptDir))) {
    const c = await getConductor();
    for (const p of prepared) {
      p.pending = pendingPublication(o.receiptDir, {
        id: p.input.id,
        seedHash: p.plan.seedHash,
        agent: c.agent,
        dna: c.dna,
        storage: o.storage,
      });
      if (p.pending) {
        const accepted = await c.call<{
          head_action_hash: Uint8Array;
          canonical: boolean;
          canonical_earned?: boolean;
        } | null>('resolve_content_head_local', p.input.id);
        if (
          accepted?.canonical &&
          accepted.canonical_earned === true &&
          encodeHashToBase64(accepted.head_action_hash) === p.pending.head &&
          p.row?.dhtAnchorHash === p.pending.head &&
          landedMismatches(p.input, p.plan.seedHash, p.row).length === 0
        ) {
          // A lost declaration response is recovered from verified history.
          // This read does not require an expired grant to become live again.
          savePublication(o.receiptDir, { ...p.pending, declaredAt: new Date().toISOString() });
          p.pending = undefined;
          p.plan.action = 'unchanged';
        } else {
          p.plan.action = 'declare';
        }
      }
    }
  }
  if (!o.dryRun && prepared.some(p => ['create', 'update', 'declare'].includes(p.plan.action))) {
    if (!o.expectedAgent || !o.expectedDna || !o.binding || !o.receiptDir)
      throw new Error(
        'writes require --device-agent, --dna-hash, --binding and a durable --receipt-dir; enroll explicitly first'
      );
    if (prepared.some(p => p.plan.action === 'blocked'))
      throw new Error('publication batch contains blocked items; resolve them before writing');
    const c = await getConductor();
    const health = await http(`${o.storage}/health`);
    if (health.status !== 200) throw new Error('publication preflight: storage health unavailable');
    const pairing = JSON.parse(health.text) as {
      conductor?: { zomePath?: string };
      dhtParticipation?: { dnaHashes?: Record<string, string>; agentKeys?: Record<string, string> };
    };
    if (
      pairing.conductor?.zomePath !== 'live' ||
      pairing.dhtParticipation?.dnaHashes?.[o.role] !== c.dna ||
      pairing.dhtParticipation?.agentKeys?.[o.role] !== c.agent
    )
      throw new Error(
        'publication preflight: storage/conductor agent or DNA pairing is not verified'
      );
    await verifyBinding(o, c);
    const grants = o.delegationsFile
      ? (JSON.parse(readFileSync(o.delegationsFile, 'utf8')) as Record<
          string,
          HeadDelegationDocument
        >)
      : {};
    const grantors = o.grantorsFile
      ? (JSON.parse(readFileSync(o.grantorsFile, 'utf8')) as Record<string, ConductorOptions>)
      : {};
    // Validate the entire batch before uploading a blob or changing a row.
    for (const p of prepared) {
      if (!['create', 'update', 'declare'].includes(p.plan.action)) continue;
      let root: Uint8Array | null = null;
      if (p.row?.dhtAnchorHash) {
        const lineage = await c.call<{
          root_action_hash: Uint8Array;
          root_author: Uint8Array;
          content_id: string;
          truncated: boolean;
        }>('get_content_lineage', {
          action_hash: decodeHashFromBase64(p.row.dhtAnchorHash),
          local: true,
        });
        if (lineage.content_id !== p.input.id || lineage.truncated)
          throw new Error(`publication preflight: incomplete or wrong lineage for ${p.input.id}`);
        root = lineage.root_action_hash;
        const grant = grants[p.input.id];
        const author = encodeHashToBase64(lineage.root_author);
        if (grant) {
          if (
            grant.rootActionHash !== encodeHashToBase64(root) ||
            grant.grantor !== author ||
            grant.delegate !== c.agent ||
            grant.dnaHash !== c.dna ||
            grant.scope !== p.input.id
          )
            throw new Error(`publication preflight: grant context mismatch for ${p.input.id}`);
          p.delegation = delegationWire(grant);
          if (!grantors[author])
            throw new Error(`root-author acceptance connection missing for ${p.input.id}`);
          p.grantor = { ...grantors[author], expectedAgent: author, expectedDna: c.dna };
          const signer = await connectConductor(p.grantor);
          await signer.close();
        }
      } else if (grants[p.input.id]) {
        throw new Error(`publication preflight: grant supplied for unresolved root ${p.input.id}`);
      }
      await c.call('preflight_head_publication', {
        id: p.input.id,
        expected_root: root,
        delegation: p.delegation ?? null,
      });
    }
  }
  let problems = 0;
  const heads = new Map<string, string>();
  for (const p of prepared) {
    const id = p.input.id;
    const reach = p.input.reach ?? '?';
    const { action, uncarried } = p.plan;
    if (action === 'unchanged') {
      line(id, reach, 'unchanged', p.row?.dhtAnchorHash ?? '', 'seedHash matches; no writes');
      heads.set(id, p.row?.dhtAnchorHash ?? '');
      if (!o.dryRun) {
        const deadline = Date.now() + o.awaitTimeoutS * 1000;
        if (!(await awaitReceivers(o, p, heads, receivers, getConductor, deadline))) {
          problems++;
          heads.delete(id);
        }
      }
      continue;
    }
    if (action === 'blocked') {
      const bodyNote =
        uncarried.includes('contentBody') && norm(p.row?.contentBody) !== null
          ? '; the conductor path cannot carry a changed body onto an anchored row'
          : '';
      line(
        id,
        reach,
        'blocked',
        p.row?.dhtAnchorHash ?? '',
        `would not carry: ${uncarried.join(', ')}${bodyNote}`
      );
      problems++;
      continue;
    }
    if (o.dryRun) {
      line(
        id,
        reach,
        'planned',
        p.row?.dhtAnchorHash ?? '',
        `would ${action}; seedHash ${p.plan.seedHash.slice(0, 20)}…`
      );
      continue;
    }
    try {
      if (p.item.kind === 'path') {
        for (const ref of pathReferences(p.item.json as unknown as Record<string, unknown>)) {
          if (prepared.some(candidate => candidate.input.id === ref) && !heads.has(ref))
            throw new Error(`course dependency ${ref} has not published successfully`);
          if (
            !prepared.some(candidate => candidate.input.id === ref) &&
            !(await getRow(o.storage, ref))?.dhtAnchorHash
          )
            throw new Error(`course dependency ${ref} is not available`);
        }
      }
      const deadline = Date.now() + o.awaitTimeoutS * 1000;
      const head = await publishOne(o, p, getConductor, deadline);
      line(id, reach, action === 'declare' ? 'declared' : 'published', head);
      heads.set(id, head);
      if (!(await awaitReceivers(o, p, heads, receivers, getConductor, deadline))) {
        problems++;
        heads.delete(id); // Do not compose a course from a dependency still pending elsewhere.
      }
    } catch (e) {
      const pending =
        conductor &&
        pendingPublication(o.receiptDir, {
          id,
          seedHash: p.plan.seedHash,
          agent: conductor.agent,
          dna: conductor.dna,
          storage: o.storage,
        });
      line(
        id,
        reach,
        pending ? 'pending' : 'failed',
        pending?.head ?? p.row?.dhtAnchorHash ?? '',
        String(e instanceof Error ? e.message : e).slice(0, 400)
      );
      problems++;
    }
  }
  await Promise.all([...receivers.values()].map(receiver => receiver.close()));
  if (conductor) await conductor.close();

  process.exit(problems > 0 ? 1 : 0);
}

main().catch(e => {
  console.error(
    `steward-publish: ${String(e instanceof Error ? (e.stack ?? e.message) : e).slice(0, 1200)}`
  );
  process.exit(1);
});
