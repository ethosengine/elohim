/**
 * Intimate-reach household step definitions.
 *
 * Feature: features/lamad/intimate-reach-household.feature
 *
 * EPR content at reach "intimate" belongs to a consented dyad. The serving
 * gate (epr_service::check_reach_authorization, intimate branch) serves such
 * content only to an agent with a MUTUAL, dual-consented intimate relationship
 * to the content's steward. A household member who merely stewards bytes (no
 * intimate relationship) is refused.
 *
 * These two scenarios are API-tier: the personas "request content" via their
 * authenticated DoorwayClient (token from login), and the anonymous-visitor
 * assertions reuse reach.steps.ts. No browser is required.
 *
 * IDENTITY ALIGNMENT (the subtle part)
 * ------------------------------------
 * The gate resolves the REQUESTING human from the request's agent identity and
 * matches the relationship's other party against the content's steward human
 * ids. The fixture logins return derived human ids (e.g.
 *   Matthew -> human-matthew.dowell@alpha.elohim.host
 *   Jessica -> human-jessica@test.elohim.host
 *   James   -> human-james@test.elohim.host
 * ) which differ from the canonical persona slugs in humans.json
 * (human-matthew-manager, human-jessica-spouse, human-james-son). We therefore
 * bind the relationship and the content's created_by to the *actual* humanIds
 * each login returns, captured on the world after the login step runs.
 *
 * Steps reused (NOT redefined here):
 *   - 'human {string} is logged in on doorway {string} with device'
 *       (fixture-humans.steps.ts) — performs the login and stores humanId.
 *   - 'an anonymous client GETs content {string}' (reach.steps.ts)
 *   - 'the 403 body requiredReach is {string}'   (reach.steps.ts)
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { request } from 'undici';

import { fixtureCredentials, isHumanDeployed } from '../../src/framework/fixtures/humans.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const REACH_INTIMATE = 'intimate';
const RELATIONSHIP_TYPE = 'partner';
const DEFAULT_DOORWAY = 'alpha';

/** Per-persona captured responses, keyed by persona name on the world. */
interface PersonaResponse {
  status: number;
  body: string;
}

interface IntimateState {
  personaResponses?: Map<string, PersonaResponse>;
}

function intimateState(world: E2EWorld): IntimateState & Record<string, unknown> {
  const w = world as unknown as Record<string, unknown>;
  w['__intimate'] ??= {};
  return w['__intimate'] as IntimateState & Record<string, unknown>;
}

function firstDoorwayUrl(world: E2EWorld): string {
  const doorway = [...world.doorways.values()][0];
  assert.ok(doorway, 'No doorway registered — run the Background "doorway ... at ..." step first');
  return doorway.url;
}

/**
 * Obtain an admin (Matthew) bearer token for raw authorized writes/reads.
 * Cached per-world after the first login so repeated steps reuse it.
 */
async function adminTokenFor(world: E2EWorld, baseUrl: string): Promise<string> {
  const state = intimateState(world);
  const cached = state['__adminToken'] as string | undefined;
  if (cached) return cached;
  const creds = fixtureCredentials('Matthew');
  const resp = await postJsonAs(baseUrl, '/auth/login', {
    identifier: creds.identifier,
    password: creds.password,
  });
  assert.ok(
    resp.status >= 200 && resp.status < 300,
    `Admin login failed: status ${resp.status}, body ${resp.body.slice(0, 200)}`
  );
  const token = (JSON.parse(resp.body) as { token?: string }).token;
  assert.ok(token, 'Admin login returned no token');
  state['__adminToken'] = token;
  return token;
}

/**
 * Resolve a persona's humanId. Prefers the id stored on the world from a prior
 * login (the authoritative value the auth response returned). If the persona is
 * NOT logged in this run (e.g. scenario 2 sets up the Matthew↔Jessica
 * relationship as a precondition without logging either in), derive it from the
 * fixture identifier — the doorway derives `humanId = "human-" + identifier`
 * (auth_routes.rs), so the derivation is deterministic and matches the login.
 */
