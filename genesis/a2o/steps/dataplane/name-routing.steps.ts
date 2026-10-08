/**
 * A doorway answers for a public name only when it is one of that name's
 * members (@concern:served-under-standing, @act:i, campaign story 2.2).
 *
 * Three homes are read, each for what it alone knows:
 *   - the MEMBERSHIP DOCUMENTS (`$MESH_DIR/membership/<name>.json`, written by
 *     relay-addr-beacon's `file` sink) — who is eligible for which name, and in
 *     which order (the first member is the name's holder);
 *   - each doorway's own FEDERATION IDENTITY at `/api/v1/federation/coherence`
 *     (`doorwayId`) — the value `x-elohim-served-by` must name. Re-deriving it
 *     from hc-mesh.sh's DOORWAY_ID conventions would make a second home for a
 *     value the running process already states;
 *   - the doorway's ANSWER to `GET /` (and `/version.json`) sent to its own
 *     address with the public name in `Host` — no ingress in the path.
 *
 * The step phrases are deliberately distinct from
 * steps/federation/name-routing.steps.ts (site-path relay): cucumber step
 * definitions are global across the suite.
 */

import { strict as assert } from 'node:assert';
import { readFile } from 'node:fs/promises';
import { join } from 'node:path';

import { Given, Then, When } from '@cucumber/cucumber';

import { getRawWithHeaders, type RawHttpResponse } from '../../src/framework/dataplane/surfaces.js';
import {
  householdMeshDir,
  loadHouseholdMeshFixture,
  membershipLanes,
  requireFixtureDoorwayUrl,
  requireMembershipAuthority,
  type HouseholdMeshFixture,
} from '../../src/framework/fixtures/household-mesh.js';
import { E2EWorld } from '../../src/framework/world.js';

const FETCH_TIMEOUT_MS = 30_000;
const SERVED_BY = 'x-elohim-served-by';
const NAME_ROUTE = 'x-elohim-name-route';

/** The fixture doorway id -> the letter `just mesh doorway-restart` takes. */
const RESTART_LETTER: Readonly<Record<string, string>> = { alpha: 'a', apex: 'b', gamma: 'c' };

interface MembershipMember {
  owner: string;
  origin: string;
}

interface MembershipDocument {
  name: string;
  members: MembershipMember[];
}

interface NameRoutingState {
  fixture: HouseholdMeshFixture;
  /** Public name -> the document this scenario read for it. */
  documents: Map<string, MembershipDocument>;
  /** Fixture doorway id -> its self-reported federation id. */
  federationIds: Map<string, string>;
  lastAnswer?: { doorway: string; name: string; response: RawHttpResponse };
}

const states = new WeakMap<E2EWorld, NameRoutingState>();

function stateOf(world: E2EWorld): NameRoutingState {
  let state = states.get(world);
  if (!state) {
    state = {
      fixture: loadHouseholdMeshFixture(),
      documents: new Map(),
      federationIds: new Map(),
    };
    states.set(world, state);
  }
  return state;
}

function lastAnswer(world: E2EWorld): NonNullable<NameRoutingState['lastAnswer']> {
  const answer = stateOf(world).lastAnswer;
  assert.ok(answer, 'no visitor request has been made in this scenario yet');
  return answer;
}

function sameOrigin(a: string, b: string): boolean {
  const norm = (s: string) => {
    let out = s.trim().toLowerCase();
    while (out.endsWith('/')) out = out.slice(0, -1);
    return out;
  };
  return norm(a) === norm(b);
}

function doorwayUrl(state: NameRoutingState, doorway: string): string {
  return requireFixtureDoorwayUrl(state.fixture, doorway);
}

/** The membership document for `name`, read from the path the household declares for it. */
async function readDocument(state: NameRoutingState, name: string): Promise<MembershipDocument> {
  requireMembershipAuthority(state.fixture);
  const lane = membershipLanes(state.fixture).find(l => l.publicName === name);
  assert.ok(
    lane,
    `the household declares no membership lane for "${name}" (lanes: ` +
      `${membershipLanes(state.fixture)
        .map(l => l.publicName)
        .join(', ')})`
  );
  let raw: string;
  try {
    raw = await readFile(lane.membershipFile, 'utf8');
  } catch (error) {
    throw new Error(
      `the membership document for "${name}" is missing (${lane.membershipFile}): ${String(error)}`
    );
  }
  const doc = JSON.parse(raw) as Partial<MembershipDocument>;
  assert.ok(Array.isArray(doc.members), `${lane.membershipFile} carries no members array`);
  assert.equal(doc.name, name, `${lane.membershipFile} names "${String(doc.name)}", not "${name}"`);
  const read = { name, members: doc.members };
  state.documents.set(name, read);
  return read;
}

