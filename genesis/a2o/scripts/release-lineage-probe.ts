/**
 * Read-only lineage diagnostic for release channel
 * `runtime:coordinators:elohim:receipt-20260901-r2` (2026-09-02).
 *
 * james's storage controller refuses release P
 * (`uhCkkeEOIsGKa4N3XAW-6GqgFfNyCOBGqqC6newNel7zr06nAIKYx`) with
 * `lineage_parent_mismatch: envelope declares Some(O) but the channel's
 * release chain supersedes None`. This script asks, per conductor
 * (james admin 4464/app 4465, matthew admin 4444/app 4445):
 *
 *   1. resolve_content_head_local(channelId) — full object, especially
 *      head_action_hash/cid and supersedes.
 *   2. resolve_canonical_election(channelId) — winner_target + tier, for
 *      comparison against (1).
 *   3. get_content(action_hash) for: the supersedes value from (1) (if
 *      Some), release O's action hash explicitly, and the channel root's
 *      action hash if determinable — null vs Content (metadata_json.kind
 *      + record id) for each.
 *   4. Whether any extern exposes a full IdToContent version listing.
 *
 * NEVER mutates anything: only resolve_content_head_local,
 * resolve_canonical_election, and get_content are called — all read-only
 * `#[hdk_extern]`s per elohim/holochain/dna/elohim/zomes/content_store/
 * src/lib.rs (verified 2026-09-02: resolve_content_head_local ~3874,
 * resolve_canonical_election ~5604, get_content ~6594). Does not restart
 * or start any process; connects to the ALREADY-RUNNING local mesh only.
 *
 * ## Rail composed, never re-derived
 *
 * The admin/app WS connect + cell resolution + zome-call-signing rail
 * (`conductor()`) below is copied in SHAPE from
 * `genesis/a2o/scripts/release-ceremony.ts`'s own `conductor()` helper
 * (itself copied from the frozen oracle `carried-election-mesh-proof.ts`):
 * `AdminWebsocket.connect` -> `listApps` -> walk `cell_info` for the
 * `provisioned` `lamad` role cell -> `authorizeSigningCredentials` ->
 * `issueAppAuthenticationToken` -> `AppWebsocket.connect` -> `callZome`.
 * `release-ceremony.ts` has no exports (standalone `main()`), so this file
 * re-implements the identical rail rather than importing it.
 *
 * ## Version-listing extern (item 4)
 *
 * Grepped `pub fn` across content_store/src/lib.rs for
 * get_content_by_id / get_content_versions / get_content_chain-shaped
 * names. Found:
 *   - `get_content_by_id(input: QueryByIdInput) -> Option<ContentOutput>`
 *     (~6627) — resolves ONE current version via the IdToContent link +
 *     healing_integration, not a list.
 *   - `gather_content_chain` (~2791) walks ALL retrievable IdToContent
 *     link-target records for an id, but it is a private fn — no
 *     `#[hdk_extern]` wraps it, so no zome call can list the chain.
 * Conclusion (printed by this script, not asserted a priori): NO extern
 * exposes a full IdToContent version listing. Consequently the channel
 * ROOT's action hash cannot be independently retrieved via any exposed
 * zome call either — this script reports that gap explicitly instead of
 * guessing at it.
 *
 * ## Usage
 *
 *   cd genesis/a2o && pnpm exec tsx scripts/release-lineage-probe.ts
 *
 * Optional overrides (positional, in order): channelId, oHash, pHash.
 */
import { AdminWebsocket, AppWebsocket, encodeHashToBase64, decodeHashFromBase64 } from '@holochain/client';

const APP_ID = 'elohim';
const ROLE = 'lamad';
const ZOME = 'content_store';
const TIMEOUT_MS = 15_000;

const CHANNEL_ID = process.argv[2] ?? 'runtime:coordinators:elohim:receipt-20260901-r2';
const RELEASE_O = process.argv[3] ?? 'uhCkkIDNjyHg6QpNy1tcU0-CXpxa5KKrDkSh8Qg-xgaDZDnTb3p2E';
const RELEASE_P = process.argv[4] ?? 'uhCkkeEOIsGKa4N3XAW-6GqgFfNyCOBGqqC6newNel7zr06nAIKYx';

interface PeerConfig {
  name: string;
  admin: number;
  app: number;
}

const PEERS: PeerConfig[] = [
  { name: 'james', admin: 4464, app: 4465 },
  { name: 'matthew', admin: 4444, app: 4445 },
];

