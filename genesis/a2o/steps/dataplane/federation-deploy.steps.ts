/**
 * Glue for `features/dataplane/federation-deploy.feature`'s FINAL scenario —
 * "two doorways that disagree about a page converge on the elected version
 * without anyone re-uploading it".
 *
 * The feature's earlier scenarios are not this file's: two are bound by existing
 * primitives in `steps/dataplane.steps.ts`, and the staging-authority scenario
 * stays `@wip` pending its own deploy. This file exists only for the
 * version-divergence scenario, whose Given/When/Then have no counterpart
 * anywhere in the suite.
 *
 * ## What this fixture is, and what it must never become
 *
 * The scenario's whole claim is ORGANIC convergence: a peer holding the older
 * head moves to the elected head because its OWN reconcile sweep obeyed an
 * election its OWN conductor re-derived in wasm. No declaration call, no
 * per-host upload, no doorway credential participates in the cure. A fixture
 * that nudged the disagreeing peer — with a declaration POST, a blob upload,
 * or an operator reconcile verb — would still go green and would prove
 * nothing. That false-green is the named regression shape in the plan's Global
 * Constraints (`genesis/docs/superpowers/plans/2026-09-05-dataplane-convergence-final-scenario-plan.md`).
 *
 * So the guard here is STRUCTURAL, not a promise. Every mutating call the
 * fixture can make goes through the append-only write ledger in
 * `src/framework/dataplane/carried-election.ts`. The WHEN step snapshots the
 * ledger's length before it starts waiting; the convergence assertion refuses
 * to pass if the ledger grew. The receipt carries the ledger verbatim, so a
 * reader sees exactly which writes staged the divergence and that none
 * followed.
 *
 * ## The staging path is the 2026-08-31 proof's, not a second one
 *
 * `scripts/carried-election-mesh-proof.ts` proved carry-the-election on the
 * household mesh. Its staging helpers now live in
 * `src/framework/dataplane/carried-election.ts` and BOTH call sites import
 * them, so this scenario exercises the mechanism that was actually measured
 * rather than a lookalike.
 *
 * ## Reuse over reinvention
 *
 * Three of the scenario's assertions are NOT here because the suite already
 * carries them:
 *   - cross-peer declared-head equality → `probeDeclaredHead`
 *     (`src/framework/dataplane/surfaces.ts`), the same helper the
 *     resiliency-saga ch10 comparator and failover concern both use.
 *   - served-versus-declared per doorway → the existing step "the served head
 *     for EPR {string} matches the declared head on peer {string}"
 *     (`steps/dataplane.steps.ts`), which reads the content row's declared
 *     `serverBlobHash` and the running doorway's `servedBundleHeads[]`
 *     attestation. The feature calls it twice, once per doorway; this file
 *     defines nothing for it.
 *   - labelled Prometheus reads → `parseLabeledPrometheusMetric` (surfaces.ts).
 *
 * ## Where this runs
 *
 * The household mesh only (`@requires:household-nodes`): the mechanism
 * assertions call `content_store` coordinator externs on two conductors'
 * admin/app websockets, which exist on the local mesh
 * (`app/elohim-app/scripts/hc-mesh.sh`) and not on a fleet doorway. Run it with
 * `just test mesh genesis/a2o/features/dataplane/federation-deploy.feature`.
 */

/* eslint-disable sonarjs/publicly-writable-directories --
 * MESH_ROOT defaults to /tmp/elohim-local-mesh because that is where
 * `app/elohim-app/scripts/hc-mesh.sh` actually puts the household mesh's per-peer
 * directories; this fixture reads and restores a file the mesh itself owns there.
 * `steps/delivery/happ-lineage-migration.steps.ts` carries the same disable for the
 * same constant and the same reason. */

import { strict as assert } from 'node:assert';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';

import { Given, When, Then, AfterAll } from '@cucumber/cucumber';

import {
  authorDeclare,
  canonicalElectionEvidence,
  connectConductor,
  meshConductorPorts,
  resetStagingWrites,
  servedHead,
  stagingWriteCount,
  stagingWrites,
  tamperLinkRecord,
  verifyCarriedElection,
  type CarriedElectionRail,
  type StagingWrite,
} from '../../src/framework/dataplane/carried-election.js';
import {
  getRaw,
  parseLabeledPrometheusMetric,
  postRaw,
  probeDeclaredHead,
  resolvePeerUrl,
} from '../../src/framework/dataplane/surfaces.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** The obey-path series and label the mechanism assertion reads. */
const OBEYED_SERIES = 'elohim_content_election_obeyed_total';
const OBEYED_PATH_LABEL = 'path';
const OBEYED_PATH_PEER_CARRIED = 'peer_carried';

