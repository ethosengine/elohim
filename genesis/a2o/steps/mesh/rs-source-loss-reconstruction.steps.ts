/**
 * Acceptance glue for the cold RS source-loss station in
 * features/resilience/peer-byteplane-reconstruction.feature.
 *
 * This station has a deliberately narrow read contract:
 *
 *   - placement inventory is local-only `HEAD /shard/{hash}`;
 *   - the first `GET /blob/{hash}` happens only after the ingest holder is
 *     stopped, so inventory cannot heal the fixture it is measuring;
 *   - the request goes to a surviving storage peer directly, never through a
 *     doorway;
 *   - placement must be real: the source must report the existing
 *     distribute_shards acknowledgement and delivery peers must already carry
 *     live custody eligibility. Seeded shard-manifest rows are not evidence.
 *
 * The source-loss leg is intentionally @wip until a live owned mesh has the
 * required remote custody bindings. A missing binding is a named precondition
 * failure before the 68 MiB PUT or any process fault.
 */

import { strict as assert } from 'node:assert';
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { randomBytes, randomUUID } from 'node:crypto';
import { once } from 'node:events';
import { readFile } from 'node:fs/promises';
import { stripVTControlCharacters } from 'node:util';

import { After, Given, Then, When } from '@cucumber/cucumber';

import {
  loadHouseholdMeshFixture,
  requireFixtureStoragePeer,
  type HouseholdMeshFixture,
} from '../../src/framework/fixtures/household-mesh.js';
import {
  findSinglePidByArgv,
  processAlive,
  storagePeerArgvMatch,
} from '../../src/framework/fixtures/process-control.js';
import { destructiveAllowed } from '../../src/framework/fixtures/substrate-scope.js';
import { E2EWorld } from '../../src/framework/world.js';
import { meshControl } from '../dataplane/epr-app-deliverability.helpers.js';

import { artifact, sha256Address, type OversizedArtifact } from './oversized-blob-ingest.steps.js';

const HOUSEHOLD_PEERS = ['matthew', 'jessica', 'james'] as const;
type HouseholdPeerName = (typeof HOUSEHOLD_PEERS)[number];
const INGEST_PEER: HouseholdPeerName = 'matthew';
const SURVIVORS: readonly HouseholdPeerName[] = ['jessica', 'james'];
// The existing owned-mesh directory is shared intentionally for its exclusive fault lock.
// eslint-disable-next-line sonarjs/publicly-writable-directories
const MESH_LOCK = process.env['MESH_DIR'] ?? '/tmp/elohim-local-mesh';

interface ManifestWire {
  blob_hash: string;
  encoding: string;
  data_shards: number;
  total_shards: number;
  shard_hashes: string[];
}

interface DeliveryPeerWire {
  peerId?: string;
  commitments?: unknown;
}

interface Placement {
  manifest: ManifestWire;
  shardHashes: string[];
  indicesByPeer: Map<HouseholdPeerName, number[]>;
}

interface DrillState {
  lock?: ChildProcessWithoutNullStreams;
  placement?: Placement;
  sourceDown: boolean;
  contentId?: string;
  survivorLogBefore: Map<HouseholdPeerName, string>;
  reconstructedBy?: HouseholdPeerName;
  reconstructedHash?: string;
  cleanup?: () => Promise<void>;
}

const states = new WeakMap<E2EWorld, DrillState>();

After({ tags: '@rs-source-loss', timeout: 360_000 }, async function (this: E2EWorld) {
  // Cucumber must see restoration failures; world.runCleanup intentionally
  // swallows callback errors and cannot certify a process-fault drill.
  await states.get(this)?.cleanup?.();
});

function logFields(log: string): Record<string, unknown>[] {
  return log.split(/\r?\n/).flatMap(line => {
    try {
      const record = JSON.parse(stripVTControlCharacters(line)) as {
        fields?: Record<string, unknown>;
      };
      return record.fields && typeof record.fields === 'object' ? [record.fields] : [];
    } catch {
      return [];
    }
  });
}

function stateFor(world: E2EWorld): DrillState {
  let state = states.get(world);
  if (!state) {
    state = { sourceDown: false, survivorLogBefore: new Map() };
    states.set(world, state);
  }
  return state;
}

