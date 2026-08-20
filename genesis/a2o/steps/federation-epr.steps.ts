/**
 * EPR cross-peer resolution step definitions — features/federation/epr-cross-peer-resolution.feature.
 *
 * This is the `federation-epr.steps.ts` file that feature's own disposition block
 * names as the gate for its scenarios. It covers:
 *
 *   - the EPR / shard protocols being live on a peer
 *   - authoring publishing an EPR Head (provenance-gated, so a 200 is real proof)
 *   - honest 404 when nothing anywhere holds the content
 *   - CROSS-PEER custody: content authored on one peer, read from the other, and
 *     the diagnostic stamp that proves the read went through P2P resolution
 *
 * "The other peer" is a real one where the fleet declares one:
 * genesis/scripts/ci/e2e-verify-api.sh now points E2E_DOORWAY_STAGING at
 * E2E_DOORWAY_BETA — doorway-B (elohim.host, adam-backed, its own storage and
 * projection DB), a federation peer of doorway-A rather than an alias of it.
 * Where no second doorway is declared, staging falls back to the alpha host, and
 * `distinctPeers` below detects the collapse so the cross-peer steps HOLD
 * ('pending') naming it instead of asserting a tautology. That guard is the point
 * of this file: it must never be bypassed to buy a green.
 *
 * Substrate references:
 *   - protocol ids: elohim/elohim-storage/src/p2p/epr_protocol.rs (EPR_PROTOCOL_ID)
 *     and .../shard_protocol.rs (SHARD_PROTOCOL_ID); both are registered on the
 *     swarm behaviour whenever the libp2p stack is up (src/p2p/behaviour.rs).
 *   - GET /epr-head/{id} derives the head with require_provenance = true
 *     (elohim/elohim-storage/src/http.rs handle_get_epr_head → epr_head::derive_epr_head),
 *     so it answers 200 ONLY for a row notarized on Holochain or published to
 *     libp2p Kad. That is what makes it a usable "the DHT has this head" probe.
 */

import { strict as assert } from 'node:assert';
import { randomUUID } from 'node:crypto';

import { Given, When, Then } from '@cucumber/cucumber';

import { request } from 'undici';

import {
  fixtureDoorwayUrl,
  loadHouseholdMeshFixture,
} from '../src/framework/fixtures/household-mesh.js';
import { E2EWorld } from '../src/framework/world.js';

// ---------------------------------------------------------------------------
// Peer resolution
// ---------------------------------------------------------------------------

/**
 * The peer every scenario in this feature authors on. Named once rather than
 * spelled inline: it is the SAME peer across the protocol probes, the authoring
 * steps, and the cross-peer fallbacks, and a stray second spelling would make
 * "the other peer" quietly mean the origin.
 */
const ALPHA_PEER = 'alpha';

/** World key carrying which peer the current scenario authored on. */
const ORIGIN_PEER_KEY = 'epr:originPeer';

/**
 * A Then that reads a capture the preceding When must have made. Shared text so
 * every such step names the same missing precondition.
 */
const NO_RESPONSE_CAPTURED =
  'No response captured — run a "peer ... requests content ..." step first';

/** Protocol ids the storage node registers on its libp2p swarm. */
const KNOWN_P2P_PROTOCOLS = new Set([
  '/elohim/epr/1.0.0',
  '/elohim/shard/1.0.0',
  '/elohim/storage-sync/1.0.0',
  '/elohim/id/1.0.0',
]);

/** Trim trailing slashes without a backtracking regex (mirrors household-mesh.ts). */
function withoutTrailingSlashes(value: string): string {
  let end = value.length;
  while (end > 0 && value[end - 1] === '/') end -= 1;
  return value.slice(0, end);
}

/**
 * Resolve a peer name from the feature ("alpha", "staging") to a doorway URL.
 * Env first (that is what CI sets), then the household-mesh fixture manifest.
 */