/** The hot-reloadable operator flag this scenario turns on for the run. */
const OBEY_FLAG = 'ELOHIM_OBEY_CARRIED_ELECTION';

const RUNTIME_CONFIG_RELOAD_PATH = '/admin/runtime-config/reload';

/** Same default as `steps/delivery/happ-lineage-migration.steps.ts`. */
const MESH_ROOT = process.env['E2E_MESH_ROOT'] ?? '/tmp/elohim-local-mesh';

/**
 * How long the WHEN step waits for the ORGANIC sweep.
 *
 * `PROJECTION_RECONCILE_SECS` defaults to 300s in elohim-storage
 * (`src/main.rs`), so a single sweep can be five minutes away at the moment the
 * divergence is staged. The 2026-08-31 proof budgeted 6 minutes and converged
 * in about 2; 12 minutes here is two full cadences plus the gossip the election
 * link needs, which is the honest floor for "the sweep did not run" versus "the
 * sweep ran and refused".
 */
const CONVERGENCE_TIMEOUT_MS = Number(
  process.env['E2E_CARRIED_ELECTION_TIMEOUT_MS'] ?? String(12 * 60_000)
);
const CONVERGENCE_POLL_INTERVAL_MS = 10_000;

/** Cucumber ceiling for the steps that ride the sweep. */
const SWEEP_STEP_TIMEOUT_MS = CONVERGENCE_TIMEOUT_MS + 120_000;

/** Where the organic-path receipt lands (gitignored, durable across container restarts). */
const RECEIPT_DIR =
  process.env['E2E_CARRIED_ELECTION_RECEIPT_DIR'] ??
  path.join(process.cwd(), 'reports', 'dataplane');

// ---------------------------------------------------------------------------
// Peer resolution — alias → doorway URL, storage URL, mesh peer, conductor ports
// ---------------------------------------------------------------------------

interface PeerRef {
  /** The alias as the feature writes it ("alpha-A", "elohim.host"). */
  alias: string;
  /** Household-mesh peer name — the runtime-config directory key. */
  meshPeer: string;
  /** Index into the mesh port scheme. */
  index: number;
  /** Doorway base URL (what a browser reaches). */
  doorwayUrl: string;
  /** elohim-storage base URL (what the conductor and /metrics live behind). */
  storageUrl: string;
}

/**
 * The two peers this scenario names, in the feature's own vocabulary.
 *
 * `alpha-A` is the feature's declared AUTHOR peer and maps to the mesh's first
 * peer; `elohim.host` is the second federation doorway and maps to the mesh's
 * second. `just test mesh` exports the storage URLs under two different name
 * shapes depending on which script emitted them (`E2E_STORAGE_URL` /
 * `E2E_STORAGE_B` from the recipe, `E2E_STORAGE_<PEER>` per roster entry), so
 * each alias carries an ordered fallback chain rather than one env name.
 */
const PEER_LAYOUT: Record<string, { meshPeer: string; index: number; storageEnv: string[] }> = {
  'alpha-A': {
    meshPeer: 'matthew',
    index: 0,
    storageEnv: ['E2E_STORAGE_ALPHA', 'E2E_STORAGE_URL', 'E2E_STORAGE_MATTHEW'],
  },
  'elohim.host': {
    meshPeer: 'jessica',
    index: 1,
    storageEnv: ['E2E_STORAGE_ELOHIM_HOST', 'E2E_STORAGE_B', 'E2E_STORAGE_JESSICA'],
  },
};

function resolvePeer(alias: string): PeerRef {
  const layout = PEER_LAYOUT[alias];
  assert.ok(
    layout,
    `federation-deploy's divergence scenario knows only ${Object.keys(PEER_LAYOUT)
      .map(a => `"${a}"`)
      .join(' and ')}, not "${alias}"`
  );
  const storageUrl = layout.storageEnv.map(name => process.env[name]).find(Boolean);
  assert.ok(
    storageUrl,
    `no storage URL for peer "${alias}" — set one of ${layout.storageEnv.join(', ')} ` +
      `(\`just test mesh\` exports these; bring the mesh up with \`just mesh start && just mesh prologue\`)`
  );
  return {
    alias,
    meshPeer: layout.meshPeer,
    index: layout.index,
    doorwayUrl: resolvePeerUrl(alias).replace(/\/$/, ''),
    storageUrl: storageUrl.replace(/\/$/, ''),
  };
}

