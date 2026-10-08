/**
 * The head answer names the election that chose it.
 *
 * Concerns: `notary-authority` (features/dataplane/notary-authority.feature)
 * and `content-addressing` (features/delivery/content-addressing.feature).
 *
 * `GET /epr-head/{id}` answers JSON with a `cid` and, beside it, an
 * UNADDRESSED `election` witness: `canonicalDeclaredAt` (the elected
 * declaration's DHT clock, RFC3339 microseconds), `earned` (tier) and
 * `linkHash` (tiebreak). The witness is envelope: it is never in the
 * dag-cbor bytes the `Accept: application/vnd.ipld.dag-cbor` arm serves, so
 * `cid` stays a function of the declared head alone. Two doorways that agree
 * on the head mint one `cid` whatever their projections recorded; two that
 * read the same election show the same witness; a lagging projection shows a
 * different witness over the same `cid`.
 *
 * Source: elohim/elohim-storage/src/epr_head.rs (`compose_head_view`,
 * `derive_epr_head_with_election`); schema
 * elohim/sdk/schemas/v1/views/epr-head-view.schema.json.
 */

import { strict as assert } from 'node:assert';
import { createHash } from 'node:crypto';

import { Then, When } from '@cucumber/cucumber';

import { request } from 'undici';

import { getRawWithHeaders } from '../src/framework/dataplane/surfaces.js';
import { E2EWorld } from '../src/framework/world.js';

const DAG_CBOR = 'application/vnd.ipld.dag-cbor';

interface ElectionWitness {
  canonicalDeclaredAt: string;
  earned: boolean;
  linkHash?: string;
  /** Only on `?election=live`, only when this peer's conductor answered the same election. */
  elector?: string;
}

interface HeadRead {
  peer: string;
  url: string;
  json: { cid?: string; election?: ElectionWitness } & Record<string, unknown>;
  cbor: Buffer;
}

const reads = new WeakMap<E2EWorld, HeadRead[]>();

function baseUrl(world: E2EWorld, name: string): string {
  const doorway = world.doorways.get(name);
  assert.ok(doorway, `No doorway or peer registered as "${name}"`);
  let url = doorway.url;
  while (url.endsWith('/')) url = url.slice(0, -1);
  return url;
}

async function readHead(
  world: E2EWorld,
  peer: string,
  id: string,
  live = false
): Promise<HeadRead> {
  const url = `${baseUrl(world, peer)}/epr-head/${encodeURIComponent(id)}`;
  const json = await getRawWithHeaders(live ? `${url}?election=live` : url, {
    timeoutMs: 15_000,
  });
  assert.equal(json.status, 200, `GET ${url} -> ${json.status}: ${json.text.slice(0, 200)}`);
  // Bytes, not text: a UTF-8 decode would corrupt the addressed dag-cbor.
  const cbor = await request(url, {
    method: 'GET',
    headers: { accept: DAG_CBOR },
    headersTimeout: 15_000,
    bodyTimeout: 15_000,
  });
  const cborBytes = Buffer.from(await cbor.body.arrayBuffer());
  assert.equal(cbor.statusCode, 200, `GET ${url} (dag-cbor) -> ${cbor.statusCode}`);
  return {
    peer,
    url,
    json: JSON.parse(json.text) as HeadRead['json'],
    cbor: cborBytes,
  };
}

/** RFC 4648 base32, lowercase, unpadded — the multibase `b` alphabet. */
function base32(bytes: Buffer): string {
  const alphabet = 'abcdefghijklmnopqrstuvwxyz234567';
  let bits = 0;
  let value = 0;
  let out = '';
  for (const byte of bytes) {
    value = (value << 8) | byte;
    bits += 8;
    while (bits >= 5) {
      out += alphabet[(value >>> (bits - 5)) & 31];
      bits -= 5;
    }
  }
  if (bits > 0) out += alphabet[(value << (5 - bits)) & 31];
  return out;
}