function humanIdOf(world: E2EWorld, name: string): string {
  const existing = world.humans.get(name);
  if (existing?.humanId) return existing.humanId;
  const creds = fixtureCredentials(name);
  return `human-${creds.identifier}`;
}

/** Resolve a logged-in human's bearer token for the given doorway. */
function tokenOf(world: E2EWorld, name: string, doorwayId = DEFAULT_DOORWAY): string {
  const human = world.getHuman(name);
  const token = human.getToken(doorwayId);
  assert.ok(token, `Human "${name}" has no token for doorway "${doorwayId}"`);
  return token;
}

/** GET a path with an optional bearer token; returns status + raw body text. */
async function getContentAs(
  baseUrl: string,
  contentId: string,
  token?: string
): Promise<PersonaResponse> {
  const headers: Record<string, string> = {};
  if (token) headers['authorization'] = `Bearer ${token}`;
  const { statusCode, body } = await request(`${baseUrl}/db/content/${contentId}`, {
    method: 'GET',
    headers,
  });
  return { status: statusCode, body: await body.text() };
}

/** POST JSON with an optional bearer token; returns status + raw body text. */
async function postJsonAs(
  baseUrl: string,
  path: string,
  payload: unknown,
  token?: string
): Promise<PersonaResponse> {
  const headers: Record<string, string> = { 'content-type': 'application/json' };
  if (token) headers['authorization'] = `Bearer ${token}`;
  const { statusCode, body } = await request(`${baseUrl}${path}`, {
    method: 'POST',
    headers,
    body: JSON.stringify(payload),
  });
  return { status: statusCode, body: await body.text() };
}

/**
 * Raw shape of a human_relationships row as returned by GET
 * /db/human-relationships. NOTE: this list endpoint returns the internal DB
 * model verbatim — snake_case keys and integer (0/1) booleans — unlike the POST
 * InputView which is camelCase. We read it as-is.
 */
interface RawRelationshipRow {
  id?: string;
  party_a_id?: string;
  party_b_id?: string;
  intimacy_level?: string;
  consent_given_by_a?: number;
  consent_given_by_b?: number;
}

/** List relationship rows involving a party (snake_case raw model). */
async function listRelationships(
  baseUrl: string,
  adminToken: string,
  partyId: string
): Promise<RawRelationshipRow[]> {
  const headers: Record<string, string> = { authorization: `Bearer ${adminToken}` };
  const { statusCode, body } = await request(
    `${baseUrl}/db/human-relationships?partyId=${encodeURIComponent(partyId)}`,
    { method: 'GET', headers }
  );
  const text = await body.text();
  if (statusCode < 200 || statusCode >= 300) return [];
  return (JSON.parse(text) as { items?: RawRelationshipRow[] }).items ?? [];
}

/** True if `row` is an intimate relationship between the two parties (either order). */
function isIntimateBetween(row: RawRelationshipRow, a: string, b: string): boolean {
  const between =
    (row.party_a_id === a && row.party_b_id === b) ||
    (row.party_a_id === b && row.party_b_id === a);
  return between && row.intimacy_level === REACH_INTIMATE;
}

/**
 * Ensure a mutual, dual-consented intimate relationship row exists between two
 * humanIds. Idempotent: if a matching dual-consented row already exists
 * (scenario rerun), do nothing. Uses an admin (Matthew) token so the write is
 * authorized.
 */