async function isMember(state: NameRoutingState, doorway: string, name: string): Promise<boolean> {
  const doc = state.documents.get(name) ?? (await readDocument(state, name));
  const origin = doorwayUrl(state, doorway);
  return doc.members.some(member => sameOrigin(member.origin, origin));
}

/** The doorway's own statement of its federation identity. */
async function federationIdOf(state: NameRoutingState, doorway: string): Promise<string> {
  const known = state.federationIds.get(doorway);
  if (known) return known;
  const url = `${doorwayUrl(state, doorway)}/api/v1/federation/coherence`;
  const res = await getRawWithHeaders(url, { timeoutMs: FETCH_TIMEOUT_MS });
  assert.equal(res.status, 200, `doorway "${doorway}" GET ${url} answered ${res.status}`);
  const id = (JSON.parse(res.text) as { doorwayId?: unknown }).doorwayId;
  assert.ok(
    typeof id === 'string' && id.length > 0,
    `doorway "${doorway}" states no doorwayId at ${url}`
  );
  state.federationIds.set(doorway, id);
  return id;
}

/** GET `path` from one doorway's own address, asking for `name` in Host. */
async function askUnderName(
  state: NameRoutingState,
  doorway: string,
  name: string,
  path: string
): Promise<RawHttpResponse> {
  return getRawWithHeaders(`${doorwayUrl(state, doorway)}${path}`, {
    timeoutMs: FETCH_TIMEOUT_MS,
    headers: { Host: name, Accept: 'text/html,application/json' },
  });
}

function stampOf(text: string): string {
  try {
    const parsed = JSON.parse(text) as { commit?: unknown };
    return typeof parsed.commit === 'string' ? parsed.commit : '';
  } catch {
    return '';
  }
}

// ---------------------------------------------------------------------------
// Background
// ---------------------------------------------------------------------------

Given(
  "the household's membership documents list doorways {string} and {string} as members of {string} and {string}",
  { timeout: 30_000 },
  async function (this: E2EWorld, first: string, second: string, nameA: string, nameB: string) {
    const state = stateOf(this);
    for (const name of [nameA, nameB]) {
      await readDocument(state, name);
      for (const doorway of [first, second]) {
        assert.ok(
          await isMember(state, doorway, name),
          `doorway "${doorway}" (${doorwayUrl(state, doorway)}) is not listed in the membership ` +
            `document for "${name}" — members: ${JSON.stringify(state.documents.get(name)?.members)}. ` +
            'Its beacon leg withdraws it while it does not serve; wait for it to rejoin.'
        );
      }
    }
  }
);

Given(
  "doorways {string}, {string} and {string} each read public-name membership from the household's membership directory",
  { timeout: 60_000 },
  async function (this: E2EWorld, a: string, b: string, c: string) {
    const state = stateOf(this);
    const name = requireMembershipAuthority(state.fixture).publicName;
    for (const doorway of [a, b, c]) {
      const res = await askUnderName(state, doorway, name, '/');
      const letter = RESTART_LETTER[doorway] ?? '<letter>';
      assert.ok(
        res.headers[SERVED_BY],
        `doorway "${doorway}" answered "${name}" (HTTP ${res.status}) with no ${SERVED_BY} header: ` +
          'it was started without DOORWAY_MEMBERSHIP_DIR, so it reads no membership documents. ' +
          '`just mesh start` passes it on every doorway launch; for a doorway started before that, ' +
          `MESH_DOORWAY_ENV_SET="DOORWAY_MEMBERSHIP_DIR=$MESH_DIR/membership" just mesh doorway-restart ${letter}`
      );
    }
  }
);

// ---------------------------------------------------------------------------
// Givens
// ---------------------------------------------------------------------------

