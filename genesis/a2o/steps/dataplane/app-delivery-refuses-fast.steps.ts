/**
 * App delivery refuses fast — step definitions for
 * features/dataplane/app-delivery-refuses-fast.feature (Act I, the household mesh this run owns).
 *
 * What is NEW here is only what the deliverability story never needed:
 *   · asking the fleet write-readiness probe (scripts/ci/fleet-write-readiness.sh) and
 *     judging its answer and how long it took — once for "not-ready", and every 5 seconds up
 *     to the declared bound for "ready", recording how long the window took to close;
 *   · shedding one doorway on purpose, and clearing it;
 *   · restarting every conductor while the probe is sampled throughout the window;
 *   · telling the deploy its budgets and reading its timing lines back.
 *
 * Publishing the run-owned site reuses the deliverability story's own steps ("a coherent EPR
 * app bundle this run just built", "an EPR record this run owns for it", "each doorway is
 * handed the bundle's bytes", "only doorway … is told …", "within … serves a page naming …")
 * and its fixtures (buildFixtureBundle, stageBundle, pollUntil, meshControl), so the deploy
 * this story times is the one CI runs, never a re-implementation of it.
 *
 * Plan: genesis/docs/superpowers/plans/2026-09-24-native-delivery-sprint-plan.md (Lane C).
 */

import { strict as assert } from 'node:assert';
import { mkdirSync, mkdtempSync, rmSync } from 'node:fs';
import { join } from 'node:path';

import { Given, When, Then } from '@cucumber/cucumber';

import { getRaw, probeHealth } from '../../src/framework/dataplane/surfaces.js';
import { E2EWorld } from '../../src/framework/world.js';

import {
  askFleetWriteReadiness,
  CONDUCTOR_RESTART_FACES,
  FLEET_WRITE_READINESS,
  IN_FLIGHT_SLACK_SECS,
  lastFace,
  namesDoorway,
  parseTimingLines,
  PLAN_FACES,
  pollReadiness,
  probeShedding,
  putShed,
  READINESS_EXIT,
  READINESS_FACES,
  READINESS_FACES_FILE,
  readFirstNotReady,
  readinessProbePresent,
  timingViolations,
  trimTrailingSlashes,
  withEnv,
  type ReadinessAnswer,
  type ToldBudgets,
} from './app-delivery-refuses-fast.helpers.js';
import {
  buildFixtureBundle,
  meshControl,
  pollUntil,
  removeFixtureBundle,
  stageBundle,
  type FixtureBundle,
  type StageOutcome,
} from './epr-app-deliverability.helpers.js';
import {
  chaosPublishedAuthority,
  type ChaosPublishedAuthority,
} from './epr-app-deliverability.steps.js';

// ---------------------------------------------------------------------------
// Scenario-local state
// ---------------------------------------------------------------------------

/** The two household doorways every probe is asked about (registered by the Background). */
const DOORWAYS = ['alpha-A', 'elohim.host'];

/** Transport budget the story tells the deploy — declared, so its timing lines can be judged. */
const TOLD_TRANSPORT_SECS = 120;
/** Re-offer cadence inside a readiness window; short, so a 40-second shed is measurable. */
const TOLD_POLL_SECS = 5;
/** Offer steps' cucumber budget. The told hard bound (+30s) must fit inside it. */
const OFFER_STEP_TIMEOUT_MS = 300_000;
/** The feature's per-answer bound for the probe; the sampler holds every answer to it. */
const PROBE_ANSWER_BOUND_MS = 30_000;
/** "answers ready within N seconds" re-asks the probe this often until it answers ready. */
const READY_POLL_INTERVAL_MS = 5_000;
/** The largest N the probe step accepts; its budget is N plus one last answer plus slack. */
const PROBE_STEP_MAX_SECS = 150;
const PROBE_STEP_TIMEOUT_MS = PROBE_STEP_MAX_SECS * 1000 + PROBE_ANSWER_BOUND_MS + 20_000;
/** A sampler that nobody stopped (a crashed scenario) stops itself after this long. */
const SAMPLER_MAX_LIFETIME_MS = 20 * 60_000;
/** The plan's restart step budget: the mesh verb waits for its conductors, not the cells. */
const RESTART_STEP_TIMEOUT_MS = 300_000;

interface Shed {
  url: string;
  secs: number;
  /** Our clock at the moment the doorway confirmed it was shedding. */
  confirmedAt: number;
  until: number;
  cleared: boolean;
}

