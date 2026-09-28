/**
 * Steward grade — a genesis peer widens its OWN rows to their authored reach.
 *
 * Rows an older seeder stamped `private` by default cannot be read — and so never
 * corrected — by the anonymous seeder (403 `{"requiredReach":"private"}`). This step
 * runs against ONE peer and that peer's own conductor. For each id it reads the row
 * anonymously; when the row is narrower than the reach its repo file authors, it asks
 * the conductors to prove the row is the genesis stewards' own, and only then PATCHes
 * the row to exactly the authored reach — storage re-signs that change through the same
 * conductor (`update_content` over an existing chain; `create_content` — this node's own
 * root — when the row has no version chain on the DHT). The steward set is this node's
 * agent plus each co-steward's agent (the other genesis peers, which seed the same ids
 * onto the shared lamad DHT), every key read from that peer's OWN conductor admin
 * interface. It never narrows, never widens past the authored reach,
 * and only widens toward `commons` / `public`. Identity comes from the conductor, never
 * from a storage header (`X-Agent-Cid` is forgeable). Cross-peer adoption is not relied
 * on for reach (backlog security-earned-election-tier-unauthenticated).
 *
 * The decisions live in scripts/lib/steward-grade-plan.ts (unit-tested); the repo
 * closure and the conductor client are shared with scripts/steward-publish.ts.
 *
 * Usage (from genesis/a2o):
 *   pnpm exec tsx scripts/steward-grade.ts [<id> …] [--manifest FILE] [--closure PATH_ID]…
 *       [--dry-run] [--storage URL] [--admin-ws URL] [--app-ws URL] [--app-id elohim]
 *       [--role lamad] [--data-dir DIR] [--co-steward-admin-ws URL]… [--concurrency N]
 *
 *   --closure  grade the path and every item it names (steps, concept ids, and one hop
 *              along the typed edges those items author) — the same closure
 *              steward-publish walks
 *   --dry-run  read and decide only: no row is patched. It still connects to the
 *              conductor when a row needs the authorship proof, and connecting
 *              authorizes signing credentials (a capability grant on the node's chain,
 *              not content); an all-current run never connects.
 *   --co-steward-admin-ws  another genesis peer's conductor admin WS (repeatable). Its
 *              app's cell agent is read there (read-only: no grant, no app interface) and
 *              admitted as a co-author. An unreachable co-steward is logged and left out.
 *   --concurrency  ids graded in parallel (default 4, STEWARD_GRADE_CONCURRENCY).
 *
 * Environment defaults (the household mesh, matthew's peer): STEWARD_STORAGE_URL
 * http://localhost:8090, STEWARD_ADMIN_WS ws://localhost:4444, STEWARD_APP_WS
 * ws://localhost:4445, STEWARD_APP_ID elohim, STEWARD_ROLE lamad, DATA_DIR
 * genesis/data/lamad.
 *
 * A PATCH the peer sheds (503 catching-up, or a zome websocket timeout) waits on a
 * bounded ladder (Retry-After, default 5 s, capped 15 s, 12 waits), re-reading the row
 * before each retry in case the shed write landed anyway.
 *
 * Output: one line per id, in completion order — `<id>  <from>→<to>  <action>` where
 * action is current | widened | widened (new root) | would-widen (dry run) |
 * refused (<reason>) | failed (<error>). Exit 0 when
 * nothing failed (refusals are reported and counted, not failures), 1 otherwise, 2 on
 * bad usage.
 */
import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { encodeHashToBase64 } from '@holochain/client';

import { connectConductor, readConductorAgent, type Conductor } from './lib/steward-conductor.js';
import {
  authoredReachFor,
  classifyRow,
  decideWiden,
  isShedPatch,
  SHED_MAX_WAITS,
  shedDelayMs,
  stewardSet,
  widenLanded,
  widenPatch,
  type ChainAnswer,
  type LineageB64,
  type ObservedRow,
  type StewardSet,
} from './lib/steward-grade-plan.js';
import { expandClosure, loadRepoItem } from './lib/steward-items.js';

