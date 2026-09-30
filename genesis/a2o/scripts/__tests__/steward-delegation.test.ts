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