function fixture(): HouseholdMeshFixture {
  return loadHouseholdMeshFixture();
}

function peerUrl(name: HouseholdPeerName): string {
  return requireFixtureStoragePeer(fixture(), name).url;
}

function peerLogPath(name: HouseholdPeerName): string {
  const peer = requireFixtureStoragePeer(fixture(), name) as { logPath?: string };
  assert.ok(peer.logPath, `${name} has no owned log path for P2P source evidence`);
  return peer.logPath;
}

async function jsonAt<T>(url: string, path: string): Promise<T> {
  const response = await fetch(`${url}${path}`, { signal: AbortSignal.timeout(10_000) });
  const text = await response.text();
  assert.equal(
    response.status,
    200,
    `GET ${url}${path} → ${response.status}: ${text.slice(0, 400)}`
  );
  return JSON.parse(text) as T;
}

async function headShard(url: string, hash: string): Promise<number | undefined> {
  const response = await fetch(`${url}/shard/${encodeURIComponent(hash)}`, {
    method: 'HEAD',
    signal: AbortSignal.timeout(10_000),
  });
  if (response.status === 404) return undefined;
  assert.equal(response.status, 200, `HEAD ${url}/shard/${hash} → ${response.status}`);
  const length = response.headers.get('content-length');
  const size = Number(length);
  assert.ok(
    length !== null && Number.isSafeInteger(size),
    `HEAD /shard/${hash} omitted Content-Length`
  );
  return size;
}

async function directBlob(url: string, hash: string): Promise<Uint8Array> {
  const response = await fetch(`${url}/blob/${encodeURIComponent(hash)}`, {
    signal: AbortSignal.timeout(240_000),
  });
  const bytes = new Uint8Array(await response.arrayBuffer());
  assert.equal(response.status, 200, `GET ${url}/blob/${hash} → ${response.status}`);
  return bytes;
}

const RS_SHARD_BYTES = 17 * 1024 * 1024;
const runSpecificArtifacts = new WeakSet<OversizedArtifact>();

function makeRunSpecific(art: OversizedArtifact): void {
  if (runSpecificArtifacts.has(art)) return;
  // Change every data shard, not only the first one: a single prefix would
  // leave three data shards identical across runs and permit a primed fixture.
  for (let index = 0; index < 4; index += 1) {
    art.bytes.set(randomBytes(64), index * RS_SHARD_BYTES);
  }
  art.hash = sha256Address(art.bytes);
  runSpecificArtifacts.add(art);
}

Given(
  'the artifact is unique to this run and its original content hash is recorded',
  function (this: E2EWorld) {
    makeRunSpecific(artifact(this));
  }
);

async function lockOwnedMesh(world: E2EWorld): Promise<void> {
  const state = stateFor(world);
  const owned = fixture();
  assert.equal(
    owned.processControl,
    true,
    `NEEDS_OWNED_SUBSTRATE: RS source-loss requires processControl=true; ` +
      `${owned.processControlReason ?? 'the fixture does not own local storage processes'}`
  );
  for (const name of HOUSEHOLD_PEERS) {
    const peer = requireFixtureStoragePeer(owned, name);
    assert.ok(peer.url, `fixture has no direct storage URL for ${name}`);
  }

  const lockPath = `${MESH_LOCK}/a2o.lock`;
  const child = spawn(
    '/usr/bin/flock',
    ['-n', lockPath, '/bin/bash', '-c', 'echo locked; read -r _'],
    { stdio: 'pipe' }
  );
  const granted = await Promise.race([
    once(child.stdout, 'data').then(([chunk]) => String(chunk).includes('locked')),
    once(child, 'exit').then(() => false),
  ]);
  assert.ok(granted, `NEEDS_OWNED_SUBSTRATE: ${lockPath} is already held by another drill`);
  state.lock = child;

  state.cleanup = async () => {
    try {
      if (state.sourceDown) {
        await meshControl('storage-restart', INGEST_PEER);
        state.sourceDown = false;
      }
      await Promise.all(
        HOUSEHOLD_PEERS.map(async name => {
          const response = await fetch(`${peerUrl(name)}/health`, {
            signal: AbortSignal.timeout(10_000),
          });
          assert.ok(response.ok, `cleanup did not restore ${name} health: ${response.status}`);
        })
      );
    } finally {
      child.stdin.end('\n');
      if (child.exitCode === null && child.signalCode === null) {
        let timer: ReturnType<typeof setTimeout> | undefined;
        try {
          await Promise.race([
            once(child, 'exit'),
            new Promise((_, reject) => {
              timer = setTimeout(() => {
                child.kill('SIGKILL');
                reject(new Error('Mesh lock holder did not exit'));
              }, 5000);
              timer.unref();
            }),
          ]);
        } finally {
          clearTimeout(timer);
        }
      }
    }
  };
}

