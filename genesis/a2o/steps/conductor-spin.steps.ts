/* eslint-disable sonarjs/no-os-command-from-path, sonarjs/publicly-writable-directories -- these steps deliberately spawn the two local-mesh host scripts and read the mesh's own /tmp work dir; the same posture as steps/compute-allocation.steps.ts and framework/testnet-manager.ts */
/**
 * Conductor validation-spin step definitions.
 *
 * Backs features/resilience/conductor-validation-spin.feature — the household
 * -scale staging of the fleet event measured on 2026-08-21 (a DNA reinstall
 * re-keyed every conductor; the projections kept referencing the dead
 * incarnations' actions; sys-validation has been retrying those unfetchable
 * dependencies every 10s ever since, saturating each conductor's DHT read
 * pool). Full mechanism and numbers:
 *   genesis/data/timeline/backlog/alpha-conductor-sys-validation-spin-unfetchable-deps.md
 *
 * These steps drive two host scripts rather than reimplementing them, because
 * the measure has to be runnable BY HAND at a desk — an operator watching a
 * conductor spin should reach for the same tool the scenario reaches for, and
 * get the same numbers:
 *
 *   app/elohim-app/scripts/hc-mesh-spin-detector.sh   the measure
 *   app/elohim-app/scripts/hc-mesh-chaos-rekey.sh     the staging
 *
 * PROCESS CONTROL. Both scripts read /proc and signal exact pids, so they are
 * local-mesh-only by construction. Against a deployed fleet every peer is a
 * remote pod: there is no pid to signal. The household fixture says which
 * substrate this is (`processControl`), and these steps refuse rather than
 * pretend — a scenario that silently passes because it never ran is worse than
 * one that says it cannot run here.
 */

import { strict as assert } from 'node:assert';
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { Given, When, Then } from '@cucumber/cucumber';

import { request } from 'undici';

import { loadHouseholdMeshFixture } from '../src/framework/fixtures/household-mesh.js';

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(HERE, '../../..');
const DETECTOR = resolve(REPO_ROOT, 'app/elohim-app/scripts/hc-mesh-spin-detector.sh');
const CHAOS = resolve(REPO_ROOT, 'app/elohim-app/scripts/hc-mesh-chaos-rekey.sh');
const MESH_DIR = process.env['MESH_DIR'] ?? '/tmp/elohim-local-mesh';

/** The peer this story re-keys. James is the household's smallest node. */
const TARGET_PEER = 'james';
/** Run tag: one staging run per cucumber process, resumable by the scripts. */
const RUN_TAG = process.env['CHAOS_REKEY_TAG'] ?? `a2o-${process.pid}`;

// A minute of measurement plus a re-key can take many minutes. Cucumber's
// 5s default would fail these steps on the clock, not on the substrate.
const MINUTE_MS = 60_000;
const DETECTOR_SLACK_MS = 90_000;
const REKEY_TIMEOUT_MS = 12 * MINUTE_MS;

interface LogStreamReading {
  linesPerSec: number;
  saturatedPerSec: number;
  noPeersPerSec: number;
  sysValPerSec: number;
  missingDeps: number | null;
  fetched: number | null;
  missingSeries: string;
  verdict: string;
}

interface DetectorReading {
  verdict: string;
  cycles: number;
  windowSecs: number;
  satThreshold: number;
  conductors: Record<string, { pid: number; avgCores?: number; maxCores?: number; state?: string }>;
  logStreams: Record<string, LogStreamReading>;
}

interface SpinState {
  detector?: DetectorReading;
  authoredIds?: string[];
  rekeyed?: boolean;
  preRekeyRowCount?: number;
  answers?: { id: string; status: number; anchor: string | null; trust: string | null }[];
}

// Module-scoped rather than on the World: the staged re-key is a property of
// the HOST, not of one scenario, and the later scenarios read what the earlier
// ones staged. Putting it on the World would quietly re-key once per scenario.
const state: SpinState = {};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Fail with the substrate fact when this host cannot control mesh processes. */
function requireProcessControl(): void {
  const fixture = loadHouseholdMeshFixture();
  if (fixture.processControl === false) {
    throw new Error(
      `this story stages and measures conductor processes on this host, and ` +
        `${fixture.processControlReason ?? 'this run declared no process control over the mesh'}. ` +
        `Run it against a local mesh (\`just mesh start\`), not a deployed fleet.`
    );
  }
  for (const script of [DETECTOR, CHAOS]) {
    assert.ok(existsSync(script), `required script is missing: ${script}`);
  }
}