interface Sampler {
  stop: boolean;
  answers: ReadinessAnswer[];
  done: Promise<void>;
  restartStartedAt: number;
  restartEndedAt?: number;
}

interface Told extends ToldBudgets {
  stateDir: string;
  hardTimeoutSecs: number;
}

interface Offer {
  doorway: string;
  outcome: StageOutcome;
  startedAt: number;
  endedAt: number;
}

interface RefusesFastState {
  lastAnswer?: ReadinessAnswer;
  sheds: Map<string, Shed>;
  sampler?: Sampler;
  told?: Told;
  previous?: ChaosPublishedAuthority;
  next?: { bundle: FixtureBundle; blobHash: string; bytesOutput: string[] };
  offers: Offer[];
}

const states = new WeakMap<E2EWorld, RefusesFastState>();

function state(world: E2EWorld): RefusesFastState {
  let current = states.get(world);
  if (!current) {
    current = { sheds: new Map(), offers: [] };
    states.set(world, current);
  }
  return current;
}

function doorwayUrl(world: E2EWorld, name: string): string {
  return trimTrailingSlashes(world.getDoorway(name).url);
}

function allDoorwayUrls(world: E2EWorld): string[] {
  return DOORWAYS.map(name => doorwayUrl(world, name));
}

function describeAnswer(answer: ReadinessAnswer): string {
  const status = answer.timedOut ? 'no exit (still running)' : `exit ${answer.code}`;
  return `${status} after ${answer.elapsedMs}ms:\n${answer.output.trim() || '(no output)'}`;
}

/** Degrade to PENDING — never a false green — while Lane A's probe is not on disk. */
function pendingProbe(world: E2EWorld): 'pending' {
  world.attach(
    `AUTHORED-UNRUN: ${FLEET_WRITE_READINESS} is not on disk in this checkout. The fleet ` +
      'write-readiness probe is Lane A of the 2026-09-24 native-delivery sprint; this ' +
      'scenario is written against its contract (exit 0 ready, 3 FLEET-NOT-READY <origin> ' +
      'face=<face> retryAfter=<s>, 2 usage; never sleeps) and runs once it lands.'
  );
  return 'pending';
}

function requireTold(world: E2EWorld): Told {
  const told = state(world).told;
  assert.ok(
    told,
    'no budget was told to the deploy — the "this run\'s deploy is told …" step must run first'
  );
  return told;
}

function requireOffer(world: E2EWorld): Offer {
  const offers = state(world).offers;
  assert.ok(offers.length > 0, 'this run has not offered the next version yet');
  const last = offers.at(-1);
  assert.ok(last, 'this run has not offered the next version yet');
  return last;
}

function requireNext(world: E2EWorld): NonNullable<RefusesFastState['next']> {
  const next = state(world).next;
  assert.ok(next, 'this run has not built a next version yet');
  return next;
}

function toldEnv(told: Told): Record<string, string> {
  return {
    STAGE_CELL_READY_BUDGET_SECS: String(told.readinessSecs),
    STAGE_CELL_READY_POLL_SECS: String(TOLD_POLL_SECS),
    STAGE_CELL_READY_STATE_DIR: told.stateDir,
    STAGE_BLOB_BUDGET_SECS: String(told.transportSecs),
    STAGE_HARD_TIMEOUT_SECS: String(told.hardTimeoutSecs),
  };
}

/**
 * A face a step names must be one the probe can print, or the step asserts something no answer
 * could ever satisfy. The vocabulary is scripts/ci/lib/readiness-faces.json, shared with the probe.
 */
function requireFace(face: string, among: readonly string[] = READINESS_FACES): void {
  assert.ok(
    among.includes(face),
    `"${face}" is not a face this step can meet — ${READINESS_FACES_FILE} lists: ${among.join(', ')}`
  );
}

/** The `blobHash` a doorway answers with for this app's record, or '' when unreadable. */
async function declaredBlobHash(url: string, slug: string): Promise<string> {
  const { status, text } = await getRaw(`${url}/db/content/${encodeURIComponent(slug)}`, {
    timeoutMs: 10_000,
  });
  if (status !== 200) return '';
  try {
    const parsed = JSON.parse(text) as { blobHash?: unknown };
    return typeof parsed.blobHash === 'string' ? parsed.blobHash : '';
  } catch {
    return '';
  }
}

// ---------------------------------------------------------------------------
// The fleet write-readiness probe
// ---------------------------------------------------------------------------

