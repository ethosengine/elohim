/**
 * Step glue for the two apex-transition scenarios
 * (features/dataplane/doorway-apex-transition.feature, @concern:doorway-failover,
 * @requires:owned-substrate). Doorway federation S1 Task 6
 * (genesis/docs/superpowers/plans/2026-09-10-doorway-federation-three-reds-to-green-plan.md).
 *
 * The feature's preamble sets the bar: "The household must own the public-name
 * routing apparatus and its fault controls before exercising these scenarios …
 * A test-only proxy that does not execute the actual routing configuration
 * cannot certify the deployed path." It now does.
 *
 * THE APPARATUS. Shared membership is the set of ORIGINS currently eligible to
 * serve one public name. `relay-addr-beacon` decides that set from a serving
 * probe with join2/leave3 hysteresis (`reconcile_membership` +
 * `state::Membership`), and PROJECTS it — into Cloudflare as multi-A records on
 * the fleet, and into a JSON document the household owns through its `file`
 * sink (`relay-addr-beacon/src/sinks/file.rs`). `hc-mesh.sh`
 * (`start_membership_beacons`) stages one leg per owned doorway — owner `alpha`
 * -> :8888, owner `apex` -> :8889, public name `elohim.local` — each writing
 * exactly its OWN entry into `$MESH_DIR/membership/elohim.local.json`, and
 * declares the apparatus in the household fixture's `membershipAuthority`.
 * Same code path as the fleet, projection target the household owns.
 *
 * So the membership claims here are read from that document, and the
 * public-name resolution below is the household projection of the SAME routing
 * configuration the fleet's DNS carries: the dual-WAN design's §3a (multi-A +
 * client retry) — try the advertised origins in order, stick to the first that
 * serves. Nothing selects a doorway by fixture id; the document decides.
 *
 * The fault control is the household's own: the doorway process is SIGSTOP'd
 * and SIGCONT'd through the same start-tick-guarded /proc technique as
 * doorway-sibling-reader.steps.ts's `signalOwnedDoorway`. That is a HARDER
 * fault than a 503 shed — a stopped doorway cannot answer its diagnostic
 * address either — so the "exclusive diagnostic name" clause is asserted as
 * what the membership authority did and did not touch, never as reachability
 * the fault itself removed. See that assertion's own note.
 *
 * When no authority is staged, every membership step fails with the named
 * absence (requireMembershipAuthority) rather than reading a stale set.
 * WAN ingress continuity remains a DISTINCT prerequisite, not implied by
 * multi-origin membership — see doorway-failover.habit.md checks.
 */

import { strict as assert } from 'node:assert';
import { execFile, spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { once } from 'node:events';
import { readFile, readlink } from 'node:fs/promises';
import { setTimeout as delay } from 'node:timers/promises';
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
  requireMembershipAuthority,
} from '../../src/framework/fixtures/household-mesh.js';
import { E2EWorld } from '../../src/framework/world.js';

// The saga's own raw-response world slot — its "the raw response status/body"
// Then steps read from here. This chapter's sibling-visit step below records
// its real observation into the SAME slot so those steps see it, rather than
// each chapter keeping a private, unsynchronized capture store.
import { rawCapture } from './resiliency-saga.steps.js';

/**
 * The household's membership authority, already proven complete (public name,
 * document path and the owner slug per doorway) by `requireMembershipAuthority`.
 */
type MembershipAuthority = ReturnType<typeof requireMembershipAuthority>;

const run = promisify(execFile);
const meshScript = fileURLToPath(
  new URL('../../../../app/elohim-app/scripts/hc-mesh.sh', import.meta.url)
);

/** One bounded GET; sized like every other single-fetch step in this suite. */
const RAW_FETCH_TIMEOUT_MS = 30_000;
/** Fallback bounds when the authority declares none (it always does today). */
const DEFAULT_WITHDRAW_BOUND_MS = 30_000;
const DEFAULT_REJOIN_BOUND_MS = 20_000;
/** Fallback for the fixture's declared recovery bound. */
const DEFAULT_RECOVERY_BOUND_MS = 60_000;
/** How often a membership poll re-reads the document. */
const MEMBERSHIP_POLL_MS = 500;

