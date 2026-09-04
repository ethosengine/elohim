/**
 * Release-channel ceremony driver (2026-09-01) — task-release-channel-ceremony-driver.
 *
 * Proves the spec's whole thesis (genesis/docs/superpowers/specs/
 * 2026-09-01-runtime-artifacts-elected-content-design.md §3-§5): "which
 * release is canonical" is the SAME ceremony as content-head election. This
 * script drives it for runtime release CHANNELS — author a channel, publish
 * releases as versions, stage → promote → revert-by-re-election — exactly as
 * `device-ceremony.ts` already drives it for epr-content (manifesto), and
 * exactly as `carried-election-mesh-proof.ts` proves the cross-peer carry.
 *
 * ## Rails composed, never re-derived
 *
 * The admin/app WS connect + cell resolution + zome-call-signing rail below
 * (`conductor()`) is copied in SHAPE from the frozen oracle
 * `carried-election-mesh-proof.ts` (2026-08-31, habit dataplane-convergence):
 * `AdminWebsocket.connect` → `listApps` → walk `cell_info` for the
 * `provisioned` `lamad` role cell → `authorizeSigningCredentials` →
 * `AppWebsocket.connect` with an issued app-auth token → `callZome`. That
 * file is FROZEN (never edited, never imported — it has no exports; it is a
 * standalone `main()`). This script re-implements the identical rail,
 * parameterized over multiple named peers, and drives DIFFERENT zome
 * functions on it (content authoring + the canonical/earned head-declare
 * pair + the election-only resolver) per the atom's hard rail #4.
 *
 * ## Zome functions driven (payload/output shapes read from source, not
 * guessed — elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs)
 *
 *   - `create_content` (~2366)         — channel root authoring
 *   - `update_content` (~2471)         — each `publish` is a new Update
 *     version on the channel's OWN content id (patches `metadata_json` only
 *     — `UpdateContentInput` has no `content` field; the release manifest
 *     rides the `metadata_json` discriminator valve, exactly as spec §5
 *     specifies for the MVP: `kind: "release-manifest"`).
 *   - `declare_canonical_content_head` (~5386) — STAGING cross-root head
 *   - `declare_earned_canonical_head` (~5431)  — EARNED head (three-arm
 *     authority: root author / delegated device / bootstrap steward)
 *   - `resolve_canonical_election` (~5604)     — election-only read (tier +
 *     winner cid, GetStrategy::Local — never a network await, so a peer that
 *     cannot be REACHED at all is honestly distinct from a peer that answers
 *     "no election yet")
 *
 * ## Adopt-before-author (atom scope item 2)
 *
 * `publish` pre-flights `resolve_canonical_election` on the acting peer
 * before authoring. If the channel's current head is already EARNED, the
 * script must not crown its own commit over an authority declaration it has
 * not adopted — but a channel is LONG-LIVED (the story's own vocabulary):
 * the steward publishes on the same channel for as long as the household
 * keeps it, so refusing every publish over an earned head forces a fresh
 * channel per release, which is the seam Station 9 exists to close.
 *
 * The pre-flight therefore has an ADMITTED arm and a refusing arm, and both
 * conditions must hold for admission:
 *
 *   (a) ADOPTED — the peer named by `--adoption-url` reports, on
 *       `GET /admin/adoption`, a row for this channel whose
 *       `appliedRelease.cid` IS the earned head's action hash. "A steward
 *       cannot push what they have not themselves adopted."
 *   (b) LINEAGE — the manifest being published declares
 *       `envelope.lineageParentCid` === that same earned head. "The
 *       candidate names that head as the release it builds on."
 *
 * Both hold → publish authors the version and declares STAGING exactly as
 * on a fresh channel (the earned head stays the head; only the promotion
 * ceremony moves it). Only one holds → a typed, one-sentence refusal naming
 * which: `not_adopted` or `lineage_parent_mismatch`. Neither holds → the
 * original refusal text, with both codes appended.
 *
 * This is the practical subset of the four-arm `services/head_adoption.rs`
 * pre-flight relevant to a single-conductor ceremony driver (LOCAL-DHT arm
 * plus the local controller's own adoption receipt); the full
 * sweep-controller semantics (PEER-HINT / AUTHOR-THEN-ADOPT /
 * CONTEST-THEN-OBEY) belong to `task-release-adoption-controller-observe`,
 * not this driver. `revert <channelId> <manifest.json>` is the one caller
 * that bypasses this pre-flight entirely: it re-elects a NEW version bound
 * to OLD bytes (a revert target can never be an existing version's action
 * hash — a release manifest's `appliesTo.coordinatorWasmHashes` names what
 * it SUPERSEDES, and the adoption controller refuses a peer not running
 * them). Revert is exempt because it is not a staging candidate contending
 * for the head at all — it declares EARNED directly, so the thing the
 * pre-flight guards against (crowning an un-adopted commit over an
 * authority declaration) does not apply; the declaring peer IS the
 * authority moving the earned head.
 *
 * NOTE ON THE ZOME FLOOR: the driver's admission is a PRE-flight, not the
 * authority. `content_store::declare_canonical_content_head` carries its own
 * earned-head guard, so a staging declaration beneath an earned head is
 * admitted only once that zome-side guard also admits it (the DNA half of
 * Station 9). Until it does, an admitted publish still surfaces the zome's
 * "earned head is protected" Guest error — a floor refusing, not this
 * driver's pre-flight.
 *
 * ## Usage
 *
 *   cd genesis/a2o && pnpm exec tsx scripts/release-ceremony.ts <verb> ...
 *
 *   channel create <channelId> [--reach <tier>] [--discipline <json|path>]
 *   publish <manifest.json> [--adoption-url <storage base url>]
 *   promote <channelId> <releaseCid> [--delegation <file>]
 *   revert <channelId> <priorReleaseCid | manifest.json> [--delegation <file>]
 *   status <channelId>
 *
 * Common flags:
 *   --as <peerName>        act as this configured peer (default: first, "matthew")
 *   --conductors <csv>     name=admin:app,... (default: local household mesh)
 *   --timeout <ms>         per-peer connect timeout (default 5000)
 *   --adoption-url <url>   storage base url whose GET /admin/adoption proves the
 *                          acting peer has ADOPTED the channel's earned head
 *                          (publish only; default: the acting peer's own storage
 *                          URL — see `resolveAdoptionUrl`)
 *
 * Environment:
 *   PEER_CONDUCTOR_PORTS   conductor CSV used when --conductors is absent
 *                          (same convention as version-matrix.ts)
 *   RELEASE_ADOPTION_URL   default for --adoption-url
 *   E2E_STORAGE_<PEER>     per-peer storage base url (the a2o mesh convention);
 *                          consulted for the acting peer before the port
 *                          derivation below
 *
 * `status` ALWAYS emits one JSON object to stdout (rail #5): a
 * `{ channelId, checkedAt, peers: [...] }` row per configured peer, with
 * `reachable: false` (connect/call failure) distinct from `reachable: true,
 * tier: "none"` (conductor answered, no election exists yet) — unreachable
 * is never read as absent.
 */