Given(
  'doorway {string} is the holder of {string}',
  { timeout: 30_000 },
  async function (this: E2EWorld, doorway: string, name: string) {
    const state = stateOf(this);
    const doc = await readDocument(state, name);
    const holder = doc.members[0];
    assert.ok(holder, `the membership document for "${name}" lists no members`);
    assert.ok(
      sameOrigin(holder.origin, doorwayUrl(state, doorway)),
      `the holder of "${name}" is its first member, ${holder.owner} at ${holder.origin}, ` +
        `not doorway "${doorway}" (${doorwayUrl(state, doorway)})`
    );
  }
);

/**
 * The public names a running doorway declares it stands for: `DOORWAY_PUBLIC_NAMES`
 * in its own process environment. Read from `/proc`, the same captured environment
 * `just mesh doorway-restart` re-execs it with — the doorway states it on no HTTP surface.
 */
async function standsFor(doorway: string): Promise<string[]> {
  const letter = RESTART_LETTER[doorway];
  assert.ok(letter, `no household pid ledger letter for doorway "${doorway}"`);
  const ledger = join(householdMeshDir(), 'pids', `doorway-${letter}`);
  const pid = (await readFile(ledger, 'utf8')).trim().split(/\s+/)[0];
  assert.ok(pid, `the pid ledger ${ledger} names no process`);
  const environ = await readFile(`/proc/${pid}/environ`, 'utf8');
  const entry = environ.split('\0').find(item => item.startsWith('DOORWAY_PUBLIC_NAMES='));
  return (entry?.slice('DOORWAY_PUBLIC_NAMES='.length) ?? '')
    .split(',')
    .map(name => name.trim().toLowerCase())
    .filter(name => name.length > 0);
}

Given(
  'doorway {string} stands for no household name',
  { timeout: 30_000 },
  async function (this: E2EWorld, doorway: string) {
    const state = stateOf(this);
    const names = membershipLanes(state.fixture).map(lane => lane.publicName);
    assert.ok(names.length > 0, 'the household declares no public names');
    const declared = await standsFor(doorway);
    for (const name of names) {
      assert.ok(
        !declared.includes(name.toLowerCase()),
        `doorway "${doorway}" declares it stands for "${name}" (DOORWAY_PUBLIC_NAMES=${declared.join(',')})`
      );
      await readDocument(state, name);
      assert.ok(
        !(await isMember(state, doorway, name)),
        `doorway "${doorway}" is listed in the membership document for "${name}"`
      );
    }
  }
);

// ---------------------------------------------------------------------------
// The request
// ---------------------------------------------------------------------------

When(
  "a visitor asks doorway {string} for {string} at that doorway's own address",
  { timeout: 60_000 },
  async function (this: E2EWorld, doorway: string, name: string) {
    const state = stateOf(this);
    const response = await askUnderName(state, doorway, name, '/');
    state.lastAnswer = { doorway, name, response };
  }
);

// ---------------------------------------------------------------------------
// Thens
// ---------------------------------------------------------------------------

Then(
  "the answer is HTTP 200 and contains the landing page's app-root element, where the site's app starts",
  function (this: E2EWorld) {
    const marker = 'app-root';
    const { doorway, name, response } = lastAnswer(this);
    assert.equal(
      response.status,
      200,
      `doorway "${doorway}" answered "${name}" with HTTP ${response.status}: ${response.text.slice(0, 300)}`
    );
    assert.ok(
      response.text.includes(marker),
      `doorway "${doorway}"'s answer for "${name}" carries no "${marker}"`
    );
  }
);

Then('the answer is HTTP 421 Misdirected Request', function (this: E2EWorld) {
  const { doorway, name, response } = lastAnswer(this);
  assert.equal(
    response.status,
    421,
    `doorway "${doorway}" answered "${name}" with HTTP ${response.status}, not 421: ` +
      response.text.slice(0, 300)
  );
});

Then(
  "the answer's x-elohim-served-by header is the federation id doorway {string} reports for itself",
  { timeout: 30_000 },
  async function (this: E2EWorld, expected: string) {
    const state = stateOf(this);
    const { doorway, name, response } = lastAnswer(this);
    const id = await federationIdOf(state, expected);
    assert.equal(
      response.headers[SERVED_BY],
      id,
      `doorway "${doorway}"'s answer for "${name}" names ${SERVED_BY}=` +
        `${String(response.headers[SERVED_BY])}; doorway "${expected}" calls itself "${id}"`
    );
  }
);

