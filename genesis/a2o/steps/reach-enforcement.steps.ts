/**
 * Reach-enforcement step definitions — the HTTP egress half of the
 * `reach-enforced-everywhere` habit.
 *
 * WHY THIS EXISTS. On 2026-08-20 the content LIST route decided reach filtering
 * from header PRESENCE rather than validation: `has_auth =
 * headers.get(AUTHORIZATION).is_some() || headers.get("X-Agent-Id").is_some()`.
 * Measured live on doorway-alpha, an anonymous listing returned 90 rows while
 * the identical request carrying the literal token `bogus` returned 1000 —
 * including private and intimate rows, each with its content body. The defect
 * was invisible for months because reach carries no `@dataplane` tag anywhere
 * in this suite, so the Dataplane Validation stage never measured this plane.
 *
 * THE ASSERTION IS A RELATION, NOT A TIER. `elohim-storage/CLAUDE.md` forbids
 * canonizing a single reach vocabulary while the multi-vocabulary drift is
 * open, so these steps never name a tier. They assert the property that must
 * hold under EVERY vocabulary: presenting a credential the substrate cannot
 * verify grants nothing beyond presenting none at all. That relation is
 * drift-proof, and it is exactly what the bypass violated.
 *
 * WHAT TURNS THIS RED. Reinstate any header-presence check on an egress route
 * — restore `has_auth` on the listing, or add an equivalent inline
 * `matches!`/`is_some()` gate on a new route — and the unverifiable-credential
 * listing grows beyond the anonymous one. The diagnostic names the rows that
 * leaked so the failure reads as a finding, not a count mismatch.
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { resolvePeerUrl, getRaw } from '../src/framework/dataplane/surfaces.js';
import { E2EWorld } from '../src/framework/world.js';

/** A listing captured under one credential posture. */
interface Listing {
  label: string;
  status: number;
  ids: string[];
  /** id -> reach, for diagnostics only — never asserted on by name. */
  reachById: Map<string, string>;
  /** id -> body length. Compared, so a same-ids-but-fuller-bodies leak fails too. */
  bodyLenById: Map<string, number>;
  bodiesPresent: number;
}

const listings = new WeakMap<E2EWorld, Map<string, Listing>>();
const peerUrls = new WeakMap<E2EWorld, Map<string, string>>();

const LISTING_PATH = '/db/content?limit=1000&offset=1000';

function peerMap(world: E2EWorld): Map<string, string> {
  let m = peerUrls.get(world);
  if (!m) {
    m = new Map();
    peerUrls.set(world, m);
  }
  return m;
}

function listingMap(world: E2EWorld): Map<string, Listing> {
  let m = listings.get(world);
  if (!m) {
    m = new Map();
    listings.set(world, m);
  }
  return m;
}

/**
 * Parse a listing response into the shape we assert on. A non-JSON or
 * unexpected body yields an EMPTY listing rather than throwing, so a shed or
 * an error surfaces as a status assertion rather than a parse crash.
 */
function parseListing(label: string, status: number, text: string): Listing {
  const listing: Listing = {
    label,
    status,
    ids: [],
    reachById: new Map(),
    bodyLenById: new Map(),
    bodiesPresent: 0,
  };
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    return listing;
  }
  const items =
    parsed && typeof parsed === 'object' && Array.isArray((parsed as { items?: unknown }).items)
      ? ((parsed as { items: unknown[] }).items as Record<string, unknown>[])
      : [];
  for (const row of items) {
    const id = typeof row.id === 'string' ? row.id : undefined;
    if (!id) continue;
    listing.ids.push(id);
    const reach = typeof row.reach === 'string' ? row.reach : '(absent)';
    listing.reachById.set(id, reach);
    const body = row.contentBody;
    const len = typeof body === 'string' ? body.length : 0;
    listing.bodyLenById.set(id, len);
    if (len > 0) listing.bodiesPresent += 1;
  }
  listing.ids.sort((a, b) => a.localeCompare(b));
  return listing;
}

async function captureListing(
  world: E2EWorld,
  peerName: string,
  label: string,
  headers?: Record<string, string>
): Promise<void> {
  const base = peerMap(world).get(peerName) ?? resolvePeerUrl(peerName);
  peerMap(world).set(peerName, base);
  const { status, text } = await getRaw(`${base}${LISTING_PATH}`, headers ? { headers } : {});
  listingMap(world).set(label, parseListing(label, status, text));
}

function requireListing(world: E2EWorld, label: string): Listing {
  const listing = listingMap(world).get(label);
  assert.ok(listing, `No "${label}" listing captured — the When step must run first.`);
  return listing;
}

Given('peer {string} is reachable for reach probes', function (this: E2EWorld, peerName: string) {
  peerMap(this).set(peerName, resolvePeerUrl(peerName));
});