function peerUrl(peerName: string): string {
  const envName = `E2E_DOORWAY_${peerName.toUpperCase().replace(/[^A-Z0-9]/g, '_')}`;
  const url =
    process.env[envName] ?? fixtureDoorwayUrl(loadHouseholdMeshFixture(), peerName) ?? undefined;
  assert.ok(
    url,
    `No URL for peer "${peerName}" — set ${envName} or declare it in the household-mesh fixture`
  );
  return withoutTrailingSlashes(url);
}

/** A peer's URL, or null when this substrate does not declare that peer at all. */
function optionalPeerUrl(peerName: string): string | null {
  try {
    return peerUrl(peerName);
  } catch {
    return null;
  }
}

/**
 * Are two peer names actually two peers on this substrate? The API harness
 * points several names at one host, and a cross-peer claim proved against one
 * host is not proved at all. An undeclared peer is likewise not a second peer.
 */
function distinctPeers(a: string, b: string): boolean {
  const first = optionalPeerUrl(a);
  const second = optionalPeerUrl(b);
  return first !== null && second !== null && first !== second;
}

/** World key for the content id a scenario alias resolved to. */
function aliasKey(alias: string): string {
  return `epr:${alias}`;
}

function resolveAlias(world: E2EWorld, alias: string): string {
  return world.contentIds.get(aliasKey(alias)) ?? alias;
}

interface HttpCapture {
  status: number;
  body: string;
}

const responses = new WeakMap<E2EWorld, HttpCapture>();

async function getJson(url: string): Promise<HttpCapture> {
  const { statusCode, body } = await request(url);
  return { status: statusCode, body: await body.text() };
}

// ---------------------------------------------------------------------------
// Background — the P2P planes the whole feature rides on
// ---------------------------------------------------------------------------

/**
 * Both protocol steps assert the same thing: this peer's libp2p stack is up, and
 * therefore the request-response protocol named is registered on it. There is no
 * HTTP surface that enumerates negotiated protocol ids, so the honest probe is
 * GET /p2p/status answering with this node's peer id. The protocol id itself is
 * checked against the ids the node actually registers, so a typo in the feature
 * fails here instead of silently passing.
 */
async function assertProtocolActive(world: E2EWorld, protocolId: string): Promise<void> {
  assert.ok(
    KNOWN_P2P_PROTOCOLS.has(protocolId),
    `"${protocolId}" is not a protocol elohim-storage registers. Known: ` +
      `${[...KNOWN_P2P_PROTOCOLS].join(', ')}`
  );
  const url = peerUrl(ALPHA_PEER);
  world.contentIds.set(ORIGIN_PEER_KEY, ALPHA_PEER);
  const status = await getJson(`${url}/p2p/status`);
  assert.equal(
    status.status,
    200,
    `GET ${url}/p2p/status returned ${status.status} — the libp2p stack that carries ` +
      `${protocolId} is not reachable`
  );
  const parsed = JSON.parse(status.body) as { peerId?: string };
  assert.ok(
    parsed.peerId,
    `${url}/p2p/status reports no peerId — no libp2p identity, so ${protocolId} is not active`
  );
}

Given(
  'the EPR protocol {string} is active between peers',
  async function (this: E2EWorld, protocolId: string) {
    await assertProtocolActive(this, protocolId);
  }
);

Given(
  'the shard protocol {string} is active between peers',
  async function (this: E2EWorld, protocolId: string) {
    await assertProtocolActive(this, protocolId);
  }
);

// ---------------------------------------------------------------------------
// Authoring on a peer
// ---------------------------------------------------------------------------

/**
 * Create a content row on a peer through the public write path. The feature
 * names content by a readable alias ("test-concept"); a run-unique id is used on
 * the wire so parallel runs and re-runs never collide, and the row is removed on
 * cleanup — the same discipline steps/federation.steps.ts uses for its authored
 * content.
 */
