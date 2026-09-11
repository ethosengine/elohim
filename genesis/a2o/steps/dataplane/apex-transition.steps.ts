/**
 * Step glue for the two apex-transition scenarios
 * (features/dataplane/doorway-apex-transition.feature, @concern:doorway-failover,
 * @wip @requires:owned-substrate). Doorway federation S1 Task 6
 * (genesis/docs/superpowers/plans/2026-09-10-doorway-federation-three-reds-to-green-plan.md).
 *
 * The feature's own preamble refuses a test-only proxy as certification: "The
 * household must own the public-name routing apparatus and its fault
 * controls before exercising these scenarios … A test-only proxy that does
 * not execute the actual routing configuration cannot certify the deployed
 * path." Today the household owns NO membership authority —
 * relay-addr-beacon's `reconcile_membership` (relay-addr-beacon/src/main.rs:315)
 * writes owner records through the Cloudflare sink only
 * (relay-addr-beacon/src/sinks/: cloudflare.rs, coturn.rs, pkarr.rs — there is
 * no household-ownable membership sink), and
 * app/elohim-app/scripts/hc-mesh.sh stages no membership apparatus at all.
 *
 * So every step whose truth depends on ONE public name spanning both owned
 * doorways (shared membership, owner records, the apex name surviving a
 * shed) fails HONESTLY with that named reason — never `'pending'`, never a
 * silent pass. What the household DOES own — two real doorway processes on
 * this host — is exercised for real: the apex doorway (fixture id "apex",
 * which shares hc-mesh.sh's doorway-B process — see hc-mesh.sh section 1b,
 * "Doorway B (apex/elohim.host stand-in)") is SIGSTOP'd and SIGCONT'd via
 * the same start-tick-guarded /proc technique as
 * doorway-sibling-reader.steps.ts:77's `signalOwnedDoorway`, and its sibling
 * (fixture id "alpha") is read directly for its serving state, declared head,
 * and build stamp. This is deliberately the SAME split the task calls for:
 * shed/serve/restore/read-sibling/build-stamp really run; "one public name"
 * assertions name the missing apparatus.
 *
 * The deliverable is MEASURED, not green: turning "2 undefined scenarios"
 * into "2 scenarios failed at a named step" is the whole win — a green is
 * not available at home. See doorway-failover.habit.md checks: "WAN ingress
 * continuity is a distinct prerequisite".
 */

