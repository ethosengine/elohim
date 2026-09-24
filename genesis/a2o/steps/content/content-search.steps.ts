/**
 * Content search — step definitions.
 *
 * Drives genesis/a2o/features/content/content-search.feature
 * (@concern:recall-reaches-authority): the household's own storage peer answers
 * `GET /db/content/search` with the recipe it ranked by (name + CID), whether
 * its ranking is known at all, the state of the fold it read, and candidates
 * that have each passed the per-reader reach gate BEFORE any snippet was cut.
 *
 * Surfaces this file reads (the thing the story claims, not a proxy):
 *   - `GET /db/content/search?q&limit` on the household's primary storage peer,
 *     read DIRECTLY (own-node route, not proxied through the doorway), with the
 *     asker's own `X-Agent-Cid` — the same credential-shaped direct-storage read
 *     `steps/delivery/acquisition-pins.steps.ts` and `steps/mesh/death-witness.steps.ts`
 *     already use. The header is the human's `agentPubKey`, set at login.
 *   - the answer's own shape, `ContentSearchView`
 *     (elohim/sdk/schemas/v1/views/content-search-view.schema.json): `rankingKnown`,
 *     `recipe{name,cid,…}`, the `fold` / `foldLag` answer envelopes
 *     (`present | absent | unreachable`), `candidates[]`, `facets`, `omissions`,
 *     `unresolved`, `totalCount`.
 *
 * Waiting, and why it is the shape it is: the fold is incremental and runs on
 * notify (plus a periodic sweep), but the content row itself only becomes
 * externally visible after storage's provenance-publish drain — measured at
 * 8–19 s on the household mesh (see PROVENANCE_VISIBILITY_BUDGET in
 * src/framework/assertions/content-sync.ts). So the search step polls, and its
 * budget is sized by that drain window rather than by the fold alone; a budget
 * that expires inside the drain measures the drain, not the index.
 *
 * The poll waits for PRESENCE only when the asker is the person who wrote the
 * row in this scenario. A non-holder's search must NOT be waited on for
 * presence — the whole point of that scenario is that the row never arrives —
 * so it takes one answer, after the holder's own control search has already
 * established that the index caught up.
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { getRaw } from '../../src/framework/dataplane/surfaces.js';
import {
  loadHouseholdMeshFixture,
  requireFixturePrimaryStorageUrl,
} from '../../src/framework/fixtures/household-mesh.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Constants and wire shapes
// ---------------------------------------------------------------------------

const PENDING = 'pending';

const SEARCH_PATH = '/db/content/search';

/** Page size asked for. The route clamps to 100; 20 is its own default. */
const SEARCH_LIMIT = 20;

/**
 * How long a search may be polled for the row this scenario just wrote.
 * Sized by the provenance-drain window (8–19 s measured), not by the fold's own
 * catch-up, which is notify-driven and far quicker.
 */
const SEARCH_CATCH_UP_BUDGET_MS = 45_000;

/** Delay between polls inside that budget. */
const SEARCH_POLL_INTERVAL_MS = 1_000;

/** Per-request bound on the direct-storage read. */
const SEARCH_REQUEST_TIMEOUT_MS = 15_000;

/**
 * The env var a fold-disabled peer is exported under. Set only by a household
 * that boots one (plan task S10); unset, the `@requires:unfolded-peer` scenario
 * holds as pending rather than failing on a peer that does not exist.
 */
const UNFOLDED_STORAGE_ENV = 'E2E_STORAGE_UNFOLDED';

/** Minimal slice of `ContentSearchView` this file reads. */
interface ContentSearchAnswer {
  query: string;
  rankingKnown: boolean;
  recipe: { name: string; cid: string };
  fold: { state: string };
  foldLag: { state: string; value?: { behind: number; limit: number; within: boolean } };
  candidates: { contentId: string; title: string; reach: string }[];
  facets: { reach: { value: string; count: number }[] };
  omissions: string[];
  unresolved: string[];
  totalCount: number;
}

/** Per-scenario state, keyed by the world so parallel workers never share it. */
interface SearchState {
  answer?: ContentSearchAnswer;
  askedBy?: string;
  query?: string;
  storageUrl?: string;
}

const STATE = new WeakMap<E2EWorld, SearchState>();

function state(world: E2EWorld): SearchState {
  let s = STATE.get(world);
  if (!s) {
    s = {};
    STATE.set(world, s);
  }
  return s;
}