async function createContentOnPeer(
  world: E2EWorld,
  peerName: string,
  alias: string
): Promise<string> {
  const url = peerUrl(peerName);
  const id = `e2e-epr-${alias}-${randomUUID().slice(0, 8)}`;
  const { statusCode, body } = await request(`${url}/db/content`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      id,
      contentType: 'article',
      title: `E2E EPR ${alias}`,
      description: `Authored by the epr-cross-peer-resolution scenario on peer "${peerName}"`,
      contentBody: `EPR cross-peer resolution fixture for "${alias}".`,
      contentFormat: 'markdown',
      tags: ['e2e', 'federation', 'epr'],
    }),
  });
  const text = await body.text();
  assert.ok(
    statusCode === 200 || statusCode === 201,
    `POST ${url}/db/content returned ${statusCode}: ${text}`
  );

  world.contentIds.set(aliasKey(alias), id);
  world.contentIds.set(ORIGIN_PEER_KEY, peerName);
  // Marks the id as authored BY THIS RUN. The request step reads it to decide
  // whether a miss is "not published yet" (wait — publication is paced) or
  // "nothing anywhere holds this" (answer immediately, which is what the honest-
  // 404 scenario asserts). Without the marker both look identical from a GET.
  world.contentIds.set(`epr:authored:${id}`, peerName);
  world.onCleanup(async () => {
    try {
      await request(`${url}/db/content/${id}`, { method: 'DELETE' });
    } catch {
      // best-effort cleanup
    }
  });
  return id;
}

Given(
  'peer {string} ingests content {string}',
  { timeout: 60_000 },
  async function (this: E2EWorld, peerName: string, alias: string) {
    await createContentOnPeer(this, peerName, alias);
  }
);

Given(
  // The '/' characters must be escaped: an unescaped '/' in a cucumber expression is
  // alternation syntax, and 'POST /db/content' reads as an EMPTY alternative — which
  // throws at support-code load and aborts the whole run (a2o CLAUDE.md watch-out).
  String.raw`peer {string} creates content {string} via POST \/db\/content`,
  { timeout: 60_000 },
  async function (this: E2EWorld, peerName: string, alias: string) {
    await createContentOnPeer(this, peerName, alias);
  }
);

/** Poll a URL until it answers 200, or the budget runs out. Returns the last answer. */
async function pollUntilOk(url: string, budgetMs: number): Promise<HttpCapture> {
  const deadline = Date.now() + budgetMs;
  let last: HttpCapture = { status: 0, body: '' };
  for (;;) {
    last = await getJson(url);
    if (last.status === 200 || Date.now() >= deadline) return last;
    await new Promise(resolve => setTimeout(resolve, 3000));
  }
}

/**
 * The publish queue's state, so a timeout can say WHY the head is not on the DHT
 * yet rather than just that it is not. Publication is paced (drain.pending), so
 * "not yet" and "never" look identical from /epr-head alone.
 */
async function publishBacklog(url: string): Promise<string> {
  try {
    const status = await getJson(`${url}/p2p/status`);
    const parsed = JSON.parse(status.body) as {
      drain?: Record<string, unknown>;
      replication?: Record<string, unknown>;
    };
    return ` Publish queue at timeout: drain=${JSON.stringify(parsed.drain)} replication=${JSON.stringify(parsed.replication)}.`;
  } catch {
    return '';
  }
}

/** How long authoring is given to reach the DHT — publication is paced, not instant. */
const PUBLISH_BUDGET_MS = 150_000;

/**
 * GET /epr-head/{id} is provenance-gated: derive_epr_head is called with
 * require_provenance = true, so it answers 200 only once the row carries a
 * Holochain anchor or a libp2p Kad publication. A 200 with a CID is therefore a
 * real "the head is on the DHT" assertion, not a projection read.
 *
 * Example: Then the DHT contains an EPR Head for "new-concept"
 */