async function ensureMutualIntimateRelationship(
  baseUrl: string,
  adminToken: string,
  partyAId: string,
  partyBId: string
): Promise<void> {
  const existing = await listRelationships(baseUrl, adminToken, partyAId);
  const match = existing.find(
    r =>
      isIntimateBetween(r, partyAId, partyBId) &&
      r.consent_given_by_a === 1 &&
      r.consent_given_by_b === 1
  );
  if (match) return; // already present, fully consented — idempotent

  const payload = {
    partyAId,
    partyBId,
    relationshipType: RELATIONSHIP_TYPE,
    intimacyLevel: REACH_INTIMATE,
    isBidirectional: true,
    consentGivenByA: true,
    consentGivenByB: true,
    initiatedBy: partyAId,
    reach: REACH_INTIMATE,
  };
  const resp = await postJsonAs(baseUrl, '/db/human-relationships', payload, adminToken);
  // A UNIQUE-constraint failure means a row for (parties, type) already exists
  // but lacked full consent above (e.g. partial prior run) — tolerate it; the
  // row is present and the gate substrate is in place.
  const isDuplicate =
    resp.status >= 400 && resp.body.includes('UNIQUE constraint failed: human_relationships');
  assert.ok(
    (resp.status >= 200 && resp.status < 300) || isDuplicate,
    `Failed to create intimate relationship (${partyAId} <-> ${partyBId}): ` +
      `status ${resp.status}, body ${resp.body.slice(0, 300)}`
  );
}

// ---------------------------------------------------------------------------
// Given — relationship + content setup
// ---------------------------------------------------------------------------

/**
 * Create/upsert a mutual intimate relationship with dual consent between the
 * two named, already-logged-in humans. Binds to the ACTUAL humanIds returned by
 * their logins (not the persona slugs).
 */
Given(
  'humans {string} and {string} have a mutual intimate relationship with dual consent',
  async function (this: E2EWorld, nameA: string, nameB: string) {
    if (!isHumanDeployed(nameA) || !isHumanDeployed(nameB)) return 'pending';
    const baseUrl = firstDoorwayUrl(this);
    const partyAId = humanIdOf(this, nameA);
    const partyBId = humanIdOf(this, nameB);
    const adminAuthToken = await adminTokenFor(this, baseUrl);
    await ensureMutualIntimateRelationship(baseUrl, adminAuthToken, partyAId, partyBId);
  }
);

/**
 * Ensure a content row exists with the given reach, stewarded (created_by) by
 * the named human's actual humanId. Upserts via POST /db/content (idempotent
 * on id).
 *
 * NOTE ON PROVENANCE (live-stack constraint): external content reads require
 * provenance (content_diesel::get_content with require_provenance=true filters
 * rows where BOTH dht_anchor_hash AND p2p_published_at are NULL). Provenance is
 * written only by the libp2p drain loop (content_diesel::mark_published, called
 * exclusively from p2p/mod.rs). When P2P networking is disabled on the local
 * stack, a freshly-POSTed row stays unprovenanced and is therefore invisible to
 * the external HTTP boundary. This step performs the legitimate POST; if the
 * row cannot become externally readable the downstream serve/refuse assertions
 * surface the precise blocker.
 */
Given(
  'content {string} is stewarded by {string} at reach {string}',
  async function (this: E2EWorld, contentId: string, stewardName: string, reach: string) {
    if (!isHumanDeployed(stewardName)) return 'pending';
    const baseUrl = firstDoorwayUrl(this);
    const stewardId = humanIdOf(this, stewardName);
    const adminAuthToken = await adminTokenFor(this, baseUrl);

    const payload = {
      id: contentId,
      title: `Love Map (${stewardName})`,
      content: 'A consented couple keeps a private love map at reach "intimate".',
      contentType: 'concept',
      contentFormat: 'markdown',
      reach,
      createdBy: stewardId,
    };
    // Idempotent upsert: POST creates the row; on rerun the row already exists
    // and the insert hits a UNIQUE constraint (surfaced as a 500 with a
    // recognizable body) — in that case PATCH the reach so the row carries the
    // intended grade. Either way the row exists with reach=<reach> afterward.
    const resp = await postJsonAs(baseUrl, '/db/content', payload, adminAuthToken);
    const isDuplicate =
      resp.status >= 400 && resp.body.includes('UNIQUE constraint failed: content.h_app_id');
    if (resp.status >= 400 && !isDuplicate) {
      assert.fail(
        `Failed to create content "${contentId}": status ${resp.status}, body ${resp.body.slice(0, 300)}`
      );
    }
    if (isDuplicate) {
      const patchHeaders: Record<string, string> = {
        'content-type': 'application/json',
        authorization: `Bearer ${adminAuthToken}`,
      };
      const { statusCode: patchStatus, body: patchBody } = await request(
        `${baseUrl}/db/content/${contentId}`,
        { method: 'PATCH', headers: patchHeaders, body: JSON.stringify({ reach }) }
      );
      const patchText = await patchBody.text();
      assert.ok(
        patchStatus >= 200 && patchStatus < 300,
        `Content "${contentId}" exists but PATCH reach failed: ` +
          `status ${patchStatus}, body ${patchText.slice(0, 300)}`
      );
    }
    this.contentIds.set('intimateContentId', contentId);
  }
);

