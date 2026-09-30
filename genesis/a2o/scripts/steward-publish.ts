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
 *       [--dry-run] [--await-peer http://localhost:8091 …] [--await-timeout 600]
 *       [--storage URL] [--admin-ws URL] [--app-ws URL] [--app-id elohim] [--role lamad]
 *       [--data-dir DIR] [--signing-credentials-dir DIR] [--device-agent KEY] [--dna-hash HASH] [--redeclare] [--closure]
 *
 *   ids        resolved to <data-dir>/content/<id>.json or <data-dir>/paths/<id>.json
 *   --manifest a file listing ids (one per line, or a JSON array; `#` comments allowed)
 *   --dry-run  read and plan only; no blob, row, chain or conductor writes
 *   --await-peer  after publishing, poll another peer's storage until it serves the
 *              published seed hash, body and head (repeatable)
 *   --closure  also publish every item a named path's walk points at (its steps and
 *              concept ids), so a path never names an item no peer holds. Each of them
 *              still passes the commons fence.
 *   --redeclare   re-sign the earned declaration even when the row already reads as
 *              current. `unchanged` trusts the row's declared head (GET …/head), which
 *              the storage also sets for the author's own (non-earned) election, so a
 *              run that died between the PATCH and the earned declaration needs this.
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
 * action is published | declared | unchanged | refused | blocked | failed. Exit 0 when
 * every item is published/declared/unchanged, 2 when the commons fence refused anything
 * (nothing is written in that case), 1 on any other problem.
 */
import { createHash } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, extname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { encodeHashToBase64 } from '@holochain/client';

import { buildContentInput, buildPathInput } from '../../seeder/src/content-input.js';

import { connectConductor, type Conductor } from './lib/steward-conductor.js';
import { expandClosure, loadRepoItem, type RepoItem } from './lib/steward-items.js';
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
      '[--app-ws URL] [--app-id ID] [--role ROLE] [--data-dir DIR] [--signing-credentials-dir DIR] [--device-agent KEY] [--dna-hash HASH]'
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
    dryRun: false,
    redeclare: false,
    closure: false,
    awaitPeers: [],
    awaitTimeoutS: 600,
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
  const r = await fetch(url, { ...init, signal: AbortSignal.timeout(180_000) });
  return { status: r.status, text: await r.text() };
}

async function getRow(storage: string, id: string): Promise<ExistingRow | undefined> {
  const r = await http(`${storage}/db/content/${encodeURIComponent(id)}`);
  if (r.status === 404) return undefined;
  if (r.status !== 200) throw new Error(`GET ${id}: ${r.status} ${r.text.slice(0, 200)}`);
  const body = JSON.parse(r.text) as ExistingRow & { content?: ExistingRow };
  return body.content ?? body;
}

interface HeadView {
  headActionHash: string;
  declared: boolean;
  dhtAnchorHash?: string | null;
}

async function getHead(storage: string, id: string): Promise<HeadView | undefined> {
  const r = await http(`${storage}/db/content/${encodeURIComponent(id)}/head`);
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
  head: string
): Promise<{ head: string; author: string; canonical: boolean }> {
  const out = await c.call<{
    head_action_hash: Uint8Array;
    author: Uint8Array;
    canonical: boolean;
  }>('declare_earned_canonical_head', {
    id,
    head_action_hash: head,
    carried_record: null,
    adopt_before_author: false,
    delegation: null,
  });
  return {
    head: encodeHashToBase64(out.head_action_hash),
    author: encodeHashToBase64(out.author),
    canonical: out.canonical,
  };
}

// ─── the run ────────────────────────────────────────────────────────────────

interface Prepared {
  item: RepoItem;
  input: CreateContentInput;
  blob?: BlobSource;
  row?: ExistingRow;
  plan: ItemPlan;
}

type Outcome =
  | 'published'
  | 'declared'
  | 'unchanged'
  | 'refused'
  | 'blocked'
  | 'failed'
  | 'planned';

function line(id: string, reach: string, action: Outcome, head: string, note = ''): void {
  console.log(
    `${id}  reach=${reach}  action=${action}  head=${head || '-'}` + (note ? `  (${note})` : '')
  );
}

function norm(v: string | null | undefined): string | null {
  return v === undefined || v === null || v === '' ? null : v;
}

