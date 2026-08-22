/**
 * One manifest-backed authority for live household-mesh acceptance fixtures.
 *
 * Set E2E_HOUSEHOLD_FIXTURE_PATH to a JSON file shaped like
 * household-mesh.fixture.example.json. Existing E2E_* variables remain
 * supported and override the manifest so CI can inject secrets/ephemeral PIDs
 * without rewriting it.
 */

import { strict as assert } from 'node:assert';
import { readFileSync } from 'node:fs';

export interface DoorwayFixture {
  url?: string;
  primaryStorageUrl?: string;
  poolStorageUrls?: string[];
  logPath?: string;
  /**
   * Declared NOT to exist on this substrate, with the reason a reader needs.
   * A scenario that names a doorway this fleet does not deploy then fails with
   * the topology fact ("this fleet deploys two doorways") instead of an
   * environment-variable name, which tells the reader nothing.
   */
  absentReason?: string;
}

export interface StoragePeerFixture {
  url?: string;
  pid?: number;
  pidFile?: string;
  /**
   * The peer's Holochain agent key (`uhCAk…`), stamped by `hc-mesh.sh`
   * (`refresh_fixture_pids`) from the running process's AGENT_PUBKEY. Custody
   * commitments name providers in THIS namespace (seed-commitments resolves
   * `/auth/me`), while `/p2p/status` reports the libp2p transport id — a drill
   * matching a provider must accept either (reconcile/custody.rs contract).
   */
  agentPubKey?: string;
}

export interface HouseholdMeshFixture {
  commonsEprId?: string;
  convergenceWindowMs?: number;
  connectedPeersFloor?: number;
  /**
   * Can this harness control the mesh PROCESSES (SIGSTOP a peer, tail a
   * doorway's log file)? True on a local stack (`just mesh start` — real PIDs
   * on this host, real log files on this disk). False against a deployed
   * fleet, where every peer and doorway is a remote pod: there is no PID to
   * signal and no file to read. Scenarios that need it are local-stack-only.
   */
  processControl?: boolean;
  processControlReason?: string;
  doorways?: Record<string, DoorwayFixture>;
  storagePeers?: Record<string, StoragePeerFixture>;
}

export interface HouseholdFootprint {
  commitmentBacked: number;
  holders: string[];
}

type FixtureEnvironment = Readonly<Record<string, string | undefined>>;

const HOUSEHOLD_PEERS = ['matthew', 'jessica', 'james'] as const;

function withoutTrailingSlashes(value: string): string {
  let end = value.length;
  while (end > 0 && value[end - 1] === '/') end -= 1;
  return value.slice(0, end);
}

function optionalInteger(value: string | undefined): number | undefined {
  if (value === undefined || value.trim() === '') return undefined;
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed)) {
    throw new Error(`household fixture expected an integer, got "${value}"`);
  }
  return parsed;
}

function csvUrls(value: string | undefined): string[] | undefined {
  const urls = value
    ?.split(',')
    .map(item => item.trim())
    .filter(Boolean)
    .map(withoutTrailingSlashes);
  return urls?.length ? [...new Set(urls)] : undefined;
}

function cloneFixture(source: HouseholdMeshFixture): HouseholdMeshFixture {
  return {
    ...source,
    doorways: Object.fromEntries(
      Object.entries(source.doorways ?? {}).map(([id, doorway]) => [
        id,
        {
          ...doorway,
          poolStorageUrls: doorway.poolStorageUrls ? [...doorway.poolStorageUrls] : undefined,
        },
      ])
    ),
    storagePeers: Object.fromEntries(
      Object.entries(source.storagePeers ?? {}).map(([id, peer]) => [id, { ...peer }])
    ),
  };
}

