/**
 * Steward grade — a genesis peer widens its OWN rows to their authored reach.
 *
 * Rows an older seeder stamped `private` by default cannot be read — and so never
 * corrected — by the anonymous seeder (403 `{"requiredReach":"private"}`). This step
 * runs against ONE peer and that peer's own conductor. For each id it reads the row
 * anonymously; when the row is narrower than the reach its repo file authors, it asks
 * the conductor to prove the node's own agent authored the row, and only then PATCHes
 * the row to exactly the authored reach — storage re-signs that change through the same
 * conductor's `update_content`. It never narrows, never widens past the authored reach,
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
 *       [--role lamad] [--data-dir DIR]
 *
 *   --closure  grade the path and every item it names (steps, concept ids, and one hop
 *              along the typed edges those items author) — the same closure
 *              steward-publish walks
 *   --dry-run  read and decide only: no row is patched. It still connects to the
 *              conductor when a row needs the authorship proof, and connecting
 *              authorizes signing credentials (a capability grant on the node's chain,
 *              not content); an all-current run never connects.
 *
 * Environment defaults (the household mesh, matthew's peer): STEWARD_STORAGE_URL
 * http://localhost:8090, STEWARD_ADMIN_WS ws://localhost:4444, STEWARD_APP_WS
 * ws://localhost:4445, STEWARD_APP_ID elohim, STEWARD_ROLE lamad, DATA_DIR
 * genesis/data/lamad.
 *
 * Output: one line per id — `<id>  <from>→<to>  <action>` where action is current |
 * widened | would-widen (dry run) | refused (<reason>) | failed (<error>). Exit 0 when
 * nothing failed (refusals are reported and counted, not failures), 1 otherwise, 2 on
 * bad usage.
 */
import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { encodeHashToBase64 } from '@holochain/client';

import { connectConductor, type Conductor } from './lib/steward-conductor.js';
import {
  authoredReachFor,
  classifyRow,
  ownAuthorshipRefusal,
  widenLanded,
  widenPatch,
  type HeadB64,
  type LineageB64,
  type ObservedRow,
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
}

function usage(msg?: string): never {
  if (msg) console.error(`steward-grade: ${msg}`);
  console.error(
    'usage: steward-grade.ts [<id>…] [--manifest FILE] [--closure PATH_ID]… [--dry-run] ' +
      '[--storage URL] [--admin-ws URL] [--app-ws URL] [--app-id ID] [--role ROLE] [--data-dir DIR]'
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

async function patchReach(storage: string, id: string, patch: object): Promise<void> {
  const r = await fetch(`${storage}/db/content/${encodeURIComponent(id)}`, {
    method: 'PATCH',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(patch),
    signal: AbortSignal.timeout(180_000),
  });
  if (r.status >= 300) throw new Error(`PATCH: ${r.status} ${(await r.text()).slice(0, 300)}`);
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

async function authorshipRefusal(c: Conductor, id: string): Promise<string | undefined> {
  let head: HeadB64 | null = null;
  let lineage: LineageB64 | null = null;
  try {
    const h = await c.call<HeadWire | null>('resolve_content_head', id);
    if (h) {
      head = {
        headActionHash: encodeHashToBase64(h.head_action_hash),
        author: encodeHashToBase64(h.author),
      };
      const l = await c.call<LineageWire>('get_content_lineage', {
        action_hash: h.head_action_hash,
        local: false,
      });
      lineage = {
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
        return `lineage names content id "${lineage.contentId}"`;
      }
    }
  } catch (e) {
    if (isPending(e)) return `authorship pending on the DHT: ${String(e).slice(0, 160)}`;
    throw e;
  }
  return ownAuthorshipRefusal(c.agent, head, lineage);
}

// ─── the run ──────────────────────────────────────────────────────────────────

function line(id: string, from: string, to: string, action: string, note = ''): void {
  console.log(`${id}  ${from}→${to}  ${action}` + (note ? ` (${note})` : ''));
}

async function main(): Promise<void> {
  const o = parseArgs(process.argv.slice(2));
  const ids = expandClosure(o.dataDir, [...o.closures, ...o.ids]);

  let conductor: Conductor | undefined;
  const getConductor = async (): Promise<Conductor> => {
    if (!conductor) {
      conductor = await connectConductor(o);
      console.log(`node agent ${conductor.agent} (app ${o.appId}, role ${o.role}, ${o.adminWs})`);
    }
    return conductor;
  };

  const counts = { current: 0, widened: 0, 'would-widen': 0, refused: 0, failed: 0 };
  for (const id of ids) {
    const { item, error } = loadRepoItem(o.dataDir, id);
    if (!item) {
      line(id, '?', '?', 'refused', error);
      counts.refused++;
      continue;
    }
    const authored = authoredReachFor(item.kind, item.json);
    let from = '?';
    const to = authored ?? 'none';
    try {
      const verdict = classifyRow(authored, await readAnonymously(o.storage, id));
      from = verdict.from;
      if (verdict.action === 'current') {
        line(id, verdict.from, verdict.to, 'current');
        counts.current++;
        continue;
      }
      if (verdict.action === 'refused') {
        line(id, verdict.from, verdict.to, 'refused', verdict.reason);
        counts.refused++;
        continue;
      }
      const why = await authorshipRefusal(await getConductor(), id);
      if (why) {
        line(id, verdict.from, verdict.to, 'refused', why);
        counts.refused++;
        continue;
      }
      if (o.dryRun) {
        line(id, verdict.from, verdict.to, 'would-widen', 'own-authored; dry run');
        counts['would-widen']++;
        continue;
      }
      await patchReach(o.storage, id, widenPatch(verdict.to));
      const reread = await readAnonymously(o.storage, id);
      if (!widenLanded(verdict.to, reread)) {
        throw new Error(`PATCH accepted but an anonymous re-read sees ${JSON.stringify(reread)}`);
      }
      line(id, verdict.from, verdict.to, 'widened');
      counts.widened++;
    } catch (e) {
      line(id, from, to, 'failed', String(e instanceof Error ? e.message : e).slice(0, 400));
      counts.failed++;
    }
  }
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
