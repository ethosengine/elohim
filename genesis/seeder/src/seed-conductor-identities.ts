/**
 * Seed Conductor Identities — create Human profiles directly on node/device conductors.
 *
 * Node and device humans own their own conductors (StatefulSets in K8s). Their
 * identity must be established directly on their conductor BEFORE they register
 * with doorway for ingress/recovery. This script models that real-world flow.
 *
 * Runs BEFORE seed-humans.ts (which registers with doorway).
 *
 * Environment variables:
 *   CONDUCTOR_URLS               Comma-separated conductor app WebSocket URLs.
 *                                Two entry forms, freely mixed:
 *                                  - hostname (today's cluster form), affine
 *                                    when it follows `elohim-<name>-<env>`:
 *                                    ws://elohim-adam-alpha:4445,ws://elohim-eve-alpha:4445
 *                                  - named `name=url` (same `name=value`
 *                                    convention as PEER_STORAGE_URLS — see
 *                                    peer-id.ts's parseNamedCsv), for hosts
 *                                    with no name-bearing hostname (local
 *                                    `just mesh`, all on localhost):
 *                                    matthew=ws://localhost:4445,jessica=ws://localhost:4455,james=ws://localhost:4465
 *                                A named entry is name-affine for that human
 *                                exactly as a hostname match is. The list is
 *                                name-affine as a whole the moment ANY entry
 *                                is named or hostname-affine — see
 *                                urlsAreNameAffine / resolveCandidateUrls.
 *   INSTALLED_APP_ID             Holochain app ID prefix (default: elohim)
 *   CONDUCTOR_CONNECT_TIMEOUT_MS Per-connect timeout (default: 10000) — fail fast on
 *                                unreachable conductors so the stage's catchError can
 *                                soft-land before the pipeline's global timeout fires
 *
 * Output:
 *   [+] Created  — Human profile was just created on the conductor
 *   [=] Exists   — THIS human's profile already present (idempotent)
 *   [C] Conflict — the human's own conductor already embodies a DIFFERENT
 *                  human id (one agent = one Human; needs operator attention)
 *   [-] Skipped  — no conductor deployed for this human (soft, not a failure)
 *   [X] Failed   — could not connect or create (see error)
 *
 * Conductor affinity: a human seeds ONLY onto their own pod, resolved by the
 * `elohim-<name>-<env>` service naming convention — the same rule
 * genesis/Jenkinsfile uses to build CONDUCTOR_URLS — OR by an explicit
 * `name=url` CONDUCTOR_URLS entry (see above). First-reachable-wins is how
 * genesis #1119 got the founder FATAL: every human probed matthew's
 * conductor first, saw SOME human existed (the old exists-check never compared
 * ids), and reported a no-op "exists" while matthew himself (doorway phase)
 * was filtered out entirely.
 *
 * A second-order variant of the same defect hit the local `just mesh`: its
 * CONDUCTOR_URLS (`ws://localhost:4445,ws://localhost:4455,ws://localhost:4465`)
 * carries no `elohim-<name>-` hostname segment at all, so the WHOLE list read
 * as non-affine and the legacy first-reachable-wins walk applied to every
 * human — including ADAM, who has no conductor in the mesh at all. ADAM
 * walked the list, found james's pod (4465) unclaimed, and squatted on it;
 * every subsequent human (jessica, james, eve, pete, terrance) then hit that
 * pod first and reported `[C] Conflict` against a name that wasn't theirs.
 * The fix is the named `name=url` form: once ANY entry in CONDUCTOR_URLS
 * carries a name, the list is name-affine as a whole (`urlsAreNameAffine`),
 * and a human with no matching name/hostname resolves to ZERO candidates
 * (`resolveCandidateUrls`) — `[-] Skipped`, never a cast onto someone else's
 * conductor. The legacy first-reachable-wins walk survives only for a FULLY
 * unnamed, non-hostname-affine list (a loopback mesh with no name=url
 * entries at all) — main() prints a loud warning when that path is taken,
 * since the cast is then genuinely arbitrary.
 *
 * Deriving a name from PEER_STORAGE_URLS by index was considered (matching
 * CONDUCTOR_URLS[i] to the i-th PEER_STORAGE_URLS entry) and rejected: it's
 * a coincidence of list ORDER, not a fact the storage surface asserts. No
 * read-only elohim-storage route exposes which Holochain conductor a storage
 * pod is wired to (`HOLOCHAIN_APP_URL`) — `GET /auth/me`'s `conductorEndpoint`
 * is the storage pod's OWN libp2p peer id / bind address (identity.rs /
 * http.rs `handle_auth_me`), and `GET /health`'s `conductor.mode` is only
 * `embedded`/`external`, never a URL. `GET /p2p/status` is the same libp2p
 * identity, again not a Holochain conductor URL. So this script does NOT
 * guess a conductor's owner from PEER_STORAGE_URLS index alignment — pass
 * `name=url` CONDUCTOR_URLS entries explicitly instead.
 *
 *   seed-results-conductor-identities.json — structured per-human results,
 *     written next to the script. The Jenkinsfile reads this to emit a
 *     partial-vs-total UNSTABLE message and to archive the artifact for
 *     orchestrator-level reconciliation (Path C).
 *
 * Exit codes (partial-readiness aware — aligns with the "seed whoever is
 * ready" architecture: partial-cluster operation is the steady state):
 *   0 — all targeted humans created or already exist
 *   2 — partial: at least one human seeded, at least one failed
 *   1 — total failure: ZERO humans seeded (or pre-flight error)
 */