Given('the fleet write-readiness probe is part of this checkout', function (this: E2EWorld) {
  if (!readinessProbePresent()) return pendingProbe(this);
  return undefined;
});

Then(
  'the fleet write-readiness probe answers {word} within {int} seconds',
  { timeout: PROBE_STEP_TIMEOUT_MS },
  async function (this: E2EWorld, expected: string, bound: number) {
    assert.ok(
      expected === 'ready' || expected === 'not-ready',
      `the probe answers "ready" or "not-ready", not "${expected}"`
    );
    assert.ok(
      bound <= PROBE_STEP_MAX_SECS,
      `a ${bound}s probe bound does not fit this step's budget`
    );
    if (!readinessProbePresent()) return pendingProbe(this);
    if (expected === 'ready') return awaitReady(this, bound);
    // not-ready keeps its one-shot meaning: the probe is asked once and must name the window
    // at once. A not-ready answer claims the window is open now, not that it opens later.
    const answer = await askFleetWriteReadiness(allDoorwayUrls(this), bound * 1000);
    state(this).lastAnswer = answer;
    assert.ok(
      !answer.timedOut,
      `the fleet write-readiness probe was still running after ${bound}s and was stopped. A ` +
        `readiness probe must answer, never wait: ${describeAnswer(answer)}`
    );
    // assert.fail, not assert.equal: cucumber's error formatter replaces an equal()'s message
    // with an actual/expected diff (and cuts any message at "…: expected"), so a miss printed
    // "-3 +0" and never the FLEET-NOT-READY line that names the doorway and its face.
    if (answer.code !== READINESS_EXIT.notReady) {
      assert.fail(
        `the probe was asked for not-ready (exit ${READINESS_EXIT.notReady}) and answered ` +
          `otherwise. Full answer, ${describeAnswer(answer)}`
      );
    }
    assert.ok(
      answer.notReady.length > 0,
      `the probe answered not-ready but named no doorway — its contract is one ` +
        `FLEET-NOT-READY line per doorway that cannot take a write: ${describeAnswer(answer)}`
    );
    return undefined;
  }
);

/**
 * "answers ready within N seconds": re-ask the probe every READY_POLL_INTERVAL_MS until it
 * answers ready or N seconds have passed (controller ruling 2026-09-25 — a step that says
 * "within" and asks once is the defect). How long the window took to close, and the face it
 * wore last, go into the scenario's report either way, so the window's length is data. The
 * step fails only at the deadline, quoting the last FLEET-NOT-READY line it saw.
 */
async function awaitReady(world: E2EWorld, bound: number): Promise<undefined> {
  const urls = allDoorwayUrls(world);
  const poll = await pollReadiness(
    async () => askFleetWriteReadiness(urls, PROBE_ANSWER_BOUND_MS),
    bound * 1000,
    READY_POLL_INTERVAL_MS
  );
  state(world).lastAnswer = poll.last;
  const notReadyFaces = poll.answers
    .filter(answer => answer.code !== READINESS_EXIT.ready)
    .map(answer => lastFace(answer));
  const lastNotReadyFace = notReadyFaces.at(-1) ?? null;
  world.attach(
    JSON.stringify({
      measure: 'fleet-write-readiness-time-to-ready',
      boundSecs: bound,
      intervalMs: READY_POLL_INTERVAL_MS,
      ready: poll.ready,
      timeToReadyMs: poll.timeToReadyMs,
      elapsedMs: poll.elapsedMs,
      asks: poll.answers.length,
      lastFace: lastFace(poll.last),
      lastNotReadyFace,
      answers: poll.answers.map(answer => ({
        at: new Date(answer.startedAt).toISOString(),
        code: answer.code,
        ms: answer.elapsedMs,
        notReady: answer.notReady.map(line => line.raw),
      })),
    }),
    'application/json'
  );
  const asks = `${poll.answers.length} ask(s) every ${READY_POLL_INTERVAL_MS / 1000}s`;
  world.log(
    poll.ready
      ? `fleet write-readiness: ready after ${poll.timeToReadyMs}ms (${asks}, bound ${bound}s, ` +
          `last not-ready face ${lastNotReadyFace ?? 'none'})`
      : `fleet write-readiness: not ready after ${poll.elapsedMs}ms (${asks}, bound ${bound}s, ` +
          `last face ${lastFace(poll.last)})`
  );
  if (!poll.ready) {
    const lastLine =
      poll.answers.flatMap(answer => answer.notReady.map(line => line.raw)).at(-1) ??
      '(no FLEET-NOT-READY line in any answer)';
    assert.fail(
      `the probe did not answer ready within ${bound}s (${asks}).\n` +
        `Last FLEET-NOT-READY line:\n${lastLine}\nLast answer, ${describeAnswer(poll.last)}`
    );
  }
  return undefined;
}

