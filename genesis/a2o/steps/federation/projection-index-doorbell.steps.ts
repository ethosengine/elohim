/**
 * Step glue for features/federation/projection-index-doorbell.feature
 * (story 4.2 slice 1, @concern:doorway-failover).
 *
 * THE MECHANISM UNDER TEST. `doorway/doorway-service/src/services/federation_doorbell.rs`:
 * the SENDER (`spawn_doorbell_ringer`) wakes on `EprRouter::changed()`, debounces 250ms, and
 * POSTs `{doorwayId, digest}` to every registered peer (`POST /api/v1/federation/doorbell`,
 * `routes::coherence::handle_doorbell`). The RECEIVER (`receive_doorbell`) resolves the
 * sender through `peer_cache`, checks the digest against the held one (a no-op if unchanged),
 * and spawns a pull of the sender's own `GET /api/v1/federation/coherence` — installing that
 * ONE holder's rows via `NameRouteTable::replace_holder`. The 60s discovery poll
 * (`services::federation::spawn_peer_discovery_task`) remains the backstop; scenario 3 proves
 * it, deliberately with the doorbell silenced via the household-fixture-only
 * `PUT /admin/dev/federation-deaf`.
 *
 * REUSE, NOT DUPLICATION. Staging an archive (`stockAppArchive`, a hand-rolled ZIP writer),
 * commitment lifecycle (`stageRoot`/`lapseContract`), local-mount confirmation
 * (`waitForLocalMount`), doorway-id resolution (`resolvedDoorwayId`), and log-file/line
 * plumbing (`doorwayLogPath`/`logLength`/`logSince`, `AskCapture`/`MSG_RELAYED`) all live in
 * `name-routing.steps.ts`, exported there for exactly this reuse — this file duplicates NONE
 * of it. What genuinely differs here: the "only" staging Given in `name-routing.steps.ts`
 * WAITS for the sibling's registry to converge before returning (`waitForRegistryToKnowHolder`,
 * up to 150s), which would make every "within N seconds" assertion in THIS feature vacuously
 * true regardless of whether the doorbell fired at all. This file's own staging Given
 * ("... without waiting for propagation") stages and confirms the LOCAL mount only, leaving
 * propagation entirely for this feature's own timing assertions to measure.
 *
 * WHY SCENARIO 3 EXISTS AS A CONTROL. Without a way to silence the doorbell, scenario 1's
 * "within 10 seconds" claim would be indistinguishable from a lucky 60s discovery-poll tick
 * landing inside the same 10s window — roughly 1 time in 6. Scenario 3 proves the negative:
 * with the doorbell silenced from before garden is ever staged, propagation does NOT happen
 * within 10 seconds, only on the poll's own schedule.
 */

import { strict as assert } from 'node:assert';
import { setTimeout as delay } from 'node:timers/promises';

import { Given, Then, When } from '@cucumber/cucumber';

import { E2EWorld } from '../../src/framework/world.js';
import {
  AskCapture,
  LogLineFields,
  MESH_LETTER,
  MSG_ALL_FAILED,
  MSG_RELAYED,
  NameRoutingState,
  RawResponse,
  beginScenario,
  doorwayLogPath,
  getState,
  lapseContract,
  logLength,
  logSince,
  originsEqual,
  rawGet,
  requireAsk,
  resolvedDoorwayId,
  stageRoot,
  waitForLocalMount,
} from './name-routing.steps.js';

/** Poll cadence for the timing assertions below — fine-grained relative to the 10s claims
 * this feature makes, without hammering the doorway. */
const POLL_INTERVAL_MS = 500;
/** The federation discovery poll's own interval (`spawn_peer_discovery_task`,
 * `doorway-service/src/main.rs`) — the backstop scenario 3 proves. */
const DISCOVERY_CYCLE_MS = 60_000;

// =============================================================================
// Small local plumbing — everything this file could not reuse verbatim
// =============================================================================

/** The one OTHER doorway this scenario has declared besides `id` — this feature's
 * Background always declares exactly alpha and beta, so this resolves to whichever of
 * those two is not `id`. Self-contained (unlike `name-routing.steps.ts`'s private
 * `otherFixtureId`, which assumes the alpha/beta pairing directly) so this file needs no
 * further export from there. */
function theOtherDeclaredDoorway(world: E2EWorld, id: string): string {
  for (const candidate of Object.keys(MESH_LETTER)) {
    if (candidate === id) continue;
    try {
      world.getDoorway(candidate);
      return candidate;
    } catch {
      // Not declared this scenario.
    }
  }
  throw new Error(`no sibling doorway declared besides "${id}"`);
}

/** This feature's own per-scenario bookkeeping, kept separate from
 * `NameRoutingState` (defined in `name-routing.steps.ts`) rather than extending it —
 * doorbell-log-offset tracking is specific to this feature and has no bearing on the
 * routing/relay scenarios that type already serves. */