/**
 * Conductor admin/app ports for a peer. `E2E_CONDUCTORS` (the same
 * `name=admin:app,...` CSV shape `scripts/release-ceremony.ts` takes) overrides
 * the mesh port scheme when a run's conductors sit somewhere else.
 */
function conductorPorts(peer: PeerRef): { adminPort: number; appPort: number } {
  const csv = process.env['E2E_CONDUCTORS'];
  if (csv) {
    for (const entry of csv.split(',')) {
      const [name, ports] = entry.split('=');
      if (name?.trim() !== peer.meshPeer) continue;
      const [adminPort, appPort] = (ports ?? '').split(':').map(Number);
      if (Number.isFinite(adminPort) && Number.isFinite(appPort)) return { adminPort, appPort };
    }
  }
  return meshConductorPorts(peer.index);
}

// ---------------------------------------------------------------------------
// Per-world state
// ---------------------------------------------------------------------------

interface DivergenceState {
  eprId: string;
  /** The peer that authors the winning revision and carries the EARNED canonical. */
  winner: PeerRef;
  /** The peer left holding the older head — the one whose sweep must move it. */
  laggard: PeerRef;
  winnerHead: string;
  laggardHead: string;
  winnerRail: CarriedElectionRail;
  laggardRail: CarriedElectionRail;
  /** Set by the EARNED-canonical step. */
  earnedDeclaredHead?: string;
  /** Ledger length at the moment the WHEN phase opened. */
  ledgerAtWhen?: number;
  /** `obeyed{path="peer_carried"}` on the laggard before the wait. `null` = series absent. */
  obeyedBefore?: number | null;
  /** `obeyed{path="peer_carried"}` on the laggard after convergence. `null` = series absent. */
  obeyedAfter?: number | null;
  /** What the laggard served when the wait ended. */
  laggardServedAfter?: string | null;
  converged?: boolean;
  waitedMs?: number;
}

const states = new WeakMap<E2EWorld, DivergenceState>();

function state(world: E2EWorld): DivergenceState {
  const s = states.get(world);
  assert.ok(
    s,
    'no staged divergence in this scenario — the Given that declares two disagreeing heads must run first'
  );
  return s;
}

// ---------------------------------------------------------------------------
// The operator flag, through the runtime-config the fixture byte-restores
// ---------------------------------------------------------------------------

function runtimeConfigPath(peer: PeerRef): string {
  return path.join(MESH_ROOT, peer.meshPeer, 'runtime-config.toml');
}

/** Original on-disk bytes per mesh peer, captured once, restored in AfterAll. */
const originalRuntimeConfigBytes = new Map<string, { file: string; bytes: Buffer | null }>();
const touchedPeers = new Map<string, PeerRef>();

/**
 * Turn the operator flag ON in one peer's runtime-config and force a re-read.
 *
 * Unlike `steps/delivery/happ-lineage-migration.steps.ts`'s run-owned follow
 * set, this PRESERVES the rest of the file. That file's rule exists because a
 * follow set is a LIST whose leftovers are hazardous (a peer left following a
 * torn-down world's channel). A single boolean has no leftover-list hazard,
 * while the mesh writes its own boot settings into the same file
 * (`ELOHIM_RUNTIME_CONFIG_PATH` is set per peer at mesh start), so writing a
 * run-owned file containing only this key would strip the mesh's own
 * configuration for the length of the run.
 *
 * The flag is not asserted as "set"; it is asserted as EFFECTIVE — the reload
 * answer carries the full settings report, and this reads the flag's
 * `effectiveValue` back out of it. Writing a file is intention; the running
 * process agreeing is evidence.
 */