/**
 * Resolving the public name tries each advertised origin in turn, so the worst
 * case is one bounded GET per member plus the landing/head/stamp reads.
 */
const FIRST_VISIT_TIMEOUT_MS = CATCHUP_RIDE_STEP_TIMEOUT_MS + 4 * RAW_FETCH_TIMEOUT_MS;
/**
 * Inducing the shed waits out the withdraw bound, then classifies the sibling
 * (up to 3 sequential bounded requests, per classifyDoorwayState) and probes
 * its declared head, on top of the process-signal work itself.
 */
const INDUCE_SHED_TIMEOUT_MS =
  DEFAULT_WITHDRAW_BOUND_MS + 3 * CLASSIFY_TIMEOUT_MS + CATCHUP_RIDE_STEP_TIMEOUT_MS + 30_000;

// ---------------------------------------------------------------------------
// The membership document — the household's projection of the eligible set.
// ---------------------------------------------------------------------------

/**
 * One advertised origin. Field names are the beacon document's own vocabulary
 * (`relay-addr-beacon/src/sinks/file.rs`), which is the same snake_case the
 * beacon's state file already uses — this is that crate's artifact, not the
 * elohim-storage View boundary.
 */
interface MembershipMember {
  owner: string;
  origin: string;
  updated_at: string;
}

interface MembershipDocument {
  name: string;
  members: MembershipMember[];
  updated_at: string;
}

async function readMembership(authority: MembershipAuthority): Promise<MembershipDocument> {
  let raw: string;
  try {
    raw = await readFile(authority.membershipFile, 'utf8');
  } catch (error) {
    throw new Error(
      `the household's membership document is missing (${authority.membershipFile}): ` +
        `${String(error)}. The beacon legs write it within a probe interval of ` +
        '`just mesh start`; check <logDir>/beacon-*.log.'
    );
  }
  const doc = JSON.parse(raw) as MembershipDocument;
  assert.ok(
    Array.isArray(doc.members),
    `membership document ${authority.membershipFile} carries no members array`
  );
  return doc;
}

/** Owner slugs currently advertised, in the document's own order. */
function advertised(doc: MembershipDocument): string[] {
  return doc.members.map(member => member.owner);
}

function memberFor(doc: MembershipDocument, owner: string): MembershipMember | undefined {
  return doc.members.find(member => member.owner === owner);
}

/**
 * Poll the membership document until `predicate` holds, or fail naming what
 * the set actually said. Membership is EARNED from consecutive probes, so
 * every membership assertion is a bounded convergence, never an instant read.
 */
async function untilMembership(
  authority: MembershipAuthority,
  boundMs: number,
  predicate: (doc: MembershipDocument) => boolean,
  what: string
): Promise<MembershipDocument> {
  const deadline = Date.now() + boundMs;
  let last: MembershipDocument | undefined;
  for (;;) {
    last = await readMembership(authority);
    if (predicate(last)) return last;
    if (Date.now() >= deadline) {
      throw new Error(
        `${what} within ${boundMs}ms; the set advertised for "${authority.publicName}" is ` +
          `[${advertised(last).join(', ')}] (document ${authority.membershipFile})`
      );
    }
    await delay(MEMBERSHIP_POLL_MS);
  }
}

/**
 * Resolve the public name THROUGH the membership document.
 *
 * This is deliberately the household projection of the same routing
 * configuration the fleet's DNS carries — dual-WAN design §3a, "multi-A +
 * client retry": the advertised addresses are tried IN ORDER and the client
 * sticks to the first that serves. Reading a doorway's own fixture URL instead
 * would be the test-only proxy this feature's preamble refuses.
 */