Then(
  'the probe names doorway {string} as not ready, with the face {string} and a retry-after no longer than the shed',
  function (this: E2EWorld, doorway: string, face: string) {
    requireFace(face);
    const current = state(this);
    const answer = current.lastAnswer;
    assert.ok(answer, 'the probe has not been asked in this scenario');
    const url = doorwayUrl(this, doorway);
    const line = answer.notReady.find(candidate => namesDoorway(candidate.host, url));
    assert.ok(
      line,
      `the probe's not-ready answer does not name doorway "${doorway}" (${url}). A host is ` +
        'matched by URL, origin or host:port — both household doorways share one machine, ' +
        `so a bare hostname names neither: ${describeAnswer(answer)}`
    );
    assert.equal(
      line.face,
      face,
      `doorway "${doorway}" was named with face "${line.face}": ${line.raw}`
    );
    const shed = current.sheds.get(doorway);
    assert.ok(shed, `this scenario never made doorway "${doorway}" shed`);
    assert.ok(
      line.retryAfter !== null && line.retryAfter > 0 && line.retryAfter <= shed.secs,
      `the probe carried retryAfter=${line.retryAfter} for doorway "${doorway}", which shed ` +
        `for ${shed.secs}s — a not-ready answer must say when to come back, and no later ` +
        `than the doorway itself said: ${line.raw}`
    );
  }
);

Then('the probe does not name doorway {string}', function (this: E2EWorld, doorway: string) {
  const answer = state(this).lastAnswer;
  assert.ok(answer, 'the probe has not been asked in this scenario');
  const url = doorwayUrl(this, doorway);
  const line = answer.notReady.find(candidate => namesDoorway(candidate.host, url));
  assert.ok(
    !line,
    `the probe named doorway "${doorway}" as not ready although nothing was done to it: ${line?.raw}`
  );
});

// ---------------------------------------------------------------------------
// Shedding on purpose
// ---------------------------------------------------------------------------

When(
  'the household makes doorway {string} shed every write for {int} seconds',
  { timeout: 60_000 },
  async function (this: E2EWorld, doorway: string, secs: number) {
    const current = state(this);
    const url = doorwayUrl(this, doorway);
    const declared = await putShed(url, secs);
    if (declared.status === 404 || declared.status === 405) {
      this.attach(
        `PENDING: PUT ${url}/admin/dev/shed answered HTTP ${declared.status} — this doorway build ` +
          'has no household shed control (doorway-service routes/admin_dev.rs), so it cannot be ' +
          'made to shed on purpose.'
      );
      return 'pending';
    }
    assert.notEqual(
      declared.status,
      403,
      `PUT ${url}/admin/dev/shed refused HTTP 403: ${declared.text.slice(0, 300)} — the control ` +
        'opens only on a doorway that declared ELOHIM_NETWORK_STAKES=simulacra at boot, to a ' +
        "loopback caller. Check hc-mesh.sh's stakes wiring for this doorway."
    );
    assert.equal(
      declared.status,
      200,
      `PUT ${url}/admin/dev/shed {"retryAfterSecs":${secs}} failed: HTTP ${declared.status} ${declared.text.slice(0, 300)}`
    );
    if (!current.sheds.has(doorway)) {
      this.onCleanup(
        async () => {
          const shed = current.sheds.get(doorway);
          if (!shed || shed.cleared || shed.until <= Date.now()) return;
          const cleared = await putShed(url, 0);
          assert.equal(
            cleared.status,
            200,
            `cleanup could not clear the shed on ${url}: ${cleared.text}`
          );
        },
        { required: true }
      );
    }
    // Never proceed on an unverified premise: the doorway must actually be turning work away.
    const probe = await probeShedding(url);
    assert.equal(
      probe.status,
      503,
      `doorway "${doorway}" answered HTTP ${probe.status} right after its shed was set — expected 503`
    );
    assert.ok(probe.retryAfter, `doorway "${doorway}"'s shed answer carries no Retry-After header`);
    const confirmedAt = Date.now();
    current.sheds.set(doorway, {
      url,
      secs,
      confirmedAt,
      until: confirmedAt + secs * 1000,
      cleared: false,
    });
    return undefined;
  }
);