async function enableObeyFlag(peer: PeerRef): Promise<void> {
  const file = runtimeConfigPath(peer);
  if (!originalRuntimeConfigBytes.has(peer.meshPeer)) {
    originalRuntimeConfigBytes.set(peer.meshPeer, {
      file,
      bytes: existsSync(file) ? readFileSync(file) : null,
    });
    touchedPeers.set(peer.meshPeer, peer);
  }
  const previous = existsSync(file) ? readFileSync(file, 'utf8') : '';
  // Split/filter/join rather than a regex: a trailing-newline regex on
  // attacker-shaped input is a backtracking hazard the linter (rightly) refuses,
  // and dropping empty trailing lines by array is both cheaper and obvious.
  const keptLines = previous.split('\n').filter(line => !line.trim().startsWith(OBEY_FLAG));
  while (keptLines.length > 0 && keptLines.at(-1)?.trim() === '') keptLines.pop();
  const preserved = keptLines.length > 0 ? `${keptLines.join('\n')}\n` : '';
  mkdirSync(path.dirname(file), { recursive: true });
  writeFileSync(file, `${preserved}${OBEY_FLAG} = "1"\n`, 'utf8');

  const { status, text } = await postRaw(`${peer.storageUrl}${RUNTIME_CONFIG_RELOAD_PATH}`);
  assert.ok(
    status >= 200 && status < 300,
    `${peer.alias}: POST ${RUNTIME_CONFIG_RELOAD_PATH} returned ${status} — the operator flag could not be enabled (${text.slice(0, 200)})`
  );
  let report: unknown;
  try {
    report = JSON.parse(text);
  } catch {
    throw new Error(
      `${peer.alias}: ${RUNTIME_CONFIG_RELOAD_PATH} answered non-JSON: ${text.slice(0, 200)}`
    );
  }
  const settings = (
    report as { report?: { settings?: { name?: string; effectiveValue?: unknown }[] } }
  ).report?.settings;
  const entry = settings?.find(s => s.name === OBEY_FLAG);
  assert.ok(
    entry,
    `${peer.alias}: the reload report names no ${OBEY_FLAG} setting — this build predates the flag, so the capability under test is not present`
  );
  assert.strictEqual(
    entry.effectiveValue,
    true,
    `${peer.alias}: ${OBEY_FLAG} reads effectiveValue=${JSON.stringify(entry.effectiveValue)} after the reload — ` +
      `the flag did not take, so any convergence below would not be the carried-election path`
  );
}

AfterAll({ timeout: 120_000 }, async function () {
  for (const [meshPeer, peer] of touchedPeers) {
    const original = originalRuntimeConfigBytes.get(meshPeer);
    if (!original) continue;
    try {
      if (original.bytes === null) {
        writeFileSync(original.file, '', 'utf8');
      } else {
        writeFileSync(original.file, original.bytes);
      }
      await postRaw(`${peer.storageUrl}${RUNTIME_CONFIG_RELOAD_PATH}`);
      console.error(`[federation-deploy] ${meshPeer}'s runtime-config.toml byte-restored.`);
    } catch (error) {
      console.error(
        `[federation-deploy] restore of ${meshPeer}'s runtime-config.toml failed: ${String(error)}`
      );
    }
  }
  touchedPeers.clear();
  originalRuntimeConfigBytes.clear();
});

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

/**
 * Read `elohim_content_election_obeyed_total{path="peer_carried"}` on a peer's
 * OWN storage /metrics — never the doorway's, which does not register the
 * `elohim_`-prefixed series (the same routing rule
 * `steps/dataplane/resiliency-saga.steps.ts` documents).
 *
 * `null` means the series is structurally absent, which is a different fact
 * from zero and is reported as such.
 */
async function obeyedPeerCarried(peer: PeerRef): Promise<number | null> {
  const { status, text } = await getRaw(`${peer.storageUrl}/metrics`);
  assert.strictEqual(
    status,
    200,
    `${peer.alias}: GET /metrics returned ${status} — cannot read the obey-path counter`
  );
  return parseLabeledPrometheusMetric(
    text,
    OBEYED_SERIES,
    OBEYED_PATH_LABEL,
    OBEYED_PATH_PEER_CARRIED
  );
}

// ---------------------------------------------------------------------------
// Receipt
// ---------------------------------------------------------------------------

interface OrganicReceipt {
  scenario: string;
  eprId: string;
  measuredAt: string;
  winner: { alias: string; meshPeer: string; storageUrl: string; head: string };
  laggard: { alias: string; meshPeer: string; storageUrl: string; headBefore: string };
  operatorFlag: { name: string; effective: true; enabledOn: string[] };
  organicPath: {
    /** Every mutating call the fixture made, in order. */
    stagingWrites: StagingWrite[];
    /** Ledger length when the WHEN phase opened. */
    ledgerAtWhen: number;
    /** Ledger length when convergence was asserted — equal, or the run is not organic. */
    ledgerAtThen: number;
    declarationCallsDuringCure: 0;
  };
  convergence: {
    converged: boolean;
    waitedMs: number;
    laggardServedHead: string | null;
    electedHead: string;
  };
  mechanism: {
    obeyedPeerCarriedBefore: number | null;
    obeyedPeerCarriedAfter: number | null;
  };
}