Then(
  'the DHT contains an EPR Head for {string}',
  { timeout: 180_000 },
  async function (this: E2EWorld, alias: string) {
    const peerName = this.contentIds.get(ORIGIN_PEER_KEY) ?? ALPHA_PEER;
    const url = peerUrl(peerName);
    const id = resolveAlias(this, alias);

    const last = await pollUntilOk(`${url}/epr-head/${id}`, PUBLISH_BUDGET_MS);

    // publishBacklog() is a NETWORK read. Interpolated straight into the
    // message argument it is awaited BEFORE assert.equal is ever called — i.e.
    // on the PASSING path too, costing a /p2p/status round-trip per green run.
    // Guard the call so the diagnostic is fetched only when it will be read.
    if (last.status !== 200) {
      assert.equal(
        last.status,
        200,
        `GET ${url}/epr-head/${id} still ${last.status} after ${PUBLISH_BUDGET_MS / 1000}s. That ` +
          `endpoint is provenance-gated, so "${alias}" was neither notarized on Holochain nor ` +
          `published to libp2p Kad after authoring.${await publishBacklog(url)} ` +
          `Body: ${last.body.slice(0, 200)}`
      );
    }
    const head = JSON.parse(last.body) as { cid?: string; id?: string };
    assert.ok(head.cid, `EPR Head for "${alias}" carries no CID: ${last.body.slice(0, 200)}`);
    assert.equal(head.id, id, `EPR Head returned id "${head.id}", expected "${id}"`);

    // A servable head is NOT the same claim as "on the DHT". /epr-head gates at
    // MinTrust::Amber (elohim/elohim-storage/src/db/content_diesel.rs), which also
    // admits crdt_converged_at — a row that only reached the Automerge plane.
    // The row's own trust label is what separates the two:
    // 'notarized' = dht_anchor_hash, 'published' = p2p_published_at (peer-attested),
    // 'unconfirmed' = CRDT-converged-only (elohim/elohim-views/src/lamad.rs).
    const row = await getJson(`${url}/db/content/${id}`);
    assert.equal(row.status, 200, `GET ${url}/db/content/${id} returned ${row.status}`);
    const view = JSON.parse(row.body) as { trust?: string; dhtAnchorHash?: string | null };
    const trustReachedTheDht = view.trust === 'notarized' || view.trust === 'published';
    // Same lazy-message discipline as the /epr-head assertion above.
    if (!trustReachedTheDht) {
      assert.ok(
        trustReachedTheDht,
        `EPR Head for "${alias}" is servable but its row reports trust="${view.trust}" ` +
          `(dhtAnchorHash=${JSON.stringify(view.dhtAnchorHash)}). "unconfirmed" means the row ` +
          `only converged over the CRDT plane — it is not on the DHT, which is what this ` +
          `scenario claims.${await publishBacklog(url)}`
      );
    }
  }
);

/**
 * Kademlia discovery is a claim about a DIFFERENT peer finding the head. When
 * the harness points both peer names at one host there is no second peer to
 * discover from, and re-reading the authoring peer would assert nothing — so the
 * step HOLDS and names the gap instead.
 *
 * Example: And peer "staging" can discover "test-concept" via Kademlia lookup
 */
Then(
  'peer {string} can discover {string} via Kademlia lookup',
  { timeout: 180_000 },
  async function (this: E2EWorld, peerName: string, alias: string) {
    const originPeer = this.contentIds.get(ORIGIN_PEER_KEY) ?? ALPHA_PEER;
    if (!distinctPeers(peerName, originPeer)) {
      // HOLD, do not pass: peer "<peerName>" and peer "<originPeer>" resolve to the
      // same URL on this substrate (CI sets E2E_DOORWAY_STAGING = E2E_DOORWAY_ALPHA),
      // so a lookup here would be the authoring node reading its own database.
      return 'pending';
    }
    const url = peerUrl(peerName);
    const id = resolveAlias(this, alias);

    const last = await pollUntilOk(`${url}/epr-head/${id}`, PUBLISH_BUDGET_MS);
    assert.equal(
      last.status,
      200,
      `Peer "${peerName}" could not resolve an EPR Head for "${alias}" within ` +
        `${PUBLISH_BUDGET_MS / 1000}s (GET ${url}/epr-head/${id} -> ${last.status})`
    );
  }
);

// ---------------------------------------------------------------------------
// Honest absence
// ---------------------------------------------------------------------------

/**
 * Establish that nothing anywhere holds the content — checked on every peer the
 * harness knows, not just the one about to be asked, because the point of the
 * scenario is that the 404 is a network-wide answer.
 *
 * Example: Given no peer has content "nonexistent-concept"
 */