When(
  'the household lets doorway {string} serve again',
  { timeout: 60_000 },
  async function (this: E2EWorld, doorway: string) {
    const shed = state(this).sheds.get(doorway);
    assert.ok(shed, `this scenario never made doorway "${doorway}" shed`);
    const cleared = await putShed(shed.url, 0);
    assert.equal(
      cleared.status,
      200,
      `clearing the shed on "${doorway}" failed: HTTP ${cleared.status} ${cleared.text}`
    );
    shed.cleared = true;
    const probe = await probeShedding(shed.url);
    assert.ok(
      !(probe.status === 503 && probe.retryAfter),
      `doorway "${doorway}" is still shedding after its shed was cleared: HTTP ${probe.status} ${probe.text.slice(0, 200)}`
    );
  }
);

// ---------------------------------------------------------------------------
// A real window: every conductor restarts while the probe is sampled
// ---------------------------------------------------------------------------

When(
  'the household restarts every conductor while the probe keeps asking every {int} seconds',
  { timeout: RESTART_STEP_TIMEOUT_MS },
  async function (this: E2EWorld, intervalSecs: number) {
    if (!readinessProbePresent()) return pendingProbe(this);
    const current = state(this);
    const urls = allDoorwayUrls(this);
    const sampler: Sampler = {
      stop: false,
      answers: [],
      done: Promise.resolve(),
      restartStartedAt: Date.now(),
    };
    sampler.done = (async () => {
      const bornAt = Date.now();
      while (!sampler.stop && Date.now() - bornAt < SAMPLER_MAX_LIFETIME_MS) {
        sampler.answers.push(await askFleetWriteReadiness(urls, PROBE_ANSWER_BOUND_MS));
        if (sampler.stop) break;
        await new Promise(done => setTimeout(done, intervalSecs * 1000));
      }
    })();
    current.sampler = sampler;
    this.onCleanup(async () => {
      sampler.stop = true;
      await sampler.done;
    });
    await meshControl('conductors-restart');
    sampler.restartEndedAt = Date.now();
    return undefined;
  }
);

Then(
  'within {int} seconds both doorways report they have caught up',
  { timeout: 700_000 },
  async function (this: E2EWorld, bound: number) {
    const sampler = state(this).sampler;
    assert.ok(sampler, 'no conductor restart was started in this scenario');
    assert.ok(bound <= 660, `a ${bound}s bound does not fit this step's budget`);
    const last: Record<string, string> = {};
    const elapsed = await pollUntil(
      async () => {
        const verdicts = await Promise.all(
          allDoorwayUrls(this).map(async url => {
            const { status, body } = await probeHealth(url);
            // `stale` is always present on the wire (doorway health.rs P2PHealth) but not on
            // the shared HealthP2P type, so it is read through a narrow local view.
            const p2p = body?.p2p as { caughtUp?: boolean; stale?: boolean } | undefined;
            last[url] = `HTTP ${status} p2p=${JSON.stringify(p2p ?? null)}`;
            return status === 200 && p2p?.caughtUp === true && p2p.stale !== true;
          })
        );
        return verdicts.every(Boolean);
      },
      bound * 1000 - (Date.now() - sampler.restartStartedAt),
      5_000
    );
    assert.ok(
      elapsed !== null,
      `the household was not caught up ${bound}s after its conductors began restarting. A ` +
        'household still behind after that is stuck, and this run has measured it. Last ' +
        `health: ${JSON.stringify(last)}`
    );
  }
);

async function stopSampler(world: E2EWorld): Promise<ReadinessAnswer[]> {
  const sampler = state(world).sampler;
  assert.ok(sampler, 'no conductor restart was started in this scenario');
  sampler.stop = true;
  await sampler.done;
  return sampler.answers;
}