function writeReceipt(receipt: OrganicReceipt): string {
  mkdirSync(RECEIPT_DIR, { recursive: true });
  const file = path.join(
    RECEIPT_DIR,
    `carried-election-organic-receipt-${receipt.measuredAt.replace(/[:.]/g, '-')}.json`
  );
  writeFileSync(file, `${JSON.stringify(receipt, null, 2)}\n`, 'utf8');
  return file;
}

// ---------------------------------------------------------------------------
// Given — stage the divergence
// ---------------------------------------------------------------------------

/**
 * Stage the disagreement the scenario then asks the substrate to heal.
 *
 * ORDER IS LOAD-BEARING. The SECOND-named peer declares FIRST, so the
 * FIRST-named peer (`alpha-A` — the feature's own declared author peer) ends up
 * holding both the EARNED tier and the LATER notarized declaration timestamp.
 * That makes the election's two rules — earned beats staging, then newest
 * timestamp — point at the same winner, so a green here cannot be a tiebreak
 * that happened to fall the right way.
 *
 * The staging is deliberately non-destructive on a real seeded EPR: the row is
 * created only if absent, the PATCH mints a new notarized action without
 * changing the page's bytes, and neither blob pointer moves. See
 * `authorDeclare`'s doc for the traced behaviour.
 *
 * Example:
 *   Given peer "alpha-A" and peer "elohim.host" both declare a head for EPR "elohim-host-landing"
 */
Given(
  'peer {string} and peer {string} both declare a head for EPR {string}',
  { timeout: 180_000 },
  async function (this: E2EWorld, winnerAlias: string, laggardAlias: string, eprId: string) {
    resetStagingWrites();
    const winner = resolvePeer(winnerAlias);
    const laggard = resolvePeer(laggardAlias);

    const winnerPorts = conductorPorts(winner);
    const laggardPorts = conductorPorts(laggard);
    const winnerRail = await connectConductor(winnerPorts.adminPort, winnerPorts.appPort);
    const laggardRail = await connectConductor(laggardPorts.adminPort, laggardPorts.appPort);
    this.onCleanup(async () => {
      await winnerRail.close().catch(() => undefined);
      await laggardRail.close().catch(() => undefined);
    });

    const stamp = new Date().toISOString();
    const laggardHead = await authorDeclare({
      storageUrl: laggard.storageUrl,
      id: eprId,
      body: `# ${eprId}\n\nRevision authored on ${laggard.alias} at ${stamp} — the older head.`,
      agent: laggardRail.agent,
    });
    const winnerHead = await authorDeclare({
      storageUrl: winner.storageUrl,
      id: eprId,
      body: `# ${eprId}\n\nRevision authored on ${winner.alias} at ${stamp} — the newer head.`,
      agent: winnerRail.agent,
    });

    states.set(this, {
      eprId,
      winner,
      laggard,
      winnerHead,
      laggardHead,
      winnerRail,
      laggardRail,
    });
  }
);

/**
 * The precondition the whole scenario rests on: two peers, one EPR, two
 * different declared heads. A fixture whose staging silently converged would
 * make every assertion below vacuous, so this is a hard failure, never a skip.
 *
 * Defined with `Given` because it sits in the feature's Given block: it states a
 * precondition of the run, not an outcome of it.
 *
 * Example:
 *   And their declared heads DISAGREE
 */
Given('their declared heads DISAGREE', function (this: E2EWorld) {
  const s = state(this);
  assert.notStrictEqual(
    s.winnerHead,
    s.laggardHead,
    `staging failed to diverge for "${s.eprId}": both ${s.winner.alias} and ${s.laggard.alias} declared ${s.winnerHead} — ` +
      `there is no disagreement for the sweep to heal, so nothing below would measure anything`
  );
});

/**
 * Put the EARNED canonical declaration on the winner's own conductor.
 *
 * `declare_earned_canonical_head` is the authority path — the tier the election
 * rule prefers over a deploy/seed STAGING declaration. This is the same call
 * the 2026-08-31 proof made, on the same peer role.
 *
 * Example:
 *   And an EARNED canonical declaration exists for the newer head on its declaring peer
 */