interface DoorbellState {
  /** The sibling's log length captured BEFORE staging — the doorbell-pull check and the
   * propagation-timing checks both read strictly after this point, so neither can be
   * satisfied by a line written before this scenario's own change happened. */
  siblingLogOffsetAtStage: number;
}
const doorbellStates = new WeakMap<E2EWorld, DoorbellState>();

function requireDoorbellState(world: E2EWorld): DoorbellState {
  const state = doorbellStates.get(world);
  assert.ok(
    state,
    'no doorbell staging offset captured — this scenario\'s staging Given must run first'
  );
  return state;
}

/** `GET {doorwayUrl}/api/v1/federation/coherence`'s `digest` field
 * (`routes::coherence::CoherenceManifest`) — the exact value a doorbell names. */
async function fetchCoherenceDigest(doorwayUrl: string): Promise<string> {
  const res = await rawGet(`${doorwayUrl}/api/v1/federation/coherence`);
  assert.equal(
    res.status,
    200,
    `GET ${doorwayUrl}/api/v1/federation/coherence failed: HTTP ${res.status} ${res.text.slice(0, 200)}`
  );
  const parsed = JSON.parse(res.text) as { digest?: string };
  assert.ok(parsed.digest, `${doorwayUrl}/api/v1/federation/coherence answered no digest`);
  return parsed.digest;
}

/** Poll `fromUrl + path` until it resolves to `holderUrl` (HTTP 200, `x-elohim-served-by`
 * naming that origin) or `budgetMs` elapses. Returns the confirming response, or `undefined`
 * on timeout — the caller decides how to word the failure. */
async function pollUntilResolvesTo(
  fromUrl: string,
  path: string,
  holderUrl: string,
  budgetMs: number
): Promise<RawResponse | undefined> {
  const deadline = Date.now() + budgetMs;
  for (;;) {
    try {
      const res = await rawGet(`${fromUrl}${path}`);
      const servedBy = res.headers['x-elohim-served-by'];
      if (res.status === 200 && servedBy && originsEqual(servedBy, holderUrl)) {
        return res;
      }
    } catch {
      // Transient — keep polling until the budget is spent.
    }
    if (Date.now() >= deadline) return undefined;
    await delay(POLL_INTERVAL_MS);
  }
}

/** Poll `url` until it answers `expectedStatus`, or `budgetMs` elapses. */
async function pollUntilStatus(
  url: string,
  expectedStatus: number,
  budgetMs: number
): Promise<RawResponse | undefined> {
  const deadline = Date.now() + budgetMs;
  for (;;) {
    try {
      const res = await rawGet(url);
      if (res.status === expectedStatus) return res;
    } catch {
      // Transient — keep polling until the budget is spent.
    }
    if (Date.now() >= deadline) return undefined;
    await delay(POLL_INTERVAL_MS);
  }
}

// =============================================================================
// Given: the doorbell-deaf fixture — REGISTERED IN name-routing.steps.ts, NOT here.
// =============================================================================
//
// `Given('doorway {string} is deaf to doorbells for {int} seconds', ...)` is defined in
// `name-routing.steps.ts` (beside `putShedOverride`, sharing its missing-route/forbidden
// degrade shape via `shedRouteMissing`/`shedRouteForbidden`/`explainFixtureForbidden`) even
// though it is USED by scenario 3 below — step registration is global across the whole a2o
// step-file set, and name-routing.steps.ts's scenario 4 also reuses this exact text. A
// second `Given` here with the same pattern would be an AMBIGUOUS step definition, not a
// harmless duplicate — Cucumber refuses to run a scenario whose step matches two
// registrations. Do not re-add it here.

// =============================================================================
// Given: staging that does NOT wait for the sibling to converge
// =============================================================================

Given(
  'the household stages the root {string} as hosted by doorway {string} only, without waiting for propagation',
  { timeout: 30_000 },
  async function (this: E2EWorld, root: string, holderId: string): Promise<void> {
    const state: NameRoutingState = beginScenario(this, root);
    const holder = this.getDoorway(holderId);
    const holderDoorwayId = await resolvedDoorwayId(this, holderId, holder.url);

    const siblingId = theOtherDeclaredDoorway(this, holderId);
    const siblingLog = await doorwayLogPath(MESH_LETTER[siblingId], `doorway ${siblingId}`);
    const siblingLogOffsetAtStage = await logLength(siblingLog);

    const staged = await stageRoot(holder.url, holderDoorwayId, state.mount, root);
    state.staged.push(staged);
    // Confirm the HOLDER genuinely, locally serves it — the same sanity the "only" step
    // runs — but deliberately go no further: no wait for the sibling's own registry.
    await waitForLocalMount(holder.url, holderId, state.path, root);

    doorbellStates.set(this, { siblingLogOffsetAtStage });
  }
);