function withTimeout<T>(p: Promise<T>, ms: number, label: string): Promise<T> {
  return new Promise((resolve, reject) => {
    const t = setTimeout(() => reject(new Error(`timeout after ${ms}ms: ${label}`)), ms);
    p.then(
      (v) => {
        clearTimeout(t);
        resolve(v);
      },
      (e) => {
        clearTimeout(t);
        reject(e);
      },
    );
  });
}

/** Copied in shape from release-ceremony.ts's conductor() — see module doc. */
async function conductor(name: string, adminPort: number, appPort: number, timeoutMs: number) {
  const wsOpts: any = { origin: APP_ID };
  const admin = await withTimeout(
    AdminWebsocket.connect({ url: new URL(`ws://127.0.0.1:${adminPort}`), wsClientOptions: wsOpts }),
    timeoutMs,
    `admin connect ${name}:${adminPort}`,
  );
  const apps = await admin.listApps({});
  const app = apps.find((a: any) => a.installed_app_id === APP_ID) ?? apps[0];
  if (!app) {
    throw new Error(
      `${name}: no installed app on adminPort=${adminPort} (saw: ${apps.map((a: any) => a.installed_app_id).join(',')})`,
    );
  }
  const cells = new Map<string, any>();
  for (const [role, infos] of Object.entries(app.cell_info)) {
    for (const info of infos as any[]) {
      if (info.type === 'provisioned' && info.value?.cell_id) cells.set(role, info.value.cell_id);
    }
  }
  const lamad = cells.get(ROLE);
  if (!lamad) {
    throw new Error(`${name}: no '${ROLE}' cell provisioned (roles: ${[...cells.keys()].join(',')})`);
  }
  await admin.authorizeSigningCredentials(lamad);
  const token = await admin.issueAppAuthenticationToken({ installed_app_id: app.installed_app_id });
  const appWs = await withTimeout(
    AppWebsocket.connect({
      url: new URL(`ws://127.0.0.1:${appPort}`),
      token: token.token,
      wsClientOptions: wsOpts,
    }),
    timeoutMs,
    `app connect ${name}:${appPort}`,
  );
  const call = (fn_name: string, payload: any) =>
    appWs.callZome({ cell_id: lamad, zome_name: ZOME, fn_name, payload });
  return { name, admin, appWs, call, agent: encodeHashToBase64(lamad[1]) as string };
}

/** Deep-walk a decoded zome return value, base64-encoding any Uint8Array
 * (raw hash bytes) it finds, so the printed JSON carries readable cids
 * instead of byte-array dumps. Leaves everything else untouched. */
function b64ify(v: unknown): unknown {
  if (v instanceof Uint8Array) {
    try {
      return encodeHashToBase64(v);
    } catch {
      return Buffer.from(v).toString('base64');
    }
  }
  if (Array.isArray(v)) return v.map(b64ify);
  if (v && typeof v === 'object') {
    const out: Record<string, unknown> = {};
    for (const [k, val] of Object.entries(v as Record<string, unknown>)) out[k] = b64ify(val);
    return out;
  }
  return v;
}

function safeParseKind(metadataJson: unknown): { kind?: unknown; parseError?: string } {
  if (typeof metadataJson !== 'string') return { parseError: 'metadata_json is not a string' };
  try {
    const parsed = JSON.parse(metadataJson);
    return { kind: parsed?.kind };
  } catch (e) {
    return { parseError: String(e).slice(0, 200) };
  }
}

async function probeGetContent(call: (fn: string, p: any) => Promise<any>, label: string, hashB64: string) {
  let actionHashBytes: Uint8Array;
  try {
    actionHashBytes = decodeHashFromBase64(hashB64);
  } catch (e) {
    return { label, hash: hashB64, error: `decodeHashFromBase64 failed: ${String(e).slice(0, 200)}` };
  }
  try {
    const result: any = await call('get_content', actionHashBytes);
    if (result === null || result === undefined) {
      return { label, hash: hashB64, result: null };
    }
    const { kind, parseError } = safeParseKind(result.content?.metadata_json);
    return {
      label,
      hash: hashB64,
      result: 'Content',
      recordId: result.content?.id ?? null,
      metadataKind: kind ?? null,
      metadataParseError: parseError,
      actionHash: b64ify(result.action_hash),
      entryHash: b64ify(result.entry_hash),
    };
  } catch (e) {
    return { label, hash: hashB64, error: String(e).slice(0, 400) };
  }
}