function applyDoorwayEnvironment(
  doorways: Record<string, DoorwayFixture>,
  env: FixtureEnvironment
): void {
  for (const id of ['alpha', 'beta', 'apex', 'gamma']) {
    // The second doorway has two live names for the same substrate fact:
    // E2E_DOORWAY_BETA is the a2o Background convention (this loop's own
    // alpha/beta/apex/gamma id naming, `Given doorway "beta" at
    // "E2E_DOORWAY_BETA"`); E2E_DOORWAY_B is the CI vocabulary
    // (scripts/ci/run-dataplane-validation.sh passes it to the quiesce gate,
    // and src/framework/dataplane/surfaces.ts's resolvePeerUrl('elohim.host')
    // already honors it). A run that sets only one must still resolve "beta"
    // — BETA wins when both are set, since the mode-aware step's direct
    // env-var lookup reads it before ever consulting this fixture.
    const betaAlias = id === 'beta' ? env['E2E_DOORWAY_B'] : undefined;
    const envUrl = env[`E2E_DOORWAY_${id.toUpperCase()}`] ?? betaAlias;
    doorways[id] = {
      ...doorways[id],
      ...(envUrl ? { url: withoutTrailingSlashes(envUrl) } : {}),
    };
  }

  const primaryStorageUrl = env['E2E_DOORWAY_PRIMARY_STORAGE_URL'] ?? env['E2E_STORAGE_URL'];
  const poolStorageUrls = csvUrls(env['E2E_DOORWAY_POOL_STORAGE_URLS']);
  doorways['alpha'] = {
    ...doorways['alpha'],
    ...(primaryStorageUrl ? { primaryStorageUrl: withoutTrailingSlashes(primaryStorageUrl) } : {}),
    ...(poolStorageUrls ? { poolStorageUrls } : {}),
    ...(env['E2E_DOORWAY_LOG_PATH'] ? { logPath: env['E2E_DOORWAY_LOG_PATH'] } : {}),
  };

  const apexPrimary = env['E2E_APEX_PRIMARY_STORAGE_URL'];
  doorways['apex'] = {
    ...doorways['apex'],
    ...(apexPrimary ? { primaryStorageUrl: withoutTrailingSlashes(apexPrimary) } : {}),
  };
}

function applyStoragePeerEnvironment(
  storagePeers: Record<string, StoragePeerFixture>,
  env: FixtureEnvironment
): void {
  for (const name of HOUSEHOLD_PEERS) {
    const upper = name.toUpperCase();
    const url = env[`E2E_STORAGE_${upper}`];
    const pid = optionalInteger(env[`E2E_STORAGE_${upper}_PID`]);
    const pidFile = env[`E2E_STORAGE_${upper}_PID_FILE`];
    storagePeers[name] = {
      ...storagePeers[name],
      ...(url ? { url: withoutTrailingSlashes(url) } : {}),
      ...(pid === undefined ? {} : { pid }),
      ...(pidFile ? { pidFile } : {}),
    };
  }
}

function applyConventionalPool(
  doorways: Record<string, DoorwayFixture>,
  storagePeers: Record<string, StoragePeerFixture>
): void {
  const conventionalPool = HOUSEHOLD_PEERS.map(name => storagePeers[name]?.url).filter(
    (url): url is string => Boolean(url)
  );
  if ((doorways['alpha']?.poolStorageUrls?.length ?? 0) > 0 || conventionalPool.length === 0) {
    return;
  }
  doorways['alpha'] = {
    ...doorways['alpha'],
    poolStorageUrls: [...new Set(conventionalPool)],
  };
}

/** Apply conventional E2E variables as overrides to a parsed fixture. */
export function mergeHouseholdMeshEnvironment(
  source: HouseholdMeshFixture,
  env: FixtureEnvironment
): HouseholdMeshFixture {
  const fixture = cloneFixture(source);
  fixture.doorways ??= {};
  fixture.storagePeers ??= {};

  applyDoorwayEnvironment(fixture.doorways, env);
  applyStoragePeerEnvironment(fixture.storagePeers, env);
  applyConventionalPool(fixture.doorways, fixture.storagePeers);

  fixture.commonsEprId = env['E2E_COMMONS_EPR_ID'] ?? fixture.commonsEprId ?? 'manifesto';
  fixture.convergenceWindowMs =
    optionalInteger(env['E2E_CONVERGENCE_WINDOW_MS']) ?? fixture.convergenceWindowMs;
  fixture.connectedPeersFloor =
    optionalInteger(env['E2E_CONNECTED_PEERS_FLOOR']) ?? fixture.connectedPeersFloor;
  return fixture;
}

export function loadHouseholdMeshFixture(
  env: FixtureEnvironment = process.env
): HouseholdMeshFixture {
  const path = env['E2E_HOUSEHOLD_FIXTURE_PATH'];
  let source: HouseholdMeshFixture = {};
  if (path) {
    try {
      source = JSON.parse(readFileSync(path, 'utf8')) as HouseholdMeshFixture;
    } catch (error) {
      throw new Error(`cannot load household fixture ${path}: ${String(error)}`);
    }
  }
  return mergeHouseholdMeshEnvironment(source, env);
}