Given(
  'no peer has content {string}',
  { timeout: 60_000 },
  async function (this: E2EWorld, alias: string) {
    // A run-unique id keeps this true even if some earlier run authored the
    // literal slug — absence has to be a fact, not a hope.
    const id = `${alias}-${randomUUID().slice(0, 8)}`;
    this.contentIds.set(aliasKey(alias), id);

    const seen = new Set<string>();
    for (const peerName of [ALPHA_PEER, 'staging']) {
      // A peer this substrate does not declare is not a peer that could be
      // holding the content; only the ones the harness knows are checked.
      const url = peerName === ALPHA_PEER ? peerUrl(peerName) : optionalPeerUrl(peerName);
      if (url === null || seen.has(url)) continue;
      seen.add(url);
      const resp = await getJson(`${url}/db/content/${id}`);
      assert.equal(
        resp.status,
        404,
        `Peer "${peerName}" already has "${id}" (GET /db/content/${id} -> ${resp.status}); ` +
          `the scenario needs content no peer holds`
      );
    }
  }
);

/**
 * Read content from a peer.
 *
 * Two scenarios share this verb with opposite expectations, and the difference is
 * a fact about the substrate, not about the assertion:
 *  - nothing anywhere holds the id (the honest-404 scenario) — one read, answered
 *    now; polling would only spend the budget on a settled answer.
 *  - the id was authored on ANOTHER peer in this run (the cross-peer scenario) —
 *    publication to Kad is PACED (drain.pending), so an immediate miss means "not
 *    yet", not "never". Give it the same publish budget the EPR-Head steps use.
 * Neither branch relaxes what the Then steps assert; the wait only lets a paced
 * substrate finish before it is judged.
 *
 * Example: When peer "staging" requests content "fct-module-01-church-dilemma"
 */
When(
  'peer {string} requests content {string}',
  { timeout: 180_000 },
  async function (this: E2EWorld, peerName: string, alias: string) {
    const url = peerUrl(peerName);
    const id = resolveAlias(this, alias);
    const authoredOn = this.contentIds.get(`epr:authored:${id}`);
    const capture =
      authoredOn && authoredOn !== peerName
        ? await pollUntilOk(`${url}/db/content/${id}`, PUBLISH_BUDGET_MS)
        : await getJson(`${url}/db/content/${id}`);
    responses.set(this, capture);
  }
);

Then('the response is 404 Not Found', function (this: E2EWorld) {
  const resp = responses.get(this);
  assert.ok(resp, NO_RESPONSE_CAPTURED);
  assert.equal(
    resp.status,
    404,
    `Expected 404 Not Found but got ${resp.status}: ${resp.body.slice(0, 200)}`
  );
});

// ---------------------------------------------------------------------------
// Cross-peer custody — the half that needs TWO peers
// ---------------------------------------------------------------------------

/**
 * The one sentence every held cross-peer step prints. Kept in one place so a
 * reader of the report learns the SUBSTRATE fact ("this fleet declared one
 * doorway under both names") rather than an environment-variable name.
 */
function collapsedPeersNote(a: string, b: string): string {
  const url = optionalPeerUrl(a);
  return (
    `peers "${a}" and "${b}" resolve to the same URL on this substrate ` +
    `(${url ?? '<undeclared>'}). A cross-peer claim proved against one host is not ` +
    `proved. CI points E2E_DOORWAY_STAGING at E2E_DOORWAY_BETA when the fleet ` +
    `declares a second doorway (genesis/scripts/ci/e2e-verify-api.sh); this run did not.`
  );
}

/** The counterpart peer a scenario is reading ACROSS from. */
function originPeerOf(world: E2EWorld): string {
  return world.contentIds.get(ORIGIN_PEER_KEY) ?? ALPHA_PEER;
}