/**
 * NON-VACUITY CONTROL. Every assertion below is of the form "the credentialled
 * listing is no larger than the anonymous one" — which is trivially true if the
 * corpus holds nothing beyond anonymous reach. Without this guard the whole
 * feature would go green on an all-commons corpus while enforcing nothing.
 *
 * `contentId` must name a row the substrate refuses to an anonymous caller. A
 * 200 here means either the control row's reach was widened or enforcement is
 * off — both make the scenarios below meaningless, so this fails loudly rather
 * than skipping.
 */
Given(
  'peer {string} holds a restricted-reach fixture an anonymous caller is refused, named {string}',
  async function (this: E2EWorld, peerName: string, contentId: string) {
    const base = peerMap(this).get(peerName) ?? resolvePeerUrl(peerName);
    peerMap(this).set(peerName, base);
    const { status } = await getRaw(`${base}/db/content/${contentId}`);
    assert.notStrictEqual(
      status,
      200,
      `NON-VACUITY CONTROL FAILED: "${contentId}" was served to an anonymous caller ` +
        `(status ${status}). Either that row is no longer beyond anonymous reach — pick ` +
        `another control id — or the single-item route has stopped enforcing, in which ` +
        `case the scenarios below would pass while proving nothing.`
    );
  }
);

When(
  'I list content on peer {string} anonymously',
  async function (this: E2EWorld, peerName: string) {
    await captureListing(this, peerName, 'anonymous');
  }
);

/**
 * `headerSpec` is a raw `Name: value` pair so the feature file reads as the
 * literal wire request an attacker would send.
 */
When(
  'I list content on peer {string} presenting header {string}',
  async function (this: E2EWorld, peerName: string, headerSpec: string) {
    const idx = headerSpec.indexOf(':');
    assert.ok(idx > 0, `Header must be "Name: value", got "${headerSpec}"`);
    const name = headerSpec.slice(0, idx).trim();
    const value = headerSpec.slice(idx + 1).trim();
    await captureListing(this, peerName, 'presented', { [name]: value });
  }
);

Then('both listings answered 200', function (this: E2EWorld) {
  for (const label of ['anonymous', 'presented']) {
    const l = requireListing(this, label);
    assert.strictEqual(l.status, 200, `${label} listing: status ${l.status}, expected 200`);
  }
});

/**
 * THE HABIT'S ASSERTION. An unverifiable credential must grant nothing beyond
 * no credential. Stated as set equality so it names no reach tier and holds
 * under any vocabulary.
 */
Then(
  'the presented credential returned no rows beyond the anonymous listing',
  function (this: E2EWorld) {
    const anon = requireListing(this, 'anonymous');
    const shown = requireListing(this, 'presented');

    const anonIds = new Set(anon.ids);
    const extra = shown.ids.filter(id => !anonIds.has(id));

    if (extra.length > 0) {
      const byReach = new Map<string, number>();
      for (const id of extra) {
        const r = shown.reachById.get(id) ?? '(absent)';
        byReach.set(r, (byReach.get(r) ?? 0) + 1);
      }
      const breakdown = [...byReach.entries()]
        .sort((a, b) => b[1] - a[1])
        .map(([reach, n]) => `${reach}=${n}`)
        .join(', ');
      const sample = extra.slice(0, 3).map(id => `${id} (${shown.reachById.get(id)})`);
      assert.fail(
        `REACH BYPASS: an unverifiable credential yielded ${extra.length} rows an ` +
          `anonymous caller does not receive (anonymous=${anon.ids.length}, ` +
          `presented=${shown.ids.length}).\n` +
          `  leaked by reach: ${breakdown}\n` +
          `  sample: ${sample.join(', ')}\n` +
          `  ${shown.bodiesPresent} of ${shown.ids.length} returned rows carried a content body.\n` +
          `  An egress route is deciding authorization from header PRESENCE rather than ` +
          `validation — see elohim-storage/src/http.rs and the 2026-08-20 run:correction.`
      );
    }

    assert.deepStrictEqual(
      shown.ids,
      anon.ids,
      'Listings differ without the presented set being a superset — investigate before assuming a fix.'
    );
  }
);

/**
 * The SECOND dimension, asserted separately so the Gherkin shows both. Same rows
 * is not the same answer: a route could return the anonymous id set with fuller
 * bodies attached, which is still a leak.
 */
Then(
  'the presented credential returned no fuller content than the anonymous listing',
  function (this: E2EWorld) {
    const anon = requireListing(this, 'anonymous');
    const shown = requireListing(this, 'presented');
    const fuller = anon.ids.filter(
      id => (shown.bodyLenById.get(id) ?? 0) !== (anon.bodyLenById.get(id) ?? 0)
    );
    assert.deepStrictEqual(
      fuller,
      [],
      `REACH BYPASS (body-level): ${fuller.length} row(s) returned a different amount of ` +
        `content to the credential-bearing caller than to the anonymous one, despite an ` +
        `identical row set. Sample: ${fuller.slice(0, 3).join(', ')}`
    );
  }
);