/** Storage base URL for a household peer, from the fixture's declared pool. */
function peerStorageUrl(peer: string): string {
  const index = ['matthew', 'jessica', 'james'].indexOf(peer);
  assert.ok(index >= 0, `unknown household peer "${peer}"`);
  const fixture = loadHouseholdMeshFixture();
  const fromFixture = fixture.storagePeers?.[peer]?.url;
  return fromFixture ?? `http://localhost:${8090 + index}`;
}

/** Run one of the host scripts, returning its stdout and exit code. */
function runScript(
  script: string,
  args: string[],
  timeoutMs: number
): { out: string; code: number } {
  const result = spawnSync('bash', [script, ...args], {
    cwd: REPO_ROOT,
    encoding: 'utf8',
    timeout: timeoutMs,
    maxBuffer: 32 * 1024 * 1024,
    env: process.env,
  });
  if (result.error) {
    throw new Error(`${script} ${args.join(' ')} could not run: ${result.error.message}`);
  }
  const out = `${result.stdout ?? ''}${result.stderr ?? ''}`;
  if (result.signal) {
    throw new Error(`${script} was killed by ${result.signal} after ${timeoutMs}ms\n${out}`);
  }
  return { out, code: result.status ?? -1 };
}

/**
 * The detector's LAST stdout line is a one-line JSON summary. Parsing that
 * rather than the human report is deliberate: the report is for a person
 * reading over an operator's shoulder, and reformatting it must never break a
 * scenario.
 */
function parseDetectorJson(out: string): DetectorReading {
  const lines = out.trim().split('\n');
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    const line = lines[i].trim();
    if (line.startsWith('{') && line.endsWith('}')) {
      return JSON.parse(line) as DetectorReading;
    }
  }
  throw new Error(`the spin detector printed no JSON summary:\n${out}`);
}

function reading(): DetectorReading {
  assert.ok(
    state.detector,
    'no spin-detector reading yet — a "watches the household conductors" step must run first'
  );
  return state.detector;
}

function authoredIds(): string[] {
  if (state.authoredIds?.length) return state.authoredIds;
  const idsFile = `${MESH_DIR}/chaos-rekey/${RUN_TAG}.ids`;
  assert.ok(
    existsSync(idsFile),
    `no authored-content list at ${idsFile} — the authoring step must run first`
  );
  state.authoredIds = readFileSync(idsFile, 'utf8').split('\n').filter(Boolean);
  return state.authoredIds;
}

async function fetchContent(peer: string, id: string) {
  const url = `${peerStorageUrl(peer)}/db/content/${encodeURIComponent(id)}`;
  const apiKey = process.env['API_KEY_ADMIN'] ?? 'mesh-admin-dev-key';
  const { statusCode, body } = await request(url, {
    headers: { authorization: `Bearer ${apiKey}` },
    headersTimeout: 15_000,
    bodyTimeout: 15_000,
  });
  const text = await body.text();
  let anchor: string | null = null;
  let trust: string | null = null;
  try {
    const parsed = JSON.parse(text) as Record<string, unknown>;
    anchor = (parsed['dhtAnchorHash'] as string | null) ?? null;
    trust = (parsed['trust'] as string | null) ?? null;
  } catch {
    // Non-JSON body (a 404 page, an empty shed response) — status carries the answer.
  }
  return { status: statusCode, anchor, trust };
}

// ---------------------------------------------------------------------------
// Background
// ---------------------------------------------------------------------------

Given(
  'the household mesh is running with peers {string}, {string} and {string}',
  { timeout: 60_000 },
  async (a: string, b: string, c: string) => {
    requireProcessControl();
    for (const peer of [a, b, c]) {
      const { statusCode } = await request(`${peerStorageUrl(peer)}/health`, {
        headersTimeout: 10_000,
        bodyTimeout: 10_000,
      });
      assert.equal(
        statusCode,
        200,
        `household peer "${peer}" is not serving at ${peerStorageUrl(peer)}`
      );
    }
  }
);

// ---------------------------------------------------------------------------
// The measure
// ---------------------------------------------------------------------------