/** CIDv1, dag-cbor (0x71), sha2-256 — the address the envelope's `cid` claims. */
function dagCborCid(bytes: Buffer): string {
  const digest = createHash('sha256').update(bytes).digest();
  return 'b' + base32(Buffer.concat([Buffer.from([0x01, 0x71, 0x12, 0x20]), digest]));
}

function captured(world: E2EWorld): HeadRead[] {
  const r = reads.get(world);
  assert.ok(r && r.length > 0, 'No head answer read yet in this scenario');
  return r;
}

When(
  'the head of EPR {string} is read from doorways {string} and {string}',
  { timeout: 45_000 },
  async function (this: E2EWorld, id: string, peerA: string, peerB: string) {
    reads.set(this, await Promise.all([readHead(this, peerA, id), readHead(this, peerB, id)]));
  }
);

When(
  'the live head of EPR {string} is read from doorways {string} and {string}',
  { timeout: 45_000 },
  async function (this: E2EWorld, id: string, peerA: string, peerB: string) {
    reads.set(
      this,
      await Promise.all([readHead(this, peerA, id, true), readHead(this, peerB, id, true)])
    );
  }
);

When(
  'the head of EPR {string} is read from doorway {string}',
  { timeout: 30_000 },
  async function (this: E2EWorld, id: string, peer: string) {
    reads.set(this, [await readHead(this, peer, id)]);
  }
);

Then('both answers carry the same address', function (this: E2EWorld) {
  const [a, b] = captured(this);
  assert.ok(b, 'This step compares two doorways');
  assert.ok(a.json.cid, `${a.peer} answered without a cid`);
  assert.equal(
    a.json.cid,
    b.json.cid,
    `${a.peer} and ${b.peer} name different heads: ${a.json.cid} vs ${b.json.cid}`
  );
});

Then(
  'both answers name the same election, by its clock and its tiebreak',
  function (this: E2EWorld) {
    const [a, b] = captured(this);
    assert.ok(b, 'This step compares two doorways');
    for (const r of [a, b]) {
      assert.ok(
        r.json.election,
        `${r.peer} answered with no election witness — its projection recorded no election ` +
          `for this head (or the doorway's storage predates the witness)`
      );
    }
    assert.equal(
      a.json.election!.linkHash,
      b.json.election!.linkHash,
      `${a.peer} and ${b.peer} read DIFFERENT elections (tiebreak ${a.json.election!.linkHash} vs ` +
        `${b.json.election!.linkHash})`
    );
    assert.equal(
      a.json.election!.canonicalDeclaredAt,
      b.json.election!.canonicalDeclaredAt,
      `${a.peer} and ${b.peer} hold the same election at different clocks — one projection is stale`
    );
  }
);

Then(
  /^the address in (?:each|the) answer is computed from bytes that do not include the election witness$/,
  function (this: E2EWorld) {
    for (const r of captured(this)) {
      assert.ok(r.json.cid, `${r.peer} answered without a cid`);
      assert.equal(
        dagCborCid(r.cbor),
        r.json.cid,
        `${r.peer}: the JSON cid is not the address of the dag-cbor bytes served at ${r.url}`
      );
      assert.ok(
        !r.cbor.includes(Buffer.from('election')) &&
          !r.cbor.includes(Buffer.from('canonicalDeclaredAt')),
        `${r.peer}: the addressed dag-cbor bytes carry the election witness`
      );
    }
  }
);

Then('both answers name the same elector for the same election', function (this: E2EWorld) {
  const [a, b] = captured(this);
  assert.ok(b, 'This step compares two doorways');
  for (const r of [a, b]) {
    assert.ok(
      r.json.election?.elector,
      `${r.peer} named no elector on a live read — its conductor did not answer the election ` +
        'its projection records, or its coordinator predates winner_author'
    );
  }
  assert.equal(a.json.election!.linkHash, b.json.election!.linkHash, 'different elections');
  assert.equal(
    a.json.election!.elector,
    b.json.election!.elector,
    `one election, two electors: ${a.peer}=${a.json.election!.elector} ${b.peer}=${b.json.election!.elector}`
  );
});