import { readFileSync, statSync } from 'node:fs';

import {
  AdminWebsocket,
  AppWebsocket,
  encodeHashToBase64,
  decodeHashFromBase64,
} from '@holochain/client';

const APP_ID = 'elohim';
const ROLE = 'lamad'; // the role name carrying the content_store cell — same as the oracle
const ZOME = 'content_store';
const DEFAULT_TIMEOUT_MS = 5_000;

// Local household mesh port scheme (app/elohim-app/scripts/hc-mesh.sh):
// admin_port(i) = 4444 + 10i, app_port(i) = 4445 + 10i. i=0 matthew, 1 jessica, 2 james.
// Same convention version-matrix.ts uses for its --conductors CSV.
const DEFAULT_CONDUCTORS = 'matthew=4444:4445,jessica=4454:4455,james=4464:4465';

/**
 * Storage HTTP port for peer index i on the local household mesh
 * (app/elohim-app/scripts/hc-mesh.sh: `storage http 8090+i`, the same
 * derivation `scripts/release-attestation-probe.ts`'s PEER_A/B/C defaults
 * carry). Used only as the LAST fallback for `--adoption-url`, after the
 * flag itself, `RELEASE_ADOPTION_URL`, and `E2E_STORAGE_<PEER>`.
 */
const DEFAULT_STORAGE_HTTP_BASE_PORT = 8090;

const ADOPTION_PATH = '/admin/adoption';

const REACH_LEVELS = [
  'private',
  'self',
  'intimate',
  'trusted',
  'familiar',
  'community',
  'public',
  'commons',
];