Given(
  'the harness holds the exclusive drill lock on its owned household mesh',
  async function (this: E2EWorld) {
    await lockOwnedMesh(this);
  }
);

Given(
  'Jessica and James have active commons consent commitments and are eligible for public-artifact placement',
  { timeout: 60_000 },
  async function (this: E2EWorld): Promise<void> {
    const peers = await jsonAt<DeliveryPeerWire[]>(peerUrl(INGEST_PEER), '/api/v1/peers/delivery');
    const status = await jsonAt<Record<string, unknown>>(peerUrl(INGEST_PEER), '/p2p/status');
    const selfPeerId = String(status['peerId'] ?? '');
    const remote = peers.filter(peer => {
      const commitments = Array.isArray(peer.commitments) ? peer.commitments : [];
      return (
        typeof peer.peerId === 'string' &&
        peer.peerId.length > 0 &&
        peer.peerId !== selfPeerId &&
        commitments.includes('commons')
      );
    });
    const connectedPeers = Number(status['connectedPeers'] ?? 0);
    if (!selfPeerId || remote.length < 2 || connectedPeers < 2) {
      throw new Error(
        `NEEDS_ELIGIBLE_REMOTE_CUSTODY: delivery peers=${JSON.stringify(peers)}, ` +
          `selfPeerId=${selfPeerId || 'missing'}, eligibleRemote=${remote.length}, ` +
          `connectedPeers=${connectedPeers}. ` +
          'Refusing the 68 MiB ingest before placement or fault injection.'
      );
    }
    for (const name of SURVIVORS) {
      const survivor = await jsonAt<{ peerId: string }>(peerUrl(name), '/p2p/status');
      assert.ok(
        remote.some(peer => peer.peerId === survivor.peerId),
        `NEEDS_ELIGIBLE_REMOTE_CUSTODY: ${name} has no eligible delivery row`
      );
      const pid = findSinglePidByArgv(storagePeerArgvMatch(new URL(peerUrl(name)).port));
      assert.ok(pid && processAlive(pid), `${name} has no uniquely owned storage process`);
      const env = await readFile(`/proc/${pid}/environ`, 'utf8');
      const filter =
        env
          .split('\0')
          .find(value => value.startsWith('RUST_LOG='))
          ?.slice(9) ?? '';
      assert.ok(
        /(?:^|,)\s*recovery::transport=(?:debug|trace)\s*(?:,|$)/.test(filter),
        `NEEDS_BYTEPLANE_SOURCE_OBSERVABILITY: ${name} must enable recovery::transport=debug before ingest; verified transfer receipts are DEBUG events`
      );
    }
  }
);