Given(
  'an EARNED canonical declaration exists for the newer head on its declaring peer',
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const s = state(this);
    const earned = (await s.winnerRail.call('declare_earned_canonical_head', {
      id: s.eprId,
      head_action_hash: s.winnerHead,
      carried_record: null,
      adopt_before_author: false,
      delegation: null,
    })) as { canonical?: boolean } | null;
    assert.ok(
      earned,
      `${s.winner.alias}: declare_earned_canonical_head answered nothing for "${s.eprId}"`
    );
    assert.strictEqual(
      earned.canonical,
      true,
      `${s.winner.alias}: declare_earned_canonical_head did not mint a canonical declaration for "${s.eprId}" ` +
        `(answered canonical=${JSON.stringify(earned.canonical)}) — there is no earned tier for the election to prefer`
    );
    s.earnedDeclaredHead = s.winnerHead;
  }
);

/**
 * Enable the dormant capability for the length of this run, through the
 * runtime-config file the fixture byte-restores in `AfterAll`.
 *
 * Both peers get it because the feature says "on the fleet" and both are the
 * federation under test; the laggard is the one whose obey arm it actually
 * unlocks.
 *
 * Example:
 *   And carried elections are enabled on the fleet via the operator flag ELOHIM_OBEY_CARRIED_ELECTION
 */
Given(
  'carried elections are enabled on the fleet via the operator flag ELOHIM_OBEY_CARRIED_ELECTION',
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const s = state(this);
    await enableObeyFlag(s.laggard);
    await enableObeyFlag(s.winner);
  }
);

// ---------------------------------------------------------------------------
// When — wait for the ORGANIC sweep. Read-only by construction.
// ---------------------------------------------------------------------------

/**
 * Wait for the peer holding the older head to move it ON ITS OWN.
 *
 * This step makes NO mutating call. It snapshots the staging write ledger and
 * the obey-path counter, then polls one read surface — `GET
 * /db/content/{id}/head` on the laggard — until the head matches the elected
 * one or the budget runs out. The verdict is recorded, not asserted: the Then
 * steps own the assertions, so a non-convergence is reported against the
 * outcome the story names rather than as an opaque timeout here.
 *
 * Example:
 *   When the reconcile sweep runs on the peer holding the older head
 */
When(
  'the reconcile sweep runs on the peer holding the older head',
  { timeout: SWEEP_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    const s = state(this);
    s.ledgerAtWhen = stagingWriteCount();
    s.obeyedBefore = await obeyedPeerCarried(s.laggard);

    const started = Date.now();
    const deadline = started + CONVERGENCE_TIMEOUT_MS;
    let served: string | null = null;
    let converged = false;
    while (Date.now() < deadline) {
      served = await servedHead(s.laggard.storageUrl, s.eprId);
      if (served === s.winnerHead) {
        converged = true;
        break;
      }
      await new Promise(resolve => setTimeout(resolve, CONVERGENCE_POLL_INTERVAL_MS));
    }
    s.waitedMs = Date.now() - started;
    s.laggardServedAfter = served;
    s.converged = converged;
    s.obeyedAfter = await obeyedPeerCarried(s.laggard);
  }
);

// ---------------------------------------------------------------------------
// Then — the visitor-facing outcome
// ---------------------------------------------------------------------------

/**
 * The outcome, plus the organic-path guard that makes the outcome mean
 * anything. Both halves fail this step, and the ledger check runs FIRST: a
 * converged head reached by a fixture write is worse than a red, because it
 * would read as a pass.
 *
 * Example:
 *   Then the peer's served head moves to the earned-tier elected head
 */