const USAGE = `Usage: release-ceremony.ts <verb> ... [options]

Verbs:
  channel create <channelId> [--reach <tier>] [--discipline <json|path>]
  publish <manifest.json> [--adoption-url <storage base url>]
  promote <channelId> <releaseCid> [--delegation <file>]
  revert <channelId> <priorReleaseCid | manifest.json> [--delegation <file>]
  status <channelId>

Options:
  --as <peerName>       act as this configured peer (default: first configured, "matthew")
  --conductors <csv>    name=admin:app,... (default: ${DEFAULT_CONDUCTORS})
  --timeout <ms>        per-peer connect timeout in ms (default: ${DEFAULT_TIMEOUT_MS})
  --adoption-url <url>  publish only: storage base url whose GET ${ADOPTION_PATH} proves the
                        acting peer has ADOPTED the channel's earned head
                        (default: RELEASE_ADOPTION_URL, else E2E_STORAGE_<PEER>, else
                        http://127.0.0.1:<${DEFAULT_STORAGE_HTTP_BASE_PORT}+peer index>)

Environment:
  PEER_CONDUCTOR_PORTS  conductor CSV used when --conductors is absent
  RELEASE_ADOPTION_URL  default for --adoption-url
  E2E_STORAGE_<PEER>    per-peer storage base url (a2o mesh convention)`;

interface PeerConfig {
  name: string;
  admin: number;
  app: number;
}

type Flags = Record<string, string>;

function parseFlags(argv: string[]): { positionals: string[]; flags: Flags } {
  const positionals: string[] = [];
  const flags: Flags = {};
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a.startsWith('--')) {
      const key = a.slice(2);
      const next = argv[i + 1];
      // Real runtime guard: argv[i+1] can be out-of-bounds undefined at runtime even
      // though TS types it as `string` here (no noUncheckedIndexedAccess) — not a
      // type-shaped bug.
      // eslint-disable-next-line sonarjs/different-types-comparison
      if (next !== undefined && !next.startsWith('--')) {
        flags[key] = next;
        i++;
      } else {
        flags[key] = 'true';
      }
    } else {
      positionals.push(a);
    }
  }
  return { positionals, flags };
}

function parseConductors(csv: string): PeerConfig[] {
  return csv
    .split(',')
    .map(s => s.trim())
    .filter(Boolean)
    .map(entry => {
      const [name, ports] = entry.split('=');
      const [adminStr, appStr] = (ports ?? '').split(':');
      const admin = Number(adminStr);
      const app = Number(appStr);
      if (!name || !Number.isInteger(admin) || !Number.isInteger(app)) {
        throw new Error(`conductor entry must be name=admin:app, got: ${entry}`);
      }
      return { name, admin, app };
    });
}

function resolveActingPeer(flags: Flags, peers: PeerConfig[]): PeerConfig {
  const name = flags.as ?? peers[0]?.name;
  const peer = peers.find(p => p.name === name);
  if (!peer) {
    throw new Error(
      `--as ${name}: no such configured peer (have: ${peers.map(p => p.name).join(', ')})`
    );
  }
  return peer;
}

function withTimeout<T>(p: Promise<T>, ms: number, label: string): Promise<T> {
  return new Promise((resolve, reject) => {
    const t = setTimeout(() => reject(new Error(`timeout after ${ms}ms: ${label}`)), ms);
    p.then(
      v => {
        clearTimeout(t);
        resolve(v);
      },
      e => {
        clearTimeout(t);
        reject(e);
      }
    );
  });
}

/**
 * The composed rail (see module doc): admin connect → resolve the
 * provisioned `lamad` cell → authorize signing credentials → app connect →
 * a bound `call(fn_name, payload)` helper. Shape copied from
 * `carried-election-mesh-proof.ts`'s `conductor()`, parameterized by peer
 * name/ports and wrapped with a connect timeout for multi-peer honesty.
 */
