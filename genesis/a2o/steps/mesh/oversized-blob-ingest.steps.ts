/**
 * Step glue for the self-ingesting scenarios in
 * features/resilience/app-blob-heal-on-read.feature: the 17 MiB sequential-
 * chunk restart proof and the 68 MiB erasure-coded ingest proof.
 *
 * WHY THIS IS ITS OWN FILE. Every other blob step in this concern reads a blob
 * some other producer put there (the seeder, a deploy stage, a peer heal). This
 * one is the only step that INGESTS bytes of its own, and it ingests a lot of
 * them — 68 MiB, deliberately above the 64 MiB erasure-coding threshold. Keeping
 * it separate keeps the read-only classifiers' read-set honest and keeps the
 * oversized payload out of every scenario that does not want it.
 *
 * WHAT IT PINS. Above `RS_THRESHOLD` (64 MiB) elohim-storage mints an `"rs-4-7"`
 * manifest: 4 data + 3 parity shards computed over PADDED data, so
 * `shard_size = ceil(len/4)` and there is no sequential-chunk correspondence
 * between shard N and byte range N. Ingest used to hand-slice the raw request
 * body as if there were, which mis-hashed the unpadded tail data shard and then
 * indexed past the end of the body at the first parity shard — panicking the
 * HTTP task mid-PUT. The client saw the connection drop after `100 Continue`
 * and the blob 404'd on every peer afterwards. The band had never worked.
 *
 * WHY BYTE-IDENTITY AND NOT JUST A 200. A 200 alone would also have been
 * returned by a truncating or zero-padding bug: RS pads the last data shard, so
 * "served something of roughly the right size" is exactly the failure this
 * scenario has to exclude. The payload is deterministic pseudo-random (an
 * all-zeros or all-'A' buffer would survive both a truncation and a padding
 * bug), and the assertion is a full byte comparison plus a sha256 re-hash of
 * what came back — the same address the peer was asked to file it under.
 *
 * OWNERSHIP GATE. The PUT writes 68 MiB into a peer's local store, so it runs
 * only where this lane owns the substrate (`destructiveAllowed()`, the same
 * declared-cap authority the chaos drills use) and answers `skipped` — never
 * `pending`, which cucumber-js strict-mode reds identically to a failure —
 * anywhere else.
 */

import { strict as assert } from 'node:assert';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';

import { Given, Then, When } from '@cucumber/cucumber';

import { destructiveAllowed } from '../../src/framework/fixtures/substrate-scope.js';
import { E2EWorld } from '../../src/framework/world.js';

/** Storage base URL — same resolution the resilience steps use. */
function storageUrl(): string {
  return process.env['E2E_STORAGE_URL'] ?? 'http://localhost:8090';
}

interface OversizedArtifact {
  bytes: Uint8Array;
  /** Canonical `sha256-{hex}` address. */
  hash: string;
  putStatus?: number;
  putBody?: string;
}

const BASH_BIN = process.env['E2E_BASH_BIN'] ?? '/bin/bash';

/** This step file lives at genesis/a2o/steps/mesh. */
function hcMeshScriptPath(): string {
  return fileURLToPath(new URL('../../../../app/elohim-app/scripts/hc-mesh.sh', import.meta.url));
}

/**
 * Per-scenario state. A `WeakMap` keyed on the world keeps it scenario-scoped
 * without widening `E2EWorld` for one feature, and lets the 68 MiB buffer be
 * collected as soon as the scenario's world goes out of scope.
 */
const artifacts = new WeakMap<E2EWorld, OversizedArtifact>();

function artifact(world: E2EWorld): OversizedArtifact {
  const found = artifacts.get(world);
  assert.ok(found, 'no artifact was built for this scenario — the Given step did not run');
  return found;
}

/**
 * Deterministic, non-uniform bytes.
 *
 * A constant fill would pass under both a truncation bug and a zero-padding
 * bug, which are precisely the two shapes RS shard alignment can produce. An
 * LCG gives reproducible entropy without pulling a dependency or spending
 * 68 MiB of real randomness.
 */
function deterministicBytes(len: number): Uint8Array {
  const out = new Uint8Array(len);
  const view = new DataView(out.buffer);
  let state = 0x9e3779b97f4a7c15n;
  const mul = 6364136223846793005n;
  const inc = 1442695040888963407n;
  const mask = (1n << 64n) - 1n;
  for (let offset = 0; offset + 8 <= len; offset += 8) {
    state = (state * mul + inc) & mask;
    view.setBigUint64(offset, state, true);
  }
  // Tail bytes below an 8-byte stride.
  state = (state * mul + inc) & mask;
  for (let i = len - (len % 8); i < len; i++) {
    out[i] = Number((state >> BigInt(8 * (i % 8))) & 0xffn);
  }
  return out;
}