/**
 * Is this content in the peer's OWN database, without asking it to go find it?
 *
 * `GET /db/content/{id}` cannot answer this: on a local miss it falls through to
 * the P2P resolve-and-fetch arm (elohim/elohim-storage/src/http.rs), so the probe
 * would create the very state it was meant to observe. `GET /epr-head/{id}` reads
 * the local DB only (`derive_epr_head` over this node's connection — no network
 * arm), which makes it the non-perturbing local-presence probe.
 *
 * It is provenance-gated, so it under-reports: a row present locally but not yet
 * published reads as absent. That is why absence here is a PRECONDITION check,
 * never the proof — the proof that the read crossed peers is the `resolved_via`
 * stamp, which storage writes ONLY on the P2P path.
 */
async function localHead(peerName: string, id: string): Promise<HttpCapture> {
  return getJson(`${peerUrl(peerName)}/epr-head/${id}`);
}

/** The parsed content row a peer serves for an id. */
interface ContentRowView {
  id?: string;
  contentBody?: string;
  metadata?: Record<string, unknown> | null;
}

async function contentRow(
  peerName: string,
  id: string
): Promise<{ capture: HttpCapture; view: ContentRowView }> {
  const capture = await getJson(`${peerUrl(peerName)}/db/content/${id}`);
  let view: ContentRowView = {};
  if (capture.status === 200) {
    try {
      view = JSON.parse(capture.body) as ContentRowView;
    } catch {
      view = {};
    }
  }
  return { capture, view };
}

/**
 * The diagnostic stamp storage writes when — and only when — a read missed
 * locally and was satisfied by EPR resolve + shard fetch from another peer
 * (`metadata_json: {"resolved_via":"p2p"}` in the resolve_and_fetch arm). Reading
 * it back is therefore proof of the crossing, not a restatement of the request.
 */
function resolvedVia(view: ContentRowView): string | undefined {
  const value = view.metadata?.['resolved_via'];
  return typeof value === 'string' ? value : undefined;
}

/**
 * Custody: the content exists on THIS peer and is attributed to the named
 * steward. `createdBy` is the only attribution the public write path accepts —
 * proportional stewardship SHARES live in a separate allocation surface, and no
 * assertion in this scenario reads them, so nothing here claims to have set them.
 *
 * Example: Given peer "alpha" has content "fct-module-01-church-dilemma" stewarded by Pete
 */
Given(
  'peer {string} has content {string} stewarded by {word}',
  { timeout: 60_000 },
  async function (this: E2EWorld, peerName: string, alias: string, steward: string) {
    const id = await createContentOnPeer(this, peerName, alias);
    this.contentIds.set(`epr:steward:${id}`, steward);
  }
);

/**
 * The asymmetry the whole scenario rests on: the other peer must not already hold
 * this content, or "resolves across peers" is untested.
 *
 * Example: And peer "staging" does not have "fct-module-01-church-dilemma" locally
 */
Given(
  'peer {string} does not have {string} locally',
  { timeout: 60_000 },
  async function (this: E2EWorld, peerName: string, alias: string) {
    const origin = originPeerOf(this);
    if (!distinctPeers(peerName, origin)) {
      // HOLD, do not pass: asserting absence on the authoring host itself would
      // be false, and asserting it on an alias would be a tautology.
      this.attach?.(`HELD: ${collapsedPeersNote(peerName, origin)}`);
      return 'pending';
    }
    const id = resolveAlias(this, alias);
    const head = await localHead(peerName, id);
    assert.equal(
      head.status,
      404,
      `Peer "${peerName}" already holds an EPR Head for "${id}" ` +
        `(GET /epr-head/${id} -> ${head.status}). The scenario needs content that is ` +
        `absent there, so the read it makes next is a real cross-peer resolution.`
    );
    return undefined;
  }
);

/**
 * The read crossed peers. Proven by the stamp storage writes on the resolve arm,
 * not by the 200 alone — a 200 could equally have come from a local row.
 *
 * Example: Then the content is resolved via EPR protocol from peer "alpha"
 */