async function resolvePublicName(
  authority: MembershipAuthority,
  path: string
): Promise<{ owner: string; origin: string; status: number; text: string }> {
  const doc = await readMembership(authority);
  assert.ok(
    doc.members.length > 0,
    `no origin is advertised for "${authority.publicName}": the public name resolves to nothing ` +
      `(document ${authority.membershipFile})`
  );
  const attempts: string[] = [];
  for (const member of doc.members) {
    try {
      const response = await getRaw(`${member.origin}${path}`, {
        timeoutMs: RAW_FETCH_TIMEOUT_MS,
      });
      if (response.status === 200) {
        return { owner: member.owner, origin: member.origin, ...response };
      }
      attempts.push(`${member.owner} (${member.origin}) -> HTTP ${response.status}`);
    } catch (error) {
      attempts.push(`${member.owner} (${member.origin}) -> no answer: ${String(error)}`);
    }
  }
  throw new Error(
    `no origin advertised for "${authority.publicName}" served ${path}: ${attempts.join('; ')}`
  );
}

// ---------------------------------------------------------------------------
// Household fault control over the owned doorway processes.
// ---------------------------------------------------------------------------

/**
 * Fixture doorway id -> the name hc-mesh.sh records that doorway's PID under
 * (`$MESH_DIR/pids/doorway-<letter>`; sections 1 and 1b — doorway B is the
 * "apex/elohim.host stand-in"). The membership owner slugs are the same
 * fixture doorway ids, by construction in `start_membership_beacons`.
 */
const PID_LEDGER_LETTER: Readonly<Record<string, string>> = { alpha: 'a', apex: 'b' };

interface ApexTransitionState {
  authority: MembershipAuthority;
  commonsId: string;
  recoveryBoundMs: number;
  /** The owner whose doorway answered the first visit — then the one we fault. */
  observedOwner?: string;
  observedOrigin?: string;
  siblingOwner?: string;
  siblingOrigin?: string;
  declaredHeadAtVisit?: string;
  buildStampAtVisit?: string;
  /** The sibling's entry captured before the fault, for the unchanged assertion. */
  siblingEntryBeforeFault?: MembershipMember;
  pid?: number;
  ticks?: string;
  executable?: string;
  paused: boolean;
}

const states = new WeakMap<E2EWorld, ApexTransitionState>();
const leases = new WeakMap<E2EWorld, ChildProcessWithoutNullStreams>();

function getState(world: E2EWorld): ApexTransitionState {
  const state = states.get(world);
  assert.ok(state, 'the scenario must establish the public name and its membership set first');
  return state;
}