Then(
  "the peer's served head moves to the earned-tier elected head",
  { timeout: 60_000 },
  // Synchronous on purpose: the WHEN step already did every read this needs, so
  // asserting here costs no I/O and must not silently re-probe a surface that
  // could have moved between the wait ending and the verdict.
  function (this: E2EWorld) {
    const s = state(this);
    const ledgerAtThen = stagingWriteCount();
    assert.strictEqual(
      ledgerAtThen,
      s.ledgerAtWhen,
      `ORGANIC-PATH VIOLATION: the fixture made ${ledgerAtThen - (s.ledgerAtWhen ?? 0)} mutating call(s) ` +
        `after the divergence was staged — ${JSON.stringify(stagingWrites().slice(s.ledgerAtWhen))}. ` +
        `A head that moved with fixture help proves nothing about the substrate.`
    );

    const receiptPath = writeReceipt({
      scenario: 'two doorways that disagree about a page converge on the elected version',
      eprId: s.eprId,
      measuredAt: new Date().toISOString(),
      winner: {
        alias: s.winner.alias,
        meshPeer: s.winner.meshPeer,
        storageUrl: s.winner.storageUrl,
        head: s.winnerHead,
      },
      laggard: {
        alias: s.laggard.alias,
        meshPeer: s.laggard.meshPeer,
        storageUrl: s.laggard.storageUrl,
        headBefore: s.laggardHead,
      },
      operatorFlag: {
        name: OBEY_FLAG,
        effective: true,
        enabledOn: [...touchedPeers.keys()],
      },
      organicPath: {
        stagingWrites: [...stagingWrites()],
        ledgerAtWhen: s.ledgerAtWhen ?? 0,
        ledgerAtThen,
        declarationCallsDuringCure: 0,
      },
      convergence: {
        converged: s.converged === true,
        waitedMs: s.waitedMs ?? 0,
        laggardServedHead: s.laggardServedAfter ?? null,
        electedHead: s.winnerHead,
      },
      mechanism: {
        obeyedPeerCarriedBefore: s.obeyedBefore ?? null,
        obeyedPeerCarriedAfter: s.obeyedAfter ?? null,
      },
    });
    // eslint-disable-next-line no-console
    console.log(`  organic-path receipt: ${receiptPath}`);

    assert.strictEqual(
      s.laggardServedAfter,
      s.winnerHead,
      `${s.laggard.alias} still serves ${s.laggardServedAfter ?? 'nothing'} for "${s.eprId}" after ` +
        `${Math.round((s.waitedMs ?? 0) / 1000)}s — the earned-tier elected head is ${s.winnerHead}. ` +
        `obeyed{path="peer_carried"} went ${s.obeyedBefore ?? 'absent'} -> ${s.obeyedAfter ?? 'absent'}; ` +
        `receipt: ${receiptPath}`
    );
  }
);

/**
 * The visitor-facing promise, read at the DOORWAYS rather than at storage: a
 * person cannot tell which of the two front doors they reached.
 *
 * Delegates to `probeDeclaredHead` — the shared helper the resiliency-saga
 * ch10 comparator and the failover concern both use — and reads both doorways
 * concurrently, because a cross-doorway head comparison is only meaningful if
 * both sides are read at the same instant.
 *
 * Example:
 *   And both doorways serve the SAME head for EPR "elohim-host-landing"
 */
Then(
  'both doorways serve the SAME head for EPR {string}',
  { timeout: 120_000 },
  async function (this: E2EWorld, eprId: string) {
    const s = state(this);
    const read = async (peer: PeerRef) => {
      try {
        return await probeDeclaredHead(peer.doorwayUrl, eprId);
      } catch (err) {
        throw new Error(
          `${err instanceof Error ? err.message : String(err)} (doorway for peer "${peer.alias}")`
        );
      }
    };
    const [a, b] = await Promise.all([read(s.winner), read(s.laggard)]);
    assert.ok(
      a.declared && b.declared,
      `a doorway reports declared=false for "${eprId}" (${s.winner.alias}=${a.declared}, ${s.laggard.alias}=${b.declared}) — ` +
        `an anchor fallback is not a declared head, and comparing fallbacks would pass without any election`
    );
    assert.strictEqual(
      a.headActionHash,
      b.headActionHash,
      `the two doorways serve different heads for "${eprId}": ${s.winner.alias}=${a.headActionHash} vs ` +
        `${s.laggard.alias}=${b.headActionHash} — a visitor's experience still depends on which front door they reach`
    );
  }
);

// ---------------------------------------------------------------------------
// Then — the mechanism, so the outcome is not a trust-the-peer copy
// ---------------------------------------------------------------------------

/**
 * The move must have been an ELECTION OBEYED through the peer-carried path.
 *
 * `elohim_content_election_obeyed_total{path="peer_carried"}` is incremented in
 * `elohim/elohim-storage/src/services/head_adoption.rs` only on the branch
 * where the row moved after this node's OWN conductor re-derived a carried
 * declaration link in wasm. A head that converged by any other route (a
 * locally-visible election, a fixture write, an operator verb) leaves this
 * series flat, which is exactly the discrimination this assertion buys.
 *
 * An ABSENT series is reported as absent, never read as zero — the 2026-08-03
 * lesson that a structurally-missing series and a measured zero are different
 * facts.
 *
 * Example:
 *   And that peer's conductor verified the carried declaration link in wasm before moving it
 */
