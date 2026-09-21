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
  chromium,
  type Browser,
  type BrowserContext,
  type Page,
  type Request,
  type WebSocket,
} from 'playwright';

import { awaitOwnedProcessRecovery } from '../../src/framework/dataplane/owned-process-recovery.js';
import {
  awaitCanonicalManifestoCandidate,
  classifyFirstPartyHttpError,
  httpOriginForWebSocketUrl,
  isOptionalNavigationCancellation,
  persistRealAppPhaseArtifacts,
  publicDoorwayUrl,
  publicDoorwayPorts,
  requiredRequestRoutingFailure,
  unexpectedOptionalNegatives,
} from '../../src/framework/dataplane/real-app-network.js';
import {
  completeOnceWithinDeadline,
  waitForStrictAuthorityConvergence,
} from '../../src/framework/dataplane/strict-authority-convergence.js';
import {
  classifyDoorwayState,
  getRaw,
  probeDeclaredHead,
  CLASSIFY_TIMEOUT_MS,
  CATCHUP_RIDE_STEP_TIMEOUT_MS,
} from '../../src/framework/dataplane/surfaces.js';
import {
  householdMeshDir,
  loadHouseholdMeshFixture,
  membershipLanes,
  requireFixtureDoorwayUrl,
  requireMembershipAuthority,
  type MembershipLaneFixture,
} from '../../src/framework/fixtures/household-mesh.js';
import { E2EWorld } from '../../src/framework/world.js';

// The saga's own raw-response world slot — its "the raw response status/body"
// Then steps read from here. This chapter's sibling-visit step below records
// its real observation into the SAME slot so those steps see it, rather than
// each chapter keeping a private, unsynchronized capture store.
import { visitInBrowser } from './epr-app-deliverability.helpers.js';
import {
  chaosAuthorReceipt,
  type ChaosAuthorReceipt,
  type ChaosPublishedAuthority,
} from './epr-app-deliverability.steps.js';
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
/** Six server reads plus one browser boot in assertExactPublishedAuthority. */
const EXACT_AUTHORITY_TIMEOUT_MS = 7 * RAW_FETCH_TIMEOUT_MS;
const AUTHORITY_PUBLICATION_BOUND_MS = 75_000;
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

/**
 * ONE DOCUMENT NAMES ONE PUBLIC NAME. A household publishing several names has
 * one document per name, maintained by the SAME legs (one repeatable
 * `--shared-record <name>=<owner>` per name, `relay-addr-beacon/src/config.rs`).
 * Every membership read below therefore names WHICH document it read — the
 * authority satisfies this shape for the converged name, and each entry of the
 * fixture's `lanes` satisfies it for its own name.
 */
interface MembershipDocumentSource {
  publicName: string;
  membershipFile: string;
}