const HERE = dirname(fileURLToPath(import.meta.url));
const GENESIS_DIR = resolve(HERE, '..', '..');

interface Options {
  ids: string[];
  closures: string[];
  dataDir: string;
  storage: string;
  adminWs: string;
  appWs: string;
  appId: string;
  role: string;
  dryRun: boolean;
  coStewardAdminWs: string[];
  concurrency: number;
}

function usage(msg?: string): never {
  if (msg) console.error(`steward-grade: ${msg}`);
  console.error(
    'usage: steward-grade.ts [<id>…] [--manifest FILE] [--closure PATH_ID]… [--dry-run] ' +
      '[--storage URL] [--admin-ws URL] [--app-ws URL] [--app-id ID] [--role ROLE] [--data-dir DIR] ' +
      '[--co-steward-admin-ws URL]… [--concurrency N]'
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
    closures: [],
    dataDir: env.DATA_DIR ?? join(GENESIS_DIR, 'data', 'lamad'),
    storage: env.STEWARD_STORAGE_URL ?? 'http://localhost:8090',
    adminWs: env.STEWARD_ADMIN_WS ?? 'ws://localhost:4444',
    appWs: env.STEWARD_APP_WS ?? 'ws://localhost:4445',
    appId: env.STEWARD_APP_ID ?? 'elohim',
    role: env.STEWARD_ROLE ?? 'lamad',
    dryRun: false,
    coStewardAdminWs: [],
    concurrency: Number(env.STEWARD_GRADE_CONCURRENCY ?? 4),
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
      case '--closure':
        o.closures.push(val());
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
      case '--dry-run':
        o.dryRun = true;
        break;
      case '--co-steward-admin-ws':
        o.coStewardAdminWs.push(val());
        break;
      case '--concurrency':
        o.concurrency = Number(val());
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
  if (o.ids.length === 0 && o.closures.length === 0) usage('no ids or --closure given');
  if (!Number.isInteger(o.concurrency) || o.concurrency < 1) usage('--concurrency must be >= 1');
  return o;
}

// ─── storage (anonymous reads, one PATCH) ─────────────────────────────────────

async function readAnonymously(storage: string, id: string): Promise<ObservedRow> {
  // Deliberately no identity headers: the question is what an anonymous reader sees.
  const r = await fetch(`${storage}/db/content/${encodeURIComponent(id)}`, {
    signal: AbortSignal.timeout(60_000),
  });
  const text = await r.text();
  if (r.status === 404) return { kind: 'absent' };
  if (r.status === 403) {
    let required: string | null = null;
    try {
      const body = JSON.parse(text) as { requiredReach?: unknown };
      if (typeof body.requiredReach === 'string') required = body.requiredReach;
    } catch {
      /* a 403 without a JSON body carries no requiredReach */
    }
    return { kind: 'gated', requiredReach: required };
  }
  if (r.status !== 200) throw new Error(`GET ${id}: ${r.status} ${text.slice(0, 200)}`);
  const body = JSON.parse(text) as { reach?: string | null; content?: { reach?: string | null } };
  const row = body.content ?? body;
  return { kind: 'served', reach: row.reach ?? null };
}

/**
 * PATCH the row's reach. A shed PATCH (503 catching-up / zome websocket timeout) waits
 * on the bounded ladder; before each retry the row is re-read, since a write that timed
 * out on the storage side may still have committed. Returns once the PATCH is accepted
 * or the re-read already serves at `to`.
 */
async function patchReach(
  storage: string,
  id: string,
  to: Parameters<typeof widenPatch>[0]
): Promise<void> {
  for (let waits = 0; ; waits++) {
    const r = await fetch(`${storage}/db/content/${encodeURIComponent(id)}`, {
      method: 'PATCH',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(widenPatch(to)),
      signal: AbortSignal.timeout(180_000),
    });
    if (r.status < 300) return;
    const text = await r.text();
    if (!isShedPatch(r.status, text) || waits >= SHED_MAX_WAITS) {
      throw new PatchError(r.status, text);
    }
    await new Promise(res => setTimeout(res, shedDelayMs(r.headers.get('Retry-After'), text)));
    if (widenLanded(to, await readAnonymously(storage, id))) return;
  }
}

class PatchError extends Error {
  constructor(
    readonly status: number,
    readonly body: string
  ) {
    super(`PATCH: ${status} ${body.slice(0, 300)}`);
  }
}

// ─── conductor (the authorship proof) ─────────────────────────────────────────

interface HeadWire {
  head_action_hash: Uint8Array;
  author: Uint8Array;
}

interface LineageWire {
  root_author: Uint8Array;
  content_id: string;
  candidates: {
    action_hash: Uint8Array;
    author: Uint8Array | null;
    fetch_outcome: string;
    in_root: boolean;
  }[];
  other_root_candidates: number;
  unfetchable_candidates: number;
  invalid_link_targets: number;
  truncated: boolean;
}

/** A zome answer that says "not yet", which is a refusal to retry, not a failure. */
function isPending(e: unknown): boolean {
  return /PENDING|not retrievable/i.test(String(e instanceof Error ? e.message : e));
}

/** What this node's conductor says about the id's version chain (rule 4's input). */
async function chainAnswer(c: Conductor, id: string): Promise<ChainAnswer> {
  try {
    const h = await c.call<HeadWire | null>('resolve_content_head', id);
    if (!h) return { kind: 'no-chain' };
    const head = {
      headActionHash: encodeHashToBase64(h.head_action_hash),
      author: encodeHashToBase64(h.author),
    };
    const l = await c.call<LineageWire>('get_content_lineage', {
      action_hash: h.head_action_hash,
      local: false,
    });
    const lineage: LineageB64 = {
      rootAuthor: encodeHashToBase64(l.root_author),
      contentId: l.content_id,
      candidates: l.candidates.map(x => ({
        actionHash: encodeHashToBase64(x.action_hash),
        author: x.author ? encodeHashToBase64(x.author) : null,
        fetchOutcome: x.fetch_outcome,
        inRoot: x.in_root,
      })),
      otherRootCandidates: l.other_root_candidates,
      unfetchableCandidates: l.unfetchable_candidates,
      invalidLinkTargets: l.invalid_link_targets,
      truncated: l.truncated,
    };
    if (lineage.contentId !== id) {
      throw new Error(`lineage names content id "${lineage.contentId}", not "${id}"`);
    }
    return { kind: 'chain', head, lineage };
  } catch (e) {
    if (isPending(e)) return { kind: 'pending', detail: String(e).slice(0, 160) };
    throw e;
  }
}

/** Connect this node's conductor and read every co-steward's key from its own conductor. */
async function connectStewards(
  o: Options
): Promise<{ conductor: Conductor; stewards: StewardSet }> {
  const conductor = await connectConductor(o);
  console.log(`node agent ${conductor.agent} (app ${o.appId}, role ${o.role}, ${o.adminWs})`);
  const co: [string, string][] = [];
  for (const url of o.coStewardAdminWs) {
    try {
      const key = await readConductorAgent(url, o.appId, o.role);
      if (key === conductor.agent) {
        console.log(`co-steward ${url} is this node's own conductor — ignored`);
        continue;
      }
      co.push([key, url]);
      console.log(`co-steward ${key} (${url})`);
    } catch (e) {
      console.log(
        `co-steward ${url} unreachable — proceeding without it (its ids stay refused): ` +
          String(e instanceof Error ? e.message : e).slice(0, 200)
      );
    }
  }
  return { conductor, stewards: stewardSet(conductor.agent, co) };
}

/** Run `fn` over `items` with at most `limit` in flight. */
async function forEachBounded<T>(
  items: readonly T[],
  limit: number,
  fn: (item: T) => Promise<void>
): Promise<void> {
  let next = 0;
  const worker = async (): Promise<void> => {
    while (next < items.length) await fn(items[next++]);
  };
  await Promise.all(Array.from({ length: Math.min(limit, items.length) }, worker));
}

// ─── the run ──────────────────────────────────────────────────────────────────

function line(id: string, from: string, to: string, action: string, note = ''): void {
  console.log(`${id}  ${from}→${to}  ${action}` + (note ? ` (${note})` : ''));
}

async function main(): Promise<void> {
  const o = parseArgs(process.argv.slice(2));
  const ids = expandClosure(o.dataDir, [...o.closures, ...o.ids]);

  // Connected lazily (once, shared by every worker): an all-current run never connects.
  let stewardsP: Promise<{ conductor: Conductor; stewards: StewardSet }> | undefined;
  const getStewards = async (): Promise<{ conductor: Conductor; stewards: StewardSet }> =>
    (stewardsP ??= connectStewards(o));

  const counts = { current: 0, widened: 0, 'would-widen': 0, refused: 0, failed: 0 };
  await forEachBounded(ids, o.concurrency, async id => {
    const { item, error } = loadRepoItem(o.dataDir, id);
    if (!item) {
      line(id, '?', '?', 'refused', error);
      counts.refused++;
      return;
    }
    const authored = authoredReachFor(item.kind, item.json);
    let from = '?';
    const to = authored ?? 'none';
    try {
      const verdict = classifyRow(authored, await readAnonymously(o.storage, id));
      from = verdict.from;
      let answer: ChainAnswer = { kind: 'no-chain' };
      let stewards = stewardSet('');
      if (verdict.action === 'needs-authorship') {
        const s = await getStewards();
        stewards = s.stewards;
        answer = await chainAnswer(s.conductor, id);
      }
      const d = decideWiden(verdict, stewards, answer);
      if (d.action === 'current') {
        line(id, d.from, d.to, 'current');
        counts.current++;
        return;
      }
      if (d.action === 'refused') {
        line(id, d.from, d.to, 'refused', d.reason);
        counts.refused++;
        return;
      }
      const newRoot = d.mode === 'new-root';
      if (o.dryRun) {
        line(
          id,
          d.from,
          d.to,
          'would-widen',
          newRoot ? 'no version chain: new root as this node; dry run' : 'steward-authored; dry run'
        );
        counts['would-widen']++;
        return;
      }
      try {
        await patchReach(o.storage, id, d.to);
      } catch (e) {
        // A root appeared on the DHT between the head read and create_content: the zome
        // refused to fork it. Not a failure — the next run grades it over that chain.
        if (newRoot && e instanceof PatchError && /already exists/i.test(e.body)) {
          line(
            id,
            d.from,
            d.to,
            'refused',
            'a version chain appeared during the grade; re-graded next run'
          );
          counts.refused++;
          return;
        }
        throw e;
      }
      const reread = await readAnonymously(o.storage, id);
      if (!widenLanded(d.to, reread)) {
        throw new Error(`PATCH accepted but an anonymous re-read sees ${JSON.stringify(reread)}`);
      }
      line(id, d.from, d.to, newRoot ? 'widened (new root)' : 'widened');
      counts.widened++;
    } catch (e) {
      line(id, from, to, 'failed', String(e instanceof Error ? e.message : e).slice(0, 400));
      counts.failed++;
    }
  });
  const conductor = stewardsP ? (await stewardsP.catch(() => undefined))?.conductor : undefined;
  if (conductor) await conductor.close();

  console.log(
    `steward-grade: ${ids.length} id(s) — current ${counts.current}, widened ${counts.widened}, ` +
      `would-widen ${counts['would-widen']}, refused ${counts.refused}, failed ${counts.failed}` +
      (o.dryRun ? ' (dry run: nothing patched)' : '')
  );
  process.exit(counts.failed > 0 ? 1 : 0);
}

main().catch(e => {
  console.error(
    `steward-grade: ${String(e instanceof Error ? (e.stack ?? e.message) : e).slice(0, 1200)}`
  );
  process.exit(1);
});
