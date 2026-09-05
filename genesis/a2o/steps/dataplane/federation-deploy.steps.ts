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
 * A substrate this run OWNS only (`@requires:owned-substrate` — available:false
 * on the shared fleet, available:true only under
 * `genesis/manifests/cluster-state.act1-household.yaml`). This fixture WRITES:
 * it authors a revision on each of two peers and flips an operator flag in a
 * peer's runtime config. `@requires:household-nodes` would not have gated that,
 * being available:true in both cluster-state files — "peers exist" is not "this
 * run may write to them", and the fleet's Dataplane Validation lane selects
 * `@dataplane and not @wip`. Run it with
 * `just test mesh genesis/a2o/features/dataplane/federation-deploy.feature`.
 */

/* eslint-disable sonarjs/publicly-writable-directories --
 * MESH_ROOT defaults to /tmp/elohim-local-mesh because that is where
 * `app/elohim-app/scripts/hc-mesh.sh` actually puts the household mesh's per-peer
 * directories; this fixture reads and restores a file the mesh itself owns there.
 * `steps/delivery/happ-lineage-migration.steps.ts` carries the same disable for the
 * same constant and the same reason. */

import { strict as assert } from 'node:assert';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import path from 'node:path';

import { Given, When, Then, AfterAll } from '@cucumber/cucumber';