// =============================================================================
// Then: propagation timing — the doorbell's whole claim
// =============================================================================

Then(
  'within 10 seconds doorway {string} resolves {string} to doorway {string} in its local registry',
  { timeout: 15_000 },
  async function (this: E2EWorld, fromId: string, root: string, holderId: string): Promise<void> {
    const state = getState(this);
    assert.equal(state.root, root, `scenario staged root "${state.root}", not "${root}"`);
    const from = this.getDoorway(fromId);
    const holder = this.getDoorway(holderId);
    const resolved = await pollUntilResolvesTo(from.url, state.path, holder.url, 10_000);
    assert.ok(
      resolved,
      `doorway "${fromId}" did not resolve "${root}" to doorway "${holderId}" within 10 ` +
        'seconds — the doorbell (services::federation_doorbell) may not have rung or been ' +
        'received'
    );
  }
);

Then(
  'within 10 seconds doorway {string} no longer resolves {string} to any holder',
  { timeout: 15_000 },
  async function (this: E2EWorld, fromId: string, root: string): Promise<void> {
    const state = getState(this);
    assert.equal(state.root, root, `scenario staged root "${state.root}", not "${root}"`);
    const from = this.getDoorway(fromId);
    // The extension-explicit entry document, not the extension-less mount root — the
    // extension-less path can answer 200 with an UNRELATED SPA fallback page even when no
    // holder exists (see name-routing.steps.ts's file-header "spaFallback" risk note),
    // which would make this poll wait for a 404 that never honestly needs to happen.
    const indexPath = `${state.mount}/index.html`;
    const dropped = await pollUntilStatus(`${from.url}${indexPath}`, 404, 10_000);
    assert.ok(
      dropped,
      `doorway "${fromId}" still answered non-404 for "${root}" (${indexPath}) after 10 ` +
        'seconds of the withdrawal — expected it to stop resolving to any holder'
    );
  }
);

Then(
  'after {int} seconds doorway {string} still does not resolve {string} to any holder',
  { timeout: 20_000 },
  async function (this: E2EWorld, seconds: number, fromId: string, root: string): Promise<void> {
    const state = getState(this);
    assert.equal(state.root, root, `scenario staged root "${state.root}", not "${root}"`);
    await delay(seconds * 1000);
    const from = this.getDoorway(fromId);
    const res = await rawGet(`${from.url}${state.path}`);
    const servedBy = res.headers['x-elohim-served-by'];
    const resolvesToAHolder = res.status === 200 && !!servedBy;
    assert.ok(
      !resolvesToAHolder,
      `doorway "${fromId}" already resolves "${root}" (served-by ${servedBy}) after only ` +
        `${seconds}s while deaf to doorbells — the deaf fixture may not be honored, or this ` +
        'scenario staged too late relative to it'
    );
  }
);

Then(
  'doorway {string} resolves {string} to doorway {string} in its local registry within one discovery cycle plus 20 seconds',
  { timeout: DISCOVERY_CYCLE_MS + 30_000 },
  async function (this: E2EWorld, fromId: string, root: string, holderId: string): Promise<void> {
    const state = getState(this);
    assert.equal(state.root, root, `scenario staged root "${state.root}", not "${root}"`);
    const from = this.getDoorway(fromId);
    const holder = this.getDoorway(holderId);
    const budgetMs = DISCOVERY_CYCLE_MS + 20_000;
    const resolved = await pollUntilResolvesTo(from.url, state.path, holder.url, budgetMs);
    assert.ok(
      resolved,
      `doorway "${fromId}" never resolved "${root}" to doorway "${holderId}" within one ` +
        `discovery cycle plus 20 seconds (${budgetMs}ms) — the backstop poll ` +
        '(services::federation::spawn_peer_discovery_task) may not be running'
    );
  }
);

// =============================================================================
// Then: the doorbell was the mechanism, not a coincidence
// =============================================================================

/** True iff `text` (a doorway's log, sliced to the window since staging) contains a
 * `"doorbell pull"` line naming `senderDoorwayId` — and, when `expectedDigest` is given,
 * naming that digest too. Shared by both the digest-naming check (scenario 1, proving the
 * pull named the SPECIFIC change staging just made) and the looser check (scenario 2, where
 * "the digest that holds garden" makes no sense post-withdrawal — the pull that matters
 * there is simply "a pull from alpha happened at all"). */
function logNamesDoorbellPull(
  text: string,
  senderDoorwayId: string,
  expectedDigest?: string
): boolean {
  return text
    .split('\n')
    .filter(line => line.trim().length > 0)
    .some(line => {
      try {
        const parsed = JSON.parse(line) as { fields?: LogLineFields };
        const fields = parsed.fields;
        if (fields?.message !== 'doorbell pull' || fields['doorway_id'] !== senderDoorwayId) {
          return false;
        }
        return expectedDigest === undefined || fields['digest'] === expectedDigest;
      } catch {
        return false;
      }
    });
}