When(
  'the harness sends the artifact to Matthew by PUT under its recorded content hash',
  { timeout: 300_000 },
  async function (this: E2EWorld): Promise<void> {
    assert.ok(
      destructiveAllowed(),
      'NEEDS_OWNED_SUBSTRATE: A2O_ALLOW_DESTRUCTIVE=1 is required before the 68 MiB PUT'
    );
    const art = artifact(this);
    makeRunSpecific(art);
    const response = await fetch(`${peerUrl(INGEST_PEER)}/blob/${art.hash}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/octet-stream' },
      body: art.bytes,
      signal: AbortSignal.timeout(240_000),
    });
    art.putStatus = response.status;
    art.putBody = (await response.text()).slice(0, 400);
  }
);

When(
  'the harness publishes a content record to Matthew naming this artifact to request peer custody',
  { timeout: 60_000 },
  async function (this: E2EWorld): Promise<void> {
    const art = artifact(this);
    const id = `e2e-rs-cold-${Date.now()}-${randomUUID().slice(0, 8)}`;
    const response = await fetch(`${peerUrl(INGEST_PEER)}/db/content`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({
        id,
        contentType: 'narrative',
        contentFormat: 'external',
        reach: 'commons',
        title: id,
        blobHash: art.hash,
        contentBody: `RS source-loss acceptance fixture ${id}`,
      }),
      signal: AbortSignal.timeout(30_000),
    });
    const text = await response.text();
    assert.ok(
      response.status === 200 || response.status === 201,
      `POST ${peerUrl(INGEST_PEER)}/db/content → ${response.status}: ${text.slice(0, 400)}`
    );
    stateFor(this).contentId = id;
  }
);

function normalizedManifest(manifest: ManifestWire): {
  encoding: string;
  dataShards: number;
  totalShards: number;
  hashes: string[];
} {
  return {
    encoding: manifest.encoding,
    dataShards: manifest.data_shards,
    totalShards: manifest.total_shards,
    hashes: manifest.shard_hashes,
  };
}

async function placementAcknowledged(
  contentId: string,
  indicesByPeer: Map<HouseholdPeerName, number[]>
): Promise<boolean> {
  const fields = logFields(await readFile(peerLogPath(INGEST_PEER), 'utf8'));
  const acknowledged = fields.some(
    row =>
      row.content_id === contentId &&
      row.message === 'Shard distribution complete' &&
      Number(row.shards) >= 4
  );
  if (!acknowledged) {
    return false;
  }
  for (const name of SURVIVORS) {
    const holder = requireFixtureStoragePeer(fixture(), name).agentPubKey;
    assert.ok(holder, `${name} lacks its custody agent identity in the owned fixture`);
    for (const index of indicesByPeer.get(name) ?? []) {
      if (
        !fields.some(
          row =>
            row.content_id === contentId &&
            row.message === 'Shard distributed' &&
            row.peer === holder &&
            row.shard_index === index &&
            (row.transport === 'libp2p' || row.transport === 'iroh')
        )
      )
        return false;
    }
  }
  return true;
}

async function readPlacement(
  art: OversizedArtifact,
  contentId: string
): Promise<Placement | undefined> {
  const manifest = await jsonAt<ManifestWire>(
    peerUrl(INGEST_PEER),
    `/manifest/${encodeURIComponent(art.hash)}`
  );
  const normalized = normalizedManifest(manifest);
  const manifestHash = manifest.blob_hash;
  assert.equal(
    manifestHash,
    art.hash,
    `manifest hash ${String(manifestHash)} does not match this run's artifact ${art.hash}`
  );
  if (
    normalized.encoding !== 'rs-4-7' ||
    normalized.dataShards !== 4 ||
    normalized.totalShards !== 7 ||
    normalized.hashes.length !== 7 ||
    new Set(normalized.hashes).size !== 7
  ) {
    throw new Error(
      `NEEDS_ELIGIBLE_REMOTE_CUSTODY: ${art.hash} manifest is not RS-4-7: ` +
        `${JSON.stringify(manifest)}`
    );
  }

  const indicesByPeer = new Map<HouseholdPeerName, number[]>();
  for (const name of HOUSEHOLD_PEERS) {
    const present: number[] = [];
    for (const [index, hash] of normalized.hashes.entries()) {
      // HEAD is intentionally the only shard inventory read. GET /shard and
      // GET /blob are healing reads and would make this precondition circular.
      const size = await headShard(peerUrl(name), hash);
      if (size !== undefined) {
        assert.equal(
          size,
          RS_SHARD_BYTES,
          `${name} holds RS shard ${index} at ${size} bytes; expected ${RS_SHARD_BYTES}`
        );
        present.push(index);
      }
    }
    indicesByPeer.set(name, present);
  }

  const all = new Set(Array.from(indicesByPeer.values()).flat());
  const survivors = new Set(SURVIVORS.flatMap(name => indicesByPeer.get(name) ?? []));
  const noWholeCopy = SURVIVORS.every(name => (indicesByPeer.get(name)?.length ?? 0) < 7);
  if (
    all.size !== 7 ||
    survivors.size < 4 ||
    SURVIVORS.some(name => (indicesByPeer.get(name)?.length ?? 0) >= 4) ||
    !noWholeCopy
  ) {
    return undefined;
  }

  for (const name of SURVIVORS) {
    assert.equal(
      await headShard(peerUrl(name), art.hash),
      undefined,
      `${name} already holds the complete artifact; refusing a warm-copy recovery proof`
    );
  }

  if (!(await placementAcknowledged(contentId, indicesByPeer))) return undefined;
  return { manifest, shardHashes: normalized.hashes, indicesByPeer };
}

Then(
  "Matthew's log records Jessica and James acknowledging their assigned shards of this artifact",
  { timeout: 240_000 },
  async function (this: E2EWorld): Promise<void> {
    const art = artifact(this);
    const contentId = stateFor(this).contentId;
    assert.ok(contentId, 'content creation/distribution trigger did not run');
    const deadline = Date.now() + 180_000;
    let last: string | undefined;
    while (Date.now() < deadline) {
      try {
        const placement = await readPlacement(art, contentId);
        if (placement) {
          stateFor(this).placement = placement;
          return;
        }
        last = 'seven shard indices / survivor floor / distribution acknowledgement not observed';
      } catch (error) {
        last = String(error);
      }
      await new Promise(resolve => setTimeout(resolve, 3_000));
    }
    throw new Error(
      `NEEDS_ELIGIBLE_REMOTE_CUSTODY: placement never became fault-safe; ${last ?? 'no observation'}. ` +
        'No storage process was stopped.'
    );
  }
);

When(
  'the harness stops Matthew after placement is proven',
  { timeout: 180_000 },
  async function (this: E2EWorld): Promise<void> {
    const state = stateFor(this);
    assert.ok(state.placement, 'placement precondition was not proven; refusing to fault the mesh');
    assert.ok(
      destructiveAllowed(),
      'NEEDS_OWNED_SUBSTRATE: A2O_ALLOW_DESTRUCTIVE=1 is required before storage-stop'
    );
    const port = new URL(peerUrl(INGEST_PEER)).port;
    const pid = findSinglePidByArgv(storagePeerArgvMatch(port));
    assert.ok(
      pid && processAlive(pid),
      `NEEDS_OWNED_SUBSTRATE: no unique live ingest peer on ${port}`
    );
    state.survivorLogBefore = new Map(
      await Promise.all(
        SURVIVORS.map(async name => {
          return [name, await readFile(peerLogPath(name), 'utf8')] as const;
        })
      )
    );
    state.sourceDown = true;
    await meshControl('storage-stop', INGEST_PEER);
    const response = await fetch(`${peerUrl(INGEST_PEER)}/health`, {
      signal: AbortSignal.timeout(2_000),
    }).catch(() => undefined);
    assert.ok(
      !response && !processAlive(pid),
      'storage-stop returned but the ingest peer still answers /health'
    );
  }
);

Then(
  'local-only non-fetching presence checks show Jessica and James together hold at least four distinct shards of its seven 17 MiB shards, fewer than four each, and no complete copy',
  function (this: E2EWorld) {
    assert.ok(
      stateFor(this).placement,
      'No acknowledged, size-checked survivor placement was observed'
    );
  }
);

When(
  'the harness restores Matthew after the recovery drill',
  { timeout: 320_000 },
  async function (this: E2EWorld) {
    const state = stateFor(this);
    assert.ok(state.sourceDown, 'Matthew was not stopped by this drill');
    await meshControl('storage-restart', INGEST_PEER);
    state.sourceDown = false;
  }
);

Then(
  'all three storage peers respond successfully to their storage health checks',
  async function () {
    for (const name of HOUSEHOLD_PEERS) {
      const response = await fetch(`${peerUrl(name)}/health`, {
        signal: AbortSignal.timeout(10_000),
      });
      assert.ok(response.ok, `${name} is not healthy after restoration: ${response.status}`);
    }
  }
);

When(
  'the client requests the artifact directly from Jessica while Matthew remains stopped',
  { timeout: 300_000 },
  async function (this: E2EWorld): Promise<void> {
    const state = stateFor(this);
    const art = artifact(this);
    assert.ok(state.placement, 'no RS placement witness was recorded');
    const target = SURVIVORS.find(
      name => (state.placement?.indicesByPeer.get(name)?.length ?? 0) < 4
    );
    assert.ok(target, 'no surviving peer met the below-four local shard requirement');
    let localShards = 0;
    for (const hash of state.placement.shardHashes) {
      if ((await headShard(peerUrl(target), hash)) !== undefined) localShards += 1;
    }
    assert.ok(
      localShards < 4,
      `${target} warmed enough shards before the read; refusing a cold-read claim`
    );
    assert.equal(
      await headShard(peerUrl(target), art.hash),
      undefined,
      `${target} has a complete local copy`
    );
    state.survivorLogBefore.set(target, await readFile(peerLogPath(target), 'utf8'));
    const bytes = await directBlob(peerUrl(target), art.hash);
    state.reconstructedHash = sha256Address(bytes);
    const sourceStillDown = await fetch(`${peerUrl(INGEST_PEER)}/health`, {
      signal: AbortSignal.timeout(2_000),
    }).catch(() => undefined);
    assert.ok(
      !sourceStillDown &&
        !findSinglePidByArgv(storagePeerArgvMatch(new URL(peerUrl(INGEST_PEER)).port)),
      'the ingest peer came back during the direct reconstruction; source loss was not observed'
    );
    state.reconstructedBy = target;
  }
);

Then('the returned bytes match the original artifact hash', function (this: E2EWorld) {
  assert.equal(
    stateFor(this).reconstructedHash,
    artifact(this).hash,
    'The direct reconstruction returned different bytes'
  );
});

Then(
  'local-only checks show Jessica now holds at least four distinct shards of this artifact',
  async function (this: E2EWorld) {
    const state = stateFor(this);
    assert.ok(
      state.placement && state.reconstructedBy === 'jessica',
      'Jessica did not complete the read'
    );
    let held = 0;
    for (const hash of state.placement.shardHashes) {
      if ((await headShard(peerUrl('jessica'), hash)) === 17 * 1024 * 1024) held += 1;
    }
    assert.ok(held >= 4, `Jessica holds only ${held} distinct shards after recovery`);
  }
);

Then(
  "Jessica's transfer log records successful receipt from James of a verified missing shard of this artifact, identifying the shard hash and peer transport during this read",
  { timeout: 60_000 },
  async function (this: E2EWorld): Promise<void> {
    const state = stateFor(this);
    const placement = state.placement;
    assert.ok(placement && state.reconstructedBy, 'the direct reconstruction did not run');
    const receiver = state.reconstructedBy;
    const source = SURVIVORS.find(name => name !== receiver);
    assert.ok(source, 'No distinct surviving source peer');
    const sourceStatus = await jsonAt<{ peerId: string }>(peerUrl(source), '/p2p/status');
    assert.ok(sourceStatus.peerId, 'Source peer has no transport identity');
    const after = await readFile(peerLogPath(receiver), 'utf8');
    const before = state.survivorLogBefore.get(receiver) ?? '';
    assert.ok(
      after.startsWith(before),
      'Receiver log rotated during the drill; source evidence is incomplete'
    );
    const evidence = logFields(after.slice(before.length));
    const sourceIndices = placement.indicesByPeer.get(source) ?? [];
    const receiverIndices = placement.indicesByPeer.get(receiver) ?? [];
    const sourceLine = evidence.find(row => {
      return (
        row.message === 'share-blob received' &&
        row.source_peer === sourceStatus.peerId &&
        (row.transport === 'libp2p' || row.transport === 'iroh') &&
        sourceIndices.some(
          index =>
            !receiverIndices.includes(index) && row.blob_hash === placement.shardHashes[index]
        )
      );
    });
    assert.ok(sourceLine, 'direct reconstruction emitted no same-line P2P shard source evidence');
    this.attach(
      JSON.stringify({
        artifactHash: artifact(this).hash,
        receiver,
        source,
        sourcePeerId: sourceStatus.peerId,
        sourceLine,
      }),
      'application/json'
    );
  }
);

Then(
  'Matthew has no running storage process and does not answer direct storage requests',
  async function () {
    const response = await fetch(`${peerUrl(INGEST_PEER)}/health`, {
      signal: AbortSignal.timeout(2_000),
    }).catch(() => undefined);
    assert.ok(
      !response && !findSinglePidByArgv(storagePeerArgvMatch(new URL(peerUrl(INGEST_PEER)).port)),
      'Matthew remains reachable or has a running storage process'
    );
  }
);