Then('the answer carries no x-elohim-name-route header', function (this: E2EWorld) {
  const { doorway, name, response } = lastAnswer(this);
  assert.equal(
    response.headers[NAME_ROUTE],
    undefined,
    `doorway "${doorway}" handed "${name}" on (${NAME_ROUTE}: ${String(response.headers[NAME_ROUTE])}) ` +
      'instead of answering it itself'
  );
});

Then(
  'doorway {string} serves the same build stamp under {string} as doorway {string} does',
  { timeout: 60_000 },
  async function (this: E2EWorld, member: string, name: string, holder: string) {
    const state = stateOf(this);
    const read = async (doorway: string) => {
      const res = await askUnderName(state, doorway, name, '/version.json');
      assert.equal(
        res.status,
        200,
        `doorway "${doorway}" GET /version.json under "${name}" answered ${res.status}`
      );
      const stamp = stampOf(res.text);
      assert.ok(stamp, `doorway "${doorway}"'s /version.json under "${name}" carries no commit`);
      return stamp;
    };
    const [memberStamp, holderStamp] = [await read(member), await read(holder)];
    assert.equal(
      memberStamp,
      holderStamp,
      `under "${name}" doorway "${member}" serves build stamp ${memberStamp}, ` +
        `the holder "${holder}" serves ${holderStamp}`
    );
  }
);

Then(
  'the refusal names every doorway the membership document lists for {string}',
  { timeout: 30_000 },
  async function (this: E2EWorld, name: string) {
    const state = stateOf(this);
    const { doorway, response } = lastAnswer(this);
    const doc = await readDocument(state, name);
    let body: { name?: unknown; members?: { owner?: unknown; origin?: unknown }[] };
    try {
      body = JSON.parse(response.text) as typeof body;
    } catch {
      throw new Error(`doorway "${doorway}"'s refusal is not JSON: ${response.text.slice(0, 300)}`);
    }
    assert.equal(body.name, name, `the refusal names "${String(body.name)}", not "${name}"`);
    const byText = (a: string, b: string) => a.localeCompare(b);
    const named = (body.members ?? [])
      .map(m => `${String(m.owner)}@${String(m.origin)}`)
      .sort(byText);
    const listed = doc.members.map(m => `${m.owner}@${m.origin}`).sort(byText);
    assert.deepEqual(
      named,
      listed,
      `the refusal names members [${named.join(', ')}]; the document lists [${listed.join(', ')}]`
    );
  }
);

interface AdvertisedHead {
  urlPath: string;
  eprId: string;
  hostnames?: string[];
  /** The bundle the doorway serves at this mount (`servedBundle` on the wire). */
  servedBundle?: string;
  /** The same value under the name an older doorway still writes mid-roll. */
  declaredHead?: string;
}

/** The bundle a coherence head says its doorway serves, under either wire name. */
function servedBundleOf(head: AdvertisedHead): string | undefined {
  return head.servedBundle ?? head.declaredHead;
}

/** Every head a doorway's coherence endpoint advertises. */
async function advertisedHeads(
  state: NameRoutingState,
  doorway: string
): Promise<AdvertisedHead[]> {
  const url = `${doorwayUrl(state, doorway)}/api/v1/federation/coherence`;
  const res = await getRawWithHeaders(url, { timeoutMs: FETCH_TIMEOUT_MS });
  assert.equal(res.status, 200, `doorway "${doorway}" GET ${url} answered ${res.status}`);
  return (JSON.parse(res.text) as { heads?: AdvertisedHead[] }).heads ?? [];
}

/**
 * The landing bundle a doorway says it serves for `name` at `/`: the head its
 * coherence endpoint advertises for the most specific mount covering `/` —
 * bound to `name` first, then any-host. This is the value the doorway's routing
 * rule compares against the holder's.
 */