/**
 * Assert/ensure the named human has NO intimate relationship with the other
 * named human. If a test-created intimate row exists between them, delete it so
 * the steward-scope-excludes-him invariant holds on rerun.
 */
Given(
  'human {string} has no intimate relationship with {string}',
  async function (this: E2EWorld, nameA: string, nameB: string) {
    if (!isHumanDeployed(nameA) || !isHumanDeployed(nameB)) return 'pending';
    const baseUrl = firstDoorwayUrl(this);
    const partyAId = humanIdOf(this, nameA);
    const partyBId = humanIdOf(this, nameB);
    const adminAuthToken = await adminTokenFor(this, baseUrl);
    const headers: Record<string, string> = { authorization: `Bearer ${adminAuthToken}` };

    // Delete any intimate row between the two parties (test-created drift).
    const offending = (await listRelationships(baseUrl, adminAuthToken, partyAId)).filter(r =>
      isIntimateBetween(r, partyAId, partyBId)
    );
    for (const rel of offending) {
      if (!rel.id) continue;
      await request(`${baseUrl}/db/human-relationships/${rel.id}`, { method: 'DELETE', headers });
    }

    // Assert absence after cleanup.
    const stillThere = (await listRelationships(baseUrl, adminAuthToken, partyAId)).some(r =>
      isIntimateBetween(r, partyAId, partyBId)
    );
    assert.ok(
      !stillThere,
      `Expected NO intimate relationship between "${nameA}" and "${nameB}", but one remains`
    );
  }
);

// ---------------------------------------------------------------------------
// When — a persona requests content
// ---------------------------------------------------------------------------

When(
  '{string} requests content {string}',
  async function (this: E2EWorld, personaName: string, contentId: string) {
    if (!isHumanDeployed(personaName)) return 'pending';
    const baseUrl = firstDoorwayUrl(this);
    const token = tokenOf(this, personaName);
    const resp = await getContentAs(baseUrl, contentId, token);
    const state = intimateState(this);
    state.personaResponses ??= new Map<string, PersonaResponse>();
    state.personaResponses.set(personaName, resp);
  }
);

// ---------------------------------------------------------------------------
// Then — serve / refuse assertions
// ---------------------------------------------------------------------------

Then('the content is served to {string}', function (this: E2EWorld, personaName: string) {
  const state = intimateState(this);
  const resp = state.personaResponses?.get(personaName);
  assert.ok(
    resp,
    `No captured response for "${personaName}" — run a "requests content" step first`
  );
  assert.equal(
    resp.status,
    200,
    `Expected intimate content to be SERVED (200) to "${personaName}", got ${resp.status}. ` +
      `Body: ${resp.body.slice(0, 300)}`
  );
});

Then(
  'the request by {string} is refused with status {int}',
  function (this: E2EWorld, personaName: string, expectedStatus: number) {
    const state = intimateState(this);
    const resp = state.personaResponses?.get(personaName);
    assert.ok(
      resp,
      `No captured response for "${personaName}" — run a "requests content" step first`
    );
    assert.equal(
      resp.status,
      expectedStatus,
      `Expected request by "${personaName}" to be REFUSED with ${expectedStatus}, got ${resp.status}. ` +
        `Body: ${resp.body.slice(0, 300)}`
    );
  }
);