async function conductor(name: string, adminPort: number, appPort: number, timeoutMs: number) {
  const wsOpts: any = { origin: APP_ID };
  const admin = await withTimeout(
    AdminWebsocket.connect({
      url: new URL(`ws://127.0.0.1:${adminPort}`),
      wsClientOptions: wsOpts,
    }),
    timeoutMs,
    `admin connect ${name}:${adminPort}`
  );
  const apps = await admin.listApps({});
  const app = apps.find((a: any) => a.installed_app_id === APP_ID) ?? apps[0];
  if (!app) {
    throw new Error(
      `${name}: no installed app on adminPort=${adminPort} (saw: ${apps.map((a: any) => a.installed_app_id).join(',')})`
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
    throw new Error(
      `${name}: no '${ROLE}' cell provisioned (roles: ${[...cells.keys()].join(',')})`
    );
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
    `app connect ${name}:${appPort}`
  );
  const call = (fn_name: string, payload: any) =>
    appWs.callZome({ cell_id: lamad, zome_name: ZOME, fn_name, payload });
  return { name, admin, appWs, call, agent: encodeHashToBase64(lamad[1]) };
}

function toB64(v: unknown): string {
  if (typeof v === 'string') return v;
  if (v instanceof Uint8Array) return encodeHashToBase64(v);
  if (v && typeof v === 'object' && 'data' in (v as any)) {
    try {
      return encodeHashToBase64(new Uint8Array((v as any).data));
    } catch {
      /* fall through */
    }
  }
  return String(v);
}

function microsToIso(ts: unknown): string | null {
  if (ts === null || ts === undefined) return null;
  if (typeof ts === 'number') return new Date(ts / 1000).toISOString();
  if (typeof ts === 'bigint') return new Date(Number(ts) / 1000).toISOString();
  return null;
}

function assertReach(reach: string) {
  if (!REACH_LEVELS.includes(reach)) {
    throw new Error(`--reach ${reach}: not a protocol reach level (${REACH_LEVELS.join(', ')})`);
  }
}

function warnUnlessChannelIdConvention(channelId: string) {
  // spec §3: runtime:<artifact-class>:<network>:<channel-name>. Soft warning
  // only — the script does not know the full artifact-class/network vocab
  // and must not block a legitimate id it can't classify.
  if (!/^runtime:[^:]+:[^:]+:[^:]+$/.test(channelId)) {
    console.error(
      `warning: channelId '${channelId}' does not match the spec §3 convention ` +
        `runtime:<artifact-class>:<network>:<channel-name>`
    );
  }
}

function parseJsonArg(value: string | undefined): Record<string, unknown> {
  if (!value) return {};
  try {
    return JSON.parse(value);
  } catch {
    /* not inline JSON — try as a file path below */
  }
  try {
    return JSON.parse(readFileSync(value, 'utf8'));
  } catch (e) {
    throw new Error(
      `--discipline: not valid inline JSON nor a readable JSON file: ${value} (${e})`
    );
  }
}

/** Optional arm-2 authority: a device-ceremony.ts-shaped delegation file
 * ({ grantor, delegate, scope, validUntil, signature }, all base64) decoded
 * into the zome's HeadDelegation wire shape. Not exercised by the DoD's
 * required run (which uses the root-author arm); supported for completeness
 * and documented as such — see the atom's Implementation notes. */
function loadDelegation(path: string | undefined): any {
  if (!path) return null;
  const json = JSON.parse(readFileSync(path, 'utf8'));
  return {
    payload: {
      grantor: decodeHashFromBase64(json.grantor),
      delegate: decodeHashFromBase64(json.delegate),
      scope: json.scope,
      valid_until: json.validUntil,
    },
    signature: new Uint8Array(Buffer.from(json.signature, 'base64')),
  };
}

/**
 * Where to read the acting peer's OWN adoption receipt from. Resolution
 * order, first hit wins:
 *
 *   1. `--adoption-url <url>`
 *   2. `RELEASE_ADOPTION_URL`
 *   3. `E2E_STORAGE_<PEERNAME>` — the a2o household-mesh convention the
 *      feature's Background already resolves peers from.
 *   4. `http://127.0.0.1:<8090 + the acting peer's index in --conductors>` —
 *      hc-mesh.sh's own port scheme. The conductor CSV is the only peer
 *      identity this script has ever known (`peers` is built from it), so
 *      the derivation stays keyed to the SAME list rather than inventing a
 *      second peer registry.
 */
function resolveAdoptionUrl(flags: Flags, peers: PeerConfig[], actingPeer: PeerConfig): string {
  const explicit = flags['adoption-url'];
  if (explicit && explicit !== 'true') return explicit.replace(/\/$/, '');
  const fromEnv =
    process.env.RELEASE_ADOPTION_URL ?? process.env[`E2E_STORAGE_${actingPeer.name.toUpperCase()}`];
  if (fromEnv) return fromEnv.replace(/\/$/, '');
  const index = Math.max(
    0,
    peers.findIndex(p => p.name === actingPeer.name)
  );
  return `http://127.0.0.1:${DEFAULT_STORAGE_HTTP_BASE_PORT + index}`;
}

interface AdoptionChannelRowLite {
  channelId: string;
  appliedRelease: { cid: string } | null;
  resolvedHead: { cid: string; tier: string } | null;
}

interface AdoptionReadResult {
  /** The row's `appliedRelease.cid`, or null when the peer holds no row / has applied nothing. */
  appliedCid: string | null;
  /** Non-null when the surface itself could not be read — never conflated with "applied nothing". */
  error: string | null;
}

/**
 * Reads ONE channel's row off a peer's `GET /admin/adoption`. An unreadable
 * surface is reported as an `error`, never as "this peer has adopted
 * nothing" — the same unreachable-≠-absent rail `status` holds for the
 * conductor read.
 */
async function readAdoptedRelease(
  adoptionUrl: string,
  channelId: string,
  timeoutMs: number
): Promise<AdoptionReadResult> {
  let response: Response;
  try {
    response = await fetch(`${adoptionUrl}${ADOPTION_PATH}`, {
      signal: AbortSignal.timeout(Math.max(timeoutMs, 5_000)),
    });
  } catch (e) {
    return {
      appliedCid: null,
      error: `could not reach ${adoptionUrl}${ADOPTION_PATH}: ${String(e).slice(0, 200)}`,
    };
  }
  if (!response.ok) {
    return {
      appliedCid: null,
      error: `GET ${adoptionUrl}${ADOPTION_PATH} returned ${response.status}`,
    };
  }
  let body: { channels?: AdoptionChannelRowLite[] };
  try {
    body = (await response.json()) as { channels?: AdoptionChannelRowLite[] };
  } catch (e) {
    return {
      appliedCid: null,
      error: `GET ${adoptionUrl}${ADOPTION_PATH} returned unparseable JSON: ${String(e).slice(0, 200)}`,
    };
  }
  const row = (body.channels ?? []).find(r => r.channelId === channelId);
  return { appliedCid: row?.appliedRelease?.cid ?? null, error: null };
}

async function resolveElectionOnPeer(peer: PeerConfig, channelId: string, timeoutMs: number) {
  try {
    const conn = await conductor(peer.name, peer.admin, peer.app, timeoutMs);
    try {
      const election: any = await conn.call('resolve_canonical_election', channelId);
      return {
        peer: peer.name,
        reachable: true,
        tier: election ? (election.canonical_earned ? 'earned' : 'staging') : 'none',
        headActionHash: election ? toB64(election.winner_target) : null,
        declaredAt: election ? microsToIso(election.canonical_declared_at) : null,
      };
    } finally {
      try {
        await (conn.appWs.client as unknown as { close(): unknown }).close();
      } catch {
        /* best-effort */
      }
      try {
        await conn.admin.client.close();
      } catch {
        /* best-effort */
      }
    }
  } catch (e) {
    return { peer: peer.name, reachable: false, error: String(e).slice(0, 300) };
  }
}

// =============================================================================
// Verbs
// =============================================================================

async function cmdChannelCreate(
  channelId: string,
  flags: Flags,
  peers: PeerConfig[],
  timeoutMs: number
) {
  warnUnlessChannelIdConvention(channelId);
  const reach = flags.reach ?? 'commons';
  assertReach(reach);
  const discipline = parseJsonArg(flags.discipline);

  const actingPeer = resolveActingPeer(flags, peers);
  const conn = await conductor(actingPeer.name, actingPeer.admin, actingPeer.app, timeoutMs);
  console.log(`${actingPeer.name} agent (channel root author): ${conn.agent}`);

  const metadata = {
    kind: 'release-channel',
    channelId,
    reach,
    discipline,
    createdAt: new Date().toISOString(),
  };
  const payload = {
    id: channelId,
    content_type: 'concept',
    title: `Release channel: ${channelId}`,
    description: `Runtime release channel (${channelId}) — spec §3 channel root.`,
    content: `# ${channelId}\n\nRelease channel root. See metadata for reach and adoption discipline.`,
    content_format: 'markdown',
    tags: ['release-channel'],
    related_node_ids: [],
    reach,
    metadata_json: JSON.stringify(metadata),
  };

  let created: any;
  try {
    created = await conn.call('create_content', payload);
  } catch (e) {
    throw new Error(
      `channel create '${channelId}' failed on ${actingPeer.name}: ${String(e).slice(0, 400)}`
    );
  }
  const result = {
    verb: 'channel create',
    channelId,
    actingPeer: actingPeer.name,
    actionHash: toB64(created.action_hash),
    entryHash: toB64(created.entry_hash),
    reach,
    discipline,
  };
  console.log(JSON.stringify(result, null, 2));
  process.exit(0);
}

interface AuthoredVersion {
  conn: Awaited<ReturnType<typeof conductor>>;
  channelId: string;
  releaseCid: string;
  actingPeer: PeerConfig;
}

/**
 * The `publish`-over-an-EARNED-head decision (module doc
 * "Adopt-before-author"). Returns normally when the publish is ADMITTED —
 * the acting peer has ADOPTED the earned head AND the manifest names that
 * head as its lineage parent — and throws the typed refusal otherwise.
 *
 * A channel is long-lived: refusing every publish over an earned head is
 * what forced a fresh channel per release, so this function is the seam
 * Station 9 closes. It refuses on the SAME two facts the story names, and
 * says which one is missing rather than making the steward guess.
 */
async function assertAdmissibleOverEarnedHead(
  channelId: string,
  earnedCid: string,
  manifest: any,
  flags: Flags,
  peers: PeerConfig[],
  actingPeer: PeerConfig,
  timeoutMs: number
): Promise<void> {
  const adoptionUrl = resolveAdoptionUrl(flags, peers, actingPeer);
  const adoption = await readAdoptedRelease(adoptionUrl, channelId, timeoutMs);
  const adopted = adoption.error === null && adoption.appliedCid === earnedCid;

  const declaredParent: unknown = manifest?.envelope?.lineageParentCid;
  const lineageOk = typeof declaredParent === 'string' && declaredParent === earnedCid;

  if (adopted && lineageOk) {
    console.error(
      `publish ADMITTED over earned head ${earnedCid} on channel '${channelId}': ` +
        `${actingPeer.name}'s own runtime reports it applied (${adoptionUrl}${ADOPTION_PATH}) and ` +
        `the manifest declares envelope.lineageParentCid=${earnedCid} — the candidate is declared ` +
        `STAGING beneath the standing earned head, which only the promotion ceremony can move.`
    );
    return;
  }

  const notAdoptedDetail = adoption.error
    ? `${actingPeer.name}'s adoption receipt could not be read (${adoption.error})`
    : `${actingPeer.name}'s runtime reports appliedRelease=${adoption.appliedCid ?? 'none'} on ` +
      `${adoptionUrl}${ADOPTION_PATH}`;

  if (!adopted && !lineageOk) {
    // Neither condition holds — the original, un-narrowed refusal, with both
    // typed codes appended so an automated caller can classify it.
    throw new Error(
      `publish refused: channel '${channelId}' already has an EARNED head ` +
        `(${earnedCid}). Publishing a fresh staging candidate over ` +
        `an earned head would crown this peer's own commit without adopting the standing ` +
        `authority declaration — promote/revert are the earned-tier verbs; adopt/resolve first ` +
        `if this peer genuinely intends to supersede it. ` +
        `[not_adopted: ${notAdoptedDetail}] ` +
        `[lineage_parent_mismatch: the manifest declares envelope.lineageParentCid=` +
        `${JSON.stringify(declaredParent ?? null)}]`
    );
  }

  if (!adopted) {
    throw new Error(
      `publish refused (not_adopted): channel '${channelId}' has EARNED head ${earnedCid}, and a ` +
        `steward cannot push what they have not themselves adopted — ${notAdoptedDetail}.`
    );
  }

  throw new Error(
    `publish refused (lineage_parent_mismatch): channel '${channelId}' has EARNED head ${earnedCid}, ` +
      `so the candidate must name that head as the release it builds on, but the manifest declares ` +
      `envelope.lineageParentCid=${JSON.stringify(declaredParent ?? null)} — repackage with ` +
      `--lineage-parent ${earnedCid}.`
  );
}

/**
 * Shared "author a version from a release manifest" path: read + duck-type
 * validate the manifest, connect as the acting peer, run the
 * adopt-before-author pre-flight, then call `update_content` with the
 * release manifest riding `metadata_json` (spec §5) — exactly the steps
 * `publish` always ran. `refuseIfEarned` is the ONLY behavioral fork:
 * `publish` (true) refuses locally when the channel already has an EARNED
 * head (see module doc "Adopt-before-author"); `revert` (false) is itself
 * the authority moving that head, so it logs the fact and proceeds instead
 * of refusing.
 */
async function authorVersionFromManifest(
  manifestPath: string,
  flags: Flags,
  peers: PeerConfig[],
  timeoutMs: number,
  refuseIfEarned: boolean
): Promise<AuthoredVersion> {
  const label = refuseIfEarned ? 'publish' : 'revert';
  const manifestRaw = readFileSync(manifestPath, 'utf8');
  const manifest = JSON.parse(manifestRaw);
  const channelId: string | undefined = manifest.channelId;
  if (!channelId || typeof channelId !== 'string') {
    throw new Error(
      `${label}: manifest at ${manifestPath} has no string 'channelId' field. ` +
        `NOTE: task-release-manifest-schema-packager (T1) had not landed a schema/packager ` +
        `as of this driver's authoring, so this manifest is read duck-typed (channelId + ` +
        `whatever else it carries), not schema-validated — see the atom's Implementation notes.`
    );
  }

  const actingPeer = resolveActingPeer(flags, peers);
  const conn = await conductor(actingPeer.name, actingPeer.admin, actingPeer.app, timeoutMs);
  console.log(`${actingPeer.name} agent (publisher): ${conn.agent}`);

  // Adopt-before-author pre-flight (LOCAL-DHT arm + the acting peer's own
  // adoption receipt; see module doc "Adopt-before-author").
  const currentElection: any = await conn.call('resolve_canonical_election', channelId);
  if (currentElection?.canonical_earned) {
    const earnedCid = toB64(currentElection.winner_target);
    if (refuseIfEarned) {
      await assertAdmissibleOverEarnedHead(
        channelId,
        earnedCid,
        manifest,
        flags,
        peers,
        actingPeer,
        timeoutMs
      );
    } else {
      console.error(
        `revert: re-electing over earned head ${earnedCid} — this ` +
          `declarer is the authority moving the earned head, not a staging candidate`
      );
    }
  }

  const releaseMetadata = {
    kind: 'release-manifest',
    publishedAt: new Date().toISOString(),
    manifest,
  };
  const updatePayload: Record<string, unknown> = {
    id: channelId,
    metadata_json: JSON.stringify(releaseMetadata),
  };
  if (typeof manifest.title === 'string') updatePayload.title = manifest.title;
  if (typeof manifest.description === 'string') updatePayload.description = manifest.description;

  let updated: any;
  try {
    updated = await conn.call('update_content', updatePayload);
  } catch (e) {
    const msg = String(e);
    if (msg.includes('no Content entry found for id')) {
      throw new Error(
        `${label}: channel '${channelId}' not found on ${actingPeer.name} — run ` +
          `'channel create ${channelId}' first. (${msg.slice(0, 200)})`
      );
    }
    throw new Error(`${label}: update_content failed on ${actingPeer.name}: ${msg.slice(0, 400)}`);
  }
  const releaseCid = toB64(updated.action_hash);
  return { conn, channelId, releaseCid, actingPeer };
}

async function cmdPublish(
  manifestPath: string,
  flags: Flags,
  peers: PeerConfig[],
  timeoutMs: number
) {
  const { conn, channelId, releaseCid, actingPeer } = await authorVersionFromManifest(
    manifestPath,
    flags,
    peers,
    timeoutMs,
    true
  );

  const declarePayload = {
    id: channelId,
    head_action_hash: releaseCid,
    carried_record: null,
    adopt_before_author: false,
    delegation: null,
  };
  const declared: any = await conn.call('declare_canonical_content_head', declarePayload);

  const result = {
    verb: 'publish',
    channelId,
    actingPeer: actingPeer.name,
    releaseCid,
    tier: 'staging',
    canonical: declared.canonical,
    canonicalDeclaredAt: microsToIso(declared.canonical_declared_at),
  };
  console.log(JSON.stringify(result, null, 2));
  process.exit(0);
}

async function declareEarned(
  verb: 'promote' | 'revert',
  channelId: string,
  releaseCid: string,
  flags: Flags,
  peers: PeerConfig[],
  timeoutMs: number
) {
  const actingPeer = resolveActingPeer(flags, peers);
  const conn = await conductor(actingPeer.name, actingPeer.admin, actingPeer.app, timeoutMs);
  console.log(`${actingPeer.name} agent (declarer): ${conn.agent}`);

  const delegation = loadDelegation(flags.delegation);
  const declarePayload = {
    id: channelId,
    head_action_hash: releaseCid,
    carried_record: null,
    adopt_before_author: false,
    delegation,
  };

  let declared: any;
  try {
    declared = await conn.call('declare_earned_canonical_head', declarePayload);
  } catch (e) {
    // The DoD's negative control: an unauthorized caller (no root-author
    // match, no delegation, no bootstrap steward) is REFUSED — print the
    // refusal in full rather than swallow it.
    console.log(
      JSON.stringify(
        {
          verb,
          channelId,
          releaseCid,
          actingPeer: actingPeer.name,
          refused: true,
          error: String(e).slice(0, 500),
        },
        null,
        2
      )
    );
    process.exit(1);
  }

  const result: Record<string, unknown> = {
    verb,
    channelId,
    releaseCid,
    actingPeer: actingPeer.name,
    tier: 'earned',
    canonical: declared.canonical,
    canonicalDeclaredAt: microsToIso(declared.canonical_declared_at),
  };

  // Never assert convergence from the declaring peer alone — resolve on a
  // SECOND configured peer (mandatory for revert per atom scope; done for
  // promote too, since it's the identical honesty question at the identical
  // cost). Gossip propagation is the ~2 min class (spec §10) — the second
  // peer may legitimately still show the prior state; report what it says,
  // do not poll-until-match and call that "proof."
  const secondPeer = peers.find(p => p.name !== actingPeer.name);
  if (secondPeer) {
    const secondAnswer = await resolveElectionOnPeer(secondPeer, channelId, timeoutMs);
    result.secondPeerVerification = secondAnswer;
  } else {
    result.secondPeerVerification = null;
    console.error(
      'warning: only one peer configured — cannot verify convergence from a second peer'
    );
  }

  console.log(JSON.stringify(result, null, 2));
  process.exit(0);
}

function isManifestFile(value: string): boolean {
  try {
    return statSync(value).isFile();
  } catch {
    return false;
  }
}

/**
 * `revert <channelId> <priorReleaseCid | manifest.json>`. When the second
 * positional is a readable file, it is a release manifest for the bytes
 * being reverted TO (see module doc "Adopt-before-author"): author it as a
 * new version via `authorVersionFromManifest` (bypassing the earned-head
 * refusal — this declarer IS the authority moving the earned head), then
 * declare that new version's action hash EARNED via the existing
 * `declareEarned`, skipping the staging tier entirely. Otherwise this is the
 * original form — declare the given action hash EARNED directly.
 */
async function cmdRevert(
  channelId: string,
  priorReleaseCidOrManifest: string,
  flags: Flags,
  peers: PeerConfig[],
  timeoutMs: number
) {
  if (!isManifestFile(priorReleaseCidOrManifest)) {
    await declareEarned('revert', channelId, priorReleaseCidOrManifest, flags, peers, timeoutMs);
    return;
  }

  const manifestPath = priorReleaseCidOrManifest;
  const authored = await authorVersionFromManifest(manifestPath, flags, peers, timeoutMs, false);
  if (authored.channelId !== channelId) {
    throw new Error(
      `revert: channelId mismatch — command line said '${channelId}' but manifest ` +
        `'${manifestPath}' declares channelId '${authored.channelId}'`
    );
  }

  // Same fields `publish` prints for a freshly authored version (releaseCid
  // greppable); tier/canonical are omitted because revert never calls
  // declare_canonical_content_head — it goes straight to the earned tier.
  console.log(
    JSON.stringify(
      {
        verb: 'publish',
        channelId,
        actingPeer: authored.actingPeer.name,
        releaseCid: authored.releaseCid,
      },
      null,
      2
    )
  );

  await declareEarned('revert', channelId, authored.releaseCid, flags, peers, timeoutMs);
}

async function cmdStatus(channelId: string, _flags: Flags, peers: PeerConfig[], timeoutMs: number) {
  const rows = await Promise.all(peers.map(p => resolveElectionOnPeer(p, channelId, timeoutMs)));
  const report = {
    channelId,
    checkedAt: new Date().toISOString(),
    peers: rows,
  };
  console.log(JSON.stringify(report, null, 2));
  const anyReachable = rows.some(r => r.reachable);
  process.exit(anyReachable ? 0 : 3);
}

// =============================================================================
// Entry point
// =============================================================================

async function main() {
  const argv = process.argv.slice(2);
  if (argv.length === 0 || argv[0] === '-h' || argv[0] === '--help') {
    console.log(USAGE);
    process.exit(argv.length === 0 ? 2 : 0);
  }

  const verb = argv[0];
  let rest = argv.slice(1);
  let subverb: string | undefined;
  if (verb === 'channel') {
    subverb = rest[0];
    rest = rest.slice(1);
  }

  const { positionals, flags } = parseFlags(rest);
  const timeoutMs = flags.timeout ? Number(flags.timeout) : DEFAULT_TIMEOUT_MS;
  const conductorCsv = flags.conductors ?? process.env.PEER_CONDUCTOR_PORTS ?? DEFAULT_CONDUCTORS;
  const peers = parseConductors(conductorCsv);

  if (verb === 'channel' && subverb === 'create') {
    const [channelId] = positionals;
    if (!channelId)
      throw new Error(
        'usage: channel create <channelId> [--reach <tier>] [--discipline <json|path>]'
      );
    await cmdChannelCreate(channelId, flags, peers, timeoutMs);
  } else if (verb === 'publish') {
    const [manifestPath] = positionals;
    if (!manifestPath) throw new Error('usage: publish <manifest.json>');
    await cmdPublish(manifestPath, flags, peers, timeoutMs);
  } else if (verb === 'promote') {
    const [channelId, releaseCid] = positionals;
    if (!channelId || !releaseCid) throw new Error('usage: promote <channelId> <releaseCid>');
    await declareEarned('promote', channelId, releaseCid, flags, peers, timeoutMs);
  } else if (verb === 'revert') {
    const [channelId, priorReleaseCidOrManifest] = positionals;
    if (!channelId || !priorReleaseCidOrManifest)
      throw new Error('usage: revert <channelId> <priorReleaseCid | manifest.json>');
    await cmdRevert(channelId, priorReleaseCidOrManifest, flags, peers, timeoutMs);
  } else if (verb === 'status') {
    const [channelId] = positionals;
    if (!channelId) throw new Error('usage: status <channelId>');
    await cmdStatus(channelId, flags, peers, timeoutMs);
  } else {
    console.error(USAGE);
    process.exit(2);
  }
}

main().catch(e => {
  console.error(String(e?.stack ?? e).slice(0, 2000));
  process.exit(1);
});