async function publishOne(
  o: Options,
  p: Prepared,
  conductor: () => Promise<Conductor>
): Promise<string> {
  const { input, plan } = p;
  const id = input.id;
  let head = p.row?.dhtAnchorHash ?? '';
  if (plan.action === 'create' || plan.action === 'update') {
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
  const c = await conductor();
  // The steward's own storage keeps authoring on the same source chain (its backfill
  // and witness sweeps re-author freshly inserted rows), so a declaration can lose
  // the race for the chain head. The publish itself has landed by then; re-read the
  // row and declare the head it now holds, a few times, before calling it failed.
  for (let attempt = 1; ; attempt++) {
    try {
      const out = await declareEarned(c, id, head);
      if (out.head !== head) {
        throw new Error(`earned declaration elected ${out.head}, not the published head ${head}`);
      }
      return head;
    } catch (e) {
      const raced = /source chain head has moved/i.test(String(e));
      if (!raced || attempt >= 4) throw e;
      await new Promise(r => setTimeout(r, 1500 * attempt));
      const landed = await getRow(o.storage, id);
      if (!landed?.dhtAnchorHash) throw e;
      const off = landedMismatches(input, plan.seedHash, landed);
      if (off.length > 0)
        throw new Error(`after a chain-head race the row no longer holds: ${off.join(', ')}`);
      head = landed.dhtAnchorHash;
    }
  }
}

async function awaitPeer(
  o: Options,
  peer: string,
  done: Prepared[],
  heads: Map<string, string>
): Promise<boolean> {
  const deadline = Date.now() + o.awaitTimeoutS * 1000;
  const pending = new Map(done.map(p => [p.input.id, p]));
  const started = Date.now();
  const lastWhy = new Map<string, string>();
  while (pending.size > 0 && Date.now() < deadline) {
    for (const [id, p] of pending) {
      let row: ExistingRow | undefined;
      try {
        row = await getRow(peer, id);
      } catch (e) {
        lastWhy.set(id, String(e).slice(0, 120));
        continue;
      }
      if (!row) {
        lastWhy.set(id, 'no row');
        continue;
      }
      const off = landedMismatches(p.input, p.plan.seedHash, row);
      const head = heads.get(id);
      if (head && row.dhtAnchorHash !== head)
        off.push(`dhtAnchorHash ${row.dhtAnchorHash ?? 'null'} ≠ ${head}`);
      if (off.length === 0) {
        console.log(
          `  ${peer}  ${id}  converged after ${Math.round((Date.now() - started) / 1000)}s`
        );
        pending.delete(id);
      } else {
        lastWhy.set(id, off.join(', '));
      }
    }
    if (pending.size > 0) await new Promise(r => setTimeout(r, 10_000));
  }
  for (const id of pending.keys()) {
    console.log(
      `  ${peer}  ${id}  NOT converged after ${o.awaitTimeoutS}s — ${lastWhy.get(id) ?? '?'}`
    );
  }
  return pending.size === 0;
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
  for (const item of items) {
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
      if (h?.declared && h.headActionHash === row.dhtAnchorHash) earned = h.headActionHash;
    }
    prepared.push({ item, input, blob, row, plan: planItem(input, row, earned) });
  }

  // 3. Execute. The conductor is connected BEFORE the first write, so a wrong port,
  //    app or role fails with nothing written; an all-unchanged run never connects
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
  const writes = prepared.some(p => ['create', 'update', 'declare'].includes(p.plan.action));
  if (writes && !o.dryRun) await getConductor();
  let problems = 0;
  const published: Prepared[] = [];
  const heads = new Map<string, string>();
  for (const p of prepared) {
    const id = p.input.id;
    const reach = p.input.reach ?? '?';
    const { action, uncarried } = p.plan;
    if (action === 'unchanged') {
      line(id, reach, 'unchanged', p.row?.dhtAnchorHash ?? '', 'seedHash matches; no writes');
      published.push(p);
      heads.set(id, p.row?.dhtAnchorHash ?? '');
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
      const head = await publishOne(o, p, getConductor);
      line(id, reach, action === 'declare' ? 'declared' : 'published', head);
      published.push(p);
      heads.set(id, head);
    } catch (e) {
      line(
        id,
        reach,
        'failed',
        p.row?.dhtAnchorHash ?? '',
        String(e instanceof Error ? e.message : e).slice(0, 400)
      );
      problems++;
    }
  }
  if (conductor) await conductor.close();

  // 4. Optionally watch other peers converge on what was published.
  if (!o.dryRun && published.length > 0) {
    for (const peer of o.awaitPeers) {
      console.log(`awaiting ${peer} (timeout ${o.awaitTimeoutS}s)`);
      if (!(await awaitPeer(o, peer, published, heads))) problems++;
    }
  }
  process.exit(problems > 0 ? 1 : 0);
}

main().catch(e => {
  console.error(
    `steward-publish: ${String(e instanceof Error ? (e.stack ?? e.message) : e).slice(0, 1200)}`
  );
  process.exit(1);
});