import { readFileSync, existsSync, writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { AdminWebsocket, AppWebsocket, type AppInfo } from '@holochain/client';
import { parseNamedCsv } from './peer-id.js';

// Canonical artifact filename from build-artifacts.json — single source of
// truth across Groovy + TypeScript + JS. Resolved once at module load so
// the writeFileSync below cannot drift from what genesis/Jenkinsfile reads.
const ARTIFACTS_MANIFEST_PATH = resolve(
  dirname(fileURLToPath(import.meta.url)),
  '..', '..', 'orchestrator', 'build-artifacts.json',
);
const ARTIFACTS = JSON.parse(readFileSync(ARTIFACTS_MANIFEST_PATH, 'utf8'));
const SEED_RESULTS_FILE: string = ARTIFACTS.genesis.seedResultsConductorIdentities;

// =============================================================================
// Types
// =============================================================================

interface HumansJsonHuman {
  id: string;
  displayName: string;
  bio?: string;
  category: string;
  profileReach: string;
  affinities?: string[];
  agencyPhase?: string;
}

interface HumansJson {
  humans: HumansJsonHuman[];
}

interface CreateHumanInput {
  id: string;
  display_name: string;
  bio: string | null;
  affinities: string[];
  profile_reach: string;
  location: null;
}

interface HumanOutput {
  id: string;
  display_name: string;
}

type SeedResult = 'created' | 'exists' | 'conflict' | 'skipped' | 'failed';

interface ConductorResult {
  displayName: string;
  humanId: string;
  conductorUrl: string;
  result: SeedResult;
  error?: string;
}

// =============================================================================
// Holochain helpers
// =============================================================================

const CONNECT_TIMEOUT_MS = parseInt(
  process.env.CONDUCTOR_CONNECT_TIMEOUT_MS ?? '10000',
  10
);

async function withTimeout<T>(
  promise: Promise<T>,
  timeoutMs: number,
  label: string
): Promise<T> {
  let timer: NodeJS.Timeout | undefined;
  const timeout = new Promise<never>((_, reject) => {
    timer = setTimeout(
      () => reject(new Error(`${label} timed out after ${timeoutMs}ms`)),
      timeoutMs
    );
  });
  try {
    return await Promise.race([promise, timeout]);
  } finally {
    if (timer) clearTimeout(timer);
  }
}

/**
 * Derive admin WebSocket URL from app WebSocket URL.
 *
 * Convention (socat in K8s): admin port = app port - 1
 *   4445 (app) → 4444 (admin)
 */
function toAdminUrl(appUrl: string): string {
  const u = new URL(appUrl);
  const appPort = parseInt(u.port, 10);
  if (isNaN(appPort)) {
    throw new Error(`Cannot derive admin port from app URL: ${appUrl}`);
  }
  u.port = String(appPort - 1);
  return u.toString();
}

interface ConductorSession {
  appWs: AppWebsocket;
  cellId: [Uint8Array, Uint8Array];
  appInfo: AppInfo;
}

/**
 * Connect to a conductor and find an installed app that starts with the given prefix.
 * Returns null if no matching app is found (this conductor isn't for this human).
 */
async function connectToConductor(
  appUrl: string,
  appIdPrefix: string
): Promise<ConductorSession | null> {
  const adminUrl = toAdminUrl(appUrl);

  let adminWs: AdminWebsocket;
  try {
    adminWs = await withTimeout(
      AdminWebsocket.connect({
        url: new URL(adminUrl),
        wsClientOptions: { origin: 'http://localhost' },
      }),
      CONNECT_TIMEOUT_MS,
      `Admin connect ${adminUrl}`
    );
  } catch (err) {
    throw new Error(
      `Admin connect failed (${adminUrl}): ${err instanceof Error ? err.message : err}`
    );
  }

  try {
    const apps = await adminWs.listApps({});
    const matchingApp = apps.find(a => a.installed_app_id.startsWith(appIdPrefix));

    if (!matchingApp) {
      await adminWs.client.close();
      return null;
    }

    // Find the imagodei role's provisioned cell.
    // @holochain/client returns two possible cell formats depending on version:
    //   { type: "provisioned", value: { cell_id: [...] } }  — newer
    //   { provisioned: { cell_id: [...] } }                 — older
    const imagodeiCells = matchingApp.cell_info['imagodei'];
    if (!imagodeiCells || imagodeiCells.length === 0) {
      await adminWs.client.close();
      throw new Error(`App '${matchingApp.installed_app_id}' has no imagodei cells`);
    }

    let cellId: [Uint8Array, Uint8Array] | null = null;
    for (const cell of imagodeiCells as unknown[]) {
      const c = cell as Record<string, unknown>;
      if (c['type'] === 'provisioned' && c['value']) {
        cellId = (c['value'] as Record<string, unknown>)['cell_id'] as [Uint8Array, Uint8Array];
        break;
      } else if (c['provisioned']) {
        cellId = (c['provisioned'] as Record<string, unknown>)['cell_id'] as [Uint8Array, Uint8Array];
        break;
      }
    }

    if (!cellId) {
      await adminWs.client.close();
      throw new Error(
        `App '${matchingApp.installed_app_id}' imagodei cell is not provisioned`
      );
    }

    // Authorize signing credentials for this cell
    await adminWs.authorizeSigningCredentials(cellId);

    // Issue a short-lived app auth token
    const tokenResult = await adminWs.issueAppAuthenticationToken({
      installed_app_id: matchingApp.installed_app_id,
      single_use: true,
      expiry_seconds: 300,
    });

    await adminWs.client.close();

    // Connect to app interface
    const appWs = await withTimeout(
      AppWebsocket.connect({
        url: new URL(appUrl),
        token: tokenResult.token,
        wsClientOptions: { origin: 'http://localhost' },
      }),
      CONNECT_TIMEOUT_MS,
      `App connect ${appUrl}`
    );

    return { appWs, cellId, appInfo: matchingApp };
  } catch (err) {
    try {
      await adminWs.client.close();
    } catch {
      // ignore close errors
    }
    throw err;
  }
}

/**
 * Check whether the agent on this conductor already has a Human profile.
 */
async function getMyHuman(
  appWs: AppWebsocket,
  cellId: [Uint8Array, Uint8Array]
): Promise<HumanOutput | null> {
  const result = await appWs.callZome({
    cell_id: cellId,
    zome_name: 'imagodei',
    fn_name: 'get_my_human',
    payload: null,
  });
  return (result as HumanOutput | null) ?? null;
}

/**
 * Create a Human profile on the conductor for the given human.
 */
async function createHuman(
  appWs: AppWebsocket,
  cellId: [Uint8Array, Uint8Array],
  input: CreateHumanInput
): Promise<HumanOutput> {
  const result = await appWs.callZome({
    cell_id: cellId,
    zome_name: 'imagodei',
    fn_name: 'create_human',
    payload: input,
  });
  return result as HumanOutput;
}

// =============================================================================
// Conductor affinity + identity helpers (pure — exported for unit tests)
// =============================================================================

/** `human-matthew-manager` → `matthew` (genesis/Jenkinsfile's exact rule). */
export function humanShortName(humanId: string): string {
  return humanId.replace(/^human-/, '').split('-')[0];
}

/** One parsed CONDUCTOR_URLS entry — named (`name=url`) or bare (`url`). */
export interface ConductorUrlEntry {
  /** null for a bare/unnamed entry — hostname affinity is the only signal. */
  name: string | null;
  url: string;
}

/**
 * Parse CONDUCTOR_URLS. Reuses peer-id.ts's parseNamedCsv (the same
 * `name=value` convention PEER_STORAGE_URLS already uses) — a bare entry
 * with no `name=` prefix comes back with `name: null` and keeps today's
 * hostname-affinity behaviour untouched.
 */
export function parseConductorUrls(raw: string): ConductorUrlEntry[] {
  return parseNamedCsv(raw).map(({ name, value }) => ({ name, url: value }));
}

/** A URL follows `elohim-<name>-<env>` — capture group extracts `<name>`. */
const NAME_AFFINE_RE = /elohim-([a-z0-9]+)-/;

/** The short name a hostname-convention URL implies, or null if none. */
function hostnameAffineName(url: string): string | null {
  return NAME_AFFINE_RE.exec(url)?.[1] ?? null;
}

/**
 * The list is name-affine as a whole the moment ANY entry is explicitly
 * named OR follows the `elohim-<name>-<env>` hostname convention. This is a
 * property of the WHOLE list, not any one human — it is what
 * resolveCandidateUrls consults to decide whether an unmatched human is
 * skipped (affine list) or falls into the legacy first-reachable-wins walk
 * (fully unnamed, non-hostname list — local loopback mesh with no names).
 */
export function urlsAreNameAffine(entries: ConductorUrlEntry[]): boolean {
  return entries.some(e => e.name !== null || NAME_AFFINE_RE.test(e.url));
}

/**
 * Resolve the conductor URL that belongs to this human — an explicit
 * `name=url` entry matching `humanShortName(humanId)` exactly, or (when no
 * named entry matches) the `elohim-<name>-<env>` hostname convention. Named
 * entries are checked first so a mixed list can override a stale/absent
 * hostname match. Null when the human has no matching entry in the list.
 */
export function conductorUrlForHuman(
  humanId: string,
  entries: ConductorUrlEntry[]
): string | null {
  const name = humanShortName(humanId);
  const named = entries.find(e => e.name === name);
  if (named) return named.url;
  return entries.find(e => hostnameAffineName(e.url) === name)?.url ?? null;
}

/**
 * Resolve the ordered list of conductor URLs to try for a human.
 *
 * - Name-affine list (ANY entry named or hostname-affine): exactly the
 *   human's own URL, or none. NEVER the whole list — this is the local-mesh
 *   fix: a human with no matching entry (adam, on `just mesh`) must be
 *   skipped, not cast onto whichever OTHER human's conductor answers first.
 * - Fully unnamed, non-hostname-affine list (a bare loopback mesh with no
 *   names at all): the legacy first-reachable-wins walk — every URL, in
 *   order. main() warns loudly when this path is taken.
 */
export function resolveCandidateUrls(
  humanId: string,
  entries: ConductorUrlEntry[]
): string[] {
  if (urlsAreNameAffine(entries)) {
    const own = conductorUrlForHuman(humanId, entries);
    return own ? [own] : [];
  }
  return entries.map(e => e.url);
}

/**
 * Cast-table label for one CONDUCTOR_URLS entry — printed at the top of
 * main() so an operator can see the resolved binding before any connection
 * is attempted. Pure; exported for tests.
 */
export function describeConductorEntry(entry: ConductorUrlEntry): string {
  if (entry.name !== null) {
    return `${entry.name} → ${entry.url} (named)`;
  }
  const hostname = hostnameAffineName(entry.url);
  if (hostname !== null) {
    return `${hostname} → ${entry.url} (hostname-affine)`;
  }
  return `? → ${entry.url} (unnamed)`;
}

/**
 * Normalize get_my_human's return across zome/client shapes:
 * `{ id }` (HumanOutput) or `{ human: { id } }` (wrapped).
 */
export function extractHumanId(result: unknown): string | undefined {
  if (!result || typeof result !== 'object') return undefined;
  const r = result as Record<string, unknown>;
  if (typeof r['id'] === 'string') return r['id'];
  const nested = r['human'];
  if (nested && typeof nested === 'object') {
    const nid = (nested as Record<string, unknown>)['id'];
    if (typeof nid === 'string') return nid;
  }
  return undefined;
}

// =============================================================================
// Per-human seeding logic
// =============================================================================

/**
 * Seed one human onto THEIR conductor (name-affine — by explicit `name=url`
 * entry or the `elohim-<name>-<env>` hostname convention), or — when
 * CONDUCTOR_URLS has NO named or hostname-affine entries at all (a bare
 * loopback mesh) — walk the whole list with the id-aware exists check.
 */
async function seedHumanOnConductor(
  human: HumansJsonHuman,
  conductorUrls: ConductorUrlEntry[],
  appIdPrefix: string
): Promise<ConductorResult> {
  const base: Pick<ConductorResult, 'displayName' | 'humanId'> = {
    displayName: human.displayName,
    humanId: human.id,
  };

  // Name-affine targeting: this human's own pod, or nothing. The legacy
  // walk survives only when NOTHING in the list is named or hostname-affine
  // (a bare loopback mesh) — resolveCandidateUrls never falls back to the
  // full list once ANY entry elsewhere in the set is affine.
  const affine = urlsAreNameAffine(conductorUrls);
  const candidates = resolveCandidateUrls(human.id, conductorUrls);

  if (candidates.length === 0) {
    return {
      ...base,
      conductorUrl: '(none)',
      result: 'skipped',
      error: `no conductor deployed for this human (no elohim-${humanShortName(human.id)}-* or name=url entry in CONDUCTOR_URLS)`,
    };
  }

  let lastError: string | undefined;

  for (const conductorUrl of candidates) {
    let session: ConductorSession | null = null;

    try {
      session = await connectToConductor(conductorUrl, appIdPrefix);
    } catch (err) {
      // Connection failed — try next URL
      lastError = `Connect error (${conductorUrl}): ${err instanceof Error ? err.message : err}`;
      continue;
    }

    if (!session) {
      // No matching app on this conductor — try next
      continue;
    }

    const { appWs, cellId } = session;

    try {
      // Identity-aware exists check: "a human exists here" only counts when
      // it is THIS human. One agent = one Human — a different id on the
      // human's own conductor is a conflict, not an idempotent no-op.
      const existing = await getMyHuman(appWs, cellId);
      const existingId = extractHumanId(existing);

      if (existingId === human.id) {
        await appWs.client.close();
        return { ...base, conductorUrl, result: 'exists' };
      }

      if (existingId !== undefined) {
        await appWs.client.close();
        if (affine) {
          return {
            ...base,
            conductorUrl,
            result: 'conflict',
            error: `conductor already embodies '${existingId}' — expected '${human.id}'`,
          };
        }
        // Legacy walk: someone else's conductor — keep looking.
        lastError = `conductor at ${conductorUrl} embodies '${existingId}'`;
        continue;
      }

      // Create Human profile
      const input: CreateHumanInput = {
        id: human.id,
        display_name: human.displayName,
        bio: human.bio ?? null,
        affinities: human.affinities ?? [],
        profile_reach: human.profileReach,
        location: null,
      };

      await createHuman(appWs, cellId, input);
      await appWs.client.close();
      return { ...base, conductorUrl, result: 'created' };
    } catch (err) {
      try {
        await appWs.client.close();
      } catch {
        // ignore
      }
      lastError = err instanceof Error ? err.message : String(err);
      // Treat zome errors as failures for this human (don't try other conductors)
      return {
        ...base,
        conductorUrl,
        result: 'failed',
        error: lastError,
      };
    }
  }

  // No conductor matched this human
  return {
    ...base,
    conductorUrl: '(none)',
    result: 'failed',
    error: lastError ?? 'No conductor found with a matching app for this human',
  };
}

// =============================================================================
// Main
// =============================================================================

async function main(): Promise<void> {
  const conductorUrlsRaw = process.env.CONDUCTOR_URLS ?? '';
  const appIdPrefix = process.env.INSTALLED_APP_ID ?? 'elohim';

  const conductorUrls = parseConductorUrls(conductorUrlsRaw);

  // Load humans.json (generated artifact from genesis/data/humans/*.md)
  const __dirname = dirname(fileURLToPath(import.meta.url));
  const jsonPath = resolve(__dirname, '../../data/humans/humans.json');
  if (!existsSync(jsonPath)) {
    console.error(`ERROR: ${jsonPath} not found`);
    console.error('  humans.json is generated from markdown. Run:');
    console.error('  pnpm --filter holochain-seeder run build:data');
    process.exit(1);
  }
  const humansJson: HumansJson = JSON.parse(readFileSync(jsonPath, 'utf-8'));

  // Node, device, AND doorway humans: doorway humans (matthew) host a full
  // conductor too — excluding them is how the household founder was never
  // seeded and formation FATALed (genesis #1119).
  const targets = humansJson.humans.filter(
    h => h.agencyPhase === 'node' || h.agencyPhase === 'device' || h.agencyPhase === 'doorway'
  );

  console.log('=== Seed Conductor Identities ===\n');
  console.log(`App ID prefix: ${appIdPrefix}`);
  if (conductorUrls.length > 0) {
    console.log('Cast:');
    for (const entry of conductorUrls) {
      console.log(`  ${describeConductorEntry(entry)}`);
    }
  } else {
    console.log('Conductors:    (none — set CONDUCTOR_URLS)');
  }
  console.log(`Humans:        ${targets.length} node/device/doorway of ${humansJson.humans.length} total`);
  console.log('');

  if (conductorUrls.length === 0) {
    console.error('ERROR: CONDUCTOR_URLS is not set. Set it to comma-separated conductor app WebSocket URLs.');
    console.error('  Example: CONDUCTOR_URLS=ws://elohim-adam-alpha:4445,ws://elohim-eve-alpha:4445');
    console.error('  Or the named local-mesh form: CONDUCTOR_URLS=matthew=ws://localhost:4445,jessica=ws://localhost:4455,james=ws://localhost:4465');
    process.exit(1);
  }

  // A fully unnamed, non-hostname-affine list (bare loopback mesh) makes
  // resolveCandidateUrls fall back to first-reachable-wins for EVERY human —
  // genuinely arbitrary. Say so loudly instead of casting silently (the
  // ADAM-onto-james's-conductor defect this file's header documents).
  if (!urlsAreNameAffine(conductorUrls)) {
    console.warn(
      'WARN: loopback mesh without names — cast is first-reachable-wins; pass name=url entries.\n' +
        '  Every human below will walk this whole list in order and bind to whichever pod\n' +
        '  answers first, INCLUDING humans with no conductor of their own (they will be cast\n' +
        '  onto someone else\'s conductor and reported as a false [C] Conflict downstream).\n' +
        '  Fix: CONDUCTOR_URLS="matthew=ws://localhost:4445,jessica=ws://localhost:4455,james=ws://localhost:4465"'
    );
    console.log('');
  }

  // Sort: doorway (founder hosts) first, then node, then device
  const phaseOrder: Record<string, number> = { doorway: 0, node: 1, device: 2 };
  const sorted = [...targets].sort(
    (a, b) => (phaseOrder[a.agencyPhase ?? 'device'] ?? 99) - (phaseOrder[b.agencyPhase ?? 'device'] ?? 99)
  );

  const results: ConductorResult[] = [];

  for (const human of sorted) {
    const result = await seedHumanOnConductor(human, conductorUrls, appIdPrefix);
    results.push(result);

    const icon =
      result.result === 'created' ? '+'
      : result.result === 'exists' ? '='
      : result.result === 'conflict' ? 'C'
      : result.result === 'skipped' ? '-'
      : 'X';
    const phase = (human.agencyPhase ?? '').padEnd(6);
    const name = result.displayName.padEnd(16);
    const suffix = result.error ? ` (${result.error})` : '';
    console.log(`  [${icon}] ${name} ${phase} ${result.conductorUrl}${suffix}`);
  }

  const created = results.filter(r => r.result === 'created').length;
  const exists = results.filter(r => r.result === 'exists').length;
  const conflict = results.filter(r => r.result === 'conflict').length;
  const skipped = results.filter(r => r.result === 'skipped').length;
  const failed = results.filter(r => r.result === 'failed').length + conflict;
  const succeeded = created + exists;

  console.log('');
  console.log(
    `=== Results: ${created} created, ${exists} existing, ${conflict} conflict, ${skipped} skipped, ${failed - conflict} failed ===`
  );

  // Structured artifact for Jenkinsfile + orchestrator-level reconciliation.
  // Schema kept stable so Path C (stageAnnotations in actual-build-graph.json)
  // can consume this without re-parsing console output.
  const report = {
    schemaVersion: '1',
    seededAt: new Date().toISOString(),
    script: 'seed-conductor-identities',
    counts: { created, exists, conflict, skipped, failed, succeeded, total: results.length },
    partial: succeeded > 0 && failed > 0,
    allSucceeded: failed === 0,
    allFailed: succeeded === 0 && failed > 0,
    results: results.map(r => ({
      humanId: r.humanId,
      displayName: r.displayName,
      result: r.result,
      conductorUrl: r.conductorUrl,
      error: r.error ?? null,
    })),
  };
  try {
    writeFileSync(SEED_RESULTS_FILE, JSON.stringify(report, null, 2));
  } catch (e) {
    console.error(`WARN: could not write ${SEED_RESULTS_FILE}:`, e);
  }

  if (failed > 0) {
    console.error('\nFailed humans:');
    for (const r of results.filter(r => r.result === 'failed' || r.result === 'conflict')) {
      console.error(`  ${r.displayName} (${r.humanId}): ${r.error}`);
    }
    // Partial-readiness aware: exit 2 if at least one succeeded, exit 1
    // only on total failure. The Jenkinsfile maps both to UNSTABLE today
    // but the distinction lets a future operator (or external Ralph
    // Wiggum loop) route partial vs total to different remediations.
    process.exit(succeeded > 0 ? 2 : 1);
  }

  process.exit(0);
}

// Only run when executed directly (same guard as seed-household-formation.ts) —
// the pure helpers above are imported by unit tests.
if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}