Then(
  'the content is resolved via EPR protocol from peer {string}',
  { timeout: 60_000 },
  async function (this: E2EWorld, originName: string) {
    const reader = 'staging';
    if (!distinctPeers(reader, originName)) {
      this.attach?.(`HELD: ${collapsedPeersNote(reader, originName)}`);
      return 'pending';
    }
    const resp = responses.get(this);
    assert.ok(resp, NO_RESPONSE_CAPTURED);
    assert.equal(
      resp.status,
      200,
      `Peer "${reader}" did not obtain the content from peer "${originName}" ` +
        `(GET /db/content -> ${resp.status}): ${resp.body.slice(0, 200)}`
    );

    const id = resp.body ? ((JSON.parse(resp.body) as ContentRowView).id ?? '') : '';
    const { view } = await contentRow(reader, id);
    assert.equal(
      resolvedVia(view),
      'p2p',
      `Peer "${reader}" served "${id}" but its local record does not carry ` +
        `metadata.resolved_via="p2p" (got ${JSON.stringify(view.metadata)}). Storage stamps ` +
        `that ONLY on the EPR resolve + shard fetch arm, so without it the read was served ` +
        `from a row this peer already had — not a cross-peer resolution.`
    );
    return undefined;
  }
);

/**
 * The BODY came too, not just the head. The resolve arm fetches the body bytes
 * over the shard plane and stores them; an EPR Head alone would leave the row
 * with no contentBody.
 *
 * Example: And the content body is fetched via shard protocol
 */
Then(
  'the content body is fetched via shard protocol',
  { timeout: 60_000 },
  // Not async: this step only re-reads the capture the resolve step already
  // made. Marking it async bought nothing and tripped require-await.
  function (this: E2EWorld) {
    const origin = originPeerOf(this);
    if (!distinctPeers('staging', origin)) {
      this.attach?.(`HELD: ${collapsedPeersNote('staging', origin)}`);
      return 'pending';
    }
    const resp = responses.get(this);
    assert.ok(resp, NO_RESPONSE_CAPTURED);
    const view = JSON.parse(resp.body) as ContentRowView;
    assert.ok(
      view.contentBody && view.contentBody.length > 0,
      `The row peer "staging" served carries no contentBody — the EPR Head crossed but the ` +
        `body did not, so the shard fetch did not complete: ${resp.body.slice(0, 200)}`
    );
    return undefined;
  }
);

/**
 * The crossing is durable, not a pass-through. After the resolve, the row is in
 * the reader's OWN database — observable through the local-only head probe.
 *
 * Example: And the content is persisted to local SQLite on peer "staging"
 */
Then(
  'the content is persisted to local SQLite on peer {string}',
  { timeout: 60_000 },
  async function (this: E2EWorld, peerName: string) {
    const origin = originPeerOf(this);
    if (!distinctPeers(peerName, origin)) {
      this.attach?.(`HELD: ${collapsedPeersNote(peerName, origin)}`);
      return 'pending';
    }
    const resp = responses.get(this);
    assert.ok(resp, NO_RESPONSE_CAPTURED);
    const id = (JSON.parse(resp.body) as ContentRowView).id ?? '';
    const { capture, view } = await contentRow(peerName, id);
    assert.equal(capture.status, 200, `GET ${peerName}/db/content/${id} -> ${capture.status}`);
    assert.equal(
      resolvedVia(view),
      'p2p',
      `Peer "${peerName}" has no persisted P2P-resolved row for "${id}" ` +
        `(metadata=${JSON.stringify(view.metadata)}). The resolve arm persists the row it ` +
        `fetched; without the row the next read would resolve all over again.`
    );
    return undefined;
  }
);

/**
 * Once the row is local, the read path returns at the local branch and never
 * reaches the P2P arm (`if !matches!(&result, Ok(None)) { return … }` precedes it
 * in elohim/elohim-storage/src/http.rs). So proving locality IS proving no second
 * resolution — and the stamp staying `p2p` proves the row was not re-created.
 *
 * Example: And subsequent requests return the content without P2P resolution
 */