/** Load the authority and seed per-scenario state from it. */
function beginScenario(world: E2EWorld): ApexTransitionState {
  const fixture = loadHouseholdMeshFixture();
  const authority = requireMembershipAuthority(fixture);
  const state: ApexTransitionState = {
    authority,
    commonsId: fixture.commonsEprId ?? 'elohim-host-landing',
    recoveryBoundMs: fixture.convergenceWindowMs ?? DEFAULT_RECOVERY_BOUND_MS,
    paused: false,
  };
  states.set(world, state);
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
  if (leases.has(world)) return;
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
 * Resolve an owned doorway's OS process via hc-mesh.sh's own start-tick ledger
 * (`live_recorded_pid doorway <letter>`) rather than pgrep, exactly as
 * doorway-sibling-reader.steps.ts:77 does.
 */
async function ownedDoorwayProcess(
  owner: string
): Promise<{ pid: number; ticks: string; executable: string }> {
  const letter = PID_LEDGER_LETTER[owner];
  assert.ok(
    letter,
    `no household fault control for membership owner "${owner}": hc-mesh.sh records doorway PIDs ` +
      `for ${Object.keys(PID_LEDGER_LETTER).join(', ')} only`
  );
  const { stdout } = await run('bash', [
    '-c',
    'source "$1"; live_recorded_pid doorway "$2"',
    'apex-transition',
    meshScript,
    letter,
  ]);
  const pid = Number(stdout.trim());
  assert.ok(
    Number.isSafeInteger(pid) && pid > 1 && pid !== process.pid,
    `safe owned doorway PID required for "${owner}" (ledger name doorway-${letter})`
  );
  const ticks = await processStartTicks(pid);
  const executable = await readlink(`/proc/${pid}/exe`);
  return { pid, ticks, executable };
}

async function signalOwnedDoorway(
  state: ApexTransitionState,
  signal: 'SIGSTOP' | 'SIGCONT'
): Promise<void> {
  assert.ok(
    state.pid !== undefined && state.ticks !== undefined && state.executable !== undefined,
    'doorway process handle not captured yet — induce the fault step first'
  );
  assert.equal(await processStartTicks(state.pid), state.ticks, 'refuse a recycled doorway PID');
  assert.equal(
    await readlink(`/proc/${state.pid}/exe`),
    state.executable,
    'doorway executable changed'
  );
  process.kill(state.pid, signal);
  state.paused = signal === 'SIGSTOP';
}

/** Stop the doorway serving `owner`, having first recorded its sibling. */
async function faultOwnedDoorway(state: ApexTransitionState, owner: string): Promise<void> {
  const handle = await ownedDoorwayProcess(owner);
  state.pid = handle.pid;
  state.ticks = handle.ticks;
  state.executable = handle.executable;
  await signalOwnedDoorway(state, 'SIGSTOP');
}

function withdrawBound(state: ApexTransitionState): number {
  return state.authority.withdrawBoundMs ?? DEFAULT_WITHDRAW_BOUND_MS;
}

function rejoinBound(state: ApexTransitionState): number {
  return state.authority.rejoinBoundMs ?? DEFAULT_REJOIN_BOUND_MS;
}

// ---------------------------------------------------------------------------
// Scenario: Shared membership removes only the doorway that sheds and
// readmits it after recovery.
// ---------------------------------------------------------------------------

Given(
  'both owned doorways advertise eligibility for the same public name',
  { timeout: DEFAULT_REJOIN_BOUND_MS + 15_000 },
  async function (this: E2EWorld): Promise<void> {
    const state = beginScenario(this);
    const owners = Object.values(state.authority.owners);
    assert.ok(
      owners.length >= 2,
      `the household declares only ${owners.length} membership owner(s); this scenario is about ` +
        'two doorways under ONE public name'
    );

    const doc = await untilMembership(
      state.authority,
      rejoinBound(state) + 10_000,
      current => owners.every(owner => memberFor(current, owner) !== undefined),
      `owners [${owners.join(', ')}] did not all advertise eligibility`
    );
    assert.equal(
      doc.name,
      state.authority.publicName,
      `the membership document names "${doc.name}", the household declares "${state.authority.publicName}"`
    );
    for (const owner of owners) {
      assert.equal(
        doc.members.filter(member => member.owner === owner).length,
        1,
        `owner "${owner}" holds more than one entry in the advertised set`
      );
    }
    // The fault in this scenario is the apex doorway's, exactly as the fleet
    // apex is the name under study; its sibling is whatever else is advertised.
    const faulted = state.authority.owners['apex'] ?? owners.at(-1);
    assert.ok(faulted, 'the household declares no owner to fault');
    state.observedOwner = faulted;
    state.siblingOwner = owners.find(owner => owner !== faulted);
    assert.ok(state.siblingOwner, 'the advertised set names no sibling owner');
    state.observedOrigin = memberFor(doc, faulted)?.origin;
    state.siblingOrigin = memberFor(doc, state.siblingOwner)?.origin;
    state.siblingEntryBeforeFault = memberFor(doc, state.siblingOwner);
  }
);

When(
  'the household makes one doorway report non-serving for three consecutive probes',
  { timeout: INDUCE_SHED_TIMEOUT_MS },
  async function (this: E2EWorld): Promise<void> {
    const state = getState(this);
    assert.ok(state.observedOwner, 'no owner selected to fault');
    await acquireLease(this);
    await faultOwnedDoorway(state, state.observedOwner);
    // Wait out the declared withdraw bound rather than polling for success:
    // the assertion belongs in the Then, where it can fail.
    await delay(withdrawBound(state));
  }
);

Then(
  "only that doorway's owner records leave shared membership",
  { timeout: 30_000 },
  async function (this: E2EWorld): Promise<void> {
    const state = getState(this);
    const doc = await readMembership(state.authority);
    assert.equal(
      memberFor(doc, state.observedOwner as string),
      undefined,
      `owner "${state.observedOwner}" is still advertised for "${state.authority.publicName}" ` +
        `after ${withdrawBound(state)}ms of non-serving probes: [${advertised(doc).join(', ')}]`
    );
    assert.ok(
      memberFor(doc, state.siblingOwner as string),
      `the sibling "${state.siblingOwner}" left the set too — a withdrawal must remove only the ` +
        `owner's OWN records; the set is now [${advertised(doc).join(', ')}]`
    );
  }
);

Then(
  "its exclusive diagnostic name and its sibling's membership remain unchanged",
  { timeout: 30_000 },
  async function (this: E2EWorld): Promise<void> {
    const state = getState(this);
    const doc = await readMembership(state.authority);

    // The sibling's entry is compared WHOLE, freshness stamp included: a leg
    // that rewrote a sibling's record — even to the same value — would not be
    // doing exact-owner writes, and this is the assertion that can see it.
    assert.deepEqual(
      memberFor(doc, state.siblingOwner as string),
      state.siblingEntryBeforeFault,
      `the sibling "${state.siblingOwner}" entry changed across the withdrawal; each doorway owns ` +
        'only the records it contributes'
    );

    // The exclusive diagnostic name is the address that reaches ONLY that
    // doorway, separate from the shared set. Its availability is not asserted
    // here on purpose: the household's fault control is SIGSTOP, which is
    // harder than the 503 shed the feature describes, and a stopped process
    // cannot answer its own diagnostic address either. What IS asserted is
    // that the membership authority left it alone — a withdrawal from the
    // shared set never retracts the doorway's own address.
    const fixture = loadHouseholdMeshFixture();
    const diagnostic = requireFixtureDoorwayUrl(fixture, 'apex');
    assert.equal(
      diagnostic,
      state.observedOrigin,
      `the withdrawn doorway's exclusive diagnostic address (${diagnostic}) no longer matches the ` +
        `origin it contributed to the shared set (${state.observedOrigin})`
    );
  }
);

When(
  'that doorway reports serving for two consecutive probes',
  { timeout: DEFAULT_REJOIN_BOUND_MS + 30_000 },
  async function (this: E2EWorld): Promise<void> {
    const state = getState(this);
    if (state.paused) await signalOwnedDoorway(state, 'SIGCONT');
    await delay(rejoinBound(state));
  }
);

Then(
  'its owner records rejoin shared membership without duplicating the sibling',
  { timeout: 30_000 },
  async function (this: E2EWorld): Promise<void> {
    const state = getState(this);
    const doc = await readMembership(state.authority);
    assert.ok(
      memberFor(doc, state.observedOwner as string),
      `owner "${state.observedOwner}" did not rejoin within ${rejoinBound(state)}ms: ` +
        `[${advertised(doc).join(', ')}]`
    );
    for (const owner of Object.values(state.authority.owners)) {
      assert.equal(
        doc.members.filter(member => member.owner === owner).length,
        1,
        `owner "${owner}" is advertised more than once after the rejoin: ` +
          `[${advertised(doc).join(', ')}]`
      );
    }
    assert.equal(
      doc.members.length,
      Object.values(state.authority.owners).length,
      `the advertised set is [${advertised(doc).join(', ')}], not exactly one entry per declared owner`
    );
  }
);

// ---------------------------------------------------------------------------
// Scenario: The apex name survives its doorway's shed.
// ---------------------------------------------------------------------------

Given(
  'a new visitor reaches the declared landing page through one owned public name',
  { timeout: FIRST_VISIT_TIMEOUT_MS },
  async function (this: E2EWorld): Promise<void> {
    const state = beginScenario(this);
    const owners = Object.values(state.authority.owners);
    await untilMembership(
      state.authority,
      rejoinBound(state) + 10_000,
      current => owners.every(owner => memberFor(current, owner) !== undefined),
      `owners [${owners.join(', ')}] are not all advertised, so no visit can fail over`
    );

    const visit = await resolvePublicName(state.authority, '/');
    assert.ok(
      visit.text.includes('app-root'),
      `"${state.authority.publicName}" served a page with no app-root mount from ${visit.origin}`
    );

    const head = await probeDeclaredHead(visit.origin, state.commonsId);
    const versionFile = await getRaw(`${visit.origin}/version.json`, {
      timeoutMs: RAW_FETCH_TIMEOUT_MS,
    });

    state.observedOwner = visit.owner;
    state.observedOrigin = visit.origin;
    state.siblingOwner = owners.find(owner => owner !== visit.owner);
    assert.ok(
      state.siblingOwner,
      `only "${visit.owner}" is advertised for "${state.authority.publicName}"; there is no sibling ` +
        'for the name to survive onto'
    );
    const doc = await readMembership(state.authority);
    state.siblingOrigin = memberFor(doc, state.siblingOwner)?.origin;
    state.siblingEntryBeforeFault = memberFor(doc, state.siblingOwner);
    state.declaredHeadAtVisit = head.headActionHash;
    state.buildStampAtVisit = stampOf(versionFile.text);
  }
);

Given('the household observes which doorway answered that visit', function (this: E2EWorld): void {
  // Not a wire signal the doorway emits: the resolution above tried the
  // advertised origins in order and recorded WHICH one answered 200, which is
  // the same thing a client learns from its own connection.
  const state = getState(this);
  assert.ok(
    state.observedOwner && state.observedOrigin,
    'the first visit must be attributed to a specific advertised origin before any fault is induced'
  );
});

When(
  'the household makes that doorway shed while its sibling keeps serving the same declared head',
  { timeout: INDUCE_SHED_TIMEOUT_MS },
  async function (this: E2EWorld): Promise<void> {
    const state = getState(this);
    await acquireLease(this);
    await faultOwnedDoorway(state, state.observedOwner as string);

    const siblingState = await classifyDoorwayState(state.siblingOrigin as string);
    assert.equal(
      siblingState,
      'serving',
      `sibling doorway (${state.siblingOrigin}) must keep serving while ${state.observedOwner} ` +
        `sheds; classified "${siblingState}"`
    );

    const siblingHead = await probeDeclaredHead(state.siblingOrigin as string, state.commonsId);
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
  { timeout: DEFAULT_WITHDRAW_BOUND_MS + 4 * RAW_FETCH_TIMEOUT_MS + 20_000 },
  async function (this: E2EWorld): Promise<void> {
    const state = getState(this);
    // The name must stop advertising the stopped doorway first — that is the
    // membership half. Selection is then the client's, over what remains.
    await untilMembership(
      state.authority,
      withdrawBound(state) + 10_000,
      doc => memberFor(doc, state.observedOwner as string) === undefined,
      `"${state.observedOwner}" is still advertised after its doorway stopped serving`
    );

    const visit = await resolvePublicName(state.authority, '/');
    assert.equal(
      visit.owner,
      state.siblingOwner,
      `"${state.authority.publicName}" was served by "${visit.owner}" (${visit.origin}); the ` +
        `sibling "${state.siblingOwner}" (${state.siblingOrigin}) is the doorway that must answer ` +
        'once the faulted one leaves the set'
    );
    assert.equal(visit.status, 200, `the raw response status was ${visit.status}`);
    assert.ok(
      visit.text.includes('app-root'),
      `the sibling (${visit.origin}) served a page with no app-root mount`
    );
    state.observedOrigin = visit.origin;
    // Record the REAL sibling response into the saga's shared world slot so
    // the scenario's following "raw response status/body" steps
    // (resiliency-saga.steps.ts) assert on what this visit actually observed,
    // not on an empty, never-populated capture.
    rawCapture.set(this, {
      status: visit.status,
      text: visit.text,
      url: `${visit.origin}/`,
      ride: '',
    });
  }
);

Then(
  'that visitor completes browser bootstrap with the same declared build stamp',
  { timeout: RAW_FETCH_TIMEOUT_MS + 5_000 },
  async function (this: E2EWorld): Promise<void> {
    const state = getState(this);
    const served = await getRaw(`${state.siblingOrigin}/version.json`, {
      timeoutMs: RAW_FETCH_TIMEOUT_MS,
    });
    assert.equal(
      served.status,
      200,
      `sibling doorway (${state.siblingOrigin}) GET /version.json answered ${served.status}`
    );
    const servedStamp = stampOf(served.text);
    assert.ok(
      servedStamp.length > 0,
      `sibling doorway (${state.siblingOrigin}) /version.json carries no "commit" field`
    );
    assert.strictEqual(
      servedStamp,
      state.buildStampAtVisit,
      `build stamp diverged across the shed: the first visit served "${state.buildStampAtVisit}", ` +
        `the sibling serves "${servedStamp}" — two builds, one name`
    );
  }
);

When(
  'the household restores the shedding doorway',
  { timeout: 20_000 },
  async function (this: E2EWorld): Promise<void> {
    const state = getState(this);
    if (state.paused) {
      await signalOwnedDoorway(state, 'SIGCONT');
    }
  }
);

Then(
  'it rejoins the serving set within the declared recovery bound',
  { timeout: DEFAULT_RECOVERY_BOUND_MS + 30_000 },
  async function (this: E2EWorld): Promise<void> {
    const state = getState(this);
    // "Serving set" is this feature's own name for shared membership (see the
    // feature preamble). Rejoining it is a membership claim, bounded by the
    // recovery window the household fixture declares.
    const doc = await untilMembership(
      state.authority,
      state.recoveryBoundMs,
      current => memberFor(current, state.observedOwner as string) !== undefined,
      `"${state.observedOwner}" did not rejoin the serving set`
    );
    assert.equal(
      doc.members.filter(member => member.owner === state.observedOwner).length,
      1,
      `"${state.observedOwner}" rejoined more than once: [${advertised(doc).join(', ')}]`
    );
  }
);

Then(
  'the same public name still serves the declared landing page',
  { timeout: 4 * RAW_FETCH_TIMEOUT_MS + CATCHUP_RIDE_STEP_TIMEOUT_MS },
  async function (this: E2EWorld): Promise<void> {
    const state = getState(this);
    const visit = await resolvePublicName(state.authority, '/');
    assert.equal(visit.status, 200, `the raw response status was ${visit.status}`);
    assert.ok(
      visit.text.includes('app-root'),
      `"${state.authority.publicName}" served a page with no app-root mount from ${visit.origin}`
    );
    const head = await probeDeclaredHead(visit.origin, state.commonsId);
    assert.equal(
      head.headActionHash,
      state.declaredHeadAtVisit,
      `after recovery "${state.authority.publicName}" declares head "${head.headActionHash}", but ` +
        `the first visit recorded "${state.declaredHeadAtVisit}"`
    );
  }
);

// ---------------------------------------------------------------------------
// Teardown: recovery has priority over everything else, exactly like
// doorway-sibling-reader.steps.ts's own @concern:doorway-failover After hook.
// A scenario that fails at a membership assertion never reaches the Gherkin
// "restores the shedding doorway" step, so this hook is the ONLY guaranteed
// release of a SIGSTOP'd doorway on that path.
// ---------------------------------------------------------------------------

After({ tags: '@concern:doorway-failover', timeout: 20_000 }, async function (this: E2EWorld) {
  const state = states.get(this);
  try {
    if (!state?.paused) return;
    await signalOwnedDoorway(state, 'SIGCONT');
  } finally {
    leases.get(this)?.stdin.end();
    leases.delete(this);
    states.delete(this);
  }
});