function sha256Address(bytes: Uint8Array): string {
  return `sha256-${createHash('sha256').update(bytes).digest('hex')}`;
}

function heldForOwnership(description: string): 'skipped' | undefined {
  if (destructiveAllowed()) return undefined;
  process.stdout.write(
    `[a2o] substrate-writing step HELD: ${description}. This lane does not declare ` +
      'owned-substrate; set A2O_ALLOW_DESTRUCTIVE=1 on a mesh you own to run it.\n'
  );
  return 'skipped';
}

Given(
  'an artifact of {int} MiB, above the erasure-coding threshold',
  function (this: E2EWorld, mib: number) {
    assert.ok(
      mib > 64,
      `an artifact of ${mib} MiB is not above the 64 MiB erasure-coding threshold — ` +
        'this scenario would silently exercise the "chunked" band instead'
    );
    const bytes = deterministicBytes(mib * 1024 * 1024);
    artifacts.set(this, { bytes, hash: sha256Address(bytes) });
  }
);

Given(
  'an artifact of {int} MiB, in the sequential chunked-blob band',
  function (this: E2EWorld, mib: number) {
    assert.ok(
      mib > 16 && mib <= 64,
      `an artifact of ${mib} MiB is outside the sequential chunked-blob band ` +
        '(>16 MiB and <=64 MiB)'
    );
    const bytes = deterministicBytes(mib * 1024 * 1024);
    artifacts.set(this, { bytes, hash: sha256Address(bytes) });
  }
);

When(
  'I PUT the artifact to the storage peer under its own hash',
  { timeout: 300_000 },
  async function (this: E2EWorld) {
    const held = heldForOwnership('PUT an oversized artifact into a storage peer');
    if (held) return held;
    const art = artifact(this);
    const response = await fetch(`${storageUrl()}/blob/${art.hash}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/octet-stream' },
      body: art.bytes,
      signal: AbortSignal.timeout(240_000),
    });
    art.putStatus = response.status;
    art.putBody = (await response.text()).slice(0, 400);
    return undefined;
  }
);

Then('the storage peer accepts the artifact', function (this: E2EWorld) {
  const held = heldForOwnership('assert the oversized PUT was accepted');
  if (held) return held;
  const art = artifact(this);
  assert.ok(
    art.putStatus === 200 || art.putStatus === 201,
    `PUT /blob/${art.hash} → ${String(art.putStatus)} (body: ${art.putBody ?? ''}). ` +
      'An artifact above the inline threshold must be accepted whole; a dropped connection ' +
      'here means the ingest task failed on shard arithmetic.'
  );
  return undefined;
});

When('the storage peer restarts', { timeout: 320_000 }, function () {
  const peer = process.env['E2E_MESH_RESTART_PEER'] ?? 'matthew';
  const held = heldForOwnership(`restart mesh storage peer "${peer}"`);
  if (held) return held;

  const script = hcMeshScriptPath();
  const result = spawnSync(BASH_BIN, [script, 'storage-restart', peer], {
    encoding: 'utf8',
    timeout: 300_000,
  });
  assert.equal(
    result.status,
    0,
    `${script} storage-restart ${peer} exited ${String(result.status)}: ` +
      `${(result.stderr ?? '').slice(-2000)}`
  );
  return undefined;
});

async function fetchAfterPossibleRestart(url: string): Promise<Response> {
  let lastError: unknown;
  for (let attempt = 0; attempt < 4; attempt += 1) {
    try {
      return await fetch(url, { signal: AbortSignal.timeout(240_000) });
    } catch (error) {
      lastError = error;
      await new Promise(resolve => setTimeout(resolve, 500));
    }
  }
  throw lastError;
}

Then(
  'GET {string} for that artifact from the same storage peer returns it byte-identical',
  { timeout: 300_000 },
  async function (this: E2EWorld, route: string) {
    const held = heldForOwnership(`read the oversized artifact back from ${route}`);
    if (held) return held;
    const art = artifact(this);
    // The scenario names the route the way the substrate exposes it —
    // `/blob/{hash}` — so a reader sees the whole request, not a prefix plus
    // an inference about how the address gets appended.
    const path = route.replace('{hash}', art.hash);
    const response = await fetchAfterPossibleRestart(`${storageUrl()}${path}`);
    assert.equal(
      response.status,
      200,
      `GET ${path} → ${response.status} — an accepted artifact must serve back`
    );
    const served = new Uint8Array(await response.arrayBuffer());
    assert.equal(
      served.length,
      art.bytes.length,
      `served ${served.length} bytes for a ${art.bytes.length}-byte artifact — ` +
        'chunked and RS reassembly must both preserve the exact original length'
    );
    assert.equal(
      sha256Address(served),
      art.hash,
      'the artifact came back with different bytes than it went in with — reassembly is lossy'
    );
    return undefined;
  }
);