async function landingBundleFor(
  state: NameRoutingState,
  doorway: string,
  name: string
): Promise<AdvertisedHead> {
  const heads = await advertisedHeads(state, doorway);
  const atRoot = heads.filter(head => head.urlPath === '/');
  const bound = atRoot.find(head => (head.hostnames ?? []).includes(name));
  const head = bound ?? atRoot.find(head => (head.hostnames ?? []).length === 0);
  assert.ok(head, `doorway "${doorway}" advertises no mount at "/" for "${name}"`);
  assert.ok(
    servedBundleOf(head),
    `doorway "${doorway}" advertises no landing bundle for "${name}" at "/" (EPR ${head.eprId})`
  );
  return head;
}

Then(
  'the landing bundle doorway {string} serves under {string} is the one doorway {string} serves for that name',
  { timeout: 30_000 },
  async function (this: E2EWorld, member: string, name: string, holder: string) {
    const state = stateOf(this);
    const mine = await landingBundleFor(state, member, name);
    const theirs = await landingBundleFor(state, holder, name);
    assert.equal(
      mine.eprId,
      theirs.eprId,
      `under "${name}" doorway "${member}" mounts EPR ${mine.eprId}, the holder "${holder}" mounts ${theirs.eprId}`
    );
    assert.equal(
      servedBundleOf(mine),
      servedBundleOf(theirs),
      `under "${name}" doorway "${member}" serves bundle ${String(servedBundleOf(mine))}, ` +
        `the holder "${holder}" serves ${String(servedBundleOf(theirs))}`
    );
  }
);

/**
 * THE RELAY WINDOW, named. Each doorway takes up a newly declared landing
 * bundle on its own clock: its bundle-heads reconciler re-reads storage every
 * `BUNDLE_HEADS_TICK_SECS` (30 s, doorway render/bundle_heads.rs) and the
 * pair learns each other's advertisement on the next federation discovery
 * tick. Until both have, a listed non-holder serves a different bundle from
 * the holder and correctly hands the request on — which is not what the
 * scenarios below are about. 120 s = the reconcile tick plus one discovery
 * tick, with margin; past it the pair has not converged and that is the
 * finding. No wire change: the existing coherence endpoint is the read.
 */
const LANDING_CONVERGENCE_BOUND_MS = 120_000;
const LANDING_CONVERGENCE_POLL_MS = 5_000;

/** `eprId|sorted hostnames` -> served bundle, for every head a doorway advertises at `/`. */
async function landingBundles(
  state: NameRoutingState,
  doorway: string
): Promise<Map<string, string | undefined>> {
  const atRoot = (await advertisedHeads(state, doorway)).filter(head => head.urlPath === '/');
  return new Map(
    atRoot.map(head => [
      `${head.eprId}|${[...(head.hostnames ?? [])].sort((x, y) => x.localeCompare(y)).join(',')}`,
      servedBundleOf(head),
    ])
  );
}

Given(
  'doorways {string} and {string} have converged on the bundle each serves at the landing mount',
  { timeout: LANDING_CONVERGENCE_BOUND_MS + 30_000 },
  async function (this: E2EWorld, first: string, second: string) {
    const state = stateOf(this);
    const deadline = Date.now() + LANDING_CONVERGENCE_BOUND_MS;
    let last = '';
    for (;;) {
      const [a, b] = await Promise.all([
        landingBundles(state, first),
        landingBundles(state, second),
      ]);
      const mounts = [...new Set([...a.keys(), ...b.keys()])].sort((x, y) => x.localeCompare(y));
      const apart = mounts.filter(key => !a.get(key) || a.get(key) !== b.get(key));
      if (mounts.length > 0 && apart.length === 0) return;
      last =
        mounts.length === 0
          ? 'neither doorway advertises a mount at "/"'
          : apart
              .map(
                key =>
                  `${key}: "${first}" serves ${String(a.get(key))}, "${second}" serves ${String(b.get(key))}`
              )
              .join('; ');
      if (Date.now() >= deadline) break;
      await new Promise(resolve => setTimeout(resolve, LANDING_CONVERGENCE_POLL_MS));
    }
    assert.fail(
      `doorways "${first}" and "${second}" did not converge on one landing bundle within ` +
        `${LANDING_CONVERGENCE_BOUND_MS / 1000} s (one bundle-heads tick plus one discovery tick, ` +
        `with margin): ${last}`
    );
  }
);