When(
  'the spin detector watches the household conductors for {int} minutes',
  { timeout: 20 * MINUTE_MS },
  (minutes: number) => {
    requireProcessControl();
    const peerLog = `${REPO_ROOT}/elohim/holochain/local-dev/.sandbox_run_log.${TARGET_PEER}`;
    const args = ['--minutes', String(minutes)];
    // The re-keyed peer runs on its own log stream, so its rates can be
    // attributed to it. `hc sandbox run -a` multiplexes the others into one
    // prefix-less file, which is why that stream is reported as "mesh".
    if (existsSync(peerLog)) {
      args.push('--log', `${TARGET_PEER}=${peerLog}`);
      args.push('--log', `mesh=${REPO_ROOT}/elohim/holochain/local-dev/.sandbox_run_log`);
    }
    const { out } = runScript(DETECTOR, args, minutes * MINUTE_MS + DETECTOR_SLACK_MS);
    state.detector = parseDetectorJson(out);
  }
);

Then("the spin detector's verdict is {string}", (expected: string) => {
  const r = reading();
  const detail = Object.entries(r.logStreams)
    .map(([name, s]) => `${name}: ${s.saturatedPerSec}/s saturated, missing ${s.missingSeries}`)
    .join('; ');
  assert.equal(
    r.verdict,
    expected,
    `spin detector said ${r.verdict}, expected ${expected} — ${detail}`
  );
});

Then('no conductor reports a missing-dependency backlog', () => {
  const r = reading();
  for (const [name, stream] of Object.entries(r.logStreams)) {
    assert.ok(
      stream.missingDeps === null || stream.missingDeps === 0,
      `${name} reports ${stream.missingDeps} missing dependencies on an undisturbed mesh ` +
        `(series ${stream.missingSeries}) — nothing has been staged, so there is nothing to be missing`
    );
  }
});

Then('no conductor accumulates a missing-dependency backlog that stops shrinking', () => {
  const r = reading();
  for (const [name, stream] of Object.entries(r.logStreams)) {
    if (stream.missingDeps === null || stream.missingDeps === 0) continue;
    const series = stream.missingSeries
      .split(',')
      .filter(v => v !== '-' && v !== '')
      .map(Number);
    const last = series.at(-1);
    const shrinking = series.length >= 2 && last !== undefined && last < series[0];
    assert.ok(
      shrinking,
      `${name} holds ${stream.missingDeps} missing dependencies and the backlog is not shrinking ` +
        `(series ${stream.missingSeries}). A backlog that drains is work; one that plateaus is a peer ` +
        `that has not learned to stop asking for actions no living chain can produce.`
    );
  }
});

Then("the conductors' read-pool saturation rate stays under the spin threshold", () => {
  const r = reading();
  for (const [name, stream] of Object.entries(r.logStreams)) {
    assert.ok(
      stream.saturatedPerSec <= r.satThreshold,
      `${name} logged ${stream.saturatedPerSec} read-pool-saturation lines/s, over the ` +
        `${r.satThreshold}/s threshold — the conductor is hammering its own database read pool`
    );
  }
});

// ---------------------------------------------------------------------------
// The staging
// ---------------------------------------------------------------------------

Given('James has authored content on his own peer', { timeout: 9 * MINUTE_MS }, () => {
  requireProcessControl();
  const { out, code } = runScript(
    CHAOS,
    ['--peer', TARGET_PEER, '--tag', RUN_TAG, '--phase', 'author'],
    8 * MINUTE_MS
  );
  assert.equal(code, 0, `authoring content on ${TARGET_PEER} failed:\n${out}`);
  state.authoredIds = undefined;
  assert.ok(authoredIds().length > 0, `no content was authored on ${TARGET_PEER}:\n${out}`);
});

Given(
  "that content is anchored on James's own conductor, signed onto his chain",
  { timeout: 5 * MINUTE_MS },
  async () => {
    for (const id of authoredIds()) {
      const { status, anchor } = await fetchContent(TARGET_PEER, id);
      assert.equal(
        status,
        200,
        `${TARGET_PEER} does not serve its own content ${id} (HTTP ${status})`
      );
      assert.ok(
        anchor,
        `${id} has no DHT anchor on ${TARGET_PEER} — it was never authored onto his conductor's chain, ` +
          `so re-keying him would make nothing unfetchable and the staging would prove nothing`
      );
    }
  }
);

Given(
  "Matthew's and Jessica's peers hold that content under James's anchor",
  { timeout: 11 * MINUTE_MS },
  () => {
    const { out, code } = runScript(
      CHAOS,
      ['--peer', TARGET_PEER, '--tag', RUN_TAG, '--phase', 'witness'],
      10 * MINUTE_MS
    );
    assert.equal(
      code,
      0,
      `waiting for the other peers to hold ${TARGET_PEER}'s content failed:\n${out}`
    );
    // Bounded quantifiers: a peer name and a count have known small sizes, and an
    // unbounded \\w+/\\d+ pair on script output is a backtracking hazard.
    const held = [...out.matchAll(/(\w{1,32}) holds (\d{1,9})\/(\d{1,9})/g)];
    assert.ok(held.length > 0, `the witness phase reported no holdings:\n${out}`);
    for (const [, peer, count] of held) {
      assert.ok(
        Number(count) > 0,
        `${peer} holds none of ${TARGET_PEER}'s content, so re-keying him takes nothing away from ${peer} ` +
          `and the staging would prove nothing:\n${out}`
      );
    }
  }
);