Then(
  'doorway {string} logged a doorbell pull from doorway {string} naming the digest that holds {string}',
  { timeout: 12_000 },
  async function (this: E2EWorld, receiverId: string, senderId: string, root: string): Promise<void> {
    const state = getState(this);
    assert.equal(state.root, root, `scenario staged root "${state.root}", not "${root}"`);
    const db = requireDoorbellState(this);
    const sender = this.getDoorway(senderId);
    const receiverLog = await doorwayLogPath(MESH_LETTER[receiverId], `doorway ${receiverId}`);

    const expectedDigest = await fetchCoherenceDigest(sender.url);
    const senderDoorwayId = await resolvedDoorwayId(this, senderId, sender.url);

    const text = await logSince(receiverLog, db.siblingLogOffsetAtStage);
    const found = logNamesDoorbellPull(text, senderDoorwayId, expectedDigest);
    assert.ok(
      found,
      `doorway "${receiverId}"'s log never recorded a "doorbell pull" line naming ` +
        `doorway_id="${senderDoorwayId}" digest="${expectedDigest}" — the doorbell ` +
        '(services::federation_doorbell::receive_doorbell) may not have fired'
    );
  }
);

/** The withdrawal-scenario sibling of the check above: "the digest that holds garden" is
 * meaningless once garden has been withdrawn, so this only proves a doorbell pull from the
 * named sender happened at all in the window since staging — evidence the SAME doorbell
 * mechanism carried the withdrawal, not a second, unverified one. */
Then(
  'doorway {string} logged a doorbell pull from doorway {string}',
  { timeout: 12_000 },
  async function (this: E2EWorld, receiverId: string, senderId: string): Promise<void> {
    const db = requireDoorbellState(this);
    const sender = this.getDoorway(senderId);
    const receiverLog = await doorwayLogPath(MESH_LETTER[receiverId], `doorway ${receiverId}`);
    const senderDoorwayId = await resolvedDoorwayId(this, senderId, sender.url);

    const text = await logSince(receiverLog, db.siblingLogOffsetAtStage);
    const found = logNamesDoorbellPull(text, senderDoorwayId);
    assert.ok(
      found,
      `doorway "${receiverId}"'s log never recorded a "doorbell pull" line naming ` +
        `doorway_id="${senderDoorwayId}" — the doorbell ` +
        '(services::federation_doorbell::receive_doorbell) may not have fired'
    );
  }
);

// =============================================================================
// When / Then: a simple withdrawal, and proving no relay was even attempted
// =============================================================================

When(
  'the household withdraws the contract for {string} on doorway {string}',
  { timeout: 15_000 },
  async function (this: E2EWorld, root: string, holderId: string): Promise<void> {
    const state = getState(this);
    assert.equal(state.root, root, `scenario staged root "${state.root}", not "${root}"`);
    const holder = this.getDoorway(holderId);
    const staged = state.staged.find(contract => originsEqual(contract.doorwayUrl, holder.url));
    assert.ok(staged, `no staged contract for "${root}" on doorway "${holderId}"`);

    // Capture the SIBLING's log offset IMMEDIATELY BEFORE the withdrawal — not the
    // staging offset `doorbellStates` may or may not already hold (this scenario reuses
    // name-routing.steps.ts's own convergence-waiting "only" staging step, which never
    // touches `doorbellStates` at all). The doorbell this withdrawal is expected to ring
    // must be found strictly after THIS point, or an earlier, unrelated pull from staging
    // could satisfy the check without the withdrawal's own doorbell ever having fired.
    const siblingId = theOtherDeclaredDoorway(this, holderId);
    const siblingLog = await doorwayLogPath(MESH_LETTER[siblingId], `doorway ${siblingId}`);
    const siblingLogOffsetAtStage = await logLength(siblingLog);
    doorbellStates.set(this, { siblingLogOffsetAtStage });

    await lapseContract(staged);
  }
);

Then(
  'doorway {string} attempted no relay for that request',
  function (this: E2EWorld, doorwayId: string): void {
    const state = getState(this);
    const ask: AskCapture = requireAsk(state);
    assert.equal(ask.askedId, doorwayId);
    const attempts = ask.askedNewLines.filter(
      f => f.message === MSG_RELAYED || f.message === MSG_ALL_FAILED
    );
    assert.equal(
      attempts.length,
      0,
      `doorway "${doorwayId}" logged ${attempts.length} relay-decision line(s) for ` +
        `${state.path} for this ask, expected none — it already knew no holder existed for ` +
        'the withdrawn root and should never have attempted a relay'
    );
  }
);