async function probePeer(peer: PeerConfig) {
  const report: Record<string, unknown> = { peer: peer.name, admin: peer.admin, app: peer.app };
  let conn: Awaited<ReturnType<typeof conductor>>;
  try {
    conn = await conductor(peer.name, peer.admin, peer.app, TIMEOUT_MS);
  } catch (e) {
    report.reachable = false;
    report.error = String(e).slice(0, 500);
    return report;
  }
  report.reachable = true;
  report.agent = conn.agent;

  try {
    // 1. resolve_content_head_local
    let headResult: any = null;
    let headError: string | null = null;
    try {
      headResult = await conn.call('resolve_content_head_local', CHANNEL_ID);
    } catch (e) {
      headError = String(e).slice(0, 500);
    }
    report.resolve_content_head_local =
      headResult === null
        ? null
        : headError
          ? { error: headError }
          : b64ify(headResult);
    if (headError) report.resolve_content_head_local_error = headError;

    const supersedesB64: string | null =
      headResult && headResult.supersedes !== null && headResult.supersedes !== undefined
        ? (b64ify(headResult.supersedes) as string)
        : null;

    // 2. resolve_canonical_election
    let electionResult: any = null;
    let electionError: string | null = null;
    try {
      electionResult = await conn.call('resolve_canonical_election', CHANNEL_ID);
    } catch (e) {
      electionError = String(e).slice(0, 500);
    }
    if (electionError) {
      report.resolve_canonical_election = { error: electionError };
    } else {
      report.resolve_canonical_election =
        electionResult === null
          ? null
          : {
              tier: electionResult.canonical_earned ? 'earned' : 'staging',
              winner_target: b64ify(electionResult.winner_target),
              canonical_declared_at: electionResult.canonical_declared_at ?? null,
              raw: b64ify(electionResult),
            };
    }

    // 3. get_content probes: supersedes (if Some), O explicitly, root (not
    // determinable — see module doc item 4; recorded as a stated gap).
    const getContentProbes: Record<string, unknown> = {};
    getContentProbes.supersedes =
      supersedesB64 === null
        ? { skipped: 'supersedes was None on this conductor — nothing to look up' }
        : await probeGetContent(conn.call, 'supersedes', supersedesB64);
    getContentProbes.release_O = await probeGetContent(conn.call, 'release_O', RELEASE_O);
    getContentProbes.channel_root = {
      skipped:
        'no exposed extern returns the channel root action hash (get_content_by_id resolves ' +
        'the CURRENT version via IdToContent + healing_integration, not the root; the private ' +
        'gather_content_chain walks the full chain but has no #[hdk_extern] wrapper) — not looked up',
    };
    report.get_content = getContentProbes;

    // Bonus context: also print release P's get_content result, since the
    // whole incident is about P being refused.
    report.get_content_release_P_context = await probeGetContent(conn.call, 'release_P_context', RELEASE_P);
  } finally {
    try {
      await conn.appWs.client.close();
    } catch {
      /* best-effort */
    }
    try {
      await conn.admin.client.close();
    } catch {
      /* best-effort */
    }
  }
  return report;
}

async function main() {
  console.error(`channelId = ${CHANNEL_ID}`);
  console.error(`release O = ${RELEASE_O}`);
  console.error(`release P = ${RELEASE_P}`);
  console.error(
    'version-listing extern check: NONE exposed — get_content_by_id returns one current ' +
      'version (not a list); gather_content_chain walks the full chain but is a private fn, ' +
      'no #[hdk_extern] wrapper (elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs).',
  );

  const reports = [];
  for (const peer of PEERS) {
    // eslint-disable-next-line no-await-in-loop
    reports.push(await probePeer(peer));
  }

  console.log(
    JSON.stringify(
      {
        channelId: CHANNEL_ID,
        releaseO: RELEASE_O,
        releaseP: RELEASE_P,
        checkedAt: new Date().toISOString(),
        versionListingExternExposed: false,
        versionListingNote:
          'get_content_by_id -> Option<ContentOutput> (current version only); ' +
          'gather_content_chain (full chain walk) is private, no #[hdk_extern]',
        peers: reports,
      },
      null,
      2,
    ),
  );
  process.exit(0);
}

main().catch((e) => {
  console.error(String(e?.stack ?? e).slice(0, 2000));
  process.exit(1);
});