Then(
  "that peer's conductor verified the carried declaration link in wasm before moving it",
  { timeout: 60_000 },
  function (this: E2EWorld) {
    const s = state(this);
    assert.notStrictEqual(
      s.obeyedAfter,
      null,
      `${s.laggard.alias}: ${OBEYED_SERIES}{${OBEYED_PATH_LABEL}="${OBEYED_PATH_PEER_CARRIED}"} is structurally ABSENT ` +
        `from /metrics — that is "never measured", not "measured zero", and this build cannot answer whether the ` +
        `carried path carried the move`
    );
    const before = s.obeyedBefore ?? 0;
    const after = s.obeyedAfter ?? 0;
    assert.ok(
      after > before,
      `${s.laggard.alias}: ${OBEYED_SERIES}{${OBEYED_PATH_LABEL}="${OBEYED_PATH_PEER_CARRIED}"} did not move ` +
        `(${before} -> ${after}) — the head converged by some OTHER route, so this run does not prove the ` +
        `peer-carried election path`
    );
  }
);

/**
 * Re-derive the election on the laggard's OWN conductor and read the two rules
 * back out of the wasm answer: the winner it names, and that the winning
 * declaration is the EARNED one (`canonical_earned`), carrying the notarized
 * `canonical_declared_at` timestamp the tiebreak reads.
 *
 * This is a READ — `verify_carried_election` proves bytes, it does not author.
 * The staging ledger is asserted unchanged by the outcome step above, and this
 * step adds nothing to it.
 *
 * Example:
 *   And the election obeyed earned-beats-staging, with ties broken on the notarized declaration timestamp
 */
Then(
  'the election obeyed earned-beats-staging, with ties broken on the notarized declaration timestamp',
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const s = state(this);
    const evidence = await canonicalElectionEvidence(s.winnerRail, s.eprId);
    assert.ok(
      evidence?.link_record,
      `${s.winner.alias} served no canonical-election evidence for "${s.eprId}" — there is no declaration link ` +
        `for the other peer's conductor to re-derive`
    );
    const verified = await verifyCarriedElection(s.laggardRail, s.eprId, evidence.link_record);
    assert.ok(
      verified,
      `${s.laggard.alias}'s conductor verified NOTHING from ${s.winner.alias}'s carried evidence`
    );
    assert.strictEqual(
      String(verified.winner_target),
      s.winnerHead,
      `${s.laggard.alias}'s merged election chose ${String(verified.winner_target)}, expected the earned head ${s.winnerHead}`
    );
    assert.strictEqual(
      verified.canonical_earned,
      true,
      `${s.laggard.alias}'s merged election named a winner whose tier is NOT earned ` +
        `(canonical_earned=${JSON.stringify(verified.canonical_earned)}) — earned-beats-staging did not decide this`
    );
    assert.ok(
      verified.canonical_declared_at !== undefined && verified.canonical_declared_at !== null,
      `${s.laggard.alias}'s merged election carries no canonical_declared_at — the timestamp the tiebreak reads is absent, ` +
        `so a tie between two earned declarations could not be broken deterministically`
    );
  }
);

/**
 * ANTI-REGRESSION. One flipped byte in the signed link record must be refused
 * IN WASM, and the laggard's served head must be exactly where the honest
 * election left it. Both halves matter: a refusal that still moved the row
 * would mean the row moved on the peer's say-so, not on proof.
 *
 * The tamper is byte-for-byte the 2026-08-31 proof's (`tamperLinkRecord`), so
 * this exercises the refusal path that was actually measured.
 *
 * Example:
 *   And a carried declaration link whose signature or binding fails wasm verification moves nothing
 */
Then(
  'a carried declaration link whose signature or binding fails wasm verification moves nothing',
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const s = state(this);
    const evidence = await canonicalElectionEvidence(s.winnerRail, s.eprId);
    assert.ok(
      evidence?.link_record,
      `${s.winner.alias} served no canonical-election evidence for "${s.eprId}" — nothing to tamper with`
    );
    const headBefore = await servedHead(s.laggard.storageUrl, s.eprId);

    let refused = false;
    let refusal = '';
    try {
      await verifyCarriedElection(s.laggardRail, s.eprId, tamperLinkRecord(evidence.link_record));
    } catch (error) {
      refused = true;
      refusal = String(error).slice(0, 200);
    }
    assert.ok(
      refused,
      `SECURITY: ${s.laggard.alias}'s conductor ACCEPTED a link record with one byte flipped — a peer could move ` +
        `this row with bytes that do not prove what they claim`
    );

    const headAfter = await servedHead(s.laggard.storageUrl, s.eprId);
    assert.strictEqual(
      headAfter,
      headBefore,
      `${s.laggard.alias}'s served head moved from ${headBefore} to ${headAfter} across a REFUSED verification ` +
        `(refusal was: ${refusal}) — the row must not move on evidence the conductor rejected`
    );
  }
);