async function readMembership(lane: MembershipDocumentSource): Promise<MembershipDocument> {
  let raw: string;
  try {
    raw = await readFile(lane.membershipFile, 'utf8');
  } catch (error) {
    throw new Error(
      `the household's membership document for "${lane.publicName}" is missing ` +
        `(${lane.membershipFile}): ${String(error)}. The beacon legs write it within a probe ` +
        'interval of `just mesh start`; check <logDir>/beacon-*.log.'
    );
  }
  const doc = JSON.parse(raw) as MembershipDocument;
  assert.ok(
    Array.isArray(doc.members),
    `membership document ${lane.membershipFile} carries no members array`
  );
  assert.equal(
    doc.name,
    lane.publicName,
    `the document at ${lane.membershipFile} names "${doc.name}", but the household declares it ` +
      `as the set for "${lane.publicName}" — one document names one public name, so a document ` +
      'carrying another name means a leg wrote the wrong set'
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
  lane: MembershipDocumentSource,
  boundMs: number,
  predicate: (doc: MembershipDocument) => boolean,
  what: string
): Promise<MembershipDocument> {
  const deadline = Date.now() + boundMs;
  let last: MembershipDocument | undefined;
  for (;;) {
    last = await readMembership(lane);
    if (predicate(last)) return last;
    if (Date.now() >= deadline) {
      throw new Error(
        `${what} within ${boundMs}ms; the set advertised for "${lane.publicName}" is ` +
          `[${advertised(last).join(', ')}] (document ${lane.membershipFile})`
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
  lane: MembershipDocumentSource,
  path: string
): Promise<{ owner: string; origin: string; status: number; text: string }> {
  const doc = await readMembership(lane);
  assert.ok(
    doc.members.length > 0,
    `no origin is advertised for "${lane.publicName}": the public name resolves to nothing ` +
      `(document ${lane.membershipFile})`
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
    `no origin advertised for "${lane.publicName}" served ${path}: ${attempts.join('; ')}`
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
const ALPHA_DOORWAY_ID = 'alpha-A';
const APEX_DOORWAY_ID = 'elohim.host';
const OWNED_DOORWAY_IDS = [ALPHA_DOORWAY_ID, APEX_DOORWAY_ID] as const;

interface ApexTransitionState {
  authority: MembershipAuthority;
  /** Both owned socket legs, frozen before any membership withdrawal. */
  ownedPorts: string[];
  commonsId: string;
  recoveryBoundMs: number;
  /** The owner whose doorway answered the first visit — then the one we fault. */
  observedOwner?: string;
  observedOrigin?: string;
  /** Immutable origin of the process actually faulted; visitor routing may later move. */
  faultedOrigin?: string;
  siblingOwner?: string;
  siblingOrigin?: string;
  declaredHeadAtVisit?: string;
  buildStampAtVisit?: string;
  authorityA?: ChaosAuthorReceipt;
  authorityB?: ChaosAuthorReceipt;
  /** The sibling's entry captured before the fault, for the unchanged assertion. */
  siblingEntryBeforeFault?: MembershipMember;
  /** Every public name this household publishes, when the scenario is about all of them. */
  lanes?: MembershipLaneFixture[];
  /** Per public name, the sibling's entry before the fault — keyed by public name. */
  siblingEntryByName?: Map<string, MembershipMember>;
  pid?: number;
  ticks?: string;
  executable?: string;
  paused: boolean;
  /** Exact contract-approved negative reads observed before the fault, by serving owner. */
  realAppBaselineOptionalNegatives?: Map<string, Set<string>>;
  /** Browser console errors observed before the fault, by serving owner. */
  realAppBaselineConsoleErrors?: Map<string, Set<string>>;
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
    ownedPorts: publicDoorwayPorts(fixture),
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

async function assertExactPublishedAuthority(
  origin: string,
  expected: ChaosPublishedAuthority
): Promise<void> {
  const nonce = `chaos-${Date.now().toString(36)}`;
  const fresh = (path: string) => `${path}${path.includes('?') ? '&' : '?'}chaos=${nonce}`;
  const headPath = `/db/content/${encodeURIComponent(expected.slug)}/head`;
  const contentPath = `/db/content/${encodeURIComponent(expected.slug)}`;
  const versionPath = `/apps/${encodeURIComponent(expected.blobHash)}/version.json`;
  const head = await getRaw(`${origin}${fresh(headPath)}`, { timeoutMs: RAW_FETCH_TIMEOUT_MS });
  assert.equal(head.status, 200, `${origin}: governed head answered ${head.status}`);
  const headBody = JSON.parse(head.text) as { headActionHash?: string };
  assert.equal(headBody.headActionHash, expected.actionHash, `${origin}: governed action diverged`);
  const content = await getRaw(`${origin}${fresh(contentPath)}`, {
    timeoutMs: RAW_FETCH_TIMEOUT_MS,
  });
  assert.equal(content.status, 200, `${origin}: governed content answered ${content.status}`);
  const contentBody = JSON.parse(content.text) as { blobHash?: string };
  assert.equal(contentBody.blobHash, expected.blobHash, `${origin}: governed blob diverged`);
  const version = await getRaw(`${origin}${fresh(versionPath)}`, {
    timeoutMs: RAW_FETCH_TIMEOUT_MS,
  });
  assert.equal(version.status, 200, `${origin}: addressed version answered ${version.status}`);
  assert.equal(stampOf(version.text), expected.version, `${origin}: governed version diverged`);
  const page = await getRaw(`${origin}${fresh(expected.mountPath)}`, {
    timeoutMs: RAW_FETCH_TIMEOUT_MS,
  });
  assert.equal(page.status, 200, `${origin}: governed page answered ${page.status}`);
  assert.ok(
    page.text.includes(expected.entryScript),
    `${origin}: page does not name current script`
  );
  for (const asset of [expected.entryScript, expected.styleSheet]) {
    const assetPath = `${expected.mountPath.replace(/\/$/, '')}/${asset}`;
    const response = await getRaw(`${origin}${fresh(assetPath)}`, {
      timeoutMs: RAW_FETCH_TIMEOUT_MS,
    });
    assert.equal(
      response.status,
      200,
      `${origin}: current asset ${asset} answered ${response.status}`
    );
  }
  const browser = await visitInBrowser(`${origin}${fresh(expected.mountPath)}`);
  assert.deepEqual(browser.pageErrors, [], `${origin}: browser page errors`);
  assert.deepEqual(browser.failedRequests, [], `${origin}: browser request failures`);
  assert.deepEqual(browser.httpErrors, [], `${origin}: browser HTTP errors`);
  assert.ok(browser.rootPresent, `${origin}: browser saw no app-root`);
  assert.ok(browser.bootstrapReady, `${origin}: current entry script did not bootstrap`);
  assert.ok(
    browser.rootText.includes(expected.version.replace(/^fixture-/, '')),
    `${origin}: browser booted a root that does not identify the expected version`
  );
}

async function lightweightAuthorityMismatch(
  origin: string,
  expected: ChaosPublishedAuthority,
  remainingMs: number
): Promise<string | undefined> {
  const timeoutMs = Math.max(1, Math.min(10_000, remainingMs));
  const nonce = `authority-${Date.now().toString(36)}`;
  const fresh = (path: string) => `${path}${path.includes('?') ? '&' : '?'}chaos=${nonce}`;
  const encodedSlug = encodeURIComponent(expected.slug);
  const encodedBlobHash = encodeURIComponent(expected.blobHash);
  const headUrl = origin + fresh(`/db/content/${encodedSlug}/head`);
  const contentUrl = origin + fresh(`/db/content/${encodedSlug}`);
  const versionUrl = origin + fresh(`/apps/${encodedBlobHash}/version.json`);
  const [head, content, version] = await Promise.all([
    getRaw(headUrl, { timeoutMs }),
    getRaw(contentUrl, { timeoutMs }),
    getRaw(versionUrl, { timeoutMs }),
  ]);
  if (head.status !== 200) return `head status ${head.status}`;
  if (content.status !== 200) return `content status ${content.status}`;
  if (version.status !== 200) return `version status ${version.status}`;
  const headBody = JSON.parse(head.text) as { headActionHash?: string; blobHash?: string };
  const contentBody = JSON.parse(content.text) as { blobHash?: string };
  if (headBody.headActionHash !== expected.actionHash)
    return `action expected ${expected.actionHash}, observed ${headBody.headActionHash ?? 'absent'}`;
  if (headBody.blobHash !== expected.blobHash)
    return `head blob expected ${expected.blobHash}, observed ${headBody.blobHash ?? 'absent'}`;
  if (contentBody.blobHash !== expected.blobHash)
    return `content blob expected ${expected.blobHash}, observed ${contentBody.blobHash ?? 'absent'}`;
  const versionStamp = stampOf(version.text);
  return versionStamp === expected.version
    ? undefined
    : `version expected ${expected.version}, observed ${versionStamp || 'absent'}`;
}

async function assertExactPublishedSurface(
  origin: string,
  expected: ChaosPublishedAuthority,
  deadlineAt: number
): Promise<void> {
  const nonce = `chaos-surface-${Date.now().toString(36)}`;
  const fresh = (path: string) => `${path}${path.includes('?') ? '&' : '?'}chaos=${nonce}`;
  const remaining = () => Math.max(1, deadlineAt - Date.now());
  const page = await getRaw(`${origin}${fresh(expected.mountPath)}`, {
    timeoutMs: Math.min(RAW_FETCH_TIMEOUT_MS, remaining()),
  });
  assert.equal(page.status, 200, `${origin}: governed page answered ${page.status}`);
  assert.ok(
    page.text.includes(expected.entryScript),
    `${origin}: page does not name current script`
  );
  for (const asset of [expected.entryScript, expected.styleSheet]) {
    const assetPath = `${expected.mountPath.replace(/\/$/, '')}/${asset}`;
    const response = await getRaw(`${origin}${fresh(assetPath)}`, {
      timeoutMs: Math.min(RAW_FETCH_TIMEOUT_MS, remaining()),
    });
    assert.equal(
      response.status,
      200,
      `${origin}: current asset ${asset} answered ${response.status}`
    );
  }
  const browser = await completeOnceWithinDeadline(
    deadlineAt,
    async remainingMs =>
      await visitInBrowser(
        `${origin}${fresh(expected.mountPath)}`,
        Math.min(RAW_FETCH_TIMEOUT_MS, remainingMs)
      )
  );
  assert.deepEqual(browser.pageErrors, [], `${origin}: browser page errors`);
  assert.deepEqual(browser.failedRequests, [], `${origin}: browser request failures`);
  assert.deepEqual(browser.httpErrors, [], `${origin}: browser HTTP errors`);
  assert.ok(browser.rootPresent, `${origin}: browser saw no app-root`);
  assert.ok(browser.bootstrapReady, `${origin}: current entry script did not bootstrap`);
  assert.ok(
    browser.rootText.includes(expected.version.replace(/^fixture-/, '')),
    `${origin}: browser booted a root that does not identify the expected version`
  );
}

async function selectJourneyDoorway(
  state: ApexTransitionState,
  phase: string
): Promise<{ owner: string; origin: string }> {
  const expectedOwner =
    phase === 'survivor' || phase === 'baseline-apex'
      ? state.authority.owners['apex']
      : state.authority.owners['alpha'];
  const selected =
    phase === 'survivor'
      ? await resolvePublicName(state.authority, '/')
      : memberFor(await readMembership(state.authority), expectedOwner);
  assert.ok(selected, `${phase}: recovered alpha-A is absent from membership`);
  assert.equal(selected.owner, expectedOwner);
  return selected;
}

async function completeRealAppJourney(world: E2EWorld, phase: string): Promise<void> {
  const state = getState(world);
  const selected = await selectJourneyDoorway(state, phase);
  const publicOrigin = publicDoorwayUrl(selected.origin, state.authority.publicName);
  let browser: Browser | undefined;
  let context: BrowserContext | undefined;
  let page: Page | undefined;
  const pageErrors: string[] = [];
  const consoleErrors: string[] = [];
  const allFirstPartyFailures: string[] = [];
  const requiredFirstPartyFailures: string[] = [];
  const httpErrors: string[] = [];
  const requiredHttpErrors: string[] = [];
  const optionalNegatives = new Set<string>();
  const activeFirstParty = new Map<Request, { rendered: string; optionalDiscovery: boolean }>();
  const activeFirstPartyAtBoundary: string[] = [];
  let manifestoText = '';
  let screenshot: Buffer | undefined;
  try {
    browser = await chromium.launch({
      headless: true,
      args: [`--host-resolver-rules=MAP ${state.authority.publicName} 127.0.0.1`],
    });
    context = await browser.newContext();
    page = await context.newPage();
    page.on('pageerror', error => pageErrors.push(error.message));
    page.on('console', message => {
      if (message.type() === 'error') consoleErrors.push(message.text());
    });
    page.on('request', request => {
      const url = new URL(request.url());
      const routingFailure = requiredRequestRoutingFailure({
        requestUrl: request.url(),
        selectedOrigin: publicOrigin,
        publicHostname: state.authority.publicName,
        ownedPorts: state.ownedPorts,
      });
      if (routingFailure)
        requiredFirstPartyFailures.push(`${request.method()} ${request.url()}: ${routingFailure}`);
      if (url.origin !== publicOrigin) return;
      activeFirstParty.set(request, {
        rendered: `${request.method()} ${request.url()}`,
        optionalDiscovery:
          request.method() === 'GET' && url.pathname === '/api/v1/federation/doorways',
      });
    });
    // A websocket is invisible to page.on('request') — the same owned-origin
    // rule must still see it, mapped to its http(s) counterpart so a ws(s)://
    // origin compares against the selected http(s):// origin correctly.
    page.on('websocket', (socket: WebSocket) => {
      const httpUrl = httpOriginForWebSocketUrl(socket.url());
      const routingFailure = requiredRequestRoutingFailure({
        requestUrl: httpUrl,
        selectedOrigin: publicOrigin,
        publicHostname: state.authority.publicName,
        ownedPorts: state.ownedPorts,
      });
      if (routingFailure) requiredFirstPartyFailures.push(`WS ${socket.url()}: ${routingFailure}`);
    });
    page.on('requestfinished', request => activeFirstParty.delete(request));
    page.on('requestfailed', request => {
      activeFirstParty.delete(request);
      if (new URL(request.url()).origin !== publicOrigin) return;
      const failure = `${request.method()} ${request.url()}: ${request.failure()?.errorText}`;
      allFirstPartyFailures.push(failure);
      const path = new URL(request.url()).pathname;
      const navigationCancelledDiscovery = isOptionalNavigationCancellation({
        path,
        errorText: request.failure()?.errorText,
      });
      if (!navigationCancelledDiscovery) requiredFirstPartyFailures.push(failure);
    });
    page.on('response', response => {
      const url = new URL(response.url());
      if (url.origin !== publicOrigin || response.status() < 400) return;
      const rendered = `${response.status()} ${response.url()}`;
      const identity = `${response.status()} ${url.pathname}`;
      httpErrors.push(rendered);
      if (
        classifyFirstPartyHttpError({ path: url.pathname, status: response.status() }) ===
        'expected-optional-negative'
      )
        optionalNegatives.add(identity);
      else requiredHttpErrors.push(rendered);
    });
    await page.goto(publicOrigin, { waitUntil: 'domcontentloaded', timeout: 30_000 });
    await page.getByTestId('footer-lamad-link').click();
    await page.waitForURL(/\/lamad\/?$/, { timeout: 30_000 });
    await page.getByTestId('home-featured-begin').click();
    await page.waitForURL(/\/lamad\/path\/elohim-protocol\/step\/0/, { timeout: 30_000 });
    const manifestoCandidates = page.locator('.markdown-content');
    const settledManifesto = await awaitCanonicalManifestoCandidate(
      async () => await manifestoCandidates.allInnerTexts(),
      Date.now() + 30_000
    );
    const manifesto = manifestoCandidates.nth(settledManifesto.index);
    await manifesto.waitFor({ state: 'visible', timeout: 30_000 });
    manifestoText = settledManifesto.text.trim();
    assert.ok(
      page.url().endsWith('/lamad/path/elohim-protocol/step/0') &&
        manifestoText.includes('Executive Summary') &&
        manifestoText.includes('Love as Technology'),
      `${phase}: step 0 is not the public manifesto`
    );
    assert.ok(manifestoText.length > 1000, `${phase}: manifesto rendered no substantive content`);
    await page.getByTestId('lamad-footer-home-link').click();
    await page.waitForURL(url => url.pathname === '/', { timeout: 30_000 });
    await page.getByTestId('landing-hero').waitFor({ state: 'visible', timeout: 30_000 });
    screenshot = await page.screenshot({ fullPage: true });
    const settleDeadline = Date.now() + 5_000;
    while (activeFirstParty.size > 0 && Date.now() < settleDeadline) await delay(50);
    for (const request of activeFirstParty.values()) {
      activeFirstPartyAtBoundary.push(request.rendered);
      if (!request.optionalDiscovery)
        requiredFirstPartyFailures.push(`${request.rendered}: still active at journey boundary`);
    }
    page.removeAllListeners('request');
    page.removeAllListeners('requestfinished');
    page.removeAllListeners('requestfailed');
    page.removeAllListeners('response');
    page.removeAllListeners('pageerror');
    page.removeAllListeners('console');
    assert.deepEqual(pageErrors, [], `${phase}: uncaught browser page errors`);
    assert.deepEqual(
      requiredFirstPartyFailures,
      [],
      `${phase}: failed required first-party requests`
    );
    assert.deepEqual(requiredHttpErrors, [], `${phase}: required first-party HTTP errors`);
    if (phase.startsWith('baseline-')) {
      state.realAppBaselineOptionalNegatives ??= new Map();
      state.realAppBaselineOptionalNegatives.set(selected.owner, optionalNegatives);
      state.realAppBaselineConsoleErrors ??= new Map();
      state.realAppBaselineConsoleErrors.set(selected.owner, new Set(consoleErrors));
    } else {
      const baselineOptionalNegatives = state.realAppBaselineOptionalNegatives?.get(selected.owner);
      const baselineConsoleErrors = state.realAppBaselineConsoleErrors?.get(selected.owner);
      assert.ok(
        baselineOptionalNegatives,
        `${phase}: no optional-negative baseline for ${selected.owner}`
      );
      assert.ok(baselineConsoleErrors, `${phase}: no console-error baseline for ${selected.owner}`);
      assert.deepEqual(
        unexpectedOptionalNegatives(baselineOptionalNegatives, optionalNegatives),
        [],
        `${phase}: introduced new optional first-party negatives`
      );
      assert.deepEqual(
        unexpectedOptionalNegatives(baselineConsoleErrors, new Set(consoleErrors)),
        [],
        `${phase}: introduced new browser console errors`
      );
    }
  } finally {
    if (!screenshot && page)
      screenshot = await page.screenshot({ fullPage: true }).catch(() => undefined);
    const receipt = {
      phase,
      owner: selected.owner,
      origin: selected.origin,
      publicOrigin,
      manifestoText,
      pageErrors,
      consoleErrors,
      allFirstPartyFailures,
      requiredFirstPartyFailures,
      httpErrors,
      requiredHttpErrors,
      optionalNegatives: [...optionalNegatives].sort((left, right) => left.localeCompare(right)),
      activeFirstPartyAtBoundary,
    };
    const runId = process.env['A2O_RUN_ID'] ?? `${process.pid}-${Date.now().toString(36)}`;
    const artifactPaths = screenshot
      ? persistRealAppPhaseArtifacts({ runId, phase, screenshot, receipt })
      : undefined;
    if (screenshot) world.attach(screenshot, 'image/png');
    world.attach(JSON.stringify({ ...receipt, artifactPaths }), 'application/json');
    await context?.close().catch(() => undefined);
    await browser?.close().catch(() => undefined);
  }
}

Given(
  'a fresh visitor completes the real landing to Lamad to manifesto and home journey at both doorway baselines',
  { timeout: 300_000 },
  async function (this: E2EWorld): Promise<void> {
    await completeRealAppJourney(this, 'baseline-alpha');
    await completeRealAppJourney(this, 'baseline-apex');
  }
);

Then(
  'a fresh visitor completes the real landing to Lamad to manifesto and home journey through the survivor',
  { timeout: 150_000 },
  async function (this: E2EWorld): Promise<void> {
    await completeRealAppJourney(this, 'survivor');
  }
);

Then(
  'a fresh visitor completes the real landing to Lamad to manifesto and home journey after recovery',
  { timeout: 150_000 },
  async function (this: E2EWorld): Promise<void> {
    await completeRealAppJourney(this, 'recovery');
  }
);

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
    ['-n', `${householdMeshDir()}/a2o.lock`, '/bin/bash', '-c', 'echo locked; read -r _'],
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

async function processState(pid: number): Promise<string> {
  const stat = await readFile(`/proc/${pid}/stat`, 'utf8');
  return stat.slice(stat.lastIndexOf(')') + 2).split(' ')[0];
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
  assert.ok(state.observedOrigin, `no captured origin for owned doorway "${owner}"`);
  state.faultedOrigin = state.observedOrigin;
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
    // Capture the ordinary public-name route before selecting the fault. The
    // transition claim is valid only if this exact leg is the one later
    // withdrawn; two snapshots that both happened to use the sibling prove no
    // route change.
    const baselineVisit = await resolvePublicName(state.authority, '/');
    // The story names the controlled transition: alpha-A is withdrawn and
    // elohim.host is the surviving entrance. Keep that name-to-owner binding
    // explicit so a fixture ordering change cannot silently reverse the proof.
    const faulted = state.authority.owners['alpha'];
    assert.ok(faulted, 'the household declares no owner to fault');
    assert.equal(
      baselineVisit.owner,
      faulted,
      `ordinary public-name selection used "${baselineVisit.owner}", but the owned operation ` +
        `withdraws "${faulted}"; reorder the declared membership fixture so the operation measures ` +
        'an actual entrance transition'
    );
    state.observedOwner = faulted;
    state.siblingOwner = owners.find(owner => owner !== faulted);
    assert.ok(state.siblingOwner, 'the advertised set names no sibling owner');
    state.observedOrigin = baselineVisit.origin;
    state.siblingOrigin = memberFor(doc, state.siblingOwner)?.origin;
    state.siblingEntryBeforeFault = memberFor(doc, state.siblingOwner);
  }
);

async function induceSelectedDoorwayFault(world: E2EWorld): Promise<void> {
  const state = getState(world);
  assert.equal(
    state.observedOwner,
    state.authority.owners['alpha'],
    'the selected fault must be alpha-A'
  );
  await acquireLease(world);
  await faultOwnedDoorway(state, state.observedOwner);
  // Wait out the declared withdraw bound rather than polling for success:
  // the assertion belongs in the Then, where it can fail.
  await delay(withdrawBound(state));
}

When(
  'the household makes one doorway report non-serving for three consecutive probes',
  { timeout: INDUCE_SHED_TIMEOUT_MS },
  async function (this: E2EWorld): Promise<void> {
    await induceSelectedDoorwayFault(this);
  }
);
When(
  'the household makes doorway {string} report non-serving for three consecutive probes',
  { timeout: INDUCE_SHED_TIMEOUT_MS },
  async function (this: E2EWorld, doorway: string): Promise<void> {
    assert.equal(doorway, ALPHA_DOORWAY_ID, 'the controlled fault target must be alpha-A');
    await induceSelectedDoorwayFault(this);
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
  "only doorway {string}'s owner records leave shared membership while doorway {string} survives",
  { timeout: 30_000 },
  async function (this: E2EWorld, withdrawn: string, survivor: string): Promise<void> {
    assert.deepEqual([withdrawn, survivor], OWNED_DOORWAY_IDS);
    const state = getState(this);
    const doc = await readMembership(state.authority);
    assert.equal(memberFor(doc, state.authority.owners['alpha']), undefined);
    assert.ok(memberFor(doc, state.authority.owners['apex']));
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
    const diagnostic = requireFixtureDoorwayUrl(fixture, 'alpha');
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

When(
  'doorway {string} reports serving for two consecutive probes',
  { timeout: DEFAULT_REJOIN_BOUND_MS + 30_000 },
  async function (this: E2EWorld, doorway: string): Promise<void> {
    assert.equal(doorway, ALPHA_DOORWAY_ID, 'the recovering doorway must be alpha-A');
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

Then(
  "doorway {string}'s owner records rejoin shared membership without duplicating doorway {string}",
  { timeout: 30_000 },
  async function (this: E2EWorld, recovered: string, sibling: string): Promise<void> {
    assert.deepEqual([recovered, sibling], OWNED_DOORWAY_IDS);
    const state = getState(this);
    const doc = await readMembership(state.authority);
    for (const owner of Object.values(state.authority.owners)) {
      assert.equal(doc.members.filter(member => member.owner === owner).length, 1);
    }
  }
);

// ---------------------------------------------------------------------------
// Scenario: A doorway that sheds leaves every public name it was advertised
// under.
//
// The household publishes several public names at once and BOTH doorways are
// members of all of them: `hc-mesh.sh start_membership_beacons` gives each leg
// one repeatable `--shared-record <name>=<owner>` per name, so a leg holds one
// membership per name under its own owner slug and writes one document per
// name. A shed is a fact about the DOORWAY — the serving probe is per leg, not
// per name — so it must reach every name that leg was advertised under. This
// scenario is what would catch a regression back to one shared name per leg:
// the leg would keep serving one set while its doorway cannot answer.
// ---------------------------------------------------------------------------

/** Every lane the scenario is about, or the named refusal for a single-name household. */
function requireLanes(state: ApexTransitionState): MembershipLaneFixture[] {
  const lanes = state.lanes ?? [];
  assert.ok(lanes.length > 0, 'the scenario must establish the household public names first');
  return lanes;
}

Given(
  'the household publishes its site under more than one public name',
  function (this: E2EWorld): void {
    const state = beginScenario(this);
    const fixture = loadHouseholdMeshFixture();
    const lanes = membershipLanes(fixture);
    assert.ok(
      lanes.length >= 2,
      `this household publishes ${lanes.length} public name(s) ` +
        `[${lanes.map(lane => lane.publicName).join(', ')}]; this scenario is about a doorway ` +
        'that belongs to SEVERAL names at once. `just mesh start` stages the second name ' +
        '(MESH_MEMBERSHIP_CANDIDATE_NAME) and `just mesh prologue` copies the lane list into ' +
        'this manifest — a manifest staged before the household carried a second name declares ' +
        'only the converged one. This is a missing-apparatus refusal, not a defect in the doorways.'
    );
    // Distinct names, or "every name" would be one name counted twice.
    const names = new Set(lanes.map(lane => lane.publicName));
    assert.equal(
      names.size,
      lanes.length,
      `the household declares the same public name more than once: ` +
        `[${lanes.map(lane => lane.publicName).join(', ')}]`
    );
    state.lanes = lanes;
  }
);

Given(
  'both owned doorways advertise eligibility under every one of those names',
  { timeout: DEFAULT_REJOIN_BOUND_MS + 30_000 },
  async function (this: E2EWorld): Promise<void> {
    const state = getState(this);
    const lanes = requireLanes(state);
    const owners = Object.values(state.authority.owners);
    assert.ok(
      owners.length >= 2,
      `the household declares only ${owners.length} membership owner(s); this scenario needs a ` +
        'doorway and a sibling under each name'
    );

    state.siblingEntryByName = new Map();
    for (const lane of lanes) {
      const doc = await untilMembership(
        lane,
        rejoinBound(state) + 10_000,
        current => owners.every(owner => memberFor(current, owner) !== undefined),
        `owners [${owners.join(', ')}] did not all advertise eligibility under "${lane.publicName}"`
      );
      for (const owner of owners) {
        assert.equal(
          doc.members.filter(member => member.owner === owner).length,
          1,
          `owner "${owner}" holds more than one entry under "${lane.publicName}"`
        );
      }
    }

    // The leg that will be faulted, and the sibling that must survive under
    // every name. Bound to the fixture doorway ids the story names, so a
    // fixture reordering cannot silently reverse which doorway is measured.
    const faulted = state.authority.owners['alpha'];
    const sibling = state.authority.owners['apex'];
    assert.ok(faulted && sibling, 'the household declares no alpha/apex owner pair to measure');
    state.observedOwner = faulted;
    state.siblingOwner = sibling;

    // Capture each name's sibling entry WHOLE — freshness stamp included — so
    // the later assertion can see a leg that rewrote a record it does not own.
    for (const lane of lanes) {
      const doc = await readMembership(lane);
      const entry = memberFor(doc, sibling);
      assert.ok(entry, `the sibling "${sibling}" is not advertised under "${lane.publicName}"`);
      state.siblingEntryByName.set(lane.publicName, entry);
    }

    // The fault control needs the origin this doorway contributes; take it from
    // the converged name's set, which is the one the ordinary visit resolves.
    const converged = lanes[0];
    const baselineVisit = await resolvePublicName(converged, '/');
    assert.equal(
      baselineVisit.owner,
      faulted,
      `ordinary selection under "${converged.publicName}" used "${baselineVisit.owner}", but the ` +
        `owned operation withdraws "${faulted}"; reorder the declared membership fixture so the ` +
        'operation measures an actual entrance transition'
    );
    state.observedOrigin = baselineVisit.origin;
    state.siblingOrigin = state.siblingEntryByName.get(converged.publicName)?.origin;
  }
);

Then(
  'doorway {string} is advertised under none of those public names',
  { timeout: 60_000 },
  async function (this: E2EWorld, doorway: string): Promise<void> {
    assert.equal(doorway, ALPHA_DOORWAY_ID, 'the withdrawn doorway must be alpha-A');
    const state = getState(this);
    const withdrawn = state.authority.owners['alpha'];
    const stillAdvertised: string[] = [];
    for (const lane of requireLanes(state)) {
      const doc = await readMembership(lane);
      if (memberFor(doc, withdrawn)) {
        stillAdvertised.push(`${lane.publicName} -> [${advertised(doc).join(', ')}]`);
      }
    }
    assert.deepEqual(
      stillAdvertised,
      [],
      `after ${withdrawBound(state)}ms of non-serving probes, "${withdrawn}" is still advertised ` +
        `under ${stillAdvertised.length} of its public name(s): ${stillAdvertised.join('; ')}. ` +
        'A shed is a fact about the doorway, so it must reach every name that doorway holds.'
    );
  }
);

Then(
  'doorway {string} keeps its own unchanged entry under every one of them',
  { timeout: 60_000 },
  async function (this: E2EWorld, doorway: string): Promise<void> {
    assert.equal(doorway, APEX_DOORWAY_ID, 'the surviving doorway must be elohim.host');
    const state = getState(this);
    const sibling = state.authority.owners['apex'];
    const before = state.siblingEntryByName;
    assert.ok(before, 'the scenario captured no per-name sibling entries before the fault');
    for (const lane of requireLanes(state)) {
      const doc = await readMembership(lane);
      // Compared WHOLE, freshness stamp included: a leg that rewrote a record
      // it does not own — even to the same value — is visible only here.
      assert.deepEqual(
        memberFor(doc, sibling),
        before.get(lane.publicName),
        `the sibling "${sibling}" entry under "${lane.publicName}" changed across the ` +
          'withdrawal; under any name a doorway writes only its own entry'
      );
    }
  }
);

Then(
  'a new visitor using any one of those public names still receives the landing page',
  { timeout: FIRST_VISIT_TIMEOUT_MS },
  async function (this: E2EWorld): Promise<void> {
    const state = getState(this);
    const sibling = state.authority.owners['apex'];
    for (const lane of requireLanes(state)) {
      const visit = await resolvePublicName(lane, '/');
      assert.equal(
        visit.status,
        200,
        `"${lane.publicName}" answered ${visit.status} while one of its doorways sheds`
      );
      assert.ok(
        visit.text.includes('app-root'),
        `"${lane.publicName}" served a page with no app-root mount from ${visit.origin}`
      );
      assert.equal(
        visit.owner,
        sibling,
        `"${lane.publicName}" was served by "${visit.owner}"; while "${state.observedOwner}" ` +
          'sheds, every name must resolve to the survivor'
      );
    }
  }
);

Then(
  'doorway {string} is advertised again under every one of those public names',
  { timeout: DEFAULT_REJOIN_BOUND_MS + 60_000 },
  async function (this: E2EWorld, doorway: string): Promise<void> {
    assert.equal(doorway, ALPHA_DOORWAY_ID, 'the recovering doorway must be alpha-A');
    const state = getState(this);
    const recovered = state.authority.owners['alpha'];
    for (const lane of requireLanes(state)) {
      await untilMembership(
        lane,
        rejoinBound(state),
        current => memberFor(current, recovered) !== undefined,
        `"${recovered}" did not rejoin "${lane.publicName}"`
      );
    }
  }
);

Then(
  'no public name advertises the same doorway twice',
  { timeout: 60_000 },
  async function (this: E2EWorld): Promise<void> {
    const state = getState(this);
    const owners = Object.values(state.authority.owners);
    for (const lane of requireLanes(state)) {
      const doc = await readMembership(lane);
      for (const owner of owners) {
        assert.equal(
          doc.members.filter(member => member.owner === owner).length,
          1,
          `owner "${owner}" is advertised ${doc.members.filter(m => m.owner === owner).length} ` +
            `times under "${lane.publicName}": [${advertised(doc).join(', ')}]`
        );
      }
      assert.equal(
        doc.members.length,
        owners.length,
        `the set advertised for "${lane.publicName}" is [${advertised(doc).join(', ')}], not ` +
          'exactly one entry per declared owner'
      );
    }
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
    const browser = await visitInBrowser(`${state.siblingOrigin}/`);
    assert.deepEqual(browser.pageErrors, [], 'sibling browser page errors');
    // Open design (legacy-web-projection): this capture has no notion of a declared external
    // origin, so any outside content a page loads on its own fails this step when that origin's
    // telemetry aborts. Do not allowlist it here — the declaration belongs to the view and the
    // protocol, not the harness. The landing hero answers it for now with a click-to-load facade.
    // genesis/data/timeline/backlog/legacy-web-content-projected-inward.md
    assert.deepEqual(browser.failedRequests, [], 'sibling browser request failures');
    assert.deepEqual(browser.httpErrors, [], 'sibling browser HTTP errors');
    assert.ok(browser.rootPresent, 'sibling browser saw no app-root');
    assert.ok(browser.bootstrapReady, 'sibling browser did not bootstrap its entry script');
    assert.ok(browser.rootText.trim().length > 0, 'sibling browser booted an empty app root');
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

Given(
  "the doorway pair records authority A as the canonical author's exact action hash, blob address, entry script, and version",
  { timeout: 2 * EXACT_AUTHORITY_TIMEOUT_MS + 10_000 },
  async function (this: E2EWorld): Promise<void> {
    const state = getState(this);
    state.authorityA = chaosAuthorReceipt(this);
    this.attach(JSON.stringify(state.authorityA), 'application/json');
    const members = (await readMembership(state.authority)).members;
    const deadlineAt = Date.parse(state.authorityA.authoredAt) + AUTHORITY_PUBLICATION_BOUND_MS;
    const convergence = await waitForStrictAuthorityConvergence(
      members.map(member => member.origin),
      deadlineAt,
      async (origin, remainingMs) =>
        await lightweightAuthorityMismatch(origin, state.authorityA!.authority, remainingMs)
    );
    assert.ok(
      convergence.converged,
      `authority A did not converge to the author's exact receipt within the shared 75s publication deadline: ${JSON.stringify(convergence.lastMismatches)}`
    );
    for (const member of members) {
      await assertExactPublishedSurface(member.origin, state.authorityA.authority, deadlineAt);
    }
  }
);

/**
 * THE STATION between "the author publishes" and the exact-match reading.
 *
 * The finish-line step below reads the surviving doorway ONCE, and nothing
 * honest can satisfy that read at zero delay: the author's publication has to
 * cross the network and be checked by the receiving doorway before its own
 * answer can move. So the waiting belongs to its own station — this one — and
 * the exact reading stays a single uninterrupted pass immediately after it.
 *
 * The bound is the Gherkin's, not the harness's: the step reads the number the
 * story states. This ceiling only sizes the step's own budget, and refuses a
 * story bound it could not honour — a 120s bound inside a 70s step would die as
 * cucumber's opaque "function timed out", losing every diagnostic below.
 */
const SIBLING_ADOPTION_BOUND_CEILING_MS = 10_000;
/** A one-second poll cannot resolve a ~1s adoption; this one can. */
const SIBLING_ADOPTION_POLL_MS = 250;

Then(
  'within {int} seconds doorway {string} answers a visitor with the version the author has just published, and nobody restarted or re-staged that doorway to get there',
  { timeout: SIBLING_ADOPTION_BOUND_CEILING_MS + 2 * RAW_FETCH_TIMEOUT_MS },
  async function (this: E2EWorld, seconds: number, survivingDoorway: string): Promise<void> {
    const state = getState(this);
    const boundMs = seconds * 1000;
    assert.ok(
      boundMs <= SIBLING_ADOPTION_BOUND_CEILING_MS,
      `the story states a ${seconds}s adoption bound, but this step's budget is sized for ` +
        `${SIBLING_ADOPTION_BOUND_CEILING_MS / 1000}s — widen SIBLING_ADOPTION_BOUND_CEILING_MS ` +
        'deliberately, together with the comment in the scenario that defends the bound'
    );
    assert.equal(survivingDoorway, APEX_DOORWAY_ID, 'the governed survivor must be elohim.host');
    assert.ok(state.authorityA, 'authority A was not recorded before withdrawal');
    assert.ok(
      state.siblingOrigin && state.siblingOwner,
      'the withdrawal named no surviving doorway'
    );
    // The author's own receipt, read before any serving-path observation — the
    // same receipt the exact-match step below reads, so the station and the
    // finish line can never disagree about which version is "the version".
    const published = chaosAuthorReceipt(this);
    assert.notEqual(
      published.authority.actionHash,
      state.authorityA.authority.actionHash,
      'the author published nothing new: the governed action still names the version recorded ' +
        'before the withdrawal, so there is no adoption to measure'
    );
    const publishedAt = Date.parse(published.authoredAt);
    const deadlineAt = publishedAt + boundMs;
    // Nobody restarts or re-stages this doorway to make it answer: the process
    // that served before the publication is the one that answers after it, and
    // no step hands it the new bytes. A recycled PID or a swapped executable
    // would be a different doorway wearing the same name.
    const incarnationBefore = await ownedDoorwayProcess(state.siblingOwner);
    const convergence = await waitForStrictAuthorityConvergence(
      [state.siblingOrigin],
      deadlineAt,
      async (origin, remainingMs) =>
        await lightweightAuthorityMismatch(origin, published.authority, remainingMs),
      { intervalMs: SIBLING_ADOPTION_POLL_MS }
    );
    const elapsedMs = Date.now() - publishedAt;
    const observed = Object.keys(convergence.lastMismatches).length
      ? JSON.stringify(convergence.lastMismatches)
      : 'nothing — the bound elapsed before one complete reading';
    // The measurement, not a pass/fail echo: how long a visitor's doorway took
    // to answer with a version published elsewhere. Carried-record adoption is
    // judged by this number (genesis/data/timeline/backlog/
    // head-authority-carried-with-content-sync-unit.md).
    this.attach(
      JSON.stringify({
        schema: 'doorway-sibling-adoption-measure/v1',
        concern: 'doorway-failover',
        doorway: survivingDoorway,
        owner: state.siblingOwner,
        origin: state.siblingOrigin,
        publicName: state.authority.publicName,
        probe: "the surviving doorway's own governed head, blob and addressed version",
        publishedAt: published.authoredAt,
        expectedGovernedAction: published.authority.actionHash,
        previousGovernedAction: state.authorityA.authority.actionHash,
        boundMs,
        pollIntervalMs: SIBLING_ADOPTION_POLL_MS,
        adopted: convergence.converged,
        timeToAdoptMs: convergence.converged ? elapsedMs : null,
        elapsedMs,
        lastMismatches: convergence.lastMismatches,
      }),
      'application/json'
    );
    assert.ok(
      convergence.converged,
      `doorway "${survivingDoorway}" (owner "${state.siblingOwner}" at ${state.siblingOrigin}) did ` +
        `not answer with the author's published version within ${seconds}s: expected governed ` +
        `action ${published.authority.actionHash}, observed ${observed}, ${elapsedMs}ms after the ` +
        `author published (bound ${boundMs}ms)`
    );
    assert.deepEqual(
      await ownedDoorwayProcess(state.siblingOwner),
      incarnationBefore,
      `doorway "${survivingDoorway}" did not carry the new version across in the process that was ` +
        'already serving: its PID, start ticks or executable changed while it adopted'
    );
  }
);

Then(
  "doorway {string} serves authority B's exact head, blob, addressed version, HTML entry, and browser bootstrap",
  { timeout: EXACT_AUTHORITY_TIMEOUT_MS + 10_000 },
  async function (this: E2EWorld, survivingDoorway: string): Promise<void> {
    const state = getState(this);
    assert.ok(state.authorityA, 'authority A was not recorded before withdrawal');
    const authorityB = chaosAuthorReceipt(this);
    assert.notEqual(
      authorityB.authority.actionHash,
      state.authorityA.authority.actionHash,
      'authority action did not move A→B'
    );
    assert.notEqual(
      authorityB.authority.blobHash,
      state.authorityA.authority.blobHash,
      'authority blob did not move A→B'
    );
    state.authorityB = authorityB;
    this.attach(JSON.stringify(authorityB), 'application/json');
    assert.ok(state.siblingOrigin, 'the withdrawal named no surviving doorway');
    assert.equal(survivingDoorway, 'elohim.host', 'the governed survivor must be elohim.host');
    assert.equal(
      state.siblingOwner,
      state.authority.owners['apex'],
      'the surviving membership owner must be the elohim.host/apex leg'
    );
    const ordinaryVisit = await resolvePublicName(state.authority, authorityB.authority.mountPath);
    assert.equal(
      ordinaryVisit.owner,
      state.siblingOwner,
      `ordinary public-name selection used "${ordinaryVisit.owner}" during the fault; the surviving ` +
        `doorway is "${state.siblingOwner}"`
    );
    await assertExactPublishedAuthority(ordinaryVisit.origin, authorityB.authority);
  }
);

Then(
  "recovered doorways {string} and {string} serve authority B's same exact head, blob, addressed version, HTML entry, and browser bootstrap",
  { timeout: 2 * EXACT_AUTHORITY_TIMEOUT_MS + 10_000 },
  async function (this: E2EWorld, recoveredA: string, recoveredB: string): Promise<void> {
    const state = getState(this);
    assert.deepEqual(
      [recoveredA, recoveredB],
      OWNED_DOORWAY_IDS,
      'the story must recover the named withdrawn and surviving entrances'
    );
    assert.ok(state.authorityB, 'authority B was not observed during the withdrawal');
    const doc = await readMembership(state.authority);
    assert.equal(
      doc.members.length,
      Object.values(state.authority.owners).length,
      'recovery did not restore exactly one membership entry per doorway'
    );
    for (const member of doc.members) {
      await assertExactPublishedAuthority(member.origin, state.authorityB.authority);
    }
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
    if (!state?.pid) return;
    if (state.paused) await signalOwnedDoorway(state, 'SIGCONT');
    assert.ok(state.faultedOrigin, 'fault handle exists without its immutable origin');
    await awaitOwnedProcessRecovery(
      {
        identityMatches: async () =>
          (await processStartTicks(state.pid as number)) === state.ticks &&
          (await readlink(`/proc/${state.pid as number}/exe`)) === state.executable,
        processRunning: async () => (await processState(state.pid as number)) !== 'T',
        faultedPathHealthy: async () =>
          (await getRaw(`${state.faultedOrigin}/health`, { timeoutMs: 2_000 })).status === 200,
      },
      15_000,
      250
    );
  } finally {
    leases.get(this)?.stdin.end();
    leases.delete(this);
    states.delete(this);
  }
});