import { strict as assert } from 'node:assert';
import { execFile, spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { once } from 'node:events';
import { readFile, readlink } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { After, Given, When, Then } from '@cucumber/cucumber';

import {
  classifyDoorwayState,
  getRaw,
  probeDeclaredHead,
  CLASSIFY_TIMEOUT_MS,
  CATCHUP_RIDE_STEP_TIMEOUT_MS,
} from '../../src/framework/dataplane/surfaces.js';
import {
  loadHouseholdMeshFixture,
  requireFixtureDoorwayUrl,
  type HouseholdMeshFixture,
} from '../../src/framework/fixtures/household-mesh.js';
import { E2EWorld } from '../../src/framework/world.js';

const run = promisify(execFile);
const meshScript = fileURLToPath(
  new URL('../../../../app/elohim-app/scripts/hc-mesh.sh', import.meta.url)
);

/** One bounded GET; sized like every other single-fetch step in this suite. */
const RAW_FETCH_TIMEOUT_MS = 30_000;
/**
 * The first visit makes one bounded GET, one declared-head probe (which can
 * RIDE the bounded catching-up shed — surfaces.ts) and one bounded GET for
 * the build stamp, sequentially.
 */
const FIRST_VISIT_TIMEOUT_MS = CATCHUP_RIDE_STEP_TIMEOUT_MS + 2 * RAW_FETCH_TIMEOUT_MS;
/**
 * Inducing the shed classifies the sibling (up to 3 sequential bounded
 * requests, per classifyDoorwayState) and then probes its declared head
 * (which can ride the shed), on top of the process-signal work itself.
 */
const INDUCE_SHED_TIMEOUT_MS = 3 * CLASSIFY_TIMEOUT_MS + CATCHUP_RIDE_STEP_TIMEOUT_MS + 15_000;

// ---------------------------------------------------------------------------
// No household-owned membership authority — the named RED this task ships.
// ---------------------------------------------------------------------------

/**
 * A future household stage could add a membership authority the fixture
 * declares (e.g. `doorways.apex.membershipAuthority` or similar); checking
 * for it — rather than throwing unconditionally — means this guard starts
 * passing the day that apparatus is staged, with no edit to this file.
 * Today no fixture ever sets it, so every call throws the same named reason.
 */
interface FixtureWithMembershipAuthority extends HouseholdMeshFixture {
  membershipAuthority?: unknown;
}

function requireHouseholdMembershipAuthority(): void {
  const fixture = loadHouseholdMeshFixture() as FixtureWithMembershipAuthority;
  if (fixture.membershipAuthority) return;
  throw new Error(
    "no household-owned membership authority: relay-addr-beacon's reconcile_membership " +
      '(relay-addr-beacon/src/main.rs:315) writes owner records through the Cloudflare sink ' +
      'only (relay-addr-beacon/src/sinks/: cloudflare.rs, coturn.rs, pkarr.rs — no ' +
      'household-ownable membership sink), and app/elohim-app/scripts/hc-mesh.sh stages no ' +
      'membership apparatus at all. This step is RED by missing apparatus, not by a defect in ' +
      'the doorways. See doorway-failover.habit.md checks: "WAN ingress continuity is a ' +
      'distinct prerequisite".'
  );
}

// ---------------------------------------------------------------------------
// Household-executable half: real SIGSTOP/SIGCONT against the apex doorway
// process, real reads of its sibling.
// ---------------------------------------------------------------------------

interface ApexTransitionState {
  apexUrl: string;
  siblingUrl: string;
  commonsId: string;
  observedDoorway?: string;
  declaredHeadAtVisit?: string;
  buildStampAtVisit?: string;
  pid?: number;
  ticks?: string;
  executable?: string;
  paused: boolean;
}

const states = new WeakMap<E2EWorld, ApexTransitionState>();
const leases = new WeakMap<E2EWorld, ChildProcessWithoutNullStreams>();

function getState(world: E2EWorld): ApexTransitionState {
  const state = states.get(world);
  assert.ok(
    state,
    'visit the declared landing page through the owned public name first ' +
      '("a new visitor reaches the declared landing page through one owned public name")'
  );
  return state;
}

/** The `commit` field of a version.json body, or '' when it carries none. */
function stampOf(text: string): string {
  try {
    const parsed = JSON.parse(text) as { commit?: unknown };
    return typeof parsed.commit === 'string' ? parsed.commit : '';
  } catch {
    return '';
  }
}

/**
 * Shared-mesh coordination, identical in shape to
 * doorway-sibling-reader.steps.ts's `acquireLease` — every SIGSTOP/SIGCONT
 * fault run must hold this lock so two fault-injecting scenarios never race
 * the same household mesh.
 */
async function acquireLease(world: E2EWorld): Promise<void> {
  const child = spawn(
    '/usr/bin/flock',
    [
      '-n',
      // eslint-disable-next-line sonarjs/publicly-writable-directories -- The owned mesh's shared lock coordinates every fault and staging run.
      `${process.env['MESH_DIR'] ?? '/tmp/elohim-local-mesh'}/a2o.lock`,
      '/bin/bash',
      '-c',
      'echo locked; read -r _',
    ],
    { stdio: 'pipe' }
  );
  leases.set(world, child);
  const granted = await Promise.race([
    once(child.stdout, 'data').then(([chunk]) => String(chunk).includes('locked')),
    once(child, 'exit').then(() => false),
  ]);
  assert.ok(granted, 'household is already in use; no fault is allowed');
}

async function processStartTicks(pid: number): Promise<string> {
  const stat = await readFile(`/proc/${pid}/stat`, 'utf8');
  return stat.slice(stat.lastIndexOf(')') + 2).split(' ')[19];
}

/**
 * Resolve the apex doorway's OS process via hc-mesh.sh's own start-tick
 * ledger (`live_recorded_pid doorway b` — hc-mesh.sh section 1b names doorway
 * B the "apex/elohim.host stand-in", the same process the household fixture's
 * "apex" entry points at) rather than pgrep, exactly as
 * doorway-sibling-reader.steps.ts:77 does for doorway "a".
 */
async function ownedApexDoorwayProcess(): Promise<{
  pid: number;
  ticks: string;
  executable: string;
}> {
  const { stdout } = await run('bash', [
    '-c',
    'source "$1"; live_recorded_pid doorway "$2"',
    'apex-transition',
    meshScript,
    'b',
  ]);
  const pid = Number(stdout.trim());
  assert.ok(
    Number.isSafeInteger(pid) && pid > 1 && pid !== process.pid,
    'safe owned apex doorway PID required'
  );
  const ticks = await processStartTicks(pid);
  const executable = await readlink(`/proc/${pid}/exe`);
  return { pid, ticks, executable };
}

async function signalApexDoorway(
  state: ApexTransitionState,
  signal: 'SIGSTOP' | 'SIGCONT'
): Promise<void> {
  assert.ok(
    state.pid !== undefined && state.ticks !== undefined && state.executable !== undefined,
    'apex doorway process handle not captured yet — induce the shed step first'
  );
  assert.equal(
    await processStartTicks(state.pid),
    state.ticks,
    'refuse a recycled apex doorway PID'
  );
  assert.equal(
    await readlink(`/proc/${state.pid}/exe`),
    state.executable,
    'apex doorway executable changed'
  );
  process.kill(state.pid, signal);
  state.paused = signal === 'SIGSTOP';
}

// ---------------------------------------------------------------------------
// Scenario: Shared membership removes only the doorway that sheds and
// readmits it after recovery — entirely a membership-authority claim.
// ---------------------------------------------------------------------------

Given('both owned doorways advertise eligibility for the same public name', function (): void {
  requireHouseholdMembershipAuthority();
});

When(
  'the household makes one doorway report non-serving for three consecutive probes',
  function (): void {
    requireHouseholdMembershipAuthority();
  }
);

Then("only that doorway's owner records leave shared membership", function (): void {
  requireHouseholdMembershipAuthority();
});

Then(
  "its exclusive diagnostic name and its sibling's membership remain unchanged",
  function (): void {
    requireHouseholdMembershipAuthority();
  }
);

When('that doorway reports serving for two consecutive probes', function (): void {
  requireHouseholdMembershipAuthority();
});

Then(
  'its owner records rejoin shared membership without duplicating the sibling',
  function (): void {
    requireHouseholdMembershipAuthority();
  }
);

// ---------------------------------------------------------------------------
// Scenario: The apex name survives its doorway's shed — household-executable
// up through the induced shed; the public-name assertions name the gap.
// ---------------------------------------------------------------------------

Given(
  'a new visitor reaches the declared landing page through one owned public name',
  { timeout: FIRST_VISIT_TIMEOUT_MS },
  async function (this: E2EWorld): Promise<void> {
    const fixture = loadHouseholdMeshFixture();
    const apexUrl = requireFixtureDoorwayUrl(fixture, 'apex');
    const siblingUrl = requireFixtureDoorwayUrl(fixture, 'alpha');
    const commonsId = fixture.commonsEprId ?? 'elohim-host-landing';

    const visit = await getRaw(`${apexUrl}/`, { timeoutMs: RAW_FETCH_TIMEOUT_MS });
    assert.equal(
      visit.status,
      200,
      `apex doorway (${apexUrl}) did not answer the declared landing page: HTTP ${visit.status}`
    );
    assert.ok(
      visit.text.includes('app-root'),
      `apex doorway (${apexUrl}) served a page with no app-root mount`
    );

    const head = await probeDeclaredHead(apexUrl, commonsId);
    const versionFile = await getRaw(`${apexUrl}/version.json`, {
      timeoutMs: RAW_FETCH_TIMEOUT_MS,
    });

    states.set(this, {
      apexUrl,
      siblingUrl,
      commonsId,
      observedDoorway: 'apex',
      declaredHeadAtVisit: head.headActionHash,
      buildStampAtVisit: stampOf(versionFile.text),
      paused: false,
    });
  }
);

Given('the household observes which doorway answered that visit', function (this: E2EWorld): void {
  // Test-side observation of which owned address this run happened to hit
  // — not a wire signal the doorway itself emits. Honest because the
  // "public name" step above visited exactly one address (apex) and
  // recorded that fact directly, rather than inferring it from a header.
  const state = getState(this);
  assert.equal(
    state.observedDoorway,
    'apex',
    'the first visit must be attributed to a specific owned doorway before any fault is induced'
  );
});

When(
  'the household makes that doorway shed while its sibling keeps serving the same declared head',
  { timeout: INDUCE_SHED_TIMEOUT_MS },
  async function (this: E2EWorld): Promise<void> {
    const state = getState(this);
    await acquireLease(this);
    const handle = await ownedApexDoorwayProcess();
    state.pid = handle.pid;
    state.ticks = handle.ticks;
    state.executable = handle.executable;
    await signalApexDoorway(state, 'SIGSTOP');

    const siblingState = await classifyDoorwayState(state.siblingUrl);
    assert.equal(
      siblingState,
      'serving',
      `sibling doorway (${state.siblingUrl}) must keep serving while apex sheds; classified "${siblingState}"`
    );

    const siblingHead = await probeDeclaredHead(state.siblingUrl, state.commonsId);
    assert.equal(
      siblingHead.headActionHash,
      state.declaredHeadAtVisit,
      `sibling doorway declares head "${siblingHead.headActionHash}", but the first visit recorded ` +
        `"${state.declaredHeadAtVisit}" — failover must not change the answer`
    );
  }
);

Then(
  'another new visitor using the same public name receives the landing page from the sibling',
  function (): void {
    // The crux of this scenario's title: the SAME name landing on a
    // DIFFERENT owned doorway is exactly the membership/routing behaviour
    // the household has no apparatus for (see requireHouseholdMembershipAuthority).
    // Querying the sibling's own distinct address directly and calling that
    // "the same public name" would be the test-only-proxy fake this
    // feature's preamble refuses to certify — so this names the gap instead.
    requireHouseholdMembershipAuthority();
  }
);

Then(
  'that visitor completes browser bootstrap with the same declared build stamp',
  { timeout: RAW_FETCH_TIMEOUT_MS + 5_000 },
  async function (this: E2EWorld): Promise<void> {
    const state = getState(this);
    const served = await getRaw(`${state.siblingUrl}/version.json`, {
      timeoutMs: RAW_FETCH_TIMEOUT_MS,
    });
    assert.equal(
      served.status,
      200,
      `sibling doorway (${state.siblingUrl}) GET /version.json answered ${served.status}`
    );
    const servedStamp = stampOf(served.text);
    assert.ok(
      servedStamp.length > 0,
      `sibling doorway (${state.siblingUrl}) /version.json carries no "commit" field`
    );
    assert.strictEqual(
      servedStamp,
      state.buildStampAtVisit,
      `build stamp diverged across the shed: apex served "${state.buildStampAtVisit}", sibling ` +
        `serves "${servedStamp}" — two builds, one address`
    );
  }
);

When(
  'the household restores the shedding doorway',
  { timeout: 20_000 },
  async function (this: E2EWorld): Promise<void> {
    const state = getState(this);
    if (state.paused) {
      await signalApexDoorway(state, 'SIGCONT');
    }
  }
);

Then('it rejoins the serving set within the declared recovery bound', function (): void {
  // "Serving set" is this feature's own name for shared membership (see the
  // feature Background: "Shared membership is the set of doorway addresses
  // currently advertised as eligible to serve."). Rejoining it is a
  // membership-authority claim, not a liveness claim.
  requireHouseholdMembershipAuthority();
});

Then('the same public name still serves the declared landing page', function (): void {
  requireHouseholdMembershipAuthority();
});

// ---------------------------------------------------------------------------
// Teardown: recovery has priority over everything else, exactly like
// doorway-sibling-reader.steps.ts's own @concern:doorway-failover After hook.
// A scenario that fails at the membership assertion above never reaches the
// Gherkin "restores the shedding doorway" step, so this hook is the ONLY
// guaranteed release of a SIGSTOP'd apex doorway on that path.
// ---------------------------------------------------------------------------

After({ tags: '@concern:doorway-failover', timeout: 20_000 }, async function (this: E2EWorld) {
  const state = states.get(this);
  try {
    if (!state?.paused) return;
    await signalApexDoorway(state, 'SIGCONT');
  } finally {
    leases.get(this)?.stdin.end();
    leases.delete(this);
    states.delete(this);
  }
});