Then(
  'while the window was open the probe answered not-ready at least once, naming the face {string}, {string} or {string}',
  { timeout: 60_000 },
  async function (this: E2EWorld, first: string, second: string, third: string) {
    // A conductor restart wears only these (controller ruling 2026-09-25): each name the
    // feature gives must be one of them, and each is in the vocabulary file. Cucumber checks
    // the arity of this function, so the three names are declared, not rest parameters.
    const named = [first, second, third];
    for (const face of named) requireFace(face, CONDUCTOR_RESTART_FACES);
    const answers = await stopSampler(this);
    this.attach(
      JSON.stringify(
        answers.map(a => ({
          at: new Date(a.startedAt).toISOString(),
          code: a.code,
          ms: a.elapsedMs,
          notReady: a.notReady.map(line => line.raw),
        }))
      ),
      'application/json'
    );
    const notReady = answers.filter(a => a.code === READINESS_EXIT.notReady);
    assert.ok(
      notReady.length > 0,
      `across ${answers.length} answers during the restart the probe never answered not-ready. ` +
        'Either the window closed before the first answer, or the probe cannot see it — in both ' +
        'cases this run has not proved that the window is reported.'
    );
    const faces = notReady.flatMap(a => a.notReady.map(line => line.face));
    const unexpected = faces.filter(face => !named.includes(face));
    const quoted = named.map(face => JSON.stringify(face)).join(', ');
    assert.ok(faces.length > 0, 'a not-ready answer during the restart named no doorway');
    assert.deepEqual(
      unexpected,
      [],
      `during a conductor restart the probe named faces other than ${quoted}: ` +
        `${[...new Set(unexpected)].join(', ')} (known faces: ${READINESS_FACES.join(', ')})`
    );
  }
);

Then(
  'every answer the probe gave was ready or not-ready, and none took longer than {int} seconds',
  { timeout: 60_000 },
  async function (this: E2EWorld, bound: number) {
    const answers = await stopSampler(this);
    assert.ok(answers.length > 0, 'the probe was never asked during the restart');
    const bad = answers.filter(
      a =>
        a.timedOut ||
        (a.code !== READINESS_EXIT.ready && a.code !== READINESS_EXIT.notReady) ||
        a.elapsedMs > bound * 1000
    );
    assert.deepEqual(
      bad.map(describeAnswer),
      [],
      `${bad.length} of ${answers.length} probe answers during the window were errors, hung, or ` +
        `took longer than ${bound}s — a window must be reported, never turned into a failure or a wait`
    );
  }
);

// ---------------------------------------------------------------------------
// The deploy, told its budgets, meeting a shed
// ---------------------------------------------------------------------------

Given(
  "this run's deploy is told it may wait at most {int} seconds for a doorway that is not ready",
  function (this: E2EWorld, readinessSecs: number) {
    const root = process.env['A2O_FIXTURE_DIR'] ?? join(process.cwd(), 'reports', 'fixtures');
    mkdirSync(root, { recursive: true });
    // One fresh run-deadline record per scenario: stage-spa-blob.sh shares ONE deadline
    // across every leg of a run, and this scenario is the run.
    const stateDir = mkdtempSync(join(root, 'stage-cell-ready-'));
    this.onCleanup(async () => {
      rmSync(stateDir, { recursive: true, force: true });
      return Promise.resolve();
    });
    const hardTimeoutSecs = readinessSecs + TOLD_TRANSPORT_SECS + 60;
    assert.ok(
      hardTimeoutSecs * 1000 + 30_000 <= OFFER_STEP_TIMEOUT_MS,
      `a ${readinessSecs}s readiness budget makes the deploy's own completion bound ` +
        `(${hardTimeoutSecs}s) outlive the step that runs it`
    );
    state(this).told = {
      readinessSecs,
      transportSecs: TOLD_TRANSPORT_SECS,
      inFlightSlackSecs: IN_FLIGHT_SLACK_SECS,
      stateDir,
      hardTimeoutSecs,
    };
  }
);

When(
  'this run builds a next version of its site and hands its bytes to each doorway',
  { timeout: OFFER_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    const current = state(this);
    const told = requireTold(this);
    const previous = chaosPublishedAuthority(this);
    current.previous = previous;
    const bundle = buildFixtureBundle({
      coherent: true,
      baseHref: `${previous.mountPath.replace(/\/$/, '')}/`,
    });
    this.onCleanup(async () => {
      removeFixtureBundle(bundle);
      return Promise.resolve();
    });
    const bytesOutput: string[] = [];
    let blobHash = '';
    for (const name of DOORWAYS) {
      const outcome = await withEnv(toldEnv(told), async () =>
        stageBundle({
          bundle,
          slug: previous.slug,
          doorwayUrl: doorwayUrl(this, name),
          declare: false,
        })
      );
      bytesOutput.push(outcome.output);
      assert.equal(
        outcome.code,
        0,
        `handing the next version's bytes to ${name} failed (exit ${outcome.code}):\n${outcome.output}`
      );
      blobHash = outcome.blobHash;
    }
    assert.ok(blobHash, 'stage-spa-blob.sh reported no blob hash for the next version');
    assert.notEqual(
      blobHash,
      previous.blobHash,
      'the next version hashed the same as the previous one'
    );
    current.next = { bundle, blobHash, bytesOutput };
  }
);

