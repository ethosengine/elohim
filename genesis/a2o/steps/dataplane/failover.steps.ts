/**
 * Step glue for the doorway-failover concern
 * (features/dataplane/doorway-failover.feature) — the "two doorways, one
 * name" invariant. See that feature's header comment for the full
 * serving/shedding/dead vocabulary; classifyDoorwayState() in
 * src/framework/dataplane/surfaces.ts is the single implementation of it.
 *
 * Three steps live here:
 *   1. `Then doorway {string} classifies as serving or shedding, not dead`
 *      — the honest-classification bar. Fails ONLY on 'dead'; shedding is a
 *      pass (it is the specified degraded contract, not an outage).
 *   2. `Then at least one of doorways {string} and {string} is serving`
 *      — the pair-floor bar. Passes iff either peer classifies 'serving'.
 *   3. `Then every serving doorway among {string} and {string} resolves the
 *      same declared head for content {string}` — the "whoever serves,
 *      serves the same truth" bar. Classifies both peers, then reads
 *      probeDeclaredHead() (surfaces.ts — shared with the ch10 comparator in
 *      resiliency-saga.steps.ts) on every peer classified 'serving'. Zero
 *      serving peers is a FAIL here (the pair-floor scenario above is the
 *      one that owns "at least one serves" — this step must never
 *      vacuously pass just because nobody was serving).
 *
 * Peer resolution goes straight through resolvePeerUrl() (not
 * world.getDoorway()) — these steps run standalone against the "alpha-A" /
 * "elohim.host" aliases the feature's Background registers by the same
 * names, so no per-scenario Given step is required for classification alone.
 */

import { strict as assert } from 'node:assert';

import { Then } from '@cucumber/cucumber';

import {
  classifyDoorwayState,
  probeDeclaredHead,
  resolvePeerUrl,
  CLASSIFY_TIMEOUT_MS,
  type DoorwayState,
} from '../../src/framework/dataplane/surfaces.js';
import { E2EWorld } from '../../src/framework/world.js';

/**
 * classifyDoorwayState issues up to 3 sequential bounded requests (/, then
 * /status.json, then /health on the 503 tie-break path) — worst case
 * ~3 * CLASSIFY_TIMEOUT_MS for ONE peer. Steps that classify two peers in
 * parallel still bound to the same per-peer worst case (Promise.all), plus
 * headroom for the declared-head fetch(es) on whichever peers are serving.
 */
const SINGLE_CLASSIFY_TIMEOUT_MS = 3 * CLASSIFY_TIMEOUT_MS + 15_000;
const PAIR_CLASSIFY_TIMEOUT_MS = 3 * CLASSIFY_TIMEOUT_MS + 15_000;
const PAIR_CLASSIFY_AND_HEAD_TIMEOUT_MS = 3 * CLASSIFY_TIMEOUT_MS + 30_000;

// ---------------------------------------------------------------------------
// 1. Honest classification — shed is not death
// ---------------------------------------------------------------------------

Then(
  'doorway {string} classifies as serving or shedding, not dead',
  { timeout: SINGLE_CLASSIFY_TIMEOUT_MS },
  async function (this: E2EWorld, peerName: string) {
    const peerUrl = resolvePeerUrl(peerName);
    const state = await classifyDoorwayState(peerUrl);
    assert.notStrictEqual(
      state,
      'dead',
      `doorway "${peerName}" (${peerUrl}) classified as dead — connect error/timeout on BOTH / and /health; ` +
        'a healthy or shedding doorway must answer at least one of them'
    );
  }
);

// ---------------------------------------------------------------------------
// 2. The pair floor — at least one doorway is serving
// ---------------------------------------------------------------------------

Then(
  'at least one of doorways {string} and {string} is serving',
  { timeout: PAIR_CLASSIFY_TIMEOUT_MS },
  async function (this: E2EWorld, peerA: string, peerB: string) {
    const urlA = resolvePeerUrl(peerA);
    const urlB = resolvePeerUrl(peerB);
    const [stateA, stateB] = await Promise.all([
      classifyDoorwayState(urlA),
      classifyDoorwayState(urlB),
    ]);
    assert.ok(
      stateA === 'serving' || stateB === 'serving',
      `neither doorway is serving — "${peerA}"=${stateA}, "${peerB}"=${stateB}. ` +
        'A correlated shed/outage across the whole pair means no one can reach the commons at all.'
    );
  }
);

// ---------------------------------------------------------------------------
// 3. Whoever serves, serves the same declared truth
// ---------------------------------------------------------------------------

Then(
  'every serving doorway among {string} and {string} resolves the same declared head for content {string}',
  { timeout: PAIR_CLASSIFY_AND_HEAD_TIMEOUT_MS },
  async function (this: E2EWorld, peerA: string, peerB: string, contentId: string) {
    const peers = [
      { name: peerA, url: resolvePeerUrl(peerA) },
      { name: peerB, url: resolvePeerUrl(peerB) },
    ];

    const states: DoorwayState[] = await Promise.all(
      peers.map(async peer => classifyDoorwayState(peer.url))
    );
    const servingPeers = peers.filter((_, i) => states[i] === 'serving');

    assert.ok(
      servingPeers.length > 0,
      `no doorway among "${peerA}" and "${peerB}" is serving (${peerA}=${states[0]}, ${peerB}=${states[1]}) — ` +
        'the pair-floor scenario owns "at least one serves"; this step must not vacuously pass on zero serving peers'
    );

    const heads = await Promise.all(
      servingPeers.map(async peer => {
        const head = await probeDeclaredHead(peer.url, contentId);
        assert.ok(
          head.declared,
          `"${peer.name}" is classified serving but reports declared=false for "${contentId}" — ` +
            'no canonical declaration for a serving doorway to have resolved'
        );
        assert.ok(
          typeof head.headActionHash === 'string' && head.headActionHash.length > 0,
          `"${peer.name}" is classified serving but returned a null/empty declared head for "${contentId}"`
        );
        return { name: peer.name, headActionHash: head.headActionHash };
      })
    );

    if (heads.length === 2) {
      assert.strictEqual(
        heads[0].headActionHash,
        heads[1].headActionHash,
        `declared head diverges between serving doorways for "${contentId}": ` +
          `"${heads[0].name}"=${heads[0].headActionHash} vs "${heads[1].name}"=${heads[1].headActionHash} — ` +
          'failover changed the answer'
      );
    }
    // heads.length === 1: the sole serving peer's own declared head is
    // sufficient — nothing to compare it against.
  }
);