/**
 * Resolve a doorway URL from the fixture, or `undefined` when the fixture
 * simply says nothing about it.
 *
 * Throws — deliberately — when the fixture DECLARES the doorway absent. A
 * declaration is knowledge; silently returning `undefined` for it discards that
 * knowledge and the caller reports a missing environment variable instead of
 * the substrate fact.
 *
 * Because it can throw, this is a RESOLVE-OR-EXPLAIN accessor, not a cheap
 * lookup: it must be the LAST thing a caller consults. Reached before the
 * caller's own candidates, a declared absence shadows a URL that was right
 * there. Use `resolveDoorwayUrl()` rather than hand-rolling that order.
 */
export function fixtureDoorwayUrl(fixture: HouseholdMeshFixture, id: string): string | undefined {
  const doorway = fixture.doorways?.[id];
  if (doorway?.url) return doorway.url;
  if (doorway?.absentReason) {
    throw new Error(`household fixture declares doorway "${id}" absent: ${doorway.absentReason}`);
  }
  return undefined;
}

/**
 * Resolve a doorway URL from the caller's own candidates FIRST, and only then
 * ask the fixture.
 *
 * The fixture is loaded lazily, inside the fallthrough, so neither a declared
 * absence nor an unreadable manifest can shadow a URL the caller already had —
 * a literal `doorway "alpha" at "http://localhost:8888"` resolves to that
 * literal even on a substrate whose manifest declares alpha absent. When
 * nothing resolves, the fixture still gets the last word, so an honest absence
 * is still reported as the topology fact rather than a variable name.
 */
export function resolveDoorwayUrl(
  id: string,
  candidates: readonly (string | undefined)[],
  loadFixture: () => HouseholdMeshFixture = loadHouseholdMeshFixture
): string | undefined {
  for (const candidate of candidates) {
    if (candidate) return candidate;
  }
  return fixtureDoorwayUrl(loadFixture(), id);
}

function knownDoorwayIds(fixture: HouseholdMeshFixture): string {
  const ids = Object.entries(fixture.doorways ?? {})
    .filter(([, doorway]) => Boolean(doorway?.url))
    .map(([id]) => id);
  return ids.length ? ids.join(', ') : 'none';
}

export function requireFixtureDoorwayUrl(fixture: HouseholdMeshFixture, id: string): string {
  const value = fixtureDoorwayUrl(fixture, id);
  if (!value) {
    throw new Error(
      `live household fixture missing doorway "${id}" ` +
        `(doorways this run resolved: ${knownDoorwayIds(fixture)}): ` +
        `set E2E_DOORWAY_${id.toUpperCase()} ` +
        'or doorways.' +
        `${id}.url in E2E_HOUSEHOLD_FIXTURE_PATH`
    );
  }
  return value;
}

export function requireFixturePrimaryStorageUrl(
  fixture: HouseholdMeshFixture,
  doorwayId: string
): string {
  const value = fixture.doorways?.[doorwayId]?.primaryStorageUrl;
  if (!value) {
    throw new Error(
      `live household fixture missing primary storage for doorway "${doorwayId}" in ` +
        'E2E_HOUSEHOLD_FIXTURE_PATH'
    );
  }
  return withoutTrailingSlashes(value);
}

export function requireFixturePoolStorageUrls(
  fixture: HouseholdMeshFixture,
  doorwayId: string
): string[] {
  const values = fixture.doorways?.[doorwayId]?.poolStorageUrls ?? [];
  if (values.length === 0) {
    throw new Error(
      `live household fixture missing storage pool for doorway "${doorwayId}": set ` +
        'E2E_DOORWAY_POOL_STORAGE_URLS or the manifest poolStorageUrls'
    );
  }
  return [...new Set(values.map(withoutTrailingSlashes))];
}

/**
 * Why this harness cannot reach into the mesh's processes. Named so a scenario
 * that needs process control reports the substrate fact, not a variable name.
 */
function processControlNote(fixture: HouseholdMeshFixture): string {
  if (fixture.processControl === false) {
    return (
      fixture.processControlReason ??
      'this run declared no process control over the mesh (processControl: false)'
    );
  }
  return (
    'process control needs the peer or doorway running as a local process on this host ' +
    '(`just mesh start`, plus pid/pidFile/logPath in E2E_HOUSEHOLD_FIXTURE_PATH)'
  );
}