import {
  authorDeclare,
  canonicalElectionEvidence,
  connectConductor,
  declareEarnedCanonicalHead,
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

/** How long the laggard gets to PROJECT the page the winner just authored, before staging its
 *  own divergent revision of it. A page that never arrives is a replication failure, and saying
 *  so is more useful than timing out later in the convergence wait. */
const PROJECTION_WAIT_MS = Number(process.env['E2E_CARRIED_ELECTION_PROJECTION_MS'] ?? '120000');

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
  /** Wall-clock ms at which the winner authored the head it later declared EARNED. */
  winnerAuthoredAt?: number;
  /** Wall-clock ms at which the laggard authored its own, LATER, competing head. */
  laggardAuthoredAt?: number;
  /** Every `elohim_content_election_*` series on the laggard after the wait — observation for
   *  the receipt, never an assertion: which arm carried a move is a property of the substrate's
   *  regime, not of this scenario. */
  electionSeriesAfter?: string[];
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
        // There was NO file before this run. Restoring means removing it, never
        // leaving an empty one: an empty runtime-config.toml is a different state
        // from no runtime-config.toml for anything that branches on file presence
        // (the watcher reports `filePresent`), and a fixture must hand the mesh
        // back exactly what it borrowed.
        rmSync(original.file, { force: true });
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

/**
 * Every `elohim_content_election_*` sample on a peer, verbatim. Recorded in the receipt so a
 * reader can see WHICH arm the substrate used, without this scenario having to assert one —
 * see the mechanism step's doc for why naming a specific arm would be an overclaim.
 */
async function electionSeries(peer: PeerRef): Promise<string[]> {
  const { status, text } = await getRaw(`${peer.storageUrl}/metrics`);
  if (status !== 200) return [];
  return text
    .split('\n')
    .map(line => line.trim())
    .filter(line => line.startsWith('elohim_content_election') && !line.startsWith('#'));
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
    /** The winner's head was authored FIRST; the laggard's competing head SECOND. A move to the
     *  winner's head is therefore a move BACKWARDS in wall-clock terms — only a tier-aware
     *  election explains it. */
    winnerAuthoredAt: string;
    laggardAuthoredAt: string;
    /** Which election arm the substrate actually used, verbatim from the laggard's /metrics. */
    electionSeries: string[];
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
  'peer {string} and peer {string} both declare a head for a page this run authored',
  { timeout: 180_000 },
  async function (this: E2EWorld, winnerAlias: string, laggardAlias: string) {
    resetStagingWrites();
    // A page THIS RUN AUTHORS, and the reason is a protocol guard, not convenience.
    //
    // MEASURED 2026-09-05 on the household mesh, against the real landing EPR: the
    // content_store zome refuses `declare_earned_canonical_head` with "restricted to the
    // root author, a device it delegated, or the bootstrap steward (progenitor) … is not
    // the author of content 'elohim-host-landing' … and carries no head delegation".
    // That guard is correct and the plan's constraints keep every stamp guard untouched, so
    // NO fixture can ever stage an EARNED declaration on content someone else authored.
    // The election under test is page-agnostic; the authority guard is not. So the vehicle
    // is a page whose root author is this run's own peer, exactly as the 2026-08-31 proof
    // staged it. The landing EPR is still asserted on below — read, never staged.
    const eprId = `federation-convergence-${Date.now()}`;
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

    // ONE ROOT, TWO DIVERGENT REVISIONS — and the shape is dictated by the substrate, not by
    // convenience. Measured 2026-09-05, twice:
    //
    //   · A page's id is UNIQUE in the DHT. The second peer to plant a root for one id is
    //     refused in wasm ("Content with id '…' already exists. Use update_content"), whether
    //     the two creates are sequential or concurrent — local gossip between household peers
    //     is faster than the race. The 2026-08-31 proof staged two roots and passed only by
    //     winning that race; it is inherently flaky, and this fixture must not inherit that.
    //   · An EARNED canonical declaration is restricted to the page's ROOT AUTHOR. So the peer
    //     that plants the root is necessarily the peer that can declare earned.
    //
    // Both facts point at the same staging: the WINNER plants the one root and authors the
    // revision it will declare EARNED; the page gossips; the LAGGARD then authors its OWN
    // revision of that same page, which its conductor-routed update declares as the laggard's
    // head at the STAGING tier. Two peers, one page, two different heads — the fleet's frozen
    // class, reached deterministically. It also sharpens the election under test: the laggard's
    // head is the LATER one in wall-clock terms, so only earned-beats-staging can elect the
    // winner's. A convergence here cannot be recency accidentally looking like an election.
    const stamp = new Date().toISOString();
    const winnerHead = await authorDeclare({
      storageUrl: winner.storageUrl,
      id: eprId,
      body: `# ${eprId}\n\nRoot revision authored on ${winner.alias} at ${stamp} — the earned head.`,
      agent: winnerRail.agent,
      ensureLocalRoot: true,
      title: `Federation convergence fixture (${stamp})`,
      description: 'a page this run authored, so it may declare an earned canonical head for it',
    });
    const winnerAuthoredAt = Date.now();

    // The laggard cannot revise a page it has not yet seen. Poll its OWN projection until the
    // row carries an anchor — never a fixed sleep, which would either flake or waste the run.
    const seenBy = Date.now() + PROJECTION_WAIT_MS;
    let laggardSees = false;
    while (Date.now() < seenBy) {
      const probe = await fetch(`${laggard.storageUrl}/db/content/${eprId}`);
      if (probe.ok) {
        const row = (await probe.json().catch(() => null)) as { dhtAnchorHash?: string } | null;
        if (row?.dhtAnchorHash) {
          laggardSees = true;
          break;
        }
      }
      await new Promise(resolve => setTimeout(resolve, 3_000));
    }
    assert.ok(
      laggardSees,
      `${laggard.alias} never projected "${eprId}" within ${PROJECTION_WAIT_MS / 1000}s of ` +
        `${winner.alias} authoring it — the page did not reach the second peer at all, so there is ` +
        `no shared page for the two of them to disagree about (a gossip/replication failure, not a ` +
        `convergence one)`
    );

    // ensureLocalRoot is deliberately FALSE here: the laggard must take the update arm on the
    // page it projected, never plant a competing root.
    const laggardHead = await authorDeclare({
      storageUrl: laggard.storageUrl,
      id: eprId,
      body: `# ${eprId}\n\nDivergent revision authored on ${laggard.alias} at ${new Date().toISOString()} — the stale head.`,
      agent: laggardRail.agent,
    });
    const laggardAuthoredAt = Date.now();

    states.set(this, {
      eprId,
      winner,
      laggard,
      winnerHead,
      laggardHead,
      winnerRail,
      laggardRail,
      winnerAuthoredAt,
      laggardAuthoredAt,
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
 * Only the page's ROOT AUTHOR may do this — the zome refuses anyone else in wasm — which is why
 * the staging plants the root on this peer. Its head is the OLDER of the two; the earned tier is
 * what elects it, not recency.
 *
 * Example:
 *   And an EARNED canonical declaration exists on the page's root author for the head it authored
 */
Given(
  "an EARNED canonical declaration exists on the page's root author for the head it authored",
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const s = state(this);
    const earned = await declareEarnedCanonicalHead(s.winnerRail, s.eprId, s.winnerHead);
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
 * Wait for the disagreeing peer to move its head ON ITS OWN.
 *
 * "Disagreeing", not "holding the older head": under this fixture's staging that peer holds the
 * NEWER head — it authored its competing revision second — and the head it must move TO is the
 * older, earned one. Naming it by age would contradict the very ordering the mechanism step
 * asserts.
 *
 * This step makes NO mutating call. It snapshots the staging write ledger and
 * the obey-path counter, then polls one read surface — `GET
 * /db/content/{id}/head` on the laggard — until the head matches the elected
 * one or the budget runs out. The verdict is recorded, not asserted: the Then
 * steps own the assertions, so a non-convergence is reported against the
 * outcome the story names rather than as an opaque timeout here.
 *
 * Example:
 *   When the reconcile sweep runs on the disagreeing peer
 */
When(
  'the reconcile sweep runs on the disagreeing peer',
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
    s.electionSeriesAfter = await electionSeries(s.laggard);
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
        winnerAuthoredAt: new Date(s.winnerAuthoredAt ?? 0).toISOString(),
        laggardAuthoredAt: new Date(s.laggardAuthoredAt ?? 0).toISOString(),
        electionSeries: s.electionSeriesAfter ?? [],
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
  'both doorways serve the SAME head for that page',
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const s = state(this);
    const eprId = s.eprId;
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

/**
 * The SCOPE TRANSITION, made executable instead of narrated.
 *
 * Everything above this line concerns the page this run authored. The two steps below it read a
 * REAL, seeded page — a different page, named in the Gherkin — to check that each doorway serves
 * what it declares. Two blind readers independently flagged that the jump between the two was
 * invisible in the Gherkin and recoverable only from a comment.
 *
 * So the transition is now an assertion, and a load-bearing one: the staging ledger must contain
 * no mutating call whose target names this EPR. That is what entitles the next two steps to read
 * the live landing page at all — they are observing a page this run never touched, not inspecting
 * its own handiwork. It also permanently forecloses the worst version of this fixture, in which a
 * future edit quietly stages divergence on the page visitors are reading.
 *
 * Example:
 *   And EPR "elohim-host-landing" was never staged by this run
 */
Then('EPR {string} was never staged by this run', function (this: E2EWorld, eprId: string) {
  const touched = stagingWrites().filter(write => write.target.includes(eprId));
  assert.deepStrictEqual(
    touched,
    [],
    `this run made ${touched.length} mutating call(s) against "${eprId}", the live seeded page: ` +
      `${JSON.stringify(touched)}. The served-versus-declared checks that follow are only ` +
      `meaningful on a page the fixture did not author, and no fixture may stage divergence on a ` +
      `page visitors are reading.`
  );
});

// ---------------------------------------------------------------------------
// Then — the mechanism, so the outcome is not a trust-the-peer copy
// ---------------------------------------------------------------------------

/**
 * THE DISCRIMINATOR between "an election was obeyed" and "the newest write won".
 *
 * The staging is deliberately built so these two answers differ. The winner authored its head
 * FIRST; the laggard then authored its own competing head SECOND. So the head the laggard
 * ends up serving is OLDER, in wall-clock terms, than the one it gave up. Nothing that
 * resolves ties by recency — a last-writer-wins projection, a naive "adopt the freshest peer",
 * a trust-the-peer copy of whoever spoke most recently — can produce that outcome. Only a rule
 * that ranks the EARNED tier above the staging tier can, which is the rule under test.
 *
 * WHY THIS IS NOT A COUNTER ASSERTION, which is what it used to be and what the task brief
 * asked for. `elohim_content_election_obeyed_total{path="peer_carried"}` is the arm for a peer
 * whose OWN conductor cannot see the election and must be handed the declaration by another
 * peer. On a healthy household mesh that situation cannot be manufactured: gossip works, the
 * laggard's conductor resolves the election itself, and the obey arm exits early — MEASURED
 * 2026-09-05, `obey_probe_total{outcome="no_election"}` and `{outcome="resolve_error"}`
 * account for every probe while the row nevertheless converged through the contest path. The
 * series is not merely zero, it is structurally ABSENT, and asserting on it here would have
 * made this scenario permanently red for a reason that has nothing to do with the cure. The
 * peer-carried arm's evidence is a FLEET matter (the arc-Empty regime the mesh cannot fake) and
 * stays where the habit atom already records it. Which arm ran is recorded verbatim in the
 * receipt instead, as observation.
 *
 * What still proves the move is trustworthy rather than credulous lives in the two steps after
 * this one: the laggard's OWN conductor re-derives the winning declaration in wasm, and refuses
 * a link record with a single byte flipped.
 *
 * Example:
 *   And the head it moved to is OLDER than the head it gave up, so recency cannot explain the move
 */
Then(
  'the head it moved to is OLDER than the head it gave up, so recency cannot explain the move',
  { timeout: 60_000 },
  function (this: E2EWorld) {
    const s = state(this);
    assert.ok(
      s.winnerAuthoredAt !== undefined && s.laggardAuthoredAt !== undefined,
      'staging did not record when each head was authored — the recency discriminator cannot be evaluated'
    );
    assert.ok(
      s.laggardAuthoredAt > s.winnerAuthoredAt,
      `staging inverted: ${s.laggard.alias} authored its competing head at ` +
        `${new Date(s.laggardAuthoredAt).toISOString()}, NOT after ${s.winner.alias}'s ` +
        `${new Date(s.winnerAuthoredAt).toISOString()}. With the elected head also the newest, a ` +
        `recency rule and an election rule would give the same answer and this run would not ` +
        `distinguish them`
    );
    assert.strictEqual(
      s.laggardServedAfter,
      s.winnerHead,
      `${s.laggard.alias} does not serve the earned head, so there is no backwards move to explain`
    );
    // eslint-disable-next-line no-console
    console.log(
      `  election arm observed on ${s.laggard.alias}: ${(s.electionSeriesAfter ?? []).join(' | ') || 'no elohim_content_election_* series'}`
    );
  }
);

/**
 * Re-derive the election on the laggard's OWN conductor and read the answer back
 * out of wasm: the winner it names, that the winning declaration is the EARNED
 * one (`canonical_earned`), and that it carries the notarized
 * `canonical_declared_at`.
 *
 * WHAT THIS DOES NOT ASSERT, deliberately. The election has two rules:
 * earned-beats-staging, then newest notarized timestamp. Only the FIRST is
 * exercised here, because the staging shape is one earned declaration against
 * one staging-tier one — the timestamp rule is reached only when two EARNED
 * declarations compete. This step therefore checks that the tiebreak's INPUT
 * exists (the timestamp is present on the winner) and stops there. Staging a
 * real two-earned tie needs a second `declare_earned_canonical_head` on the
 * laggard and a deliberate timestamp ordering; that is its own scenario, and
 * claiming it from here would be an overclaim the Gherkin used to carry.
 *
 * This is a READ — `verify_carried_election` proves bytes, it does not author.
 * The staging ledger is asserted unchanged by the outcome step above, and this
 * step adds nothing to it.
 *
 * Example:
 *   And the elected head carries the EARNED canonical declaration, and that declaration carries a notarized timestamp
 */
Then(
  'the elected head carries the EARNED canonical declaration, and that declaration carries a notarized timestamp',
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