When(
  'this run tells doorway {string}, while it sheds, that the next version is current',
  { timeout: OFFER_STEP_TIMEOUT_MS },
  async function (this: E2EWorld, doorway: string) {
    const current = state(this);
    const told = requireTold(this);
    const next = requireNext(this);
    const previous = current.previous;
    assert.ok(previous, 'this run has no previous version to replace');
    const shed = current.sheds.get(doorway);
    assert.ok(
      shed && !shed.cleared && shed.until > Date.now(),
      `doorway "${doorway}" is not shedding as the offer begins — the offer would not meet the window`
    );
    const startedAt = Date.now();
    const outcome = await withEnv(toldEnv(told), async () =>
      stageBundle({
        bundle: next.bundle,
        slug: previous.slug,
        doorwayUrl: doorwayUrl(this, doorway),
        declare: true,
      })
    );
    const endedAt = Date.now();
    current.offers.push({ doorway, outcome, startedAt, endedAt });
    this.attach(
      `stage-spa-blob.sh exit ${outcome.code} after ${Math.round((endedAt - startedAt) / 1000)}s:\n${outcome.output}`
    );
  }
);

Then(
  'the deploy met the window, waited it out, and went through without being sent again',
  function (this: E2EWorld) {
    const current = state(this);
    const told = requireTold(this);
    const offer = requireOffer(this);
    assert.equal(
      current.offers.length,
      1,
      `the next version was offered ${current.offers.length} times`
    );
    assert.equal(
      offer.outcome.code,
      0,
      `the deploy did not go through (exit ${offer.outcome.code}):\n${offer.outcome.output}`
    );
    assert.notEqual(
      readFirstNotReady(told.stateDir),
      null,
      `the deploy's run-deadline record in ${told.stateDir} holds no first not-ready answer — the ` +
        'offer never met the window, so this run proved nothing about waiting one out'
    );
    const lines = parseTimingLines(offer.outcome.output);
    assert.ok(
      lines.some(line => line.kind === 'readiness-wait' && line.face === 'catching-up'),
      `the deploy printed no wait on the catching-up face:\n${offer.outcome.output}`
    );
    assert.ok(
      lines.some(line => line.kind === 'readiness-cleared'),
      `the deploy never reported taking the write after its not-ready answers:\n${offer.outcome.output}`
    );
    const shed = current.sheds.get(offer.doorway);
    assert.ok(shed, `the offer went to "${offer.doorway}", which this scenario never made shed`);
    assert.ok(
      offer.endedAt >= shed.until - 2_000,
      `the deploy finished ${Math.round((shed.until - offer.endedAt) / 1000)}s before the shed ` +
        'ended — a doorway turning work away cannot have taken it'
    );
  }
);

Then(
  "doorway {string} answers with the next version as this app's declared head",
  { timeout: 60_000 },
  async function (this: E2EWorld, doorway: string) {
    const next = requireNext(this);
    const previous = state(this).previous;
    assert.ok(previous, 'this run has no published app');
    const url = doorwayUrl(this, doorway);
    let last = '';
    const elapsed = await pollUntil(async () => {
      last = await declaredBlobHash(url, previous.slug);
      return last === next.blobHash;
    }, 30_000);
    assert.ok(
      elapsed !== null,
      `doorway "${doorway}" answers ${last || 'nothing'} as the declared head, not the offered ${next.blobHash}`
    );
  }
);