export function requireFixtureDoorwayLogPath(
  fixture: HouseholdMeshFixture,
  doorwayId: string
): string {
  const value = fixture.doorways?.[doorwayId]?.logPath;
  if (!value) {
    throw new Error(
      `cannot read doorway "${doorwayId}" logs: no log file for it. ` +
        `${processControlNote(fixture)}. A deployed doorway logs to its pod's stdout, so no file ` +
        'exists to tail — that scenario is local-stack-only and belongs behind @local. ' +
        'On a local stack set E2E_DOORWAY_LOG_PATH or the manifest logPath.'
    );
  }
  return value;
}

export function requireFixtureStoragePeer(
  fixture: HouseholdMeshFixture,
  name: string
): Required<Pick<StoragePeerFixture, 'url'>> & StoragePeerFixture {
  const peer = fixture.storagePeers?.[name];
  if (!peer?.url) {
    throw new Error(
      `live household fixture missing storage peer "${name}": set ` +
        `E2E_STORAGE_${name.toUpperCase()} or storagePeers.${name}.url in the manifest ` +
        '(CI wires these from genesis/orchestrator/data/deployments.json)'
    );
  }
  return { ...peer, url: withoutTrailingSlashes(peer.url) };
}

export function requireFixturePeerPid(fixture: HouseholdMeshFixture, name: string): number {
  const peer = fixture.storagePeers?.[name];
  let fromFile: string | undefined;
  if (peer?.pidFile) {
    try {
      fromFile = readFileSync(peer.pidFile, 'utf8').trim();
    } catch (error) {
      throw new Error(`cannot read PID file for household peer "${name}": ${String(error)}`);
    }
  }
  const pid = peer?.pid ?? optionalInteger(fromFile);
  if (!pid || pid <= 1 || pid === process.pid) {
    throw new Error(
      `cannot pause or resume household peer "${name}": no safe PID for it. ` +
        `${processControlNote(fixture)}. Against a deployed fleet the peer is a remote pod, so ` +
        'there is nothing here to signal — that scenario is local-stack-only and belongs behind ' +
        `@local. On a local stack set E2E_STORAGE_${name.toUpperCase()}_PID, its _PID_FILE, or ` +
        'pid/pidFile in the manifest.'
    );
  }
  return pid;
}

/**
 * Reduce a household resilience response to the guide-star convergence bar.
 * Labels and array order are doorway-local presentation; holder kind+id and
 * the commitment-backed count are protocol testimony.
 */
export function normalizeHouseholdFootprint(
  snapshot: Record<string, unknown>,
  contentId: string
): HouseholdFootprint {
  assert.equal(snapshot['contentId'], contentId, 'doorway testified about a different EPR');

  const commitmentBacked = snapshot['commitmentBackedCollectives'];
  assert.ok(
    typeof commitmentBacked === 'number' && Number.isInteger(commitmentBacked),
    `snapshot omitted integer commitmentBackedCollectives: ${JSON.stringify(snapshot)}`
  );
  assert.ok(commitmentBacked > 0, 'commons fixture is not commitment-backed yet');

  const felt = snapshot['feltStatus'] as Record<string, unknown> | undefined;
  const details = snapshot['details'] as Record<string, unknown> | undefined;
  const entries =
    (felt?.['heldBy'] as Record<string, unknown>[] | undefined) ??
    (details?.['stewardingCollectives'] as Record<string, unknown>[] | undefined);
  assert.ok(Array.isArray(entries) && entries.length > 0, 'snapshot exposes no holder set');

  const holders = entries.map(entry => {
    const id = entry['id'];
    const kind = entry['kind'];
    assert.ok(
      typeof id === 'string' && id.length > 0,
      `holder has no id: ${JSON.stringify(entry)}`
    );
    assert.ok(
      typeof kind === 'string' && kind.length > 0,
      `holder has no kind: ${JSON.stringify(entry)}`
    );
    return `${kind}:${id}`;
  });

  assert.equal(new Set(holders).size, holders.length, 'snapshot contains duplicate holders');
  holders.sort((first, second) => first.localeCompare(second));
  return { commitmentBacked, holders };
}