Then(
  'subsequent requests return the content without P2P resolution',
  { timeout: 60_000 },
  async function (this: E2EWorld) {
    const origin = originPeerOf(this);
    if (!distinctPeers('staging', origin)) {
      this.attach?.(`HELD: ${collapsedPeersNote('staging', origin)}`);
      return 'pending';
    }
    const resp = responses.get(this);
    assert.ok(resp, NO_RESPONSE_CAPTURED);
    const first = JSON.parse(resp.body) as ContentRowView;
    const id = first.id ?? '';

    const head = await localHead('staging', id);
    assert.equal(
      head.status,
      200,
      `Peer "staging" has no local EPR Head for "${id}" (GET /epr-head/${id} -> ${head.status}), ` +
        `so the next read would fall through to the P2P arm again rather than being served locally.`
    );

    const { capture, view } = await contentRow('staging', id);
    assert.equal(capture.status, 200, `Second GET staging/db/content/${id} -> ${capture.status}`);
    assert.equal(
      view.contentBody,
      first.contentBody,
      `The second read of "${id}" returned a different body than the first — the row was not ` +
        `simply served from local storage.`
    );
    return undefined;
  }
);

/**
 * The compound precondition scenario 2 states in one line: content authored on
 * the OTHER peer, absent here, then read here — so the read is a real crossing.
 *
 * Example: Given peer "staging" resolves "cross-steward-concept" via P2P
 */
Given(
  'peer {string} resolves {string} via P2P',
  { timeout: 240_000 },
  async function (this: E2EWorld, peerName: string, alias: string) {
    // "alpha" is the only counterpart this harness declares; the feature names
    // the READER, so the author is the other one.
    const origin = ALPHA_PEER;
    if (!distinctPeers(peerName, origin)) {
      this.attach?.(`HELD: ${collapsedPeersNote(peerName, origin)}`);
      return 'pending';
    }

    const id = await createContentOnPeer(this, origin, alias);

    const before = await localHead(peerName, id);
    assert.equal(
      before.status,
      404,
      `Peer "${peerName}" already holds "${id}" before the crossing ` +
        `(GET /epr-head/${id} -> ${before.status}) — nothing would be resolved.`
    );

    const read = await pollUntilOk(`${peerUrl(peerName)}/db/content/${id}`, PUBLISH_BUDGET_MS);
    assert.equal(
      read.status,
      200,
      `Peer "${peerName}" never obtained "${id}" from peer "${origin}" within ` +
        `${PUBLISH_BUDGET_MS / 1000}s (GET /db/content/${id} -> ${read.status}).` +
        `${await publishBacklog(peerUrl(origin))} Body: ${read.body.slice(0, 200)}`
    );
    responses.set(this, read);
    this.contentIds.set('epr:resolverPeer', peerName);
    return undefined;
  }
);

/**
 * The provenance the operator reads. Asserted against the row the RESOLVING peer
 * holds — the diagnostic belongs to the peer that did the resolving.
 *
 * Example: Then the local content record has metadata "resolved_via" = "p2p"
 */
Then(
  'the local content record has metadata {string} = {string}',
  { timeout: 60_000 },
  async function (this: E2EWorld, key: string, expected: string) {
    const peerName = this.contentIds.get('epr:resolverPeer') ?? 'staging';
    const origin = originPeerOf(this);
    if (!distinctPeers(peerName, origin)) {
      this.attach?.(`HELD: ${collapsedPeersNote(peerName, origin)}`);
      return 'pending';
    }
    const resp = responses.get(this);
    assert.ok(resp, 'No response captured — run a "peer ... resolves ... via P2P" step first');
    const id = (JSON.parse(resp.body) as ContentRowView).id ?? '';
    const { view } = await contentRow(peerName, id);
    const actual = view.metadata?.[key];
    assert.equal(
      actual,
      expected,
      `Peer "${peerName}"'s record for "${id}" has metadata.${key}=${JSON.stringify(actual)}, ` +
        `expected "${expected}" (full metadata: ${JSON.stringify(view.metadata)}). Storage stamps ` +
        `this on the resolve arm, so a missing value means the read never crossed peers.`
    );
    return undefined;
  }
);