Then(
  "within {int} seconds both doorways serve a page naming the next version's entry script",
  { timeout: 180_000 },
  async function (this: E2EWorld, bound: number) {
    assert.ok(bound <= 150, `a ${bound}s bound does not fit this step's budget`);
    const next = requireNext(this);
    const offer = requireOffer(this);
    const previous = state(this).previous;
    assert.ok(previous, 'this run has no published app');
    for (const name of DOORWAYS) {
      const pageUrl = `${doorwayUrl(this, name)}${previous.mountPath}`;
      let last = { status: 0, text: '' };
      const elapsed = await pollUntil(
        async () => {
          last = await getRaw(pageUrl, { timeoutMs: 10_000 });
          if (last.status !== 200 || !last.text.includes(next.bundle.entryScript)) return false;
          const assets = await Promise.all(
            [next.bundle.entryScript, next.bundle.styleSheet].map(async file =>
              getRaw(`${pageUrl}/${file}`, { timeoutMs: 5_000 })
            )
          );
          return assets.every(asset => asset.status === 200);
        },
        bound * 1000 - (Date.now() - offer.endedAt)
      );
      assert.ok(
        elapsed !== null,
        `${name} did not serve the next version within ${bound}s of the deploy going through: ` +
          `GET ${pageUrl} answered ${last.status} without "${next.bundle.entryScript}" and its assets`
      );
    }
  }
);

Then('nothing in the deploy waited longer than it was told', function (this: E2EWorld) {
  const told = requireTold(this);
  const offer = requireOffer(this);
  const next = requireNext(this);
  const output = [...next.bytesOutput, offer.outcome.output].join('\n');
  const lines = parseTimingLines(output);
  assert.ok(
    lines.length > 0,
    'the deploy printed no timing line to read back — either it never waited, or the shape of ' +
      'its lines changed and this story can no longer judge them'
  );
  const violations = timingViolations(lines, told);
  assert.deepEqual(
    violations,
    [],
    `the deploy waited longer than it was told:\n${violations.join('\n')}`
  );
  const wallSecs = (offer.endedAt - offer.startedAt) / 1000;
  assert.ok(
    wallSecs <= told.hardTimeoutSecs,
    `the offer ran ${Math.round(wallSecs)}s, past its own completion bound of ${told.hardTimeoutSecs}s`
  );
});

Then(
  'the deploy stopped at its deadline, saying the doorway was still not ready with the face {string}',
  function (this: E2EWorld, face: string) {
    // The deploy (stage-spa-blob.sh `not_ready_face`) waits on the plan faces only.
    requireFace(face, PLAN_FACES);
    const offer = requireOffer(this);
    assert.notEqual(
      offer.outcome.code,
      0,
      `the deploy reported success although the doorway shed for longer than its budget:\n${offer.outcome.output}`
    );
    const exhausted = parseTimingLines(offer.outcome.output).find(
      line => line.kind === 'readiness-exhausted'
    );
    assert.ok(
      exhausted,
      `the deploy failed without saying its readiness deadline was reached:\n${offer.outcome.output}`
    );
    assert.equal(
      exhausted.face,
      face,
      `the deploy stopped naming face "${exhausted.face}": ${exhausted.raw}`
    );
  }
);

Then(
  'the deploy stopped no later than {int} seconds after its deadline',
  function (this: E2EWorld, slackSecs: number) {
    const told = requireTold(this);
    const offer = requireOffer(this);
    const first = readFirstNotReady(told.stateDir);
    assert.ok(first !== null, `the deploy recorded no first not-ready answer in ${told.stateDir}`);
    const deadline = first + told.readinessSecs;
    const stoppedAt = offer.endedAt / 1000;
    assert.ok(
      stoppedAt >= deadline - 1,
      `the deploy stopped ${Math.round(deadline - stoppedAt)}s before its deadline — it gave up early rather than at the budget`
    );
    assert.ok(
      stoppedAt - deadline <= slackSecs,
      `the deploy stopped ${Math.round(stoppedAt - deadline)}s after its ${told.readinessSecs}s ` +
        `deadline, more than the ${slackSecs}s one in-flight attempt is allowed`
    );
  }
);

Then(
  "doorway {string} still answers with the previous version as this app's declared head",
  { timeout: 60_000 },
  async function (this: E2EWorld, doorway: string) {
    const next = requireNext(this);
    const previous = state(this).previous;
    assert.ok(previous, 'this run has no previous version');
    const url = doorwayUrl(this, doorway);
    let last = '';
    const elapsed = await pollUntil(async () => {
      last = await declaredBlobHash(url, previous.slug);
      return last !== '';
    }, 30_000);
    assert.ok(
      elapsed !== null,
      `doorway "${doorway}" answered no declared head for ${previous.slug}`
    );
    assert.notEqual(
      last,
      next.blobHash,
      `the refused next version became the declared head on "${doorway}"`
    );
    assert.equal(
      last,
      previous.blobHash,
      `doorway "${doorway}" answers ${last}, not the previous ${previous.blobHash}`
    );
  }
);