Given(
  'the content James already stored stays on his disk through the re-key',
  { timeout: 2 * MINUTE_MS },
  async () => {
    // The staging keeps the peer's storage database and destroys only its
    // conductor sandbox — that asymmetry IS the alpha shape: rows describing
    // content the machine still holds, signed by a chain that no longer exists.
    // Wiping both would be a clean reinstall, which reproduces nothing.
    const { statusCode, body } = await request(
      `${peerStorageUrl(TARGET_PEER)}/db/content?limit=1`,
      {
        headersTimeout: 15_000,
        bodyTimeout: 15_000,
      }
    );
    assert.equal(statusCode, 200, `${TARGET_PEER} does not answer a content listing`);
    const listing = JSON.parse(await body.text()) as { count?: number };
    state.preRekeyRowCount = listing.count ?? 0;
    assert.ok(
      authoredIds().length > 0,
      `${TARGET_PEER} has no authored content staged, so keeping his disk preserves nothing`
    );
  }
);

When(
  "James's peer is re-keyed, taking a new conductor identity",
  { timeout: REKEY_TIMEOUT_MS + MINUTE_MS },
  () => {
    requireProcessControl();
    const { out, code } = runScript(
      CHAOS,
      ['--peer', TARGET_PEER, '--tag', RUN_TAG, '--phase', 'rekey'],
      REKEY_TIMEOUT_MS
    );
    assert.equal(code, 0, `re-keying ${TARGET_PEER} failed:\n${out}`);
    assert.match(
      out,
      /agent key re-keyed: /,
      `${TARGET_PEER} did not come back under a NEW agent key, so nothing became unfetchable:\n${out}`
    );
    state.rekeyed = true;
  }
);

Given("James's peer has been re-keyed after authoring content", () => {
  const stateFile = `${MESH_DIR}/chaos-rekey/${RUN_TAG}.state`;
  const staged =
    state.rekeyed ??
    (existsSync(stateFile) && readFileSync(stateFile, 'utf8').includes('REKEYED_AT='));
  assert.ok(
    staged,
    `${TARGET_PEER} has not been re-keyed in this run (no REKEYED_AT in ${stateFile}). ` +
      `Run the re-key scenario first, or stage it by hand: ` +
      `bash app/elohim-app/scripts/hc-mesh-chaos-rekey.sh --peer ${TARGET_PEER} --tag ${RUN_TAG}`
  );
});

When("Matthew asks James's peer for that content", { timeout: 5 * MINUTE_MS }, async () => {
  const answers = [];
  for (const id of authoredIds()) {
    answers.push({ id, ...(await fetchContent(TARGET_PEER, id)) });
  }
  state.answers = answers;
});

Then(
  "James's peer either re-adopts that content under his new key or reports it as unfetchable",
  () => {
    const answers = state.answers;
    assert.ok(answers?.length, 'no answers recorded — the asking step must run first');
    for (const a of answers) {
      const readopted = a.status === 200 && Boolean(a.anchor);
      const honestlyAbsent = a.status === 404 || a.status === 410;
      assert.ok(
        readopted || honestlyAbsent,
        `${a.id}: HTTP ${a.status}, anchor ${a.anchor ?? 'none'}, trust ${a.trust ?? 'none'}. ` +
          `A re-keyed peer owes one of two honest answers — "I have this again, under my new key" ` +
          `(200 with a live anchor) or "I cannot prove this any more" (404/410). This is neither.`
      );
    }
  }
);

Then("James's peer does not answer with silence while its conductor grinds", () => {
  const answers = state.answers ?? [];
  for (const a of answers) {
    assert.notEqual(
      a.status,
      503,
      `${a.id}: ${TARGET_PEER} shed the request (503) instead of answering. Shedding under a ` +
        `conductor that is burning CPU on unfetchable dependencies is the failure this story names: ` +
        `the household gets neither the content nor the truth about it.`
    );
    assert.ok(
      a.status < 500,
      `${a.id}: ${TARGET_PEER} answered HTTP ${a.status} — a server-side failure, not an honest absence`
    );
  }
});