function answerOf(world: E2EWorld): ContentSearchAnswer {
  const answer = state(world).answer;
  assert.ok(answer, 'no search answer captured — a "searches …" step must run first');
  return answer;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** The household's primary storage peer behind doorway "alpha". */
function householdStorageUrl(): string {
  return requireFixturePrimaryStorageUrl(loadHouseholdMeshFixture(), 'alpha');
}

/** The asker's own agent key — what the peer resolves the reach gate against. */
function agentCidOf(world: E2EWorld, humanName: string): string {
  const human = world.getHuman(humanName);
  assert.ok(
    human.agentPubKey,
    `${humanName} has no agentPubKey — the sign-in step must run before a search, since the ` +
      'reach gate is evaluated against the asker, not against the connection'
  );
  return human.agentPubKey;
}

/** One `GET /db/content/search` against a peer, as a named human. */
async function searchOnce(
  storageUrl: string,
  agentCid: string,
  query: string
): Promise<ContentSearchAnswer> {
  const url = `${storageUrl}${SEARCH_PATH}?q=${encodeURIComponent(query)}&limit=${SEARCH_LIMIT}`;
  const { status, text } = await getRaw(url, {
    timeoutMs: SEARCH_REQUEST_TIMEOUT_MS,
    headers: { 'X-Agent-Cid': agentCid },
  });
  assert.equal(
    status,
    200,
    `search on ${storageUrl} answered ${status} — the route must answer, even with no index ` +
      `(body: ${text.slice(0, 300)})`
  );
  let parsed: ContentSearchAnswer;
  try {
    parsed = JSON.parse(text) as ContentSearchAnswer;
  } catch (error) {
    throw new Error(`search answer is not JSON (${String(error)}): ${text.slice(0, 300)}`);
  }
  return parsed;
}

function includesContent(answer: ContentSearchAnswer, contentId: string): boolean {
  return answer.candidates.some(candidate => candidate.contentId === contentId);
}

async function sleep(ms: number): Promise<void> {
  await new Promise(resolve => setTimeout(resolve, ms));
}

/**
 * Ask the peer until the named row is in the answer, or the budget runs out.
 * Returns the LAST answer either way, so the Then step reports what the peer
 * actually said rather than a timeout with no evidence.
 */
async function searchUntilPresent(
  storageUrl: string,
  agentCid: string,
  query: string,
  contentId: string
): Promise<ContentSearchAnswer> {
  const deadline = Date.now() + SEARCH_CATCH_UP_BUDGET_MS;
  let answer = await searchOnce(storageUrl, agentCid, query);
  while (!includesContent(answer, contentId) && Date.now() < deadline) {
    await sleep(SEARCH_POLL_INTERVAL_MS);
    answer = await searchOnce(storageUrl, agentCid, query);
  }
  return answer;
}

// ---------------------------------------------------------------------------
// When — asking the household's own peer
// ---------------------------------------------------------------------------

When(
  '{word} searches the household library for that title',
  { timeout: SEARCH_CATCH_UP_BUDGET_MS + 30_000 },
  async function (this: E2EWorld, humanName: string) {
    const title = this.contentIds.get('lastContentTitle');
    assert.ok(
      title,
      'no content title captured — a "has created content titled …" step must run first'
    );
    const contentId = this.contentIds.get('lastContentId');
    assert.ok(
      contentId,
      'no content id captured — a "has created content titled …" step must run first'
    );

    const storageUrl = householdStorageUrl();
    const agentCid = agentCidOf(this, humanName);
    // Wait for the index to catch up ONLY when the asker is the person who
    // wrote the row: a non-holder's answer is expected never to carry it, and
    // polling for something that must not arrive would only burn the budget.
    const holder = this.contentIds.get('lastContentAuthor');
    const answer =
      holder === humanName
        ? await searchUntilPresent(storageUrl, agentCid, title, contentId)
        : await searchOnce(storageUrl, agentCid, title);

    const s = state(this);
    s.answer = answer;
    s.askedBy = humanName;
    s.query = title;
    s.storageUrl = storageUrl;
  }
);

/**
 * The unfolded peer is booted by the mesh, not by this step — so the Given has
 * no setup to do. It is in the Gherkin because the precondition belongs in the
 * causal chain a reader follows, and it is the honest place to hold the whole
 * scenario when no such peer is running.
 *
 * The same promise is pinned meanwhile by the storage integration test
 * `unfolded_store_answers_fold_absent`, which asks an unfolded store directly.
 */
Given('the household also runs a peer that holds no index', function (this: E2EWorld) {
  return process.env[UNFOLDED_STORAGE_ENV] ? undefined : PENDING;
});

When(
  '{word} asks that peer for anything at all',
  { timeout: 60_000 },
  async function (this: E2EWorld, humanName: string) {
    const storageUrl = process.env[UNFOLDED_STORAGE_ENV];
    if (!storageUrl) return PENDING;

    const query = 'anything at all';
    const answer = await searchOnce(storageUrl, agentCidOf(this, humanName), query);
    const s = state(this);
    s.answer = answer;
    s.askedBy = humanName;
    s.query = query;
    s.storageUrl = storageUrl;
    return undefined;
  }
);

// ---------------------------------------------------------------------------
// Then — what the answer must say
// ---------------------------------------------------------------------------

Then('the first result is that content', function (this: E2EWorld) {
  const answer = answerOf(this);
  const contentId = this.contentIds.get('lastContentId');
  assert.ok(contentId, 'no content id captured for this scenario');
  assert.ok(
    answer.candidates.length > 0,
    `the peer returned no results for "${answer.query}" (rankingKnown=${answer.rankingKnown}, ` +
      `fold=${answer.fold.state}, foldLag=${JSON.stringify(answer.foldLag)}, ` +
      `omissions=${JSON.stringify(answer.omissions)})`
  );
  assert.equal(
    answer.candidates[0].contentId,
    contentId,
    `the run's own item is not first for its own title: first was ` +
      `"${answer.candidates[0].title}" (${answer.candidates[0].contentId}) of ` +
      `${answer.totalCount} admitted`
  );
});

Then('no result is that content', function (this: E2EWorld) {
  const answer = answerOf(this);
  const contentId = this.contentIds.get('lastContentId');
  assert.ok(contentId, 'no content id captured for this scenario');
  assert.equal(
    includesContent(answer, contentId),
    false,
    `${state(this).askedBy} was served an item they do not hold: ${contentId} appears among ` +
      `${answer.candidates.length} results for "${answer.query}"`
  );
});

Then("the answer's tally by reach names no private item at all", function (this: E2EWorld) {
  const answer = answerOf(this);
  const privateFacet = answer.facets.reach.find(entry => entry.value === 'private');
  assert.equal(
    privateFacet,
    undefined,
    `the tally by reach counts ${privateFacet?.count} private item(s) for ` +
      `${state(this).askedBy}, so a withheld item shows through the counts even though it ` +
      'is missing from the results'
  );
});

Then('the answer says its ranking is known', function (this: E2EWorld) {
  const answer = answerOf(this);
  assert.equal(
    answer.rankingKnown,
    true,
    `the peer would not vouch for the order it returned (fold=${answer.fold.state}, ` +
      `foldLag=${JSON.stringify(answer.foldLag)})`
  );
});

Then('the answer says its ranking is not known', function (this: E2EWorld) {
  if (!process.env[UNFOLDED_STORAGE_ENV]) return PENDING;
  const answer = answerOf(this);
  assert.equal(answer.rankingKnown, false, 'a peer with no index claimed its ranking was known');
  return undefined;
});

Then('the answer names the recipe it ranked by, with a content address', function (this: E2EWorld) {
  const answer = answerOf(this);
  assert.ok(
    answer.recipe.name.length > 0,
    'the answer named no recipe, so the rules that ordered it cannot be looked up'
  );
  assert.ok(
    answer.recipe.cid.length > 0,
    `the recipe "${answer.recipe.name}" carries no content address, so the exact rules behind ` +
      'this order cannot be fetched or contested'
  );
});

Then(
  'asking the same question again names the same recipe address',
  { timeout: 60_000 },
  async function (this: E2EWorld) {
    const s = state(this);
    const first = answerOf(this);
    assert.ok(s.query && s.askedBy && s.storageUrl, 'no search was captured for this scenario');
    const again = await searchOnce(s.storageUrl, agentCidOf(this, s.askedBy), s.query);
    assert.equal(
      again.recipe.cid,
      first.recipe.cid,
      `the recipe address changed between two identical questions (${first.recipe.cid} then ` +
        `${again.recipe.cid}) — a fingerprint that moves is a label, not an address`
    );
    assert.equal(
      again.recipe.name,
      first.recipe.name,
      `the recipe name changed between two identical questions (${first.recipe.name} then ` +
        `${again.recipe.name})`
    );
  }
);

Then('the answer says that peer holds no index', function (this: E2EWorld) {
  if (!process.env[UNFOLDED_STORAGE_ENV]) return PENDING;
  const answer = answerOf(this);
  assert.equal(
    answer.fold.state,
    'absent',
    `a peer booted without an index reported fold "${answer.fold.state}" instead of "absent"`
  );
  return undefined;
});

Then('the answer carries no results alongside that admission', function (this: E2EWorld) {
  if (!process.env[UNFOLDED_STORAGE_ENV]) return PENDING;
  const answer = answerOf(this);
  assert.equal(
    answer.candidates.length,
    0,
    `a peer that cannot rank returned ${answer.candidates.length} result(s), which reads as a ` +
      'ranking it never made'
  );
  return undefined;
});
