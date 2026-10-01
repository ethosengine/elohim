import assert from 'node:assert/strict';
import { test } from 'node:test';

import { encodeHashToBase64 } from '@holochain/client';

import { delegationDocument, delegationWire } from '../lib/steward-delegation.js';

const hash = (byte: number): Uint8Array => new Uint8Array(39).fill(byte);
const signature = new Uint8Array(64).fill(7);

function wire(issuance?: Uint8Array) {
  return {
    payload: {
      grantor: hash(1),
      delegate: hash(2),
      scope: 'fct-course',
      valid_until: 100,
      root_action_hash: hash(3),
      dna_hash: hash(4),
      ...(issuance ? { issuance_action_hash: issuance } : {}),
    },
    signature,
    acceptance: {
      head_action_hash: hash(5),
      witness_action_hash: hash(6),
      accepted_at: 99,
      signature,
    },
  };
}

void test('portable grant round-trip retains the unique issuance and witness actions', () => {
  const original = wire(hash(8));
  const document = delegationDocument(original);
  assert.equal(document.issuanceActionHash, encodeHashToBase64(hash(8)));
  assert.equal(document.acceptance?.witnessActionHash, encodeHashToBase64(hash(6)));
  const decoded = delegationWire(document);
  assert.deepEqual(decoded.payload.issuance_action_hash, hash(8));
  assert.deepEqual(decoded.acceptance?.witness_action_hash, hash(6));
});

void test('legacy v2 grant round-trip keeps issuance field omitted', () => {
  const document = delegationDocument(wire());
  assert.equal(Object.hasOwn(document, 'issuanceActionHash'), false);
  const decoded = delegationWire(document);
  assert.equal(Object.hasOwn(decoded.payload, 'issuance_action_hash'), false);
});

void test('portable witnessed-device grant preserves native attribution and binding in the signed payload', () => {
  const original = wire(hash(8));
  const grant = {
    ...original,
    payload: {
      ...original.payload,
      device_binding: hash(9),
      exercise: { requester: hash(10), executor: hash(1), policy: 'fct-commons-v1' },
    },
  };
  const decoded = delegationWire(delegationDocument(grant));
  assert.deepEqual(decoded, grant);
});

void test('canonical hints reject duplicates and native lineage cannot certify a different root or author', async () => {
  const { canonicalRootHints, verifyRootHint } = await import('../lib/steward-delegation.js');
  const native = (prefix: number, byte: number) =>
    new Uint8Array([132, prefix, 36, ...new Uint8Array(36).fill(byte)]);
  const root = native(41, 1),
    author = native(32, 2);
  const row = {
    id: 'lesson',
    root: encodeHashToBase64(root),
    rootAuthor: encodeHashToBase64(author),
  };
  assert.throws(() => canonicalRootHints({ canonicalRoots: [row, row] }), /duplicate/);
  assert.throws(
    () => canonicalRootHints({ canonicalRoots: [{ ...row, root: row.rootAuthor }] }),
    /native Create/
  );
  const hint = canonicalRootHints({ canonicalRoots: [row] }).lesson;
  for (const changed of [
    { content_id: 'wrong' },
    { truncated: true },
    { referenced_action_hash: native(41, 3) },
    { root_action_hash: native(41, 4) },
    { root_author: native(32, 5) },
  ]) {
    const conductor = {
      call: async () =>
        Promise.resolve({
          content_id: 'lesson',
          truncated: false,
          referenced_action_hash: root,
          root_action_hash: root,
          root_author: author,
          ...changed,
        }),
    } as unknown as Pick<import('../lib/steward-conductor.js').Conductor, 'call'>;
    await assert.rejects(verifyRootHint(conductor, 'lesson', hint), /mismatched native root/);
  }
});

void test('exact controller publication witness survives portable receipt recovery', () => {
  const original = wire(hash(8));
  const accepted = {
    ...original,
    acceptance: {
      ...original.acceptance,
      device_witness_action_hash: hash(11),
    },
  };
  const document = delegationDocument(accepted);
  assert.equal(document.acceptance?.deviceWitnessActionHash, encodeHashToBase64(hash(11)));
  assert.deepEqual(delegationWire(document), accepted);
});
